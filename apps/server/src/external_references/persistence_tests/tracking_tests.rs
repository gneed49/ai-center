//! ACP-T08 extends the same explicit isolated-stack contract used by T03.
use super::*;

#[allow(clippy::too_many_arguments)]
pub(super) async fn exercise_tracking(
    state: &AppState,
    context: &RequestContext,
    github: &GitHubRuntime,
    mode: &AtomicU16,
    admin: &sqlx::PgPool,
    project_id: Uuid,
    pack_id: Uuid,
    connection_id: Uuid,
    mut proof_input: CreateExternalEvidence,
) -> anyhow::Result<()> {
    let input = CreateGitHubReference {
        url: "https://github.com/acme/context/pull/9".into(),
        tool_connection_id: Some(connection_id),
        tracking: Some(ExternalTrackingInput {
            context_pack_id: pack_id,
            transmission_confirmed: true,
        }),
    };
    let mut invalid = input.clone();
    invalid.tracking.as_mut().unwrap().transmission_confirmed = false;
    assert!(matches!(
        create_github_reference(state, context, github, project_id, Uuid::new_v4(), invalid).await,
        Err(AppError::Invalid(_))
    ));
    let other_project = service::create_project(
        state,
        CreateProject {
            name: "Other scope".into(),
            objective: "Test external provenance project scope".into(),
        },
    )
    .await?;
    assert!(matches!(
        create_github_reference(
            state,
            context,
            github,
            other_project.public_id,
            Uuid::new_v4(),
            input.clone()
        )
        .await,
        Err(AppError::NotFound)
    ));
    let key = Uuid::new_v4();
    let linked =
        create_github_reference(state, context, github, project_id, key, input.clone()).await?;
    assert_eq!(
        linked,
        create_github_reference(state, context, github, project_id, key, input.clone()).await?
    );
    assert_eq!(linked.tracking.len(), 1);
    let chain = &linked.tracking[0];
    assert_eq!(chain.context_pack_public_id, pack_id);
    assert!(chain.context_pack_current);
    assert_eq!(
        chain.status, "running",
        "green checks cannot complete an external task"
    );
    assert_eq!(chain.artifacts.len(), 1);
    assert_eq!(chain.events.len(), 1);
    let reference_id = linked.reference.public_id;
    let original_artifact = chain.artifacts[0].clone();
    let original_task_id = chain.task_public_id;
    let original_execution_id = chain.execution_public_id;
    let repeated = create_github_reference(
        state,
        context,
        github,
        project_id,
        Uuid::new_v4(),
        input.clone(),
    )
    .await?;
    assert_eq!(
        repeated.tracking.len(),
        1,
        "reference/pack declaration is unique across command keys"
    );
    assert_eq!(repeated.tracking[0].task_public_id, original_task_id);
    assert_eq!(
        repeated.tracking[0].artifacts, chain.artifacts,
        "identical observations do not duplicate artifacts"
    );
    assert!(matches!(
        create_evidence(
            state,
            context,
            reference_id,
            Uuid::new_v4(),
            proof_input.clone()
        )
        .await,
        Err(AppError::Invalid(_))
    ));
    proof_input.artifact_id = Some(Uuid::new_v4());
    assert!(matches!(
        create_evidence(
            state,
            context,
            reference_id,
            Uuid::new_v4(),
            proof_input.clone()
        )
        .await,
        Err(AppError::NotFound)
    ));
    proof_input.artifact_id = Some(original_artifact.public_id);
    let candidate_key = Uuid::new_v4();
    let candidate = create_evidence(
        state,
        context,
        reference_id,
        candidate_key,
        proof_input.clone(),
    )
    .await?;
    assert_eq!(
        candidate.artifact_public_id,
        Some(original_artifact.public_id)
    );
    assert_eq!(candidate.status, "candidate");
    assert_eq!(
        candidate,
        create_evidence(
            state,
            context,
            reference_id,
            candidate_key,
            proof_input.clone()
        )
        .await?
    );
    let reviewed = review_evidence(
        state,
        context,
        reference_id,
        candidate.public_id,
        Uuid::new_v4(),
        ReviewExternalEvidence {
            decision: EvidenceDecision::Validate,
        },
    )
    .await?;
    assert_eq!(reviewed.status, "valid");
    let before = get(state, context, reference_id).await?;
    assert_eq!(
        before.tracking[0].execution_public_id,
        original_execution_id
    );
    assert_eq!(
        before.tracking[0]
            .events
            .iter()
            .filter(|event| event.event_type == "evidence.candidate_created")
            .count(),
        1
    );
    assert_eq!(
        before.tracking[0].events.last().unwrap().event_type,
        "evidence.validated"
    );
    mode.store(403, Ordering::Relaxed);
    let retry = Uuid::new_v4();
    for _ in 0..2 {
        assert!(matches!(
            refresh(state, context, github, reference_id, retry).await,
            Err(AppError::ConnectorRateLimited { .. })
        ));
    }
    let limited = get(state, context, reference_id).await?;
    assert_eq!(limited.tracking[0].artifacts, before.tracking[0].artifacts);
    assert_eq!(limited.evidences, before.evidences);
    assert_eq!(
        limited.tracking[0].events.len(),
        before.tracking[0].events.len() + 1
    );
    assert_eq!(
        coverage_status(state, context, candidate.public_id).await?,
        "covered"
    );
    mode.store(200, Ordering::Relaxed);
    let recovered = refresh(state, context, github, reference_id, retry).await?;
    assert_eq!(
        recovered,
        refresh(state, context, github, reference_id, retry).await?
    );
    assert_eq!(
        recovered.tracking[0].artifacts,
        before.tracking[0].artifacts
    );
    assert_eq!(
        recovered.tracking[0].events.len(),
        limited.tracking[0].events.len() + 1
    );
    assert_eq!(recovered.evidences[0].status, "valid");
    exercise_runtime_roles(state, context, admin, reference_id, original_task_id).await?;
    mode.store(201, Ordering::Relaxed);
    let changed = refresh(state, context, github, reference_id, Uuid::new_v4()).await?;
    assert_eq!(changed.tracking[0].artifacts.len(), 2);
    assert_eq!(changed.tracking[0].artifacts[0], original_artifact);
    assert_eq!(changed.evidences[0].status, "stale");
    assert!(matches!(
        create_evidence(
            state,
            context,
            reference_id,
            Uuid::new_v4(),
            proof_input.clone()
        )
        .await,
        Err(AppError::Conflict(_))
    ));
    proof_input.artifact_id = Some(changed.tracking[0].artifacts[1].public_id);
    let new_candidate = create_evidence(
        state,
        context,
        reference_id,
        Uuid::new_v4(),
        proof_input.clone(),
    )
    .await?;
    assert_eq!(new_candidate.status, "candidate");
    let mut tx = begin_scoped_transaction(state, context).await?;
    sqlx::query("update app.projects set graph_version=graph_version+1 where public_id=$1")
        .bind(project_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    let obsolete = get(state, context, reference_id).await?;
    assert!(!obsolete.tracking[0].context_pack_current);
    assert_eq!(
        obsolete.tracking[0].artifacts,
        changed.tracking[0].artifacts
    );
    assert!(matches!(
        create_github_reference(state, context, github, project_id, Uuid::new_v4(), input).await,
        Err(AppError::Conflict(_))
    ));
    assert!(matches!(
        create_evidence(state, context, reference_id, Uuid::new_v4(), proof_input).await,
        Err(AppError::Conflict(_))
    ));
    assert!(matches!(
        review_evidence(
            state,
            context,
            reference_id,
            new_candidate.public_id,
            Uuid::new_v4(),
            ReviewExternalEvidence {
                decision: EvidenceDecision::Validate
            }
        )
        .await,
        Err(AppError::Conflict(_))
    ));
    mode.store(404, Ordering::Relaxed);
    let unavailable = refresh(state, context, github, reference_id, Uuid::new_v4()).await?;
    assert_eq!(unavailable.tracking[0].status, "failed");
    assert_eq!(unavailable.tracking[0].artifacts.len(), 3);
    assert_eq!(unavailable.tracking[0].artifacts[0], original_artifact);
    assert_eq!(
        unavailable.tracking[0]
            .events
            .iter()
            .map(|event| event.sequence_number)
            .collect::<Vec<_>>(),
        (1..=i32::try_from(unavailable.tracking[0].events.len())?).collect::<Vec<_>>()
    );
    Ok(())
}

async fn exercise_runtime_roles(
    state: &AppState,
    context: &RequestContext,
    admin: &sqlx::PgPool,
    reference_id: Uuid,
    task_public_id: Uuid,
) -> anyhow::Result<()> {
    let mut tx = begin_scoped_transaction(state, context).await?;
    let (workspace, project, pack, contract, task, execution, reference): (i64,i64,i64,i64,i64,i64,i64) = sqlx::query_as("select task.workspace_id,task.project_id,task.context_pack_id,task.contract_id,task.id,execution.id,reference.id from app.tasks task join app.executions execution on execution.task_id=task.id join app.external_references reference on reference.public_id=$2 where task.public_id=$1")
        .bind(task_public_id).bind(reference_id).fetch_one(&mut *tx).await?;
    tx.rollback().await?;
    let inserts = [
        format!(
            "insert into app.tasks(workspace_id,project_id,context_pack_id,contract_id,title) values({workspace},{project},{pack},{contract},'RLS contract')"
        ),
        format!(
            "insert into app.executions(workspace_id,project_id,task_id,executor_key) values({workspace},{project},{task},'rls-test')"
        ),
        format!(
            "insert into app.execution_events(workspace_id,project_id,execution_id,event_type,sequence_number) values({workspace},{project},{execution},'rls-test',999)"
        ),
        format!(
            "insert into app.artifacts(workspace_id,project_id,execution_id,external_reference_id,artifact_type,title,reference) values({workspace},{project},{execution},{reference},'rls-test','RLS test','https://github.com/acme/context/pull/9')"
        ),
    ];
    let foreign_actor = Uuid::new_v4();
    let (foreign_workspace, foreign_public_id): (i64,Uuid) = sqlx::query_as("insert into app.workspaces(owner_actor_id,name) values($1,'Tracking isolation fixture') returning id,public_id").bind(foreign_actor).fetch_one(admin).await?;
    sqlx::query("insert into app.workspace_members(workspace_id,actor_id,role,invitation_status,accepted_at) values($1,$2,'owner','accepted',now()) on conflict(workspace_id,actor_id) do nothing").bind(foreign_workspace).bind(foreign_actor).execute(admin).await?;
    for role in ["owner", "editor", "viewer", "foreign_owner"] {
        let actor = if role == "owner" {
            context.actor_id
        } else if role == "foreign_owner" {
            foreign_actor
        } else {
            Uuid::new_v4()
        };
        if matches!(role, "editor" | "viewer") {
            sqlx::query("insert into app.workspace_members(workspace_id,actor_id,role,invitation_status,accepted_at) values($1,$2,$3,'accepted',now())").bind(workspace).bind(actor).bind(role).execute(admin).await?;
        }
        let scoped_context = RequestContext {
            actor_id: actor,
            workspace_id: if role == "foreign_owner" {
                foreign_public_id
            } else {
                context.workspace_id
            },
            workspace_internal_id: Some(if role == "foreign_owner" {
                foreign_workspace
            } else {
                workspace
            }),
            workspace_role: if role == "foreign_owner" {
                "owner"
            } else {
                role
            }
            .into(),
        };
        let scoped_state = state.scoped(&scoped_context);
        if role == "foreign_owner" {
            assert!(matches!(
                get(&scoped_state, &scoped_context, reference_id).await,
                Err(AppError::NotFound)
            ));
            assert!(matches!(
                refresh(
                    &scoped_state,
                    &scoped_context,
                    &GitHubRuntime {
                        client: Arc::new(GitHubClient::for_contract_test(
                            "http://127.0.0.1:1",
                            Duration::from_millis(1)
                        )?),
                        installation_id: 777
                    },
                    reference_id,
                    Uuid::new_v4()
                )
                .await,
                Err(AppError::NotFound)
            ));
        }
        for (table, insert) in ["tasks", "executions", "execution_events", "artifacts"]
            .into_iter()
            .zip(&inserts)
        {
            let mut tx = begin_scoped_transaction(&scoped_state, &scoped_context).await?;
            let count: i64 = sqlx::query_scalar(&format!(
                "select count(*) from app.{table} where project_id={project}"
            ))
            .fetch_one(&mut *tx)
            .await?;
            assert_eq!(count > 0, role != "foreign_owner", "{role} reads {table}");
            let result = sqlx::query(insert).execute(&mut *tx).await;
            if matches!(role, "owner" | "editor") {
                assert_eq!(result?.rows_affected(), 1, "{role} inserts {table}");
            } else {
                assert_eq!(
                    result
                        .unwrap_err()
                        .as_database_error()
                        .and_then(sqlx::error::DatabaseError::code)
                        .as_deref(),
                    Some("42501"),
                    "{role} cannot insert {table}"
                );
            }
            tx.rollback().await?;
            let mut tx = begin_scoped_transaction(&scoped_state, &scoped_context).await?;
            let update = if matches!(table, "tasks" | "executions") {
                format!("update app.{table} set status='running' where project_id={project}")
            } else {
                format!("update app.{table} set created_at=now() where project_id={project}")
            };
            let result = sqlx::query(&update).execute(&mut *tx).await;
            if matches!(table, "execution_events" | "artifacts") {
                assert_eq!(
                    result
                        .unwrap_err()
                        .as_database_error()
                        .and_then(sqlx::error::DatabaseError::code)
                        .as_deref(),
                    Some("42501"),
                    "append-only {table}"
                );
            } else {
                assert_eq!(
                    result?.rows_affected() > 0,
                    matches!(role, "owner" | "editor"),
                    "{role} updates {table}"
                );
            }
            tx.rollback().await?;
        }
    }
    Ok(())
}
