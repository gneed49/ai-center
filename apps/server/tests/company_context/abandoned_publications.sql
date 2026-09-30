-- [FICTIF] Invalid synthetic ciphertext is never decrypted or sent anywhere.
with document as (
 insert into app.artifact_documents(workspace_id,project_id,artifact_type,created_by_actor_id)
 select $1,id,'specification',$2 from app.projects where workspace_id=$1 and scope_kind='company'
 returning id,project_id
), version as (
 insert into app.artifact_document_versions(document_id,workspace_id,project_id,version,title,body_markdown,status,created_by_actor_id,validated_at,content_hash)
 select id,$1,project_id,1,'[FICTIF] Maintenance','Synthetic source','validated',$2,now(),repeat('d',64) from document
 returning id,project_id
), connection as (
 insert into app.work_tool_connections(public_id,workspace_id,provider,name,encrypted_credential,credential_actor_id)
 values(gen_random_uuid(),$1,'github','[FICTIF] No network',convert_to(repeat('FICTIF',8),'UTF8'),$2)
 returning id
)
insert into app.publication_jobs(workspace_id,project_id,artifact_version_id,connection_id,connection_revision,requested_by_actor_id,provider,target_id,title,body_markdown,content_hash,status,attempt_count,lease_token,lease_until,created_at,updated_at)
select $1,v.project_id,v.id,c.id,1,$2,'github','fictitious/'||s.status,'[FICTIF] Abandoned','Synthetic',repeat('e',64),s.status,
 case when s.status='queued' then 0 else 1 end,
 case when s.status='queued' then null else $3 end,
 case when s.status='queued' then null else now()-interval '20 minutes' end,
 now()-interval '25 minutes',now()-interval '25 minutes'
from version v cross join connection c cross join unnest(array['queued','processing','needs_review']) s(status)
returning public_id,status
