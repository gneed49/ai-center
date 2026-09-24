-- Exact immutable artifact sources for agent-generated drafts.
alter table app.artifact_version_sources add column source_artifact_version_id bigint;
alter table app.artifact_version_sources drop constraint artifact_version_sources_source_kind_check;
alter table app.artifact_version_sources add constraint artifact_version_sources_source_kind_check check (source_kind in ('knowledge','context_pack','deliverable','session','artifact_version'));
alter table app.artifact_version_sources drop constraint artifact_version_sources_check;
alter table app.artifact_version_sources add constraint artifact_version_sources_check check (
    (source_kind='knowledge' and knowledge_version_id is not null and context_pack_id is null and deliverable_id is null and session_id is null and source_artifact_version_id is null)
    or (source_kind='context_pack' and knowledge_version_id is null and context_pack_id is not null and deliverable_id is null and session_id is null and source_artifact_version_id is null)
    or (source_kind='deliverable' and knowledge_version_id is null and context_pack_id is null and deliverable_id is not null and session_id is null and source_artifact_version_id is null)
    or (source_kind='session' and knowledge_version_id is null and context_pack_id is null and deliverable_id is null and session_id is not null and source_artifact_version_id is null)
    or (source_kind='artifact_version' and knowledge_version_id is null and context_pack_id is null and deliverable_id is null and session_id is null and source_artifact_version_id is not null)
  );
alter table app.artifact_version_sources add constraint artifact_sources_artifact_project_fkey
  foreign key (source_artifact_version_id,source_project_id) references app.artifact_document_versions(id,project_id);
create index artifact_version_sources_artifact_idx on app.artifact_version_sources(source_artifact_version_id) where source_artifact_version_id is not null;
alter table app.model_runs drop constraint model_runs_operation_valid;
alter table app.model_runs add constraint model_runs_operation_valid check (operation in ('extract_knowledge','select_context','generate_technical_plan','assess_contradiction','assess_coverage','generate_artifact'));

alter table app.ai_call_reservations drop constraint ai_call_reservations_operation_check;
alter table app.ai_call_reservations add constraint ai_call_reservations_operation_check check(operation in ('respond','select_context','technical_plan','coverage','steward','artifact_draft'));
