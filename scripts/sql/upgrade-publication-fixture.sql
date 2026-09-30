-- [FICTIF] One historical aggregated issue, inserted before the ticket migration.
-- Only used inside the disposable upgrade database. No provider is contacted.
do $$
declare tenant app.workspaces; project bigint; document bigint; version bigint;
  connection bigint; job app.publication_jobs; observation app.publication_observations;
  identity uuid=gen_random_uuid(); prepared text;
begin
  select * into strict tenant from app.workspaces where public_id='91000000-0000-0000-0000-000000000001';
  perform set_config('app.current_workspace_id',tenant.id::text,true);
  perform set_config('app.current_actor_id',tenant.owner_actor_id::text,true);
  perform set_config('app.current_workspace_role','owner',true);
  insert into app.projects(workspace_id,template_id,name,objective,created_by_actor_id)
    select tenant.id,t.id,'[FICTIF] Legacy publication','[FICTIF] Preserve the aggregated issue',tenant.owner_actor_id
    from app.project_templates t where t.template_key='upgrade-legacy'
    returning id into project;
  insert into app.artifact_documents(workspace_id,project_id,artifact_type,created_by_actor_id)
    values(tenant.id,project,'product_tickets',tenant.owner_actor_id) returning id into document;
  insert into app.artifact_document_versions(document_id,workspace_id,project_id,version,title,body_markdown,
    status,created_by_actor_id,validated_at,content_hash)
    values(document,tenant.id,project,1,'[FICTIF] Legacy batch','[FICTIF] Two tickets in one document.',
      'validated',tenant.owner_actor_id,now(),repeat('1',64)) returning id into version;
  update app.artifact_documents set current_version_id=version where id=document;
  insert into app.work_tool_connections(public_id,workspace_id,provider,name,encrypted_credential,credential_actor_id)
    values(gen_random_uuid(),tenant.id,'linear','[FICTIF] Never contacted',decode(repeat('aa',32),'hex'),tenant.owner_actor_id)
    returning id into connection;
  prepared='[FICTIF] Historical document.'||E'\nAI Center publication: '||identity;
  insert into app.publication_jobs(public_id,workspace_id,project_id,artifact_version_id,connection_id,
    connection_revision,requested_by_actor_id,provider,target_id,title,body_markdown,content_hash,status,
    attempt_count,external_id,external_url)
    values(identity,tenant.id,project,version,connection,1,tenant.owner_actor_id,'linear',
      '93000000-0000-0000-0000-000000000001','[FICTIF] Legacy batch',prepared,repeat('1',64),'succeeded',1,
      '93000000-0000-0000-0000-000000000002','https://linear.app/fixture/issue/FICTIF-1')
    returning * into job;
  insert into app.publication_observations(workspace_id,publication_job_id,observation_kind,external_id,external_url,snapshot)
    values(tenant.id,job.id,'created',job.external_id,job.external_url,
      jsonb_build_object('title',job.title,'body_markdown',job.body_markdown,'complete',true))
    returning * into observation;
  insert into app.audit_events(workspace_id,project_id,actor_id,action,object_kind,object_public_id,after_state)
    values(tenant.id,project,tenant.owner_actor_id,'fixture.upgrade.publication','publication',identity,
      jsonb_build_object('job',to_jsonb(job),'observation',to_jsonb(observation)));
end;
$$;
