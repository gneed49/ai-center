//! [FICTIF] Exercise the real handoff against a company-first finalization lock.
use super::*;
use ai_center_server::models::CreateHandoff;
use std::time::Duration;

#[tokio::test]
async fn handoff_never_holds_project_while_waiting_for_company_scope() -> Result<()> {
    let actor = isolated_actor().await?;
    let owner = create(&actor, Uuid::new_v4()).await?;
    let (project, session) = ready_project(&owner, "[FICTIF] Ordre partagé").await?;
    let company = company::overview(&owner)
        .await?
        .company_scope
        .context("company scope")?
        .project_public_id;
    fixture_knowledge(
        &owner,
        company,
        "business_rule",
        "[FICTIF] La validation humaine reste obligatoire.",
    )
    .await?;
    let pack = compile_for(&owner, project, session, "tech").await?;
    let admin = erasure::isolated_admin().await?;
    let mut finalizer = admin.begin().await?;
    let blocker: i32 = sqlx::query_scalar("select pg_backend_pid()")
        .fetch_one(&mut *finalizer)
        .await?;
    sqlx::query("select id from app.projects where public_id=$1 for update")
        .bind(company)
        .execute(&mut *finalizer)
        .await?;
    let state = owner.clone();
    let pending = tokio::spawn(async move {
        service::create_handoff(
            &state,
            project,
            CreateHandoff {
                source_session_id: session,
                context_pack_id: pack.public_id,
            },
        )
        .await
    });
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let waiting: bool = sqlx::query_scalar(
                "select exists(select 1 from pg_stat_activity where $1=any(pg_blocking_pids(pid)))",
            )
            .bind(blocker)
            .fetch_one(&admin)
            .await?;
            if waiting {
                return Ok::<(), anyhow::Error>(());
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .context("handoff must actually wait for the company scope")??;
    // This is the next lock used by the compiler/chat's ordered scope finalizer.
    // The previous implementation held it already and timed out/deadlocked here.
    sqlx::query("set local lock_timeout='500ms'")
        .execute(&mut *finalizer)
        .await?;
    sqlx::query("select id from app.projects where public_id=$1 for update")
        .bind(project)
        .execute(&mut *finalizer)
        .await?;
    finalizer.commit().await?;
    tokio::time::timeout(Duration::from_secs(5), pending).await???;
    owner.pool.close().await;
    admin.close().await;
    Ok(())
}
