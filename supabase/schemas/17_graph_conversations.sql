-- Conversations participate in validated scoped graph relationships.
create or replace function app.graph_endpoint_exists(endpoint_kind text, endpoint_id uuid, scope_id bigint, tenant_id bigint)
returns boolean language plpgsql stable security invoker set search_path = '' as $$
begin
  case endpoint_kind
    when 'project' then return exists(select 1 from app.projects where id=scope_id and public_id=endpoint_id and workspace_id=tenant_id);
    when 'context_node' then return exists(select 1 from app.context_nodes where public_id=endpoint_id and project_id=scope_id and workspace_id=tenant_id);
    when 'knowledge_entry' then return exists(select 1 from app.knowledge_entries where public_id=endpoint_id and project_id=scope_id and workspace_id=tenant_id);
    when 'knowledge_entry_version' then return exists(select 1 from app.knowledge_entry_versions where public_id=endpoint_id and project_id=scope_id and workspace_id=tenant_id);
    when 'deliverable' then return exists(select 1 from app.deliverables where public_id=endpoint_id and project_id=scope_id and workspace_id=tenant_id);
    when 'context_pack' then return exists(select 1 from app.context_packs where public_id=endpoint_id and project_id=scope_id and workspace_id=tenant_id);
    when 'artifact' then return exists(select 1 from app.artifacts where public_id=endpoint_id and project_id=scope_id and workspace_id=tenant_id);
    when 'artifact_document_version' then return exists(select 1 from app.artifact_document_versions where public_id=endpoint_id and project_id=scope_id and workspace_id=tenant_id);
    when 'external_reference' then return exists(select 1 from app.external_references where public_id=endpoint_id and project_id=scope_id and workspace_id=tenant_id);
    when 'external_reference_observation' then return exists(select 1 from app.external_reference_observations where public_id=endpoint_id and project_id=scope_id and workspace_id=tenant_id);
    when 'publication_observation' then return exists(select 1 from app.publication_observations o join app.publication_jobs j on j.id=o.publication_job_id where o.public_id=endpoint_id and j.project_id=scope_id and o.workspace_id=tenant_id);
    when 'session' then return exists(select 1 from app.sessions where public_id=endpoint_id and project_id=scope_id and workspace_id=tenant_id);
    when 'task' then return exists(select 1 from app.tasks where public_id=endpoint_id and project_id=scope_id and workspace_id=tenant_id);
    when 'execution' then return exists(select 1 from app.executions where public_id=endpoint_id and project_id=scope_id and workspace_id=tenant_id);
    when 'insight' then return exists(select 1 from app.insights where public_id=endpoint_id and project_id=scope_id and workspace_id=tenant_id);
    when 'github_code_file_observation' then return exists(select 1 from app.github_code_file_observations where public_id=endpoint_id and project_id=scope_id and workspace_id=tenant_id);
    else return false;
  end case;
end;
$$;
revoke all on function app.graph_endpoint_exists(text,uuid,bigint,bigint) from public,anon,authenticated,service_role;
grant execute on function app.graph_endpoint_exists(text,uuid,bigint,bigint) to ai_center_runtime;
