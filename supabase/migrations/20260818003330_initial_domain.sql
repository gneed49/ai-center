-- Migration unit 1: schema_changes
-- Transaction mode: transactional
-- Boundary reason: default

SET check_function_bodies = false;

CREATE SCHEMA app AUTHORIZATION postgres;

CREATE FUNCTION app.prevent_audit_mutation()
  RETURNS TRIGGER
  LANGUAGE plpgsql
  SET search_path TO ''
  AS $function$
begin
  raise exception 'audit_events are immutable';
end;
$function$;

REVOKE ALL ON FUNCTION app.prevent_audit_mutation() FROM PUBLIC;

CREATE FUNCTION app.set_updated_at()
  RETURNS TRIGGER
  LANGUAGE plpgsql
  SET search_path TO ''
  AS $function$
begin
  new.updated_at = now();
  return new;
end;
$function$;

REVOKE ALL ON FUNCTION app.set_updated_at() FROM PUBLIC;

CREATE TABLE app.agent_profiles (
  id               bigint                   GENERATED ALWAYS AS IDENTITY NOT NULL,
  public_id        uuid                     DEFAULT gen_random_uuid() NOT NULL,
  template_id      bigint                   NOT NULL,
  profile_key      text                     NOT NULL,
  name             text                     NOT NULL,
  scope_kind       text                     NOT NULL,
  instructions     text                     NOT NULL,
  retrieval_policy jsonb                    DEFAULT '{}'::jsonb NOT NULL,
  output_schema    jsonb                    DEFAULT '{}'::jsonb NOT NULL,
  tools            jsonb                    DEFAULT '[]'::jsonb NOT NULL,
  created_at       timestamp with time zone DEFAULT now() NOT NULL
);

ALTER TABLE app.agent_profiles
  ENABLE ROW LEVEL SECURITY;

ALTER TABLE app.agent_profiles
  ADD CONSTRAINT agent_profiles_instructions_not_blank CHECK (btrim(instructions) <> ''::text);

ALTER TABLE app.agent_profiles
  ADD CONSTRAINT agent_profiles_pkey PRIMARY KEY (id);

ALTER TABLE app.agent_profiles
  ADD CONSTRAINT agent_profiles_public_id_key UNIQUE (public_id);

ALTER TABLE app.agent_profiles
  ADD CONSTRAINT agent_profiles_scope_kind_valid CHECK (scope_kind = ANY (ARRAY['product'::text, 'tech'::text, 'steward'::text]));

ALTER TABLE app.agent_profiles
  ADD CONSTRAINT agent_profiles_template_key_unique UNIQUE (template_id, profile_key);

CREATE INDEX agent_profiles_template_id_idx ON app.agent_profiles (template_id);

CREATE TABLE app.artifacts (
  id             bigint                   GENERATED ALWAYS AS IDENTITY NOT NULL,
  public_id      uuid                     DEFAULT gen_random_uuid() NOT NULL,
  workspace_id   bigint                   NOT NULL,
  project_id     bigint                   NOT NULL,
  execution_id   bigint,
  deliverable_id bigint,
  artifact_type  text                     NOT NULL,
  title          text                     NOT NULL,
  reference      text                     NOT NULL,
  metadata       jsonb                    DEFAULT '{}'::jsonb NOT NULL,
  created_at     timestamp with time zone DEFAULT now() NOT NULL
);

ALTER TABLE app.artifacts
  ENABLE ROW LEVEL SECURITY;

ALTER TABLE app.artifacts
  ADD CONSTRAINT artifacts_pkey PRIMARY KEY (id);

ALTER TABLE app.artifacts
  ADD CONSTRAINT artifacts_public_id_key UNIQUE (public_id);

CREATE INDEX artifacts_execution_id_idx ON app.artifacts (execution_id);

CREATE INDEX artifacts_workspace_id_idx ON app.artifacts (workspace_id);

CREATE INDEX artifacts_deliverable_id_idx ON app.artifacts (deliverable_id);

CREATE INDEX artifacts_project_created_idx ON app.artifacts (project_id, created_at DESC);

CREATE TABLE app.audit_events (
  id               bigint                   GENERATED ALWAYS AS IDENTITY NOT NULL,
  public_id        uuid                     DEFAULT gen_random_uuid() NOT NULL,
  workspace_id     bigint                   NOT NULL,
  project_id       bigint,
  actor_id         uuid                     NOT NULL,
  action           text                     NOT NULL,
  object_kind      text                     NOT NULL,
  object_public_id uuid                     NOT NULL,
  before_state     jsonb,
  after_state      jsonb,
  occurred_at      timestamp with time zone DEFAULT now() NOT NULL
);

ALTER TABLE app.audit_events
  ENABLE ROW LEVEL SECURITY;

ALTER TABLE app.audit_events
  ADD CONSTRAINT audit_events_pkey PRIMARY KEY (id);

ALTER TABLE app.audit_events
  ADD CONSTRAINT audit_events_public_id_key UNIQUE (public_id);

CREATE INDEX audit_events_workspace_occurred_idx ON app.audit_events (workspace_id, occurred_at DESC, id DESC);

CREATE INDEX audit_events_project_occurred_idx ON app.audit_events (project_id, occurred_at DESC, id DESC);

CREATE TRIGGER audit_events_prevent_update
  BEFORE DELETE OR UPDATE ON app.audit_events
  FOR EACH ROW
  EXECUTE FUNCTION app.prevent_audit_mutation();

CREATE TABLE app.context_nodes (
  id                          bigint                   GENERATED ALWAYS AS IDENTITY NOT NULL,
  public_id                   uuid                     DEFAULT gen_random_uuid() NOT NULL,
  workspace_id                bigint                   NOT NULL,
  project_id                  bigint                   NOT NULL,
  parent_id                   bigint,
  agent_profile_id            bigint                   NOT NULL,
  node_key                    text                     NOT NULL,
  title                       text                     NOT NULL,
  description                 text                     DEFAULT ''::text NOT NULL,
  summary                     text                     DEFAULT ''::text NOT NULL,
  context_rules               jsonb                    DEFAULT '{}'::jsonb NOT NULL,
  preferred_entry_types       text[]                   DEFAULT '{}'::text[] NOT NULL,
  preferred_deliverable_types text[]                   DEFAULT '{}'::text[] NOT NULL,
  created_at                  timestamp with time zone DEFAULT now() NOT NULL,
  updated_at                  timestamp with time zone DEFAULT now() NOT NULL
);

ALTER TABLE app.context_nodes
  ENABLE ROW LEVEL SECURITY;

ALTER TABLE app.context_nodes
  ADD CONSTRAINT context_nodes_agent_profile_id_fkey FOREIGN KEY (agent_profile_id) REFERENCES app.agent_profiles(id);

ALTER TABLE app.context_nodes
  ADD CONSTRAINT context_nodes_key_not_blank CHECK (btrim(node_key) <> ''::text);

ALTER TABLE app.context_nodes
  ADD CONSTRAINT context_nodes_pkey PRIMARY KEY (id);

ALTER TABLE app.context_nodes
  ADD CONSTRAINT context_nodes_parent_id_fkey FOREIGN KEY (parent_id) REFERENCES app.context_nodes(id) ON DELETE CASCADE;

ALTER TABLE app.context_nodes
  ADD CONSTRAINT context_nodes_project_key_unique UNIQUE (project_id, node_key);

ALTER TABLE app.context_nodes
  ADD CONSTRAINT context_nodes_public_id_key UNIQUE (public_id);

CREATE INDEX context_nodes_agent_profile_id_idx ON app.context_nodes (agent_profile_id);

CREATE INDEX context_nodes_parent_id_idx ON app.context_nodes (parent_id);

CREATE INDEX context_nodes_project_id_idx ON app.context_nodes (project_id);

CREATE INDEX context_nodes_workspace_id_idx ON app.context_nodes (workspace_id);

CREATE TRIGGER context_nodes_set_updated_at
  BEFORE UPDATE ON app.context_nodes
  FOR EACH ROW
  EXECUTE FUNCTION app.set_updated_at();

CREATE TABLE app.context_pack_sources (
  id                         bigint GENERATED ALWAYS AS IDENTITY NOT NULL,
  context_pack_id            bigint NOT NULL,
  knowledge_entry_id         bigint NOT NULL,
  knowledge_entry_version_id bigint NOT NULL,
  source_role                text   NOT NULL,
  included_reason            text   NOT NULL
);

ALTER TABLE app.context_pack_sources
  ENABLE ROW LEVEL SECURITY;

ALTER TABLE app.context_pack_sources
  ADD CONSTRAINT context_pack_sources_pack_version_unique UNIQUE (context_pack_id, knowledge_entry_version_id);

ALTER TABLE app.context_pack_sources
  ADD CONSTRAINT context_pack_sources_pkey PRIMARY KEY (id);

CREATE INDEX context_pack_sources_version_id_idx ON app.context_pack_sources (knowledge_entry_version_id);

CREATE INDEX context_pack_sources_pack_id_idx ON app.context_pack_sources (context_pack_id);

CREATE INDEX context_pack_sources_entry_id_idx ON app.context_pack_sources (knowledge_entry_id);

CREATE TABLE app.context_packs (
  id                      bigint                   GENERATED ALWAYS AS IDENTITY NOT NULL,
  public_id               uuid                     DEFAULT gen_random_uuid() NOT NULL,
  workspace_id            bigint                   NOT NULL,
  project_id              bigint                   NOT NULL,
  source_node_id          bigint                   NOT NULL,
  target_node_id          bigint                   NOT NULL,
  target_agent_profile_id bigint                   NOT NULL,
  task_kind               text                     NOT NULL,
  objective               text                     NOT NULL,
  content                 jsonb                    NOT NULL,
  version                 integer                  DEFAULT 1 NOT NULL,
  status                  text                     DEFAULT 'current'::text NOT NULL,
  compiled_at             timestamp with time zone DEFAULT now() NOT NULL,
  invalidated_at          timestamp with time zone
);

ALTER TABLE app.context_packs
  ENABLE ROW LEVEL SECURITY;

ALTER TABLE app.context_packs
  ADD CONSTRAINT context_packs_objective_not_blank CHECK (btrim(objective) <> ''::text);

ALTER TABLE app.context_packs
  ADD CONSTRAINT context_packs_pkey PRIMARY KEY (id);

ALTER TABLE app.context_pack_sources
  ADD CONSTRAINT context_pack_sources_context_pack_id_fkey FOREIGN KEY (context_pack_id) REFERENCES app.context_packs(id) ON DELETE CASCADE;

ALTER TABLE app.context_packs
  ADD CONSTRAINT context_packs_public_id_key UNIQUE (public_id);

ALTER TABLE app.context_packs
  ADD CONSTRAINT context_packs_source_node_id_fkey FOREIGN KEY (source_node_id) REFERENCES app.context_nodes(id);

ALTER TABLE app.context_packs
  ADD CONSTRAINT context_packs_status_valid CHECK (status = ANY (ARRAY['current'::text, 'stale'::text, 'superseded'::text]));

ALTER TABLE app.context_packs
  ADD CONSTRAINT context_packs_target_agent_profile_id_fkey FOREIGN KEY (target_agent_profile_id) REFERENCES app.agent_profiles(id);

ALTER TABLE app.context_packs
  ADD CONSTRAINT context_packs_target_node_id_fkey FOREIGN KEY (target_node_id) REFERENCES app.context_nodes(id);

ALTER TABLE app.context_packs
  ADD CONSTRAINT context_packs_version_positive CHECK (version > 0);

CREATE INDEX context_packs_project_status_compiled_idx ON app.context_packs (project_id, status, compiled_at DESC);

CREATE INDEX context_packs_workspace_id_idx ON app.context_packs (workspace_id);

CREATE INDEX context_packs_source_node_id_idx ON app.context_packs (source_node_id);

CREATE INDEX context_packs_target_node_id_idx ON app.context_packs (target_node_id);

CREATE INDEX context_packs_target_agent_profile_id_idx ON app.context_packs (target_agent_profile_id);

CREATE TABLE app.deliverable_contracts (
  id                        bigint                   GENERATED ALWAYS AS IDENTITY NOT NULL,
  public_id                 uuid                     DEFAULT gen_random_uuid() NOT NULL,
  template_id               bigint                   NOT NULL,
  contract_key              text                     NOT NULL,
  name                      text                     NOT NULL,
  required_sections         text[]                   DEFAULT '{}'::text[] NOT NULL,
  accepted_evidence_types   text[]                   DEFAULT '{}'::text[] NOT NULL,
  completion_rules          jsonb                    DEFAULT '{}'::jsonb NOT NULL,
  human_validation_required boolean                  DEFAULT false NOT NULL,
  created_at                timestamp with time zone DEFAULT now() NOT NULL
);

ALTER TABLE app.deliverable_contracts
  ENABLE ROW LEVEL SECURITY;

ALTER TABLE app.deliverable_contracts
  ADD CONSTRAINT deliverable_contracts_pkey PRIMARY KEY (id);

ALTER TABLE app.deliverable_contracts
  ADD CONSTRAINT deliverable_contracts_public_id_key UNIQUE (public_id);

ALTER TABLE app.deliverable_contracts
  ADD CONSTRAINT deliverable_contracts_template_key_unique UNIQUE (template_id, contract_key);

CREATE INDEX deliverable_contracts_template_id_idx ON app.deliverable_contracts (template_id);

CREATE TABLE app.deliverable_sections (
  id             bigint  GENERATED ALWAYS AS IDENTITY NOT NULL,
  public_id      uuid    DEFAULT gen_random_uuid() NOT NULL,
  deliverable_id bigint  NOT NULL,
  section_key    text    NOT NULL,
  title          text    NOT NULL,
  body           text    NOT NULL,
  ordinal        integer NOT NULL,
  source_data    jsonb   DEFAULT '{}'::jsonb NOT NULL
);

ALTER TABLE app.deliverable_sections
  ENABLE ROW LEVEL SECURITY;

ALTER TABLE app.deliverable_sections
  ADD CONSTRAINT deliverable_sections_deliverable_key_unique UNIQUE (deliverable_id, section_key);

ALTER TABLE app.deliverable_sections
  ADD CONSTRAINT deliverable_sections_ordinal_nonnegative CHECK (ordinal >= 0);

ALTER TABLE app.deliverable_sections
  ADD CONSTRAINT deliverable_sections_pkey PRIMARY KEY (id);

ALTER TABLE app.deliverable_sections
  ADD CONSTRAINT deliverable_sections_public_id_key UNIQUE (public_id);

CREATE INDEX deliverable_sections_deliverable_ordinal_idx ON app.deliverable_sections (deliverable_id, ordinal);

CREATE TABLE app.deliverables (
  id                     bigint                   GENERATED ALWAYS AS IDENTITY NOT NULL,
  public_id              uuid                     DEFAULT gen_random_uuid() NOT NULL,
  workspace_id           bigint                   NOT NULL,
  project_id             bigint                   NOT NULL,
  context_node_id        bigint                   NOT NULL,
  contract_id            bigint                   NOT NULL,
  source_session_id      bigint,
  source_context_pack_id bigint,
  deliverable_type       text                     NOT NULL,
  title                  text                     NOT NULL,
  summary                text                     DEFAULT ''::text NOT NULL,
  content                jsonb                    NOT NULL,
  status                 text                     DEFAULT 'draft'::text NOT NULL,
  coverage_status        text                     DEFAULT 'missing'::text NOT NULL,
  version                integer                  DEFAULT 1 NOT NULL,
  committed_at           timestamp with time zone,
  stale_at               timestamp with time zone,
  created_at             timestamp with time zone DEFAULT now() NOT NULL,
  updated_at             timestamp with time zone DEFAULT now() NOT NULL
);

ALTER TABLE app.deliverables
  ENABLE ROW LEVEL SECURITY;

ALTER TABLE app.deliverables
  ADD CONSTRAINT deliverables_context_node_id_fkey FOREIGN KEY (context_node_id) REFERENCES app.context_nodes(id);

ALTER TABLE app.deliverables
  ADD CONSTRAINT deliverables_contract_id_fkey FOREIGN KEY (contract_id) REFERENCES app.deliverable_contracts(id);

ALTER TABLE app.deliverables
  ADD CONSTRAINT deliverables_coverage_status_valid CHECK (coverage_status = ANY (ARRAY['covered'::text, 'partial'::text, 'missing'::text]));

ALTER TABLE app.deliverables
  ADD CONSTRAINT deliverables_pkey PRIMARY KEY (id);

ALTER TABLE app.artifacts
  ADD CONSTRAINT artifacts_deliverable_id_fkey FOREIGN KEY (deliverable_id) REFERENCES app.deliverables(id);

ALTER TABLE app.deliverable_sections
  ADD CONSTRAINT deliverable_sections_deliverable_id_fkey FOREIGN KEY (deliverable_id) REFERENCES app.deliverables(id) ON DELETE CASCADE;

ALTER TABLE app.deliverables
  ADD CONSTRAINT deliverables_public_id_key UNIQUE (public_id);

ALTER TABLE app.deliverables
  ADD CONSTRAINT deliverables_source_context_pack_id_fkey FOREIGN KEY (source_context_pack_id) REFERENCES app.context_packs(id);

ALTER TABLE app.deliverables
  ADD CONSTRAINT deliverables_status_valid CHECK (status = ANY (ARRAY['draft'::text, 'committed'::text, 'stale'::text, 'superseded'::text]));

ALTER TABLE app.deliverables
  ADD CONSTRAINT deliverables_version_positive CHECK (version > 0);

CREATE INDEX deliverables_project_type_created_idx ON app.deliverables (project_id, deliverable_type, created_at DESC);

CREATE INDEX deliverables_workspace_id_idx ON app.deliverables (workspace_id);

CREATE INDEX deliverables_context_node_id_idx ON app.deliverables (context_node_id);

CREATE INDEX deliverables_contract_id_idx ON app.deliverables (contract_id);

CREATE INDEX deliverables_source_session_id_idx ON app.deliverables (source_session_id);

CREATE INDEX deliverables_source_context_pack_id_idx ON app.deliverables (source_context_pack_id);

CREATE TRIGGER deliverables_set_updated_at
  BEFORE UPDATE ON app.deliverables
  FOR EACH ROW
  EXECUTE FUNCTION app.set_updated_at();

CREATE TABLE app.domain_events (
  id                  bigint                   GENERATED ALWAYS AS IDENTITY NOT NULL,
  public_id           uuid                     DEFAULT gen_random_uuid() NOT NULL,
  workspace_id        bigint                   NOT NULL,
  project_id          bigint,
  event_type          text                     NOT NULL,
  aggregate_kind      text                     NOT NULL,
  aggregate_public_id uuid                     NOT NULL,
  payload             jsonb                    DEFAULT '{}'::jsonb NOT NULL,
  status              text                     DEFAULT 'pending'::text NOT NULL,
  occurred_at         timestamp with time zone DEFAULT now() NOT NULL,
  processed_at        timestamp with time zone
);

ALTER TABLE app.domain_events
  ENABLE ROW LEVEL SECURITY;

ALTER TABLE app.domain_events
  ADD CONSTRAINT domain_events_pkey PRIMARY KEY (id);

ALTER TABLE app.domain_events
  ADD CONSTRAINT domain_events_public_id_key UNIQUE (public_id);

ALTER TABLE app.domain_events
  ADD CONSTRAINT domain_events_status_valid CHECK (status = ANY (ARRAY['pending'::text, 'processed'::text, 'failed'::text]));

CREATE INDEX domain_events_workspace_id_idx ON app.domain_events (workspace_id);

CREATE INDEX domain_events_project_id_idx ON app.domain_events (project_id);

CREATE INDEX domain_events_status_occurred_idx ON app.domain_events (status, occurred_at, id);

CREATE TABLE app.edges (
  id                  bigint                   GENERATED ALWAYS AS IDENTITY NOT NULL,
  public_id           uuid                     DEFAULT gen_random_uuid() NOT NULL,
  workspace_id        bigint                   NOT NULL,
  project_id          bigint                   NOT NULL,
  source_kind         text                     NOT NULL,
  source_public_id    uuid                     NOT NULL,
  target_kind         text                     NOT NULL,
  target_public_id    uuid                     NOT NULL,
  edge_type           text                     NOT NULL,
  status              text                     DEFAULT 'confirmed'::text NOT NULL,
  provenance          jsonb                    DEFAULT '{}'::jsonb NOT NULL,
  created_by_actor_id uuid                     NOT NULL,
  created_at          timestamp with time zone DEFAULT now() NOT NULL
);

ALTER TABLE app.edges
  ENABLE ROW LEVEL SECURITY;

ALTER TABLE app.edges
  ADD CONSTRAINT edges_not_self CHECK (source_kind <> target_kind OR source_public_id <> target_public_id);

ALTER TABLE app.edges
  ADD CONSTRAINT edges_pkey PRIMARY KEY (id);

ALTER TABLE app.edges
  ADD CONSTRAINT edges_public_id_key UNIQUE (public_id);

ALTER TABLE app.edges
  ADD CONSTRAINT edges_relation_unique UNIQUE (project_id, source_public_id, target_public_id, edge_type);

ALTER TABLE app.edges
  ADD CONSTRAINT edges_status_valid CHECK (status = ANY (ARRAY['proposed'::text, 'confirmed'::text, 'rejected'::text]));

ALTER TABLE app.edges
  ADD CONSTRAINT edges_type_valid
    CHECK
    (edge_type = ANY (ARRAY['references'::text, 'depends_on'::text, 'informs'::text, 'supersedes'::text, 'contradicts'::text, 'derived_from'::text, 'satisfies'::text,
    'evidenced_by'::text, 'implemented_by'::text]));

CREATE INDEX edges_project_source_idx ON app.edges (project_id, source_public_id);

CREATE INDEX edges_project_target_idx ON app.edges (project_id, target_public_id);

CREATE INDEX edges_workspace_id_idx ON app.edges (workspace_id);

CREATE TABLE app.evidences (
  id                     bigint                   GENERATED ALWAYS AS IDENTITY NOT NULL,
  public_id              uuid                     DEFAULT gen_random_uuid() NOT NULL,
  workspace_id           bigint                   NOT NULL,
  project_id             bigint                   NOT NULL,
  requirement_entry_id   bigint                   NOT NULL,
  requirement_version_id bigint                   NOT NULL,
  deliverable_id         bigint,
  deliverable_section_id bigint,
  artifact_id            bigint,
  evidence_type          text                     NOT NULL,
  title                  text                     NOT NULL,
  description            text                     DEFAULT ''::text NOT NULL,
  source_reference       text                     NOT NULL,
  status                 text                     DEFAULT 'valid'::text NOT NULL,
  created_at             timestamp with time zone DEFAULT now() NOT NULL
);

ALTER TABLE app.evidences
  ENABLE ROW LEVEL SECURITY;

ALTER TABLE app.evidences
  ADD CONSTRAINT evidences_artifact_id_fkey FOREIGN KEY (artifact_id) REFERENCES app.artifacts(id) ON DELETE SET NULL;

ALTER TABLE app.evidences
  ADD CONSTRAINT evidences_deliverable_id_fkey FOREIGN KEY (deliverable_id) REFERENCES app.deliverables(id) ON DELETE CASCADE;

ALTER TABLE app.evidences
  ADD CONSTRAINT evidences_deliverable_section_id_fkey FOREIGN KEY (deliverable_section_id) REFERENCES app.deliverable_sections(id) ON DELETE CASCADE;

ALTER TABLE app.evidences
  ADD CONSTRAINT evidences_pkey PRIMARY KEY (id);

ALTER TABLE app.evidences
  ADD CONSTRAINT evidences_public_id_key UNIQUE (public_id);

ALTER TABLE app.evidences
  ADD CONSTRAINT evidences_status_valid CHECK (status = ANY (ARRAY['valid'::text, 'stale'::text, 'rejected'::text]));

CREATE INDEX evidences_workspace_id_idx ON app.evidences (workspace_id);

CREATE INDEX evidences_artifact_id_idx ON app.evidences (artifact_id);

CREATE INDEX evidences_deliverable_section_id_idx ON app.evidences (deliverable_section_id);

CREATE INDEX evidences_deliverable_id_idx ON app.evidences (deliverable_id);

CREATE INDEX evidences_requirement_version_id_idx ON app.evidences (requirement_version_id);

CREATE INDEX evidences_project_id_idx ON app.evidences (project_id);

CREATE INDEX evidences_requirement_status_idx ON app.evidences (requirement_entry_id, status);

CREATE TABLE app.execution_events (
  id              bigint                   GENERATED ALWAYS AS IDENTITY NOT NULL,
  public_id       uuid                     DEFAULT gen_random_uuid() NOT NULL,
  execution_id    bigint                   NOT NULL,
  event_type      text                     NOT NULL,
  sequence_number integer                  NOT NULL,
  payload         jsonb                    DEFAULT '{}'::jsonb NOT NULL,
  created_at      timestamp with time zone DEFAULT now() NOT NULL
);

ALTER TABLE app.execution_events
  ENABLE ROW LEVEL SECURITY;

ALTER TABLE app.execution_events
  ADD CONSTRAINT execution_events_execution_sequence_unique UNIQUE (execution_id, sequence_number);

ALTER TABLE app.execution_events
  ADD CONSTRAINT execution_events_pkey PRIMARY KEY (id);

ALTER TABLE app.execution_events
  ADD CONSTRAINT execution_events_public_id_key UNIQUE (public_id);

ALTER TABLE app.execution_events
  ADD CONSTRAINT execution_events_sequence_positive CHECK (sequence_number > 0);

CREATE INDEX execution_events_execution_sequence_idx ON app.execution_events (execution_id, sequence_number);

CREATE TABLE app.executions (
  id           bigint                   GENERATED ALWAYS AS IDENTITY NOT NULL,
  public_id    uuid                     DEFAULT gen_random_uuid() NOT NULL,
  workspace_id bigint                   NOT NULL,
  project_id   bigint                   NOT NULL,
  task_id      bigint                   NOT NULL,
  executor_key text                     NOT NULL,
  status       text                     DEFAULT 'queued'::text NOT NULL,
  result       jsonb,
  started_at   timestamp with time zone,
  completed_at timestamp with time zone,
  created_at   timestamp with time zone DEFAULT now() NOT NULL
);

ALTER TABLE app.executions
  ENABLE ROW LEVEL SECURITY;

ALTER TABLE app.executions
  ADD CONSTRAINT executions_pkey PRIMARY KEY (id);

ALTER TABLE app.artifacts
  ADD CONSTRAINT artifacts_execution_id_fkey FOREIGN KEY (execution_id) REFERENCES app.executions(id);

ALTER TABLE app.execution_events
  ADD CONSTRAINT execution_events_execution_id_fkey FOREIGN KEY (execution_id) REFERENCES app.executions(id) ON DELETE CASCADE;

ALTER TABLE app.executions
  ADD CONSTRAINT executions_public_id_key UNIQUE (public_id);

ALTER TABLE app.executions
  ADD CONSTRAINT executions_status_valid CHECK (status = ANY (ARRAY['queued'::text, 'running'::text, 'completed'::text, 'failed'::text, 'cancelled'::text]));

CREATE INDEX executions_workspace_id_idx ON app.executions (workspace_id);

CREATE INDEX executions_task_created_idx ON app.executions (task_id, created_at DESC);

CREATE INDEX executions_project_status_created_idx ON app.executions (project_id, status, created_at DESC);

CREATE TABLE app.gates (
  id              bigint                   GENERATED ALWAYS AS IDENTITY NOT NULL,
  public_id       uuid                     DEFAULT gen_random_uuid() NOT NULL,
  workspace_id    bigint                   NOT NULL,
  project_id      bigint                   NOT NULL,
  context_node_id bigint                   NOT NULL,
  gate_key        text                     NOT NULL,
  status          text                     DEFAULT 'pending'::text NOT NULL,
  evaluation      jsonb                    DEFAULT '{}'::jsonb NOT NULL,
  graph_version   bigint                   NOT NULL,
  evaluated_at    timestamp with time zone DEFAULT now() NOT NULL
);

ALTER TABLE app.gates
  ENABLE ROW LEVEL SECURITY;

ALTER TABLE app.gates
  ADD CONSTRAINT gates_context_node_id_fkey FOREIGN KEY (context_node_id) REFERENCES app.context_nodes(id);

ALTER TABLE app.gates
  ADD CONSTRAINT gates_pkey PRIMARY KEY (id);

ALTER TABLE app.gates
  ADD CONSTRAINT gates_project_key_graph_unique UNIQUE (project_id, gate_key, graph_version);

ALTER TABLE app.gates
  ADD CONSTRAINT gates_public_id_key UNIQUE (public_id);

ALTER TABLE app.gates
  ADD CONSTRAINT gates_status_valid CHECK (status = ANY (ARRAY['pending'::text, 'passed'::text, 'passed_with_warning'::text, 'blocked'::text]));

CREATE INDEX gates_context_node_id_idx ON app.gates (context_node_id);

CREATE INDEX gates_workspace_id_idx ON app.gates (workspace_id);

CREATE INDEX gates_project_evaluated_idx ON app.gates (project_id, evaluated_at DESC);

CREATE TABLE app.handoffs (
  id                    bigint                   GENERATED ALWAYS AS IDENTITY NOT NULL,
  public_id             uuid                     DEFAULT gen_random_uuid() NOT NULL,
  workspace_id          bigint                   NOT NULL,
  project_id            bigint                   NOT NULL,
  source_session_id     bigint                   NOT NULL,
  target_session_id     bigint,
  context_pack_id       bigint                   NOT NULL,
  status                text                     DEFAULT 'ready'::text NOT NULL,
  initiated_by_actor_id uuid                     NOT NULL,
  created_at            timestamp with time zone DEFAULT now() NOT NULL,
  completed_at          timestamp with time zone
);

ALTER TABLE app.handoffs
  ENABLE ROW LEVEL SECURITY;

ALTER TABLE app.handoffs
  ADD CONSTRAINT handoffs_context_pack_id_fkey FOREIGN KEY (context_pack_id) REFERENCES app.context_packs(id);

ALTER TABLE app.handoffs
  ADD CONSTRAINT handoffs_pkey PRIMARY KEY (id);

ALTER TABLE app.handoffs
  ADD CONSTRAINT handoffs_public_id_key UNIQUE (public_id);

ALTER TABLE app.handoffs
  ADD CONSTRAINT handoffs_status_valid CHECK (status = ANY (ARRAY['ready'::text, 'completed'::text, 'cancelled'::text]));

CREATE INDEX handoffs_source_session_id_idx ON app.handoffs (source_session_id);

CREATE INDEX handoffs_workspace_id_idx ON app.handoffs (workspace_id);

CREATE INDEX handoffs_project_created_idx ON app.handoffs (project_id, created_at DESC);

CREATE INDEX handoffs_context_pack_id_idx ON app.handoffs (context_pack_id);

CREATE INDEX handoffs_target_session_id_idx ON app.handoffs (target_session_id);

CREATE TABLE app.insight_sources (
  id                         bigint GENERATED ALWAYS AS IDENTITY NOT NULL,
  insight_id                 bigint NOT NULL,
  source_role                text   NOT NULL,
  object_kind                text   NOT NULL,
  object_public_id           uuid   NOT NULL,
  knowledge_entry_version_id bigint
);

ALTER TABLE app.insight_sources
  ENABLE ROW LEVEL SECURITY;

ALTER TABLE app.insight_sources
  ADD CONSTRAINT insight_sources_pkey PRIMARY KEY (id);

ALTER TABLE app.insight_sources
  ADD CONSTRAINT insight_sources_relation_unique UNIQUE (insight_id, source_role, object_public_id);

CREATE INDEX insight_sources_version_id_idx ON app.insight_sources (knowledge_entry_version_id);

CREATE INDEX insight_sources_insight_id_idx ON app.insight_sources (insight_id);

CREATE TABLE app.insights (
  id                       bigint                   GENERATED ALWAYS AS IDENTITY NOT NULL,
  public_id                uuid                     DEFAULT gen_random_uuid() NOT NULL,
  workspace_id             bigint                   NOT NULL,
  project_id               bigint                   NOT NULL,
  insight_type             text                     NOT NULL,
  status                   text                     DEFAULT 'candidate'::text NOT NULL,
  severity                 text                     NOT NULL,
  confidence               numeric(4,3)             NOT NULL,
  title                    text                     NOT NULL,
  explanation              text                     NOT NULL,
  resolution_justification text,
  resolved_by_actor_id     uuid,
  detected_at              timestamp with time zone DEFAULT now() NOT NULL,
  updated_at               timestamp with time zone DEFAULT now() NOT NULL
);

ALTER TABLE app.insights
  ENABLE ROW LEVEL SECURITY;

ALTER TABLE app.insights
  ADD CONSTRAINT insights_confidence_range CHECK (confidence >= 0::numeric AND confidence <= 1::numeric);

ALTER TABLE app.insights
  ADD CONSTRAINT insights_pkey PRIMARY KEY (id);

ALTER TABLE app.insight_sources
  ADD CONSTRAINT insight_sources_insight_id_fkey FOREIGN KEY (insight_id) REFERENCES app.insights(id) ON DELETE CASCADE;

ALTER TABLE app.insights
  ADD CONSTRAINT insights_public_id_key UNIQUE (public_id);

ALTER TABLE app.insights
  ADD CONSTRAINT insights_severity_valid CHECK (severity = ANY (ARRAY['notice'::text, 'warning'::text, 'blocking'::text]));

ALTER TABLE app.insights
  ADD CONSTRAINT insights_status_valid CHECK (status = ANY (ARRAY['candidate'::text, 'open'::text, 'accepted'::text, 'resolved'::text, 'dismissed'::text]));

ALTER TABLE app.insights
  ADD CONSTRAINT insights_type_valid CHECK (insight_type = ANY (ARRAY['contradiction'::text, 'coverage_gap'::text]));

CREATE INDEX insights_project_status_detected_idx ON app.insights (project_id, status, detected_at DESC);

CREATE INDEX insights_workspace_id_idx ON app.insights (workspace_id);

CREATE TRIGGER insights_set_updated_at
  BEFORE UPDATE ON app.insights
  FOR EACH ROW
  EXECUTE FUNCTION app.set_updated_at();

CREATE TABLE app.knowledge_entries (
  id              bigint                   GENERATED ALWAYS AS IDENTITY NOT NULL,
  public_id       uuid                     DEFAULT gen_random_uuid() NOT NULL,
  workspace_id    bigint                   NOT NULL,
  project_id      bigint                   NOT NULL,
  context_node_id bigint                   NOT NULL,
  entry_type      text                     NOT NULL,
  status          text                     DEFAULT 'confirmed'::text NOT NULL,
  latest_version  integer                  DEFAULT 1 NOT NULL,
  created_at      timestamp with time zone DEFAULT now() NOT NULL,
  updated_at      timestamp with time zone DEFAULT now() NOT NULL
);

ALTER TABLE app.knowledge_entries
  ENABLE ROW LEVEL SECURITY;

ALTER TABLE app.knowledge_entries
  ADD CONSTRAINT knowledge_entries_context_node_id_fkey FOREIGN KEY (context_node_id) REFERENCES app.context_nodes(id) ON DELETE CASCADE;

ALTER TABLE app.knowledge_entries
  ADD CONSTRAINT knowledge_entries_latest_version_positive CHECK (latest_version > 0);

ALTER TABLE app.knowledge_entries
  ADD CONSTRAINT knowledge_entries_pkey PRIMARY KEY (id);

ALTER TABLE app.context_pack_sources
  ADD CONSTRAINT context_pack_sources_knowledge_entry_id_fkey FOREIGN KEY (knowledge_entry_id) REFERENCES app.knowledge_entries(id);

ALTER TABLE app.evidences
  ADD CONSTRAINT evidences_requirement_entry_id_fkey FOREIGN KEY (requirement_entry_id) REFERENCES app.knowledge_entries(id);

ALTER TABLE app.knowledge_entries
  ADD CONSTRAINT knowledge_entries_public_id_key UNIQUE (public_id);

ALTER TABLE app.knowledge_entries
  ADD CONSTRAINT knowledge_entries_status_valid CHECK (status = ANY (ARRAY['confirmed'::text, 'superseded'::text, 'archived'::text]));

ALTER TABLE app.knowledge_entries
  ADD CONSTRAINT knowledge_entries_type_valid
    CHECK
    (entry_type = ANY (ARRAY['decision'::text, 'business_rule'::text, 'technical_rule'::text, 'requirement'::text, 'acceptance_criterion'::text, 'constraint'::text,
    'open_question'::text]));

CREATE INDEX knowledge_entries_workspace_id_idx ON app.knowledge_entries (workspace_id);

CREATE INDEX knowledge_entries_project_node_type_idx ON app.knowledge_entries (project_id, context_node_id, entry_type);

CREATE TRIGGER knowledge_entries_set_updated_at
  BEFORE UPDATE ON app.knowledge_entries
  FOR EACH ROW
  EXECUTE FUNCTION app.set_updated_at();

CREATE TABLE app.knowledge_entry_versions (
  id                       bigint                   GENERATED ALWAYS AS IDENTITY NOT NULL,
  public_id                uuid                     DEFAULT gen_random_uuid() NOT NULL,
  knowledge_entry_id       bigint                   NOT NULL,
  workspace_id             bigint                   NOT NULL,
  project_id               bigint                   NOT NULL,
  context_node_id          bigint                   NOT NULL,
  version_number           integer                  NOT NULL,
  entry_type               text                     NOT NULL,
  title                    text                     NOT NULL,
  statement                text                     NOT NULL,
  rationale                text                     DEFAULT ''::text NOT NULL,
  status                   text                     DEFAULT 'confirmed'::text NOT NULL,
  author_actor_id          uuid                     NOT NULL,
  origin_type              text                     NOT NULL,
  origin_public_id         uuid,
  source_message_public_id uuid,
  created_at               timestamp with time zone DEFAULT now() NOT NULL
);

ALTER TABLE app.knowledge_entry_versions
  ENABLE ROW LEVEL SECURITY;

ALTER TABLE app.knowledge_entry_versions
  ADD CONSTRAINT knowledge_entry_versions_context_node_id_fkey FOREIGN KEY (context_node_id) REFERENCES app.context_nodes(id) ON DELETE CASCADE;

ALTER TABLE app.knowledge_entry_versions
  ADD CONSTRAINT knowledge_entry_versions_entry_version_unique UNIQUE (knowledge_entry_id, version_number);

ALTER TABLE app.knowledge_entry_versions
  ADD CONSTRAINT knowledge_entry_versions_knowledge_entry_id_fkey FOREIGN KEY (knowledge_entry_id) REFERENCES app.knowledge_entries(id) ON DELETE CASCADE;

ALTER TABLE app.knowledge_entry_versions
  ADD CONSTRAINT knowledge_entry_versions_pkey PRIMARY KEY (id);

ALTER TABLE app.context_pack_sources
  ADD CONSTRAINT context_pack_sources_knowledge_entry_version_id_fkey FOREIGN KEY (knowledge_entry_version_id) REFERENCES app.knowledge_entry_versions(id);

ALTER TABLE app.evidences
  ADD CONSTRAINT evidences_requirement_version_id_fkey FOREIGN KEY (requirement_version_id) REFERENCES app.knowledge_entry_versions(id);

ALTER TABLE app.insight_sources
  ADD CONSTRAINT insight_sources_knowledge_entry_version_id_fkey FOREIGN KEY (knowledge_entry_version_id) REFERENCES app.knowledge_entry_versions(id);

ALTER TABLE app.knowledge_entry_versions
  ADD CONSTRAINT knowledge_entry_versions_public_id_key UNIQUE (public_id);

ALTER TABLE app.knowledge_entry_versions
  ADD CONSTRAINT knowledge_entry_versions_statement_not_blank CHECK (btrim(statement) <> ''::text);

ALTER TABLE app.knowledge_entry_versions
  ADD CONSTRAINT knowledge_entry_versions_title_not_blank CHECK (btrim(title) <> ''::text);

ALTER TABLE app.knowledge_entry_versions
  ADD CONSTRAINT knowledge_entry_versions_type_valid
    CHECK
    (entry_type = ANY (ARRAY['decision'::text, 'business_rule'::text, 'technical_rule'::text, 'requirement'::text, 'acceptance_criterion'::text, 'constraint'::text,
    'open_question'::text]));

ALTER TABLE app.knowledge_entry_versions
  ADD CONSTRAINT knowledge_entry_versions_version_positive CHECK (version_number > 0);

CREATE INDEX knowledge_entry_versions_workspace_id_idx ON app.knowledge_entry_versions (workspace_id);

CREATE INDEX knowledge_entry_versions_project_node_idx ON app.knowledge_entry_versions (project_id, context_node_id);

CREATE INDEX knowledge_entry_versions_entry_created_idx ON app.knowledge_entry_versions (knowledge_entry_id, created_at DESC);

CREATE TABLE app.messages (
  id           bigint                   GENERATED ALWAYS AS IDENTITY NOT NULL,
  public_id    uuid                     DEFAULT gen_random_uuid() NOT NULL,
  workspace_id bigint                   NOT NULL,
  project_id   bigint                   NOT NULL,
  session_id   bigint                   NOT NULL,
  role         text                     NOT NULL,
  content      text                     NOT NULL,
  agent_scope  text,
  metadata     jsonb                    DEFAULT '{}'::jsonb NOT NULL,
  created_at   timestamp with time zone DEFAULT now() NOT NULL
);

ALTER TABLE app.messages
  ENABLE ROW LEVEL SECURITY;

ALTER TABLE app.messages
  ADD CONSTRAINT messages_content_not_blank CHECK (btrim(content) <> ''::text);

ALTER TABLE app.messages
  ADD CONSTRAINT messages_pkey PRIMARY KEY (id);

ALTER TABLE app.messages
  ADD CONSTRAINT messages_public_id_key UNIQUE (public_id);

ALTER TABLE app.messages
  ADD CONSTRAINT messages_role_valid CHECK (role = ANY (ARRAY['user'::text, 'assistant'::text, 'system'::text]));

CREATE INDEX messages_session_created_idx ON app.messages (session_id, created_at, id);

CREATE INDEX messages_workspace_id_idx ON app.messages (workspace_id);

CREATE INDEX messages_project_id_idx ON app.messages (project_id);

CREATE TABLE app.mutation_proposals (
  id                           bigint                   GENERATED ALWAYS AS IDENTITY NOT NULL,
  public_id                    uuid                     DEFAULT gen_random_uuid() NOT NULL,
  workspace_id                 bigint                   NOT NULL,
  project_id                   bigint                   NOT NULL,
  session_id                   bigint                   NOT NULL,
  source_message_id            bigint                   NOT NULL,
  proposed_by_agent_profile_id bigint                   NOT NULL,
  entry_type                   text                     NOT NULL,
  title                        text                     NOT NULL,
  statement                    text                     NOT NULL,
  rationale                    text                     DEFAULT ''::text NOT NULL,
  status                       text                     DEFAULT 'proposed'::text NOT NULL,
  source_data                  jsonb                    DEFAULT '{}'::jsonb NOT NULL,
  decided_by_actor_id          uuid,
  decided_at                   timestamp with time zone,
  created_at                   timestamp with time zone DEFAULT now() NOT NULL
);

ALTER TABLE app.mutation_proposals
  ENABLE ROW LEVEL SECURITY;

ALTER TABLE app.mutation_proposals
  ADD CONSTRAINT mutation_proposals_entry_type_valid
    CHECK
    (entry_type = ANY (ARRAY['decision'::text, 'business_rule'::text, 'technical_rule'::text, 'requirement'::text, 'acceptance_criterion'::text, 'constraint'::text,
    'open_question'::text]));

ALTER TABLE app.mutation_proposals
  ADD CONSTRAINT mutation_proposals_pkey PRIMARY KEY (id);

ALTER TABLE app.mutation_proposals
  ADD CONSTRAINT mutation_proposals_proposed_by_agent_profile_id_fkey FOREIGN KEY (proposed_by_agent_profile_id) REFERENCES app.agent_profiles(id);

ALTER TABLE app.mutation_proposals
  ADD CONSTRAINT mutation_proposals_public_id_key UNIQUE (public_id);

ALTER TABLE app.mutation_proposals
  ADD CONSTRAINT mutation_proposals_source_message_id_fkey FOREIGN KEY (source_message_id) REFERENCES app.messages(id) ON DELETE CASCADE;

ALTER TABLE app.mutation_proposals
  ADD CONSTRAINT mutation_proposals_status_valid CHECK (status = ANY (ARRAY['proposed'::text, 'confirmed'::text, 'rejected'::text]));

CREATE INDEX mutation_proposals_workspace_id_idx ON app.mutation_proposals (workspace_id);

CREATE INDEX mutation_proposals_agent_profile_id_idx ON app.mutation_proposals (proposed_by_agent_profile_id);

CREATE INDEX mutation_proposals_source_message_id_idx ON app.mutation_proposals (source_message_id);

CREATE INDEX mutation_proposals_project_id_idx ON app.mutation_proposals (project_id);

CREATE INDEX mutation_proposals_session_status_created_idx ON app.mutation_proposals (session_id, status, created_at);

CREATE TABLE app.project_templates (
  id           bigint                   GENERATED ALWAYS AS IDENTITY NOT NULL,
  public_id    uuid                     DEFAULT gen_random_uuid() NOT NULL,
  template_key text                     NOT NULL,
  name         text                     NOT NULL,
  version      integer                  DEFAULT 1 NOT NULL,
  definition   jsonb                    DEFAULT '{}'::jsonb NOT NULL,
  is_system    boolean                  DEFAULT true NOT NULL,
  created_at   timestamp with time zone DEFAULT now() NOT NULL
);

ALTER TABLE app.project_templates
  ENABLE ROW LEVEL SECURITY;

ALTER TABLE app.project_templates
  ADD CONSTRAINT project_templates_key_not_blank CHECK (btrim(template_key) <> ''::text);

ALTER TABLE app.project_templates
  ADD CONSTRAINT project_templates_pkey PRIMARY KEY (id);

ALTER TABLE app.agent_profiles
  ADD CONSTRAINT agent_profiles_template_id_fkey FOREIGN KEY (template_id) REFERENCES app.project_templates(id) ON DELETE CASCADE;

ALTER TABLE app.deliverable_contracts
  ADD CONSTRAINT deliverable_contracts_template_id_fkey FOREIGN KEY (template_id) REFERENCES app.project_templates(id) ON DELETE CASCADE;

ALTER TABLE app.project_templates
  ADD CONSTRAINT project_templates_public_id_key UNIQUE (public_id);

ALTER TABLE app.project_templates
  ADD CONSTRAINT project_templates_template_key_key UNIQUE (template_key);

ALTER TABLE app.project_templates
  ADD CONSTRAINT project_templates_version_positive CHECK (version > 0);

CREATE TABLE app.projects (
  id                  bigint                   GENERATED ALWAYS AS IDENTITY NOT NULL,
  public_id           uuid                     DEFAULT gen_random_uuid() NOT NULL,
  workspace_id        bigint                   NOT NULL,
  template_id         bigint                   NOT NULL,
  name                text                     NOT NULL,
  objective           text                     DEFAULT ''::text NOT NULL,
  summary             text                     DEFAULT ''::text NOT NULL,
  status              text                     DEFAULT 'active'::text NOT NULL,
  graph_version       bigint                   DEFAULT 0 NOT NULL,
  created_by_actor_id uuid                     NOT NULL,
  created_at          timestamp with time zone DEFAULT now() NOT NULL,
  updated_at          timestamp with time zone DEFAULT now() NOT NULL
);

ALTER TABLE app.projects
  ENABLE ROW LEVEL SECURITY;

ALTER TABLE app.projects
  ADD CONSTRAINT projects_graph_version_nonnegative CHECK (graph_version >= 0);

ALTER TABLE app.projects
  ADD CONSTRAINT projects_name_not_blank CHECK (btrim(name) <> ''::text);

ALTER TABLE app.projects
  ADD CONSTRAINT projects_pkey PRIMARY KEY (id);

ALTER TABLE app.artifacts
  ADD CONSTRAINT artifacts_project_id_fkey FOREIGN KEY (project_id) REFERENCES app.projects(id) ON DELETE CASCADE;

ALTER TABLE app.audit_events
  ADD CONSTRAINT audit_events_project_id_fkey FOREIGN KEY (project_id) REFERENCES app.projects(id) ON DELETE CASCADE;

ALTER TABLE app.context_nodes
  ADD CONSTRAINT context_nodes_project_id_fkey FOREIGN KEY (project_id) REFERENCES app.projects(id) ON DELETE CASCADE;

ALTER TABLE app.context_packs
  ADD CONSTRAINT context_packs_project_id_fkey FOREIGN KEY (project_id) REFERENCES app.projects(id) ON DELETE CASCADE;

ALTER TABLE app.deliverables
  ADD CONSTRAINT deliverables_project_id_fkey FOREIGN KEY (project_id) REFERENCES app.projects(id) ON DELETE CASCADE;

ALTER TABLE app.domain_events
  ADD CONSTRAINT domain_events_project_id_fkey FOREIGN KEY (project_id) REFERENCES app.projects(id) ON DELETE CASCADE;

ALTER TABLE app.edges
  ADD CONSTRAINT edges_project_id_fkey FOREIGN KEY (project_id) REFERENCES app.projects(id) ON DELETE CASCADE;

ALTER TABLE app.evidences
  ADD CONSTRAINT evidences_project_id_fkey FOREIGN KEY (project_id) REFERENCES app.projects(id) ON DELETE CASCADE;

ALTER TABLE app.executions
  ADD CONSTRAINT executions_project_id_fkey FOREIGN KEY (project_id) REFERENCES app.projects(id) ON DELETE CASCADE;

ALTER TABLE app.gates
  ADD CONSTRAINT gates_project_id_fkey FOREIGN KEY (project_id) REFERENCES app.projects(id) ON DELETE CASCADE;

ALTER TABLE app.handoffs
  ADD CONSTRAINT handoffs_project_id_fkey FOREIGN KEY (project_id) REFERENCES app.projects(id) ON DELETE CASCADE;

ALTER TABLE app.insights
  ADD CONSTRAINT insights_project_id_fkey FOREIGN KEY (project_id) REFERENCES app.projects(id) ON DELETE CASCADE;

ALTER TABLE app.knowledge_entries
  ADD CONSTRAINT knowledge_entries_project_id_fkey FOREIGN KEY (project_id) REFERENCES app.projects(id) ON DELETE CASCADE;

ALTER TABLE app.knowledge_entry_versions
  ADD CONSTRAINT knowledge_entry_versions_project_id_fkey FOREIGN KEY (project_id) REFERENCES app.projects(id) ON DELETE CASCADE;

ALTER TABLE app.messages
  ADD CONSTRAINT messages_project_id_fkey FOREIGN KEY (project_id) REFERENCES app.projects(id) ON DELETE CASCADE;

ALTER TABLE app.mutation_proposals
  ADD CONSTRAINT mutation_proposals_project_id_fkey FOREIGN KEY (project_id) REFERENCES app.projects(id) ON DELETE CASCADE;

ALTER TABLE app.projects
  ADD CONSTRAINT projects_public_id_key UNIQUE (public_id);

ALTER TABLE app.projects
  ADD CONSTRAINT projects_status_valid CHECK (status = ANY (ARRAY['active'::text, 'archived'::text]));

ALTER TABLE app.projects
  ADD CONSTRAINT projects_template_id_fkey FOREIGN KEY (template_id) REFERENCES app.project_templates(id);

CREATE INDEX projects_template_id_idx ON app.projects (template_id);

CREATE INDEX projects_workspace_status_created_idx ON app.projects (workspace_id, status, created_at DESC);

CREATE TRIGGER projects_set_updated_at
  BEFORE UPDATE ON app.projects
  FOR EACH ROW
  EXECUTE FUNCTION app.set_updated_at();

CREATE TABLE app.requirement_coverage (
  id                     bigint                   GENERATED ALWAYS AS IDENTITY NOT NULL,
  project_id             bigint                   NOT NULL,
  requirement_entry_id   bigint                   NOT NULL,
  requirement_version_id bigint                   NOT NULL,
  deliverable_id         bigint                   NOT NULL,
  deliverable_section_id bigint,
  evidence_id            bigint,
  status                 text                     NOT NULL,
  explanation            text                     DEFAULT ''::text NOT NULL,
  updated_at             timestamp with time zone DEFAULT now() NOT NULL
);

ALTER TABLE app.requirement_coverage
  ENABLE ROW LEVEL SECURITY;

ALTER TABLE app.requirement_coverage
  ADD CONSTRAINT requirement_coverage_deliverable_id_fkey FOREIGN KEY (deliverable_id) REFERENCES app.deliverables(id) ON DELETE CASCADE;

ALTER TABLE app.requirement_coverage
  ADD CONSTRAINT requirement_coverage_deliverable_section_id_fkey FOREIGN KEY (deliverable_section_id) REFERENCES app.deliverable_sections(id) ON DELETE CASCADE;

ALTER TABLE app.requirement_coverage
  ADD CONSTRAINT requirement_coverage_evidence_id_fkey FOREIGN KEY (evidence_id) REFERENCES app.evidences(id) ON DELETE SET NULL;

ALTER TABLE app.requirement_coverage
  ADD CONSTRAINT requirement_coverage_pkey PRIMARY KEY (id);

ALTER TABLE app.requirement_coverage
  ADD CONSTRAINT requirement_coverage_project_id_fkey FOREIGN KEY (project_id) REFERENCES app.projects(id) ON DELETE CASCADE;

ALTER TABLE app.requirement_coverage
  ADD CONSTRAINT requirement_coverage_relation_unique UNIQUE (requirement_version_id, deliverable_id, deliverable_section_id);

ALTER TABLE app.requirement_coverage
  ADD CONSTRAINT requirement_coverage_requirement_entry_id_fkey FOREIGN KEY (requirement_entry_id) REFERENCES app.knowledge_entries(id) ON DELETE CASCADE;

ALTER TABLE app.requirement_coverage
  ADD CONSTRAINT requirement_coverage_requirement_version_id_fkey FOREIGN KEY (requirement_version_id) REFERENCES app.knowledge_entry_versions(id);

ALTER TABLE app.requirement_coverage
  ADD CONSTRAINT requirement_coverage_status_valid CHECK (status = ANY (ARRAY['covered'::text, 'partial'::text, 'missing'::text]));

CREATE INDEX requirement_coverage_requirement_entry_id_idx ON app.requirement_coverage (requirement_entry_id);

CREATE INDEX requirement_coverage_evidence_id_idx ON app.requirement_coverage (evidence_id);

CREATE INDEX requirement_coverage_deliverable_id_idx ON app.requirement_coverage (deliverable_id);

CREATE INDEX requirement_coverage_section_id_idx ON app.requirement_coverage (deliverable_section_id);

CREATE INDEX requirement_coverage_project_status_idx ON app.requirement_coverage (project_id, status);

CREATE TRIGGER requirement_coverage_set_updated_at
  BEFORE UPDATE ON app.requirement_coverage
  FOR EACH ROW
  EXECUTE FUNCTION app.set_updated_at();

CREATE TABLE app.sessions (
  id                  bigint                   GENERATED ALWAYS AS IDENTITY NOT NULL,
  public_id           uuid                     DEFAULT gen_random_uuid() NOT NULL,
  workspace_id        bigint                   NOT NULL,
  project_id          bigint                   NOT NULL,
  context_node_id     bigint                   NOT NULL,
  agent_profile_id    bigint                   NOT NULL,
  context_pack_id     bigint,
  title               text                     NOT NULL,
  status              text                     DEFAULT 'active'::text NOT NULL,
  created_by_actor_id uuid                     NOT NULL,
  created_at          timestamp with time zone DEFAULT now() NOT NULL,
  updated_at          timestamp with time zone DEFAULT now() NOT NULL
);

ALTER TABLE app.sessions
  ENABLE ROW LEVEL SECURITY;

ALTER TABLE app.sessions
  ADD CONSTRAINT sessions_agent_profile_id_fkey FOREIGN KEY (agent_profile_id) REFERENCES app.agent_profiles(id);

ALTER TABLE app.sessions
  ADD CONSTRAINT sessions_context_node_id_fkey FOREIGN KEY (context_node_id) REFERENCES app.context_nodes(id);

ALTER TABLE app.sessions
  ADD CONSTRAINT sessions_context_pack_id_fkey FOREIGN KEY (context_pack_id) REFERENCES app.context_packs(id);

ALTER TABLE app.sessions
  ADD CONSTRAINT sessions_pkey PRIMARY KEY (id);

ALTER TABLE app.deliverables
  ADD CONSTRAINT deliverables_source_session_id_fkey FOREIGN KEY (source_session_id) REFERENCES app.sessions(id);

ALTER TABLE app.handoffs
  ADD CONSTRAINT handoffs_source_session_id_fkey FOREIGN KEY (source_session_id) REFERENCES app.sessions(id);

ALTER TABLE app.handoffs
  ADD CONSTRAINT handoffs_target_session_id_fkey FOREIGN KEY (target_session_id) REFERENCES app.sessions(id);

ALTER TABLE app.messages
  ADD CONSTRAINT messages_session_id_fkey FOREIGN KEY (session_id) REFERENCES app.sessions(id) ON DELETE CASCADE;

ALTER TABLE app.mutation_proposals
  ADD CONSTRAINT mutation_proposals_session_id_fkey FOREIGN KEY (session_id) REFERENCES app.sessions(id) ON DELETE CASCADE;

ALTER TABLE app.sessions
  ADD CONSTRAINT sessions_project_id_fkey FOREIGN KEY (project_id) REFERENCES app.projects(id) ON DELETE CASCADE;

ALTER TABLE app.sessions
  ADD CONSTRAINT sessions_public_id_key UNIQUE (public_id);

ALTER TABLE app.sessions
  ADD CONSTRAINT sessions_status_valid CHECK (status = ANY (ARRAY['active'::text, 'completed'::text, 'archived'::text]));

CREATE INDEX sessions_agent_profile_id_idx ON app.sessions (agent_profile_id);

CREATE INDEX sessions_project_updated_idx ON app.sessions (project_id, updated_at DESC);

CREATE INDEX sessions_workspace_id_idx ON app.sessions (workspace_id);

CREATE INDEX sessions_context_node_id_idx ON app.sessions (context_node_id);

CREATE INDEX sessions_context_pack_id_idx ON app.sessions (context_pack_id);

CREATE TRIGGER sessions_set_updated_at
  BEFORE UPDATE ON app.sessions
  FOR EACH ROW
  EXECUTE FUNCTION app.set_updated_at();

CREATE TABLE app.tasks (
  id              bigint                   GENERATED ALWAYS AS IDENTITY NOT NULL,
  public_id       uuid                     DEFAULT gen_random_uuid() NOT NULL,
  workspace_id    bigint                   NOT NULL,
  project_id      bigint                   NOT NULL,
  context_pack_id bigint                   NOT NULL,
  contract_id     bigint                   NOT NULL,
  title           text                     NOT NULL,
  status          text                     DEFAULT 'ready'::text NOT NULL,
  created_at      timestamp with time zone DEFAULT now() NOT NULL,
  updated_at      timestamp with time zone DEFAULT now() NOT NULL
);

ALTER TABLE app.tasks
  ENABLE ROW LEVEL SECURITY;

ALTER TABLE app.tasks
  ADD CONSTRAINT tasks_context_pack_id_fkey FOREIGN KEY (context_pack_id) REFERENCES app.context_packs(id);

ALTER TABLE app.tasks
  ADD CONSTRAINT tasks_contract_id_fkey FOREIGN KEY (contract_id) REFERENCES app.deliverable_contracts(id);

ALTER TABLE app.tasks
  ADD CONSTRAINT tasks_pkey PRIMARY KEY (id);

ALTER TABLE app.executions
  ADD CONSTRAINT executions_task_id_fkey FOREIGN KEY (task_id) REFERENCES app.tasks(id) ON DELETE CASCADE;

ALTER TABLE app.tasks
  ADD CONSTRAINT tasks_project_id_fkey FOREIGN KEY (project_id) REFERENCES app.projects(id) ON DELETE CASCADE;

ALTER TABLE app.tasks
  ADD CONSTRAINT tasks_public_id_key UNIQUE (public_id);

ALTER TABLE app.tasks
  ADD CONSTRAINT tasks_status_valid CHECK (status = ANY (ARRAY['ready'::text, 'running'::text, 'completed'::text, 'failed'::text, 'cancelled'::text]));

CREATE INDEX tasks_project_status_created_idx ON app.tasks (project_id, status, created_at DESC);

CREATE INDEX tasks_contract_id_idx ON app.tasks (contract_id);

CREATE INDEX tasks_context_pack_id_idx ON app.tasks (context_pack_id);

CREATE INDEX tasks_workspace_id_idx ON app.tasks (workspace_id);

CREATE TRIGGER tasks_set_updated_at
  BEFORE UPDATE ON app.tasks
  FOR EACH ROW
  EXECUTE FUNCTION app.set_updated_at();

CREATE TABLE app.workspaces (
  id             bigint                   GENERATED ALWAYS AS IDENTITY NOT NULL,
  public_id      uuid                     DEFAULT gen_random_uuid() NOT NULL,
  owner_actor_id uuid                     NOT NULL,
  name           text                     NOT NULL,
  created_at     timestamp with time zone DEFAULT now() NOT NULL,
  updated_at     timestamp with time zone DEFAULT now() NOT NULL
);

ALTER TABLE app.workspaces
  ENABLE ROW LEVEL SECURITY;

ALTER TABLE app.workspaces
  ADD CONSTRAINT workspaces_name_not_blank CHECK (btrim(name) <> ''::text);

ALTER TABLE app.workspaces
  ADD CONSTRAINT workspaces_pkey PRIMARY KEY (id);

ALTER TABLE app.artifacts
  ADD CONSTRAINT artifacts_workspace_id_fkey FOREIGN KEY (workspace_id) REFERENCES app.workspaces(id) ON DELETE CASCADE;

ALTER TABLE app.audit_events
  ADD CONSTRAINT audit_events_workspace_id_fkey FOREIGN KEY (workspace_id) REFERENCES app.workspaces(id) ON DELETE CASCADE;

ALTER TABLE app.context_nodes
  ADD CONSTRAINT context_nodes_workspace_id_fkey FOREIGN KEY (workspace_id) REFERENCES app.workspaces(id) ON DELETE CASCADE;

ALTER TABLE app.context_packs
  ADD CONSTRAINT context_packs_workspace_id_fkey FOREIGN KEY (workspace_id) REFERENCES app.workspaces(id) ON DELETE CASCADE;

ALTER TABLE app.deliverables
  ADD CONSTRAINT deliverables_workspace_id_fkey FOREIGN KEY (workspace_id) REFERENCES app.workspaces(id) ON DELETE CASCADE;

ALTER TABLE app.domain_events
  ADD CONSTRAINT domain_events_workspace_id_fkey FOREIGN KEY (workspace_id) REFERENCES app.workspaces(id) ON DELETE CASCADE;

ALTER TABLE app.edges
  ADD CONSTRAINT edges_workspace_id_fkey FOREIGN KEY (workspace_id) REFERENCES app.workspaces(id) ON DELETE CASCADE;

ALTER TABLE app.evidences
  ADD CONSTRAINT evidences_workspace_id_fkey FOREIGN KEY (workspace_id) REFERENCES app.workspaces(id) ON DELETE CASCADE;

ALTER TABLE app.executions
  ADD CONSTRAINT executions_workspace_id_fkey FOREIGN KEY (workspace_id) REFERENCES app.workspaces(id) ON DELETE CASCADE;

ALTER TABLE app.gates
  ADD CONSTRAINT gates_workspace_id_fkey FOREIGN KEY (workspace_id) REFERENCES app.workspaces(id) ON DELETE CASCADE;

ALTER TABLE app.handoffs
  ADD CONSTRAINT handoffs_workspace_id_fkey FOREIGN KEY (workspace_id) REFERENCES app.workspaces(id) ON DELETE CASCADE;

ALTER TABLE app.insights
  ADD CONSTRAINT insights_workspace_id_fkey FOREIGN KEY (workspace_id) REFERENCES app.workspaces(id) ON DELETE CASCADE;

ALTER TABLE app.knowledge_entries
  ADD CONSTRAINT knowledge_entries_workspace_id_fkey FOREIGN KEY (workspace_id) REFERENCES app.workspaces(id) ON DELETE CASCADE;

ALTER TABLE app.knowledge_entry_versions
  ADD CONSTRAINT knowledge_entry_versions_workspace_id_fkey FOREIGN KEY (workspace_id) REFERENCES app.workspaces(id) ON DELETE CASCADE;

ALTER TABLE app.messages
  ADD CONSTRAINT messages_workspace_id_fkey FOREIGN KEY (workspace_id) REFERENCES app.workspaces(id) ON DELETE CASCADE;

ALTER TABLE app.mutation_proposals
  ADD CONSTRAINT mutation_proposals_workspace_id_fkey FOREIGN KEY (workspace_id) REFERENCES app.workspaces(id) ON DELETE CASCADE;

ALTER TABLE app.projects
  ADD CONSTRAINT projects_workspace_id_fkey FOREIGN KEY (workspace_id) REFERENCES app.workspaces(id) ON DELETE CASCADE;

ALTER TABLE app.sessions
  ADD CONSTRAINT sessions_workspace_id_fkey FOREIGN KEY (workspace_id) REFERENCES app.workspaces(id) ON DELETE CASCADE;

ALTER TABLE app.tasks
  ADD CONSTRAINT tasks_workspace_id_fkey FOREIGN KEY (workspace_id) REFERENCES app.workspaces(id) ON DELETE CASCADE;

ALTER TABLE app.workspaces
  ADD CONSTRAINT workspaces_public_id_key UNIQUE (public_id);

CREATE TRIGGER workspaces_set_updated_at
  BEFORE UPDATE ON app.workspaces
  FOR EACH ROW
  EXECUTE FUNCTION app.set_updated_at();