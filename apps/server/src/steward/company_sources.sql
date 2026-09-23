with scopes as (select * from app.projects where workspace_id=app.current_workspace_id() and status='active'),
objects as (
 select v.id as source_id,p.id as source_project_id,p.public_id as source_project_public_id,
   'knowledge_entry_version'::text as source_kind,v.public_id as source_public_id,k.public_id as parent_public_id,
   case when p.id=$1 then 'focus/' else p.public_id::text||'/' end||n.node_key as node_key,
   v.entry_type,v.title,left(v.statement,6000) as statement,length(v.statement)>6000 as insufficient,
   case when p.id=$1 then 0 when p.scope_kind='company' then 1 else 2 end as priority,v.created_at as observed_at,'{}'::jsonb as provenance
 from app.knowledge_entries k join app.knowledge_entry_versions v on v.knowledge_entry_id=k.id and v.version_number=k.latest_version
 join scopes p on p.id=k.project_id join app.context_nodes n on n.id=k.context_node_id
 where k.status='confirmed' and v.status='confirmed' and v.entry_type<>'open_question'
 union all
 select v.id,p.id,p.public_id,'artifact_document_version',v.public_id,d.public_id,
   case when p.id=$1 then 'focus/' else p.public_id::text||'/' end||'artifact','decision',v.title,
   left(v.body_markdown||E'\n'||v.structured_content::text,6000),length(v.body_markdown||v.structured_content::text)>6000,
   case when p.id=$1 then 0 when p.scope_kind='company' then 1 else 2 end,v.created_at,'{}'::jsonb
 from app.artifact_document_versions v join app.artifact_documents d on d.id=v.document_id join scopes p on p.id=d.project_id
 where v.status='validated' and not exists(select 1 from app.artifact_document_versions newer where newer.document_id=d.id and newer.status='validated' and newer.version>v.version)
 union all
 select o.id,p.id,p.public_id,'external_reference_observation',o.public_id,r.public_id,
   case when p.id=$1 then 'focus/' else p.public_id::text||'/' end||'github-metadata','technical_rule',r.display_title,
   left(o.observed_state::text,6000),true,case when p.id=$1 then 0 else 2 end,o.observed_at,'{}'::jsonb
 from app.external_reference_observations o join app.external_references r on r.id=o.external_reference_id join scopes p on p.id=o.project_id
 where not exists(select 1 from app.external_reference_observations newer where newer.external_reference_id=r.id and newer.id>o.id)
 union all
 select o.id,p.id,p.public_id,'publication_observation',o.public_id,j.public_id,
   case when p.id=$1 then 'focus/' else p.public_id::text||'/' end||j.provider||'-document','decision',coalesce(o.snapshot->>'title',j.title),
   left(coalesce(o.snapshot->>'body_markdown','Source indisponible ou non relue'),6000),
   o.observation_kind='unavailable' or coalesce(o.snapshot->>'complete','false')<>'true' or length(coalesce(o.snapshot->>'body_markdown',''))>6000
     or length(btrim(coalesce(o.snapshot->>'body_markdown','')))=0,
   case when p.id=$1 then 0 else 2 end,o.observed_at,'{}'::jsonb
 from app.publication_observations o join app.publication_jobs j on j.id=o.publication_job_id join scopes p on p.id=j.project_id
 where not exists(select 1 from app.publication_observations newer where newer.publication_job_id=j.id and newer.id>o.id)
 union all
 select f.id,p.id,p.public_id,'github_code_file_observation',f.public_id,f.public_id,
   case when p.id=$1 then 'focus/' else p.public_id::text||'/' end||'github-code','technical_rule',
   c.repository||'@'||c.commit_sha||':'||f.path,
   left(coalesce(f.content_text,'Contenu non lu : '||f.status),6000),
   f.status<>'code_read' or not c.commit_verified or length(coalesce(f.content_text,''))>6000 or f.line_count=0,
   case when p.id=$1 then 0 else 2 end,f.observed_at,
   jsonb_build_object('repository',c.repository,'commit_sha',c.commit_sha,'commit_verified',c.commit_verified,
     'path',f.path,'status',f.status,'reason_code',f.reason_code,'content_hash',f.content_hash,'blob_sha',f.blob_sha,
     'line_start',case when f.status='code_read' and f.line_count>0 then 1 else null end,
     'line_end',case when f.status='code_read' and f.line_count>0 then least(f.line_count,
       case when right(left(f.content_text,6000),1)=E'\n' then 0 else 1 end
         +length(left(f.content_text,6000))-length(replace(left(f.content_text,6000),E'\n',''))) else null end,
     'excerpt_partial',length(coalesce(f.content_text,''))>6000,'repository_coverage','selected_file_only')
 from app.github_code_file_observations f join app.github_code_corpora c on c.id=f.corpus_id join scopes p on p.id=f.project_id
 where not exists(select 1 from app.github_code_file_observations newer
   join app.github_code_corpora nc on nc.id=newer.corpus_id where newer.project_id=f.project_id
     and nc.repository=c.repository and newer.path=f.path and newer.id>f.id)
)
select o.source_id,o.source_project_id,o.source_project_public_id,o.source_kind,o.source_public_id,o.parent_public_id,o.node_key,o.entry_type,o.title,o.statement,o.insufficient,o.provenance,p.graph_version,p.scope_kind,o.observed_at
 from objects o join scopes p on p.id=o.source_project_id
