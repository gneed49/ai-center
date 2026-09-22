use super::*;
use ai_center_server::{
    artifacts::{self, CreateArtifact, SourceInput, ValidateArtifact},
    automation::{self, SetControl},
    company::data::{self, ArchiveProject},
};
use serde_json::Value;

pub(super) async fn isolated_admin() -> Result<sqlx::PgPool> {
    let raw = std::env::var("AI_CENTER_ADMIN_DATABASE_URL")?;
    let url =
        url::Url::parse(&raw).map_err(|_| anyhow::anyhow!("invalid isolated admin connection"))?;
    ensure!(
        url.scheme() == "postgresql"
            && url.host_str() == Some("127.0.0.1")
            && url.port() == Some(55322)
            && url.username() == "postgres"
            && url.path() == "/postgres"
            && url.query().is_none()
            && url.fragment().is_none(),
        "isolated admin only"
    );
    Ok(PgPoolOptions::new()
        .max_connections(2)
        .connect(&raw)
        .await?)
}
async fn manifest(admin: &sqlx::PgPool, workspace: Uuid) -> Result<Value> {
    Ok(
        sqlx::query_scalar("select app.operator_workspace_manifest($1)")
            .bind(workspace)
            .fetch_one(admin)
            .await?,
    )
}
pub(super) async fn tenant_hash(admin: &sqlx::PgPool, workspace: i64) -> Result<Vec<String>> {
    let names: Vec<String> = sqlx::query_scalar("select unnest(app.operator_purge_tables())")
        .fetch_all(admin)
        .await?;
    let mut hashes = Vec::new();
    for table in names {
        ensure!(
            table.bytes().all(|c| c.is_ascii_lowercase() || c == b'_'),
            "trusted table inventory"
        );
        let key = if table == "workspaces" {
            "id"
        } else {
            "workspace_id"
        };
        hashes.push(sqlx::query_scalar(&format!("select md5(coalesce(jsonb_agg(to_jsonb(t) order by to_jsonb(t)::text)::text,'[]')) from app.{table} t where {key}=$1")).bind(workspace).fetch_one(admin).await?);
    }
    Ok(hashes)
}

#[tokio::test]
#[allow(clippy::too_many_lines)]
async fn operator_erasure_is_tenant_exact_transactional_and_unavailable_to_runtime() -> Result<()> {
    let actor = isolated_actor().await?;
    let owner = create(&actor, Uuid::new_v4()).await?;
    let foreign = create(&actor, Uuid::new_v4()).await?;
    let admin = isolated_admin().await?;
    let (project, session) = ready_project(&owner, "[FICTIF] Effacement ciblé").await?;
    let (source, _) = ready_project(&owner, "[FICTIF] Source liée").await?;
    link_projects(&owner, project, source).await?;
    let pack = compile_for(&owner, project, session, "tech").await?;
    let draft = artifacts::create(
        &owner,
        source,
        CreateArtifact {
            artifact_type: "specification".into(),
            title: "[FICTIF] Cycle document version".into(),
            body_markdown: "Conserver les décisions".into(),
            structured_content: json!({}),
            sources: vec![SourceInput {
                kind: "context_pack".into(),
                public_id: pack.public_id,
            }],
        },
        None,
    )
    .await?;
    artifacts::validate(
        &owner,
        draft.artifact.public_id,
        ValidateArtifact {
            expected_version_id: draft.current_version.public_id,
        },
        None,
    )
    .await?;
    ready_project(&foreign, "[FICTIF] Doit rester intact").await?;
    let other_before = tenant_hash(
        &admin,
        foreign.workspace_internal_id.context("foreign scope")?,
    )
    .await?;
    let global_before: String = sqlx::query_scalar(
        "select md5(jsonb_agg(to_jsonb(t) order by id)::text) from app.project_templates t",
    )
    .fetch_one(&admin)
    .await?;
    let initial = manifest(&admin, owner.workspace_id).await?;
    // An active workspace cannot be erased even by the operator.
    assert!(
        sqlx::query("select app.operator_purge_workspace($1,$1,$2)")
            .bind(owner.workspace_id)
            .bind(initial["receipt"].as_str())
            .execute(&admin)
            .await
            .is_err()
    );
    assert!(
        sqlx::query("select app.operator_workspace_manifest($1)")
            .bind(owner.workspace_id)
            .execute(&owner.pool)
            .await
            .is_err()
    );
    let mut runtime = owner.pool.begin().await?;
    sqlx::query("select set_config('app.operator_purge_workspace_id',$1,true),set_config('app.operator_purge_transaction_id',txid_current()::text,true)").bind(owner.workspace_internal_id.context("scope")?.to_string()).execute(&mut *runtime).await?;
    assert!(
        sqlx::query("select app.operator_purge_workspace($1,$1,$2)")
            .bind(owner.workspace_id)
            .bind(initial["receipt"].as_str())
            .execute(&mut *runtime)
            .await
            .is_err()
    );
    runtime.rollback().await?;
    automation::set(
        &owner,
        SetControl {
            enabled: false,
            expected_generation: 0,
        },
        None,
    )
    .await?;
    data::archive(&owner, project, ArchiveProject { archived: true }, None).await?;
    data::archive(&owner, source, ArchiveProject { archived: true }, None).await?;
    let ready = manifest(&admin, owner.workspace_id).await?;
    let receipt = ready["receipt"].as_str().context("preview receipt")?;
    assert!(
        sqlx::query("select app.operator_purge_workspace($1,$2,$3)")
            .bind(owner.workspace_id)
            .bind(Uuid::new_v4())
            .bind(receipt)
            .execute(&admin)
            .await
            .is_err()
    );
    assert!(
        sqlx::query("select app.operator_purge_workspace($1,$1,$2)")
            .bind(owner.workspace_id)
            .bind(initial["receipt"].as_str())
            .execute(&admin)
            .await
            .is_err()
    );
    // A maintenance rollback restores every row, including immutable histories.
    let mut rollback = admin.begin().await?;
    sqlx::query("select app.operator_purge_workspace($1,$1,$2)")
        .bind(owner.workspace_id)
        .bind(receipt)
        .execute(&mut *rollback)
        .await?;
    rollback.rollback().await?;
    assert_eq!(manifest(&admin, owner.workspace_id).await?, ready);
    // Schema drift fails closed; the test-only table is transactional and rolled back.
    let mut schema_change = admin.begin().await?;
    sqlx::query("create table app.erasure_unreviewed_fixture(workspace_id bigint)")
        .execute(&mut *schema_change)
        .await?;
    assert!(
        sqlx::query("select app.operator_workspace_manifest($1)")
            .bind(owner.workspace_id)
            .execute(&mut *schema_change)
            .await
            .is_err()
    );
    schema_change.rollback().await?;
    let result: Value = sqlx::query_scalar("select app.operator_purge_workspace($1,$1,$2)")
        .bind(owner.workspace_id)
        .bind(receipt)
        .fetch_one(&admin)
        .await?;
    assert_eq!(
        result["workspace_public_id"],
        owner.workspace_id.to_string()
    );
    let exists: bool =
        sqlx::query_scalar("select exists(select 1 from app.workspaces where public_id=$1)")
            .bind(owner.workspace_id)
            .fetch_one(&admin)
            .await?;
    assert!(!exists);
    assert_eq!(
        tenant_hash(
            &admin,
            foreign.workspace_internal_id.context("foreign scope")?
        )
        .await?,
        other_before
    );
    let global_after: String = sqlx::query_scalar(
        "select md5(jsonb_agg(to_jsonb(t) order by id)::text) from app.project_templates t",
    )
    .fetch_one(&admin)
    .await?;
    assert_eq!(global_after, global_before);
    assert!(
        sqlx::query("delete from app.knowledge_entry_versions where workspace_id=$1")
            .bind(foreign.workspace_internal_id)
            .execute(&admin)
            .await
            .is_err(),
        "append-only protection remains active after the maintenance transaction"
    );
    Ok(())
}
