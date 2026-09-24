//! Bounded, authorized context traversal shared by chat and immutable packs.
//! Traversal follows confirmed relationships inside one workspace, at depth two.
use crate::{
    context::{CompiledContextPack, ContextCandidate},
    error::{AppError, AppResult},
};
use serde::Serialize;
use serde_json::{Value, json};
use sqlx::{FromRow, Postgres, Transaction, types::Json};
use std::collections::HashSet;
use uuid::Uuid;

mod chat;
pub(crate) use chat::load_for_query;

const MAX_SCOPES: usize = 20;
const MAX_KNOWLEDGE: usize = 160;
const MAX_ARTIFACTS: usize = 20;
const MAX_CONTEXT_BYTES: usize = 160_000;

#[derive(Debug, Clone, Serialize, FromRow)]
pub(crate) struct ScopeStamp {
    #[serde(skip)]
    pub project_id: i64,
    pub project_public_id: Uuid,
    pub graph_version: i64,
    pub scope_kind: String,
}

#[derive(Clone)]
pub(crate) struct SourceCandidate {
    pub source_id: i64,
    pub source_project_id: i64,
    pub source_project_public_id: Uuid,
    pub source_kind: &'static str,
    pub candidate: ContextCandidate,
}

impl SourceCandidate {
    fn provenance(&self) -> Value {
        json!({"source_kind":self.source_kind,"source_project_public_id":self.source_project_public_id,
            "source_version_public_id":self.candidate.version_public_id})
    }
}

pub(crate) struct ScopeSnapshot {
    pub sources: Vec<SourceCandidate>,
    pub scopes: Vec<ScopeStamp>,
    pub summaries: Vec<Value>,
    pub summaries_truncated: bool,
    pub retrieval: Value,
}

impl ScopeSnapshot {
    pub fn candidates(&self) -> Vec<ContextCandidate> {
        self.sources
            .iter()
            .map(|source| source.candidate.clone())
            .collect()
    }
    pub fn source_ids(&self) -> Vec<Uuid> {
        self.sources
            .iter()
            .map(|source| source.candidate.version_public_id)
            .collect()
    }
    pub fn context(&self, agent_scope: &str, node_key: &str) -> Value {
        json!({"scope":agent_scope,"node":node_key,"knowledge":self.candidates(),
            "source_scopes":self.scopes,"source_provenance":self.sources.iter().map(SourceCandidate::provenance).collect::<Vec<_>>(),
            "project_summaries":self.summaries,"project_summaries_truncated":self.summaries_truncated,"retrieval":self.retrieval,"traversal":{"max_depth":2,"source_count":self.sources.len(),"status":"bounded_authorized_snapshot"}})
    }
    pub fn extra_content(&self) -> Value {
        json!({"source_scopes":self.scopes,"source_provenance":self.sources.iter().map(SourceCandidate::provenance).collect::<Vec<_>>()})
    }
}

type CandidateRow = (i64, i64, Uuid, Json<ContextCandidate>);

#[allow(clippy::too_many_lines)] // Exhaustive compiler projection, separate from question retrieval.
pub(crate) async fn load(
    tx: &mut Transaction<'_, Postgres>,
    project_id: i64,
) -> AppResult<ScopeSnapshot> {
    let mut scopes:Vec<ScopeStamp>=sqlx::query_as("with recursive linked(id,depth,path) as (
        select id,0,array[id] from app.projects where id=$1 and workspace_id=app.current_workspace_id()
        union all select case when e.project_id=l.id then e.target_project_id else e.project_id end,l.depth+1,
          l.path || case when e.project_id=l.id then e.target_project_id else e.project_id end
        from linked l join app.edges e on (e.project_id=l.id or e.target_project_id=l.id)
        where e.workspace_id=app.current_workspace_id() and e.status='confirmed' and l.depth<2
          and not (case when e.project_id=l.id then e.target_project_id else e.project_id end=any(l.path))
      ) select p.id as project_id,p.public_id as project_public_id,p.graph_version,p.scope_kind from app.projects p
      where p.workspace_id=app.current_workspace_id() and p.status='active' and (p.id in(select id from linked) or p.scope_kind='company')
      order by case when p.id=$1 then 0 when p.scope_kind='company' then 1 else 2 end,p.id limit 21")
        .bind(project_id).fetch_all(&mut **tx).await?;
    if scopes.len() > MAX_SCOPES {
        return Err(AppError::Invalid(
            "Context reaches more than 20 scopes; narrow the confirmed relationships".into(),
        ));
    }
    if !scopes.iter().any(|scope| scope.project_id == project_id) {
        return Err(AppError::NotFound);
    }
    let ids: Vec<i64> = scopes.iter().map(|scope| scope.project_id).collect();
    let rows:Vec<CandidateRow>=sqlx::query_as("select v.id,p.id,p.public_id,jsonb_build_object(
        'knowledge_public_id',k.public_id,'version_public_id',v.public_id,'version_number',v.version_number,
        'entry_type',v.entry_type,'title',v.title,'statement',v.statement,'rationale',v.rationale,
        'node_key',case when p.id=$1 then n.node_key else p.scope_kind||'/'||n.node_key end)
      from app.knowledge_entries k join app.knowledge_entry_versions v on v.knowledge_entry_id=k.id and v.version_number=k.latest_version
      join app.context_nodes n on n.id=k.context_node_id join app.projects p on p.id=k.project_id
      where p.id=any($2) and k.status='confirmed' and v.status='confirmed'
        and (p.id=$1 or (p.scope_kind='company' and v.entry_type in ('business_rule','constraint'))
          or (p.scope_kind='project' and (v.entry_type in ('business_rule','constraint','decision') or exists(
            select 1 from app.edges e where e.status='confirmed' and e.workspace_id=p.workspace_id and
             ((e.source_kind='knowledge_entry_version' and e.source_public_id=v.public_id) or
              (e.target_kind='knowledge_entry_version' and e.target_public_id=v.public_id))))))
      order by case when p.id=$1 then 0 when p.scope_kind='company' then 1 else 2 end,v.created_at,v.id limit 161")
        .bind(project_id).bind(&ids).fetch_all(&mut **tx).await?;
    if rows.len() > MAX_KNOWLEDGE {
        return Err(AppError::Invalid(
            "Context contains more than 160 confirmed knowledge versions; narrow the scope".into(),
        ));
    }
    let mut sources: Vec<SourceCandidate> = rows
        .into_iter()
        .map(
            |(source_id, source_project_id, source_project_public_id, Json(candidate))| {
                SourceCandidate {
                    source_id,
                    source_project_id,
                    source_project_public_id,
                    source_kind: "knowledge_entry_version",
                    candidate,
                }
            },
        )
        .collect();
    let artifacts:Vec<CandidateRow>=sqlx::query_as("select v.id,p.id,p.public_id,jsonb_build_object(
        'knowledge_public_id',d.public_id,'version_public_id',v.public_id,'version_number',v.version,
        'entry_type','artifact','title',v.title,
        'statement',left(v.body_markdown||E'\\n'||v.structured_content::text,8000),
        'rationale','Validated artifact version; excerpt limited to 8000 characters; hash='||v.content_hash,
        'node_key',case when p.id=$2 then 'artifact' else p.scope_kind||'/artifact' end)
      from app.artifact_document_versions v join app.artifact_documents d on d.id=v.document_id join app.projects p on p.id=v.project_id
      where p.id=any($1::bigint[]) and v.status='validated' and not exists(select 1 from app.artifact_document_versions newer
        where newer.document_id=v.document_id and newer.status='validated' and newer.version>v.version)
      order by v.created_at desc,v.id desc limit 21")
        .bind(&ids).bind(project_id).fetch_all(&mut **tx).await?;
    if artifacts.len() > MAX_ARTIFACTS {
        return Err(AppError::Invalid(
            "Context contains more than 20 validated artifacts; narrow the scope".into(),
        ));
    }
    sources.extend(artifacts.into_iter().map(
        |(source_id, source_project_id, source_project_public_id, Json(candidate))| {
            SourceCandidate {
                source_id,
                source_project_id,
                source_project_public_id,
                source_kind: "artifact_document_version",
                candidate,
            }
        },
    ));
    let mut unique = HashSet::new();
    sources
        .retain(|source| unique.insert((source.source_kind, source.candidate.version_public_id)));
    let (summaries, summaries_truncated) = project_summaries(tx, project_id, &mut scopes).await?;
    let snapshot = ScopeSnapshot {
        sources,
        scopes,
        summaries,
        summaries_truncated,
        retrieval: json!({"mode":"exhaustive_compiler_candidates"}),
    };
    let bytes = serde_json::to_vec(&snapshot.context("preview", "preview"))
        .map_err(|error| AppError::Internal(error.to_string()))?;
    if bytes.len() > MAX_CONTEXT_BYTES {
        return Err(AppError::Invalid(
            "Authorized context exceeds the bounded snapshot size; narrow its content".into(),
        ));
    }
    Ok(snapshot)
}

async fn project_summaries(
    tx: &mut Transaction<'_, Postgres>,
    project_id: i64,
    scopes: &mut Vec<ScopeStamp>,
) -> AppResult<(Vec<Value>, bool)> {
    if !scopes
        .iter()
        .any(|scope| scope.project_id == project_id && scope.scope_kind == "company")
    {
        return Ok((Vec::new(), false));
    }
    let summaries: Vec<(i64,Uuid,i64,String,String,String)>=sqlx::query_as(
        "select id,public_id,graph_version,name,left(objective,1000),left(summary,1000) from app.projects
         where workspace_id=app.current_workspace_id() and scope_kind='project' and status='active' order by updated_at desc,id desc limit 21")
        .fetch_all(&mut **tx).await?;
    let truncated = summaries.len() > 20;
    let summaries=summaries.into_iter().take(20).map(|(internal_id,id,graph_version,name,objective,summary)|{
        if !scopes.iter().any(|scope|scope.project_id==internal_id) {
            scopes.push(ScopeStamp{project_id:internal_id,project_public_id:id,graph_version,scope_kind:"project".into()});
        }
        json!({"project_public_id":id,"graph_version":graph_version,"name":name,"objective":objective,"summary":summary,
            "evidence_status":"project_summary_only"})
    }).collect();
    Ok((summaries, truncated))
}

pub(crate) async fn verify_snapshot(
    tx: &mut Transaction<'_, Postgres>,
    stamps: &[ScopeStamp],
) -> AppResult<bool> {
    // Lock in a stable order so concurrent requests spanning the same scopes
    // cannot finalize a response against a changed or partially read snapshot.
    let ids: Vec<i64> = stamps.iter().map(|stamp| stamp.project_id).collect();
    let current: Vec<(i64, i64, String)> = sqlx::query_as(
        "select id,graph_version,status from app.projects where id=any($1) order by id for update",
    )
    .bind(&ids)
    .fetch_all(&mut **tx)
    .await?;
    Ok(current.len() == stamps.len()
        && stamps.iter().all(|stamp| {
            current.iter().any(|(id, version, status)| {
                *id == stamp.project_id && *version == stamp.graph_version && status == "active"
            })
        }))
}

pub(crate) async fn pack_current(
    tx: &mut Transaction<'_, Postgres>,
    pack_id: i64,
) -> AppResult<bool> {
    Ok(
        sqlx::query_scalar("select app.context_pack_scopes_current($1)")
            .bind(pack_id)
            .fetch_one(&mut **tx)
            .await?,
    )
}

pub(crate) async fn pack_stamps(
    tx: &mut Transaction<'_, Postgres>,
    pack_id: i64,
) -> AppResult<Vec<ScopeStamp>> {
    Ok(sqlx::query_as(
        "select p.id as project_id,p.public_id as project_public_id,p.graph_version,p.scope_kind
        from app.context_pack_scope_versions s join app.projects p on p.id=s.source_project_id
        where s.context_pack_id=$1 order by p.id",
    )
    .bind(pack_id)
    .fetch_all(&mut **tx)
    .await?)
}

pub(crate) async fn pack_source_ids(
    tx: &mut Transaction<'_, Postgres>,
    pack_id: i64,
) -> AppResult<Vec<Uuid>> {
    Ok(sqlx::query_scalar("select v.public_id from app.context_pack_sources s join app.knowledge_entry_versions v on v.id=s.knowledge_entry_version_id where s.context_pack_id=$1
        union select source_public_id from app.context_pack_scope_sources where context_pack_id=$1 and decision='included' order by 1")
        .bind(pack_id).fetch_all(&mut **tx).await?)
}

pub(crate) async fn persist(
    tx: &mut Transaction<'_, Postgres>,
    project_id: i64,
    pack_id: i64,
    snapshot: &ScopeSnapshot,
    compiled: &CompiledContextPack,
) -> AppResult<()> {
    for stamp in &snapshot.scopes {
        sqlx::query("insert into app.context_pack_scope_versions(workspace_id,project_id,context_pack_id,source_project_id,graph_version)
            values(app.current_workspace_id(),$1,$2,$3,$4)").bind(project_id).bind(pack_id).bind(stamp.project_id).bind(stamp.graph_version).execute(&mut **tx).await?;
    }
    for item in &compiled.selection_items {
        let source = snapshot
            .sources
            .iter()
            .find(|source| source.candidate.version_public_id == item.version_public_id)
            .ok_or_else(|| {
                AppError::Internal("Compiled source missing from its authorized snapshot".into())
            })?;
        if source.source_project_id == project_id && source.source_kind == "knowledge_entry_version"
        {
            continue;
        }
        let knowledge_id =
            (source.source_kind == "knowledge_entry_version").then_some(source.source_id);
        let artifact_id =
            (source.source_kind == "artifact_document_version").then_some(source.source_id);
        sqlx::query("insert into app.context_pack_scope_sources(workspace_id,project_id,context_pack_id,source_project_id,source_kind,source_public_id,
            knowledge_version_id,artifact_version_id,decision,reason_code,explanation,rank,estimated_tokens,is_mandatory)
            values(app.current_workspace_id(),$1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13)")
            .bind(project_id).bind(pack_id).bind(source.source_project_id).bind(source.source_kind).bind(item.version_public_id)
            .bind(knowledge_id).bind(artifact_id).bind(if item.included {"included"} else {"excluded"}).bind(&item.reason_code)
            .bind(&item.explanation).bind(item.rank).bind(item.token_estimate).bind(item.required).execute(&mut **tx).await?;
    }
    Ok(())
}
