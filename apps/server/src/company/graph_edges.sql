with scopes as (select * from app.projects where id=any($1) and workspace_id=app.current_workspace_id()),
stored as (
  select e.public_id as id,
    case e.source_kind when 'project' then 'scope' when 'context_node' then 'agent'
      when 'knowledge_entry_version' then 'knowledge' when 'knowledge_entry' then 'knowledge'
      when 'deliverable' then 'artifact' when 'context_pack' then 'artifact' when 'artifact_document_version' then 'artifact' when 'external_reference_observation' then 'external_reference' when 'publication_observation' then 'external_reference' when 'github_code_file_observation' then 'external_reference' else e.source_kind end as source_kind,
    case when e.source_kind='knowledge_entry' then (select v.public_id from app.knowledge_entries k
      join app.knowledge_entry_versions v on v.knowledge_entry_id=k.id and v.version_number=k.latest_version where k.public_id=e.source_public_id)
      else e.source_public_id end as source_public_id,
    p.public_id as source_project_public_id,
    case e.target_kind when 'project' then 'scope' when 'context_node' then 'agent'
      when 'knowledge_entry_version' then 'knowledge' when 'knowledge_entry' then 'knowledge'
      when 'deliverable' then 'artifact' when 'context_pack' then 'artifact' when 'artifact_document_version' then 'artifact' when 'external_reference_observation' then 'external_reference' when 'publication_observation' then 'external_reference' when 'github_code_file_observation' then 'external_reference' else e.target_kind end as target_kind,
    case when e.target_kind='knowledge_entry' then (select v.public_id from app.knowledge_entries k
      join app.knowledge_entry_versions v on v.knowledge_entry_id=k.id and v.version_number=k.latest_version where k.public_id=e.target_public_id)
      else e.target_public_id end as target_public_id,
    target.public_id as target_project_public_id,e.edge_type,e.status,
    jsonb_build_object('origin',case when e.provenance->>'origin'='human' then 'human' else 'recorded' end,
      'created_at',e.created_at,'persisted',true) as provenance
  from app.edges e join scopes p on p.id=e.project_id join scopes target on target.id=e.target_project_id
), sources as (
  select md5('pack-source:'||s.id::text)::uuid as id,'artifact'::text as source_kind,c.public_id as source_public_id,
    p.public_id as source_project_public_id,'knowledge'::text as target_kind,v.public_id as target_public_id,
    p.public_id as target_project_public_id,'derived_from'::text as edge_type,'confirmed'::text as status,
    jsonb_build_object('origin','source_record','persisted',true) as provenance
  from app.context_pack_sources s join app.context_packs c on c.id=s.context_pack_id
  join app.knowledge_entry_versions v on v.id=s.knowledge_entry_version_id join scopes p on p.id=c.project_id
  union all
  select md5('scoped-pack-source:'||s.id::text)::uuid,'artifact',c.public_id,p.public_id,
    case s.source_kind when 'knowledge_entry_version' then 'knowledge' else 'artifact' end,s.source_public_id,
    target.public_id,'derived_from','confirmed',jsonb_build_object('origin','source_record','persisted',true)
  from app.context_pack_scope_sources s join app.context_packs c on c.id=s.context_pack_id
  join scopes p on p.id=c.project_id join scopes target on target.id=s.source_project_id where s.decision='included'
  union all
  select md5('deliverable-source:'||s.id::text)::uuid,'artifact',d.public_id,p.public_id,
    case s.source_kind when 'knowledge_entry_version' then 'knowledge' else 'artifact' end,
    s.source_public_id,p.public_id,'derived_from','confirmed',jsonb_build_object('origin','source_record','persisted',true)
  from app.deliverable_sources s join app.deliverables d on d.id=s.deliverable_id join scopes p on p.id=d.project_id
  where s.source_kind in ('knowledge_entry_version','context_pack')
  union all
  select md5('artifact-document-source:'||s.id::text)::uuid,'artifact',v.public_id,p.public_id,
    case s.source_kind when 'knowledge' then 'knowledge' when 'session' then 'session' else 'artifact' end,s.source_public_id,
    source_scope.public_id,'derived_from','confirmed',jsonb_build_object('origin','source_record','persisted',true)
  from app.artifact_version_sources s join app.artifact_document_versions v on v.id=s.version_id
  join scopes p on p.id=v.project_id join scopes source_scope on source_scope.id=s.source_project_id
  union all
  select md5('steward-source:'||s.id::text)::uuid,'insight',i.public_id,p.public_id,
    case s.source_kind when 'knowledge_entry_version' then 'knowledge' when 'artifact_document_version' then 'artifact' else 'external_reference' end,
    s.source_public_id,target.public_id,'derived_from',
    case when app.steward_scope_sources_current(s.assessment_id) then 'confirmed' else 'stale' end,
    jsonb_build_object('origin','source_record','persisted',true)
  from app.steward_scope_sources s join app.insights i on i.steward_assessment_id=s.assessment_id
  join scopes p on p.id=i.project_id join scopes target on target.id=s.source_project_id
  union all
  select md5('session-pack:'||s.id::text)::uuid,'session',s.public_id,p.public_id,
    'artifact',c.public_id,p.public_id,'derived_from','confirmed',jsonb_build_object('origin','session_context','persisted',true)
  from app.sessions s join app.context_packs c on c.id=s.context_pack_id join scopes p on p.id=s.project_id
  union all
  select md5('task-pack:'||t.id::text)::uuid,'task',t.public_id,p.public_id,
    'artifact',c.public_id,p.public_id,'derived_from','confirmed',jsonb_build_object('origin','task_context','persisted',true)
  from app.tasks t join app.context_packs c on c.id=t.context_pack_id join scopes p on p.id=t.project_id
  union all
  select md5('deliverable-session:'||d.id::text)::uuid,'artifact',d.public_id,p.public_id,
    'session',s.public_id,p.public_id,'derived_from','confirmed',jsonb_build_object('origin','source_session','persisted',true)
  from app.deliverables d join app.sessions s on s.id=d.source_session_id join scopes p on p.id=d.project_id
), membership as (
  select md5('scope-agent:'||n.id::text)::uuid as id,'scope'::text as source_kind,p.public_id as source_public_id,
    p.public_id as source_project_public_id,'agent'::text as target_kind,n.public_id as target_public_id,
    p.public_id as target_project_public_id,'contains'::text as edge_type,'confirmed'::text as status,
    jsonb_build_object('origin','scope_membership','persisted',false) as provenance
  from app.context_nodes n join scopes p on p.id=n.project_id
  union all
  select md5('node-knowledge:'||v.id::text)::uuid,'agent',n.public_id,p.public_id,'knowledge',v.public_id,p.public_id,
    'contains','confirmed',jsonb_build_object('origin','scope_membership','persisted',false)
  from app.knowledge_entries k join app.knowledge_entry_versions v on v.knowledge_entry_id=k.id and v.version_number=k.latest_version
  join app.context_nodes n on n.id=k.context_node_id join scopes p on p.id=k.project_id
  union all
  select md5('agent-session:'||s.id::text)::uuid,'agent',n.public_id,p.public_id,'session',s.public_id,p.public_id,
    'contains','confirmed',jsonb_build_object('origin','scope_membership','persisted',false)
  from app.sessions s join app.context_nodes n on n.id=s.context_node_id join scopes p on p.id=s.project_id
  union all
  select md5('company-project:'||p.id::text)::uuid,'scope',company.public_id,company.public_id,'scope',p.public_id,p.public_id,
    'contains','confirmed',jsonb_build_object('origin','scope_membership','persisted',false)
  from scopes company join scopes p on p.workspace_id=company.workspace_id
  where company.scope_kind='company' and p.scope_kind='project'
)
select * from (select * from stored union all select * from sources union all select * from membership) graph_edges
where source_kind in ('scope','agent','knowledge','artifact','external_reference','insight','session','task')
  and target_kind in ('scope','agent','knowledge','artifact','external_reference','insight','session','task')
order by id limit $2
