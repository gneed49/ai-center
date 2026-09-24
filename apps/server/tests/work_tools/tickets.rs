//! Synthetic acceptance for explicit per-ticket admission and external outcomes.
use super::*;
use ai_center_server::{
    artifacts::generation_contract::{self, ArtifactGenerationInput},
    auth::AuthRuntime,
    config::AuthMode,
    routes,
    work_tools::{
        ticket_models::{PreviewTickets, PublishTickets, TicketCoverageQuery, TicketPreview},
        tickets,
    },
};

async fn setup(
    state: &AppState,
    provider: &str,
    count: usize,
) -> Result<(Uuid, Uuid, PreviewTickets)> {
    let connection_id = Uuid::new_v4();
    let mut configured = connection(connection_id);
    configured.provider = provider.into();
    work_tools::save_connection(state, configured, None).await?;
    let target = if provider == "github" {
        "fictif/repository".into()
    } else {
        Uuid::new_v4().to_string()
    };
    artifacts::set_destination(
        state,
        None,
        SetDestination {
            artifact_type: "product_tickets".into(),
            provider: provider.into(),
            target_id: Some(target.clone()),
            label: "[FICTIF] Tickets".into(),
            expected_revision: 0,
        },
        None,
    )
    .await?;
    let project = service::create_project(
        state,
        CreateProject {
            name: "[FICTIF] Tickets individuels".into(),
            objective: "Préserver le travail confirmé".into(),
        },
    )
    .await?;
    let mut draft = generation_contract::deterministic(ArtifactGenerationInput {
        artifact_type: "product_tickets".into(),
        instructions: "[FICTIF] Travail à faire".into(),
        agent_instructions: String::new(),
        objective: "[FICTIF] Recette".into(),
        context: json!({}),
        conversation: json!([]),
        source_ids: vec![],
    })?
    .output;
    draft.tickets = (0..count)
        .map(|i| {
            let mut ticket = draft.tickets[0].clone();
            ticket.title = format!("[FICTIF] Ticket {}", i + 1);
            ticket.description = format!("[FICTIF] Description distincte {}", i + 1);
            ticket
        })
        .collect();
    let document=artifacts::create(state,project.public_id,CreateArtifact{artifact_type:"product_tickets".into(),title:draft.title.clone(),body_markdown:generation_contract::markdown(&draft),structured_content:json!({"format":"agent-artifact-v1","artifact_type":"product_tickets","draft":draft}),sources:vec![]},None).await?;
    let validated = artifacts::validate(
        state,
        document.artifact.public_id,
        ValidateArtifact {
            expected_version_id: document.current_version.public_id,
        },
        None,
    )
    .await?;
    let input = PreviewTickets {
        version_id: validated.current_version.public_id,
        connection_id,
        expected_provider: provider.into(),
        expected_target_id: target,
        ticket_indexes: (0..count).map(|i| i16::try_from(i).unwrap()).collect(),
    };
    Ok((document.artifact.public_id, project.public_id, input))
}
fn command(input: &PreviewTickets, preview: &TicketPreview) -> PublishTickets {
    PublishTickets {
        version_id: input.version_id,
        connection_id: input.connection_id,
        expected_provider: input.expected_provider.clone(),
        expected_target_id: input.expected_target_id.clone(),
        ticket_indexes: input.ticket_indexes.clone(),
        preview_fingerprint: preview.preview_fingerprint.clone(),
        prior_publications_fingerprint: preview.prior_publications_fingerprint.clone(),
        confirm_additional_issues: preview.requires_additional_confirmation,
    }
}
async fn cleanup(f: &Fixture) -> Result<()> {
    sqlx::query("update app.publication_jobs set status='cancelled' where workspace_id=$1 and status='queued'").bind(f.owner.workspace_internal_id).execute(&f.admin).await?;
    f.owner.pool.close().await;
    f.admin.close().await;
    Ok(())
}
async fn count_jobs(f: &Fixture) -> Result<i64> {
    Ok(
        sqlx::query_scalar("select count(*) from app.publication_jobs where workspace_id=$1")
            .bind(f.owner.workspace_internal_id)
            .fetch_one(&f.admin)
            .await?,
    )
}
async fn serve(state: AppState, mode: AuthMode) -> Result<(String, tokio::task::JoinHandle<()>)> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let url = format!("http://{}", listener.local_addr()?);
    let auth = Arc::new(AuthRuntime::new(
        mode,
        state.workspace_id,
        state.actor_id,
        None,
    )?);
    let app = routes::router(Arc::new(state), auth, None, vec![]);
    Ok((
        url,
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        }),
    ))
}
#[tokio::test]
async fn preview_middleware_atomic_commands_and_receipts_never_publish_implicitly() -> Result<()> {
    let f = fixture().await?;
    let (artifact, _, input) = setup(&f.owner, "github", 3).await?;
    let client = reqwest::Client::new();
    let (url, http) = serve(f.owner.clone(), AuthMode::Local).await?;
    let endpoint = format!("{url}/api/artifacts/{artifact}/ticket-publications");
    let preview_response = client
        .post(format!("{endpoint}/preview"))
        .json(&input)
        .send()
        .await?;
    assert_eq!(preview_response.status(), StatusCode::OK);
    assert_eq!(count_jobs(&f).await?, 0);
    let preview = tickets::preview(&f.owner, artifact, input.clone()).await?;
    let payload = command(&input, &preview);
    assert_eq!(
        client.post(&endpoint).json(&payload).send().await?.status(),
        StatusCode::UNPROCESSABLE_ENTITY
    );
    for (state, mode, expected) in [
        (f.viewer.clone(), AuthMode::Local, StatusCode::FORBIDDEN),
        (f.foreign.clone(), AuthMode::Local, StatusCode::NOT_FOUND),
        (
            f.owner.clone(),
            AuthMode::Supabase,
            StatusCode::UNAUTHORIZED,
        ),
    ] {
        let (address, server) = serve(state, mode).await?;
        assert_eq!(
            client
                .post(format!(
                    "{address}/api/artifacts/{artifact}/ticket-publications/preview"
                ))
                .json(&input)
                .send()
                .await?
                .status(),
            expected
        );
        server.abort();
    }
    let key = Uuid::new_v4();
    let first = client
        .post(&endpoint)
        .header("Idempotency-Key", key.to_string())
        .json(&payload)
        .send()
        .await?;
    assert!(first.status().is_success());
    let body: Value = first.json().await?;
    let second: Value = client
        .post(&endpoint)
        .header("Idempotency-Key", key.to_string())
        .json(&payload)
        .send()
        .await?
        .json()
        .await?;
    assert_eq!(first_result_ids(&body), first_result_ids(&second));
    assert_eq!(body, second);
    assert_eq!(count_jobs(&f).await?, 3);
    let receipt: Value = client
        .get(format!(
            "{url}/api/artifacts/{artifact}/ticket-publication-commands/{key}"
        ))
        .send()
        .await?
        .json()
        .await?;
    assert_eq!(receipt["status"], "completed");
    assert_eq!(receipt["result"], body);
    // Another actor cannot resume the personal command, even with access to jobs.
    let own = tickets::receipt(&f.editor, artifact, key).await?;
    assert_eq!(own["status"], "not_received");
    sqlx::query("update app.idempotency_records set created_at=now()-interval '2 days',expires_at=now()-interval '1 second' where actor_id=$1 and operation_key=$2 and idempotency_key=$3")
        .bind(f.owner.actor_id).bind(tickets::operation(artifact)).bind(key.to_string()).execute(&f.admin).await?;
    let expired = tickets::receipt(&f.owner, artifact, key).await?;
    assert_eq!(expired["status"], "expired");
    assert_eq!(expired["can_retry"], false);
    assert_eq!(
        client
            .post(&endpoint)
            .header("Idempotency-Key", key.to_string())
            .json(&payload)
            .send()
            .await?
            .status(),
        StatusCode::CONFLICT
    );
    assert_eq!(count_jobs(&f).await?, 3);
    http.abort();
    cleanup(&f).await
}
fn first_result_ids(body: &Value) -> Vec<Value> {
    body["publications"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["public_id"].clone())
        .collect()
}

#[tokio::test]
async fn concurrent_batches_reuse_exact_entries_and_reject_stale_or_unauthorized_previews()
-> Result<()> {
    let f = fixture().await?;
    let (artifact, _, mut input) = setup(&f.owner, "github", 30).await?;
    input.ticket_indexes = vec![2, 0, 1];
    let preview = tickets::preview(&f.owner, artifact, input.clone()).await?;
    assert_eq!(preview.ticket_indexes, vec![0, 1, 2]);
    let for_editor = tickets::preview(&f.editor, artifact, input.clone()).await?;
    let (a, b) = tokio::join!(
        tickets::publish(&f.owner, artifact, command(&input, &preview), None),
        tickets::publish(&f.editor, artifact, command(&input, &for_editor), None)
    );
    let a = a?;
    let b = b?;
    assert_eq!(a.created_count + b.created_count, 3);
    assert_eq!(a.existing_count + b.existing_count, 3);
    assert_eq!(
        a.publications
            .iter()
            .map(|j| j.public_id)
            .collect::<Vec<_>>(),
        b.publications
            .iter()
            .map(|j| j.public_id)
            .collect::<Vec<_>>()
    );
    let coverage = tickets::coverage(
        &f.owner,
        artifact,
        TicketCoverageQuery {
            version_id: input.version_id,
            provider: input.expected_provider.clone(),
            target_id: input.expected_target_id.clone(),
        },
    )
    .await?;
    assert_eq!(coverage.items.len(), 30);
    assert_eq!(
        coverage
            .items
            .iter()
            .filter(|i| i.existing_publication.is_some())
            .count(),
        3
    );
    assert!(matches!(
        tickets::publish(&f.viewer, artifact, command(&input, &preview), None).await,
        Err(AppError::Forbidden)
    ));
    assert!(
        tickets::publish(&f.foreign, artifact, command(&input, &preview), None)
            .await
            .is_err()
    );
    input.ticket_indexes = vec![3];
    let before = tickets::preview(&f.owner, artifact, input.clone()).await?;
    let mut forged = command(&input, &before);
    forged.ticket_indexes = vec![4];
    assert!(matches!(
        tickets::publish(&f.owner, artifact, forged, None).await,
        Err(AppError::Conflict(_))
    ));
    let mut changed = connection(input.connection_id);
    changed.expected_revision = 1;
    changed.api_key = None;
    work_tools::save_connection(&f.owner, changed, None).await?;
    assert!(matches!(
        tickets::publish(&f.owner, artifact, command(&input, &before), None).await,
        Err(AppError::Conflict(_))
    ));
    assert_eq!(count_jobs(&f).await?, 3);
    assert!(
        work_tools::publish(
            &f.owner,
            artifact,
            publish_input(input.version_id, input.connection_id),
            None
        )
        .await
        .is_err()
    );
    cleanup(&f).await
}

#[tokio::test]
async fn quota_rejects_the_whole_selection_and_counts_only_new_jobs() -> Result<()> {
    let f = fixture().await?;
    let (artifact, _, mut input) = setup(&f.owner, "github", 30).await?;
    input.ticket_indexes = (0..24).collect();
    let preview = tickets::preview(&f.owner, artifact, input.clone()).await?;
    let first = tickets::publish(&f.owner, artifact, command(&input, &preview), None).await?;
    assert_eq!(first.created_count, 24);
    input.ticket_indexes = vec![24, 25];
    let preview = tickets::preview(&f.owner, artifact, input.clone()).await?;
    assert!(matches!(
        tickets::publish(&f.owner, artifact, command(&input, &preview), None).await,
        Err(AppError::Capacity { .. })
    ));
    assert_eq!(count_jobs(&f).await?, 24);
    input.ticket_indexes = vec![0, 24];
    let preview = tickets::preview(&f.owner, artifact, input.clone()).await?;
    let added = tickets::publish(&f.owner, artifact, command(&input, &preview), None).await?;
    assert_eq!((added.created_count, added.existing_count), (1, 1));
    let again = tickets::publish(&f.owner, artifact, command(&input, &preview), None).await?;
    assert_eq!((again.created_count, again.existing_count), (0, 2));
    assert_eq!(count_jobs(&f).await?, 25);
    cleanup(&f).await
}

#[derive(Clone)]
struct BatchRemote {
    provider: String,
    target: String,
    bodies: Arc<Mutex<Vec<Value>>>,
    ambiguous_second: bool,
}
async fn batch_remote(
    State(remote): State<BatchRemote>,
    request: axum::http::Request<axum::body::Body>,
) -> axum::response::Response {
    let method = request.method().clone();
    let path = request.uri().path().to_owned();
    let (number, body) = if method == axum::http::Method::POST {
        let bytes = axum::body::to_bytes(request.into_body(), 100_000)
            .await
            .unwrap();
        let payload: Value = serde_json::from_slice(&bytes).unwrap();
        let value = if remote.provider == "linear" {
            json!({"title":payload["variables"]["input"]["title"],"body":payload["variables"]["input"]["description"]})
        } else {
            payload
        };
        let mut bodies = remote.bodies.lock().unwrap();
        bodies.push(value.clone());
        (bodies.len(), value)
    } else {
        let n = path.rsplit('/').next().unwrap().parse::<usize>().unwrap();
        (n, remote.bodies.lock().unwrap()[n - 1].clone())
    };
    if method == axum::http::Method::POST && number == 2 && remote.ambiguous_second {
        return StatusCode::BAD_GATEWAY.into_response();
    }
    if remote.provider == "linear" {
        Json(json!({"data":{"issueCreate":{"success":true,"issue":{"id":Uuid::from_u128(100+number as u128),"url":format!("https://linear.app/fictif/issue/TP-{number}"),"title":body["title"],"description":body["body"],"updatedAt":"2026-09-23T12:00:00Z","team":{"id":remote.target}}}}})).into_response()
    } else {
        Json(json!({"number":number,"html_url":format!("https://github.com/fictif/repository/issues/{number}"),"title":body["title"],"body":body["body"],"updated_at":"2026-09-23T12:00:00Z"})).into_response()
    }
}
#[tokio::test]
async fn three_tickets_create_three_distinct_issues_and_only_uncertain_ticket_needs_reconciliation()
-> Result<()> {
    for provider in ["linear", "github"] {
        let f = fixture().await?;
        let (artifact, _, input) = setup(&f.owner, provider, 3).await?;
        let preview = tickets::preview(&f.owner, artifact, input.clone()).await?;
        let expected_bodies: Vec<_> = preview
            .items
            .iter()
            .map(|i| i.business_body_markdown.clone())
            .collect();
        let result = tickets::publish(&f.owner, artifact, command(&input, &preview), None).await?;
        let remote = BatchRemote {
            provider: provider.into(),
            target: input.expected_target_id.clone(),
            bodies: Arc::new(Mutex::new(vec![])),
            ambiguous_second: provider == "github",
        };
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let client = ToolClient::loopback(
            &format!("http://{}/", listener.local_addr()?),
            Duration::from_secs(5),
        )?;
        let app = Router::new()
            .fallback(any(batch_remote))
            .with_state(remote.clone());
        let http = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        for _ in 0..3 {
            assert!(worker::drain_one(&f.owner, &client).await?);
        }
        assert!(!worker::drain_one(&f.owner, &client).await?);
        assert_eq!(remote.bodies.lock().unwrap().len(), 3);
        for (index, job) in result.publications.iter().enumerate() {
            let detail = work_tools::detail(&f.owner, job.public_id).await?;
            let body = remote.bodies.lock().unwrap()[index]["body"]
                .as_str()
                .unwrap()
                .to_owned();
            assert!(body.starts_with(&expected_bodies[index]));
            assert!(body.contains(&job.public_id.to_string()));
            assert_eq!(
                detail.publication.status,
                if provider == "github" && index == 1 {
                    "needs_review"
                } else {
                    "succeeded"
                }
            );
            if provider == "github" && index == 1 {
                assert!(
                    work_tools::observe_with_client(
                        &f.owner,
                        job.public_id,
                        Some("1".into()),
                        &client
                    )
                    .await
                    .is_err()
                );
                assert_eq!(
                    work_tools::observe_with_client(
                        &f.owner,
                        job.public_id,
                        Some("2".into()),
                        &client
                    )
                    .await?
                    .status,
                    "succeeded"
                );
            }
        }
        assert_eq!(remote.bodies.lock().unwrap().len(), 3);
        http.abort();
        cleanup(&f).await?;
    }
    Ok(())
}

#[tokio::test]
async fn legacy_receipts_replay_unchanged_and_new_versions_require_fresh_additional_confirmation()
-> Result<()> {
    let f = fixture().await?;
    let (artifact, _, mut input) = setup(&f.owner, "github", 2).await?;
    let legacy = Uuid::new_v4();
    sqlx::query("insert into app.publication_jobs(public_id,workspace_id,project_id,artifact_version_id,connection_id,connection_revision,requested_by_actor_id,provider,target_id,title,body_markdown,content_hash,status,attempt_count)
      select $1,v.workspace_id,v.project_id,v.id,c.id,c.revision,$4,'github','fictif/repository','[FICTIF] Ancien document','[FICTIF] Corps historique inchangé',v.content_hash,'needs_review',1 from app.artifact_document_versions v cross join app.work_tool_connections c where v.public_id=$2 and c.public_id=$3")
        .bind(legacy).bind(input.version_id).bind(input.connection_id).bind(f.owner.actor_id).execute(&f.admin).await?;
    let canonical = work_tools::detail(&f.owner, legacy).await?.publication;
    assert_eq!(canonical.source_ticket_index, -1);
    let mut old = json!(canonical);
    for key in ["source_ticket_index", "title", "source_version_number"] {
        old.as_object_mut().unwrap().remove(key);
    }
    let decoded: work_tools::Publication = serde_json::from_value(old.clone())?;
    assert_eq!(decoded.source_ticket_index, -1);
    assert!(decoded.title.is_none());
    assert!(decoded.source_version_number.is_none());
    let key = Uuid::new_v4();
    let old_input = publish_input(input.version_id, input.connection_id);
    let hash = ai_center_server::idempotency::hash_json(
        &json!({"artifact_id":artifact,"input":old_input}),
    );
    sqlx::query("insert into app.idempotency_records(workspace_id,actor_id,operation_key,idempotency_key,request_hash,status,response_status,response_body,locked_until) values($1,$2,'publication.create',$3,$4,'completed',200,$5,null)")
        .bind(f.owner.workspace_internal_id).bind(f.owner.actor_id).bind(key.to_string()).bind(hash).bind(&old).execute(&f.admin).await?;
    let (url, http) = serve(f.owner.clone(), AuthMode::Local).await?;
    let client = reqwest::Client::new();
    let replay = client
        .post(format!("{url}/api/artifacts/{artifact}/publications"))
        .header("Idempotency-Key", key.to_string())
        .json(&old_input)
        .send()
        .await?;
    assert_eq!(replay.status(), StatusCode::OK);
    assert_eq!(replay.json::<Value>().await?, old);
    let hydrated: Value = client
        .get(format!("{url}/api/publications/{legacy}"))
        .send()
        .await?
        .json()
        .await?;
    assert_eq!(hydrated["publication"]["source_ticket_index"], -1);
    assert_eq!(hydrated["publication"]["title"], "[FICTIF] Ancien document");
    let stored:Value=sqlx::query_scalar("select response_body from app.idempotency_records where workspace_id=$1 and idempotency_key=$2")
        .bind(f.owner.workspace_internal_id).bind(key.to_string()).fetch_one(&f.admin).await?;
    assert_eq!(stored, old);
    assert_eq!(count_jobs(&f).await?, 1);
    let before = tickets::preview(&f.owner, artifact, input.clone()).await?;
    assert!(before.requires_additional_confirmation);
    let mut missing_confirmation = command(&input, &before);
    missing_confirmation.confirm_additional_issues = false;
    assert!(matches!(
        tickets::publish(&f.owner, artifact, missing_confirmation, None).await,
        Err(AppError::Invalid(_))
    ));
    sqlx::query("update app.publication_jobs set status='failed' where public_id=$1")
        .bind(legacy)
        .execute(&f.admin)
        .await?;
    assert!(matches!(
        tickets::publish(&f.owner, artifact, command(&input, &before), None).await,
        Err(AppError::Conflict(_))
    ));
    let preview = tickets::preview(&f.owner, artifact, input.clone()).await?;
    let original = tickets::publish(&f.owner, artifact, command(&input, &preview), None).await?;
    let version = artifacts::version(&f.owner, artifact, input.version_id).await?;
    let mut content = version.structured_content;
    content["draft"]["tickets"]
        .as_array_mut()
        .unwrap()
        .reverse();
    let changed: generation_contract::ArtifactDraft =
        serde_json::from_value(content["draft"].clone())?;
    let draft = artifacts::save_draft(
        &f.owner,
        artifact,
        artifacts::SaveDraft {
            expected_version_id: input.version_id,
            title: changed.title.clone(),
            body_markdown: generation_contract::markdown(&changed),
            structured_content: content,
            sources: vec![],
        },
        None,
    )
    .await?;
    let validated = artifacts::validate(
        &f.owner,
        artifact,
        ValidateArtifact {
            expected_version_id: draft.current_version.public_id,
        },
        None,
    )
    .await?;
    assert!(
        tickets::publish(&f.owner, artifact, command(&input, &preview), None)
            .await
            .is_err()
    );
    input.version_id = validated.current_version.public_id;
    let second = tickets::preview(&f.owner, artifact, input.clone()).await?;
    assert!(second.requires_additional_confirmation);
    assert_eq!(second.new_count, 2);
    assert_eq!(second.items[0].title, "[FICTIF] Ticket 2");
    assert!(
        second
            .prior_publications
            .iter()
            .any(|job| job.public_id == original.publications[0].public_id)
    );
    let result = tickets::publish(&f.owner, artifact, command(&input, &second), None).await?;
    assert_eq!(result.created_count, 2);
    assert_eq!(count_jobs(&f).await?, 5);
    assert_ne!(
        result.publications[0].public_id,
        original.publications[0].public_id
    );
    http.abort();
    cleanup(&f).await
}

#[tokio::test]
async fn excessive_history_is_refused_before_any_new_admission() -> Result<()> {
    let f = fixture().await?;
    let (artifact, _, mut input) = setup(&f.owner, "github", 30).await?;
    // Preserve the head=latest invariant: the 17 populated versions are followed
    // by one validated head with no publications, all in the same transaction.
    input.version_id = sqlx::query_scalar("with source as (select * from app.artifact_document_versions where public_id=$1),
      versions as (insert into app.artifact_document_versions(workspace_id,project_id,document_id,version,title,body_markdown,structured_content,status,created_by_actor_id,validated_at,content_hash)
      select workspace_id,project_id,document_id,version+n,title,body_markdown,structured_content,status,created_by_actor_id,validated_at,content_hash from source cross join generate_series(1,17) n returning *),
      jobs as (insert into app.publication_jobs(workspace_id,project_id,artifact_version_id,connection_id,connection_revision,requested_by_actor_id,provider,target_id,title,body_markdown,content_hash,status,source_ticket_index)
      select v.workspace_id,v.project_id,v.id,c.id,c.revision,$3,'github','fictif/repository','[FICTIF] Historique','[FICTIF] Corps',v.content_hash,'failed',n::smallint from versions v cross join app.work_tool_connections c cross join generate_series(0,29) n where c.public_id=$2 returning id),
      head as (insert into app.artifact_document_versions(workspace_id,project_id,document_id,version,title,body_markdown,structured_content,status,created_by_actor_id,validated_at,content_hash)
      select workspace_id,project_id,document_id,version+18,title,body_markdown,structured_content,status,created_by_actor_id,validated_at,content_hash from source returning id,document_id,public_id)
      update app.artifact_documents d set current_version_id=head.id from head where d.id=head.document_id returning head.public_id")
        .bind(input.version_id).bind(input.connection_id).bind(f.owner.actor_id).fetch_one(&f.admin).await?;
    input.ticket_indexes = vec![0];
    assert!(
        matches!(tickets::preview(&f.owner,artifact,input.clone()).await,Err(AppError::Invalid(message)) if message.contains("500"))
    );
    let fake = PublishTickets {
        version_id: input.version_id,
        connection_id: input.connection_id,
        expected_provider: input.expected_provider,
        expected_target_id: input.expected_target_id,
        ticket_indexes: vec![0],
        preview_fingerprint: "0".repeat(64),
        prior_publications_fingerprint: "0".repeat(64),
        confirm_additional_issues: true,
    };
    assert!(
        matches!(tickets::publish(&f.owner,artifact,fake,None).await,Err(AppError::Invalid(message)) if message.contains("500"))
    );
    assert_eq!(count_jobs(&f).await?, 510);
    cleanup(&f).await
}

async fn request_transaction(
    state: &AppState,
) -> Result<sqlx::Transaction<'static, sqlx::Postgres>> {
    let mut tx = state.pool.begin().await?;
    sqlx::query("select set_config('app.current_actor_id',$1,true),set_config('app.current_workspace_id',$2,true),set_config('app.current_workspace_role',$3,true)")
        .bind(state.actor_id.to_string())
        .bind(state.workspace_internal_id.unwrap().to_string())
        .bind(&state.workspace_role)
        .execute(&mut *tx).await?;
    Ok(tx)
}

#[tokio::test]
async fn ticket_command_expiring_while_waiting_rolls_back_jobs_and_receipt() -> Result<()> {
    tokio::time::timeout(Duration::from_secs(30), async {
        use ai_center_server::idempotency::{self, BeginOutcome, BeginRequest};
        let f = fixture().await?;
        let (artifact, _, input) = setup(&f.owner, "github", 2).await?;
        let preview = tickets::preview(&f.owner, artifact, input.clone()).await?;
        let payload = command(&input, &preview);
        for retention_expired in [false, true] {
            let mut tx = request_transaction(&f.owner).await?;
            let project: i64 = sqlx::query_scalar("select project_id from app.artifact_documents where public_id=$1")
                .bind(artifact).fetch_one(&mut *tx).await?;
            let operation = tickets::operation(artifact);
            let key = Uuid::new_v4().to_string();
            let hash = idempotency::hash_json(&json!({"artifact_id":artifact,"input":payload}));
            let BeginOutcome::New { lease, .. } = idempotency::begin(&mut tx, BeginRequest {
                workspace_id: f.owner.workspace_internal_id.unwrap(), project_id: Some(project),
                actor_id: f.owner.actor_id, operation_key: &operation, idempotency_key: &key,
                request_hash: &hash,
            }).await? else { anyhow::bail!("fresh fixture must own its command"); };
            tx.commit().await?;

            let mut blocker = request_transaction(&f.owner).await?;
            let blocker_pid: i32 = sqlx::query_scalar("select pg_backend_pid()")
                .fetch_one(&mut *blocker).await?;
            sqlx::query("select id from app.artifact_documents where public_id=$1 for update")
                .bind(artifact).execute(&mut *blocker).await?;
            let state = f.owner.clone();
            let command = payload.clone();
            let owned_lease = lease.clone();
            // Exercise the service commit itself: a heartbeat is deliberately absent,
            // so only durable fencing can prevent an expired owner committing jobs.
            let pending = tokio::spawn(async move {
                tokio::time::timeout(Duration::from_secs(10), tickets::publish(&state, artifact, command, Some(&owned_lease))).await
            });
            tokio::time::timeout(Duration::from_secs(5), async {
                loop {
                    let waiting: bool = sqlx::query_scalar("select exists(select 1 from pg_stat_activity where $1 = any(pg_blocking_pids(pid)))")
                        .bind(blocker_pid).fetch_one(&f.admin).await?;
                    if waiting { return Ok::<_, anyhow::Error>(()); }
                    tokio::task::yield_now().await;
                }
            }).await??;
            // The server transaction already started before these wall-clock deadlines.
            sqlx::query("update app.idempotency_records set created_at=now()-interval '2 days', locked_until=case when $2 then now()+interval '1 minute' else now()-interval '1 second' end, expires_at=case when $2 then now()-interval '1 second' else now()+interval '1 day' end where public_id=$1")
                .bind(lease.record_public_id).bind(retention_expired).execute(&f.admin).await?;
            blocker.commit().await?;
            assert!(matches!(pending.await??, Err(AppError::Conflict(_))));
            assert_eq!(count_jobs(&f).await?, 0);
            let record: (String, Option<Value>) = sqlx::query_as("select status,response_body from app.idempotency_records where public_id=$1")
                .bind(lease.record_public_id).fetch_one(&f.admin).await?;
            assert_eq!(record, ("processing".into(), None));
            let mut tx = request_transaction(&f.owner).await?;
            assert!(matches!(idempotency::renew(&mut tx, &lease).await, Err(AppError::Conflict(_))));
            tx.rollback().await?;
        }
        cleanup(&f).await
    }).await?
}

#[tokio::test]
async fn editor_admission_then_connection_rotation_cancels_each_ticket_before_http() -> Result<()> {
    let f = fixture().await?;
    let (artifact, _, input) = setup(&f.owner, "github", 2).await?;
    let preview = tickets::preview(&f.editor, artifact, input.clone()).await?;
    let result = tickets::publish(&f.editor, artifact, command(&input, &preview), None).await?;
    assert_eq!(result.created_count, 2);
    let mut rotated = connection(input.connection_id);
    rotated.expected_revision = 1;
    work_tools::save_connection(&f.owner, rotated, None).await?;
    let (client, remote, http) = remote_server().await?;
    for job in result.publications {
        assert!(worker::drain_one(&f.owner, &client).await?);
        assert_eq!(
            work_tools::detail(&f.editor, job.public_id)
                .await?
                .publication
                .status,
            "cancelled"
        );
    }
    assert_eq!(remote.calls.load(Ordering::SeqCst), 0);
    assert!(!worker::drain_one(&f.owner, &client).await?);
    http.abort();
    cleanup(&f).await
}
