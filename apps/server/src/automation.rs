//! Shared company limits and cancellation, independent of the selected model.
use crate::{
    agent::{
        AgentEngine, AgentInput, ContextSelectionDraft, ContextSelectionInput,
        CoverageEvaluationDraft, CoverageEvaluationInput, EngineOutput, StewardInput,
        StewardOutput, TechnicalPlanDraft, TechnicalPlanInput,
    },
    error::{AppError, AppResult},
    idempotency::IdempotencyLease,
    models::AgentTurn,
    service::AppState,
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{future::Future, sync::Arc, time::Duration};
use uuid::Uuid;

#[derive(Clone, Copy, Serialize)]
pub struct Limits {
    pub calls_per_hour: i64,
    pub concurrent_calls: i64,
    pub per_actor_calls_per_hour: i64,
    pub per_actor_concurrent_calls: i64,
    pub call_timeout_seconds: u64,
}
impl Limits {
    fn configured() -> Self {
        fn bounded(name: &str, default: u64, min: u64, max: u64) -> u64 {
            std::env::var(name)
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(default)
                .clamp(min, max)
        }
        Self {
            calls_per_hour: i64::try_from(bounded("AI_CENTER_AI_CALLS_PER_HOUR", 120, 1, 10000))
                .unwrap_or(120),
            concurrent_calls: i64::try_from(bounded("AI_CENTER_AI_CONCURRENT_CALLS", 4, 1, 32))
                .unwrap_or(4),
            per_actor_calls_per_hour: i64::try_from(bounded(
                "AI_CENTER_AI_ACTOR_CALLS_PER_HOUR",
                60,
                1,
                10000,
            ))
            .unwrap_or(60),
            per_actor_concurrent_calls: i64::try_from(bounded(
                "AI_CENTER_AI_ACTOR_CONCURRENT_CALLS",
                2,
                1,
                32,
            ))
            .unwrap_or(2),
            call_timeout_seconds: bounded("AI_CENTER_AI_CALL_TIMEOUT_SECONDS", 120, 10, 600),
        }
    }
}
#[derive(Serialize)]
pub struct Status {
    pub enabled: bool,
    pub generation: i64,
    pub limits: Limits,
    pub calls_last_hour: i64,
    pub active_calls: i64,
    pub actor_calls_last_hour: i64,
    pub actor_active_calls: i64,
    pub known_estimated_cost_usd: Option<f64>,
    pub runs_without_cost: i64,
}
#[derive(Serialize)]
pub struct ControlReceipt {
    pub enabled: bool,
    pub generation: i64,
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SetControl {
    pub enabled: bool,
    pub expected_generation: i64,
}

async fn control(state: &AppState, executing: bool) -> AppResult<(bool, i64)> {
    let mut tx = state.begin_request().await?;
    let allowed = if executing {
        vec!["owner", "editor"]
    } else {
        vec!["owner", "editor", "viewer"]
    };
    let member: bool =
        sqlx::query_scalar("select app.has_workspace_role(app.current_workspace_id(),$1::text[])")
            .bind(allowed)
            .fetch_one(&mut *tx)
            .await?;
    if !member {
        return Err(AppError::Forbidden);
    }
    let value=sqlx::query_as("select enabled,generation from app.workspace_automation_controls where workspace_id=app.current_workspace_id()").fetch_optional(&mut *tx).await?.unwrap_or((true,0));
    tx.commit().await?;
    Ok(value)
}
/// Reports whether the company's automation is currently enabled.
/// # Errors
/// Returns withdrawn membership or database failure.
pub async fn enabled(state: &AppState) -> AppResult<bool> {
    Ok(control(state, false).await?.0)
}
/// Rechecks write permission and returns the control generation for an active job.
/// # Errors
/// Returns withdrawn or downgraded membership and database failures.
pub async fn execution_control(state: &AppState) -> AppResult<(bool, i64)> {
    control(state, true).await
}
/// Returns current controls, bounded counters and explicitly partial costs.
/// # Errors
/// Returns withdrawn membership or database failure.
pub async fn status(state: &AppState) -> AppResult<Status> {
    let (enabled, generation) = control(state, false).await?;
    let mut tx = state.begin_request().await?;
    let (calls_last_hour,active_calls,actor_calls_last_hour,actor_active_calls):(i64,i64,i64,i64)=sqlx::query_as("select count(*),count(*) filter(where status='running' and lease_until>now()),count(*) filter(where actor_id=app.current_actor_id()),count(*) filter(where actor_id=app.current_actor_id() and status='running' and lease_until>now()) from app.ai_call_reservations where workspace_id=app.current_workspace_id() and created_at>now()-interval '1 hour'").fetch_one(&mut *tx).await?;
    let (known_estimated_cost_usd,runs_without_cost):(Option<f64>,i64)=sqlx::query_as("select sum(estimated_cost)::float8,count(*) filter(where estimated_cost is null) from app.model_runs where workspace_id=app.current_workspace_id() and created_at>now()-interval '1 hour'").fetch_one(&mut *tx).await?;
    tx.commit().await?;
    Ok(Status {
        enabled,
        generation,
        limits: Limits::configured(),
        calls_last_hour,
        active_calls,
        actor_calls_last_hour,
        actor_active_calls,
        known_estimated_cost_usd,
        runs_without_cost,
    })
}
/// Changes the company's automation control with a compare-and-swap generation.
/// # Errors
/// Returns insufficient rights, a stale command or a database error.
pub async fn set(
    state: &AppState,
    input: SetControl,
    lease: Option<&IdempotencyLease>,
) -> AppResult<ControlReceipt> {
    if state.workspace_role != "owner" {
        return Err(AppError::Forbidden);
    }
    let mut tx = state.begin_request().await?;
    sqlx::query("select pg_advisory_xact_lock(hashtextextended('ai-automation:'||app.current_workspace_id()::text,0))").execute(&mut *tx).await?;
    let current:i64=sqlx::query_scalar("select generation from app.workspace_automation_controls where workspace_id=app.current_workspace_id()").fetch_optional(&mut *tx).await?.unwrap_or(0);
    if current != input.expected_generation {
        return Err(AppError::Conflict(
            "Le réglage a changé. Rechargez son état avant de recommencer.".into(),
        ));
    }
    sqlx::query("insert into app.workspace_automation_controls(workspace_id,enabled,updated_by_actor_id) values(app.current_workspace_id(),$1,app.current_actor_id()) on conflict(workspace_id) do update set enabled=excluded.enabled,generation=app.workspace_automation_controls.generation+1,updated_by_actor_id=excluded.updated_by_actor_id,updated_at=now()")
        .bind(input.enabled).execute(&mut *tx).await?;
    sqlx::query("insert into app.audit_events(workspace_id,actor_id,action,object_kind,object_public_id,after_state) values(app.current_workspace_id(),app.current_actor_id(),'automation.control_changed','workspace',$1,$2)").bind(state.workspace_id).bind(json!({"enabled":input.enabled,"generation":current+1})).execute(&mut *tx).await?;
    // A stable compact receipt is completed in the same transaction as the switch.
    let value = ControlReceipt {
        enabled: input.enabled,
        generation: current + 1,
    };
    crate::company::complete(&mut tx, lease, &value).await?;
    tx.commit().await?;
    Ok(value)
}

struct ControlledEngine {
    state: AppState,
    inner: Arc<dyn AgentEngine>,
    limits: Limits,
}
/// Wraps every selected engine with the same durable company limits.
#[must_use]
pub fn wrap(state: &AppState, inner: Arc<dyn AgentEngine>) -> Arc<dyn AgentEngine> {
    Arc::new(ControlledEngine {
        state: state.clone(),
        inner,
        limits: Limits::configured(),
    })
}
impl ControlledEngine {
    async fn reserve(&self, operation: &str) -> AppResult<(Uuid, i64)> {
        let mut tx = self.state.begin_request().await?;
        sqlx::query("select pg_advisory_xact_lock(hashtextextended('ai-automation:'||app.current_workspace_id()::text,0))").execute(&mut *tx).await?;
        let member: bool = sqlx::query_scalar(
            "select app.has_workspace_role(app.current_workspace_id(),array['owner','editor'])",
        )
        .fetch_one(&mut *tx)
        .await?;
        if !member {
            return Err(AppError::Forbidden);
        }
        let (enabled,generation):(bool,i64)=sqlx::query_as("select enabled,generation from app.workspace_automation_controls where workspace_id=app.current_workspace_id()").fetch_optional(&mut *tx).await?.unwrap_or((true,0));
        if !enabled {
            return Err(AppError::AutomationPaused);
        }
        let (hour,active,actor_hour,actor_active):(i64,i64,i64,i64)=sqlx::query_as("select count(*),count(*) filter(where status='running' and lease_until>now()),count(*) filter(where actor_id=app.current_actor_id()),count(*) filter(where actor_id=app.current_actor_id() and status='running' and lease_until>now()) from app.ai_call_reservations where workspace_id=app.current_workspace_id() and created_at>now()-interval '1 hour'").fetch_one(&mut *tx).await?;
        let hourly_full = hour >= self.limits.calls_per_hour
            || actor_hour >= self.limits.per_actor_calls_per_hour;
        if hourly_full
            || active >= self.limits.concurrent_calls
            || actor_active >= self.limits.per_actor_concurrent_calls
        {
            return Err(AppError::Capacity {
                retry_after_seconds: if hourly_full { 3600 } else { 5 },
            });
        }
        let id = Uuid::new_v4();
        sqlx::query("insert into app.ai_call_reservations(public_id,workspace_id,actor_id,operation,control_generation,lease_until) values($1,app.current_workspace_id(),app.current_actor_id(),$2,$3,now()+make_interval(secs=>$4))").bind(id).bind(operation).bind(generation).bind(f64::from(u32::try_from(self.limits.call_timeout_seconds + 5).unwrap_or(125))).execute(&mut *tx).await?;
        tx.commit().await?;
        Ok((id, generation))
    }
    async fn run<T: Send, F: Future<Output = AppResult<T>> + Send>(
        &self,
        operation: &str,
        future: F,
    ) -> AppResult<T> {
        let (id, generation) = self.reserve(operation).await?;
        let mut future = Box::pin(future);
        let deadline = tokio::time::sleep(Duration::from_secs(self.limits.call_timeout_seconds));
        tokio::pin!(deadline);
        // Monitor and transport stay concurrently polled. Awaiting a database
        // check inside a select branch would stop the provider future itself,
        // deadlocking providers that need the same one-connection test pool.
        let monitor = async {
            loop {
                tokio::time::sleep(Duration::from_millis(500)).await;
                match tokio::time::timeout(Duration::from_secs(2), control(&self.state, true)).await
                {
                    Ok(Ok((true, current))) if current == generation => {}
                    Ok(Err(error)) => return error,
                    Ok(Ok(_)) => return AppError::AutomationPaused,
                    Err(_) => return AppError::AutomationInterrupted,
                }
            }
        };
        let result = tokio::select! {
            biased;
            () = &mut deadline => Err(AppError::AutomationInterrupted),
            error = monitor => Err(error),
            result = &mut future => result,
        };
        // Drop an unfinished transport before settlement, releasing any held
        // resource (including a local subscription process or test connection).
        drop(future);
        let status = match &result {
            Ok(_) => "completed",
            Err(AppError::AutomationPaused | AppError::Forbidden) => "cancelled",
            Err(_) => "failed",
        };
        // Narrow settlement works even after membership revocation. It can
        // only close this actor's reservation, never create a new call.
        if let Ok(mut tx) = self.state.begin_request().await
            && sqlx::query("select app.finish_ai_call_reservation($1,$2)")
                .bind(id)
                .bind(status)
                .execute(&mut *tx)
                .await
                .is_ok()
        {
            let _ = tx.commit().await;
        }
        result
    }
}
#[async_trait]
impl AgentEngine for ControlledEngine {
    fn provider_name(&self) -> &'static str {
        self.inner.provider_name()
    }
    fn requested_model(&self) -> &str {
        self.inner.requested_model()
    }
    async fn respond(&self, input: AgentInput) -> AppResult<EngineOutput<AgentTurn>> {
        self.run("respond", self.inner.respond(input)).await
    }
    async fn generate_artifact(
        &self,
        input: crate::artifacts::generation_contract::ArtifactGenerationInput,
    ) -> AppResult<EngineOutput<crate::artifacts::generation_contract::ArtifactDraft>> {
        self.run("artifact_draft", self.inner.generate_artifact(input))
            .await
    }
    async fn select_context(
        &self,
        input: ContextSelectionInput,
    ) -> AppResult<EngineOutput<ContextSelectionDraft>> {
        self.run("select_context", self.inner.select_context(input))
            .await
    }
    async fn generate_technical_plan(
        &self,
        input: TechnicalPlanInput,
    ) -> AppResult<EngineOutput<TechnicalPlanDraft>> {
        self.run("technical_plan", self.inner.generate_technical_plan(input))
            .await
    }
    async fn evaluate_coverage(
        &self,
        input: CoverageEvaluationInput,
    ) -> AppResult<EngineOutput<CoverageEvaluationDraft>> {
        self.run("coverage", self.inner.evaluate_coverage(input))
            .await
    }
    async fn analyze_contradictions(
        &self,
        input: StewardInput,
    ) -> AppResult<EngineOutput<StewardOutput>> {
        self.run("steward", self.inner.analyze_contradictions(input))
            .await
    }
}

#[cfg(test)]
mod tests;
