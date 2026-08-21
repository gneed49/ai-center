create schema if not exists app;

revoke all on schema app from public, anon, authenticated;

create table app.workspaces (
  id bigint generated always as identity primary key,
  public_id uuid not null default gen_random_uuid() unique,
  owner_actor_id uuid not null,
  name text not null,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  constraint workspaces_name_not_blank check (btrim(name) <> '')
);

create table app.project_templates (
  id bigint generated always as identity primary key,
  public_id uuid not null default gen_random_uuid() unique,
  template_key text not null unique,
  name text not null,
  version integer not null default 1,
  definition jsonb not null default '{}'::jsonb,
  is_system boolean not null default true,
  created_at timestamptz not null default now(),
  constraint project_templates_version_positive check (version > 0),
  constraint project_templates_key_not_blank check (btrim(template_key) <> '')
);

create table app.agent_profiles (
  id bigint generated always as identity primary key,
  public_id uuid not null default gen_random_uuid() unique,
  template_id bigint not null references app.project_templates(id) on delete cascade,
  profile_key text not null,
  name text not null,
  scope_kind text not null,
  instructions text not null,
  retrieval_policy jsonb not null default '{}'::jsonb,
  output_schema jsonb not null default '{}'::jsonb,
  tools jsonb not null default '[]'::jsonb,
  created_at timestamptz not null default now(),
  constraint agent_profiles_template_key_unique unique (template_id, profile_key),
  constraint agent_profiles_scope_kind_valid check (scope_kind in ('product', 'tech', 'steward')),
  constraint agent_profiles_instructions_not_blank check (btrim(instructions) <> '')
);

create index agent_profiles_template_id_idx on app.agent_profiles(template_id);

create table app.deliverable_contracts (
  id bigint generated always as identity primary key,
  public_id uuid not null default gen_random_uuid() unique,
  template_id bigint not null references app.project_templates(id) on delete cascade,
  contract_key text not null,
  name text not null,
  required_sections text[] not null default '{}',
  accepted_evidence_types text[] not null default '{}',
  completion_rules jsonb not null default '{}'::jsonb,
  human_validation_required boolean not null default false,
  created_at timestamptz not null default now(),
  constraint deliverable_contracts_template_key_unique unique (template_id, contract_key)
);

create index deliverable_contracts_template_id_idx
  on app.deliverable_contracts(template_id);

create table app.projects (
  id bigint generated always as identity primary key,
  public_id uuid not null default gen_random_uuid() unique,
  workspace_id bigint not null references app.workspaces(id) on delete cascade,
  template_id bigint not null references app.project_templates(id),
  name text not null,
  objective text not null default '',
  summary text not null default '',
  status text not null default 'active',
  graph_version bigint not null default 0,
  created_by_actor_id uuid not null,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  constraint projects_status_valid check (status in ('active', 'archived')),
  constraint projects_name_not_blank check (btrim(name) <> ''),
  constraint projects_graph_version_nonnegative check (graph_version >= 0)
);

create index projects_workspace_status_created_idx
  on app.projects(workspace_id, status, created_at desc);
create index projects_template_id_idx on app.projects(template_id);

create table app.context_nodes (
  id bigint generated always as identity primary key,
  public_id uuid not null default gen_random_uuid() unique,
  workspace_id bigint not null references app.workspaces(id) on delete cascade,
  project_id bigint not null references app.projects(id) on delete cascade,
  parent_id bigint references app.context_nodes(id) on delete cascade,
  agent_profile_id bigint not null references app.agent_profiles(id),
  node_key text not null,
  title text not null,
  description text not null default '',
  summary text not null default '',
  context_rules jsonb not null default '{}'::jsonb,
  preferred_entry_types text[] not null default '{}',
  preferred_deliverable_types text[] not null default '{}',
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  constraint context_nodes_project_key_unique unique (project_id, node_key),
  constraint context_nodes_key_not_blank check (btrim(node_key) <> '')
);

create index context_nodes_workspace_id_idx on app.context_nodes(workspace_id);
create index context_nodes_project_id_idx on app.context_nodes(project_id);
create index context_nodes_parent_id_idx on app.context_nodes(parent_id);
create index context_nodes_agent_profile_id_idx on app.context_nodes(agent_profile_id);

create table app.knowledge_entries (
  id bigint generated always as identity primary key,
  public_id uuid not null default gen_random_uuid() unique,
  workspace_id bigint not null references app.workspaces(id) on delete cascade,
  project_id bigint not null references app.projects(id) on delete cascade,
  context_node_id bigint not null references app.context_nodes(id) on delete cascade,
  entry_type text not null,
  status text not null default 'confirmed',
  latest_version integer not null default 1,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  constraint knowledge_entries_type_valid check (
    entry_type in (
      'decision', 'business_rule', 'technical_rule', 'requirement',
      'acceptance_criterion', 'constraint', 'open_question'
    )
  ),
  constraint knowledge_entries_status_valid check (
    status in ('confirmed', 'superseded', 'archived')
  ),
  constraint knowledge_entries_latest_version_positive check (latest_version > 0)
);

create index knowledge_entries_project_node_type_idx
  on app.knowledge_entries(project_id, context_node_id, entry_type);
create index knowledge_entries_workspace_id_idx on app.knowledge_entries(workspace_id);

create table app.knowledge_entry_versions (
  id bigint generated always as identity primary key,
  public_id uuid not null default gen_random_uuid() unique,
  knowledge_entry_id bigint not null references app.knowledge_entries(id) on delete cascade,
  workspace_id bigint not null references app.workspaces(id) on delete cascade,
  project_id bigint not null references app.projects(id) on delete cascade,
  context_node_id bigint not null references app.context_nodes(id) on delete cascade,
  version_number integer not null,
  entry_type text not null,
  title text not null,
  statement text not null,
  rationale text not null default '',
  status text not null default 'confirmed',
  author_actor_id uuid not null,
  origin_type text not null,
  origin_public_id uuid,
  source_message_public_id uuid,
  created_at timestamptz not null default now(),
  constraint knowledge_entry_versions_entry_version_unique
    unique (knowledge_entry_id, version_number),
  constraint knowledge_entry_versions_version_positive check (version_number > 0),
  constraint knowledge_entry_versions_title_not_blank check (btrim(title) <> ''),
  constraint knowledge_entry_versions_statement_not_blank check (btrim(statement) <> ''),
  constraint knowledge_entry_versions_type_valid check (
    entry_type in (
      'decision', 'business_rule', 'technical_rule', 'requirement',
      'acceptance_criterion', 'constraint', 'open_question'
    )
  )
);

create index knowledge_entry_versions_entry_created_idx
  on app.knowledge_entry_versions(knowledge_entry_id, created_at desc);
create index knowledge_entry_versions_project_node_idx
  on app.knowledge_entry_versions(project_id, context_node_id);
create index knowledge_entry_versions_workspace_id_idx
  on app.knowledge_entry_versions(workspace_id);

create table app.edges (
  id bigint generated always as identity primary key,
  public_id uuid not null default gen_random_uuid() unique,
  workspace_id bigint not null references app.workspaces(id) on delete cascade,
  project_id bigint not null references app.projects(id) on delete cascade,
  source_kind text not null,
  source_public_id uuid not null,
  target_kind text not null,
  target_public_id uuid not null,
  edge_type text not null,
  status text not null default 'confirmed',
  provenance jsonb not null default '{}'::jsonb,
  created_by_actor_id uuid not null,
  created_at timestamptz not null default now(),
  constraint edges_relation_unique
    unique (project_id, source_public_id, target_public_id, edge_type),
  constraint edges_not_self check (
    source_kind <> target_kind or source_public_id <> target_public_id
  ),
  constraint edges_type_valid check (
    edge_type in (
      'references', 'depends_on', 'informs', 'supersedes', 'contradicts',
      'derived_from', 'satisfies', 'evidenced_by', 'implemented_by'
    )
  ),
  constraint edges_status_valid check (status in ('proposed', 'confirmed', 'rejected'))
);

create index edges_project_source_idx on app.edges(project_id, source_public_id);
create index edges_project_target_idx on app.edges(project_id, target_public_id);
create index edges_workspace_id_idx on app.edges(workspace_id);

create table app.context_packs (
  id bigint generated always as identity primary key,
  public_id uuid not null default gen_random_uuid() unique,
  workspace_id bigint not null references app.workspaces(id) on delete cascade,
  project_id bigint not null references app.projects(id) on delete cascade,
  source_node_id bigint not null references app.context_nodes(id),
  target_node_id bigint not null references app.context_nodes(id),
  target_agent_profile_id bigint not null references app.agent_profiles(id),
  task_kind text not null,
  objective text not null,
  content jsonb not null,
  version integer not null default 1,
  status text not null default 'current',
  compiled_at timestamptz not null default now(),
  invalidated_at timestamptz,
  constraint context_packs_version_positive check (version > 0),
  constraint context_packs_status_valid check (status in ('current', 'stale', 'superseded')),
  constraint context_packs_objective_not_blank check (btrim(objective) <> '')
);

create index context_packs_project_status_compiled_idx
  on app.context_packs(project_id, status, compiled_at desc);
create index context_packs_workspace_id_idx on app.context_packs(workspace_id);
create index context_packs_source_node_id_idx on app.context_packs(source_node_id);
create index context_packs_target_node_id_idx on app.context_packs(target_node_id);
create index context_packs_target_agent_profile_id_idx
  on app.context_packs(target_agent_profile_id);

create table app.context_pack_sources (
  id bigint generated always as identity primary key,
  context_pack_id bigint not null references app.context_packs(id) on delete cascade,
  knowledge_entry_id bigint not null references app.knowledge_entries(id),
  knowledge_entry_version_id bigint not null references app.knowledge_entry_versions(id),
  source_role text not null,
  included_reason text not null,
  constraint context_pack_sources_pack_version_unique
    unique (context_pack_id, knowledge_entry_version_id)
);

create index context_pack_sources_pack_id_idx on app.context_pack_sources(context_pack_id);
create index context_pack_sources_entry_id_idx on app.context_pack_sources(knowledge_entry_id);
create index context_pack_sources_version_id_idx
  on app.context_pack_sources(knowledge_entry_version_id);

create table app.sessions (
  id bigint generated always as identity primary key,
  public_id uuid not null default gen_random_uuid() unique,
  workspace_id bigint not null references app.workspaces(id) on delete cascade,
  project_id bigint not null references app.projects(id) on delete cascade,
  context_node_id bigint not null references app.context_nodes(id),
  agent_profile_id bigint not null references app.agent_profiles(id),
  context_pack_id bigint references app.context_packs(id),
  title text not null,
  status text not null default 'active',
  created_by_actor_id uuid not null,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  constraint sessions_status_valid check (status in ('active', 'completed', 'archived'))
);

create index sessions_project_updated_idx on app.sessions(project_id, updated_at desc);
create index sessions_workspace_id_idx on app.sessions(workspace_id);
create index sessions_context_node_id_idx on app.sessions(context_node_id);
create index sessions_agent_profile_id_idx on app.sessions(agent_profile_id);
create index sessions_context_pack_id_idx on app.sessions(context_pack_id);

create table app.messages (
  id bigint generated always as identity primary key,
  public_id uuid not null default gen_random_uuid() unique,
  workspace_id bigint not null references app.workspaces(id) on delete cascade,
  project_id bigint not null references app.projects(id) on delete cascade,
  session_id bigint not null references app.sessions(id) on delete cascade,
  role text not null,
  content text not null,
  agent_scope text,
  metadata jsonb not null default '{}'::jsonb,
  created_at timestamptz not null default now(),
  constraint messages_role_valid check (role in ('user', 'assistant', 'system')),
  constraint messages_content_not_blank check (btrim(content) <> '')
);

create index messages_session_created_idx on app.messages(session_id, created_at, id);
create index messages_project_id_idx on app.messages(project_id);
create index messages_workspace_id_idx on app.messages(workspace_id);

create table app.mutation_proposals (
  id bigint generated always as identity primary key,
  public_id uuid not null default gen_random_uuid() unique,
  workspace_id bigint not null references app.workspaces(id) on delete cascade,
  project_id bigint not null references app.projects(id) on delete cascade,
  session_id bigint not null references app.sessions(id) on delete cascade,
  source_message_id bigint not null references app.messages(id) on delete cascade,
  proposed_by_agent_profile_id bigint not null references app.agent_profiles(id),
  entry_type text not null,
  title text not null,
  statement text not null,
  rationale text not null default '',
  status text not null default 'proposed',
  source_data jsonb not null default '{}'::jsonb,
  decided_by_actor_id uuid,
  decided_at timestamptz,
  created_at timestamptz not null default now(),
  constraint mutation_proposals_status_valid check (
    status in ('proposed', 'confirmed', 'rejected')
  ),
  constraint mutation_proposals_entry_type_valid check (
    entry_type in (
      'decision', 'business_rule', 'technical_rule', 'requirement',
      'acceptance_criterion', 'constraint', 'open_question'
    )
  )
);

create index mutation_proposals_session_status_created_idx
  on app.mutation_proposals(session_id, status, created_at);
create index mutation_proposals_workspace_id_idx on app.mutation_proposals(workspace_id);
create index mutation_proposals_project_id_idx on app.mutation_proposals(project_id);
create index mutation_proposals_source_message_id_idx
  on app.mutation_proposals(source_message_id);
create index mutation_proposals_agent_profile_id_idx
  on app.mutation_proposals(proposed_by_agent_profile_id);

create table app.gates (
  id bigint generated always as identity primary key,
  public_id uuid not null default gen_random_uuid() unique,
  workspace_id bigint not null references app.workspaces(id) on delete cascade,
  project_id bigint not null references app.projects(id) on delete cascade,
  context_node_id bigint not null references app.context_nodes(id),
  gate_key text not null,
  status text not null default 'pending',
  evaluation jsonb not null default '{}'::jsonb,
  graph_version bigint not null,
  evaluated_at timestamptz not null default now(),
  constraint gates_project_key_graph_unique unique (project_id, gate_key, graph_version),
  constraint gates_status_valid check (
    status in ('pending', 'passed', 'passed_with_warning', 'blocked')
  )
);

create index gates_project_evaluated_idx on app.gates(project_id, evaluated_at desc);
create index gates_workspace_id_idx on app.gates(workspace_id);
create index gates_context_node_id_idx on app.gates(context_node_id);

create table app.handoffs (
  id bigint generated always as identity primary key,
  public_id uuid not null default gen_random_uuid() unique,
  workspace_id bigint not null references app.workspaces(id) on delete cascade,
  project_id bigint not null references app.projects(id) on delete cascade,
  source_session_id bigint not null references app.sessions(id),
  target_session_id bigint references app.sessions(id),
  context_pack_id bigint not null references app.context_packs(id),
  status text not null default 'ready',
  initiated_by_actor_id uuid not null,
  created_at timestamptz not null default now(),
  completed_at timestamptz,
  constraint handoffs_status_valid check (status in ('ready', 'completed', 'cancelled'))
);

create index handoffs_project_created_idx on app.handoffs(project_id, created_at desc);
create index handoffs_workspace_id_idx on app.handoffs(workspace_id);
create index handoffs_source_session_id_idx on app.handoffs(source_session_id);
create index handoffs_target_session_id_idx on app.handoffs(target_session_id);
create index handoffs_context_pack_id_idx on app.handoffs(context_pack_id);

create table app.deliverables (
  id bigint generated always as identity primary key,
  public_id uuid not null default gen_random_uuid() unique,
  workspace_id bigint not null references app.workspaces(id) on delete cascade,
  project_id bigint not null references app.projects(id) on delete cascade,
  context_node_id bigint not null references app.context_nodes(id),
  contract_id bigint not null references app.deliverable_contracts(id),
  source_session_id bigint references app.sessions(id),
  source_context_pack_id bigint references app.context_packs(id),
  deliverable_type text not null,
  title text not null,
  summary text not null default '',
  content jsonb not null,
  status text not null default 'draft',
  coverage_status text not null default 'missing',
  version integer not null default 1,
  committed_at timestamptz,
  stale_at timestamptz,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  constraint deliverables_version_positive check (version > 0),
  constraint deliverables_status_valid check (
    status in ('draft', 'committed', 'stale', 'superseded')
  ),
  constraint deliverables_coverage_status_valid check (
    coverage_status in ('covered', 'partial', 'missing')
  )
);

create index deliverables_project_type_created_idx
  on app.deliverables(project_id, deliverable_type, created_at desc);
create index deliverables_workspace_id_idx on app.deliverables(workspace_id);
create index deliverables_context_node_id_idx on app.deliverables(context_node_id);
create index deliverables_contract_id_idx on app.deliverables(contract_id);
create index deliverables_source_session_id_idx on app.deliverables(source_session_id);
create index deliverables_source_context_pack_id_idx
  on app.deliverables(source_context_pack_id);

create table app.deliverable_sections (
  id bigint generated always as identity primary key,
  public_id uuid not null default gen_random_uuid() unique,
  deliverable_id bigint not null references app.deliverables(id) on delete cascade,
  section_key text not null,
  title text not null,
  body text not null,
  ordinal integer not null,
  source_data jsonb not null default '{}'::jsonb,
  constraint deliverable_sections_deliverable_key_unique
    unique (deliverable_id, section_key),
  constraint deliverable_sections_ordinal_nonnegative check (ordinal >= 0)
);

create index deliverable_sections_deliverable_ordinal_idx
  on app.deliverable_sections(deliverable_id, ordinal);

create table app.tasks (
  id bigint generated always as identity primary key,
  public_id uuid not null default gen_random_uuid() unique,
  workspace_id bigint not null references app.workspaces(id) on delete cascade,
  project_id bigint not null references app.projects(id) on delete cascade,
  context_pack_id bigint not null references app.context_packs(id),
  contract_id bigint not null references app.deliverable_contracts(id),
  title text not null,
  status text not null default 'ready',
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  constraint tasks_status_valid check (
    status in ('ready', 'running', 'completed', 'failed', 'cancelled')
  )
);

create index tasks_project_status_created_idx
  on app.tasks(project_id, status, created_at desc);
create index tasks_workspace_id_idx on app.tasks(workspace_id);
create index tasks_context_pack_id_idx on app.tasks(context_pack_id);
create index tasks_contract_id_idx on app.tasks(contract_id);

create table app.executions (
  id bigint generated always as identity primary key,
  public_id uuid not null default gen_random_uuid() unique,
  workspace_id bigint not null references app.workspaces(id) on delete cascade,
  project_id bigint not null references app.projects(id) on delete cascade,
  task_id bigint not null references app.tasks(id) on delete cascade,
  executor_key text not null,
  status text not null default 'queued',
  result jsonb,
  started_at timestamptz,
  completed_at timestamptz,
  created_at timestamptz not null default now(),
  constraint executions_status_valid check (
    status in ('queued', 'running', 'completed', 'failed', 'cancelled')
  )
);

create index executions_task_created_idx on app.executions(task_id, created_at desc);
create index executions_project_status_created_idx
  on app.executions(project_id, status, created_at desc);
create index executions_workspace_id_idx on app.executions(workspace_id);

create table app.execution_events (
  id bigint generated always as identity primary key,
  public_id uuid not null default gen_random_uuid() unique,
  execution_id bigint not null references app.executions(id) on delete cascade,
  event_type text not null,
  sequence_number integer not null,
  payload jsonb not null default '{}'::jsonb,
  created_at timestamptz not null default now(),
  constraint execution_events_execution_sequence_unique
    unique (execution_id, sequence_number),
  constraint execution_events_sequence_positive check (sequence_number > 0)
);

create index execution_events_execution_sequence_idx
  on app.execution_events(execution_id, sequence_number);

create table app.artifacts (
  id bigint generated always as identity primary key,
  public_id uuid not null default gen_random_uuid() unique,
  workspace_id bigint not null references app.workspaces(id) on delete cascade,
  project_id bigint not null references app.projects(id) on delete cascade,
  execution_id bigint references app.executions(id),
  deliverable_id bigint references app.deliverables(id),
  artifact_type text not null,
  title text not null,
  reference text not null,
  metadata jsonb not null default '{}'::jsonb,
  created_at timestamptz not null default now()
);

create index artifacts_project_created_idx on app.artifacts(project_id, created_at desc);
create index artifacts_workspace_id_idx on app.artifacts(workspace_id);
create index artifacts_execution_id_idx on app.artifacts(execution_id);
create index artifacts_deliverable_id_idx on app.artifacts(deliverable_id);

create table app.evidences (
  id bigint generated always as identity primary key,
  public_id uuid not null default gen_random_uuid() unique,
  workspace_id bigint not null references app.workspaces(id) on delete cascade,
  project_id bigint not null references app.projects(id) on delete cascade,
  requirement_entry_id bigint not null references app.knowledge_entries(id),
  requirement_version_id bigint not null references app.knowledge_entry_versions(id),
  deliverable_id bigint references app.deliverables(id) on delete cascade,
  deliverable_section_id bigint references app.deliverable_sections(id) on delete cascade,
  artifact_id bigint references app.artifacts(id) on delete set null,
  evidence_type text not null,
  title text not null,
  description text not null default '',
  source_reference text not null,
  status text not null default 'valid',
  created_at timestamptz not null default now(),
  constraint evidences_status_valid check (status in ('valid', 'stale', 'rejected'))
);

create index evidences_requirement_status_idx
  on app.evidences(requirement_entry_id, status);
create index evidences_workspace_id_idx on app.evidences(workspace_id);
create index evidences_project_id_idx on app.evidences(project_id);
create index evidences_requirement_version_id_idx on app.evidences(requirement_version_id);
create index evidences_deliverable_id_idx on app.evidences(deliverable_id);
create index evidences_deliverable_section_id_idx on app.evidences(deliverable_section_id);
create index evidences_artifact_id_idx on app.evidences(artifact_id);

create table app.requirement_coverage (
  id bigint generated always as identity primary key,
  project_id bigint not null references app.projects(id) on delete cascade,
  requirement_entry_id bigint not null references app.knowledge_entries(id) on delete cascade,
  requirement_version_id bigint not null references app.knowledge_entry_versions(id),
  deliverable_id bigint not null references app.deliverables(id) on delete cascade,
  deliverable_section_id bigint references app.deliverable_sections(id) on delete cascade,
  evidence_id bigint references app.evidences(id) on delete set null,
  status text not null,
  explanation text not null default '',
  updated_at timestamptz not null default now(),
  constraint requirement_coverage_relation_unique
    unique (requirement_version_id, deliverable_id, deliverable_section_id),
  constraint requirement_coverage_status_valid check (
    status in ('covered', 'partial', 'missing')
  )
);

create index requirement_coverage_project_status_idx
  on app.requirement_coverage(project_id, status);
create index requirement_coverage_requirement_entry_id_idx
  on app.requirement_coverage(requirement_entry_id);
create index requirement_coverage_deliverable_id_idx
  on app.requirement_coverage(deliverable_id);
create index requirement_coverage_section_id_idx
  on app.requirement_coverage(deliverable_section_id);
create index requirement_coverage_evidence_id_idx on app.requirement_coverage(evidence_id);

create table app.insights (
  id bigint generated always as identity primary key,
  public_id uuid not null default gen_random_uuid() unique,
  workspace_id bigint not null references app.workspaces(id) on delete cascade,
  project_id bigint not null references app.projects(id) on delete cascade,
  insight_type text not null,
  status text not null default 'candidate',
  severity text not null,
  confidence numeric(4,3) not null,
  title text not null,
  explanation text not null,
  resolution_justification text,
  resolved_by_actor_id uuid,
  detected_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  constraint insights_type_valid check (insight_type in ('contradiction', 'coverage_gap')),
  constraint insights_status_valid check (
    status in ('candidate', 'open', 'accepted', 'resolved', 'dismissed')
  ),
  constraint insights_severity_valid check (severity in ('notice', 'warning', 'blocking')),
  constraint insights_confidence_range check (confidence >= 0 and confidence <= 1)
);

create index insights_project_status_detected_idx
  on app.insights(project_id, status, detected_at desc);
create index insights_workspace_id_idx on app.insights(workspace_id);

create table app.insight_sources (
  id bigint generated always as identity primary key,
  insight_id bigint not null references app.insights(id) on delete cascade,
  source_role text not null,
  object_kind text not null,
  object_public_id uuid not null,
  knowledge_entry_version_id bigint references app.knowledge_entry_versions(id),
  constraint insight_sources_relation_unique
    unique (insight_id, source_role, object_public_id)
);

create index insight_sources_insight_id_idx on app.insight_sources(insight_id);
create index insight_sources_version_id_idx
  on app.insight_sources(knowledge_entry_version_id);

create table app.domain_events (
  id bigint generated always as identity primary key,
  public_id uuid not null default gen_random_uuid() unique,
  workspace_id bigint not null references app.workspaces(id) on delete cascade,
  project_id bigint references app.projects(id) on delete cascade,
  event_type text not null,
  aggregate_kind text not null,
  aggregate_public_id uuid not null,
  payload jsonb not null default '{}'::jsonb,
  status text not null default 'pending',
  occurred_at timestamptz not null default now(),
  processed_at timestamptz,
  constraint domain_events_status_valid check (status in ('pending', 'processed', 'failed'))
);

create index domain_events_status_occurred_idx
  on app.domain_events(status, occurred_at, id);
create index domain_events_workspace_id_idx on app.domain_events(workspace_id);
create index domain_events_project_id_idx on app.domain_events(project_id);

create table app.audit_events (
  id bigint generated always as identity primary key,
  public_id uuid not null default gen_random_uuid() unique,
  workspace_id bigint not null references app.workspaces(id) on delete cascade,
  project_id bigint references app.projects(id) on delete cascade,
  actor_id uuid not null,
  action text not null,
  object_kind text not null,
  object_public_id uuid not null,
  before_state jsonb,
  after_state jsonb,
  occurred_at timestamptz not null default now()
);

create index audit_events_workspace_occurred_idx
  on app.audit_events(workspace_id, occurred_at desc, id desc);
create index audit_events_project_occurred_idx
  on app.audit_events(project_id, occurred_at desc, id desc);

create or replace function app.set_updated_at()
returns trigger
language plpgsql
set search_path = ''
as $$
begin
  new.updated_at = now();
  return new;
end;
$$;

revoke execute on function app.set_updated_at() from public, anon, authenticated;

create trigger workspaces_set_updated_at
before update on app.workspaces
for each row execute function app.set_updated_at();

create trigger projects_set_updated_at
before update on app.projects
for each row execute function app.set_updated_at();

create trigger context_nodes_set_updated_at
before update on app.context_nodes
for each row execute function app.set_updated_at();

create trigger knowledge_entries_set_updated_at
before update on app.knowledge_entries
for each row execute function app.set_updated_at();

create trigger sessions_set_updated_at
before update on app.sessions
for each row execute function app.set_updated_at();

create trigger deliverables_set_updated_at
before update on app.deliverables
for each row execute function app.set_updated_at();

create trigger tasks_set_updated_at
before update on app.tasks
for each row execute function app.set_updated_at();

create trigger requirement_coverage_set_updated_at
before update on app.requirement_coverage
for each row execute function app.set_updated_at();

create trigger insights_set_updated_at
before update on app.insights
for each row execute function app.set_updated_at();

create or replace function app.prevent_audit_mutation()
returns trigger
language plpgsql
set search_path = ''
as $$
begin
  raise exception 'audit_events are immutable';
end;
$$;

revoke execute on function app.prevent_audit_mutation() from public, anon, authenticated;

create trigger audit_events_prevent_update
before update or delete on app.audit_events
for each row execute function app.prevent_audit_mutation();

alter table app.workspaces enable row level security;
alter table app.project_templates enable row level security;
alter table app.agent_profiles enable row level security;
alter table app.deliverable_contracts enable row level security;
alter table app.projects enable row level security;
alter table app.context_nodes enable row level security;
alter table app.knowledge_entries enable row level security;
alter table app.knowledge_entry_versions enable row level security;
alter table app.edges enable row level security;
alter table app.context_packs enable row level security;
alter table app.context_pack_sources enable row level security;
alter table app.sessions enable row level security;
alter table app.messages enable row level security;
alter table app.mutation_proposals enable row level security;
alter table app.gates enable row level security;
alter table app.handoffs enable row level security;
alter table app.deliverables enable row level security;
alter table app.deliverable_sections enable row level security;
alter table app.tasks enable row level security;
alter table app.executions enable row level security;
alter table app.execution_events enable row level security;
alter table app.artifacts enable row level security;
alter table app.evidences enable row level security;
alter table app.requirement_coverage enable row level security;
alter table app.insights enable row level security;
alter table app.insight_sources enable row level security;
alter table app.domain_events enable row level security;
alter table app.audit_events enable row level security;
