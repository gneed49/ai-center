//! Explicit agent document generation with the same quota/lease/model lifecycle as chat.
use super::{
    AppError, AppResult, AppState, IdempotencyLease, ModelRunStart, Postgres, Transaction, Uuid,
    complete_model_run, context_pack_by_id, fail_model_run_in_transaction, idempotency,
    project_by_public, record_failed_model_run, record_failed_model_run_with_output,
    session_by_public, sha256_json, start_model_run,
};
use crate::artifacts::{
    self, ArtifactDetail, CreateArtifact, SourceInput,
    generation_contract::{self as contract, ArtifactGenerationInput, GenerateArtifact},
};
use serde_json::json;

fn validate_request(input: &GenerateArtifact, scope: &str) -> AppResult<()> {
    let technical = matches!(
        input.artifact_type.as_str(),
        "technical_plan" | "technical_tickets"
    );
    let allowed = if technical {
        matches!(scope, "tech" | "dev")
    } else {
        matches!(scope, "general" | "product" | "sales")
    };
    if !allowed
        || contract::section_keys(&input.artifact_type).is_empty()
        || input.instructions.trim().is_empty()
        || input.instructions.len() > 8000
    {
        return Err(AppError::Invalid(
            "Choisissez un livrable adapté à cet agent et une consigne de 1 à 8000 caractères."
                .into(),
        ));
    }
    Ok(())
}

/// Generates a draft only. Publication and human validation remain separate commands.
/// # Errors
/// Rejects inaccessible scopes, stale packs, invalid citations, exhausted quotas and provider errors.
#[allow(clippy::too_many_lines)]
pub async fn generate(
    state: &AppState,
    project_public: Uuid,
    input: GenerateArtifact,
    lease: Option<&IdempotencyLease>,
) -> AppResult<ArtifactDetail> {
    artifacts::editor(state)?;
    let mut tx = state.begin_request().await?;
    let project = project_by_public(&mut tx, state.workspace_id, project_public).await?;
    crate::company::data::require_active(&mut tx, project.id).await?;
    let session = session_by_public(&mut tx, state.workspace_id, input.session_id).await?;
    if session.project_id != project.id {
        return Err(AppError::NotFound);
    }
    validate_request(&input, &session.scope_kind)?;
    let conversation =
        crate::conversation_context::capture_latest(&mut tx, session.id, session.public_id).await?;
    let origin = artifacts::session_origin(&mut tx, session.public_id).await?;
    if origin["message_count"].as_i64() != Some(conversation.available_messages) {
        return Err(AppError::Conflict(
            "La conversation a changé pendant la préparation du contexte.".into(),
        ));
    }
    let (context, source_ids, stamps) = if matches!(session.scope_kind.as_str(), "tech" | "dev") {
        let pack_id = session.context_pack_id.ok_or_else(|| {
            AppError::Invalid(
                "Préparez un contexte transmis avant de demander un livrable technique.".into(),
            )
        })?;
        let pack = context_pack_by_id(&mut tx, pack_id).await?;
        if pack.status != "current" || !crate::scope_context::pack_current(&mut tx, pack_id).await?
        {
            return Err(AppError::Conflict(
                "Le contexte transmis a changé. Préparez une nouvelle transmission.".into(),
            ));
        }
        (
            pack.content,
            crate::scope_context::pack_source_ids(&mut tx, pack_id).await?,
            crate::scope_context::pack_stamps(&mut tx, pack_id).await?,
        )
    } else {
        let snapshot =
            crate::scope_context::load_for_query(&mut tx, project.id, &input.instructions).await?;
        (
            snapshot.context(&session.scope_kind, &session.node_key),
            snapshot.source_ids(),
            snapshot.scopes,
        )
    };
    let generation_input = ArtifactGenerationInput {
        artifact_type: input.artifact_type.clone(),
        instructions: input.instructions.clone(),
        agent_instructions: session.instructions.clone(),
        objective: project.objective.clone(),
        context,
        conversation: json!(conversation),
        source_ids: source_ids.clone(),
    };
    let input_hash = sha256_json(&generation_input)?;
    let resolution = crate::providers::resolve_engine_in_transaction(state, &mut tx).await?;
    let run = start_model_run(
        &mut tx,
        &resolution.provider,
        &resolution.model,
        ModelRunStart {
            workspace_id: project.workspace_id,
            project_id: project.id,
            session_id: Some(session.id),
            context_pack_id: session.context_pack_id,
            operation: "generate_artifact",
            prompt_version: "artifact-draft-v1",
            schema_version: "artifact-draft-v1",
            source_graph_version: project.graph_version,
            input_hash: &input_hash,
            source_public_ids: &source_ids,
        },
    )
    .await?;
    let engine = match resolution.engine {
        Ok(engine) => engine,
        Err(error) => {
            fail_model_run_in_transaction(&mut tx, run.id, None, None, &error).await?;
            tx.commit().await?;
            return Err(error);
        }
    };
    tx.commit().await?;
    let generated = match idempotency::with_optional_lease(
        state,
        lease,
        engine.generate_artifact(generation_input),
    )
    .await
    {
        Ok(result) => result,
        Err(error) => {
            record_failed_model_run(state, run.id, &error).await?;
            return Err(error);
        }
    };
    super::with_model_run_access_cleanup(state, run.id, async {
    let output = json!(generated.output);
    if let Err(error) = contract::validate(&input.artifact_type, &generated.output, &source_ids) {
        record_failed_model_run_with_output(
            state,
            run.id,
            &generated.metadata,
            Some(&output),
            &error,
        )
        .await?;
        return Err(error);
    }
    let finalized:AppResult<ArtifactDetail>=async {
    let mut tx = state.begin_request().await?;
    let current = crate::scope_context::verify_snapshot(&mut tx, &stamps).await?
        && crate::conversation_context::verify_latest(&mut tx, session.id, &conversation).await?;
    let pack_current = if let Some(pack) = session.context_pack_id {
        crate::scope_context::pack_current(&mut tx, pack).await?
    } else {
        true
    };
    if !current || !pack_current {
        let error=AppError::Conflict("Le contexte ou la conversation a changé pendant la génération. Relisez les changements avant de réessayer.".into());
        return Err(error);
    }
    let mut cited: Vec<Uuid> = generated
        .output
        .sections
        .iter()
        .flat_map(|s| s.source_ids.iter())
        .chain(
            generated
                .output
                .tickets
                .iter()
                .flat_map(|s| s.source_ids.iter()),
        )
        .copied()
        .collect();
    cited.sort_unstable();
    cited.dedup();
    let mut sources = exact_sources(&mut tx, &cited).await?;
    sources.push(SourceInput {
        kind: "session".into(),
        public_id: session.public_id,
    });
    if let Some(pack) = session.context_pack_id {
        let public_id = sqlx::query_scalar("select public_id from app.context_packs where id=$1")
            .bind(pack)
            .fetch_one(&mut *tx)
            .await?;
        sources.push(SourceInput {
            kind: "context_pack".into(),
            public_id,
        });
    }
    let document = CreateArtifact {
        artifact_type: input.artifact_type.clone(),
        title: generated.output.title.clone(),
        body_markdown: contract::markdown(&generated.output),
        structured_content: json!({
            "format":"agent-artifact-v1","artifact_type":input.artifact_type,"draft":generated.output,
            "generation":{"model_run_id":run.public_id,"input_hash":input_hash,"agent_scope":session.scope_kind,
                "conversation":conversation.provenance(),"source_version_ids":source_ids,"scope_versions":stamps}
        }),
        sources,
    };
    complete_model_run(
        &mut tx,
        run.id,
        session.context_pack_id,
        &generated.metadata,
        &output,
    )
    .await?;
    let result =
        artifacts::create_in_transaction(&mut tx, state, project_public, document, lease, Some(origin)).await?;
    tx.commit().await?;
    Ok(result)
    }.await;
    if let Err(error) = &finalized {
        record_failed_model_run_with_output(
            state,
            run.id,
            &generated.metadata,
            Some(&output),
            error,
        )
        .await?;
    }
    finalized
    }).await
}

async fn exact_sources(
    tx: &mut Transaction<'_, Postgres>,
    ids: &[Uuid],
) -> AppResult<Vec<SourceInput>> {
    let rows:Vec<(String,Uuid)>=sqlx::query_as("select 'knowledge'::text,public_id from app.knowledge_entry_versions where public_id=any($1) and workspace_id=app.current_workspace_id() union all select 'artifact_version',public_id from app.artifact_document_versions where public_id=any($1) and workspace_id=app.current_workspace_id()")
        .bind(ids).fetch_all(&mut **tx).await?;
    if rows.len() != ids.len() {
        return Err(AppError::NotFound);
    }
    Ok(rows
        .into_iter()
        .map(|(kind, public_id)| SourceInput { kind, public_id })
        .collect())
}

/// Read-only receipt scoped to this actor, project and company; never invokes AI.
/// # Errors
/// Rejects inaccessible projects and database failures.
pub async fn receipt(state: &AppState, project: Uuid, key: Uuid) -> AppResult<serde_json::Value> {
    command_receipt(state, project, key, "artifact.generate").await
}

pub(crate) async fn command_receipt(
    state: &AppState,
    project: Uuid,
    key: Uuid,
    operation: &str,
) -> AppResult<serde_json::Value> {
    let mut tx = state.begin_request().await?;
    let project = project_by_public(&mut tx, state.workspace_id, project).await?;
    let result:Option<serde_json::Value>=sqlx::query_scalar("select jsonb_build_object(
        'status',case when status='completed' then 'completed' when expires_at<=now() then 'expired' when status='processing' and locked_until>now() then 'processing' when status='processing' then 'interrupted' when status='failed' and response_body->>'retryable'='true' then 'retryable' else 'failed' end,
        'can_retry',coalesce(expires_at>now() and ((status='processing' and locked_until<=now()) or (status='failed' and response_body->>'retryable'='true')),false),
        'result',case when status='completed' then response_body else null end)
        from app.idempotency_records where workspace_id=app.current_workspace_id() and actor_id=app.current_actor_id()
          and project_id=$1 and operation_key=$3 and idempotency_key=$2")
        .bind(project.id).bind(key.to_string()).bind(operation).fetch_optional(&mut *tx).await?;
    tx.commit().await?;
    let mut result =
        result.unwrap_or_else(|| json!({"status":"not_received","can_retry":true,"result":null}));
    if !matches!(state.workspace_role.as_str(), "owner" | "editor") || project.status != "active" {
        result["can_retry"] = json!(false);
    }
    Ok(result)
}
