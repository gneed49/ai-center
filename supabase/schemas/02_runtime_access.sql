-- Runtime database privileges are part of the declarative schema state.
-- The application role receives only the operations used by the Axum API and
-- outbox worker. In particular it receives no DELETE, TRUNCATE, REFERENCES,
-- TRIGGER, MAINTAIN, schema CREATE, or DDL privilege.

revoke all privileges on schema app from ai_center_runtime;
grant usage on schema app to ai_center_runtime;

revoke all privileges on all tables in schema app from ai_center_runtime;
revoke all privileges on all sequences in schema app from ai_center_runtime;
revoke execute on all functions in schema app
  from public, anon, authenticated, ai_center_runtime;

grant select on table
  app.workspaces,
  app.workspace_members,
  app.project_templates,
  app.agent_profiles,
  app.deliverable_contracts,
  app.projects,
  app.context_nodes,
  app.knowledge_entries,
  app.knowledge_entry_versions,
  app.edges,
  app.context_packs,
  app.context_pack_sources,
  app.context_pack_selection_items,
  app.sessions,
  app.model_runs,
  app.messages,
  app.idempotency_records,
  app.mutation_proposals,
  app.gates,
  app.handoffs,
  app.deliverables,
  app.deliverable_sections,
  app.deliverable_sources,
  app.tool_connections,
  app.external_references,
  app.external_reference_observations,
  app.artifacts,
  app.evidences,
  app.requirement_coverage,
  app.steward_assessments,
  app.steward_assessment_sources,
  app.insights,
  app.insight_sources,
  app.insight_resolutions,
  app.domain_events,
  app.audit_events
to ai_center_runtime;

grant insert on table
  app.workspace_members,
  app.projects,
  app.context_nodes,
  app.knowledge_entries,
  app.knowledge_entry_versions,
  app.edges,
  app.context_packs,
  app.context_pack_sources,
  app.context_pack_selection_items,
  app.sessions,
  app.model_runs,
  app.messages,
  app.idempotency_records,
  app.mutation_proposals,
  app.gates,
  app.handoffs,
  app.deliverables,
  app.deliverable_sections,
  app.deliverable_sources,
  app.external_references,
  app.external_reference_observations,
  app.evidences,
  app.requirement_coverage,
  app.steward_assessments,
  app.steward_assessment_sources,
  app.insights,
  app.insight_sources,
  app.insight_resolutions,
  app.domain_events,
  app.audit_events
to ai_center_runtime;

grant update on table
  app.workspace_members,
  app.projects,
  app.sessions,
  app.idempotency_records,
  app.mutation_proposals,
  app.context_packs,
  app.gates,
  app.model_runs,
  app.deliverables,
  app.external_references,
  app.evidences,
  app.requirement_coverage,
  app.insights,
  app.knowledge_entries,
  app.domain_events
to ai_center_runtime;

grant usage on sequence
  app.workspace_members_id_seq,
  app.projects_id_seq,
  app.context_nodes_id_seq,
  app.knowledge_entries_id_seq,
  app.knowledge_entry_versions_id_seq,
  app.edges_id_seq,
  app.context_packs_id_seq,
  app.context_pack_sources_id_seq,
  app.context_pack_selection_items_id_seq,
  app.sessions_id_seq,
  app.model_runs_id_seq,
  app.messages_id_seq,
  app.idempotency_records_id_seq,
  app.mutation_proposals_id_seq,
  app.gates_id_seq,
  app.handoffs_id_seq,
  app.deliverables_id_seq,
  app.deliverable_sections_id_seq,
  app.deliverable_sources_id_seq,
  app.external_references_id_seq,
  app.external_reference_observations_id_seq,
  app.evidences_id_seq,
  app.requirement_coverage_id_seq,
  app.steward_assessments_id_seq,
  app.steward_assessment_sources_id_seq,
  app.insights_id_seq,
  app.insight_sources_id_seq,
  app.insight_resolutions_id_seq,
  app.domain_events_id_seq,
  app.audit_events_id_seq
to ai_center_runtime;

grant execute on function app.current_actor_id() to ai_center_runtime;
grant execute on function app.current_workspace_id() to ai_center_runtime;
grant execute on function app.current_workspace_role() to ai_center_runtime;
grant execute on function app.has_workspace_role(bigint, text[])
  to ai_center_runtime;
grant execute on function app.authorize_workspace_member(uuid, uuid)
  to ai_center_runtime;
grant execute on function app.list_actor_workspaces() to ai_center_runtime;
grant execute on function app.list_due_steward_workspaces(integer)
  to ai_center_runtime;

-- External tracking describes read-only GitHub observations; no runner.
grant select, insert on table app.tasks, app.executions, app.execution_events, app.artifacts to ai_center_runtime;
grant update on table app.tasks, app.executions to ai_center_runtime;
grant usage on sequence app.tasks_id_seq, app.executions_id_seq, app.execution_events_id_seq, app.artifacts_id_seq to ai_center_runtime;
