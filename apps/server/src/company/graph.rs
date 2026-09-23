//! Bounded graph projections from canonical `PostgreSQL` objects and source rows.
use super::{
    audit, complete, database_error, editor,
    models::{
        CreateGraphEdge, GraphEdge, GraphEndpoint, GraphLimits, GraphNode, GraphQuery, GraphView,
        ScopeVersion,
    },
};
use crate::{
    error::{AppError, AppResult},
    idempotency::IdempotencyLease,
    service::AppState,
};
use serde_json::json;
use sqlx::{Postgres, Transaction};
use std::collections::HashSet;
use uuid::Uuid;

const DEFAULT_NODE_LIMIT: u32 = 250;
const MAX_NODE_LIMIT: u32 = 500;

fn graph_limit(query: &GraphQuery) -> AppResult<u32> {
    let limit = query.limit.unwrap_or(DEFAULT_NODE_LIMIT);
    if !(1..=MAX_NODE_LIMIT).contains(&limit) {
        return Err(AppError::Invalid(
            "Graph limit must be between 1 and 500".into(),
        ));
    }
    Ok(limit)
}

/// The project view includes its directly linked scopes; the company view is
/// the bounded union. Both resolve through the caller's existing RLS context.
///
/// # Errors
/// Rejects unknown scopes, invalid budgets and failed database reads.
pub async fn graph(state: &AppState, query: GraphQuery) -> AppResult<GraphView> {
    let limit = graph_limit(&query)?;
    let mut tx = state.begin_request().await?;
    if let Some(project_id) = query.project_id {
        let exists:bool=sqlx::query_scalar("select exists(select 1 from app.projects where public_id=$1 and workspace_id=app.current_workspace_id())")
            .bind(project_id).fetch_one(&mut *tx).await?;
        if !exists {
            return Err(AppError::NotFound);
        }
    }
    let mut scopes:Vec<i64>=sqlx::query_scalar("select p.id from app.projects p where p.workspace_id=app.current_workspace_id() and (p.status='active' or p.public_id=$1)
        and ($1::uuid is null or p.public_id=$1 or p.scope_kind='company' or exists(select 1 from app.edges e join app.projects selected on selected.public_id=$1
          where e.workspace_id=p.workspace_id and e.status='confirmed' and
          ((e.project_id=selected.id and e.target_project_id=p.id) or (e.target_project_id=selected.id and e.project_id=p.id)))
          or exists(select 1 from app.context_pack_scope_sources s join app.context_packs pack on pack.id=s.context_pack_id
            join app.projects selected on selected.id=pack.project_id where selected.public_id=$1 and s.source_project_id=p.id and s.decision='included')
          or exists(select 1 from app.artifact_version_sources s join app.artifact_document_versions v on v.id=s.version_id
             join app.projects selected on selected.id=v.project_id where selected.public_id=$1 and s.source_project_id=p.id)
          or exists(select 1 from app.steward_scope_sources s join app.projects selected on selected.id=s.project_id
             where selected.public_id=$1 and s.source_project_id=p.id))
        order by case when p.public_id=$1 then 0 else 1 end,p.id limit 101")
        .bind(query.project_id).fetch_all(&mut *tx).await?;
    let scopes_truncated = scopes.len() > 100;
    scopes.truncate(100);
    let source_graph_versions:Vec<ScopeVersion>=sqlx::query_as("select public_id as project_public_id,graph_version from app.projects where id=any($1) order by id")
        .bind(&scopes).fetch_all(&mut *tx).await?;
    let mut nodes: Vec<GraphNode> = sqlx::query_as(include_str!("graph_nodes.sql"))
        .bind(&scopes)
        .bind(i64::from(limit) + 1)
        .bind(query.project_id)
        .fetch_all(&mut *tx)
        .await?;
    let mut truncated =
        scopes_truncated || nodes.len() > usize::try_from(limit).unwrap_or(usize::MAX);
    nodes.truncate(usize::try_from(limit).unwrap_or(usize::MAX));
    let identities: HashSet<(String, Uuid)> = nodes
        .iter()
        .map(|node| (node.kind.clone(), node.id))
        .collect();
    let mut edges: Vec<GraphEdge> = sqlx::query_as(include_str!("graph_edges.sql"))
        .bind(&scopes)
        .bind(i64::from(limit) * 3 + 1)
        .fetch_all(&mut *tx)
        .await?;
    let edge_limit = limit * 3;
    truncated |= edges.len() > usize::try_from(edge_limit).unwrap_or(usize::MAX);
    edges.truncate(usize::try_from(edge_limit).unwrap_or(usize::MAX));
    // A truncated graph never emits a dangling endpoint. The client can ask for
    // a narrower project view when the response reports truncation.
    edges.retain(|edge| {
        identities.contains(&(edge.source_kind.clone(), edge.source_public_id))
            && identities.contains(&(edge.target_kind.clone(), edge.target_public_id))
    });
    tx.commit().await?;
    Ok(GraphView {
        workspace_public_id: state.workspace_id,
        project_public_id: query.project_id,
        nodes,
        edges,
        source_graph_versions,
        truncated,
        limits: GraphLimits {
            max_nodes: limit,
            max_edges: edge_limit,
        },
    })
}

fn validate_edge(input: &CreateGraphEdge) -> AppResult<()> {
    if input.public_id.is_nil()
        || input.source.public_id.is_nil()
        || input.target.public_id.is_nil()
        || input.source.project_public_id.is_nil()
        || input.target.project_public_id.is_nil()
    {
        return Err(AppError::Invalid("Graph identities must not be nil".into()));
    }
    for endpoint in [&input.source, &input.target] {
        if !matches!(
            endpoint.kind.as_str(),
            "scope"
                | "agent"
                | "knowledge"
                | "artifact"
                | "external_reference"
                | "insight"
                | "session"
                | "task"
        ) {
            return Err(AppError::Invalid("Unknown graph object kind".into()));
        }
    }
    if input.source.kind == input.target.kind && input.source.public_id == input.target.public_id {
        return Err(AppError::Invalid(
            "A graph relationship cannot target itself".into(),
        ));
    }
    if !matches!(
        input.edge_type.as_str(),
        "references"
            | "depends_on"
            | "informs"
            | "supersedes"
            | "contradicts"
            | "derived_from"
            | "satisfies"
            | "evidenced_by"
            | "implemented_by"
            | "tracked_by"
    ) {
        return Err(AppError::Invalid("Unknown graph relationship".into()));
    }
    Ok(())
}

async fn endpoint(
    tx: &mut Transaction<'_, Postgres>,
    input: &GraphEndpoint,
) -> AppResult<(i64, String)> {
    let project_id:i64=sqlx::query_scalar("select id from app.projects where public_id=$1 and workspace_id=app.current_workspace_id() and status='active' for share")
        .bind(input.project_public_id).fetch_optional(&mut **tx).await?.ok_or(AppError::NotFound)?;
    let kinds: &[&str] = match input.kind.as_str() {
        "scope" => &["project"],
        "session" => &["session"],
        "task" => &["task"],
        "agent" => &["context_node"],
        "knowledge" => &["knowledge_entry_version"],
        "artifact" => &[
            "deliverable",
            "context_pack",
            "artifact",
            "artifact_document_version",
        ],
        "external_reference" => &[
            "external_reference",
            "external_reference_observation",
            "publication_observation",
            "github_code_file_observation",
        ],
        "insight" => &["insight"],
        _ => return Err(AppError::Invalid("Unknown graph object kind".into())),
    };
    let mut found = None;
    for kind in kinds {
        let exists: bool = sqlx::query_scalar(
            "select app.graph_endpoint_exists($1,$2,$3,app.current_workspace_id())",
        )
        .bind(kind)
        .bind(input.public_id)
        .bind(project_id)
        .fetch_one(&mut **tx)
        .await?;
        if exists {
            if found.is_some() {
                return Err(AppError::Conflict("Ambiguous graph object identity".into()));
            }
            found = Some((*kind).to_owned());
        }
    }
    Ok((project_id, found.ok_or(AppError::NotFound)?))
}

/// Records a human-confirmed relationship between two validated graph objects.
///
/// # Errors
/// Rejects missing privileges, invalid or foreign endpoints and conflicting identities.
pub async fn create_edge(
    state: &AppState,
    input: CreateGraphEdge,
    lease: Option<&IdempotencyLease>,
) -> AppResult<GraphEdge> {
    editor(state)?;
    validate_edge(&input)?;
    let mut tx = state.begin_request().await?;
    // Take write locks in stable scope order before endpoint checks. Two
    // concurrent relationships must never upgrade shared locks against each other.
    sqlx::query("select id from app.projects where public_id=any($1) and workspace_id=app.current_workspace_id() and status='active' order by id for update")
        .bind(vec![input.source.project_public_id,input.target.project_public_id])
        .fetch_all(&mut *tx).await?;
    let (source_project, source_kind) = endpoint(&mut tx, &input.source).await?;
    let (target_project, target_kind) = endpoint(&mut tx, &input.target).await?;
    let inserted=sqlx::query("insert into app.edges(public_id,workspace_id,project_id,target_project_id,source_kind,source_public_id,target_kind,target_public_id,edge_type,status,provenance,created_by_actor_id)
        values($1,app.current_workspace_id(),$2,$3,$4,$5,$6,$7,$8,'confirmed','{\"origin\":\"human\"}'::jsonb,$9) on conflict do nothing")
        .bind(input.public_id).bind(source_project).bind(target_project).bind(&source_kind).bind(input.source.public_id)
        .bind(&target_kind).bind(input.target.public_id).bind(&input.edge_type).bind(state.actor_id).execute(&mut *tx).await.map_err(database_error)?;
    let persisted:Option<(Uuid,String,Uuid,String,Uuid,i64,i64)>=sqlx::query_as("select public_id,source_kind,source_public_id,target_kind,target_public_id,project_id,target_project_id from app.edges
        where public_id=$1 and workspace_id=app.current_workspace_id() and edge_type=$2
          and status='confirmed' and provenance->>'origin'='human' and created_by_actor_id=$3")
        .bind(input.public_id).bind(&input.edge_type).bind(state.actor_id).fetch_optional(&mut *tx).await?;
    let Some((
        id,
        stored_source_kind,
        source_id,
        stored_target_kind,
        target_id,
        stored_source_project,
        stored_target_project,
    )) = persisted
    else {
        return Err(AppError::Conflict(
            "This relationship or identity already exists".into(),
        ));
    };
    if stored_source_kind != source_kind
        || source_id != input.source.public_id
        || stored_target_kind != target_kind
        || target_id != input.target.public_id
        || stored_source_project != source_project
        || stored_target_project != target_project
    {
        return Err(AppError::Conflict(
            "This graph identity is bound to another relationship".into(),
        ));
    }
    if inserted.rows_affected() > 0 {
        sqlx::query("update app.projects set graph_version=graph_version+1 where id=any($1)")
            .bind(vec![source_project, target_project])
            .execute(&mut *tx)
            .await?;
        // A new relationship changes the selectable context. Preserve previous
        // versions while making their former validity explicit to every client.
        sqlx::query("update app.context_packs set status='stale',invalidated_at=now(),stale_reason='confirmed graph relationship changed'
            where project_id=any($1) and status='current'")
            .bind(vec![source_project,target_project]).execute(&mut *tx).await?;
        sqlx::query("update app.deliverables set status='stale',stale_at=now() where project_id=any($1) and status in ('draft','committed')")
            .bind(vec![source_project,target_project]).execute(&mut *tx).await?;
        sqlx::query("insert into app.domain_events(workspace_id,project_id,event_type,aggregate_kind,aggregate_public_id,payload)
            values(app.current_workspace_id(),$1,'graph.relationship_confirmed','edge',$2,$3)")
            .bind(source_project).bind(id).bind(json!({"source_project_public_id":input.source.project_public_id,"target_project_public_id":input.target.project_public_id}))
            .execute(&mut *tx).await?;
        audit(
            &mut tx,
            state.actor_id,
            "graph.relationship_confirmed",
            "edge",
            id,
            json!({"edge_type":input.edge_type}),
        )
        .await?;
    }
    let result = GraphEdge {
        id,
        source_kind: input.source.kind,
        source_public_id: source_id,
        source_project_public_id: input.source.project_public_id,
        target_kind: input.target.kind,
        target_public_id: target_id,
        target_project_public_id: input.target.project_public_id,
        edge_type: input.edge_type,
        status: "confirmed".into(),
        provenance: json!({"origin":"human"}),
    };
    complete(&mut tx, lease, &result).await?;
    tx.commit().await?;
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn graph_projection_budget_cannot_be_unbounded() {
        assert_eq!(graph_limit(&GraphQuery::default()).unwrap(), 250);
        for limit in [0, 501, u32::MAX] {
            assert!(
                graph_limit(&GraphQuery {
                    limit: Some(limit),
                    project_id: None
                })
                .is_err()
            );
        }
    }
    #[test]
    fn graph_rejects_self_links_and_unknown_kinds_and_relations() {
        let first = GraphEndpoint {
            kind: "knowledge".into(),
            public_id: Uuid::new_v4(),
            project_public_id: Uuid::new_v4(),
        };
        let mut edge = CreateGraphEdge {
            public_id: Uuid::new_v4(),
            source: first.clone(),
            target: first,
            edge_type: "references".into(),
        };
        assert!(validate_edge(&edge).is_err());
        edge.target.public_id = Uuid::new_v4();
        assert!(validate_edge(&edge).is_ok());
        edge.target.kind = "provider_credentials".into();
        assert!(validate_edge(&edge).is_err());
        edge.target.kind = "knowledge".into();
        edge.edge_type = "execute".into();
        assert!(validate_edge(&edge).is_err());
    }
}
