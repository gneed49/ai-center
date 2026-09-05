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

create table app.workspace_members (
  id bigint generated always as identity primary key,
  public_id uuid not null default gen_random_uuid() unique,
  workspace_id bigint not null references app.workspaces(id) on delete cascade,
  actor_id uuid not null,
  role text not null default 'viewer',
  invitation_status text not null default 'pending',
  invited_by_actor_id uuid,
  accepted_at timestamptz,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  constraint workspace_members_workspace_actor_unique unique (workspace_id, actor_id),
  constraint workspace_members_role_valid check (role in ('owner', 'editor', 'viewer')),
  constraint workspace_members_invitation_status_valid check (
    invitation_status in ('pending', 'accepted', 'revoked')
  ),
  constraint workspace_members_acceptance_consistent check (
    (invitation_status = 'accepted' and accepted_at is not null)
    or (invitation_status = 'pending' and accepted_at is null)
    or invitation_status = 'revoked'
  )
);

create index workspace_members_actor_status_idx
  on app.workspace_members(actor_id, invitation_status, workspace_id);

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
      'derived_from', 'satisfies', 'evidenced_by', 'implemented_by', 'tracked_by'
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
  source_graph_version bigint not null,
  compiler_version text not null,
  selection_mode text not null default 'deterministic',
  content_hash text not null,
  token_budget integer not null default 12000,
  estimated_tokens integer not null default 0,
  supersedes_context_pack_id bigint references app.context_packs(id),
  stale_reason text,
  compiled_at timestamptz not null default now(),
  invalidated_at timestamptz,
  constraint context_packs_target_version_unique
    unique (project_id, target_node_id, task_kind, version),
  constraint context_packs_version_positive check (version > 0),
  constraint context_packs_graph_version_nonnegative check (source_graph_version >= 0),
  constraint context_packs_compiler_version_not_blank check (btrim(compiler_version) <> ''),
  constraint context_packs_content_hash_valid check (content_hash ~ '^[0-9a-f]{64}$'),
  constraint context_packs_token_budget_positive check (token_budget > 0),
  constraint context_packs_estimated_tokens_nonnegative check (estimated_tokens >= 0),
  constraint context_packs_within_token_budget check (estimated_tokens <= token_budget),
  constraint context_packs_selection_mode_valid check (
    selection_mode in ('deterministic', 'hybrid')
  ),
  constraint context_packs_status_valid check (status in ('current', 'stale', 'superseded')),
  constraint context_packs_objective_not_blank check (btrim(objective) <> ''),
  constraint context_packs_invalidation_consistent check (
    (status = 'current' and invalidated_at is null and stale_reason is null)
    or (status <> 'current' and invalidated_at is not null and stale_reason is not null)
  )
);

create index context_packs_project_status_compiled_idx
  on app.context_packs(project_id, status, compiled_at desc);
create index context_packs_workspace_id_idx on app.context_packs(workspace_id);
create index context_packs_source_node_id_idx on app.context_packs(source_node_id);
create index context_packs_target_node_id_idx on app.context_packs(target_node_id);
create index context_packs_target_agent_profile_id_idx
  on app.context_packs(target_agent_profile_id);
create index context_packs_supersedes_id_idx
  on app.context_packs(supersedes_context_pack_id)
  where supersedes_context_pack_id is not null;
create unique index context_packs_one_current_target_idx
  on app.context_packs(project_id, target_node_id, task_kind)
  where status = 'current';

create table app.context_pack_sources (
  id bigint generated always as identity primary key,
  workspace_id bigint not null references app.workspaces(id) on delete cascade,
  project_id bigint not null references app.projects(id) on delete cascade,
  context_pack_id bigint not null references app.context_packs(id) on delete cascade,
  knowledge_entry_id bigint not null references app.knowledge_entries(id),
  knowledge_entry_version_id bigint not null references app.knowledge_entry_versions(id),
  source_role text not null,
  included_reason text not null,
  constraint context_pack_sources_pack_version_unique
    unique (context_pack_id, knowledge_entry_version_id)
);

create index context_pack_sources_pack_id_idx on app.context_pack_sources(context_pack_id);
create index context_pack_sources_workspace_id_idx on app.context_pack_sources(workspace_id);
create index context_pack_sources_project_id_idx on app.context_pack_sources(project_id);
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

create table app.model_runs (
  id bigint generated always as identity primary key,
  public_id uuid not null default gen_random_uuid() unique,
  workspace_id bigint not null references app.workspaces(id) on delete cascade,
  project_id bigint references app.projects(id) on delete cascade,
  session_id bigint references app.sessions(id) on delete set null,
  context_pack_id bigint references app.context_packs(id) on delete set null,
  operation text not null,
  provider text not null,
  model text not null,
  prompt_version text not null,
  schema_version text not null,
  source_graph_version bigint,
  input_hash text not null,
  source_public_ids uuid[] not null default '{}',
  provider_response_id text,
  status text not null default 'pending',
  output jsonb,
  usage jsonb not null default '{}'::jsonb,
  input_tokens integer,
  output_tokens integer,
  estimated_cost numeric(12, 6),
  latency_ms integer,
  attempt_count integer not null default 0,
  error_class text,
  error_message text,
  started_at timestamptz,
  completed_at timestamptz,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  constraint model_runs_operation_valid check (
    operation in (
      'extract_knowledge', 'select_context', 'generate_technical_plan',
      'assess_contradiction', 'assess_coverage'
    )
  ),
  constraint model_runs_status_valid check (
    status in ('pending', 'running', 'completed', 'failed', 'cancelled')
  ),
  constraint model_runs_provider_not_blank check (btrim(provider) <> ''),
  constraint model_runs_model_not_blank check (btrim(model) <> ''),
  constraint model_runs_prompt_version_not_blank check (btrim(prompt_version) <> ''),
  constraint model_runs_schema_version_not_blank check (btrim(schema_version) <> ''),
  constraint model_runs_input_hash_valid check (input_hash ~ '^[0-9a-f]{64}$'),
  constraint model_runs_project_links_consistent check (
    project_id is not null or (session_id is null and context_pack_id is null)
  ),
  constraint model_runs_source_graph_version_nonnegative check (
    source_graph_version is null or source_graph_version >= 0
  ),
  constraint model_runs_input_tokens_nonnegative check (input_tokens is null or input_tokens >= 0),
  constraint model_runs_output_tokens_nonnegative check (output_tokens is null or output_tokens >= 0),
  constraint model_runs_estimated_cost_nonnegative check (
    estimated_cost is null or estimated_cost >= 0
  ),
  constraint model_runs_latency_nonnegative check (latency_ms is null or latency_ms >= 0),
  constraint model_runs_attempt_count_nonnegative check (attempt_count >= 0),
  constraint model_runs_completion_consistent check (
    (
      status in ('pending', 'running')
      and completed_at is null
    )
    or (
      status = 'completed'
      and started_at is not null
      and completed_at is not null
      and output is not null
      and error_class is null
    )
    or (
      status = 'failed'
      and started_at is not null
      and completed_at is not null
      and error_class is not null
    )
    or (
      status = 'cancelled'
      and completed_at is not null
    )
  )
);

create index model_runs_project_operation_created_idx
  on app.model_runs(project_id, operation, created_at desc)
  where project_id is not null;
create index model_runs_workspace_status_created_idx
  on app.model_runs(workspace_id, status, created_at desc);
create index model_runs_session_id_idx on app.model_runs(session_id)
  where session_id is not null;
create index model_runs_context_pack_id_idx on app.model_runs(context_pack_id)
  where context_pack_id is not null;

create table app.messages (
  id bigint generated always as identity primary key,
  public_id uuid not null default gen_random_uuid() unique,
  workspace_id bigint not null references app.workspaces(id) on delete cascade,
  project_id bigint not null references app.projects(id) on delete cascade,
  session_id bigint not null references app.sessions(id) on delete cascade,
  client_message_id uuid,
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
create unique index messages_session_role_client_id_idx
  on app.messages(session_id, role, client_message_id)
  where client_message_id is not null;

create table app.idempotency_records (
  id bigint generated always as identity primary key,
  public_id uuid not null default gen_random_uuid() unique,
  workspace_id bigint not null references app.workspaces(id) on delete cascade,
  project_id bigint references app.projects(id) on delete cascade,
  actor_id uuid not null,
  operation_key text not null,
  idempotency_key text not null,
  request_hash text not null,
  status text not null default 'processing',
  response_status smallint,
  response_body jsonb,
  error_code text,
  locked_until timestamptz default (now() + interval '30 seconds'),
  expires_at timestamptz not null default (now() + interval '24 hours'),
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  lease_generation uuid not null default gen_random_uuid(),
  constraint idempotency_records_scope_key_unique
    unique (workspace_id, actor_id, operation_key, idempotency_key),
  constraint idempotency_records_operation_not_blank check (btrim(operation_key) <> ''),
  constraint idempotency_records_key_not_blank check (btrim(idempotency_key) <> ''),
  constraint idempotency_records_request_hash_valid check (request_hash ~ '^[0-9a-f]{64}$'),
  constraint idempotency_records_status_valid check (
    status in ('processing', 'completed', 'failed')
  ),
  constraint idempotency_records_response_status_valid check (
    response_status is null or response_status between 100 and 599
  ),
  constraint idempotency_records_result_consistent check (
    (
      status = 'processing'
      and response_status is null
      and error_code is null
      and locked_until is not null
    )
    or (
      status = 'completed'
      and response_status is not null
      and error_code is null
      and locked_until is null
    )
    or (
      status = 'failed'
      and error_code is not null
      and locked_until is null
    )
  ),
  constraint idempotency_records_expiry_valid check (expires_at > created_at)
);

create index idempotency_records_workspace_expiry_idx
  on app.idempotency_records(workspace_id, expires_at);
create index idempotency_records_project_id_idx
  on app.idempotency_records(project_id)
  where project_id is not null;
create index idempotency_records_reclaimable_idx
  on app.idempotency_records(locked_until, created_at)
  where status = 'processing';

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
  lineage_public_id uuid not null default gen_random_uuid(),
  supersedes_deliverable_id bigint references app.deliverables(id),
  source_graph_version bigint not null,
  content_hash text not null,
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
  constraint deliverables_lineage_version_unique
    unique (project_id, lineage_public_id, version),
  constraint deliverables_supersedes_unique unique (supersedes_deliverable_id),
  constraint deliverables_version_positive check (version > 0),
  constraint deliverables_source_graph_version_nonnegative check (source_graph_version >= 0),
  constraint deliverables_content_hash_valid check (content_hash ~ '^[0-9a-f]{64}$'),
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
create index deliverables_lineage_created_idx
  on app.deliverables(project_id, lineage_public_id, created_at desc);
create unique index deliverables_one_current_lineage_idx
  on app.deliverables(project_id, lineage_public_id)
  where status <> 'superseded';

create table app.deliverable_sections (
  id bigint generated always as identity primary key,
  public_id uuid not null default gen_random_uuid() unique,
  workspace_id bigint not null references app.workspaces(id) on delete cascade,
  project_id bigint not null references app.projects(id) on delete cascade,
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
create index deliverable_sections_workspace_id_idx on app.deliverable_sections(workspace_id);
create index deliverable_sections_project_id_idx on app.deliverable_sections(project_id);

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
  workspace_id bigint not null references app.workspaces(id) on delete cascade,
  project_id bigint not null references app.projects(id) on delete cascade,
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
create index execution_events_workspace_id_idx on app.execution_events(workspace_id);
create index execution_events_project_id_idx on app.execution_events(project_id);

create table app.tool_connections (
  id bigint generated always as identity primary key,
  public_id uuid not null default gen_random_uuid() unique,
  workspace_id bigint not null references app.workspaces(id) on delete cascade,
  provider text not null,
  auth_mode text not null,
  external_account_id text not null,
  display_name text not null,
  capabilities text[] not null default '{}',
  secret_reference text not null,
  configuration jsonb not null default '{}'::jsonb,
  status text not null default 'pending',
  created_by_actor_id uuid not null,
  last_verified_at timestamptz,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  constraint tool_connections_workspace_provider_account_unique
    unique (workspace_id, provider, external_account_id),
  constraint tool_connections_provider_valid check (provider in ('github')),
  constraint tool_connections_auth_mode_valid check (auth_mode in ('github_app')),
  constraint tool_connections_external_account_not_blank check (
    btrim(external_account_id) <> ''
  ),
  constraint tool_connections_display_name_not_blank check (btrim(display_name) <> ''),
  constraint tool_connections_secret_reference_not_blank check (
    btrim(secret_reference) <> ''
  ),
  constraint tool_connections_status_valid check (
    status in ('pending', 'active', 'degraded', 'revoked')
  ),
  constraint tool_connections_configuration_is_object check (
    jsonb_typeof(configuration) = 'object'
  )
);

create index tool_connections_workspace_status_idx
  on app.tool_connections(workspace_id, status, created_at desc);

create table app.external_references (
  id bigint generated always as identity primary key,
  public_id uuid not null default gen_random_uuid() unique,
  workspace_id bigint not null references app.workspaces(id) on delete cascade,
  project_id bigint not null references app.projects(id) on delete cascade,
  tool_connection_id bigint not null references app.tool_connections(id),
  provider text not null,
  object_kind text not null,
  external_id text not null,
  canonical_url text not null,
  repository_full_name text not null,
  display_title text not null,
  sync_status text not null default 'pending',
  etag text,
  last_synced_at timestamptz,
  last_error_code text,
  last_error_message text,
  created_by_actor_id uuid not null,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  constraint external_references_project_provider_external_unique
    unique (project_id, provider, external_id),
  constraint external_references_provider_valid check (provider in ('github')),
  constraint external_references_object_kind_valid check (
    object_kind in ('repository', 'pull_request', 'commit', 'check_run')
  ),
  constraint external_references_external_id_not_blank check (btrim(external_id) <> ''),
  constraint external_references_canonical_url_github check (
    canonical_url ~ '^https://github\.com/[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+(?:/.*)?$'
  ),
  constraint external_references_repository_full_name_valid check (
    repository_full_name ~ '^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+$'
  ),
  constraint external_references_display_title_not_blank check (btrim(display_title) <> ''),
  constraint external_references_sync_status_valid check (
    sync_status in ('pending', 'current', 'stale', 'unavailable', 'error')
  )
);

create index external_references_project_status_updated_idx
  on app.external_references(project_id, sync_status, updated_at desc);
create index external_references_workspace_id_idx on app.external_references(workspace_id);
create index external_references_tool_connection_id_idx
  on app.external_references(tool_connection_id);

create table app.external_reference_observations (
  id bigint generated always as identity primary key,
  public_id uuid not null default gen_random_uuid() unique,
  workspace_id bigint not null references app.workspaces(id) on delete cascade,
  project_id bigint not null references app.projects(id) on delete cascade,
  external_reference_id bigint not null references app.external_references(id) on delete cascade,
  observation_status text not null,
  content_hash text not null,
  etag text,
  observed_state jsonb not null default '{}'::jsonb,
  provider_updated_at timestamptz,
  observed_at timestamptz not null default now(),
  constraint external_reference_observations_status_valid check (
    observation_status in ('current', 'stale', 'unavailable')
  ),
  constraint external_reference_observations_content_hash_valid check (
    content_hash ~ '^[0-9a-f]{64}$'
  ),
  constraint external_reference_observations_state_is_object check (
    jsonb_typeof(observed_state) = 'object'
  )
);

create index external_reference_observations_reference_observed_idx
  on app.external_reference_observations(external_reference_id, id desc);
create index external_reference_observations_workspace_id_idx
  on app.external_reference_observations(workspace_id);
create index external_reference_observations_project_id_idx
  on app.external_reference_observations(project_id);

create table app.context_pack_selection_items (
  id bigint generated always as identity primary key,
  public_id uuid not null default gen_random_uuid() unique,
  workspace_id bigint not null references app.workspaces(id) on delete cascade,
  project_id bigint not null references app.projects(id) on delete cascade,
  context_pack_id bigint not null references app.context_packs(id) on delete cascade,
  candidate_kind text not null,
  candidate_public_id uuid not null,
  knowledge_entry_version_id bigint references app.knowledge_entry_versions(id),
  external_reference_observation_id bigint references app.external_reference_observations(id),
  decision text not null,
  reason_code text not null,
  explanation text not null default '',
  rank integer,
  estimated_tokens integer not null default 0,
  is_mandatory boolean not null default false,
  created_at timestamptz not null default now(),
  constraint context_pack_selection_items_candidate_unique
    unique (context_pack_id, candidate_kind, candidate_public_id),
  constraint context_pack_selection_items_candidate_kind_valid check (
    candidate_kind in ('knowledge_entry_version', 'external_reference_observation')
  ),
  constraint context_pack_selection_items_typed_source_consistent check (
    (
      candidate_kind = 'knowledge_entry_version'
      and knowledge_entry_version_id is not null
      and external_reference_observation_id is null
    )
    or (
      candidate_kind = 'external_reference_observation'
      and knowledge_entry_version_id is null
      and external_reference_observation_id is not null
    )
  ),
  constraint context_pack_selection_items_decision_valid check (
    decision in ('included', 'excluded')
  ),
  constraint context_pack_selection_items_reason_code_not_blank check (
    btrim(reason_code) <> ''
  ),
  constraint context_pack_selection_items_rank_nonnegative check (rank is null or rank >= 0),
  constraint context_pack_selection_items_tokens_nonnegative check (estimated_tokens >= 0),
  constraint context_pack_selection_items_mandatory_included check (
    not is_mandatory or decision = 'included'
  )
);

create index context_pack_selection_items_pack_decision_rank_idx
  on app.context_pack_selection_items(context_pack_id, decision, rank);
create index context_pack_selection_items_workspace_id_idx
  on app.context_pack_selection_items(workspace_id);
create index context_pack_selection_items_project_id_idx
  on app.context_pack_selection_items(project_id);
create index context_pack_selection_items_knowledge_version_id_idx
  on app.context_pack_selection_items(knowledge_entry_version_id)
  where knowledge_entry_version_id is not null;
create index context_pack_selection_items_external_observation_id_idx
  on app.context_pack_selection_items(external_reference_observation_id)
  where external_reference_observation_id is not null;

create table app.deliverable_sources (
  id bigint generated always as identity primary key,
  public_id uuid not null default gen_random_uuid() unique,
  workspace_id bigint not null references app.workspaces(id) on delete cascade,
  project_id bigint not null references app.projects(id) on delete cascade,
  deliverable_id bigint not null references app.deliverables(id) on delete cascade,
  source_kind text not null,
  source_public_id uuid not null,
  knowledge_entry_version_id bigint references app.knowledge_entry_versions(id),
  context_pack_id bigint references app.context_packs(id),
  external_reference_observation_id bigint references app.external_reference_observations(id),
  model_run_id bigint references app.model_runs(id),
  source_role text not null,
  included_reason text not null default '',
  created_at timestamptz not null default now(),
  constraint deliverable_sources_relation_unique
    unique (deliverable_id, source_kind, source_public_id),
  constraint deliverable_sources_kind_valid check (
    source_kind in (
      'knowledge_entry_version', 'context_pack',
      'external_reference_observation', 'model_run'
    )
  ),
  constraint deliverable_sources_typed_source_consistent check (
    (
      source_kind = 'knowledge_entry_version'
      and knowledge_entry_version_id is not null
      and context_pack_id is null
      and external_reference_observation_id is null
      and model_run_id is null
    )
    or (
      source_kind = 'context_pack'
      and knowledge_entry_version_id is null
      and context_pack_id is not null
      and external_reference_observation_id is null
      and model_run_id is null
    )
    or (
      source_kind = 'external_reference_observation'
      and knowledge_entry_version_id is null
      and context_pack_id is null
      and external_reference_observation_id is not null
      and model_run_id is null
    )
    or (
      source_kind = 'model_run'
      and knowledge_entry_version_id is null
      and context_pack_id is null
      and external_reference_observation_id is null
      and model_run_id is not null
    )
  ),
  constraint deliverable_sources_role_not_blank check (btrim(source_role) <> '')
);

create index deliverable_sources_deliverable_id_idx
  on app.deliverable_sources(deliverable_id);
create index deliverable_sources_workspace_id_idx on app.deliverable_sources(workspace_id);
create index deliverable_sources_project_id_idx on app.deliverable_sources(project_id);
create index deliverable_sources_knowledge_version_id_idx
  on app.deliverable_sources(knowledge_entry_version_id)
  where knowledge_entry_version_id is not null;
create index deliverable_sources_context_pack_id_idx
  on app.deliverable_sources(context_pack_id)
  where context_pack_id is not null;
create index deliverable_sources_external_observation_id_idx
  on app.deliverable_sources(external_reference_observation_id)
  where external_reference_observation_id is not null;
create index deliverable_sources_model_run_id_idx
  on app.deliverable_sources(model_run_id)
  where model_run_id is not null;

create table app.artifacts (
  id bigint generated always as identity primary key,
  public_id uuid not null default gen_random_uuid() unique,
  workspace_id bigint not null references app.workspaces(id) on delete cascade,
  project_id bigint not null references app.projects(id) on delete cascade,
  execution_id bigint references app.executions(id),
  deliverable_id bigint references app.deliverables(id),
  external_reference_id bigint references app.external_references(id) on delete set null,
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
create index artifacts_external_reference_id_idx
  on app.artifacts(external_reference_id)
  where external_reference_id is not null;

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
  external_reference_id bigint references app.external_references(id) on delete set null,
  evidence_type text not null,
  title text not null,
  description text not null default '',
  source_reference text not null,
  status text not null default 'candidate',
  created_at timestamptz not null default now(),
  constraint evidences_status_valid check (
    status in ('candidate', 'valid', 'stale', 'unavailable', 'rejected')
  )
);

create index evidences_requirement_status_idx
  on app.evidences(requirement_entry_id, status);
create index evidences_workspace_id_idx on app.evidences(workspace_id);
create index evidences_project_id_idx on app.evidences(project_id);
create index evidences_requirement_version_id_idx on app.evidences(requirement_version_id);
create index evidences_deliverable_id_idx on app.evidences(deliverable_id);
create index evidences_deliverable_section_id_idx on app.evidences(deliverable_section_id);
create index evidences_artifact_id_idx on app.evidences(artifact_id);
create index evidences_external_reference_id_idx
  on app.evidences(external_reference_id)
  where external_reference_id is not null;

create table app.requirement_coverage (
  id bigint generated always as identity primary key,
  workspace_id bigint not null references app.workspaces(id) on delete cascade,
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
create index requirement_coverage_workspace_id_idx
  on app.requirement_coverage(workspace_id);
create index requirement_coverage_requirement_entry_id_idx
  on app.requirement_coverage(requirement_entry_id);
create index requirement_coverage_deliverable_id_idx
  on app.requirement_coverage(deliverable_id);
create index requirement_coverage_section_id_idx
  on app.requirement_coverage(deliverable_section_id);
create index requirement_coverage_evidence_id_idx on app.requirement_coverage(evidence_id);

create table app.steward_assessments (
  id bigint generated always as identity primary key,
  public_id uuid not null default gen_random_uuid() unique,
  workspace_id bigint not null references app.workspaces(id) on delete cascade,
  project_id bigint not null references app.projects(id) on delete cascade,
  model_run_id bigint references app.model_runs(id) on delete set null,
  graph_version bigint not null,
  fingerprint text not null,
  classification text not null,
  severity text,
  confidence numeric(4, 3) not null,
  title text not null,
  explanation text not null,
  created_at timestamptz not null default now(),
  constraint steward_assessments_project_fingerprint_graph_unique
    unique (project_id, fingerprint, graph_version),
  constraint steward_assessments_graph_version_nonnegative check (graph_version >= 0),
  constraint steward_assessments_fingerprint_valid check (fingerprint ~ '^[0-9a-f]{64}$'),
  constraint steward_assessments_classification_valid check (
    classification in ('contradiction', 'compatible', 'ambiguous')
  ),
  constraint steward_assessments_severity_valid check (
    severity is null or severity in ('notice', 'warning', 'blocking')
  ),
  constraint steward_assessments_contradiction_severity_required check (
    classification <> 'contradiction' or severity is not null
  ),
  constraint steward_assessments_confidence_range check (confidence >= 0 and confidence <= 1),
  constraint steward_assessments_title_not_blank check (btrim(title) <> ''),
  constraint steward_assessments_explanation_not_blank check (btrim(explanation) <> '')
);

create index steward_assessments_project_classification_created_idx
  on app.steward_assessments(project_id, classification, created_at desc);
create index steward_assessments_workspace_id_idx on app.steward_assessments(workspace_id);
create index steward_assessments_model_run_id_idx
  on app.steward_assessments(model_run_id)
  where model_run_id is not null;

create table app.steward_assessment_sources (
  id bigint generated always as identity primary key,
  workspace_id bigint not null references app.workspaces(id) on delete cascade,
  project_id bigint not null references app.projects(id) on delete cascade,
  assessment_id bigint not null references app.steward_assessments(id) on delete cascade,
  source_role text not null,
  knowledge_entry_version_id bigint not null references app.knowledge_entry_versions(id),
  constraint steward_assessment_sources_role_unique
    unique (assessment_id, source_role),
  constraint steward_assessment_sources_role_valid check (source_role in ('left', 'right'))
);

create index steward_assessment_sources_workspace_id_idx
  on app.steward_assessment_sources(workspace_id);
create index steward_assessment_sources_project_id_idx
  on app.steward_assessment_sources(project_id);
create index steward_assessment_sources_assessment_id_idx
  on app.steward_assessment_sources(assessment_id);
create index steward_assessment_sources_version_id_idx
  on app.steward_assessment_sources(knowledge_entry_version_id);

create table app.insights (
  id bigint generated always as identity primary key,
  public_id uuid not null default gen_random_uuid() unique,
  workspace_id bigint not null references app.workspaces(id) on delete cascade,
  project_id bigint not null references app.projects(id) on delete cascade,
  steward_assessment_id bigint references app.steward_assessments(id) on delete set null,
  insight_type text not null,
  status text not null default 'candidate',
  severity text not null,
  confidence numeric(4,3) not null,
  title text not null,
  explanation text not null,
  resolution_justification text,
  resolved_by_actor_id uuid,
  resolved_at timestamptz,
  detected_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  constraint insights_type_valid check (insight_type in ('contradiction', 'coverage_gap')),
  constraint insights_status_valid check (
    status in ('candidate', 'open', 'accepted', 'resolved', 'dismissed')
  ),
  constraint insights_severity_valid check (severity in ('notice', 'warning', 'blocking')),
  constraint insights_confidence_range check (confidence >= 0 and confidence <= 1),
  constraint insights_resolution_consistent check (
    (
      status in ('candidate', 'open')
      and resolution_justification is null
      and resolved_by_actor_id is null
      and resolved_at is null
    )
    or (
      status in ('accepted', 'resolved', 'dismissed')
      and resolution_justification is not null
      and resolved_by_actor_id is not null
      and resolved_at is not null
    )
  )
);

create index insights_project_status_detected_idx
  on app.insights(project_id, status, detected_at desc);
create index insights_workspace_id_idx on app.insights(workspace_id);
create unique index insights_steward_assessment_id_idx
  on app.insights(steward_assessment_id)
  where steward_assessment_id is not null;

create table app.insight_sources (
  id bigint generated always as identity primary key,
  workspace_id bigint not null references app.workspaces(id) on delete cascade,
  project_id bigint not null references app.projects(id) on delete cascade,
  insight_id bigint not null references app.insights(id) on delete cascade,
  source_role text not null,
  object_kind text not null,
  object_public_id uuid not null,
  knowledge_entry_version_id bigint references app.knowledge_entry_versions(id),
  constraint insight_sources_relation_unique
    unique (insight_id, source_role, object_public_id)
);

create index insight_sources_insight_id_idx on app.insight_sources(insight_id);
create index insight_sources_workspace_id_idx on app.insight_sources(workspace_id);
create index insight_sources_project_id_idx on app.insight_sources(project_id);
create index insight_sources_version_id_idx
  on app.insight_sources(knowledge_entry_version_id);

create table app.insight_resolutions (
  id bigint generated always as identity primary key,
  public_id uuid not null default gen_random_uuid() unique,
  workspace_id bigint not null references app.workspaces(id) on delete cascade,
  project_id bigint not null references app.projects(id) on delete cascade,
  insight_id bigint not null references app.insights(id) on delete cascade,
  action text not null,
  knowledge_entry_version_id bigint references app.knowledge_entry_versions(id),
  source_graph_version bigint not null,
  resulting_graph_version bigint not null,
  justification text not null,
  resolved_by_actor_id uuid not null,
  created_at timestamptz not null default now(),
  constraint insight_resolutions_insight_unique unique (insight_id),
  constraint insight_resolutions_action_valid check (
    action in ('accept', 'dismiss', 'resolve')
  ),
  constraint insight_resolutions_graph_versions_valid check (
    source_graph_version >= 0
    and resulting_graph_version >= source_graph_version
  ),
  constraint insight_resolutions_mutation_consistent check (
    (
      action = 'resolve'
      and knowledge_entry_version_id is not null
      and resulting_graph_version > source_graph_version
    )
    or (
      action in ('accept', 'dismiss')
      and knowledge_entry_version_id is null
      and resulting_graph_version = source_graph_version
    )
  ),
  constraint insight_resolutions_justification_not_blank check (btrim(justification) <> '')
);

create index insight_resolutions_workspace_id_idx on app.insight_resolutions(workspace_id);
create index insight_resolutions_project_id_idx on app.insight_resolutions(project_id);
create index insight_resolutions_knowledge_version_id_idx
  on app.insight_resolutions(knowledge_entry_version_id)
  where knowledge_entry_version_id is not null;

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
  attempt_count integer not null default 0,
  available_at timestamptz not null default now(),
  locked_at timestamptz,
  locked_until timestamptz,
  locked_by text,
  last_error_code text,
  last_error_message text,
  occurred_at timestamptz not null default now(),
  processed_at timestamptz,
  failed_at timestamptz,
  constraint domain_events_status_valid check (
    status in ('pending', 'processing', 'processed', 'dead_letter')
  ),
  constraint domain_events_attempt_count_nonnegative check (attempt_count >= 0),
  constraint domain_events_lease_consistent check (
    (status = 'processing' and locked_at is not null and locked_until is not null and locked_by is not null)
    or (status <> 'processing' and locked_at is null and locked_until is null and locked_by is null)
  ),
  constraint domain_events_completion_consistent check (
    (status = 'processed' and processed_at is not null and failed_at is null)
    or (status = 'dead_letter' and processed_at is null and failed_at is not null)
    or (status in ('pending', 'processing') and processed_at is null and failed_at is null)
  )
);

create index domain_events_pending_available_idx
  on app.domain_events(available_at, occurred_at, id)
  where status = 'pending';
create index domain_events_processing_lease_idx
  on app.domain_events(locked_until, id)
  where status = 'processing';
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

alter table app.projects
  add constraint projects_id_workspace_unique unique (id, workspace_id);
alter table app.context_nodes
  add constraint context_nodes_id_project_unique unique (id, project_id);
alter table app.knowledge_entries
  add constraint knowledge_entries_id_project_unique unique (id, project_id);
alter table app.knowledge_entry_versions
  add constraint knowledge_entry_versions_id_project_unique unique (id, project_id);
alter table app.context_packs
  add constraint context_packs_id_project_unique unique (id, project_id);
alter table app.sessions
  add constraint sessions_id_project_unique unique (id, project_id);
alter table app.messages
  add constraint messages_id_project_unique unique (id, project_id);
alter table app.model_runs
  add constraint model_runs_id_project_unique unique (id, project_id);
alter table app.deliverables
  add constraint deliverables_id_project_unique unique (id, project_id);
alter table app.deliverable_sections
  add constraint deliverable_sections_id_project_unique unique (id, project_id);
alter table app.tasks
  add constraint tasks_id_project_unique unique (id, project_id);
alter table app.executions
  add constraint executions_id_project_unique unique (id, project_id);
alter table app.tool_connections
  add constraint tool_connections_id_workspace_unique unique (id, workspace_id);
alter table app.external_references
  add constraint external_references_id_project_unique unique (id, project_id);
alter table app.external_reference_observations
  add constraint external_reference_observations_id_project_unique unique (id, project_id);
alter table app.artifacts
  add constraint artifacts_id_project_unique unique (id, project_id);
alter table app.evidences
  add constraint evidences_id_project_unique unique (id, project_id);
alter table app.steward_assessments
  add constraint steward_assessments_id_project_unique unique (id, project_id);
alter table app.insights
  add constraint insights_id_project_unique unique (id, project_id);

alter table app.context_nodes
  add constraint context_nodes_project_workspace_fkey
  foreign key (project_id, workspace_id) references app.projects(id, workspace_id)
  on delete cascade;
alter table app.knowledge_entries
  add constraint knowledge_entries_project_workspace_fkey
  foreign key (project_id, workspace_id) references app.projects(id, workspace_id)
  on delete cascade;
alter table app.knowledge_entry_versions
  add constraint knowledge_entry_versions_project_workspace_fkey
  foreign key (project_id, workspace_id) references app.projects(id, workspace_id)
  on delete cascade;
alter table app.edges
  add constraint edges_project_workspace_fkey
  foreign key (project_id, workspace_id) references app.projects(id, workspace_id)
  on delete cascade;
alter table app.context_packs
  add constraint context_packs_project_workspace_fkey
  foreign key (project_id, workspace_id) references app.projects(id, workspace_id)
  on delete cascade;
alter table app.context_pack_sources
  add constraint context_pack_sources_project_workspace_fkey
  foreign key (project_id, workspace_id) references app.projects(id, workspace_id)
  on delete cascade;
alter table app.context_pack_selection_items
  add constraint context_pack_selection_items_project_workspace_fkey
  foreign key (project_id, workspace_id) references app.projects(id, workspace_id)
  on delete cascade;
alter table app.sessions
  add constraint sessions_project_workspace_fkey
  foreign key (project_id, workspace_id) references app.projects(id, workspace_id)
  on delete cascade;
alter table app.model_runs
  add constraint model_runs_project_workspace_fkey
  foreign key (project_id, workspace_id) references app.projects(id, workspace_id)
  on delete cascade;
alter table app.messages
  add constraint messages_project_workspace_fkey
  foreign key (project_id, workspace_id) references app.projects(id, workspace_id)
  on delete cascade;
alter table app.idempotency_records
  add constraint idempotency_records_project_workspace_fkey
  foreign key (project_id, workspace_id) references app.projects(id, workspace_id)
  on delete cascade;
alter table app.mutation_proposals
  add constraint mutation_proposals_project_workspace_fkey
  foreign key (project_id, workspace_id) references app.projects(id, workspace_id)
  on delete cascade;
alter table app.gates
  add constraint gates_project_workspace_fkey
  foreign key (project_id, workspace_id) references app.projects(id, workspace_id)
  on delete cascade;
alter table app.handoffs
  add constraint handoffs_project_workspace_fkey
  foreign key (project_id, workspace_id) references app.projects(id, workspace_id)
  on delete cascade;
alter table app.deliverables
  add constraint deliverables_project_workspace_fkey
  foreign key (project_id, workspace_id) references app.projects(id, workspace_id)
  on delete cascade;
alter table app.deliverable_sections
  add constraint deliverable_sections_project_workspace_fkey
  foreign key (project_id, workspace_id) references app.projects(id, workspace_id)
  on delete cascade;
alter table app.deliverable_sources
  add constraint deliverable_sources_project_workspace_fkey
  foreign key (project_id, workspace_id) references app.projects(id, workspace_id)
  on delete cascade;
alter table app.tasks
  add constraint tasks_project_workspace_fkey
  foreign key (project_id, workspace_id) references app.projects(id, workspace_id)
  on delete cascade;
alter table app.executions
  add constraint executions_project_workspace_fkey
  foreign key (project_id, workspace_id) references app.projects(id, workspace_id)
  on delete cascade;
alter table app.execution_events
  add constraint execution_events_project_workspace_fkey
  foreign key (project_id, workspace_id) references app.projects(id, workspace_id)
  on delete cascade;
alter table app.external_references
  add constraint external_references_project_workspace_fkey
  foreign key (project_id, workspace_id) references app.projects(id, workspace_id)
  on delete cascade;
alter table app.external_reference_observations
  add constraint external_reference_observations_project_workspace_fkey
  foreign key (project_id, workspace_id) references app.projects(id, workspace_id)
  on delete cascade;
alter table app.artifacts
  add constraint artifacts_project_workspace_fkey
  foreign key (project_id, workspace_id) references app.projects(id, workspace_id)
  on delete cascade;
alter table app.evidences
  add constraint evidences_project_workspace_fkey
  foreign key (project_id, workspace_id) references app.projects(id, workspace_id)
  on delete cascade;
alter table app.requirement_coverage
  add constraint requirement_coverage_project_workspace_fkey
  foreign key (project_id, workspace_id) references app.projects(id, workspace_id)
  on delete cascade;
alter table app.steward_assessments
  add constraint steward_assessments_project_workspace_fkey
  foreign key (project_id, workspace_id) references app.projects(id, workspace_id)
  on delete cascade;
alter table app.steward_assessment_sources
  add constraint steward_assessment_sources_project_workspace_fkey
  foreign key (project_id, workspace_id) references app.projects(id, workspace_id)
  on delete cascade;
alter table app.insights
  add constraint insights_project_workspace_fkey
  foreign key (project_id, workspace_id) references app.projects(id, workspace_id)
  on delete cascade;
alter table app.insight_sources
  add constraint insight_sources_project_workspace_fkey
  foreign key (project_id, workspace_id) references app.projects(id, workspace_id)
  on delete cascade;
alter table app.insight_resolutions
  add constraint insight_resolutions_project_workspace_fkey
  foreign key (project_id, workspace_id) references app.projects(id, workspace_id)
  on delete cascade;
alter table app.domain_events
  add constraint domain_events_project_workspace_fkey
  foreign key (project_id, workspace_id) references app.projects(id, workspace_id)
  on delete cascade;
alter table app.audit_events
  add constraint audit_events_project_workspace_fkey
  foreign key (project_id, workspace_id) references app.projects(id, workspace_id)
  on delete cascade;

alter table app.context_nodes
  add constraint context_nodes_parent_project_fkey
  foreign key (parent_id, project_id) references app.context_nodes(id, project_id);
alter table app.knowledge_entries
  add constraint knowledge_entries_context_node_project_fkey
  foreign key (context_node_id, project_id) references app.context_nodes(id, project_id);
alter table app.knowledge_entry_versions
  add constraint knowledge_entry_versions_entry_project_fkey
  foreign key (knowledge_entry_id, project_id) references app.knowledge_entries(id, project_id);
alter table app.knowledge_entry_versions
  add constraint knowledge_entry_versions_context_node_project_fkey
  foreign key (context_node_id, project_id) references app.context_nodes(id, project_id);
alter table app.context_packs
  add constraint context_packs_source_node_project_fkey
  foreign key (source_node_id, project_id) references app.context_nodes(id, project_id);
alter table app.context_packs
  add constraint context_packs_target_node_project_fkey
  foreign key (target_node_id, project_id) references app.context_nodes(id, project_id);
alter table app.context_packs
  add constraint context_packs_supersedes_project_fkey
  foreign key (supersedes_context_pack_id, project_id) references app.context_packs(id, project_id);
alter table app.context_pack_sources
  add constraint context_pack_sources_pack_project_fkey
  foreign key (context_pack_id, project_id) references app.context_packs(id, project_id);
alter table app.context_pack_sources
  add constraint context_pack_sources_entry_project_fkey
  foreign key (knowledge_entry_id, project_id) references app.knowledge_entries(id, project_id);
alter table app.context_pack_sources
  add constraint context_pack_sources_version_project_fkey
  foreign key (knowledge_entry_version_id, project_id)
  references app.knowledge_entry_versions(id, project_id);
alter table app.context_pack_selection_items
  add constraint context_pack_selection_items_pack_project_fkey
  foreign key (context_pack_id, project_id) references app.context_packs(id, project_id);
alter table app.context_pack_selection_items
  add constraint context_pack_selection_items_version_project_fkey
  foreign key (knowledge_entry_version_id, project_id)
  references app.knowledge_entry_versions(id, project_id);
alter table app.context_pack_selection_items
  add constraint context_pack_selection_items_observation_project_fkey
  foreign key (external_reference_observation_id, project_id)
  references app.external_reference_observations(id, project_id);
alter table app.sessions
  add constraint sessions_context_node_project_fkey
  foreign key (context_node_id, project_id) references app.context_nodes(id, project_id);
alter table app.sessions
  add constraint sessions_context_pack_project_fkey
  foreign key (context_pack_id, project_id) references app.context_packs(id, project_id);
alter table app.model_runs
  add constraint model_runs_session_project_fkey
  foreign key (session_id, project_id) references app.sessions(id, project_id);
alter table app.model_runs
  add constraint model_runs_context_pack_project_fkey
  foreign key (context_pack_id, project_id) references app.context_packs(id, project_id);
alter table app.messages
  add constraint messages_session_project_fkey
  foreign key (session_id, project_id) references app.sessions(id, project_id);
alter table app.mutation_proposals
  add constraint mutation_proposals_session_project_fkey
  foreign key (session_id, project_id) references app.sessions(id, project_id);
alter table app.mutation_proposals
  add constraint mutation_proposals_message_project_fkey
  foreign key (source_message_id, project_id) references app.messages(id, project_id);
alter table app.gates
  add constraint gates_context_node_project_fkey
  foreign key (context_node_id, project_id) references app.context_nodes(id, project_id);
alter table app.handoffs
  add constraint handoffs_source_session_project_fkey
  foreign key (source_session_id, project_id) references app.sessions(id, project_id);
alter table app.handoffs
  add constraint handoffs_target_session_project_fkey
  foreign key (target_session_id, project_id) references app.sessions(id, project_id);
alter table app.handoffs
  add constraint handoffs_context_pack_project_fkey
  foreign key (context_pack_id, project_id) references app.context_packs(id, project_id);
alter table app.deliverables
  add constraint deliverables_context_node_project_fkey
  foreign key (context_node_id, project_id) references app.context_nodes(id, project_id);
alter table app.deliverables
  add constraint deliverables_source_session_project_fkey
  foreign key (source_session_id, project_id) references app.sessions(id, project_id);
alter table app.deliverables
  add constraint deliverables_context_pack_project_fkey
  foreign key (source_context_pack_id, project_id) references app.context_packs(id, project_id);
alter table app.deliverables
  add constraint deliverables_supersedes_project_fkey
  foreign key (supersedes_deliverable_id, project_id) references app.deliverables(id, project_id);
alter table app.deliverable_sections
  add constraint deliverable_sections_deliverable_project_fkey
  foreign key (deliverable_id, project_id) references app.deliverables(id, project_id);
alter table app.deliverable_sources
  add constraint deliverable_sources_deliverable_project_fkey
  foreign key (deliverable_id, project_id) references app.deliverables(id, project_id);
alter table app.deliverable_sources
  add constraint deliverable_sources_version_project_fkey
  foreign key (knowledge_entry_version_id, project_id)
  references app.knowledge_entry_versions(id, project_id);
alter table app.deliverable_sources
  add constraint deliverable_sources_pack_project_fkey
  foreign key (context_pack_id, project_id) references app.context_packs(id, project_id);
alter table app.deliverable_sources
  add constraint deliverable_sources_observation_project_fkey
  foreign key (external_reference_observation_id, project_id)
  references app.external_reference_observations(id, project_id);
alter table app.deliverable_sources
  add constraint deliverable_sources_model_run_project_fkey
  foreign key (model_run_id, project_id) references app.model_runs(id, project_id);
alter table app.tasks
  add constraint tasks_context_pack_project_fkey
  foreign key (context_pack_id, project_id) references app.context_packs(id, project_id);
alter table app.executions
  add constraint executions_task_project_fkey
  foreign key (task_id, project_id) references app.tasks(id, project_id);
alter table app.execution_events
  add constraint execution_events_execution_project_fkey
  foreign key (execution_id, project_id) references app.executions(id, project_id);
alter table app.external_references
  add constraint external_references_connection_workspace_fkey
  foreign key (tool_connection_id, workspace_id)
  references app.tool_connections(id, workspace_id);
alter table app.external_reference_observations
  add constraint external_reference_observations_reference_project_fkey
  foreign key (external_reference_id, project_id)
  references app.external_references(id, project_id);
alter table app.artifacts
  add constraint artifacts_execution_project_fkey
  foreign key (execution_id, project_id) references app.executions(id, project_id);
alter table app.artifacts
  add constraint artifacts_deliverable_project_fkey
  foreign key (deliverable_id, project_id) references app.deliverables(id, project_id);
alter table app.artifacts
  add constraint artifacts_external_reference_project_fkey
  foreign key (external_reference_id, project_id)
  references app.external_references(id, project_id);
alter table app.evidences
  add constraint evidences_requirement_project_fkey
  foreign key (requirement_entry_id, project_id) references app.knowledge_entries(id, project_id);
alter table app.evidences
  add constraint evidences_requirement_version_project_fkey
  foreign key (requirement_version_id, project_id)
  references app.knowledge_entry_versions(id, project_id);
alter table app.evidences
  add constraint evidences_deliverable_project_fkey
  foreign key (deliverable_id, project_id) references app.deliverables(id, project_id);
alter table app.evidences
  add constraint evidences_section_project_fkey
  foreign key (deliverable_section_id, project_id)
  references app.deliverable_sections(id, project_id);
alter table app.evidences
  add constraint evidences_artifact_project_fkey
  foreign key (artifact_id, project_id) references app.artifacts(id, project_id);
alter table app.evidences
  add constraint evidences_external_reference_project_fkey
  foreign key (external_reference_id, project_id)
  references app.external_references(id, project_id);
alter table app.requirement_coverage
  add constraint requirement_coverage_requirement_project_fkey
  foreign key (requirement_entry_id, project_id) references app.knowledge_entries(id, project_id);
alter table app.requirement_coverage
  add constraint requirement_coverage_version_project_fkey
  foreign key (requirement_version_id, project_id)
  references app.knowledge_entry_versions(id, project_id);
alter table app.requirement_coverage
  add constraint requirement_coverage_deliverable_project_fkey
  foreign key (deliverable_id, project_id) references app.deliverables(id, project_id);
alter table app.requirement_coverage
  add constraint requirement_coverage_section_project_fkey
  foreign key (deliverable_section_id, project_id)
  references app.deliverable_sections(id, project_id);
alter table app.requirement_coverage
  add constraint requirement_coverage_evidence_project_fkey
  foreign key (evidence_id, project_id) references app.evidences(id, project_id);
alter table app.steward_assessment_sources
  add constraint steward_assessment_sources_assessment_project_fkey
  foreign key (assessment_id, project_id) references app.steward_assessments(id, project_id);
alter table app.steward_assessment_sources
  add constraint steward_assessment_sources_version_project_fkey
  foreign key (knowledge_entry_version_id, project_id)
  references app.knowledge_entry_versions(id, project_id);
alter table app.insights
  add constraint insights_assessment_project_fkey
  foreign key (steward_assessment_id, project_id)
  references app.steward_assessments(id, project_id);
alter table app.insight_sources
  add constraint insight_sources_insight_project_fkey
  foreign key (insight_id, project_id) references app.insights(id, project_id);
alter table app.insight_sources
  add constraint insight_sources_version_project_fkey
  foreign key (knowledge_entry_version_id, project_id)
  references app.knowledge_entry_versions(id, project_id);
alter table app.insight_resolutions
  add constraint insight_resolutions_insight_project_fkey
  foreign key (insight_id, project_id) references app.insights(id, project_id);
alter table app.insight_resolutions
  add constraint insight_resolutions_version_project_fkey
  foreign key (knowledge_entry_version_id, project_id)
  references app.knowledge_entry_versions(id, project_id);

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

create trigger workspace_members_set_updated_at
before update on app.workspace_members
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

create trigger model_runs_set_updated_at
before update on app.model_runs
for each row execute function app.set_updated_at();

create trigger idempotency_records_set_updated_at
before update on app.idempotency_records
for each row execute function app.set_updated_at();

create trigger deliverables_set_updated_at
before update on app.deliverables
for each row execute function app.set_updated_at();

create trigger tasks_set_updated_at
before update on app.tasks
for each row execute function app.set_updated_at();

create trigger tool_connections_set_updated_at
before update on app.tool_connections
for each row execute function app.set_updated_at();

create trigger external_references_set_updated_at
before update on app.external_references
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

create or replace function app.prevent_append_only_mutation()
returns trigger
language plpgsql
set search_path = ''
as $$
begin
  raise exception '% rows are append-only', tg_table_name;
end;
$$;

revoke execute on function app.prevent_append_only_mutation()
  from public, anon, authenticated;

create trigger external_reference_observations_prevent_update
before update or delete on app.external_reference_observations
for each row execute function app.prevent_append_only_mutation();

create trigger steward_assessments_prevent_update
before update or delete on app.steward_assessments
for each row execute function app.prevent_append_only_mutation();

create trigger steward_assessment_sources_prevent_update
before update or delete on app.steward_assessment_sources
for each row execute function app.prevent_append_only_mutation();

create trigger insight_resolutions_prevent_update
before update or delete on app.insight_resolutions
for each row execute function app.prevent_append_only_mutation();

create trigger knowledge_entry_versions_prevent_update
before update or delete on app.knowledge_entry_versions
for each row execute function app.prevent_append_only_mutation();

create trigger context_pack_sources_prevent_update
before update or delete on app.context_pack_sources
for each row execute function app.prevent_append_only_mutation();

create trigger context_pack_selection_items_prevent_update
before update or delete on app.context_pack_selection_items
for each row execute function app.prevent_append_only_mutation();

create trigger messages_prevent_update
before update or delete on app.messages
for each row execute function app.prevent_append_only_mutation();

create trigger deliverable_sections_prevent_update
before update or delete on app.deliverable_sections
for each row execute function app.prevent_append_only_mutation();

create trigger deliverable_sources_prevent_update
before update or delete on app.deliverable_sources
for each row execute function app.prevent_append_only_mutation();

create trigger execution_events_prevent_update
before update or delete on app.execution_events
for each row execute function app.prevent_append_only_mutation();

create trigger artifacts_prevent_update
before update or delete on app.artifacts
for each row execute function app.prevent_append_only_mutation();

create trigger insight_sources_prevent_update
before update or delete on app.insight_sources
for each row execute function app.prevent_append_only_mutation();

create or replace function app.enforce_context_pack_immutability()
returns trigger
language plpgsql
set search_path = ''
as $$
begin
  if row(
    new.workspace_id,
    new.project_id,
    new.source_node_id,
    new.target_node_id,
    new.target_agent_profile_id,
    new.task_kind,
    new.objective,
    new.content,
    new.version,
    new.source_graph_version,
    new.compiler_version,
    new.selection_mode,
    new.content_hash,
    new.token_budget,
    new.estimated_tokens,
    new.supersedes_context_pack_id,
    new.compiled_at
  ) is distinct from row(
    old.workspace_id,
    old.project_id,
    old.source_node_id,
    old.target_node_id,
    old.target_agent_profile_id,
    old.task_kind,
    old.objective,
    old.content,
    old.version,
    old.source_graph_version,
    old.compiler_version,
    old.selection_mode,
    old.content_hash,
    old.token_budget,
    old.estimated_tokens,
    old.supersedes_context_pack_id,
    old.compiled_at
  ) then
    raise exception 'context pack payloads are immutable';
  end if;

  if not (
    new.status = old.status
    or (old.status = 'current' and new.status in ('stale', 'superseded'))
    or (old.status = 'stale' and new.status = 'superseded')
  ) then
    raise exception 'invalid context pack status transition: % -> %', old.status, new.status;
  end if;

  if old.status <> 'current'
    and row(new.stale_reason, new.invalidated_at)
      is distinct from row(old.stale_reason, old.invalidated_at)
  then
    raise exception 'context pack invalidation metadata is immutable';
  end if;

  return new;
end;
$$;

revoke execute on function app.enforce_context_pack_immutability()
  from public, anon, authenticated;

create trigger context_packs_enforce_immutability
before update on app.context_packs
for each row execute function app.enforce_context_pack_immutability();

create trigger context_packs_prevent_delete
before delete on app.context_packs
for each row execute function app.prevent_append_only_mutation();

create or replace function app.current_actor_id()
returns uuid
language sql
stable
security invoker
set search_path = ''
as $$
  select nullif(current_setting('app.current_actor_id', true), '')::uuid;
$$;

create or replace function app.current_workspace_id()
returns bigint
language sql
stable
security invoker
set search_path = ''
as $$
  select nullif(current_setting('app.current_workspace_id', true), '')::bigint;
$$;

create or replace function app.current_workspace_role()
returns text
language sql
stable
security invoker
set search_path = ''
as $$
  select nullif(current_setting('app.current_workspace_role', true), '');
$$;

create or replace function app.has_workspace_role(
  requested_workspace_id bigint,
  allowed_roles text[]
)
returns boolean
language sql
stable
-- Intentional narrow SECURITY DEFINER lookup: app is a private schema, the
-- caller identity comes only from request-scoped GUCs, and the function
-- returns a boolean without exposing membership rows.
security definer
set search_path = ''
set row_security = off
as $$
  select
    requested_workspace_id = app.current_workspace_id()
    and exists (
      select 1
      from app.workspace_members member
      where member.workspace_id = requested_workspace_id
        and member.actor_id = app.current_actor_id()
        and member.invitation_status = 'accepted'
        and member.role = app.current_workspace_role()
        and member.role = any(allowed_roles)
    );
$$;

create or replace function app.authorize_workspace_member(
  requested_workspace_public_id uuid,
  requested_actor_id uuid
)
returns table(workspace_id bigint, role text)
language sql
stable
security definer
set search_path = ''
set row_security = off
as $$
  select member.workspace_id, member.role
  from app.workspace_members member
  join app.workspaces workspace on workspace.id = member.workspace_id
  where workspace.public_id = requested_workspace_public_id
    and member.actor_id = requested_actor_id
    and member.invitation_status = 'accepted';
$$;

revoke all on function app.authorize_workspace_member(uuid, uuid) from public;

create or replace function app.list_actor_workspaces()
returns table(public_id uuid, name text, role text)
language sql
stable
-- Intentional narrow SECURITY DEFINER bootstrap lookup. The actor identity is
-- injected by the authenticated server in a transaction-local GUC. The
-- function exposes only accepted memberships and lives in the private app
-- schema; EXECUTE remains revoked from PUBLIC.
security definer
set search_path = ''
set row_security = off
as $$
  select workspace.public_id, workspace.name, member.role
  from app.workspace_members member
  join app.workspaces workspace on workspace.id = member.workspace_id
  where member.actor_id = app.current_actor_id()
    and member.invitation_status = 'accepted'
  order by workspace.name, workspace.id;
$$;

revoke all on function app.list_actor_workspaces() from public;

create or replace function app.list_due_steward_workspaces(
  requested_limit integer default 16
)
returns table(
  workspace_id bigint,
  workspace_public_id uuid,
  actor_id uuid,
  workspace_role text
)
language sql
-- Intentional narrow SECURITY DEFINER recovery bootstrap. The function is in
-- the private app schema, hardcodes the Steward event allowlist, returns at
-- most 128 scopes, and selects only an accepted owner/editor. It never claims
-- or reads event payloads; all processing still runs as that member under the
-- runtime role, request GUCs and forced RLS.
security definer
set search_path = ''
set row_security = off
rows 128
as $$
  with scan_bounds as (
    select
      least(greatest(coalesce(requested_limit, 0), 0), 128) as scan_limit,
      clock_timestamp() as observed_at
  ),
  due_workspaces as (
    select
      event.workspace_id,
      min(
        case
          when event.status = 'pending' then event.available_at
          else event.locked_until
        end
      ) as due_at
    from app.domain_events event
    cross join scan_bounds bounds
    where event.event_type in ('knowledge.committed', 'knowledge.revised')
      and (
        (event.status = 'pending' and event.available_at <= bounds.observed_at)
        or (
          event.status = 'processing'
          and event.locked_until <= bounds.observed_at
        )
      )
      and exists (
        select 1
        from app.workspace_members eligible_member
        where eligible_member.workspace_id = event.workspace_id
          and eligible_member.invitation_status = 'accepted'
          and eligible_member.role in ('owner', 'editor')
      )
    group by event.workspace_id
    order by due_at, event.workspace_id
    limit (select scan_limit from scan_bounds)
  )
  select
    due.workspace_id,
    workspace.public_id,
    worker.actor_id,
    worker.role
  from due_workspaces due
  join app.workspaces workspace on workspace.id = due.workspace_id
  cross join lateral (
    select member.actor_id, member.role
    from app.workspace_members member
    where member.workspace_id = due.workspace_id
      and member.invitation_status = 'accepted'
      and member.role in ('owner', 'editor')
    order by
      case member.role when 'owner' then 0 else 1 end,
      member.accepted_at,
      member.id
    limit 1
  ) worker
  order by due.due_at, due.workspace_id;
$$;

revoke all on function app.list_due_steward_workspaces(integer)
  from public, anon, authenticated, service_role, ai_center_runtime;

do $$
declare
  relation_name text;
begin
  foreach relation_name in array array[
    'workspaces', 'workspace_members', 'project_templates', 'agent_profiles',
    'deliverable_contracts', 'projects', 'context_nodes', 'knowledge_entries',
    'knowledge_entry_versions', 'edges', 'context_packs', 'context_pack_sources',
    'context_pack_selection_items', 'sessions', 'model_runs', 'messages',
    'idempotency_records', 'mutation_proposals', 'gates', 'handoffs',
    'deliverables', 'deliverable_sections', 'deliverable_sources', 'tasks',
    'executions', 'execution_events', 'tool_connections', 'external_references',
    'external_reference_observations', 'artifacts', 'evidences',
    'requirement_coverage', 'steward_assessments', 'steward_assessment_sources',
    'insights', 'insight_sources', 'insight_resolutions', 'domain_events',
    'audit_events'
  ]
  loop
    execute format('alter table app.%I enable row level security', relation_name);
    execute format('alter table app.%I force row level security', relation_name);
  end loop;
end;
$$;

create policy workspaces_member_select
on app.workspaces for select
using (
  id = app.current_workspace_id()
  and (select app.has_workspace_role(
    id,
    array['owner', 'editor', 'viewer']::text[]
  ))
);

create policy workspaces_owner_update
on app.workspaces for update
using (
  id = app.current_workspace_id()
  and (select app.has_workspace_role(id, array['owner']::text[]))
)
with check (
  id = app.current_workspace_id()
  and (select app.has_workspace_role(id, array['owner']::text[]))
);

create policy workspace_members_scoped_select
on app.workspace_members for select
using (
  workspace_id = app.current_workspace_id()
  and (select app.has_workspace_role(
    workspace_id,
    array['owner', 'editor', 'viewer']::text[]
  ))
);

create policy workspace_members_owner_insert
on app.workspace_members for insert
with check (
  workspace_id = app.current_workspace_id()
  and (select app.has_workspace_role(workspace_id, array['owner']::text[]))
);

create policy workspace_members_owner_update
on app.workspace_members for update
using (
  workspace_id = app.current_workspace_id()
  and (select app.has_workspace_role(workspace_id, array['owner']::text[]))
)
with check (
  workspace_id = app.current_workspace_id()
  and (select app.has_workspace_role(workspace_id, array['owner']::text[]))
);

do $$
declare
  relation_name text;
begin
  foreach relation_name in array array[
    'project_templates', 'agent_profiles', 'deliverable_contracts'
  ]
  loop
    execute format(
      'create policy system_catalog_member_select on app.%I for select using ((select app.has_workspace_role(app.current_workspace_id(), array[''owner'', ''editor'', ''viewer'']::text[])))',
      relation_name
    );
  end loop;
end;
$$;

do $$
declare
  relation_name text;
begin
  foreach relation_name in array array[
    'knowledge_entry_versions', 'context_pack_sources',
    'context_pack_selection_items', 'messages', 'deliverable_sections',
    'deliverable_sources', 'execution_events', 'external_reference_observations',
    'artifacts', 'steward_assessments', 'steward_assessment_sources',
    'insight_sources', 'insight_resolutions', 'audit_events'
  ]
  loop
    execute format(
      'create policy tenant_member_select on app.%I for select using (workspace_id = app.current_workspace_id() and (select app.has_workspace_role(workspace_id, array[''owner'', ''editor'', ''viewer'']::text[])))',
      relation_name
    );
    execute format(
      'create policy tenant_editor_insert on app.%I for insert with check (workspace_id = app.current_workspace_id() and (select app.has_workspace_role(workspace_id, array[''owner'', ''editor'']::text[])))',
      relation_name
    );
  end loop;
end;
$$;

do $$
declare
  relation_name text;
begin
  foreach relation_name in array array[
    'projects', 'context_nodes', 'knowledge_entries', 'edges', 'context_packs',
    'sessions', 'model_runs', 'idempotency_records', 'mutation_proposals',
    'gates', 'handoffs', 'deliverables', 'tasks', 'executions',
    'tool_connections', 'external_references', 'evidences',
    'requirement_coverage', 'insights', 'domain_events'
  ]
  loop
    execute format(
      'create policy tenant_member_select on app.%I for select using (workspace_id = app.current_workspace_id() and (select app.has_workspace_role(workspace_id, array[''owner'', ''editor'', ''viewer'']::text[])))',
      relation_name
    );
    execute format(
      'create policy tenant_editor_insert on app.%I for insert with check (workspace_id = app.current_workspace_id() and (select app.has_workspace_role(workspace_id, array[''owner'', ''editor'']::text[])))',
      relation_name
    );
    execute format(
      'create policy tenant_editor_update on app.%I for update using (workspace_id = app.current_workspace_id() and (select app.has_workspace_role(workspace_id, array[''owner'', ''editor'']::text[]))) with check (workspace_id = app.current_workspace_id() and (select app.has_workspace_role(workspace_id, array[''owner'', ''editor'']::text[])))',
      relation_name
    );
  end loop;
end;
$$;
