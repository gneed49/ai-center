//! Synthetic source-frontier acceptance; requires the guarded integration DB.
use super::*;
use ai_center_server::{
    agent::{
        AgentEngine, AgentInput, ContextSelectionDraft, ContextSelectionInput,
        CoverageEvaluationDraft, CoverageEvaluationInput, EngineOutput, StewardInput,
        StewardOutput, TechnicalPlanDraft, TechnicalPlanInput,
    },
    error::AppResult,
    models::AgentTurn,
    steward::{self, StewardConfig},
};
use async_trait::async_trait;
use std::time::Duration;

#[derive(Default)]
struct PausedSteward {
    started: tokio::sync::Notify,
    release: tokio::sync::Notify,
}
#[async_trait]
impl AgentEngine for PausedSteward {
    fn provider_name(&self) -> &'static str {
        "deterministic"
    }
    fn requested_model(&self) -> &'static str {
        "paused-steward"
    }
    async fn respond(&self, input: AgentInput) -> AppResult<EngineOutput<AgentTurn>> {
        DeterministicEngine.respond(input).await
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
        self.started.notify_one();
        tokio::time::timeout(Duration::from_secs(10), self.release.notified())
            .await
            .map_err(|_| AppError::Internal("steward test release timed out".into()))?;
        DeterministicEngine.analyze_contradictions(input).await
    }
}

async fn scoped_tx(owner: &AppState) -> Result<sqlx::Transaction<'_, sqlx::Postgres>> {
    let mut tx = owner.pool.begin().await?;
    sqlx::query("select set_config('app.current_actor_id',$1,true),set_config('app.current_workspace_id',$2,true),set_config('app.current_workspace_role',$3,true)")
        .bind(owner.actor_id.to_string()).bind(owner.workspace_internal_id.context("workspace")?.to_string()).bind(&owner.workspace_role).execute(&mut *tx).await?;
    Ok(tx)
}
async fn project(owner: &AppState, name: &str) -> Result<Uuid> {
    Ok(service::create_project(
        owner,
        CreateProject {
            name: format!("[FICTIF] {name}"),
            objective: "Sources synthétiques de validation".into(),
        },
    )
    .await?
    .public_id)
}
async fn runs(owner: &AppState) -> Result<i64> {
    let mut tx = scoped_tx(owner).await?;
    let count=sqlx::query_scalar("select count(*) from app.model_runs where workspace_id=app.current_workspace_id() and operation='assess_contradiction'").fetch_one(&mut *tx).await?;
    tx.commit().await?;
    Ok(count)
}

#[tokio::test]
async fn old_company_rule_reaches_a_late_project_and_unrelated_sources_do_not_spend_calls()
-> Result<()> {
    tokio::time::timeout(Duration::from_secs(900), complete_large_frontier())
        .await
        .context("the complete 132-source frontier exceeded the 15-minute integration limit")?
}

#[allow(clippy::too_many_lines)] // Complete bounded frontier, restart and access lifecycle.
async fn complete_large_frontier() -> Result<()> {
    let actor = isolated_actor().await?;
    let owner = create(&actor, Uuid::new_v4()).await?;
    let foreign = create(&actor, Uuid::new_v4()).await?;
    let company = company::overview(&owner)
        .await?
        .company_scope
        .context("company")?
        .project_public_id;
    let (_, rule) = fixture_knowledge(
        &owner,
        company,
        "business_rule",
        "[FICTIF] Quartzretention conserve les décisions sans expiration.",
    )
    .await?;
    let busy = project(&owner, "Projet chargé").await?;
    for n in 0..130 {
        fixture_knowledge(
            &owner,
            busy,
            "decision",
            &format!("[FICTIF] Brouillard{n}."),
        )
        .await?;
    }
    let other = project(&owner, "Implémentation tardive").await?;
    let (_, implementation) = fixture_knowledge(
        &owner,
        other,
        "technical_rule",
        "[FICTIF] Quartzretention purge les décisions après 90 jours.",
    )
    .await?;
    let first = steward::analyze_project(&owner, busy, StewardConfig::default()).await?;
    assert_eq!(first.candidate_pair_count, 1);
    let detail = service::insight_detail(&owner, first.insight_public_ids[0]).await?;
    assert!(
        detail
            .sources
            .iter()
            .any(|s| s.version_public_id == Some(rule))
    );
    assert!(
        detail
            .sources
            .iter()
            .any(|s| s.version_public_id == Some(implementation))
    );
    assert_eq!(runs(&owner).await?, 1);
    // Fresh process/state objects resume the ledger; no source-count window
    // or event replay is allowed to rewind already inspected versions.
    let resumed = select(&actor, owner.workspace_id, owner.actor_id).await?;
    for _ in 0..132 {
        steward::analyze_project(&resumed, busy, StewardConfig::default()).await?;
    }
    assert_eq!(
        runs(&owner).await?,
        1,
        "existing pairs and unrelated text require no model call"
    );
    let status = steward::company_progress(&owner).await?;
    assert_eq!(status["available_sources"], 132);
    assert_eq!(status["pending_sources"], 0);
    assert_eq!(status["progress"]["status"], "idle");
    assert_eq!(status["exhaustive"], false);
    assert_eq!(
        steward::company_progress(&foreign).await?["available_sources"],
        0
    );
    let viewer = Uuid::new_v4();
    add_member(&owner, viewer, "viewer").await?;
    let viewer = select(&actor, owner.workspace_id, viewer).await?;
    assert_eq!(
        steward::company_progress(&viewer).await?["pending_sources"],
        0
    );
    assert!(matches!(
        steward::analyze_project(&viewer, busy, StewardConfig::default()).await,
        Err(AppError::Forbidden)
    ));
    let mut tx = scoped_tx(&owner).await?;
    let continuation:bool=sqlx::query_scalar("select exists(select 1 from app.domain_events where workspace_id=app.current_workspace_id() and event_type='steward.continue' and requested_by_actor_id=app.current_actor_id())").fetch_one(&mut *tx).await?;
    assert!(continuation);
    tx.commit().await?;
    Ok(())
}

#[tokio::test]
async fn new_frontiers_pass_48_pairs_without_repeating_them_and_stop_at_the_steward_budget()
-> Result<()> {
    let actor = isolated_actor().await?;
    let owner = create(&actor, Uuid::new_v4()).await?;
    let project = project(&owner, "Nombreux voisins").await?;
    for n in 0..70 {
        fixture_knowledge(
            &owner,
            project,
            "decision",
            &format!("[FICTIF] Quartzbudget partagé numéro {n}."),
        )
        .await?;
    }
    for _ in 0..6 {
        let run = steward::analyze_project(&owner, project, StewardConfig::default()).await?;
        assert!(run.candidate_pair_count <= 12);
    }
    let mut tx = scoped_tx(&owner).await?;
    let (total,distinct):(i64,i64)=sqlx::query_as("select count(*),count(distinct fingerprint) from app.steward_assessments where workspace_id=app.current_workspace_id()").fetch_one(&mut *tx).await?;
    assert!(
        total > 48,
        "the first 48 pairs cannot permanently monopolize analysis"
    );
    assert_eq!(total, distinct);
    tx.commit().await?;
    assert!(matches!(
        steward::analyze_project(&owner, project, StewardConfig::default()).await,
        Err(AppError::Capacity { .. })
    ));
    assert_eq!(runs(&owner).await?, 6);
    let status = steward::company_progress(&owner).await?;
    assert!(status["pending_sources"].as_i64().context("pending")? > 0);
    assert!(status["omitted_neighbors"].as_i64().context("omitted")? > 0);
    assert_eq!(status["max_provider_calls_per_hour"], 6);
    Ok(())
}

#[tokio::test]
async fn active_company_lease_defers_without_provider_and_expired_lease_can_resume() -> Result<()> {
    let actor = isolated_actor().await?;
    let owner = create(&actor, Uuid::new_v4()).await?;
    let project = project(&owner, "Bail").await?;
    fixture_knowledge(
        &owner,
        project,
        "business_rule",
        "[FICTIF] Quartzlease conservé sans expiration.",
    )
    .await?;
    fixture_knowledge(
        &owner,
        project,
        "technical_rule",
        "[FICTIF] Quartzlease purgé après 90 jours.",
    )
    .await?;
    let mut tx = scoped_tx(&owner).await?;
    sqlx::query("insert into app.steward_scan_progress(workspace_id,status,lease_token,lease_until) values(app.current_workspace_id(),'running',$1,clock_timestamp()+interval '1 minute')").bind(Uuid::new_v4()).execute(&mut *tx).await?;
    tx.commit().await?;
    assert!(matches!(
        steward::analyze_project(&owner, project, StewardConfig::default()).await,
        Err(AppError::Capacity { .. })
    ));
    assert_eq!(runs(&owner).await?, 0);
    let mut tx = scoped_tx(&owner).await?;
    sqlx::query("update app.steward_scan_progress set lease_until=clock_timestamp()-interval '1 second' where workspace_id=app.current_workspace_id()").execute(&mut *tx).await?;
    tx.commit().await?;
    assert_eq!(
        steward::analyze_project(&owner, project, StewardConfig::default())
            .await?
            .candidate_pair_count,
        1
    );
    assert_eq!(runs(&owner).await?, 1);
    Ok(())
}

#[tokio::test]
async fn expired_worker_cannot_commit_or_release_a_new_workers_lease() -> Result<()> {
    let actor = isolated_actor().await?;
    let owner = create(&actor, Uuid::new_v4()).await?;
    let project = project(&owner, "Fencing concurrent").await?;
    for (kind, statement) in [
        (
            "business_rule",
            "[FICTIF] Quartzfencing conservé sans expiration.",
        ),
        (
            "technical_rule",
            "[FICTIF] Quartzfencing purgé après 90 jours.",
        ),
    ] {
        fixture_knowledge(&owner, project, kind, statement).await?;
    }
    let old_engine = Arc::new(PausedSteward::default());
    let mut old_owner = owner.clone();
    old_owner.engine = old_engine.clone();
    let old_worker = tokio::spawn(async move {
        steward::analyze_project(&old_owner, project, StewardConfig::default()).await
    });
    tokio::time::timeout(Duration::from_secs(10), old_engine.started.notified()).await?;
    let mut tx = scoped_tx(&owner).await?;
    sqlx::query("update app.steward_scan_progress set lease_until=clock_timestamp()-interval '1 second' where workspace_id=app.current_workspace_id()").execute(&mut *tx).await?;
    tx.commit().await?;
    let new_engine = Arc::new(PausedSteward::default());
    let mut new_owner = owner.clone();
    new_owner.engine = new_engine.clone();
    let new_worker = tokio::spawn(async move {
        steward::analyze_project(&new_owner, project, StewardConfig::default()).await
    });
    tokio::time::timeout(Duration::from_secs(10), new_engine.started.notified()).await?;
    let mut tx = scoped_tx(&owner).await?;
    let new_token:Uuid = sqlx::query_scalar("select lease_token from app.steward_scan_progress where workspace_id=app.current_workspace_id()").fetch_one(&mut *tx).await?;
    tx.commit().await?;
    old_engine.release.notify_one();
    assert!(matches!(old_worker.await?, Err(AppError::Conflict(_))));
    let mut tx = scoped_tx(&owner).await?;
    let actual:Uuid = sqlx::query_scalar("select lease_token from app.steward_scan_progress where workspace_id=app.current_workspace_id() and status='running'").fetch_one(&mut *tx).await?;
    assert_eq!(
        actual, new_token,
        "stale cleanup must not release the new capability"
    );
    let (assessments,receipts,continuations,failed):(i64,i64,i64,i64)=sqlx::query_as("select (select count(*) from app.steward_assessments where workspace_id=app.current_workspace_id()),(select count(*) from app.steward_scan_sources where workspace_id=app.current_workspace_id()),(select count(*) from app.domain_events where workspace_id=app.current_workspace_id() and event_type='steward.continue'),(select count(*) from app.model_runs where workspace_id=app.current_workspace_id() and status='failed')").fetch_one(&mut *tx).await?;
    assert_eq!((assessments, receipts, continuations, failed), (0, 0, 0, 1));
    tx.commit().await?;
    new_engine.release.notify_one();
    assert_eq!(new_worker.await??.assessment_count, 1);
    let mut tx = scoped_tx(&owner).await?;
    let (assessments,receipts,continuations):(i64,i64,i64)=sqlx::query_as("select (select count(*) from app.steward_assessments where workspace_id=app.current_workspace_id()),(select count(*) from app.steward_scan_sources where workspace_id=app.current_workspace_id()),(select count(*) from app.domain_events where workspace_id=app.current_workspace_id() and event_type='steward.continue')").fetch_one(&mut *tx).await?;
    assert_eq!((assessments, receipts, continuations), (1, 1, 1));
    tx.commit().await?;
    Ok(())
}

#[tokio::test]
async fn revoked_actors_pending_continuation_does_not_strand_authorized_progress() -> Result<()> {
    let actor = isolated_actor().await?;
    let owner = create(&actor, Uuid::new_v4()).await?;
    let editor_actor = Uuid::new_v4();
    let editor_member = add_member(&owner, editor_actor, "editor").await?;
    let editor = select(&actor, owner.workspace_id, editor_actor).await?;
    let project = project(&owner, "Reprise autorisée").await?;
    for statement in [
        "[FICTIF] Quartzalpha.",
        "[FICTIF] Quartzbeta.",
        "[FICTIF] Quartzgamma.",
    ] {
        fixture_knowledge(&owner, project, "decision", statement).await?;
    }
    steward::analyze_project(&editor, project, StewardConfig::default()).await?;
    company::update_member(
        &owner,
        editor_member,
        UpdateMember {
            role: None,
            invitation_status: Some("revoked".into()),
        },
        None,
    )
    .await?;
    assert!(matches!(
        steward::analyze_project(&editor, project, StewardConfig::default()).await,
        Err(AppError::Forbidden | AppError::NotFound)
    ));
    steward::analyze_project(&owner, project, StewardConfig::default()).await?;
    let mut tx = scoped_tx(&owner).await?;
    let actors:Vec<Uuid>=sqlx::query_scalar("select requested_by_actor_id from app.domain_events where workspace_id=app.current_workspace_id() and event_type='steward.continue' and status='pending'").fetch_all(&mut *tx).await?;
    assert_eq!(actors.len(), 2);
    assert!(actors.contains(&editor_actor));
    assert!(actors.contains(&owner.actor_id));
    tx.commit().await?;
    assert_eq!(runs(&owner).await?, 0);
    assert_eq!(
        steward::company_progress(&owner).await?["pending_sources"],
        1
    );
    Ok(())
}

#[tokio::test]
async fn distinct_files_of_the_same_verified_corpus_are_comparable() -> Result<()> {
    let actor = isolated_actor().await?;
    let owner = create(&actor, Uuid::new_v4()).await?;
    let project = project(&owner, "Deux fichiers").await?;
    let mut tx = scoped_tx(&owner).await?;
    let internal: i64 = sqlx::query_scalar("select id from app.projects where public_id=$1")
        .bind(project)
        .fetch_one(&mut *tx)
        .await?;
    let connection:i64=sqlx::query_scalar("insert into app.work_tool_connections(public_id,workspace_id,provider,name,encrypted_credential,credential_actor_id) values($1,app.current_workspace_id(),'github','[FICTIF] Non utilisable',$2,app.current_actor_id()) returning id")
        .bind(Uuid::new_v4()).bind(vec![0_u8;32]).fetch_one(&mut *tx).await?;
    let corpus:i64=sqlx::query_scalar("insert into app.github_code_corpora(workspace_id,project_id,connection_id,repository,commit_sha,commit_verified,requested_paths,requested_by_actor_id) values(app.current_workspace_id(),$1,$2,'fictif/retention',$3,true,'[\"README.md\",\"retention.ts\"]',app.current_actor_id()) returning id")
        .bind(internal).bind(connection).bind("a".repeat(40)).fetch_one(&mut *tx).await?;
    for (path, content) in [
        (
            "README.md",
            "[FICTIF] Quartzcode conserve les décisions sans expiration.",
        ),
        (
            "retention.ts",
            "// [FICTIF] Quartzcode purge les décisions après 90 jours.",
        ),
    ] {
        sqlx::query("insert into app.github_code_file_observations(workspace_id,project_id,corpus_id,path,status,blob_sha,content_hash,content_text,line_count) values(app.current_workspace_id(),$1,$2,$3,'code_read',$4,$5,$6,1)")
            .bind(internal).bind(corpus).bind(path).bind("b".repeat(40)).bind("c".repeat(64)).bind(content).execute(&mut *tx).await?;
    }
    tx.commit().await?;
    let result = steward::analyze_project(&owner, project, StewardConfig::default()).await?;
    assert_eq!(result.candidate_pair_count, 1);
    assert_eq!(result.assessment_count, 1);
    let mut tx = scoped_tx(&owner).await?;
    let files:i64=sqlx::query_scalar("select count(distinct source_public_id) from app.steward_scope_sources where workspace_id=app.current_workspace_id() and source_kind='github_code_file_observation'").fetch_one(&mut *tx).await?;
    assert_eq!(files, 2);
    tx.commit().await?;
    Ok(())
}
