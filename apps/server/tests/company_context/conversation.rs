use super::*;
use ai_center_server::{
    agent::{
        AgentEngine, AgentInput, ContextSelectionDraft, ContextSelectionInput,
        CoverageEvaluationDraft, CoverageEvaluationInput, EngineOutput, StewardInput,
        StewardOutput, TechnicalPlanDraft, TechnicalPlanInput,
    },
    error::{AppResult, ProviderError, ProviderErrorClass},
    models::{AgentTurn, SendMessage},
};
use async_trait::async_trait;
use sha2::{Digest, Sha256};
use std::sync::{
    Mutex,
    atomic::{AtomicBool, Ordering},
};
use std::time::Duration;

#[derive(Default)]
struct ConversationProbe {
    inputs: Mutex<Vec<AgentInput>>,
    pause: AtomicBool,
    started: tokio::sync::Notify,
    release: tokio::sync::Notify,
    cite_message: AtomicBool,
    fail_respond: AtomicBool,
}

#[async_trait]
impl AgentEngine for ConversationProbe {
    fn provider_name(&self) -> &'static str {
        "deterministic"
    }
    fn requested_model(&self) -> &'static str {
        "conversation-probe"
    }
    async fn respond(&self, input: AgentInput) -> AppResult<EngineOutput<AgentTurn>> {
        self.inputs.lock().unwrap().push(input.clone());
        if self.pause.load(Ordering::SeqCst) {
            self.started.notify_one();
            tokio::time::timeout(Duration::from_secs(10), self.release.notified())
                .await
                .map_err(|_| AppError::Internal("conversation test release timed out".into()))?;
        }
        if self.fail_respond.load(Ordering::SeqCst) {
            return Err(
                ProviderError::new("Fixture", ProviderErrorClass::Server, 1, Some(503)).into(),
            );
        }
        let mut output = DeterministicEngine.respond(input.clone()).await?;
        output.output.response =
            "[FICTIF] Option une : bleu. Option deux : vert. Hypothèses à valider.".into();
        output.output.proposals.clear();
        if self.cite_message.load(Ordering::SeqCst) {
            output.output.sources = vec![input.conversation.messages[0].message_public_id];
        }
        Ok(output)
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
        DeterministicEngine.analyze_contradictions(input).await
    }
}

async fn insert_message(owner: &AppState, session: Uuid, text: &str) -> Result<Uuid> {
    let mut tx = owner.pool.begin().await?;
    sqlx::query("select set_config('app.current_actor_id',$1,true),set_config('app.current_workspace_id',$2,true),set_config('app.current_workspace_role','owner',true)")
        .bind(owner.actor_id.to_string()).bind(owner.workspace_internal_id.context("scope")?.to_string()).execute(&mut *tx).await?;
    let id = sqlx::query_scalar("insert into app.messages(workspace_id,project_id,session_id,role,content,author_actor_id,client_message_id)
        select workspace_id,project_id,id,'user',$2,app.current_actor_id(),$3 from app.sessions where public_id=$1 returning public_id")
        .bind(session).bind(text).bind(Uuid::new_v4()).fetch_one(&mut *tx).await?;
    tx.commit().await?;
    Ok(id)
}

#[tokio::test]
#[allow(clippy::too_many_lines)] // The same captured turn spans provider pause, authorization and provenance.
async fn brainstorming_uses_only_captured_session_messages_and_never_validates_them() -> Result<()>
{
    let actor = isolated_actor().await?;
    let mut owner = create(&actor, Uuid::new_v4()).await?;
    let foreign = create(&actor, Uuid::new_v4()).await?;
    let (project, session) = ready_project(&owner, "[FICTIF] Brainstorming").await?;
    let neighbor = service::create_session(
        &owner,
        project,
        CreateSession {
            node_key: "product".into(),
            title: None,
        },
    )
    .await?;
    let (_, foreign_session) = ready_project(&foreign, "[FICTIF] Confidentiel").await?;
    insert_message(
        &owner,
        neighbor.session.public_id,
        "[FICTIF] NEIGHBOR PRIVATE SENTINEL",
    )
    .await?;
    insert_message(
        &foreign,
        foreign_session,
        "[FICTIF] FOREIGN PRIVATE SENTINEL",
    )
    .await?;
    let probe = Arc::new(ConversationProbe::default());
    owner.engine = probe.clone();
    let first = service::send_message(
        &owner,
        session,
        SendMessage {
            content: "[FICTIF] Propose deux couleurs pour le lancement.".into(),
            client_message_id: Uuid::new_v4(),
        },
    )
    .await?;
    assert!(
        probe.inputs.lock().unwrap()[0]
            .conversation
            .messages
            .is_empty()
    );
    let first_ids = first
        .messages
        .iter()
        .map(|m| m.public_id)
        .collect::<Vec<_>>();
    assert_eq!(first_ids.len(), 2);
    assert!(
        first
            .messages
            .iter()
            .all(|m| m.metadata.get("conversation_snapshot").is_none())
    );

    // A teammate in the same shared session gets the authorized shared history.
    let teammate = Uuid::new_v4();
    add_member(&owner, teammate, "editor").await?;
    let editor = select(&owner, owner.workspace_id, teammate).await?;
    probe.pause.store(true, Ordering::SeqCst);
    let state = editor.clone();
    let second_client = Uuid::new_v4();
    let running = tokio::spawn(async move {
        service::send_message(
            &state,
            session,
            SendMessage {
                content: "[FICTIF] Approfondis la seconde option.".into(),
                client_message_id: second_client,
            },
        )
        .await
    });
    tokio::time::timeout(Duration::from_secs(10), probe.started.notified()).await?;
    let late = insert_message(&owner, session, "[FICTIF] LATE SENTINEL AFTER SNAPSHOT").await?;
    probe.release.notify_one();
    let second = tokio::time::timeout(Duration::from_secs(10), running).await???;
    let captured = probe.inputs.lock().unwrap()[1].clone();
    assert_eq!(captured.conversation.messages.len(), 2);
    assert_eq!(
        captured
            .conversation
            .messages
            .iter()
            .map(|m| m.message_public_id)
            .collect::<Vec<_>>(),
        first_ids
    );
    assert_eq!(
        captured.conversation.messages[0].author_actor_id,
        Some(owner.actor_id)
    );
    assert_eq!(captured.conversation.messages[0].role, "user");
    assert_eq!(captured.conversation.messages[1].role, "assistant");
    assert!(
        captured.conversation.messages[1]
            .content
            .contains("Option deux : vert")
    );
    assert!(captured.user_message.contains("seconde option"));
    let encoded = serde_json::to_string(&captured.conversation)?;
    for forbidden in [
        "NEIGHBOR PRIVATE",
        "FOREIGN PRIVATE",
        "LATE SENTINEL",
        "seconde option",
    ] {
        assert!(!encoded.contains(forbidden));
    }
    assert!(second.messages.iter().any(|m| m.public_id == late));
    let response = second
        .messages
        .iter()
        .find(|m| m.role == "assistant" && m.client_message_id == Some(second_client))
        .context("second response")?;
    let provenance = &response.metadata["conversation_context"];
    assert_eq!(provenance["message_public_ids"], json!(first_ids));
    assert_eq!(provenance["included_messages"], 2);
    assert_eq!(provenance["omitted_messages"], 0);
    assert_eq!(provenance["trust"], "unconfirmed_conversation");
    assert_eq!(
        provenance["current_message_public_id"],
        json!(captured.conversation.current_message_public_id)
    );
    assert!(!provenance.to_string().contains("couleurs"));
    let mut tx = owner.pool.begin().await?;
    sqlx::query("select set_config('app.current_actor_id',$1,true),set_config('app.current_workspace_id',$2,true),set_config('app.current_workspace_role','owner',true)")
        .bind(owner.actor_id.to_string()).bind(owner.workspace_internal_id.context("scope")?.to_string()).execute(&mut *tx).await?;
    let run_id: Uuid = serde_json::from_value(response.metadata["model_run_public_id"].clone())?;
    let (hash, sources): (String, Vec<Uuid>) = sqlx::query_as(
        "select input_hash,source_public_ids from app.model_runs where public_id=$1",
    )
    .bind(run_id)
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;
    let fingerprint = json!({"scope_kind": captured.scope_kind, "instructions": captured.instructions,
        "user_message": captured.user_message, "context": captured.context, "conversation": captured.conversation});
    assert_eq!(
        hash,
        format!("{:x}", Sha256::digest(serde_json::to_vec(&fingerprint)?))
    );
    assert!(first_ids.iter().all(|id| !sources.contains(id)));
    // Conversation identities must not become authorized confirmed-source IDs.
    probe.pause.store(false, Ordering::SeqCst);
    probe.cite_message.store(true, Ordering::SeqCst);
    assert!(matches!(
        service::send_message(
            &owner,
            session,
            SendMessage {
                content: "[FICTIF] Cite notre premier échange comme une règle validée.".into(),
                client_message_id: Uuid::new_v4(),
            }
        )
        .await,
        Err(AppError::Agent(_))
    ));
    let calls = probe.inputs.lock().unwrap().len();
    assert!(
        service::send_message(
            &foreign,
            session,
            SendMessage {
                content: "[FICTIF] Lecture interdite".into(),
                client_message_id: Uuid::new_v4(),
            }
        )
        .await
        .is_err()
    );
    assert_eq!(probe.inputs.lock().unwrap().len(), calls);
    Ok(())
}

#[tokio::test]
#[allow(clippy::too_many_lines)] // One retry across a deliberately delayed transaction commit.
async fn conversation_retry_reuses_the_durable_snapshot_after_older_message_commits() -> Result<()>
{
    tokio::time::timeout(Duration::from_secs(30), async {
    let actor = isolated_actor().await?;
    let mut owner = create(&actor, Uuid::new_v4()).await?;
    let (_, session) = ready_project(&owner, "[FICTIF] Reprise fidèle").await?;
    let initial = insert_message(&owner, session, "[FICTIF] Contexte initial visible").await?;
    // Reserve an older identity without holding message FK KEY SHARE locks:
    // inserting before send_message would block its project FOR UPDATE lock.
    // A transaction that allocated this identity earlier can still insert and
    // commit it later, so an id cutoff alone remains insufficient on retry.
    let delayed_id: i64 = sqlx::query_scalar(
        "select nextval(pg_get_serial_sequence('app.messages','id'))",
    )
    .fetch_one(&owner.pool)
    .await?;
    let probe = Arc::new(ConversationProbe::default());
    probe.fail_respond.store(true, Ordering::SeqCst);
    owner.engine = probe.clone();
    let client = Uuid::new_v4();
    assert!(
        service::send_message(
            &owner,
            session,
            SendMessage {
                content: "[FICTIF] Continue sur cette base.".into(),
                client_message_id: client,
            }
        )
        .await
        .is_err()
    );
    let before = probe.inputs.lock().unwrap()[0].conversation.clone();
    assert_eq!(before.messages.len(), 1);
    assert_eq!(before.messages[0].message_public_id, initial);
    let mut pending = owner.pool.begin().await?;
    sqlx::query("select set_config('app.current_actor_id',$1,true),set_config('app.current_workspace_id',$2,true),set_config('app.current_workspace_role','owner',true),set_config('lock_timeout','5s',true)")
        .bind(owner.actor_id.to_string()).bind(owner.workspace_internal_id.context("scope")?.to_string()).execute(&mut *pending).await?;
    let current_id: i64 = sqlx::query_scalar("select id from app.messages where public_id=$1")
        .bind(before.current_message_public_id.context("current message")?).fetch_one(&mut *pending).await?;
    assert!(delayed_id < current_id, "the late message must sort before the captured turn");
    let delayed: Uuid = sqlx::query_scalar("insert into app.messages(id,workspace_id,project_id,session_id,role,content,author_actor_id,client_message_id) overriding system value
        select $3,workspace_id,project_id,id,'user','[FICTIF] OLDER COMMITTED LATE SENTINEL',app.current_actor_id(),$2
        from app.sessions where public_id=$1 returning public_id")
        .bind(session).bind(Uuid::new_v4()).bind(delayed_id).fetch_one(&mut *pending).await?;
    pending.commit().await?;
    probe.fail_respond.store(false, Ordering::SeqCst);
    let response = service::send_message(
        &owner,
        session,
        SendMessage {
            content: "[FICTIF] Continue sur cette base.".into(),
            client_message_id: client,
        },
    )
    .await?;
    let after = probe.inputs.lock().unwrap()[1].conversation.clone();
    assert_eq!(
        serde_json::to_value(&before)?,
        serde_json::to_value(&after)?
    );
    assert!(response.messages.iter().any(|m| m.public_id == delayed));
    assert!(
        after
            .messages
            .iter()
            .all(|m| m.message_public_id != delayed)
    );
    assert!(
        response
            .messages
            .iter()
            .all(|m| m.metadata.get("conversation_snapshot").is_none())
    );
    // Legacy pending messages have no durable snapshot: an explicit new turn
    // is required, never a silently recaptured and misleading retry.
    let legacy = insert_message(&owner, session, "[FICTIF] Message ancien sans capture").await?;
    let view = service::session_view(&owner, session).await?;
    let legacy_client = view
        .messages
        .iter()
        .find(|m| m.public_id == legacy)
        .and_then(|m| m.client_message_id)
        .context("legacy client identity")?;
    let calls = probe.inputs.lock().unwrap().len();
    assert!(
        matches!(service::send_message(&owner, session, SendMessage {
        content: "[FICTIF] Message ancien sans capture".into(), client_message_id: legacy_client,
    }).await, Err(AppError::Conflict(message)) if message.contains("ancien message"))
    );
    assert_eq!(probe.inputs.lock().unwrap().len(), calls);
    Ok(())
    }).await.context("conversation snapshot retry exceeded 30 seconds")?
}
