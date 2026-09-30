//! [FICTIF] Exact observed context through real RLS/service transactions; no HTTP.
use super::*;
use ai_center_server::{
    agent::{
        AgentEngine, AgentInput, ContextSelectionDraft, ContextSelectionInput,
        CoverageEvaluationDraft, CoverageEvaluationInput, EngineOutput, StewardInput,
        StewardOutput, TechnicalPlanDraft, TechnicalPlanInput,
    },
    artifacts::{
        self, CreateArtifact, SetDestination, ValidateArtifact,
        generation_contract::{self, ArtifactDraft, ArtifactGenerationInput, GenerateArtifact},
    },
    error::AppResult,
    models::{AgentTurn, CreateHandoff, SendMessage},
    work_tools::{ticket_models::PreviewTickets, tickets},
};
use async_trait::async_trait;
use serde_json::Value;
use std::{
    sync::{
        Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

#[derive(Default)]
struct Probe {
    inputs: Mutex<Vec<Value>>,
    pause: AtomicBool,
    invalid: AtomicBool,
    started: tokio::sync::Notify,
    release: tokio::sync::Notify,
}
impl Probe {
    async fn inspect(&self, context: &Value) -> AppResult<()> {
        self.inputs.lock().unwrap().push(context.clone());
        if self.pause.load(Ordering::SeqCst) {
            self.started.notify_one();
            tokio::time::timeout(Duration::from_secs(20), self.release.notified())
                .await
                .map_err(|_| AppError::Internal("[FICTIF] Model release timed out".into()))?;
        }
        Ok(())
    }
}
#[async_trait]
impl AgentEngine for Probe {
    fn provider_name(&self) -> &'static str {
        "deterministic"
    }
    fn requested_model(&self) -> &'static str {
        "observed-context-probe"
    }
    async fn respond(&self, input: AgentInput) -> AppResult<EngineOutput<AgentTurn>> {
        self.inspect(&input.context).await?;
        let ids = input.context["knowledge"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|item| item["version_public_id"].as_str()?.parse().ok())
            .collect();
        let mut result = DeterministicEngine.respond(input).await?;
        result.output.response = "[FICTIF] OBSERVED_OUTPUT_SENTINEL".into();
        result.output.proposals.clear();
        result.output.sources = if self.invalid.load(Ordering::SeqCst) {
            vec![Uuid::new_v4()]
        } else {
            ids
        };
        Ok(result)
    }
    async fn generate_artifact(
        &self,
        input: ArtifactGenerationInput,
    ) -> AppResult<EngineOutput<ArtifactDraft>> {
        self.inspect(&input.context).await?;
        let ids = input.source_ids.clone();
        let mut result = generation_contract::deterministic(input)?;
        for section in &mut result.output.sections {
            section.source_ids.clone_from(&ids);
        }
        for ticket in &mut result.output.tickets {
            ticket.source_ids.clone_from(&ids);
        }
        result.output.summary = "[FICTIF] OBSERVED_OUTPUT_SENTINEL".into();
        if self.invalid.load(Ordering::SeqCst) {
            result.output.sections[0].source_ids.push(Uuid::new_v4());
        }
        Ok(result)
    }
    async fn select_context(
        &self,
        input: ContextSelectionInput,
    ) -> AppResult<EngineOutput<ContextSelectionDraft>> {
        DeterministicEngine.select_context(input).await
    }
    async fn generate_technical_plan(
        &self,
        input: TechnicalPlanInput,
    ) -> AppResult<EngineOutput<TechnicalPlanDraft>> {
        DeterministicEngine.generate_technical_plan(input).await
    }
    async fn evaluate_coverage(
        &self,
        input: CoverageEvaluationInput,
    ) -> AppResult<EngineOutput<CoverageEvaluationDraft>> {
        DeterministicEngine.evaluate_coverage(input).await
    }
    async fn analyze_contradictions(
        &self,
        input: StewardInput,
    ) -> AppResult<EngineOutput<StewardOutput>> {
        self.inspect(&input.candidate_pairs).await?;
        let mut generated = DeterministicEngine.analyze_contradictions(input).await?;
        assert!(
            !generated.output.assessments.is_empty(),
            "fixture must exercise the steward model"
        );
        for assessment in &mut generated.output.assessments {
            assessment.explanation = "[FICTIF] OBSERVED_OUTPUT_SENTINEL".into();
        }
        if self.invalid.load(Ordering::SeqCst) {
            generated.output.assessments[0]
                .source_version_ids
                .push(Uuid::new_v4());
        }
        Ok(generated)
    }
}
async fn scoped(owner: &AppState) -> Result<sqlx::Transaction<'_, sqlx::Postgres>> {
    let mut tx = owner.pool.begin().await?;
    sqlx::query("select set_config('app.current_actor_id',$1,true),set_config('app.current_workspace_id',$2,true),set_config('app.current_workspace_role',$3,true)")
        .bind(owner.actor_id.to_string()).bind(owner.workspace_internal_id.context("scope")?.to_string()).bind(&owner.workspace_role).execute(&mut *tx).await?;
    Ok(tx)
}
async fn connection(owner: &mut AppState) -> Result<Uuid> {
    use ai_center_server::{
        providers::{ProviderRuntime, encryption::CredentialCipher},
        work_tools::{self, SaveConnection},
    };
    use secrecy::SecretString;
    owner.providers = Some(Arc::new(ProviderRuntime {
        cipher: Some(CredentialCipher::from_bytes(&[37; 32])?),
        subscriptions: None,
    }));
    let id = Uuid::new_v4();
    work_tools::save_connection(
        owner,
        SaveConnection {
            id,
            provider: "linear".into(),
            name: "[FICTIF] No real provider".into(),
            expected_revision: 0,
            allow_existing_reads: Some(true),
            api_key: Some(SecretString::from("synthetic-credential-no-real-account")),
        },
        None,
    )
    .await?;
    Ok(id)
}

async fn attach(
    owner: &AppState,
    project: Uuid,
    connection: Uuid,
    external: Uuid,
    body: &str,
) -> Result<(Uuid, Uuid)> {
    let mut tx = scoped(owner).await?;
    let reference=sqlx::query_scalar("insert into app.tool_source_references(workspace_id,project_id,provider,object_kind,external_id,canonical_url,connection_id,connection_revision,created_by_actor_id) select p.workspace_id,p.id,'linear','issue',$3,'https://linear.app/fixture/issue/FICTIF-1',c.id,c.revision,app.current_actor_id() from app.projects p cross join app.work_tool_connections c where p.public_id=$1 and c.public_id=$2 returning public_id")
        .bind(project).bind(connection).bind(external).fetch_one(&mut *tx).await?;
    let observation = observe(&mut tx, reference, body).await?;
    tx.commit().await?;
    Ok((reference, observation))
}
async fn observe(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    reference: Uuid,
    body: &str,
) -> Result<Uuid> {
    observe_with_metadata(tx, reference, body, &linear_metadata()).await
}
fn linear_metadata() -> Value {
    json!({"identifier":"FICTIF-42","team_id":"20000000-0000-4000-8000-000000000001",
        "state":{"id":"30000000-0000-4000-8000-000000000001","name":"En cours","type":"started"},
        "arbitrary_provider_field":"[FICTIF] MUST_NOT_REACH_MODEL"})
}
fn context_metadata(metadata: &Value) -> Value {
    json!({"identifier":metadata["identifier"],"state":metadata["state"]})
}
async fn observe_with_metadata(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    reference: Uuid,
    body: &str,
    metadata: &Value,
) -> Result<Uuid> {
    let observation=sqlx::query_scalar("insert into app.tool_source_observations(workspace_id,project_id,reference_id,version,provider,object_kind,external_id,canonical_url,connection_id,connection_revision,title,body_markdown,availability,coverage,omission_reasons,projection_version,content_hash,snapshot_hash,metadata) select r.workspace_id,r.project_id,r.id,coalesce((select max(o.version) from app.tool_source_observations o where o.reference_id=r.id),0)+1,r.provider,r.object_kind,r.external_id,r.canonical_url,r.connection_id,r.connection_revision,'[FICTIF] Quartz observed rule',$2,'available','partial','[\"comments_not_read\"]','existing-tool-text-v1',encode(sha256(convert_to(jsonb_build_object('body',$2::text,'state',$3::jsonb->'state')::text,'UTF8')),'hex'),encode(sha256(convert_to(jsonb_build_object('body',$2::text,'metadata',$3::jsonb)::text,'UTF8')),'hex'),$3 from app.tool_source_references r where r.public_id=$1 returning public_id")
        .bind(reference).bind(body).bind(metadata).fetch_one(&mut **tx).await?;
    sqlx::query("update app.tool_source_references r set current_observation_id=o.id,last_checked_at=clock_timestamp(),last_check_status='partial',revision=r.revision+1 from app.tool_source_observations o where o.public_id=$1 and r.id=o.reference_id")
        .bind(observation).execute(&mut **tx).await?;
    Ok(observation)
}
async fn publication(
    owner: &AppState,
    project: Uuid,
    connection: Uuid,
    external: Uuid,
) -> Result<Uuid> {
    let artifact = artifacts::create(
        owner,
        project,
        CreateArtifact {
            artifact_type: "specification".into(),
            title: "[FICTIF] Published source".into(),
            body_markdown: "[FICTIF] Quartz publication body".into(),
            structured_content: json!({}),
            sources: vec![],
        },
        None,
    )
    .await?;
    let validated = artifacts::validate(
        owner,
        artifact.artifact.public_id,
        ValidateArtifact {
            expected_version_id: artifact.current_version.public_id,
        },
        None,
    )
    .await?;
    let mut tx = scoped(owner).await?;
    let job:Uuid=sqlx::query_scalar("insert into app.publication_jobs(workspace_id,project_id,artifact_version_id,connection_id,connection_revision,requested_by_actor_id,provider,target_id,title,body_markdown,content_hash) select v.workspace_id,v.project_id,v.id,c.id,c.revision,app.current_actor_id(),'linear',$3,'[FICTIF] Quartz published issue','[FICTIF] Publication source text',repeat('a',64) from app.artifact_document_versions v cross join app.work_tool_connections c where v.public_id=$1 and c.public_id=$2 returning public_id")
        .bind(validated.current_version.public_id).bind(connection).bind(Uuid::new_v4().to_string()).fetch_one(&mut *tx).await?;
    sqlx::query("update app.publication_jobs set status='succeeded',external_id=$2,external_url='https://linear.app/fixture/issue/FICTIF-2' where public_id=$1").bind(job).bind(external.to_string()).execute(&mut *tx).await?;
    let observation=sqlx::query_scalar("insert into app.publication_observations(workspace_id,publication_job_id,observation_kind,external_id,external_url,snapshot,connection_id,connection_revision) select workspace_id,id,'created',external_id,external_url,jsonb_build_object('title',title,'body_markdown',body_markdown,'complete',true),connection_id,connection_revision from app.publication_jobs where public_id=$1 returning public_id")
        .bind(job).fetch_one(&mut *tx).await?;
    tx.commit().await?;
    Ok(observation)
}
async fn question(
    owner: &AppState,
    session: Uuid,
) -> AppResult<ai_center_server::models::SessionView> {
    service::send_message(
        owner,
        session,
        SendMessage {
            content: "[FICTIF] Quartz source et règles obligatoires ?".into(),
            client_message_id: Uuid::new_v4(),
        },
    )
    .await
}
fn generation(session: Uuid) -> GenerateArtifact {
    GenerateArtifact {
        session_id: session,
        artifact_type: "product_tickets".into(),
        instructions: "[FICTIF] Quartz : préparer les tickets à partir des observations datées."
            .into(),
    }
}
fn external_sources(context: &Value) -> Vec<&Value> {
    context["knowledge"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|s| s["observation"].is_object())
        .collect()
}
fn assert_context_metadata(context: &Value, id: Uuid, metadata: &Value) -> Result<()> {
    let sources = external_sources(context);
    let source = sources
        .iter()
        .find(|s| s["version_public_id"] == id.to_string())
        .context("observed source")?;
    assert_eq!(
        source["observation"]["metadata"],
        context_metadata(metadata)
    );
    assert!(
        !source["statement"]
            .as_str()
            .context("observed body")?
            .contains(metadata["state"]["name"].as_str().context("state name")?)
    );
    assert!(!source.to_string().contains("MUST_NOT_REACH_MODEL"));
    Ok(())
}
async fn pack_current(owner: &AppState, pack: Uuid) -> Result<bool> {
    let mut tx = scoped(owner).await?;
    let result = sqlx::query_scalar(
        "select app.context_pack_scopes_current(id) from app.context_packs where public_id=$1",
    )
    .bind(pack)
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(result)
}

#[tokio::test]
#[allow(clippy::too_many_lines)] // One immutable provenance chain spans all consumers.
async fn observed_sources_flow_from_chat_to_tickets_and_handoff_without_becoming_rules()
-> Result<()> {
    let actor = isolated_actor().await?;
    let mut owner = create(&actor, Uuid::new_v4()).await?;
    let (project, session) = ready_project(&owner, "[FICTIF] Observed flow").await?;
    let company_scope = company::overview(&owner)
        .await?
        .company_scope
        .context("company scope")?
        .project_public_id;
    fixture_knowledge(
        &owner,
        company_scope,
        "business_rule",
        "[FICTIF] Quartz existing issue text must retain the observed state.",
    )
    .await?;
    let conn = connection(&mut owner).await?;
    let (reference, attached) = attach(
        &owner,
        project,
        conn,
        Uuid::new_v4(),
        "[FICTIF] Quartz existing issue text",
    )
    .await?;
    let published = publication(&owner, project, conn, Uuid::new_v4()).await?;
    let probe = Arc::new(Probe::default());
    owner.engine = probe.clone();
    let response = question(&owner, session).await?;
    let context = probe.inputs.lock().unwrap()[0].clone();
    let observed = external_sources(&context);
    assert_eq!(observed.len(), 2);
    for (kind, id) in [
        ("tool_source_observation", attached),
        ("publication_observation", published),
    ] {
        let source = observed
            .iter()
            .find(|s| s["version_public_id"] == id.to_string())
            .context("exact candidate")?;
        assert_eq!(source["source_kind"], kind);
        assert_eq!(source["observation"]["trust"], "observed_external");
        assert_eq!(source["observation"]["mandatory"], false);
        assert!(source["observation"]["observed_at"].is_string());
        assert!(
            response.messages.last().context("response")?.metadata["sources"]
                .as_array()
                .context("citations")?
                .contains(&json!(id))
        );
    }
    assert_context_metadata(&context, attached, &linear_metadata())?;
    let draft =
        service::artifact_generation::generate(&owner, project, generation(session), None).await?;
    let validated = artifacts::validate(
        &owner,
        draft.artifact.public_id,
        ValidateArtifact {
            expected_version_id: draft.current_version.public_id,
        },
        None,
    )
    .await?;
    for id in [attached, published] {
        assert!(
            validated
                .current_version
                .sources
                .iter()
                .any(|s| s["public_id"] == id.to_string())
        );
    }
    let attached_artifact_source = validated
        .current_version
        .sources
        .iter()
        .find(|s| s["public_id"] == attached.to_string())
        .context("artifact observation")?;
    assert_eq!(
        attached_artifact_source["metadata"],
        context_metadata(&linear_metadata())
    );
    assert!(
        !attached_artifact_source
            .to_string()
            .contains("MUST_NOT_REACH_MODEL")
    );
    let target = Uuid::new_v4().to_string();
    artifacts::set_destination(
        &owner,
        Some(project),
        SetDestination {
            artifact_type: "product_tickets".into(),
            provider: "linear".into(),
            target_id: Some(target.clone()),
            label: "[FICTIF] Preview only".into(),
            expected_revision: 0,
        },
        None,
    )
    .await?;
    let preview = tickets::preview(
        &owner,
        draft.artifact.public_id,
        PreviewTickets {
            version_id: validated.current_version.public_id,
            connection_id: conn,
            expected_provider: "linear".into(),
            expected_target_id: target,
            ticket_indexes: vec![0],
        },
    )
    .await?;
    for id in [attached, published] {
        assert!(
            preview.items[0]
                .business_body_markdown
                .contains(&id.to_string())
        );
    }
    assert!(
        preview.items[0]
            .business_body_markdown
            .contains("comments_not_read")
    );
    let pack = compile_for(&owner, project, session, "tech").await?;
    assert_eq!(external_sources(&pack.content).len(), 2);
    assert_context_metadata(&pack.content, attached, &linear_metadata())?;
    let handoff = service::create_handoff(
        &owner,
        project,
        CreateHandoff {
            source_session_id: session,
            context_pack_id: pack.public_id,
        },
    )
    .await?;
    question(&owner, handoff.target_session_public_id).await?;
    let handoff_context = probe.inputs.lock().unwrap().last().unwrap().clone();
    assert_eq!(external_sources(&handoff_context).len(), 2);
    assert_context_metadata(&handoff_context, attached, &linear_metadata())?;
    ai_center_server::steward::analyze_project(
        &owner,
        project,
        ai_center_server::steward::StewardConfig::default(),
    )
    .await?;
    let pairs = probe.inputs.lock().unwrap().last().unwrap().clone();
    let steward_source = pairs
        .as_array()
        .context("steward pairs")?
        .iter()
        .flat_map(|pair| [&pair["left"], &pair["right"]])
        .find(|source| source["version_public_id"] == attached.to_string())
        .context("steward observed source")?;
    let rationale: Value = serde_json::from_str(
        steward_source["rationale"]
            .as_str()
            .context("steward provenance")?,
    )?;
    assert_eq!(
        rationale["provenance"]["metadata"],
        context_metadata(&linear_metadata())
    );
    assert_eq!(rationale["code_read"], false);
    assert!(!rationale.to_string().contains("MUST_NOT_REACH_MODEL"));
    let mut tx = scoped(&owner).await?;
    sqlx::query("update app.tool_source_references set revision=revision+1,last_checked_at=clock_timestamp() where public_id=$1").bind(reference).execute(&mut *tx).await?;
    tx.commit().await?;
    assert!(
        pack_current(&owner, pack.public_id).await?,
        "same snapshot must not invalidate the pack"
    );
    let mut tx = scoped(&owner).await?;
    let mut changed_metadata = linear_metadata();
    changed_metadata["state"] =
        json!({"id":"30000000-0000-4000-8000-000000000002","name":"Terminé","type":"completed"});
    let changed = observe_with_metadata(
        &mut tx,
        reference,
        "[FICTIF] Quartz existing issue text",
        &changed_metadata,
    )
    .await?;
    let (same_body, different_hash):(bool,bool)=sqlx::query_as("select previous.body_markdown=current.body_markdown,previous.content_hash<>current.content_hash from app.tool_source_observations previous cross join app.tool_source_observations current where previous.public_id=$1 and current.public_id=$2")
        .bind(attached).bind(changed).fetch_one(&mut *tx).await?;
    assert!(
        same_body && different_hash,
        "only the observed state changed"
    );
    tx.commit().await?;
    assert!(!pack_current(&owner, pack.public_id).await?);
    let historical = artifacts::get(&owner, draft.artifact.public_id).await?;
    assert!(
        historical
            .current_version
            .sources
            .iter()
            .any(|s| s["public_id"] == attached.to_string())
    );
    assert!(
        !historical
            .current_version
            .sources
            .iter()
            .any(|s| s["public_id"] == changed.to_string())
    );
    let historical_source = historical
        .current_version
        .sources
        .iter()
        .find(|s| s["public_id"] == attached.to_string())
        .context("historical source")?;
    assert_eq!(
        historical_source["metadata"],
        context_metadata(&linear_metadata())
    );
    question(&owner, session).await?;
    assert_context_metadata(
        probe.inputs.lock().unwrap().last().unwrap(),
        changed,
        &changed_metadata,
    )?;
    let replacement = compile_for(&owner, project, session, "tech").await?;
    assert_context_metadata(&replacement.content, changed, &changed_metadata)?;
    let mut tx = scoped(&owner).await?;
    sqlx::query("update app.tool_source_references set status='detached',revision=revision+1 where public_id=$1").bind(reference).execute(&mut *tx).await?;
    tx.commit().await?;
    assert!(!pack_current(&owner, replacement.public_id).await?);
    Ok(())
}

#[tokio::test]
async fn observed_retrieval_deduplicates_objects_and_bounds_them_after_mandatory_rules()
-> Result<()> {
    let actor = isolated_actor().await?;
    let mut owner = create(&actor, Uuid::new_v4()).await?;
    let (project, session) = ready_project(&owner, "[FICTIF] Bounded observations").await?;
    let conn = connection(&mut owner).await?;
    let external = Uuid::new_v4();
    attach(
        &owner,
        project,
        conn,
        external,
        "[FICTIF] Quartz duplicate incoming object",
    )
    .await?;
    publication(&owner, project, conn, external).await?;
    for _ in 0..24 {
        attach(
            &owner,
            project,
            conn,
            Uuid::new_v4(),
            &format!("[FICTIF] Quartz {}", "é".repeat(5000)),
        )
        .await?;
    }
    let company = company::overview(&owner).await?;
    let (_, rule) = fixture_knowledge(
        &owner,
        company
            .company_scope
            .context("company scope")?
            .project_public_id,
        "business_rule",
        "[FICTIF] Mandatory confirmed company rule",
    )
    .await?;
    let probe = Arc::new(Probe::default());
    owner.engine = probe.clone();
    question(&owner, session).await?;
    let context = probe.inputs.lock().unwrap()[0].clone();
    let observed = external_sources(&context);
    assert!(observed.len() <= 20);
    assert!(
        context["retrieval"]["external_omitted_sources"]
            .as_i64()
            .context("omissions")?
            > 0
    );
    assert_eq!(context["retrieval"]["external_duplicate_sources"], 1);
    assert_eq!(context["retrieval"]["external_eligible_objects"], 25);
    assert!(
        context["knowledge"]
            .as_array()
            .context("knowledge")?
            .iter()
            .any(|s| s["version_public_id"] == rule.to_string())
    );
    assert!(
        observed
            .iter()
            .all(|s| s["statement"].as_str().unwrap().len() <= 8192)
    );
    assert!(
        observed
            .iter()
            .any(|s| s["observation"]["excerpt_truncated"] == true)
    );
    assert!(serde_json::to_vec(&context)?.len() <= 160_000);
    Ok(())
}

#[tokio::test]
#[allow(clippy::too_many_lines)] // Six operations share the same in-flight authority race and persistence assertions.
async fn rotation_and_identical_reattestation_during_valid_or_invalid_model_output_never_persist_text()
-> Result<()> {
    for (operation, invalid) in [
        ("chat", false),
        ("chat", true),
        ("artifact", false),
        ("artifact", true),
        ("steward", false),
        ("steward", true),
    ] {
        let actor = isolated_actor().await?;
        let mut owner = create(&actor, Uuid::new_v4()).await?;
        let (project, session) = ready_project(&owner, "[FICTIF] Authority race").await?;
        let conn = connection(&mut owner).await?;
        let (reference, observation) = attach(
            &owner,
            project,
            conn,
            Uuid::new_v4(),
            "[FICTIF] Quartz unchanged remote source",
        )
        .await?;
        let publication = publication(&owner, project, conn, Uuid::new_v4()).await?;
        if operation == "steward" {
            let company_scope = company::overview(&owner)
                .await?
                .company_scope
                .context("company scope")?
                .project_public_id;
            fixture_knowledge(
                &owner,
                company_scope,
                "business_rule",
                "[FICTIF] Quartz unchanged remote source remains mandatory.",
            )
            .await?;
        }
        let probe = Arc::new(Probe::default());
        probe.pause.store(true, Ordering::SeqCst);
        probe.invalid.store(invalid, Ordering::SeqCst);
        owner.engine = probe.clone();
        let state = owner.clone();
        let mut running = tokio::spawn(async move {
            if operation == "steward" {
                ai_center_server::steward::analyze_project(
                    &state,
                    project,
                    ai_center_server::steward::StewardConfig::default(),
                )
                .await
                .map(|_| ())
            } else if operation == "artifact" {
                service::artifact_generation::generate(&state, project, generation(session), None)
                    .await
                    .map(|_| ())
            } else {
                question(&state, session).await.map(|_| ())
            }
        });
        tokio::select! {
            started = tokio::time::timeout(Duration::from_secs(20), probe.started.notified()) => {
                started.with_context(|| format!("{operation}, invalid={invalid}: model was not called"))?;
            }
            finished = &mut running => {
                anyhow::bail!("{operation}, invalid={invalid}: ended before model: {finished:?}");
            }
        }
        let mut tx = scoped(&owner).await?;
        sqlx::query("select pg_advisory_xact_lock(hashtextextended(app.current_workspace_id()::text||':work-tool:'||$1::uuid::text,0))").bind(conn).execute(&mut *tx).await?;
        sqlx::query("update app.work_tool_connections set revision=revision+1 where public_id=$1")
            .bind(conn)
            .execute(&mut *tx)
            .await?;
        sqlx::query("update app.tool_source_references r set connection_revision=c.revision,revision=r.revision+1 from app.work_tool_connections c where r.connection_id=c.id and r.public_id=$1").bind(reference).execute(&mut *tx).await?;
        assert!(sqlx::query_scalar::<_,bool>("select app.tool_source_observation_current(id) from app.tool_source_observations where public_id=$1").bind(observation).fetch_one(&mut *tx).await?);
        sqlx::query("insert into app.publication_observations(workspace_id,publication_job_id,observation_kind,external_id,external_url,remote_updated_at,snapshot,connection_id,connection_revision) select o.workspace_id,o.publication_job_id,'unchanged',o.external_id,o.external_url,o.remote_updated_at,o.snapshot,c.id,c.revision from app.publication_observations o join app.work_tool_connections c on c.id=o.connection_id where o.public_id=$1")
            .bind(publication).execute(&mut *tx).await?;
        assert!(sqlx::query_scalar::<_,bool>("select app.publication_observation_current(id) from app.publication_observations where public_id=$1").bind(publication).fetch_one(&mut *tx).await?);
        tx.commit().await?;
        probe.release.notify_one();
        assert!(matches!(
            tokio::time::timeout(Duration::from_secs(20), running).await??,
            Err(AppError::Conflict(_))
        ));
        let mut tx = scoped(&owner).await?;
        let (status,output):(String,Option<Value>)=sqlx::query_as("select r.status,r.output from app.model_runs r join app.projects p on p.id=r.project_id where p.public_id=$1 order by r.id desc limit 1").bind(project).fetch_one(&mut *tx).await?;
        assert_eq!(
            status,
            if operation == "steward" {
                "cancelled"
            } else {
                "failed"
            }
        );
        assert!(output.is_none());
        if operation == "steward" {
            let running:bool=sqlx::query_scalar("select status='running' from app.steward_scan_progress where workspace_id=app.current_workspace_id()").fetch_one(&mut *tx).await?;
            assert!(
                !running,
                "an authorized actor must release a cancelled scan for a subsequent analysis"
            );
            let insights: i64 = sqlx::query_scalar(
                "select count(*) from app.insights where workspace_id=app.current_workspace_id()",
            )
            .fetch_one(&mut *tx)
            .await?;
            let assessments:i64=sqlx::query_scalar("select count(*) from app.steward_assessments where workspace_id=app.current_workspace_id()").fetch_one(&mut *tx).await?;
            assert_eq!((insights, assessments), (0, 0));
        }
        let assistants:i64=sqlx::query_scalar("select count(*) from app.messages m join app.projects p on p.id=m.project_id where p.public_id=$1 and m.role='assistant'").bind(project).fetch_one(&mut *tx).await?;
        let artifacts:i64=sqlx::query_scalar("select count(*) from app.artifact_documents d join app.projects p on p.id=d.project_id where p.public_id=$1").bind(project).fetch_one(&mut *tx).await?;
        assert_eq!((assistants, artifacts), (0, 1)); // Only the original publication artifact remains.
        tx.commit().await?;
        owner.pool.close().await;
    }
    Ok(())
}
