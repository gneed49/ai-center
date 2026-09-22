with scopes as (select * from app.projects where id=any($1) and workspace_id=app.current_workspace_id()),
objects as (
  select p.public_id as id,'scope'::text as kind,p.public_id as project_public_id,p.scope_kind,
    p.name as label,p.status,null::uuid as version_public_id,null::text as source_url,0 as priority
  from scopes p
  union all
  select n.public_id,'agent',p.public_id,p.scope_kind,n.title,'active',null,null,1
  from app.context_nodes n join scopes p on p.id=n.project_id
  union all
  select v.public_id,'knowledge',p.public_id,p.scope_kind,v.title,
    case when v.version_number=e.latest_version then e.status else 'superseded' end,v.public_id,null,2
  from app.knowledge_entries e join app.knowledge_entry_versions v on v.knowledge_entry_id=e.id
  join scopes p on p.id=e.project_id
  where v.version_number=e.latest_version
    or exists(select 1 from app.context_pack_sources s where s.knowledge_entry_version_id=v.id)
    or exists(select 1 from app.context_pack_scope_sources s where s.knowledge_version_id=v.id)
    or exists(select 1 from app.steward_scope_sources s where s.knowledge_version_id=v.id)
    or exists(select 1 from app.deliverable_sources s where s.knowledge_entry_version_id=v.id)
    or exists(select 1 from app.artifact_version_sources s where s.knowledge_version_id=v.id)
    or exists(select 1 from app.edges edge where
      (edge.source_kind='knowledge_entry_version' and edge.source_public_id=v.public_id)
      or (edge.target_kind='knowledge_entry_version' and edge.target_public_id=v.public_id))
  union all
  select d.public_id,'artifact',p.public_id,p.scope_kind,d.title,d.status,d.public_id,null,3
  from app.deliverables d join scopes p on p.id=d.project_id
  union all
  select c.public_id,'artifact',p.public_id,p.scope_kind,c.objective,case when c.status='current' and not app.context_pack_scopes_current(c.id) then 'stale' else c.status end,c.public_id,null,3
  from app.context_packs c join scopes p on p.id=c.project_id
  union all
  select a.public_id,'artifact',p.public_id,p.scope_kind,a.title,'recorded',null,null,3
  from app.artifacts a join scopes p on p.id=a.project_id
  union all
  select v.public_id,'artifact',p.public_id,p.scope_kind,v.title,
    case when v.id=d.current_version_id then v.status else 'superseded' end,v.public_id,null,3
  from app.artifact_documents d join app.artifact_document_versions v on v.document_id=d.id
  join scopes p on p.id=d.project_id
  union all
  select r.public_id,'external_reference',p.public_id,p.scope_kind,r.display_title,r.sync_status,null,r.canonical_url,4
  from app.external_references r join scopes p on p.id=r.project_id
  union all
  select o.public_id,'external_reference',p.public_id,p.scope_kind,r.display_title,o.observation_status,o.public_id,r.canonical_url,4
  from app.external_reference_observations o join app.external_references r on r.id=o.external_reference_id join scopes p on p.id=o.project_id
  where exists(select 1 from app.steward_scope_sources s where s.external_observation_id=o.id)
  union all
  select o.public_id,'external_reference',p.public_id,p.scope_kind,coalesce(o.snapshot->>'title',j.title),
    case when not exists(select 1 from app.publication_observations newer where newer.publication_job_id=j.id and newer.id>o.id) then o.observation_kind else 'stale' end,
    o.public_id,o.external_url,4
  from app.publication_observations o join app.publication_jobs j on j.id=o.publication_job_id join scopes p on p.id=j.project_id
  union all
  select f.public_id,'external_reference',p.public_id,p.scope_kind,c.repository||'@'||c.commit_sha||':'||f.path,
    case when exists(select 1 from app.github_code_file_observations newer join app.github_code_corpora nc on nc.id=newer.corpus_id
      where newer.project_id=f.project_id and newer.path=f.path and nc.repository=c.repository and newer.id>f.id) then 'stale' else f.status end,
    f.public_id,null,4
  from app.github_code_file_observations f join app.github_code_corpora c on c.id=f.corpus_id join scopes p on p.id=f.project_id
  union all
  select s.public_id,'session',p.public_id,p.scope_kind,s.title,s.status,null,null,3
  from app.sessions s join scopes p on p.id=s.project_id
  union all
  select t.public_id,'task',p.public_id,p.scope_kind,t.title,t.status,null,null,3
  from app.tasks t join scopes p on p.id=t.project_id
  union all
  select i.public_id,'insight',p.public_id,p.scope_kind,i.title,i.status,null,null,5
  from app.insights i join scopes p on p.id=i.project_id
)
select id,kind,project_public_id,scope_kind,label,status,version_public_id,source_url,
  (select version_number from (
    select v.version_number from app.knowledge_entry_versions v where v.public_id=objects.version_public_id and objects.kind='knowledge'
    union all select v.version from app.artifact_document_versions v where v.public_id=objects.version_public_id and objects.kind='artifact'
    union all select d.version from app.deliverables d where d.public_id=objects.version_public_id and objects.kind='artifact'
    union all select c.version from app.context_packs c where c.public_id=objects.version_public_id and objects.kind='artifact'
  ) exact_version limit 1) as version_number,
  case when kind='artifact' then coalesce(
    (select '/artifacts/'||d.public_id||'?version='||v.public_id from app.artifact_document_versions v join app.artifact_documents d on d.id=v.document_id where v.public_id=objects.id),
    (select '/projects/'||objects.project_public_id||'/deliverables/'||d.public_id from app.deliverables d where d.public_id=objects.id),
    (select '/projects/'||objects.project_public_id||'/sources/context_pack/'||c.public_id from app.context_packs c where c.public_id=objects.id),
    (select '/projects/'||objects.project_public_id||'/sources/artifact/'||a.public_id from app.artifacts a where a.public_id=objects.id)
  ) when kind in ('knowledge','task') then '/projects/'||project_public_id||'/sources/'||kind||'/'||id
  when kind='session' then '/projects/'||project_public_id||'/sessions/'||id
  when kind='external_reference' then (
    select '/projects/'||objects.project_public_id||'/code?observation='||c.public_id||'&file='||f.public_id
    from app.github_code_file_observations f join app.github_code_corpora c on c.id=f.corpus_id
    where f.public_id=objects.id
  )
  when kind='insight' then '/projects/'||project_public_id||'/insights/'||id else null end as app_path
from objects
order by case when project_public_id=$3::uuid then 0 else 1 end,priority,project_public_id,id limit $2
