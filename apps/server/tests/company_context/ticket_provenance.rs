//! [FICTIF] SQL-only publication receipts: no HTTP, credential decryption or AI.
use super::*;
use ai_center_server::{
    artifacts::{
        self, CreateArtifact, SaveDraft, ValidateArtifact,
        generation_contract::{ArtifactDraft, markdown},
    },
    automation::{self, SetControl},
    company::{
        data::{self, ArchiveProject},
        project_data,
    },
};
use serde_json::Value;

struct Fixture {
    project: Uuid,
    artifact: Uuid,
    version: Uuid,
    connection: Uuid,
    jobs: Vec<Uuid>,
    observations: Vec<Uuid>,
}

async fn scoped(owner: &AppState) -> Result<sqlx::Transaction<'_, sqlx::Postgres>> {
    scoped_pool(owner, &owner.pool).await
}

async fn scoped_pool<'a>(
    owner: &AppState,
    pool: &'a sqlx::PgPool,
) -> Result<sqlx::Transaction<'a, sqlx::Postgres>> {
    let mut tx = pool.begin().await?;
    sqlx::query("select set_config('app.current_actor_id',$1,true),set_config('app.current_workspace_id',$2,true),set_config('app.current_workspace_role',$3,true)")
        .bind(owner.actor_id.to_string()).bind(owner.workspace_internal_id.context("workspace")?.to_string()).bind(&owner.workspace_role).execute(&mut *tx).await?;
    Ok(tx)
}

fn ticket_content(title: &str) -> Result<(String, Value)> {
    let content = json!({"format":"agent-artifact-v1","artifact_type":"product_tickets","draft":{
        "title":title,"summary":"[FICTIF] Parcours traçable",
        "sections":[
            {"key":"objective","title":"Objectif","body":"[FICTIF] Préserver la filiation.","source_ids":[]},
            {"key":"prioritization","title":"Priorités","body":"[FICTIF] Deux tâches indépendantes.","source_ids":[]}],
        "tickets":[
            {"title":"[FICTIF] Ticket A","description":"[FICTIF] Première tâche.","acceptance_criteria":["[FICTIF] A vérifiable"],"source_ids":[]},
            {"title":"[FICTIF] Ticket B","description":"[FICTIF] Seconde tâche.","acceptance_criteria":["[FICTIF] B vérifiable"],"source_ids":[]}],
        "open_questions":[]}});
    let draft: ArtifactDraft = serde_json::from_value(content["draft"].clone())?;
    Ok((markdown(&draft), content))
}

async fn project(owner: &AppState, name: &str) -> Result<Uuid> {
    Ok(service::create_project(
        owner,
        CreateProject {
            name: name.into(),
            objective: "[FICTIF] Provenance de tickets sans réseau".into(),
        },
    )
    .await?
    .public_id)
}

async fn insert_job(
    owner: &AppState,
    version: Uuid,
    connection: Uuid,
    index: i16,
    provider: &str,
) -> std::result::Result<Uuid, sqlx::Error> {
    let mut tx = owner.pool.begin().await?;
    sqlx::query("select set_config('app.current_actor_id',$1,true),set_config('app.current_workspace_id',$2,true),set_config('app.current_workspace_role',$3,true)")
        .bind(owner.actor_id.to_string()).bind(owner.workspace_internal_id.unwrap_or_default().to_string()).bind(&owner.workspace_role).execute(&mut *tx).await?;
    let result = sqlx::query_scalar("insert into app.publication_jobs(workspace_id,project_id,artifact_version_id,source_ticket_index,connection_id,connection_revision,requested_by_actor_id,provider,target_id,title,body_markdown,content_hash)
        select v.workspace_id,v.project_id,v.id,$3,c.id,c.revision,app.current_actor_id(),$4,'fictitious-team','[FICTIF] Ticket '||$3::text,'[FICTIF] Contenu préparé '||$3::text,repeat('a',64)
        from app.artifact_document_versions v cross join app.work_tool_connections c where v.public_id=$1 and c.public_id=$2 returning public_id")
        .bind(version).bind(connection).bind(index).bind(provider).fetch_one(&mut *tx).await?;
    tx.commit().await?;
    Ok(result)
}

async fn fixture(owner: &AppState) -> Result<Fixture> {
    let project = project(owner, "[FICTIF] Tickets publiés").await?;
    let (body_markdown, structured_content) = ticket_content("[FICTIF] Deux tickets")?;
    let draft = artifacts::create(
        owner,
        project,
        CreateArtifact {
            artifact_type: "product_tickets".into(),
            title: "[FICTIF] Deux tickets".into(),
            body_markdown,
            structured_content,
            sources: vec![],
        },
        None,
    )
    .await?;
    let validated = artifacts::validate(
        owner,
        draft.artifact.public_id,
        ValidateArtifact {
            expected_version_id: draft.current_version.public_id,
        },
        None,
    )
    .await?;
    let mut tx = scoped(owner).await?;
    // Synthetic bytes are deliberately not usable credentials.
    let connection = sqlx::query_scalar("insert into app.work_tool_connections(public_id,workspace_id,provider,name,encrypted_credential,credential_actor_id)
        values(gen_random_uuid(),app.current_workspace_id(),'linear','[FICTIF] Aucun fournisseur',convert_to(repeat('FICTIF',8),'UTF8'),app.current_actor_id()) returning public_id")
        .fetch_one(&mut *tx).await?;
    tx.commit().await?;
    let mut jobs = Vec::new();
    let mut observations = Vec::new();
    let admin = erasure::isolated_admin().await?;
    for index in [0_i16, 1] {
        let job = insert_job(
            owner,
            validated.current_version.public_id,
            connection,
            index,
            "linear",
        )
        .await?;
        // Operator-only fixture setup simulates worker settlement; runtime cannot set attempts.
        let mut tx = scoped_pool(owner, &admin).await?;
        let external = Uuid::new_v4().to_string();
        let url = format!(
            "https://linear.app/fictitious/issue/FICTIF-{}/ticket",
            index + 1
        );
        sqlx::query("update app.publication_jobs set status='succeeded',attempt_count=1,external_id=$2,external_url=$3 where public_id=$1")
            .bind(job).bind(&external).bind(&url).execute(&mut *tx).await?;
        let observation = sqlx::query_scalar("insert into app.publication_observations(workspace_id,publication_job_id,observation_kind,external_id,external_url,snapshot)
            select workspace_id,id,'created',external_id,external_url,jsonb_build_object('title',title,'body_markdown',body_markdown,'complete',true) from app.publication_jobs where public_id=$1 returning public_id")
            .bind(job).fetch_one(&mut *tx).await?;
        tx.commit().await?;
        jobs.push(job);
        observations.push(observation);
    }
    Ok(Fixture {
        project,
        artifact: validated.artifact.public_id,
        version: validated.current_version.public_id,
        connection,
        jobs,
        observations,
    })
}

async fn assert_provenance(owner: &AppState, source: &Fixture) -> Result<()> {
    let graph = company::graph(
        owner,
        GraphQuery {
            project_id: Some(source.project),
            limit: None,
        },
    )
    .await?;
    let edges: Vec<_> = graph
        .edges
        .iter()
        .filter(|edge| edge.provenance["origin"] == "publication_source")
        .collect();
    assert_eq!(edges.len(), 2);
    for (index, observation) in source.observations.iter().enumerate() {
        let edge = edges
            .iter()
            .find(|edge| edge.source_public_id == *observation)
            .context("exact observation edge")?;
        assert_eq!(edge.edge_type, "derived_from");
        assert_eq!(edge.target_public_id, source.version);
        assert_eq!(edge.source_project_public_id, source.project);
        assert_eq!(edge.target_project_public_id, source.project);
        assert_eq!(edge.provenance["source_ticket_index"], json!(index));
        assert_eq!(
            edge.provenance["publication_id"],
            source.jobs[index].to_string()
        );
        assert_eq!(
            edge.provenance["artifact_version_id"],
            source.version.to_string()
        );
        let node = graph
            .nodes
            .iter()
            .find(|node| node.id == *observation)
            .context("publication node")?;
        assert_eq!(node.kind, "external_reference");
        assert_eq!(
            node.app_path.as_deref(),
            Some(
                format!(
                    "/artifacts/{}?version={}&ticket={index}",
                    source.artifact, source.version
                )
                .as_str()
            )
        );
    }
    Ok(())
}

#[tokio::test]
async fn ticket_graph_and_export_preserve_exact_historical_entries_and_tenant_isolation()
-> Result<()> {
    let actor = isolated_actor().await?;
    let owner = create(&actor, Uuid::new_v4()).await?;
    let foreign = create(&actor, Uuid::new_v4()).await?;
    let source = fixture(&owner).await?;
    assert_provenance(&owner, &source).await?;
    let (body_markdown, structured_content) = ticket_content("[FICTIF] Version ultérieure")?;
    let revised = artifacts::save_draft(
        &owner,
        source.artifact,
        SaveDraft {
            expected_version_id: source.version,
            title: "[FICTIF] Version ultérieure".into(),
            body_markdown,
            structured_content,
            sources: vec![],
        },
        None,
    )
    .await?;
    assert_ne!(revised.current_version.public_id, source.version);
    assert_provenance(&owner, &source).await?;
    let graph = company::graph(
        &owner,
        GraphQuery {
            project_id: Some(source.project),
            limit: None,
        },
    )
    .await?;
    assert_eq!(
        graph
            .nodes
            .iter()
            .find(|node| node.id == source.version)
            .context("historical version")?
            .status,
        "superseded"
    );
    let exported = project_data::export(&owner, source.project).await?;
    assert!(exported.complete);
    let jobs = &exported.data["publication_jobs"];
    assert_eq!(jobs.len(), 2);
    for (index, job) in source.jobs.iter().enumerate() {
        let row = jobs
            .iter()
            .find(|row| row["public_id"] == job.to_string())
            .context("exported job")?;
        assert_eq!(row["source_ticket_index"], json!(index));
        for private in [
            "lease_token",
            "lease_until",
            "encrypted_credential",
            "credential_actor_id",
        ] {
            assert!(
                row.get(private).is_none(),
                "private field {private} must be excluded"
            );
        }
    }
    assert_eq!(exported.data["publication_observations"].len(), 2);
    let serialized = serde_json::to_string(&exported)?;
    assert!(!serialized.contains("FICTIFFICTIF"));
    assert!(!serialized.contains("encrypted_credential"));
    assert!(matches!(
        project_data::export(&foreign, source.project).await,
        Err(AppError::NotFound)
    ));
    let foreign_graph = company::graph(
        &foreign,
        GraphQuery {
            project_id: None,
            limit: None,
        },
    )
    .await?;
    assert!(
        !foreign_graph
            .nodes
            .iter()
            .any(|node| source.observations.contains(&node.id))
    );
    assert!(
        !foreign_graph
            .edges
            .iter()
            .any(|edge| source.observations.contains(&edge.source_public_id))
    );
    let mut tx = scoped(&foreign).await?;
    let count: i64 =
        sqlx::query_scalar("select count(*) from app.publication_jobs where public_id=any($1)")
            .bind(&source.jobs)
            .fetch_one(&mut *tx)
            .await?;
    assert_eq!(count, 0, "runtime RLS hides the foreign receipt itself");
    Ok(())
}

fn check_violation(error: &sqlx::Error) {
    assert_eq!(
        error
            .as_database_error()
            .and_then(sqlx::error::DatabaseError::code)
            .as_deref(),
        Some("23514"),
        "expected SQL source constraint, got {error}"
    );
}

#[tokio::test]
async fn ticket_source_index_and_prepared_destination_are_enforced_by_sql() -> Result<()> {
    let actor = isolated_actor().await?;
    let owner = create(&actor, Uuid::new_v4()).await?;
    let source = fixture(&owner).await?;
    let admin = erasure::isolated_admin().await?;
    for assignment in [
        "source_ticket_index=1",
        "target_id='other-team'",
        "body_markdown='[FICTIF] Substituted body'",
    ] {
        // Exercise the immutable-content trigger beyond the runtime column-grant boundary.
        let mut tx = scoped_pool(&owner, &admin).await?;
        let error = sqlx::query(&format!(
            "update app.publication_jobs set {assignment} where public_id=$1"
        ))
        .bind(source.jobs[0])
        .execute(&mut *tx)
        .await
        .expect_err("committed publication content is immutable");
        check_violation(&error);
        tx.rollback().await?;
    }
    for (index, provider) in [(2, "linear"), (30, "linear"), (0, "notion")] {
        let error = insert_job(&owner, source.version, source.connection, index, provider)
            .await
            .expect_err("ticket source/provider must be checked in SQL");
        check_violation(&error);
    }
    assert_provenance(&owner, &source).await?;
    Ok(())
}

async fn manifest(admin: &sqlx::PgPool, owner: &AppState, project: Uuid) -> Result<Value> {
    Ok(
        sqlx::query_scalar("select app.operator_project_manifest($1,$2)")
            .bind(owner.workspace_id)
            .bind(project)
            .fetch_one(admin)
            .await?,
    )
}

#[tokio::test]
async fn erasing_ticket_publications_waits_for_active_jobs_and_preserves_neighbors() -> Result<()> {
    let actor = isolated_actor().await?;
    let owner = create(&actor, Uuid::new_v4()).await?;
    let foreign = create(&actor, Uuid::new_v4()).await?;
    let source = fixture(&owner).await?;
    let retained = fixture(&owner).await?;
    let _foreign_source = fixture(&foreign).await?;
    let admin = erasure::isolated_admin().await?;
    let retained_before = project_data::export(&owner, retained.project).await?.data;
    let foreign_before = erasure::tenant_hash(
        &admin,
        foreign.workspace_internal_id.context("foreign workspace")?,
    )
    .await?;
    automation::set(
        &owner,
        SetControl {
            enabled: false,
            expected_generation: 0,
        },
        None,
    )
    .await?;
    data::archive(
        &owner,
        source.project,
        ArchiveProject { archived: true },
        None,
    )
    .await?;
    // Operator-only setup simulates an in-flight worker; never execute that worker.
    sqlx::query("update app.publication_jobs set status='processing',lease_token=$2,lease_until=now()+interval '2 minutes' where public_id=$1")
        .bind(source.jobs[0]).bind(Uuid::new_v4()).execute(&admin).await?;
    let preview = manifest(&admin, &owner, source.project).await?;
    assert_eq!(preview["eligible"], true, "{preview}");
    assert_eq!(preview["row_counts"]["publication_jobs"], 2);
    assert_eq!(preview["row_counts"]["publication_observations"], 2);
    let refusal = sqlx::query("select app.operator_purge_project($1,$2,$2,$3)")
        .bind(owner.workspace_id)
        .bind(source.project)
        .bind(preview["receipt"].as_str())
        .execute(&admin)
        .await
        .expect_err("active publication must block purge");
    assert_eq!(
        refusal
            .as_database_error()
            .and_then(sqlx::error::DatabaseError::code)
            .as_deref(),
        Some("55000")
    );
    assert!(refusal.to_string().contains("active work"));
    sqlx::query("update app.publication_jobs set status='succeeded',lease_token=null,lease_until=null where public_id=$1")
        .bind(source.jobs[0]).execute(&admin).await?;
    let refreshed = manifest(&admin, &owner, source.project).await?;
    let receipt: Value = sqlx::query_scalar("select app.operator_purge_project($1,$2,$2,$3)")
        .bind(owner.workspace_id)
        .bind(source.project)
        .bind(refreshed["receipt"].as_str())
        .fetch_one(&admin)
        .await?;
    assert_eq!(receipt["deleted_rows"]["publication_jobs"], 2);
    assert_eq!(receipt["deleted_rows"]["publication_observations"], 2);
    assert_eq!(receipt["retained_application_data_verified"], true);
    let remaining: i64 = sqlx::query_scalar("select (select count(*) from app.publication_jobs where public_id=any($1))+(select count(*) from app.publication_observations where public_id=any($2))")
        .bind(&source.jobs).bind(&source.observations).fetch_one(&admin).await?;
    assert_eq!(remaining, 0);
    assert_eq!(
        project_data::export(&owner, retained.project).await?.data,
        retained_before
    );
    assert_eq!(
        erasure::tenant_hash(
            &admin,
            foreign.workspace_internal_id.context("foreign workspace")?
        )
        .await?,
        foreign_before
    );
    Ok(())
}
