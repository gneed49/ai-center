use super::*;
use serde_json::Value;

async fn manifest(admin: &sqlx::PgPool, workspace: Uuid) -> Result<Value> {
    Ok(
        sqlx::query_scalar("select app.operator_abandoned_manifest($1)")
            .bind(workspace)
            .fetch_one(admin)
            .await?,
    )
}

#[tokio::test]
#[allow(clippy::too_many_lines)]
async fn offline_closure_requires_stale_work_and_fences_every_abandoned_lease() -> Result<()> {
    let actor = isolated_actor().await?;
    let owner = create(&actor, Uuid::new_v4()).await?;
    let foreign = create(&actor, Uuid::new_v4()).await?;
    let admin = erasure::isolated_admin().await?;
    let workspace = owner.workspace_internal_id.context("scope")?;
    let other_before =
        erasure::tenant_hash(&admin, foreign.workspace_internal_id.context("scope")?).await?;
    assert_eq!(
        manifest(&admin, owner.workspace_id).await?["eligible"],
        false
    );
    sqlx::query("insert into app.workspace_automation_controls(workspace_id,enabled,updated_by_actor_id,updated_at) values($1,false,$2,now()-interval '15 minutes')")
        .bind(workspace).bind(owner.actor_id).execute(&admin).await?;
    let old_generation = Uuid::new_v4();
    let command:Uuid=sqlx::query_scalar("insert into app.idempotency_records(workspace_id,actor_id,operation_key,idempotency_key,request_hash,created_at,updated_at,locked_until,expires_at,lease_generation) values($1,$2,'fictitious.offline',$3,repeat('a',64),now()-interval '25 minutes',now()-interval '25 minutes',now()-interval '20 minutes',now()-interval '1 minute',$4) returning public_id")
        .bind(workspace).bind(owner.actor_id).bind(Uuid::new_v4().to_string()).bind(old_generation).fetch_one(&admin).await?;
    let run:Uuid=sqlx::query_scalar("insert into app.model_runs(workspace_id,operation,provider,model,prompt_version,schema_version,input_hash,status,created_at,updated_at,started_at) values($1,'extract_knowledge','deterministic','[FICTIF]','v1','v1',repeat('b',64),'running',now()-interval '25 minutes',now()-interval '25 minutes',now()-interval '25 minutes') returning public_id")
        .bind(workspace).fetch_one(&admin).await?;
    let reservation = Uuid::new_v4();
    let steward_lease = Uuid::new_v4();
    sqlx::query("insert into app.steward_scan_progress(workspace_id,status,lease_token,lease_until,updated_at) values($1,'running',$2,now()-interval '20 minutes',now()-interval '25 minutes')")
        .bind(workspace).bind(steward_lease).execute(&admin).await?;
    sqlx::query("insert into app.ai_call_reservations(public_id,workspace_id,actor_id,operation,control_generation,created_at,lease_until) values($1,$2,$3,'respond',0,now()-interval '25 minutes',now()-interval '20 minutes')")
        .bind(reservation).bind(workspace).bind(owner.actor_id).execute(&admin).await?;
    let event:Uuid=sqlx::query_scalar("insert into app.domain_events(workspace_id,event_type,aggregate_kind,aggregate_public_id,status,occurred_at,locked_at,locked_until,locked_by,attempt_count) values($1,'fictitious.maintenance','workspace',$2,'processing',now()-interval '25 minutes',now()-interval '25 minutes',now()-interval '20 minutes','fictitious-worker',1) returning public_id")
        .bind(workspace).bind(owner.workspace_id).fetch_one(&admin).await?;
    let publication_lease = Uuid::new_v4();
    let mut publication_fixture = admin.begin().await?;
    let publications: Vec<(Uuid, String)> =
        sqlx::query_as(include_str!("abandoned_publications.sql"))
            .bind(workspace)
            .bind(owner.actor_id)
            .bind(publication_lease)
            .fetch_all(&mut *publication_fixture)
            .await?;
    sqlx::query("update app.artifact_documents d set current_version_id=(select v.id from app.artifact_document_versions v where v.document_id=d.id order by v.version desc limit 1) where d.workspace_id=$1 and d.current_version_id is null")
        .bind(workspace).execute(&mut *publication_fixture).await?;
    publication_fixture.commit().await?;
    let ready = manifest(&admin, owner.workspace_id).await?;
    assert_eq!(ready["eligible"], true, "{ready}");
    assert_eq!(ready["total_operations"], 8);
    // A still-owned command lease blocks the entire transaction.
    let mut active = admin.begin().await?;
    sqlx::query("insert into app.idempotency_records(workspace_id,actor_id,operation_key,idempotency_key,request_hash,created_at,updated_at,locked_until) values($1,$2,'fictitious.active',$3,repeat('c',64),now()-interval '25 minutes',now()-interval '25 minutes',now()+interval '1 minute')")
        .bind(workspace).bind(owner.actor_id).bind(Uuid::new_v4().to_string()).execute(&mut *active).await?;
    let blocked: Value = sqlx::query_scalar("select app.operator_abandoned_manifest($1)")
        .bind(owner.workspace_id)
        .fetch_one(&mut *active)
        .await?;
    assert_eq!(blocked["eligible"], false);
    assert!(
        blocked["blockers"]
            .as_array()
            .context("blockers")?
            .iter()
            .any(|item| item["kind"] == "active_leases")
    );
    active.rollback().await?;
    // New work after the pause also blocks closure, even if already completed.
    let mut recent = admin.begin().await?;
    sqlx::query("insert into app.domain_events(workspace_id,event_type,aggregate_kind,aggregate_public_id) values($1,'fictitious.recent','workspace',$2)")
        .bind(workspace).bind(owner.workspace_id).execute(&mut *recent).await?;
    let blocked: Value = sqlx::query_scalar("select app.operator_abandoned_manifest($1)")
        .bind(owner.workspace_id)
        .fetch_one(&mut *recent)
        .await?;
    assert!(
        blocked["blockers"]
            .as_array()
            .context("blockers")?
            .iter()
            .any(|item| item["kind"] == "activity_after_pause")
    );
    recent.rollback().await?;
    for (confirmation, receipt, stopped) in [
        (
            foreign.workspace_id,
            ready["receipt"].as_str().context("receipt")?,
            true,
        ),
        (owner.workspace_id, "invalid-receipt", true),
        (
            owner.workspace_id,
            ready["receipt"].as_str().context("receipt")?,
            false,
        ),
    ] {
        assert!(
            sqlx::query("select app.operator_close_abandoned($1,$2,$3,$4,$5)")
                .bind(owner.workspace_id)
                .bind(confirmation)
                .bind(receipt)
                .bind(stopped)
                .bind(owner.actor_id)
                .execute(&admin)
                .await
                .is_err()
        );
    }
    assert!(
        sqlx::query("select app.operator_abandoned_manifest($1)")
            .bind(owner.workspace_id)
            .execute(&owner.pool)
            .await
            .is_err()
    );
    let closed: Value = sqlx::query_scalar("select app.operator_close_abandoned($1,$1,$2,true,$3)")
        .bind(owner.workspace_id)
        .bind(ready["receipt"].as_str())
        .bind(owner.actor_id)
        .fetch_one(&admin)
        .await?;
    assert_eq!(closed["remote_outcome"], "unknown");
    assert_eq!(closed["closed_counts"]["idempotency_records"], 1);
    assert_eq!(closed["closed_counts"]["publication_jobs"], 3);
    assert_eq!(closed["closed_counts"]["steward_scan_progress"], 1);
    let progress: (String, Option<Uuid>, Option<chrono::DateTime<chrono::Utc>>) = sqlx::query_as(
        "select status,lease_token,lease_until from app.steward_scan_progress where workspace_id=$1",
    ).bind(workspace).fetch_one(&admin).await?;
    assert_eq!(progress, ("pending".into(), None, None));
    let late_worker = sqlx::query("update app.steward_scan_progress set status='idle',lease_token=null,lease_until=null where workspace_id=$1 and lease_token=$2")
        .bind(workspace).bind(steward_lease).execute(&admin).await?;
    assert_eq!(
        late_worker.rows_affected(),
        0,
        "closed steward lease must fence late settlement"
    );
    for (publication, original_status) in publications {
        let (after, lease): (String, Option<Uuid>) = sqlx::query_as(
            "select status,lease_token from app.publication_jobs where public_id=$1",
        )
        .bind(publication)
        .fetch_one(&admin)
        .await?;
        assert_eq!(
            after,
            if original_status == "queued" {
                "cancelled"
            } else {
                "needs_review"
            }
        );
        assert!(lease.is_none());
        let accepted: bool = sqlx::query_scalar(
            "select app.finish_publication_job($1,$2,'failed','fictitious_late_worker',null)",
        )
        .bind(publication)
        .bind(publication_lease)
        .fetch_one(&admin)
        .await?;
        assert!(
            !accepted,
            "a closed publication must reject a late worker capability"
        );
    }
    let (status,new_generation,body):(String,Uuid,Value)=sqlx::query_as("select status,lease_generation,response_body from app.idempotency_records where public_id=$1").bind(command).fetch_one(&admin).await?;
    assert_eq!(status, "failed");
    assert_ne!(new_generation, old_generation);
    assert_eq!(body["retryable"], false);
    assert_eq!(body["code"], "operator_abandoned_closed");
    let settled:(String,String,String)=sqlx::query_as("select (select status from app.model_runs where public_id=$1),(select status from app.ai_call_reservations where public_id=$2),(select status from app.domain_events where public_id=$3)")
        .bind(run).bind(reservation).bind(event).fetch_one(&admin).await?;
    assert_eq!(
        settled,
        ("cancelled".into(), "cancelled".into(), "dead_letter".into())
    );
    assert_eq!(
        erasure::tenant_hash(&admin, foreign.workspace_internal_id.context("scope")?).await?,
        other_before
    );
    // Reusing the old preview cannot close anything else.
    assert!(
        sqlx::query("select app.operator_close_abandoned($1,$1,$2,true,$3)")
            .bind(owner.workspace_id)
            .bind(ready["receipt"].as_str())
            .bind(owner.actor_id)
            .execute(&admin)
            .await
            .is_err()
    );
    Ok(())
}
