with scope as (
 select id,public_id,name,scope_kind from app.projects where public_id=$1 and workspace_id=app.current_workspace_id()
), objects as (
 select v.public_id,'knowledge'::text kind,v.title,
   case when v.version_number=e.latest_version then e.status else 'superseded' end status,
   v.version_number,v.created_at recorded_at,
   jsonb_build_object('statement',v.statement,'rationale',v.rationale,'entry_type',v.entry_type,
    'origin_type',v.origin_type,'origin_public_id',v.origin_public_id,'source_message_public_id',v.source_message_public_id) content,
   coalesce((select jsonb_agg(jsonb_build_object('label',s.title,'app_path','/projects/'||p.public_id||'/sessions/'||s.public_id))
    from app.sessions s join scope p on p.id=s.project_id
    where s.public_id=v.origin_public_id or exists(select 1 from app.messages m where m.session_id=s.id and m.public_id=v.source_message_public_id)),'[]') links
 from app.knowledge_entry_versions v join app.knowledge_entries e on e.id=v.knowledge_entry_id join scope p on p.id=v.project_id
 where $2='knowledge' and v.public_id=$3
 union all
 select c.public_id,'context_pack',c.objective,case when c.status='current' and not app.context_pack_scopes_current(c.id) then 'stale' else c.status end,c.version,c.compiled_at,c.content,
  coalesce((select jsonb_agg(link order by link->>'label') from (
   select jsonb_build_object('label',v.title||' · v'||v.version_number,'app_path','/projects/'||p.public_id||'/sources/knowledge/'||v.public_id) link
   from app.context_pack_sources s join app.knowledge_entry_versions v on v.id=s.knowledge_entry_version_id
   join app.projects p on p.id=v.project_id where s.context_pack_id=c.id
   union
   select jsonb_build_object('label',v.title||' · v'||v.version_number,'app_path','/projects/'||p.public_id||'/sources/knowledge/'||v.public_id)
   from app.context_pack_scope_sources s join app.knowledge_entry_versions v on v.id=s.knowledge_version_id
   join app.projects p on p.id=v.project_id where s.context_pack_id=c.id and s.decision='included'
   union
   select jsonb_build_object('label',v.title||' · v'||v.version,'app_path','/artifacts/'||d.public_id||'?version='||v.public_id)
   from app.context_pack_scope_sources s join app.artifact_document_versions v on v.id=s.artifact_version_id
   join app.artifact_documents d on d.id=v.document_id where s.context_pack_id=c.id and s.decision='included'
  ) sources),'[]')
 from app.context_packs c join scope p on p.id=c.project_id where $2='context_pack' and c.public_id=$3
 union all
 select t.public_id,'task',t.title,t.status,null,t.created_at,
  jsonb_build_object('statement','Cette tâche conserve son contexte de travail. Son état enregistré ne certifie pas une implémentation dans un outil externe.'),
  jsonb_build_array(jsonb_build_object('label',c.objective||' · v'||c.version,'app_path','/projects/'||p.public_id||'/sources/context_pack/'||c.public_id))
 from app.tasks t join app.context_packs c on c.id=t.context_pack_id join scope p on p.id=t.project_id where $2='task' and t.public_id=$3
 union all
 select a.public_id,'artifact',a.title,'recorded',null,a.created_at,
  jsonb_build_object('statement',a.reference,'artifact_type',a.artifact_type),
  coalesce((select jsonb_agg(jsonb_build_object('label',d.title,'app_path','/projects/'||p.public_id||'/deliverables/'||d.public_id))
   from app.deliverables d where d.id=a.deliverable_id),'[]')
 from app.artifacts a join scope p on p.id=a.project_id where $2='artifact' and a.public_id=$3
)
select jsonb_build_object('public_id',o.public_id,'project_public_id',p.public_id,'project_name',p.name,'scope_kind',p.scope_kind,
 'kind',o.kind,'title',o.title,'status',o.status,'version_number',o.version_number,'recorded_at',o.recorded_at,'content',o.content,'links',o.links)
from objects o cross join scope p
