//! At-most-one automatic create attempt; ambiguous outcomes require review.
use super::{client::ToolClient, credential, publications};
use crate::{auth::RequestContext, error::AppResult, service::AppState};
use serde_json::json;
use std::{sync::Arc, time::Duration};
use uuid::Uuid;

#[derive(sqlx::FromRow)]
struct Claim {
    job_id: Uuid,
    workspace_id: i64,
    workspace_public_id: Uuid,
    actor_id: Uuid,
    actor_role: String,
    lease: Uuid,
}

/// Starts the persistent queue supervisor. Aborting it never retries a create.
#[must_use]
pub fn start(state: Arc<AppState>) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let Ok(client) = ToolClient::official() else {
            tracing::error!("publication_worker_client_unavailable");
            return;
        };
        let mut tick = tokio::time::interval(Duration::from_secs(2));
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            tick.tick().await;
            // A bounded pass prevents sustained publications starving shutdown.
            for _ in 0..8 {
                match drain_one(&state, &client).await {
                    Ok(true) => {}
                    Ok(false) => break,
                    Err(_) => {
                        tracing::warn!("publication_worker_pass_failed");
                        break;
                    }
                }
            }
        }
    })
}

/// Claims and settles at most one persisted job, using the same code in tests.
///
/// # Errors
/// Returns database failures; a lost settlement is recovered as `needs_review`.
pub async fn drain_one(state: &AppState, client: &ToolClient) -> AppResult<bool> {
    let claim: Option<Claim> = sqlx::query_as("select * from app.claim_publication_job()")
        .fetch_optional(&state.pool)
        .await?;
    let Some(claim) = claim else {
        return Ok(false);
    };
    let scoped = state.scoped(&RequestContext {
        actor_id: claim.actor_id,
        workspace_id: claim.workspace_public_id,
        workspace_internal_id: Some(claim.workspace_id),
        workspace_role: claim.actor_role,
    });
    let input = load(&scoped, claim.job_id).await;
    let (status, error, receipt) = match input {
        Ok((job, credential, generation)) => {
            let request = client.create(
                &job.provider,
                &credential.secret,
                &job.target_id,
                &job.title,
                &job.body_markdown,
            );
            tokio::pin!(request);
            let outcome = tokio::select! {
                biased;
                outcome=&mut request=>outcome,
                error=monitor_execution(&scoped,generation)=>Err(error),
            };
            match outcome {
                Ok(receipt)
                    if !receipt.complete
                        || receipt
                            .body_markdown
                            .contains(&publications::marker(job.public_id)) =>
                {
                    ("succeeded", None, Some(json!(receipt)))
                }
                Ok(_) => ("needs_review", Some("remote_marker_missing"), None),
                Err(error) => (
                    if error.ambiguous {
                        "needs_review"
                    } else {
                        "failed"
                    },
                    Some(error.code),
                    None,
                ),
            }
        }
        Err(_) => (
            "cancelled",
            Some("authorization_connection_or_version_changed"),
            None,
        ),
    };
    let _: bool = sqlx::query_scalar("select app.finish_publication_job($1,$2,$3,$4,$5)")
        .bind(claim.job_id)
        .bind(claim.lease)
        .bind(status)
        .bind(error)
        .bind(receipt)
        .fetch_one(&state.pool)
        .await?;
    Ok(true)
}
async fn load(
    state: &AppState,
    id: Uuid,
) -> AppResult<(super::models::JobInput, super::models::Credential, i64)> {
    let (enabled, generation) = crate::automation::execution_control(state).await?;
    if !enabled {
        return Err(crate::error::AppError::AutomationPaused);
    }
    let mut tx = state.begin_request().await?;
    let job = publications::job(&mut tx, id).await?;
    let current:bool=sqlx::query_scalar("select exists(select 1 from app.publication_jobs j join app.artifact_document_versions v on v.id=j.artifact_version_id join app.artifact_documents d on d.id=v.document_id where j.public_id=$1 and d.current_version_id=v.id and v.status='validated')")
        .bind(id).fetch_one(&mut *tx).await?;
    tx.commit().await?;
    if !current || job.status != "processing" {
        return Err(crate::error::AppError::Conflict(
            "Publication version changed".into(),
        ));
    }
    let credential = credential(state, job.connection_public_id).await?;
    if credential.revision != job.connection_revision || credential.provider != job.provider {
        return Err(crate::error::AppError::Conflict(
            "Publication connection changed".into(),
        ));
    }
    let (enabled, current) = crate::automation::execution_control(state).await?;
    if !enabled || current != generation {
        return Err(crate::error::AppError::AutomationPaused);
    }
    Ok((job, credential, generation))
}

async fn monitor_execution(state: &AppState, generation: i64) -> super::client::RemoteError {
    loop {
        let permitted = tokio::time::timeout(
            Duration::from_secs(3),
            crate::automation::execution_control(state),
        )
        .await;
        if !matches!(permitted,Ok(Ok((true,current))) if current==generation) {
            return super::client::RemoteError {
                code: "automation_or_permissions_changed_during_request",
                ambiguous: true,
            };
        }
        // This monitor is polled beside the HTTP request; a slow database check
        // must never prevent the remote future from receiving its receipt.
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}
