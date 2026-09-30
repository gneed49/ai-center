//! [FICTIF] Context graph and retention invariants against the guarded database.
use super::*;
use ai_center_server::{
    artifacts::{self, CreateArtifact, SourceInput},
    automation::{self, SetControl},
    company::{
        data::{self, ArchiveProject},
        project_data,
    },
};
use serde_json::Value;

async fn scoped(owner: &AppState) -> Result<sqlx::Transaction<'_, sqlx::Postgres>> {
    let mut tx = owner.pool.begin().await?;
    sqlx::query("select set_config('app.current_actor_id',$1,true),set_config('app.current_workspace_id',$2,true),set_config('app.current_workspace_role',$3,true)")
        .bind(owner.actor_id.to_string()).bind(owner.workspace_internal_id.context("scope")?.to_string()).bind(&owner.workspace_role).execute(&mut *tx).await?;
    Ok(tx)
}

async fn fixture(owner: &AppState) -> Result<(Uuid, Uuid, Uuid, Uuid)> {
    let project = service::create_project(
        owner,
        CreateProject {
            name: "[FICTIF] Source existante".into(),
            objective: "[FICTIF] Graphe et rétention sans HTTP".into(),
        },
    )
    .await?
    .public_id;
    let mut tx = scoped(owner).await?;
    let connection: Uuid = sqlx::query_scalar("insert into app.work_tool_connections(public_id,workspace_id,provider,name,encrypted_credential,credential_actor_id,allow_existing_reads) values(gen_random_uuid(),app.current_workspace_id(),'linear','[FICTIF] Aucune clé',convert_to(repeat('FICTIF',8),'UTF8'),app.current_actor_id(),true) returning public_id")
        .fetch_one(&mut *tx).await?;
    let reference: Uuid = sqlx::query_scalar("insert into app.tool_source_references(workspace_id,project_id,provider,object_kind,external_id,canonical_url,connection_id,connection_revision,created_by_actor_id)
        select p.workspace_id,p.id,'linear','issue',gen_random_uuid(),'https://linear.app/fixture/issue/FICTIF-1',c.id,c.revision,app.current_actor_id() from app.projects p cross join app.work_tool_connections c where p.public_id=$1 and c.public_id=$2 returning public_id")
        .bind(project).bind(connection).fetch_one(&mut *tx).await?;
    let observation: Uuid = sqlx::query_scalar("insert into app.tool_source_observations(workspace_id,project_id,reference_id,version,provider,object_kind,external_id,canonical_url,connection_id,connection_revision,title,body_markdown,availability,coverage,omission_reasons,projection_version,content_hash,snapshot_hash,metadata)
        select workspace_id,project_id,id,1,provider,object_kind,external_id,canonical_url,connection_id,connection_revision,'[FICTIF] Décision observée','[FICTIF] Le client dispose de sept jours.','available','complete','[]','existing-tool-text-v1',repeat('a',64),repeat('b',64),'{}' from app.tool_source_references where public_id=$1 returning public_id")
        .bind(reference).fetch_one(&mut *tx).await?;
    sqlx::query("update app.tool_source_references r set current_observation_id=o.id,last_checked_at=now(),last_check_status='available' from app.tool_source_observations o where o.public_id=$1 and r.id=o.reference_id")
        .bind(observation).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok((project, reference, observation, connection))
}

#[tokio::test]
async fn exact_external_sources_keep_graph_history_and_scope_isolation() -> Result<()> {
    let actor = isolated_actor().await?;
    let owner = create(&actor, Uuid::new_v4()).await?;
    let foreign = create(&actor, Uuid::new_v4()).await?;
    let (project, reference, observation, connection) = fixture(&owner).await?;
    let mut tx = scoped(&owner).await?;
    assert!(sqlx::query_scalar::<_,bool>("select app.tool_source_observation_current(id) from app.tool_source_observations where public_id=$1").bind(observation).fetch_one(&mut *tx).await?);
    sqlx::query("update app.work_tool_connections set revision=revision+1 where public_id=$1")
        .bind(connection)
        .execute(&mut *tx)
        .await?;
    assert!(!sqlx::query_scalar::<_,bool>("select app.tool_source_observation_current(id) from app.tool_source_observations where public_id=$1").bind(observation).fetch_one(&mut *tx).await?);
    // A verified unchanged read reattests the reference, never rewrites history.
    sqlx::query("update app.tool_source_references r set connection_revision=c.revision,revision=r.revision+1 from app.work_tool_connections c where c.id=r.connection_id and r.public_id=$1").bind(reference).execute(&mut *tx).await?;
    assert!(sqlx::query_scalar::<_,bool>("select app.tool_source_observation_current(id) from app.tool_source_observations where public_id=$1").bind(observation).fetch_one(&mut *tx).await?);
    let historical_revision: i32 = sqlx::query_scalar(
        "select connection_revision from app.tool_source_observations where public_id=$1",
    )
    .bind(observation)
    .fetch_one(&mut *tx)
    .await?;
    assert_eq!(historical_revision, 1);
    tx.commit().await?;
    let graph = company::graph(
        &owner,
        GraphQuery {
            project_id: Some(project),
            ..Default::default()
        },
    )
    .await?;
    let node = graph
        .nodes
        .iter()
        .find(|n| n.id == observation)
        .context("exact observation node")?;
    assert_eq!(
        node.app_path.as_deref(),
        Some(format!("/source-observations/{observation}").as_str())
    );
    assert!(
        graph
            .edges
            .iter()
            .any(|edge| edge.source_public_id == observation && edge.target_public_id == reference)
    );
    let mut tx = scoped(&foreign).await?;
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "select count(*) from app.tool_source_observations where public_id=$1"
        )
        .bind(observation)
        .fetch_one(&mut *tx)
        .await?,
        0
    );
    tx.commit().await?;
    Ok(())
}

#[tokio::test]
async fn external_observation_export_and_governed_erasure_preserve_neighbors() -> Result<()> {
    let actor = isolated_actor().await?;
    let owner = create(&actor, Uuid::new_v4()).await?;
    let (project, _, observation, _) = fixture(&owner).await?;
    let (neighbor, _, _, _) = fixture(&owner).await?;
    let artifact = artifacts::create(
        &owner,
        project,
        CreateArtifact {
            artifact_type: "specification".into(),
            title: "[FICTIF] Références conservées".into(),
            body_markdown: "[FICTIF] Travail à partir de l’observation.".into(),
            structured_content: json!({}),
            sources: vec![SourceInput {
                kind: "tool_source_observation".into(),
                public_id: observation,
            }],
        },
        None,
    )
    .await?;
    let exported = project_data::export(&owner, project).await?;
    let text = serde_json::to_string(&exported)?;
    assert!(text.contains(&observation.to_string()));
    assert!(text.contains("observed_external"));
    assert!(!text.contains("encrypted_credential"));
    let admin = erasure::isolated_admin().await?;
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
    let before:Value=sqlx::query_scalar("select jsonb_agg(to_jsonb(o) order by o.id) from app.tool_source_observations o join app.projects p on p.id=o.project_id where p.public_id=$1").bind(neighbor).fetch_one(&admin).await?;
    let manifest: Value = sqlx::query_scalar("select app.operator_project_manifest($1,$2)")
        .bind(owner.workspace_id)
        .bind(project)
        .fetch_one(&admin)
        .await?;
    assert_eq!(manifest["eligible"], true);
    let result: Value = sqlx::query_scalar("select app.operator_purge_project($1,$2,$2,$3)")
        .bind(owner.workspace_id)
        .bind(project)
        .bind(manifest["receipt"].as_str())
        .fetch_one(&admin)
        .await?;
    assert_eq!(result["deleted_rows"]["tool_source_observations"], 1);
    assert_eq!(result["deleted_rows"]["tool_source_references"], 1);
    assert_eq!(result["retained_application_data_verified"], true);
    let after:Value=sqlx::query_scalar("select jsonb_agg(to_jsonb(o) order by o.id) from app.tool_source_observations o join app.projects p on p.id=o.project_id where p.public_id=$1").bind(neighbor).fetch_one(&admin).await?;
    assert_eq!(before, after);
    assert!(
        artifacts::get(&owner, artifact.artifact.public_id)
            .await
            .is_err()
    );
    Ok(())
}
