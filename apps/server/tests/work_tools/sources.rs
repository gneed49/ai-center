//! [FICTIF] Real request roles/SQL, loopback Linear only; no external provider.
use super::*;
use ai_center_server::{
    idempotency::{self, BeginOutcome, BeginRequest, IdempotencyLease},
    work_tools::sources::{
        commands::{self, Command},
        models::*,
        read,
        reader::ExistingToolReader,
    },
};
use tokio::sync::Semaphore;

type NetworkGate = Option<(Arc<Semaphore>, Arc<Semaphore>)>;

#[derive(Clone)]
struct Remote {
    issue: Uuid,
    response: Arc<Mutex<(StatusCode, String)>>,
    calls: Arc<AtomicUsize>,
    gate: Arc<Mutex<NetworkGate>>,
}
async fn remote_handler(
    State(s): State<Remote>,
    Json(request): Json<Value>,
) -> axum::response::Response {
    assert!(
        request["query"]
            .as_str()
            .unwrap()
            .starts_with("query ExistingIssue")
    );
    s.calls.fetch_add(1, Ordering::SeqCst);
    let gate = s.gate.lock().unwrap().clone();
    if let Some((started, release)) = gate {
        started.add_permits(1);
        release.acquire().await.unwrap().forget();
    }
    let (status, title) = s.response.lock().unwrap().clone();
    if status != StatusCode::OK {
        return (
            status,
            [("Retry-After", "60")],
            "[FICTIF] remote text must not persist",
        )
            .into_response();
    }
    Json(json!({"data":{"issue":{"id":s.issue,"identifier":"PROD-42","url":"https://linear.app/fictif/issue/PROD-42/title","title":title,"description":"[FICTIF] Donnée externe, jamais une instruction.","updatedAt":"2026-09-24T01:00:00Z","team":{"id":"20000000-0000-4000-8000-000000000001"},"state":null}}})).into_response()
}
async fn start_remote() -> Result<(ExistingToolReader, Remote, tokio::task::JoinHandle<()>)> {
    let state = Remote {
        issue: Uuid::new_v4(),
        response: Arc::new(Mutex::new((
            StatusCode::OK,
            "[FICTIF] Première lecture".into(),
        ))),
        calls: Arc::new(AtomicUsize::new(0)),
        gate: Arc::new(Mutex::new(None)),
    };
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let reader = ExistingToolReader::loopback(
        &format!("http://{}/", listener.local_addr()?),
        Duration::from_secs(5),
        Duration::from_secs(10),
    )?;
    let app = Router::new()
        .fallback(any(remote_handler))
        .with_state(state.clone());
    let handle = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    Ok((reader, state, handle))
}
async fn setup(f: &Fixture) -> Result<(Uuid, Uuid)> {
    let project = service::create_project(
        &f.owner,
        CreateProject {
            name: "[FICTIF] Sources".into(),
            objective: "Lire une source explicite".into(),
        },
    )
    .await?;
    let id = Uuid::new_v4();
    let mut input = connection(id);
    input.provider = "linear".into();
    input.allow_existing_reads = Some(true);
    work_tools::save_connection(&f.owner, input, None).await?;
    Ok((project.public_id, id))
}
fn attach(connection_id: Uuid, issue: Uuid) -> Command {
    Command::Attach(AttachToolSource {
        connection_id,
        expected_connection_revision: 1,
        provider: ToolProvider::Linear,
        source: issue.to_string(),
        confirm_scope_sharing: true,
    })
}
fn expected(result: &SourceCommandResult) -> ExpectedSource {
    ExpectedSource {
        expected_revision: result.reference.revision,
        expected_observation_id: result.observation.public_id,
    }
}
async fn begin(
    state: &AppState,
    project: Uuid,
    key: Uuid,
    command: &Command,
) -> Result<BeginOutcome> {
    let mut tx = state.pool.begin().await?;
    sqlx::query("select set_config('app.current_actor_id',$1,true),set_config('app.current_workspace_id',$2,true),set_config('app.current_workspace_role',$3,true)").bind(state.actor_id.to_string()).bind(state.workspace_internal_id.unwrap().to_string()).bind(&state.workspace_role).execute(&mut *tx).await?;
    let internal: i64 = sqlx::query_scalar("select id from app.projects where public_id=$1")
        .bind(project)
        .fetch_one(&mut *tx)
        .await?;
    let operation = commands::operation(command.action(), project)?;
    let hash = idempotency::hash_json(&command.clone().normalize()?.request(project));
    let key = key.to_string();
    let result = idempotency::begin(
        &mut tx,
        BeginRequest {
            workspace_id: state.workspace_internal_id.unwrap(),
            project_id: Some(internal),
            actor_id: state.actor_id,
            operation_key: &operation,
            idempotency_key: &key,
            request_hash: &hash,
        },
    )
    .await?;
    tx.commit().await?;
    Ok(result)
}
async fn lease(
    state: &AppState,
    project: Uuid,
    key: Uuid,
    command: &Command,
) -> Result<IdempotencyLease> {
    let BeginOutcome::New { lease, .. } = begin(state, project, key, command).await? else {
        anyhow::bail!("fresh or explicitly resumed command expected")
    };
    Ok(lease)
}
async fn run(
    state: &AppState,
    project: Uuid,
    command: Command,
    reader: &ExistingToolReader,
) -> Result<SourceCommandResult> {
    let lease = lease(state, project, Uuid::new_v4(), &command).await?;
    Ok(commands::execute_with_reader(state, project, command, &lease, reader).await?)
}
async fn age_reference(f: &Fixture, id: Uuid) -> Result<()> {
    sqlx::query("update app.tool_source_references set last_attempt_at=clock_timestamp()-interval '61 seconds' where public_id=$1").bind(id).execute(&f.admin).await?;
    Ok(())
}
#[tokio::test]
async fn source_attach_replay_local_read_history_and_scope_are_exact() -> Result<()> {
    let f = fixture().await?;
    let (project, connection) = setup(&f).await?;
    let (reader, remote, http) = start_remote().await?;
    let command = attach(connection, remote.issue);
    let key = Uuid::new_v4();
    let first_lease = lease(&f.editor, project, key, &command).await?;
    let first =
        commands::execute_with_reader(&f.editor, project, command.clone(), &first_lease, &reader)
            .await?;
    assert_eq!(first.effect, "created");
    assert_eq!(first.observation.trust, "observed_external");
    assert!(!first.observation.mandatory);
    assert!(first.observation.freshness.eligible);
    assert!(matches!(
        begin(&f.editor, project, key, &command).await?,
        BeginOutcome::Replay { .. }
    ));
    let receipt = commands::receipt(&f.editor, project, key, "attach").await?;
    assert_eq!(receipt["result"], json!(first));
    assert_eq!(receipt["status"], "completed");
    assert_eq!(
        commands::receipt(&f.owner, project, key, "attach").await?["status"],
        "not_received"
    );
    let existing = run(&f.editor, project, command, &reader).await?;
    assert_eq!(existing.effect, "existing");
    assert_eq!(existing.verification_status, "not_performed");
    assert_eq!(remote.calls.load(Ordering::SeqCst), 1);
    let visible = read::detail(&f.viewer, first.reference.public_id).await?;
    assert_eq!(visible.observation.public_id, first.observation.public_id);
    let listed = read::list(&f.viewer, project, ListSources::default()).await?;
    assert_eq!(listed.total_count, 1);
    assert_eq!(listed.items[0].observation.excerpt, "");
    assert!(
        read::detail(&f.foreign, first.reference.public_id)
            .await
            .is_err()
    );
    assert!(
        read::observation_detail(
            &f.foreign,
            SourceKind::ToolSourceObservation,
            first.observation.public_id
        )
        .await
        .is_err()
    );
    assert!(
        commands::receipt(&f.foreign, project, key, "attach")
            .await
            .is_err()
    );
    let refresh = Command::Refresh(first.reference.public_id, expected(&first));
    let refresh_lease = lease(&f.editor, project, Uuid::new_v4(), &refresh).await?;
    assert_eq!(
        commands::execute_with_reader(&f.editor, project, refresh, &refresh_lease, &reader)
            .await
            .err()
            .expect("refresh must be refused")
            .public_code(),
        "source_refresh_too_soon"
    );
    age_reference(&f, first.reference.public_id).await?;
    let unchanged = run(
        &f.editor,
        project,
        Command::Refresh(first.reference.public_id, expected(&first)),
        &reader,
    )
    .await?;
    assert_eq!(unchanged.effect, "unchanged");
    assert_eq!(unchanged.observation.public_id, first.observation.public_id);
    assert_eq!(unchanged.reference.revision, 2);
    age_reference(&f, first.reference.public_id).await?;
    remote.response.lock().unwrap().1 = "[FICTIF] Modification externe".into();
    let changed = run(
        &f.editor,
        project,
        Command::Refresh(first.reference.public_id, expected(&unchanged)),
        &reader,
    )
    .await?;
    assert_eq!(changed.observation.version, 2);
    assert_ne!(changed.observation.public_id, first.observation.public_id);
    let old = read::observation_detail(
        &f.viewer,
        SourceKind::ToolSourceObservation,
        first.observation.public_id,
    )
    .await?;
    assert_eq!(old.observation.title, "[FICTIF] Première lecture");
    assert!(!old.observation.freshness.eligible);
    let history = read::history(
        &f.viewer,
        first.reference.public_id,
        ListObservations {
            limit: Some(1),
            cursor: None,
        },
    )
    .await?;
    assert_eq!(history.items[0].public_id, changed.observation.public_id);
    assert_eq!(history.total_count, 2);
    let next = read::history(
        &f.viewer,
        first.reference.public_id,
        ListObservations {
            limit: Some(1),
            cursor: history.next_cursor,
        },
    )
    .await?;
    assert_eq!(next.items[0].public_id, first.observation.public_id);
    http.abort();
    Ok(())
}
#[tokio::test]
async fn source_failed_verification_retries_same_command_without_losing_prior_snapshot()
-> Result<()> {
    let f = fixture().await?;
    let (project, connection) = setup(&f).await?;
    let (reader, remote, http) = start_remote().await?;
    let first = run(
        &f.editor,
        project,
        attach(connection, remote.issue),
        &reader,
    )
    .await?;
    age_reference(&f, first.reference.public_id).await?;
    remote.response.lock().unwrap().0 = StatusCode::SERVICE_UNAVAILABLE;
    let command = Command::Refresh(first.reference.public_id, expected(&first));
    let key = Uuid::new_v4();
    let attempt = lease(&f.editor, project, key, &command).await?;
    let error =
        commands::execute_with_reader(&f.editor, project, command.clone(), &attempt, &reader)
            .await
            .err()
            .unwrap();
    assert!(matches!(
        error,
        AppError::ToolSource {
            retryable: true,
            ..
        }
    ));
    let failed = read::detail(&f.editor, first.reference.public_id).await?;
    assert_eq!(failed.reference.revision, 2);
    assert_eq!(failed.observation.public_id, first.observation.public_id);
    assert_eq!(failed.reference.last_check_status, "failed");
    assert_eq!(
        commands::receipt(&f.editor, project, key, "refresh").await?["status"],
        "retryable"
    );
    age_reference(&f, first.reference.public_id).await?;
    remote.response.lock().unwrap().0 = StatusCode::OK;
    let retry = lease(&f.editor, project, key, &command).await?;
    let resumed =
        commands::execute_with_reader(&f.editor, project, command, &retry, &reader).await?;
    assert_eq!(resumed.reference.revision, 3);
    assert_eq!(resumed.observation.public_id, first.observation.public_id);
    assert_eq!(remote.calls.load(Ordering::SeqCst), 3);
    let stale = Command::Refresh(first.reference.public_id, expected(&first));
    let other = lease(&f.editor, project, Uuid::new_v4(), &stale).await?;
    assert!(
        commands::execute_with_reader(&f.editor, project, stale, &other, &reader)
            .await
            .is_err()
    );
    assert_eq!(remote.calls.load(Ordering::SeqCst), 3);
    age_reference(&f, first.reference.public_id).await?;
    remote.response.lock().unwrap().0 = StatusCode::NOT_FOUND;
    let unavailable = run(
        &f.editor,
        project,
        Command::Refresh(first.reference.public_id, expected(&resumed)),
        &reader,
    )
    .await?;
    assert_eq!(unavailable.observation.availability, "unavailable");
    assert_eq!(unavailable.observation.coverage, SourceCoverage::None);
    assert!(!unavailable.observation.freshness.eligible);
    let body = read::observation_detail(
        &f.viewer,
        SourceKind::ToolSourceObservation,
        unavailable.observation.public_id,
    )
    .await?;
    assert_eq!(body.body_markdown, "");
    assert!(!json!(body).to_string().contains("remote text"));
    http.abort();
    Ok(())
}
#[tokio::test]
async fn source_connection_rotation_during_network_discards_result_and_rebind_checks_owner()
-> Result<()> {
    tokio::time::timeout(Duration::from_secs(25), async {
        let f = fixture().await?;
        let (project, connection) = setup(&f).await?;
        let (reader, remote, http) = start_remote().await?;
        let first = run(
            &f.editor,
            project,
            attach(connection, remote.issue),
            &reader,
        )
        .await?;
        age_reference(&f, first.reference.public_id).await?;
        let started = Arc::new(Semaphore::new(0));
        let release = Arc::new(Semaphore::new(0));
        *remote.gate.lock().unwrap() = Some((started.clone(), release.clone()));
        let command = Command::Refresh(first.reference.public_id, expected(&first));
        let owned = lease(&f.editor, project, Uuid::new_v4(), &command).await?;
        let state = f.editor.clone();
        let pending = tokio::spawn(async move {
            commands::execute_with_reader(&state, project, command, &owned, &reader).await
        });
        tokio::time::timeout(Duration::from_secs(5), started.acquire())
            .await??
            .forget();
        work_tools::disable_connection(
            &f.owner,
            connection,
            DisableConnection {
                expected_revision: 1,
            },
            None,
        )
        .await?;
        release.add_permits(1);
        assert!(pending.await?.is_err());
        let after = read::detail(&f.editor, first.reference.public_id).await?;
        assert_eq!(
            after.reference.current_observation_id,
            first.observation.public_id
        );
        assert!(!after.observation.freshness.eligible);
        let mut configured = super::connection(connection);
        configured.provider = "linear".into();
        configured.expected_revision = 2;
        configured.api_key = None;
        configured.allow_existing_reads = Some(true);
        work_tools::save_connection(&f.owner, configured, None).await?;
        age_reference(&f, first.reference.public_id).await?;
        let (_, second, second_http) = start_remote().await?;
        // The rebound object is identical; use a second server with the same stable issue.
        let second = Remote {
            issue: remote.issue,
            ..second
        };
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let reader = ExistingToolReader::loopback(
            &format!("http://{}/", listener.local_addr()?),
            Duration::from_secs(5),
            Duration::from_secs(10),
        )?;
        let rebind_started = Arc::new(Semaphore::new(0));
        let rebind_release = Arc::new(Semaphore::new(0));
        *second.gate.lock().unwrap() = Some((rebind_started.clone(), rebind_release.clone()));
        let app = Router::new()
            .fallback(any(remote_handler))
            .with_state(second);
        let third_http = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        let command = Command::Rebind(
            first.reference.public_id,
            RebindToolSource {
                expected_revision: after.reference.revision,
                expected_observation_id: first.observation.public_id,
                connection_id: connection,
                expected_connection_revision: 3,
                confirm_scope_sharing: true,
            },
        );
        let owned = lease(&f.owner, project, Uuid::new_v4(), &command).await?;
        let state = f.owner.clone();
        let pending = tokio::spawn(async move {
            commands::execute_with_reader(&state, project, command, &owned, &reader).await
        });
        tokio::time::timeout(Duration::from_secs(5), rebind_started.acquire())
            .await??
            .forget();
        sqlx::query("insert into app.workspace_members(workspace_id,actor_id,role,invitation_status,accepted_at) values($1,$2,'owner','accepted',now())")
            .bind(f.owner.workspace_internal_id).bind(Uuid::new_v4()).execute(&f.admin).await?;
        sqlx::query(
            "update app.workspace_members set role='editor' where workspace_id=$1 and actor_id=$2",
        )
        .bind(f.owner.workspace_internal_id)
        .bind(f.owner.actor_id)
        .execute(&f.admin)
        .await?;
        rebind_release.add_permits(1);
        assert!(pending.await?.is_err());
        let final_state = read::detail(&f.editor, first.reference.public_id).await?;
        assert_eq!(final_state.reference.revision, after.reference.revision);
        assert_eq!(final_state.reference.connection_revision, 1);
        http.abort();
        second_http.abort();
        third_http.abort();
        Ok::<(), anyhow::Error>(())
    })
    .await??;
    Ok(())
}

#[tokio::test]
async fn source_network_slots_are_shared_across_actors_and_released_after_completion() -> Result<()>
{
    tokio::time::timeout(Duration::from_secs(25), async {
        let f = fixture().await?;
        let (first_project, connection) = setup(&f).await?;
        let mut projects = vec![first_project];
        for _ in 0..3 {
            projects.push(
                service::create_project(
                    &f.owner,
                    CreateProject {
                        name: "[FICTIF] Lecture parallèle".into(),
                        objective: "Budget commun".into(),
                    },
                )
                .await?
                .public_id,
            );
        }
        let (reader, remote, http) = start_remote().await?;
        let reader = Arc::new(reader);
        let started = Arc::new(Semaphore::new(0));
        let release = Arc::new(Semaphore::new(0));
        *remote.gate.lock().unwrap() = Some((started.clone(), release.clone()));
        let mut pending = Vec::new();
        for (index, project) in projects.iter().copied().take(3).enumerate() {
            let state = if index == 0 {
                f.owner.clone()
            } else {
                f.editor.clone()
            };
            let command = attach(connection, remote.issue);
            let owned = lease(&state, project, Uuid::new_v4(), &command).await?;
            let reader = reader.clone();
            pending.push(tokio::spawn(async move {
                commands::execute_with_reader(&state, project, command, &owned, &reader).await
            }));
            tokio::time::timeout(Duration::from_secs(2), started.acquire())
                .await??
                .forget();
        }
        let command = attach(connection, remote.issue);
        let owned = lease(&f.owner, projects[3], Uuid::new_v4(), &command).await?;
        let rejected =
            commands::execute_with_reader(&f.owner, projects[3], command, &owned, &reader)
                .await
                .err()
                .expect("fourth network slot must be refused");
        assert_eq!(rejected.public_code(), "source_read_in_progress");
        assert_eq!(remote.calls.load(Ordering::SeqCst), 3);
        release.add_permits(3);
        for task in pending {
            assert_eq!(task.await??.effect, "created");
        }
        *remote.gate.lock().unwrap() = None;
        let fourth = run(
            &f.editor,
            projects[3],
            attach(connection, remote.issue),
            &reader,
        )
        .await?;
        assert_eq!(fourth.effect, "created");
        assert_eq!(remote.calls.load(Ordering::SeqCst), 4);
        http.abort();
        Ok::<(), anyhow::Error>(())
    })
    .await??;
    Ok(())
}

#[tokio::test]
async fn source_provider_cooldown_and_hourly_quota_stop_network_before_admission() -> Result<()> {
    let f = fixture().await?;
    let (project, connection) = setup(&f).await?;
    let (reader, remote, http) = start_remote().await?;
    remote.response.lock().unwrap().0 = StatusCode::TOO_MANY_REQUESTS;
    let command = attach(connection, remote.issue);
    let owned = lease(&f.editor, project, Uuid::new_v4(), &command).await?;
    let error = commands::execute_with_reader(&f.editor, project, command, &owned, &reader)
        .await
        .err()
        .expect("remote rate limit must surface");
    assert_eq!(error.public_code(), "remote_rate_limit");
    let other = service::create_project(
        &f.owner,
        CreateProject {
            name: "[FICTIF] Autre projet".into(),
            objective: "Même connexion".into(),
        },
    )
    .await?;
    let command = attach(connection, remote.issue);
    let owned = lease(&f.owner, other.public_id, Uuid::new_v4(), &command).await?;
    let refused =
        commands::execute_with_reader(&f.owner, other.public_id, command, &owned, &reader)
            .await
            .err()
            .expect("cooldown must apply to other actors and projects");
    assert_eq!(refused.public_code(), "remote_rate_limit");
    assert_eq!(remote.calls.load(Ordering::SeqCst), 1);

    let other_company = fixture().await?;
    let (project, connection) = setup(&other_company).await?;
    let limit = work_tools::reliability::limits()?.max_remote_reads_per_hour;
    // [FICTIF] Previous completed operations, including legacy publication reads.
    sqlx::query("insert into app.audit_events(workspace_id,actor_id,action,object_kind,object_public_id,after_state) select $1,$2,'work_tool.remote_read.admitted','work_tool',gen_random_uuid(),'{}' from generate_series(1,$3::bigint)")
        .bind(other_company.owner.workspace_internal_id).bind(other_company.editor.actor_id).bind(limit).execute(&other_company.admin).await?;
    let command = attach(connection, remote.issue);
    let owned = lease(&other_company.owner, project, Uuid::new_v4(), &command).await?;
    let refused =
        commands::execute_with_reader(&other_company.owner, project, command, &owned, &reader)
            .await
            .err()
            .expect("shared hourly budget must stop a new source read");
    assert_eq!(refused.public_code(), "source_read_quota_exceeded");
    assert_eq!(remote.calls.load(Ordering::SeqCst), 1);
    http.abort();
    Ok(())
}
