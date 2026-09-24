with refs as (
 select source_kind,source_public_id,source_project_id from app.context_pack_scope_sources where project_id=$1
 union
 select s.source_kind,s.source_public_id,s.source_project_id from app.steward_scope_sources s where s.project_id=$1
 union
 select case s.source_kind when 'knowledge' then 'knowledge_entry_version' else s.source_kind end,s.source_public_id,s.source_project_id
 from app.artifact_version_sources s join app.artifact_document_versions v on v.id=s.version_id where v.project_id=$1 and s.source_kind<>'session'
 union
 select 'knowledge_entry_version',v.public_id,v.project_id from app.model_runs m join app.knowledge_entry_versions v on v.public_id=any(m.source_public_ids)
 where m.project_id=$1 and v.workspace_id=app.current_workspace_id()
 union
 select 'artifact_document_version',v.public_id,v.project_id from app.model_runs m join app.artifact_document_versions v on v.public_id=any(m.source_public_ids)
 where m.project_id=$1 and v.workspace_id=app.current_workspace_id()
), records as (
 select 'knowledge_entry_version'::text kind,v.public_id,v.project_id,jsonb_build_object('version_number',v.version_number,'entry_type',v.entry_type,'title',v.title,'statement',v.statement,'rationale',v.rationale,'created_at',v.created_at) record
 from app.knowledge_entry_versions v where v.workspace_id=app.current_workspace_id()
 union all select 'artifact_document_version',v.public_id,v.project_id,jsonb_build_object('document_id',d.public_id,'version',v.version,'title',v.title,'body_markdown',v.body_markdown,'structured_content',v.structured_content,'status',v.status,'content_hash',v.content_hash,'created_at',v.created_at)
 from app.artifact_document_versions v join app.artifact_documents d on d.id=v.document_id where v.workspace_id=app.current_workspace_id()
 union all select 'context_pack',p.public_id,p.project_id,jsonb_build_object('version',p.version,'content_hash',p.content_hash,'content',p.content,'status',p.status,'compiled_at',p.compiled_at)
 from app.context_packs p where p.workspace_id=app.current_workspace_id()
 union all select 'deliverable',d.public_id,d.project_id,jsonb_build_object('version',d.version,'title',d.title,'content',d.content,'content_hash',d.content_hash,'status',d.status,'created_at',d.created_at)
 from app.deliverables d where d.workspace_id=app.current_workspace_id()
 union all select 'external_reference_observation',o.public_id,o.project_id,jsonb_build_object('content_hash',o.content_hash,'status',o.observation_status,'observed_state',o.observed_state,'observed_at',o.observed_at,'source_url',r.canonical_url)
 from app.external_reference_observations o join app.external_references r on r.id=o.external_reference_id where o.workspace_id=app.current_workspace_id()
 union all select 'publication_observation',o.public_id,j.project_id,jsonb_build_object('observed_at',o.observed_at,'kind',o.observation_kind,'snapshot',o.snapshot,'source_url',o.external_url,
   'publication_id',j.public_id,'artifact_id',d.public_id,'artifact_version_id',v.public_id,'source_ticket_index',j.source_ticket_index)
 from app.publication_observations o join app.publication_jobs j on j.id=o.publication_job_id
 join app.artifact_document_versions v on v.id=j.artifact_version_id join app.artifact_documents d on d.id=v.document_id
 where o.workspace_id=app.current_workspace_id()
 union all select 'github_code_file_observation',f.public_id,f.project_id,jsonb_build_object('repository',c.repository,'commit_sha',c.commit_sha,'commit_verified',c.commit_verified,'path',f.path,'status',f.status,'reason_code',f.reason_code,'content_hash',f.content_hash,'blob_sha',f.blob_sha,'content_text',f.content_text,'line_count',f.line_count,'observed_at',f.observed_at)
 from app.github_code_file_observations f join app.github_code_corpora c on c.id=f.corpus_id where f.workspace_id=app.current_workspace_id()
), exported as (
 select r.source_kind,r.source_public_id,p.public_id source_project_public_id,o.record
 from refs r join records o on o.kind=r.source_kind and o.public_id=r.source_public_id and o.project_id=r.source_project_id
 join app.projects p on p.id=r.source_project_id
 where r.source_project_id<>$1
 union all
 select 'session_origin_receipt',s.source_public_id,p.public_id,s.snapshot
 from app.artifact_version_sources s join app.artifact_document_versions v on v.id=s.version_id join app.projects p on p.id=s.source_project_id
 where v.project_id=$1 and s.source_kind='session' and s.source_project_id<>$1
)
select source_kind,source_public_id,source_project_public_id,record from exported order by source_kind,source_public_id,record::text
