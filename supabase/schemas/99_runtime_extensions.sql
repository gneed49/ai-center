-- Extensions are replayed after the baseline role reset by runtime-db-role.sh.
-- Keep this file privileges-only: role repair must not rerun schema DDL or data.
grant update(name, description) on app.workspaces to ai_center_runtime;
grant execute on function app.ensure_scope_agents(uuid) to ai_center_runtime;
grant execute on function app.create_company_workspace(uuid,text,text,text) to ai_center_runtime;
grant execute on function app.graph_endpoint_exists(text,uuid,bigint,bigint) to ai_center_runtime;

grant select, insert on app.artifact_documents, app.artifact_document_versions,
  app.artifact_version_sources to ai_center_runtime;
grant update(current_version_id, updated_at) on app.artifact_documents to ai_center_runtime;
grant select, insert, update on app.artifact_destination_settings to ai_center_runtime;
grant usage on sequence app.artifact_documents_id_seq, app.artifact_document_versions_id_seq,
  app.artifact_version_sources_id_seq, app.artifact_destination_settings_id_seq to ai_center_runtime;

grant select, insert on app.context_pack_scope_versions, app.context_pack_scope_sources to ai_center_runtime;
grant usage on sequence app.context_pack_scope_versions_id_seq, app.context_pack_scope_sources_id_seq to ai_center_runtime;
grant execute on function app.context_pack_scopes_current(bigint) to ai_center_runtime;

-- Company collaboration and durable external publication.

grant select,insert on app.work_tool_connections,app.publication_jobs,app.publication_observations to ai_center_runtime;
grant update(name,encrypted_credential,credential_actor_id,enabled,revision,updated_at) on app.work_tool_connections to ai_center_runtime;
grant update(status,error_code,external_id,external_url,updated_at) on app.publication_jobs to ai_center_runtime;
grant usage on sequence app.work_tool_connections_id_seq,app.publication_jobs_id_seq,app.publication_observations_id_seq to ai_center_runtime;
grant execute on function app.claim_publication_job(),app.finish_publication_job(uuid,uuid,text,text,jsonb) to ai_center_runtime;

grant select,insert on app.workspace_invitations to ai_center_runtime;
grant update(status,revoked_at) on app.workspace_invitations to ai_center_runtime;
grant usage on sequence app.workspace_invitations_id_seq to ai_center_runtime;
grant execute on function app.preview_workspace_invitation(uuid,text) to ai_center_runtime;
grant execute on function app.accept_workspace_invitation(uuid,text,text) to ai_center_runtime;
grant execute on function app.set_member_display_name(text) to ai_center_runtime;

grant select,insert on app.steward_scope_sources to ai_center_runtime;
grant usage on sequence app.steward_scope_sources_id_seq to ai_center_runtime;
grant execute on function app.steward_scope_sources_current(bigint) to ai_center_runtime;
grant execute on function app.list_due_steward_workspaces(integer) to ai_center_runtime;
grant execute on function app.steward_scope_source_status(bigint) to ai_center_runtime;
grant execute on function app.graph_endpoint_exists(text,uuid,bigint,bigint) to ai_center_runtime;

grant select,insert on app.github_code_corpora,app.github_code_file_observations to ai_center_runtime;
grant usage on sequence app.github_code_corpora_id_seq,app.github_code_file_observations_id_seq to ai_center_runtime;

grant select,insert on app.workspace_automation_controls,app.ai_call_reservations to ai_center_runtime;
grant update(enabled,generation,updated_by_actor_id,updated_at) on app.workspace_automation_controls to ai_center_runtime;
grant update(status,finished_at) on app.ai_call_reservations to ai_center_runtime;
grant usage on sequence app.workspace_automation_controls_id_seq,app.ai_call_reservations_id_seq to ai_center_runtime;
grant execute on function app.finish_ai_call_reservation(uuid,text) to ai_center_runtime;
grant execute on function app.cancel_model_run_after_access_loss(bigint) to ai_center_runtime;

-- Source frontier and public coverage, never an exposed worker capability.
grant select,insert,update on app.steward_scan_progress to ai_center_runtime;
grant select,insert on app.steward_scan_sources to ai_center_runtime;
grant usage on sequence app.steward_scan_progress_id_seq,app.steward_scan_sources_id_seq to ai_center_runtime;
