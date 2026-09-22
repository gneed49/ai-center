with recursive origin as (
 select id,scope_kind from app.projects where id=$1 and workspace_id=app.current_workspace_id() and status='active'
), linked(id,depth,path) as (
 select id,0,array[id] from origin
 union all
 select case when e.project_id=l.id then e.target_project_id else e.project_id end,l.depth+1,
   l.path||case when e.project_id=l.id then e.target_project_id else e.project_id end
 from linked l join app.edges e on e.project_id=l.id or e.target_project_id=l.id
 where e.workspace_id=app.current_workspace_id() and e.status='confirmed' and l.depth<2
   and not (case when e.project_id=l.id then e.target_project_id else e.project_id end=any(l.path))
), scopes as (
 select p.* from app.projects p cross join origin o
 where p.workspace_id=app.current_workspace_id() and p.status='active'
   and (o.scope_kind='company' or p.scope_kind='company' or p.id in(select id from linked))
), candidates as (
 select v.id as source_id,p.id as project_id,p.public_id as project_public_id,p.graph_version,p.scope_kind,
   'knowledge_entry_version'::text as source_kind,
   p.scope_kind='company' and v.entry_type in('business_rule','constraint') as mandatory,
   length(v.statement)>40000 as excerpt,
   jsonb_build_object('knowledge_public_id',k.public_id,'version_public_id',v.public_id,'version_number',v.version_number,
     'entry_type',v.entry_type,'title',v.title,'statement',left(v.statement,40000),'rationale',left(v.rationale,2000),
     'node_key',case when p.id=$1 then n.node_key else p.scope_kind||'/'||n.node_key end) as candidate,
   ts_rank(to_tsvector('simple',left(v.title||' '||v.statement,40000)),to_tsquery('simple',$2)) as relevance,v.created_at
 from app.knowledge_entries k join app.knowledge_entry_versions v on v.knowledge_entry_id=k.id and v.version_number=k.latest_version
 join scopes p on p.id=k.project_id join app.context_nodes n on n.id=k.context_node_id
 where k.status='confirmed' and v.status='confirmed'
 union all
 select v.id,p.id,p.public_id,p.graph_version,p.scope_kind,'artifact_document_version',false,
   length(v.body_markdown||E'\n'||v.structured_content::text)>8000,
   jsonb_build_object('knowledge_public_id',d.public_id,'version_public_id',v.public_id,'version_number',v.version,
     'entry_type','artifact','title',v.title,'statement',left(v.body_markdown||E'\n'||v.structured_content::text,8000),
     'rationale','Validated artifact; potentially partial excerpt; hash='||v.content_hash,
     'node_key',case when p.id=$1 then 'artifact' else p.scope_kind||'/artifact' end),
   ts_rank(to_tsvector('simple',left(v.title||' '||v.body_markdown||' '||v.structured_content::text,40000)),to_tsquery('simple',$2)),v.created_at
 from app.artifact_document_versions v join app.artifact_documents d on d.id=v.document_id join scopes p on p.id=d.project_id
 where v.status='validated' and not exists(select 1 from app.artifact_document_versions newer where newer.document_id=d.id and newer.status='validated' and newer.version>v.version)
), ranked as (
 select *,count(*) over() as total_sources,count(*) filter(where mandatory) over() as mandatory_count,
   row_number() over(partition by source_kind order by mandatory desc,relevance desc,(project_id=$1) desc,created_at desc,source_id desc) as ordinal
 from candidates
)
select source_id,project_id,project_public_id,graph_version,scope_kind,source_kind,mandatory,excerpt,candidate,total_sources,mandatory_count
 from ranked where (source_kind='knowledge_entry_version' and ordinal<=161) or (source_kind='artifact_document_version' and ordinal<=21)
 order by mandatory desc,relevance desc,(project_id=$1) desc,created_at desc,source_kind,source_id desc
