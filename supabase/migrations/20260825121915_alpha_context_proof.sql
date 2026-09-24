-- Migration unit 1: schema_changes
-- Transaction mode: transactional
-- Boundary reason: default

SET check_function_bodies = false;

ALTER TABLE app.domain_events
  DROP CONSTRAINT domain_events_status_valid;

ALTER TABLE app.edges
  DROP CONSTRAINT edges_type_valid;

ALTER TABLE app.evidences
  DROP CONSTRAINT evidences_status_valid;

DROP INDEX app.domain_events_status_occurred_idx;

GRANT USAGE ON SCHEMA app TO ai_center_runtime;

CREATE FUNCTION app.authorize_workspace_member (
  requested_workspace_public_id uuid,
  requested_actor_id            uuid
)
  RETURNS TABLE (
    workspace_id bigint,
    role         text
  )
  LANGUAGE sql
  STABLE
  SECURITY DEFINER
  SET search_path TO ''
  SET row_security TO 'off'
  AS $function$
  select member.workspace_id, member.role
  from app.workspace_members member
  join app.workspaces workspace on workspace.id = member.workspace_id
  where workspace.public_id = requested_workspace_public_id
    and member.actor_id = requested_actor_id
    and member.invitation_status = 'accepted';
$function$;

REVOKE ALL ON FUNCTION app.authorize_workspace_member(uuid, uuid) FROM PUBLIC;

GRANT ALL ON FUNCTION app.authorize_workspace_member(uuid, uuid) TO ai_center_runtime;

CREATE FUNCTION app.current_actor_id()
  RETURNS uuid
  LANGUAGE sql
  STABLE
  SET search_path TO ''
  AS $function$
  select nullif(current_setting('app.current_actor_id', true), '')::uuid;
$function$;

REVOKE ALL ON FUNCTION app.current_actor_id() FROM PUBLIC;

GRANT ALL ON FUNCTION app.current_actor_id() TO ai_center_runtime;

CREATE FUNCTION app.current_workspace_id()
  RETURNS bigint
  LANGUAGE sql
  STABLE
  SET search_path TO ''
  AS $function$
  select nullif(current_setting('app.current_workspace_id', true), '')::bigint;
$function$;

REVOKE ALL ON FUNCTION app.current_workspace_id() FROM PUBLIC;

GRANT ALL ON FUNCTION app.current_workspace_id() TO ai_center_runtime;

CREATE FUNCTION app.current_workspace_role()
  RETURNS text
  LANGUAGE sql
  STABLE
  SET search_path TO ''
  AS $function$
  select nullif(current_setting('app.current_workspace_role', true), '');
$function$;

REVOKE ALL ON FUNCTION app.current_workspace_role() FROM PUBLIC;

GRANT ALL ON FUNCTION app.current_workspace_role() TO ai_center_runtime;

CREATE FUNCTION app.enforce_context_pack_immutability()
  RETURNS TRIGGER
  LANGUAGE plpgsql
  SET search_path TO ''
  AS $function$
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
$function$;

REVOKE ALL ON FUNCTION app.enforce_context_pack_immutability() FROM PUBLIC;

CREATE FUNCTION app.has_workspace_role (
  requested_workspace_id bigint,
  allowed_roles          text[]
)
  RETURNS boolean
  LANGUAGE sql
  STABLE
  SECURITY DEFINER
  SET search_path TO ''
  SET row_security TO 'off'
  AS $function$
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
$function$;

REVOKE ALL ON FUNCTION app.has_workspace_role(bigint, text[]) FROM PUBLIC;

GRANT ALL ON FUNCTION app.has_workspace_role(bigint, text[]) TO ai_center_runtime;

CREATE FUNCTION app.list_actor_workspaces()
  RETURNS TABLE (
    public_id uuid,
    name      text,
    role      text
  )
  LANGUAGE sql
  STABLE
  SECURITY DEFINER
  SET search_path TO ''
  SET row_security TO 'off'
  AS $function$
  select workspace.public_id, workspace.name, member.role
  from app.workspace_members member
  join app.workspaces workspace on workspace.id = member.workspace_id
  where member.actor_id = app.current_actor_id()
    and member.invitation_status = 'accepted'
  order by workspace.name, workspace.id;
$function$;

REVOKE ALL ON FUNCTION app.list_actor_workspaces() FROM PUBLIC;

GRANT ALL ON FUNCTION app.list_actor_workspaces() TO ai_center_runtime;

CREATE FUNCTION app.prevent_append_only_mutation()
  RETURNS TRIGGER
  LANGUAGE plpgsql
  SET search_path TO ''
  AS $function$
begin
  raise exception '% rows are append-only', tg_table_name;
end;
$function$;

REVOKE ALL ON FUNCTION app.prevent_append_only_mutation() FROM PUBLIC;

ALTER TABLE app.evidences
  ALTER COLUMN status SET DEFAULT 'candidate'::text;

ALTER TABLE app.agent_profiles
  FORCE ROW LEVEL SECURITY;

GRANT SELECT ON app.agent_profiles TO ai_center_runtime;

CREATE POLICY system_catalog_member_select ON app.agent_profiles
  FOR SELECT
  USING (( SELECT app.has_workspace_role(app.current_workspace_id(), ARRAY['owner'::text, 'editor'::text, 'viewer'::text]) AS has_workspace_role));

ALTER TABLE app.artifacts
  FORCE ROW LEVEL SECURITY;

ALTER TABLE app.artifacts
  ADD CONSTRAINT artifacts_id_project_unique UNIQUE (id, project_id);

ALTER TABLE app.artifacts
  ADD COLUMN external_reference_id bigint;

GRANT SELECT ON app.artifacts TO ai_center_runtime;

CREATE INDEX artifacts_external_reference_id_idx ON app.artifacts (external_reference_id)
  WHERE external_reference_id IS NOT NULL;

CREATE TRIGGER artifacts_prevent_update
  BEFORE DELETE OR UPDATE ON app.artifacts
  FOR EACH ROW
  EXECUTE FUNCTION app.prevent_append_only_mutation();

CREATE POLICY tenant_editor_insert ON app.artifacts
  FOR INSERT
  WITH
    CHECK (((workspace_id = app.current_workspace_id()) AND ( SELECT app.has_workspace_role(artifacts.workspace_id, ARRAY['owner'::text, 'editor'::text]) AS has_workspace_role)));

CREATE POLICY tenant_member_select ON app.artifacts
  FOR SELECT
  USING
    (((workspace_id = app.current_workspace_id()) AND ( SELECT app.has_workspace_role(artifacts.workspace_id, ARRAY['owner'::text, 'editor'::text, 'viewer'::text]) AS
    has_workspace_role)));

ALTER TABLE app.audit_events
  FORCE ROW LEVEL SECURITY;

GRANT INSERT, SELECT ON app.audit_events TO ai_center_runtime;

CREATE POLICY tenant_editor_insert ON app.audit_events
  FOR INSERT
  WITH
    CHECK
    (((workspace_id = app.current_workspace_id()) AND ( SELECT app.has_workspace_role(audit_events.workspace_id, ARRAY['owner'::text, 'editor'::text]) AS has_workspace_role)));

CREATE POLICY tenant_member_select ON app.audit_events
  FOR SELECT
  USING
    (((workspace_id = app.current_workspace_id()) AND ( SELECT app.has_workspace_role(audit_events.workspace_id, ARRAY['owner'::text, 'editor'::text, 'viewer'::text]) AS
    has_workspace_role)));

ALTER TABLE app.context_nodes
  FORCE ROW LEVEL SECURITY;

ALTER TABLE app.context_nodes
  ADD CONSTRAINT context_nodes_id_project_unique UNIQUE (id, project_id);

ALTER TABLE app.context_nodes
  ADD CONSTRAINT context_nodes_parent_project_fkey FOREIGN KEY (parent_id, project_id) REFERENCES app.context_nodes(id, project_id);

GRANT INSERT, SELECT ON app.context_nodes TO ai_center_runtime;

CREATE POLICY tenant_editor_insert ON app.context_nodes
  FOR INSERT
  WITH
    CHECK
    (((workspace_id = app.current_workspace_id()) AND ( SELECT app.has_workspace_role(context_nodes.workspace_id, ARRAY['owner'::text, 'editor'::text]) AS has_workspace_role)));

CREATE POLICY tenant_editor_update ON app.context_nodes
  FOR UPDATE
  USING (((workspace_id = app.current_workspace_id()) AND ( SELECT app.has_workspace_role(context_nodes.workspace_id, ARRAY['owner'::text, 'editor'::text]) AS has_workspace_role)))
  WITH
    CHECK
    (((workspace_id = app.current_workspace_id()) AND ( SELECT app.has_workspace_role(context_nodes.workspace_id, ARRAY['owner'::text, 'editor'::text]) AS has_workspace_role)));

CREATE POLICY tenant_member_select ON app.context_nodes
  FOR SELECT
  USING
    (((workspace_id = app.current_workspace_id()) AND ( SELECT app.has_workspace_role(context_nodes.workspace_id, ARRAY['owner'::text, 'editor'::text, 'viewer'::text]) AS
    has_workspace_role)));

CREATE TABLE app.context_pack_selection_items (
  id                                bigint                   GENERATED ALWAYS AS IDENTITY NOT NULL,
  public_id                         uuid                     DEFAULT gen_random_uuid() NOT NULL,
  workspace_id                      bigint                   NOT NULL,
  project_id                        bigint                   NOT NULL,
  context_pack_id                   bigint                   NOT NULL,
  candidate_kind                    text                     NOT NULL,
  candidate_public_id               uuid                     NOT NULL,
  knowledge_entry_version_id        bigint,
  external_reference_observation_id bigint,
  decision                          text                     NOT NULL,
  reason_code                       text                     NOT NULL,
  explanation                       text                     DEFAULT ''::text NOT NULL,
  rank                              integer,
  estimated_tokens                  integer                  DEFAULT 0 NOT NULL,
  is_mandatory                      boolean                  DEFAULT false NOT NULL,
  created_at                        timestamp with time zone DEFAULT now() NOT NULL
);

ALTER TABLE app.context_pack_selection_items
  ENABLE ROW LEVEL SECURITY;

ALTER TABLE app.context_pack_selection_items
  FORCE ROW LEVEL SECURITY;

ALTER TABLE app.context_pack_selection_items
  ADD CONSTRAINT context_pack_selection_items_candidate_kind_valid CHECK (candidate_kind = ANY (ARRAY['knowledge_entry_version'::text, 'external_reference_observation'::text]));

ALTER TABLE app.context_pack_selection_items
  ADD CONSTRAINT context_pack_selection_items_candidate_unique UNIQUE (context_pack_id, candidate_kind, candidate_public_id);

ALTER TABLE app.context_pack_selection_items
  ADD CONSTRAINT context_pack_selection_items_context_pack_id_fkey FOREIGN KEY (context_pack_id) REFERENCES app.context_packs(id) ON DELETE CASCADE;

ALTER TABLE app.context_pack_selection_items
  ADD CONSTRAINT context_pack_selection_items_decision_valid CHECK (decision = ANY (ARRAY['included'::text, 'excluded'::text]));

ALTER TABLE app.context_pack_selection_items
  ADD CONSTRAINT context_pack_selection_items_knowledge_entry_version_id_fkey FOREIGN KEY (knowledge_entry_version_id) REFERENCES app.knowledge_entry_versions(id);

ALTER TABLE app.context_pack_selection_items
  ADD CONSTRAINT context_pack_selection_items_mandatory_included CHECK (NOT is_mandatory OR decision = 'included'::text);

ALTER TABLE app.context_pack_selection_items
  ADD CONSTRAINT context_pack_selection_items_pkey PRIMARY KEY (id);

ALTER TABLE app.context_pack_selection_items
  ADD CONSTRAINT context_pack_selection_items_project_id_fkey FOREIGN KEY (project_id) REFERENCES app.projects(id) ON DELETE CASCADE;

ALTER TABLE app.context_pack_selection_items
  ADD CONSTRAINT context_pack_selection_items_public_id_key UNIQUE (public_id);

ALTER TABLE app.context_pack_selection_items
  ADD CONSTRAINT context_pack_selection_items_rank_nonnegative CHECK (rank IS NULL OR rank >= 0);

ALTER TABLE app.context_pack_selection_items
  ADD CONSTRAINT context_pack_selection_items_reason_code_not_blank CHECK (btrim(reason_code) <> ''::text);

ALTER TABLE app.context_pack_selection_items
  ADD CONSTRAINT context_pack_selection_items_tokens_nonnegative CHECK (estimated_tokens >= 0);

ALTER TABLE app.context_pack_selection_items
  ADD CONSTRAINT context_pack_selection_items_typed_source_consistent CHECK (candidate_kind = 'knowledge_entry_version'::text AND knowledge_entry_version_id IS
    NOT NULL AND external_reference_observation_id IS NULL OR candidate_kind = 'external_reference_observation'::text AND knowledge_entry_version_id IS NULL AND
    external_reference_observation_id IS NOT NULL);

ALTER TABLE app.context_pack_selection_items
  ADD CONSTRAINT context_pack_selection_items_workspace_id_fkey FOREIGN KEY (workspace_id) REFERENCES app.workspaces(id) ON DELETE CASCADE;

GRANT INSERT, SELECT ON app.context_pack_selection_items TO ai_center_runtime;

CREATE INDEX context_pack_selection_items_knowledge_version_id_idx ON app.context_pack_selection_items (knowledge_entry_version_id)
  WHERE knowledge_entry_version_id IS NOT NULL;

CREATE INDEX context_pack_selection_items_external_observation_id_idx ON app.context_pack_selection_items (external_reference_observation_id)
  WHERE external_reference_observation_id IS NOT NULL;

CREATE INDEX context_pack_selection_items_pack_decision_rank_idx ON app.context_pack_selection_items (context_pack_id, decision, rank);

CREATE INDEX context_pack_selection_items_workspace_id_idx ON app.context_pack_selection_items (workspace_id);

CREATE INDEX context_pack_selection_items_project_id_idx ON app.context_pack_selection_items (project_id);

CREATE TRIGGER context_pack_selection_items_prevent_update
  BEFORE DELETE OR UPDATE ON app.context_pack_selection_items
  FOR EACH ROW
  EXECUTE FUNCTION app.prevent_append_only_mutation();

CREATE POLICY tenant_editor_insert ON app.context_pack_selection_items
  FOR INSERT
  WITH
    CHECK
    (((workspace_id = app.current_workspace_id()) AND ( SELECT app.has_workspace_role(context_pack_selection_items.workspace_id, ARRAY['owner'::text, 'editor'::text]) AS
    has_workspace_role)));

CREATE POLICY tenant_member_select ON app.context_pack_selection_items
  FOR SELECT
  USING
    (((workspace_id = app.current_workspace_id()) AND ( SELECT app.has_workspace_role(context_pack_selection_items.workspace_id, ARRAY['owner'::text, 'editor'::text,
    'viewer'::text]) AS has_workspace_role)));

ALTER TABLE app.context_pack_sources
  FORCE ROW LEVEL SECURITY;

ALTER TABLE app.context_pack_sources
  ADD COLUMN workspace_id bigint;

ALTER TABLE app.context_pack_sources
  ADD CONSTRAINT context_pack_sources_workspace_id_fkey FOREIGN KEY (workspace_id) REFERENCES app.workspaces(id) ON DELETE CASCADE;

ALTER TABLE app.context_pack_sources
  ADD COLUMN project_id bigint;

UPDATE app.context_pack_sources source
SET workspace_id = pack.workspace_id,
    project_id = pack.project_id
FROM app.context_packs pack
WHERE pack.id = source.context_pack_id;

ALTER TABLE app.context_pack_sources
  ALTER COLUMN workspace_id SET NOT NULL,
  ALTER COLUMN project_id SET NOT NULL;

ALTER TABLE app.context_pack_sources
  ADD CONSTRAINT context_pack_sources_project_id_fkey FOREIGN KEY (project_id) REFERENCES app.projects(id) ON DELETE CASCADE;

GRANT INSERT, SELECT ON app.context_pack_sources TO ai_center_runtime;

CREATE INDEX context_pack_sources_project_id_idx ON app.context_pack_sources (project_id);

CREATE INDEX context_pack_sources_workspace_id_idx ON app.context_pack_sources (workspace_id);

CREATE TRIGGER context_pack_sources_prevent_update
  BEFORE DELETE OR UPDATE ON app.context_pack_sources
  FOR EACH ROW
  EXECUTE FUNCTION app.prevent_append_only_mutation();

CREATE POLICY tenant_editor_insert ON app.context_pack_sources
  FOR INSERT
  WITH
    CHECK
    (((workspace_id = app.current_workspace_id()) AND ( SELECT app.has_workspace_role(context_pack_sources.workspace_id, ARRAY['owner'::text, 'editor'::text]) AS
    has_workspace_role)));

CREATE POLICY tenant_member_select ON app.context_pack_sources
  FOR SELECT
  USING
    (((workspace_id = app.current_workspace_id()) AND ( SELECT app.has_workspace_role(context_pack_sources.workspace_id, ARRAY['owner'::text, 'editor'::text, 'viewer'::text]) AS
    has_workspace_role)));

ALTER TABLE app.context_packs
  FORCE ROW LEVEL SECURITY;

ALTER TABLE app.context_packs
  ADD CONSTRAINT context_packs_id_project_unique UNIQUE (id, project_id);

ALTER TABLE app.context_pack_selection_items
  ADD CONSTRAINT context_pack_selection_items_pack_project_fkey FOREIGN KEY (context_pack_id, project_id) REFERENCES app.context_packs(id, project_id);

ALTER TABLE app.context_pack_sources
  ADD CONSTRAINT context_pack_sources_pack_project_fkey FOREIGN KEY (context_pack_id, project_id) REFERENCES app.context_packs(id, project_id);

ALTER TABLE app.context_packs
  ADD CONSTRAINT context_packs_source_node_project_fkey FOREIGN KEY (source_node_id, project_id) REFERENCES app.context_nodes(id, project_id);

ALTER TABLE app.context_packs
  ADD CONSTRAINT context_packs_target_node_project_fkey FOREIGN KEY (target_node_id, project_id) REFERENCES app.context_nodes(id, project_id);

ALTER TABLE app.context_packs
  ADD CONSTRAINT context_packs_target_version_unique UNIQUE (project_id, target_node_id, task_kind, VERSION);

ALTER TABLE app.context_packs
  ADD COLUMN source_graph_version bigint;

ALTER TABLE app.context_packs
  ADD CONSTRAINT context_packs_graph_version_nonnegative CHECK (source_graph_version >= 0);

ALTER TABLE app.context_packs
  ADD COLUMN compiler_version text;

ALTER TABLE app.context_packs
  ADD CONSTRAINT context_packs_compiler_version_not_blank CHECK (btrim(compiler_version) <> ''::text);

ALTER TABLE app.context_packs
  ADD COLUMN selection_mode text DEFAULT 'deterministic'::text NOT NULL;

ALTER TABLE app.context_packs
  ADD CONSTRAINT context_packs_selection_mode_valid CHECK (selection_mode = ANY (ARRAY['deterministic'::text, 'hybrid'::text]));

ALTER TABLE app.context_packs
  ADD COLUMN content_hash text;

ALTER TABLE app.context_packs
  ADD CONSTRAINT context_packs_content_hash_valid CHECK (content_hash ~ '^[0-9a-f]{64}$'::text);

ALTER TABLE app.context_packs
  ADD COLUMN token_budget integer DEFAULT 12000 NOT NULL;

ALTER TABLE app.context_packs
  ADD CONSTRAINT context_packs_token_budget_positive CHECK (token_budget > 0);

ALTER TABLE app.context_packs
  ADD COLUMN estimated_tokens integer DEFAULT 0 NOT NULL;

ALTER TABLE app.context_packs
  ADD CONSTRAINT context_packs_estimated_tokens_nonnegative CHECK (estimated_tokens >= 0);

UPDATE app.context_packs pack
SET source_graph_version = CASE
      WHEN pack.content ->> 'graph_version' ~ '^[0-9]+$'
        THEN (pack.content ->> 'graph_version')::bigint
      ELSE project.graph_version
    END,
    compiler_version = 'legacy-v1',
    content_hash = encode(
      extensions.digest(convert_to(pack.content::text, 'UTF8'), 'sha256'),
      'hex'
    )
FROM app.projects project
WHERE project.id = pack.project_id;

ALTER TABLE app.context_packs
  ALTER COLUMN source_graph_version SET NOT NULL,
  ALTER COLUMN compiler_version SET NOT NULL,
  ALTER COLUMN content_hash SET NOT NULL;

ALTER TABLE app.context_packs
  ADD CONSTRAINT context_packs_within_token_budget CHECK (estimated_tokens <= token_budget);

ALTER TABLE app.context_packs
  ADD COLUMN supersedes_context_pack_id bigint;

ALTER TABLE app.context_packs
  ADD CONSTRAINT context_packs_supersedes_context_pack_id_fkey FOREIGN KEY (supersedes_context_pack_id) REFERENCES app.context_packs(id);

ALTER TABLE app.context_packs
  ADD CONSTRAINT context_packs_supersedes_project_fkey FOREIGN KEY (supersedes_context_pack_id, project_id) REFERENCES app.context_packs(id, project_id);

ALTER TABLE app.context_packs
  ADD COLUMN stale_reason text;

WITH ranked_current AS (
  SELECT id,
         row_number() OVER (
           PARTITION BY project_id, target_node_id, task_kind
           ORDER BY compiled_at DESC, id DESC
         ) AS current_rank
  FROM app.context_packs
  WHERE status = 'current'
)
UPDATE app.context_packs pack
SET status = 'superseded',
    invalidated_at = coalesce(pack.invalidated_at, clock_timestamp()),
    stale_reason = 'legacy duplicate current pack superseded during alpha migration'
FROM ranked_current ranked
WHERE ranked.id = pack.id
  AND ranked.current_rank > 1;

UPDATE app.context_packs
SET invalidated_at = coalesce(invalidated_at, clock_timestamp()),
    stale_reason = coalesce(stale_reason, 'legacy pack invalidated before alpha migration')
WHERE status <> 'current';

ALTER TABLE app.context_packs
  ADD CONSTRAINT context_packs_invalidation_consistent
    CHECK (status = 'current'::text AND invalidated_at IS NULL AND stale_reason IS NULL OR status <> 'current'::text AND invalidated_at IS NOT NULL AND stale_reason IS NOT NULL);

GRANT INSERT, SELECT, UPDATE ON app.context_packs TO ai_center_runtime;

CREATE INDEX context_packs_supersedes_id_idx ON app.context_packs (supersedes_context_pack_id)
  WHERE supersedes_context_pack_id IS NOT NULL;

CREATE UNIQUE INDEX context_packs_one_current_target_idx ON app.context_packs (project_id, target_node_id, task_kind)
  WHERE status = 'current'::text;

CREATE TRIGGER context_packs_enforce_immutability
  BEFORE UPDATE ON app.context_packs
  FOR EACH ROW
  EXECUTE FUNCTION app.enforce_context_pack_immutability();

CREATE TRIGGER context_packs_prevent_delete
  BEFORE DELETE ON app.context_packs
  FOR EACH ROW
  EXECUTE FUNCTION app.prevent_append_only_mutation();

CREATE POLICY tenant_editor_insert ON app.context_packs
  FOR INSERT
  WITH
    CHECK
    (((workspace_id = app.current_workspace_id()) AND ( SELECT app.has_workspace_role(context_packs.workspace_id, ARRAY['owner'::text, 'editor'::text]) AS has_workspace_role)));

CREATE POLICY tenant_editor_update ON app.context_packs
  FOR UPDATE
  USING (((workspace_id = app.current_workspace_id()) AND ( SELECT app.has_workspace_role(context_packs.workspace_id, ARRAY['owner'::text, 'editor'::text]) AS has_workspace_role)))
  WITH
    CHECK
    (((workspace_id = app.current_workspace_id()) AND ( SELECT app.has_workspace_role(context_packs.workspace_id, ARRAY['owner'::text, 'editor'::text]) AS has_workspace_role)));

CREATE POLICY tenant_member_select ON app.context_packs
  FOR SELECT
  USING
    (((workspace_id = app.current_workspace_id()) AND ( SELECT app.has_workspace_role(context_packs.workspace_id, ARRAY['owner'::text, 'editor'::text, 'viewer'::text]) AS
    has_workspace_role)));

ALTER TABLE app.deliverable_contracts
  FORCE ROW LEVEL SECURITY;

GRANT SELECT ON app.deliverable_contracts TO ai_center_runtime;

CREATE POLICY system_catalog_member_select ON app.deliverable_contracts
  FOR SELECT
  USING (( SELECT app.has_workspace_role(app.current_workspace_id(), ARRAY['owner'::text, 'editor'::text, 'viewer'::text]) AS has_workspace_role));

ALTER TABLE app.deliverable_sections
  FORCE ROW LEVEL SECURITY;

ALTER TABLE app.deliverable_sections
  ADD COLUMN workspace_id bigint;

ALTER TABLE app.deliverable_sections
  ADD CONSTRAINT deliverable_sections_workspace_id_fkey FOREIGN KEY (workspace_id) REFERENCES app.workspaces(id) ON DELETE CASCADE;

ALTER TABLE app.deliverable_sections
  ADD COLUMN project_id bigint;

UPDATE app.deliverable_sections section
SET workspace_id = deliverable.workspace_id,
    project_id = deliverable.project_id
FROM app.deliverables deliverable
WHERE deliverable.id = section.deliverable_id;

ALTER TABLE app.deliverable_sections
  ALTER COLUMN workspace_id SET NOT NULL,
  ALTER COLUMN project_id SET NOT NULL;

ALTER TABLE app.deliverable_sections
  ADD CONSTRAINT deliverable_sections_id_project_unique UNIQUE (id, project_id);

ALTER TABLE app.deliverable_sections
  ADD CONSTRAINT deliverable_sections_project_id_fkey FOREIGN KEY (project_id) REFERENCES app.projects(id) ON DELETE CASCADE;

GRANT INSERT, SELECT ON app.deliverable_sections TO ai_center_runtime;

CREATE INDEX deliverable_sections_workspace_id_idx ON app.deliverable_sections (workspace_id);

CREATE INDEX deliverable_sections_project_id_idx ON app.deliverable_sections (project_id);

CREATE TRIGGER deliverable_sections_prevent_update
  BEFORE DELETE OR UPDATE ON app.deliverable_sections
  FOR EACH ROW
  EXECUTE FUNCTION app.prevent_append_only_mutation();

CREATE POLICY tenant_editor_insert ON app.deliverable_sections
  FOR INSERT
  WITH
    CHECK
    (((workspace_id = app.current_workspace_id()) AND ( SELECT app.has_workspace_role(deliverable_sections.workspace_id, ARRAY['owner'::text, 'editor'::text]) AS
    has_workspace_role)));

CREATE POLICY tenant_member_select ON app.deliverable_sections
  FOR SELECT
  USING
    (((workspace_id = app.current_workspace_id()) AND ( SELECT app.has_workspace_role(deliverable_sections.workspace_id, ARRAY['owner'::text, 'editor'::text, 'viewer'::text]) AS
    has_workspace_role)));

CREATE TABLE app.deliverable_sources (
  id                                bigint                   GENERATED ALWAYS AS IDENTITY NOT NULL,
  public_id                         uuid                     DEFAULT gen_random_uuid() NOT NULL,
  workspace_id                      bigint                   NOT NULL,
  project_id                        bigint                   NOT NULL,
  deliverable_id                    bigint                   NOT NULL,
  source_kind                       text                     NOT NULL,
  source_public_id                  uuid                     NOT NULL,
  knowledge_entry_version_id        bigint,
  context_pack_id                   bigint,
  external_reference_observation_id bigint,
  model_run_id                      bigint,
  source_role                       text                     NOT NULL,
  included_reason                   text                     DEFAULT ''::text NOT NULL,
  created_at                        timestamp with time zone DEFAULT now() NOT NULL
);

ALTER TABLE app.deliverable_sources
  ENABLE ROW LEVEL SECURITY;

ALTER TABLE app.deliverable_sources
  FORCE ROW LEVEL SECURITY;

ALTER TABLE app.deliverable_sources
  ADD CONSTRAINT deliverable_sources_context_pack_id_fkey FOREIGN KEY (context_pack_id) REFERENCES app.context_packs(id);

ALTER TABLE app.deliverable_sources
  ADD CONSTRAINT deliverable_sources_deliverable_id_fkey FOREIGN KEY (deliverable_id) REFERENCES app.deliverables(id) ON DELETE CASCADE;

ALTER TABLE app.deliverable_sources
  ADD CONSTRAINT deliverable_sources_kind_valid
    CHECK (source_kind = ANY (ARRAY['knowledge_entry_version'::text, 'context_pack'::text, 'external_reference_observation'::text, 'model_run'::text]));

ALTER TABLE app.deliverable_sources
  ADD CONSTRAINT deliverable_sources_knowledge_entry_version_id_fkey FOREIGN KEY (knowledge_entry_version_id) REFERENCES app.knowledge_entry_versions(id);

ALTER TABLE app.deliverable_sources
  ADD CONSTRAINT deliverable_sources_pack_project_fkey FOREIGN KEY (context_pack_id, project_id) REFERENCES app.context_packs(id, project_id);

ALTER TABLE app.deliverable_sources
  ADD CONSTRAINT deliverable_sources_pkey PRIMARY KEY (id);

ALTER TABLE app.deliverable_sources
  ADD CONSTRAINT deliverable_sources_project_id_fkey FOREIGN KEY (project_id) REFERENCES app.projects(id) ON DELETE CASCADE;

ALTER TABLE app.deliverable_sources
  ADD CONSTRAINT deliverable_sources_public_id_key UNIQUE (public_id);

ALTER TABLE app.deliverable_sources
  ADD CONSTRAINT deliverable_sources_relation_unique UNIQUE (deliverable_id, source_kind, source_public_id);

ALTER TABLE app.deliverable_sources
  ADD CONSTRAINT deliverable_sources_role_not_blank CHECK (btrim(source_role) <> ''::text);

ALTER TABLE app.deliverable_sources
  ADD CONSTRAINT deliverable_sources_typed_source_consistent CHECK (source_kind = 'knowledge_entry_version'::text AND knowledge_entry_version_id IS
    NOT NULL AND context_pack_id IS NULL AND external_reference_observation_id IS NULL AND model_run_id IS NULL OR source_kind = 'context_pack'::text AND knowledge_entry_version_id
    IS NULL AND context_pack_id IS
    NOT NULL AND external_reference_observation_id IS NULL AND model_run_id IS NULL OR source_kind = 'external_reference_observation'::text AND knowledge_entry_version_id IS NULL
    AND context_pack_id IS NULL AND external_reference_observation_id IS
    NOT NULL AND model_run_id IS NULL OR source_kind = 'model_run'::text AND knowledge_entry_version_id IS NULL AND context_pack_id IS NULL AND external_reference_observation_id IS
    NULL AND model_run_id IS NOT NULL);

ALTER TABLE app.deliverable_sources
  ADD CONSTRAINT deliverable_sources_workspace_id_fkey FOREIGN KEY (workspace_id) REFERENCES app.workspaces(id) ON DELETE CASCADE;

GRANT INSERT, SELECT ON app.deliverable_sources TO ai_center_runtime;

CREATE INDEX deliverable_sources_project_id_idx ON app.deliverable_sources (project_id);

CREATE INDEX deliverable_sources_knowledge_version_id_idx ON app.deliverable_sources (knowledge_entry_version_id)
  WHERE knowledge_entry_version_id IS NOT NULL;

CREATE INDEX deliverable_sources_context_pack_id_idx ON app.deliverable_sources (context_pack_id)
  WHERE context_pack_id IS NOT NULL;

CREATE INDEX deliverable_sources_external_observation_id_idx ON app.deliverable_sources (external_reference_observation_id)
  WHERE external_reference_observation_id IS NOT NULL;

CREATE INDEX deliverable_sources_model_run_id_idx ON app.deliverable_sources (model_run_id)
  WHERE model_run_id IS NOT NULL;

CREATE INDEX deliverable_sources_deliverable_id_idx ON app.deliverable_sources (deliverable_id);

CREATE INDEX deliverable_sources_workspace_id_idx ON app.deliverable_sources (workspace_id);

CREATE TRIGGER deliverable_sources_prevent_update
  BEFORE DELETE OR UPDATE ON app.deliverable_sources
  FOR EACH ROW
  EXECUTE FUNCTION app.prevent_append_only_mutation();

CREATE POLICY tenant_editor_insert ON app.deliverable_sources
  FOR INSERT
  WITH
    CHECK
    (((workspace_id = app.current_workspace_id()) AND ( SELECT app.has_workspace_role(deliverable_sources.workspace_id, ARRAY['owner'::text, 'editor'::text]) AS
    has_workspace_role)));

CREATE POLICY tenant_member_select ON app.deliverable_sources
  FOR SELECT
  USING
    (((workspace_id = app.current_workspace_id()) AND ( SELECT app.has_workspace_role(deliverable_sources.workspace_id, ARRAY['owner'::text, 'editor'::text, 'viewer'::text]) AS
    has_workspace_role)));

ALTER TABLE app.deliverables
  FORCE ROW LEVEL SECURITY;

ALTER TABLE app.deliverables
  ADD CONSTRAINT deliverables_context_node_project_fkey FOREIGN KEY (context_node_id, project_id) REFERENCES app.context_nodes(id, project_id);

ALTER TABLE app.deliverables
  ADD CONSTRAINT deliverables_context_pack_project_fkey FOREIGN KEY (source_context_pack_id, project_id) REFERENCES app.context_packs(id, project_id);

ALTER TABLE app.deliverables
  ADD CONSTRAINT deliverables_id_project_unique UNIQUE (id, project_id);

ALTER TABLE app.artifacts
  ADD CONSTRAINT artifacts_deliverable_project_fkey FOREIGN KEY (deliverable_id, project_id) REFERENCES app.deliverables(id, project_id);

ALTER TABLE app.deliverable_sections
  ADD CONSTRAINT deliverable_sections_deliverable_project_fkey FOREIGN KEY (deliverable_id, project_id) REFERENCES app.deliverables(id, project_id);

ALTER TABLE app.deliverable_sources
  ADD CONSTRAINT deliverable_sources_deliverable_project_fkey FOREIGN KEY (deliverable_id, project_id) REFERENCES app.deliverables(id, project_id);

ALTER TABLE app.deliverables
  ADD COLUMN lineage_public_id uuid DEFAULT gen_random_uuid() NOT NULL;

ALTER TABLE app.deliverables
  ADD CONSTRAINT deliverables_lineage_version_unique UNIQUE (project_id, lineage_public_id, VERSION);

ALTER TABLE app.deliverables
  ADD COLUMN supersedes_deliverable_id bigint;

ALTER TABLE app.deliverables
  ADD CONSTRAINT deliverables_supersedes_deliverable_id_fkey FOREIGN KEY (supersedes_deliverable_id) REFERENCES app.deliverables(id);

ALTER TABLE app.deliverables
  ADD CONSTRAINT deliverables_supersedes_project_fkey FOREIGN KEY (supersedes_deliverable_id, project_id) REFERENCES app.deliverables(id, project_id);

ALTER TABLE app.deliverables
  ADD CONSTRAINT deliverables_supersedes_unique UNIQUE (supersedes_deliverable_id);

ALTER TABLE app.deliverables
  ADD COLUMN source_graph_version bigint;

ALTER TABLE app.deliverables
  ADD CONSTRAINT deliverables_source_graph_version_nonnegative CHECK (source_graph_version >= 0);

ALTER TABLE app.deliverables
  ADD COLUMN content_hash text;

UPDATE app.deliverables deliverable
SET source_graph_version = project.graph_version,
    content_hash = encode(
      extensions.digest(convert_to(deliverable.content::text, 'UTF8'), 'sha256'),
      'hex'
    )
FROM app.projects project
WHERE project.id = deliverable.project_id;

ALTER TABLE app.deliverables
  ALTER COLUMN source_graph_version SET NOT NULL,
  ALTER COLUMN content_hash SET NOT NULL;

ALTER TABLE app.deliverables
  ADD CONSTRAINT deliverables_content_hash_valid CHECK (content_hash ~ '^[0-9a-f]{64}$'::text);

GRANT INSERT, SELECT, UPDATE ON app.deliverables TO ai_center_runtime;

CREATE UNIQUE INDEX deliverables_one_current_lineage_idx ON app.deliverables (project_id, lineage_public_id)
  WHERE status <> 'superseded'::text;

CREATE INDEX deliverables_lineage_created_idx ON app.deliverables (project_id, lineage_public_id, created_at DESC);

CREATE POLICY tenant_editor_insert ON app.deliverables
  FOR INSERT
  WITH
    CHECK
    (((workspace_id = app.current_workspace_id()) AND ( SELECT app.has_workspace_role(deliverables.workspace_id, ARRAY['owner'::text, 'editor'::text]) AS has_workspace_role)));

CREATE POLICY tenant_editor_update ON app.deliverables
  FOR UPDATE
  USING (((workspace_id = app.current_workspace_id()) AND ( SELECT app.has_workspace_role(deliverables.workspace_id, ARRAY['owner'::text, 'editor'::text]) AS has_workspace_role)))
  WITH
    CHECK
    (((workspace_id = app.current_workspace_id()) AND ( SELECT app.has_workspace_role(deliverables.workspace_id, ARRAY['owner'::text, 'editor'::text]) AS has_workspace_role)));

CREATE POLICY tenant_member_select ON app.deliverables
  FOR SELECT
  USING
    (((workspace_id = app.current_workspace_id()) AND ( SELECT app.has_workspace_role(deliverables.workspace_id, ARRAY['owner'::text, 'editor'::text, 'viewer'::text]) AS
    has_workspace_role)));

ALTER TABLE app.domain_events
  FORCE ROW LEVEL SECURITY;

UPDATE app.domain_events
SET status = 'dead_letter'
WHERE status = 'failed';

ALTER TABLE app.domain_events
  ADD CONSTRAINT domain_events_status_valid CHECK (status = ANY (ARRAY['pending'::text, 'processing'::text, 'processed'::text, 'dead_letter'::text]));

ALTER TABLE app.domain_events
  ADD COLUMN attempt_count integer DEFAULT 0 NOT NULL;

ALTER TABLE app.domain_events
  ADD CONSTRAINT domain_events_attempt_count_nonnegative CHECK (attempt_count >= 0);

ALTER TABLE app.domain_events
  ADD COLUMN available_at timestamp with time zone DEFAULT now() NOT NULL;

ALTER TABLE app.domain_events
  ADD COLUMN locked_at timestamp with time zone;

ALTER TABLE app.domain_events
  ADD COLUMN locked_until timestamp with time zone;

ALTER TABLE app.domain_events
  ADD COLUMN locked_by text;

ALTER TABLE app.domain_events
  ADD CONSTRAINT domain_events_lease_consistent CHECK (status = 'processing'::text AND locked_at IS NOT NULL AND locked_until IS NOT NULL AND locked_by IS
    NOT NULL OR status <> 'processing'::text AND locked_at IS NULL AND locked_until IS NULL AND locked_by IS NULL);

ALTER TABLE app.domain_events
  ADD COLUMN last_error_code text;

ALTER TABLE app.domain_events
  ADD COLUMN last_error_message text;

ALTER TABLE app.domain_events
  ADD COLUMN failed_at timestamp with time zone;

UPDATE app.domain_events
SET attempt_count = greatest(attempt_count, 1),
    processed_at = NULL,
    failed_at = coalesce(failed_at, occurred_at),
    last_error_code = coalesce(last_error_code, 'legacy_failed_event'),
    last_error_message = coalesce(last_error_message, 'event failed before alpha outbox migration')
WHERE status = 'dead_letter';

UPDATE app.domain_events
SET processed_at = coalesce(processed_at, occurred_at),
    failed_at = NULL
WHERE status = 'processed';

UPDATE app.domain_events
SET processed_at = NULL,
    failed_at = NULL
WHERE status = 'pending';

ALTER TABLE app.domain_events
  ADD CONSTRAINT domain_events_completion_consistent CHECK (status = 'processed'::text AND processed_at IS
    NOT NULL AND failed_at IS NULL OR status = 'dead_letter'::text AND processed_at IS NULL AND failed_at IS
    NOT NULL OR (status = ANY (ARRAY['pending'::text, 'processing'::text])) AND processed_at IS NULL AND failed_at IS NULL);

GRANT INSERT, SELECT, UPDATE ON app.domain_events TO ai_center_runtime;

CREATE INDEX domain_events_pending_available_idx ON app.domain_events (available_at, occurred_at, id)
  WHERE status = 'pending'::text;

CREATE INDEX domain_events_processing_lease_idx ON app.domain_events (locked_until, id)
  WHERE status = 'processing'::text;

CREATE POLICY tenant_editor_insert ON app.domain_events
  FOR INSERT
  WITH
    CHECK
    (((workspace_id = app.current_workspace_id()) AND ( SELECT app.has_workspace_role(domain_events.workspace_id, ARRAY['owner'::text, 'editor'::text]) AS has_workspace_role)));

CREATE POLICY tenant_editor_update ON app.domain_events
  FOR UPDATE
  USING (((workspace_id = app.current_workspace_id()) AND ( SELECT app.has_workspace_role(domain_events.workspace_id, ARRAY['owner'::text, 'editor'::text]) AS has_workspace_role)))
  WITH
    CHECK
    (((workspace_id = app.current_workspace_id()) AND ( SELECT app.has_workspace_role(domain_events.workspace_id, ARRAY['owner'::text, 'editor'::text]) AS has_workspace_role)));

CREATE POLICY tenant_member_select ON app.domain_events
  FOR SELECT
  USING
    (((workspace_id = app.current_workspace_id()) AND ( SELECT app.has_workspace_role(domain_events.workspace_id, ARRAY['owner'::text, 'editor'::text, 'viewer'::text]) AS
    has_workspace_role)));

ALTER TABLE app.edges
  FORCE ROW LEVEL SECURITY;

ALTER TABLE app.edges
  ADD CONSTRAINT edges_type_valid
    CHECK
    (edge_type = ANY (ARRAY['references'::text, 'depends_on'::text, 'informs'::text, 'supersedes'::text, 'contradicts'::text, 'derived_from'::text, 'satisfies'::text,
    'evidenced_by'::text, 'implemented_by'::text, 'tracked_by'::text]));

GRANT INSERT, SELECT ON app.edges TO ai_center_runtime;

CREATE POLICY tenant_editor_insert ON app.edges
  FOR INSERT
  WITH CHECK (((workspace_id = app.current_workspace_id()) AND ( SELECT app.has_workspace_role(edges.workspace_id, ARRAY['owner'::text, 'editor'::text]) AS has_workspace_role)));

CREATE POLICY tenant_editor_update ON app.edges
  FOR UPDATE
  USING (((workspace_id = app.current_workspace_id()) AND ( SELECT app.has_workspace_role(edges.workspace_id, ARRAY['owner'::text, 'editor'::text]) AS has_workspace_role)))
  WITH CHECK (((workspace_id = app.current_workspace_id()) AND ( SELECT app.has_workspace_role(edges.workspace_id, ARRAY['owner'::text, 'editor'::text]) AS has_workspace_role)));

CREATE POLICY tenant_member_select ON app.edges
  FOR SELECT
  USING
    (((workspace_id = app.current_workspace_id()) AND ( SELECT app.has_workspace_role(edges.workspace_id, ARRAY['owner'::text, 'editor'::text, 'viewer'::text]) AS
    has_workspace_role)));

ALTER TABLE app.evidences
  FORCE ROW LEVEL SECURITY;

ALTER TABLE app.evidences
  ADD CONSTRAINT evidences_artifact_project_fkey FOREIGN KEY (artifact_id, project_id) REFERENCES app.artifacts(id, project_id);

ALTER TABLE app.evidences
  ADD CONSTRAINT evidences_deliverable_project_fkey FOREIGN KEY (deliverable_id, project_id) REFERENCES app.deliverables(id, project_id);

ALTER TABLE app.evidences
  ADD CONSTRAINT evidences_id_project_unique UNIQUE (id, project_id);

ALTER TABLE app.evidences
  ADD CONSTRAINT evidences_section_project_fkey FOREIGN KEY (deliverable_section_id, project_id) REFERENCES app.deliverable_sections(id, project_id);

ALTER TABLE app.evidences
  ADD CONSTRAINT evidences_status_valid CHECK (status = ANY (ARRAY['candidate'::text, 'valid'::text, 'stale'::text, 'unavailable'::text, 'rejected'::text]));

ALTER TABLE app.evidences
  ADD COLUMN external_reference_id bigint;

GRANT INSERT, SELECT, UPDATE ON app.evidences TO ai_center_runtime;

CREATE INDEX evidences_external_reference_id_idx ON app.evidences (external_reference_id)
  WHERE external_reference_id IS NOT NULL;

CREATE POLICY tenant_editor_insert ON app.evidences
  FOR INSERT
  WITH
    CHECK (((workspace_id = app.current_workspace_id()) AND ( SELECT app.has_workspace_role(evidences.workspace_id, ARRAY['owner'::text, 'editor'::text]) AS has_workspace_role)));

CREATE POLICY tenant_editor_update ON app.evidences
  FOR UPDATE
  USING (((workspace_id = app.current_workspace_id()) AND ( SELECT app.has_workspace_role(evidences.workspace_id, ARRAY['owner'::text, 'editor'::text]) AS has_workspace_role)))
  WITH
    CHECK (((workspace_id = app.current_workspace_id()) AND ( SELECT app.has_workspace_role(evidences.workspace_id, ARRAY['owner'::text, 'editor'::text]) AS has_workspace_role)));

CREATE POLICY tenant_member_select ON app.evidences
  FOR SELECT
  USING
    (((workspace_id = app.current_workspace_id()) AND ( SELECT app.has_workspace_role(evidences.workspace_id, ARRAY['owner'::text, 'editor'::text, 'viewer'::text]) AS
    has_workspace_role)));

ALTER TABLE app.execution_events
  FORCE ROW LEVEL SECURITY;

ALTER TABLE app.execution_events
  ADD COLUMN workspace_id bigint;

ALTER TABLE app.execution_events
  ADD CONSTRAINT execution_events_workspace_id_fkey FOREIGN KEY (workspace_id) REFERENCES app.workspaces(id) ON DELETE CASCADE;

ALTER TABLE app.execution_events
  ADD COLUMN project_id bigint;

UPDATE app.execution_events event
SET workspace_id = execution.workspace_id,
    project_id = execution.project_id
FROM app.executions execution
WHERE execution.id = event.execution_id;

ALTER TABLE app.execution_events
  ALTER COLUMN workspace_id SET NOT NULL,
  ALTER COLUMN project_id SET NOT NULL;

ALTER TABLE app.execution_events
  ADD CONSTRAINT execution_events_project_id_fkey FOREIGN KEY (project_id) REFERENCES app.projects(id) ON DELETE CASCADE;

CREATE INDEX execution_events_workspace_id_idx ON app.execution_events (workspace_id);

CREATE INDEX execution_events_project_id_idx ON app.execution_events (project_id);

CREATE TRIGGER execution_events_prevent_update
  BEFORE DELETE OR UPDATE ON app.execution_events
  FOR EACH ROW
  EXECUTE FUNCTION app.prevent_append_only_mutation();

CREATE POLICY tenant_editor_insert ON app.execution_events
  FOR INSERT
  WITH
    CHECK
    (((workspace_id = app.current_workspace_id()) AND ( SELECT app.has_workspace_role(execution_events.workspace_id, ARRAY['owner'::text, 'editor'::text]) AS has_workspace_role)));

CREATE POLICY tenant_member_select ON app.execution_events
  FOR SELECT
  USING
    (((workspace_id = app.current_workspace_id()) AND ( SELECT app.has_workspace_role(execution_events.workspace_id, ARRAY['owner'::text, 'editor'::text, 'viewer'::text]) AS
    has_workspace_role)));

ALTER TABLE app.executions
  FORCE ROW LEVEL SECURITY;

ALTER TABLE app.executions
  ADD CONSTRAINT executions_id_project_unique UNIQUE (id, project_id);

ALTER TABLE app.artifacts
  ADD CONSTRAINT artifacts_execution_project_fkey FOREIGN KEY (execution_id, project_id) REFERENCES app.executions(id, project_id);

ALTER TABLE app.execution_events
  ADD CONSTRAINT execution_events_execution_project_fkey FOREIGN KEY (execution_id, project_id) REFERENCES app.executions(id, project_id);

CREATE POLICY tenant_editor_insert ON app.executions
  FOR INSERT
  WITH
    CHECK (((workspace_id = app.current_workspace_id()) AND ( SELECT app.has_workspace_role(executions.workspace_id, ARRAY['owner'::text, 'editor'::text]) AS has_workspace_role)));

CREATE POLICY tenant_editor_update ON app.executions
  FOR UPDATE
  USING (((workspace_id = app.current_workspace_id()) AND ( SELECT app.has_workspace_role(executions.workspace_id, ARRAY['owner'::text, 'editor'::text]) AS has_workspace_role)))
  WITH
    CHECK (((workspace_id = app.current_workspace_id()) AND ( SELECT app.has_workspace_role(executions.workspace_id, ARRAY['owner'::text, 'editor'::text]) AS has_workspace_role)));

CREATE POLICY tenant_member_select ON app.executions
  FOR SELECT
  USING
    (((workspace_id = app.current_workspace_id()) AND ( SELECT app.has_workspace_role(executions.workspace_id, ARRAY['owner'::text, 'editor'::text, 'viewer'::text]) AS
    has_workspace_role)));

CREATE TABLE app.external_reference_observations (
  id                    bigint                   GENERATED ALWAYS AS IDENTITY NOT NULL,
  public_id             uuid                     DEFAULT gen_random_uuid() NOT NULL,
  workspace_id          bigint                   NOT NULL,
  project_id            bigint                   NOT NULL,
  external_reference_id bigint                   NOT NULL,
  observation_status    text                     NOT NULL,
  content_hash          text                     NOT NULL,
  etag                  text,
  observed_state        jsonb                    DEFAULT '{}'::jsonb NOT NULL,
  provider_updated_at   timestamp with time zone,
  observed_at           timestamp with time zone DEFAULT now() NOT NULL
);

ALTER TABLE app.external_reference_observations
  ENABLE ROW LEVEL SECURITY;

ALTER TABLE app.external_reference_observations
  FORCE ROW LEVEL SECURITY;

ALTER TABLE app.external_reference_observations
  ADD CONSTRAINT external_reference_observations_content_hash_valid CHECK (content_hash ~ '^[0-9a-f]{64}$'::text);

ALTER TABLE app.external_reference_observations
  ADD CONSTRAINT external_reference_observations_id_project_unique UNIQUE (id, project_id);

ALTER TABLE app.context_pack_selection_items
  ADD CONSTRAINT context_pack_selection_items_observation_project_fkey FOREIGN KEY (external_reference_observation_id, project_id)
    REFERENCES app.external_reference_observations(id, project_id);

ALTER TABLE app.deliverable_sources
  ADD CONSTRAINT deliverable_sources_observation_project_fkey FOREIGN KEY (external_reference_observation_id, project_id)
    REFERENCES app.external_reference_observations(id, project_id);

ALTER TABLE app.external_reference_observations
  ADD CONSTRAINT external_reference_observations_pkey PRIMARY KEY (id);

ALTER TABLE app.context_pack_selection_items
  ADD CONSTRAINT context_pack_selection_items_external_reference_observatio_fkey FOREIGN KEY (external_reference_observation_id) REFERENCES app.external_reference_observations(id);

ALTER TABLE app.deliverable_sources
  ADD CONSTRAINT deliverable_sources_external_reference_observation_id_fkey FOREIGN KEY (external_reference_observation_id) REFERENCES app.external_reference_observations(id);

ALTER TABLE app.external_reference_observations
  ADD CONSTRAINT external_reference_observations_project_id_fkey FOREIGN KEY (project_id) REFERENCES app.projects(id) ON DELETE CASCADE;

ALTER TABLE app.external_reference_observations
  ADD CONSTRAINT external_reference_observations_public_id_key UNIQUE (public_id);

ALTER TABLE app.external_reference_observations
  ADD CONSTRAINT external_reference_observations_reference_hash_unique UNIQUE (external_reference_id, content_hash);

ALTER TABLE app.external_reference_observations
  ADD CONSTRAINT external_reference_observations_state_is_object CHECK (jsonb_typeof(observed_state) = 'object'::text);

ALTER TABLE app.external_reference_observations
  ADD CONSTRAINT external_reference_observations_status_valid CHECK (observation_status = ANY (ARRAY['current'::text, 'stale'::text, 'unavailable'::text]));

ALTER TABLE app.external_reference_observations
  ADD CONSTRAINT external_reference_observations_workspace_id_fkey FOREIGN KEY (workspace_id) REFERENCES app.workspaces(id) ON DELETE CASCADE;

GRANT INSERT, SELECT ON app.external_reference_observations TO ai_center_runtime;

CREATE INDEX external_reference_observations_reference_observed_idx ON app.external_reference_observations (external_reference_id, observed_at DESC);

CREATE INDEX external_reference_observations_workspace_id_idx ON app.external_reference_observations (workspace_id);

CREATE INDEX external_reference_observations_project_id_idx ON app.external_reference_observations (project_id);

CREATE TRIGGER external_reference_observations_prevent_update
  BEFORE DELETE OR UPDATE ON app.external_reference_observations
  FOR EACH ROW
  EXECUTE FUNCTION app.prevent_append_only_mutation();

CREATE POLICY tenant_editor_insert ON app.external_reference_observations
  FOR INSERT
  WITH
    CHECK
    (((workspace_id = app.current_workspace_id()) AND ( SELECT app.has_workspace_role(external_reference_observations.workspace_id, ARRAY['owner'::text, 'editor'::text]) AS
    has_workspace_role)));

CREATE POLICY tenant_member_select ON app.external_reference_observations
  FOR SELECT
  USING
    (((workspace_id = app.current_workspace_id()) AND ( SELECT app.has_workspace_role(external_reference_observations.workspace_id, ARRAY['owner'::text, 'editor'::text,
    'viewer'::text]) AS has_workspace_role)));

CREATE TABLE app.external_references (
  id                   bigint                   GENERATED ALWAYS AS IDENTITY NOT NULL,
  public_id            uuid                     DEFAULT gen_random_uuid() NOT NULL,
  workspace_id         bigint                   NOT NULL,
  project_id           bigint                   NOT NULL,
  tool_connection_id   bigint                   NOT NULL,
  provider             text                     NOT NULL,
  object_kind          text                     NOT NULL,
  external_id          text                     NOT NULL,
  canonical_url        text                     NOT NULL,
  repository_full_name text                     NOT NULL,
  display_title        text                     NOT NULL,
  sync_status          text                     DEFAULT 'pending'::text NOT NULL,
  etag                 text,
  last_synced_at       timestamp with time zone,
  last_error_code      text,
  last_error_message   text,
  created_by_actor_id  uuid                     NOT NULL,
  created_at           timestamp with time zone DEFAULT now() NOT NULL,
  updated_at           timestamp with time zone DEFAULT now() NOT NULL
);

ALTER TABLE app.external_references
  ENABLE ROW LEVEL SECURITY;

ALTER TABLE app.external_references
  FORCE ROW LEVEL SECURITY;

ALTER TABLE app.external_references
  ADD CONSTRAINT external_references_canonical_url_github CHECK (canonical_url ~ '^https://github\.com/[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+(?:/.*)?$'::text);

ALTER TABLE app.external_references
  ADD CONSTRAINT external_references_display_title_not_blank CHECK (btrim(display_title) <> ''::text);

ALTER TABLE app.external_references
  ADD CONSTRAINT external_references_external_id_not_blank CHECK (btrim(external_id) <> ''::text);

ALTER TABLE app.external_references
  ADD CONSTRAINT external_references_id_project_unique UNIQUE (id, project_id);

ALTER TABLE app.artifacts
  ADD CONSTRAINT artifacts_external_reference_project_fkey FOREIGN KEY (external_reference_id, project_id) REFERENCES app.external_references(id, project_id);

ALTER TABLE app.evidences
  ADD CONSTRAINT evidences_external_reference_project_fkey FOREIGN KEY (external_reference_id, project_id) REFERENCES app.external_references(id, project_id);

ALTER TABLE app.external_reference_observations
  ADD CONSTRAINT external_reference_observations_reference_project_fkey FOREIGN KEY (external_reference_id, project_id) REFERENCES app.external_references(id, project_id);

ALTER TABLE app.external_references
  ADD CONSTRAINT external_references_object_kind_valid CHECK (object_kind = ANY (ARRAY['repository'::text, 'pull_request'::text, 'commit'::text, 'check_run'::text]));

ALTER TABLE app.external_references
  ADD CONSTRAINT external_references_pkey PRIMARY KEY (id);

ALTER TABLE app.artifacts
  ADD CONSTRAINT artifacts_external_reference_id_fkey FOREIGN KEY (external_reference_id) REFERENCES app.external_references(id) ON DELETE SET NULL;

ALTER TABLE app.evidences
  ADD CONSTRAINT evidences_external_reference_id_fkey FOREIGN KEY (external_reference_id) REFERENCES app.external_references(id) ON DELETE SET NULL;

ALTER TABLE app.external_reference_observations
  ADD CONSTRAINT external_reference_observations_external_reference_id_fkey FOREIGN KEY (external_reference_id) REFERENCES app.external_references(id) ON DELETE CASCADE;

ALTER TABLE app.external_references
  ADD CONSTRAINT external_references_project_id_fkey FOREIGN KEY (project_id) REFERENCES app.projects(id) ON DELETE CASCADE;

ALTER TABLE app.external_references
  ADD CONSTRAINT external_references_project_provider_external_unique UNIQUE (project_id, PROVIDER, external_id);

ALTER TABLE app.external_references
  ADD CONSTRAINT external_references_provider_valid CHECK (provider = 'github'::text);

ALTER TABLE app.external_references
  ADD CONSTRAINT external_references_public_id_key UNIQUE (public_id);

ALTER TABLE app.external_references
  ADD CONSTRAINT external_references_repository_full_name_valid CHECK (repository_full_name ~ '^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+$'::text);

ALTER TABLE app.external_references
  ADD CONSTRAINT external_references_sync_status_valid CHECK (sync_status = ANY (ARRAY['pending'::text, 'current'::text, 'stale'::text, 'unavailable'::text, 'error'::text]));

ALTER TABLE app.external_references
  ADD CONSTRAINT external_references_workspace_id_fkey FOREIGN KEY (workspace_id) REFERENCES app.workspaces(id) ON DELETE CASCADE;

GRANT INSERT, SELECT, UPDATE ON app.external_references TO ai_center_runtime;

CREATE INDEX external_references_workspace_id_idx ON app.external_references (workspace_id);

CREATE INDEX external_references_project_status_updated_idx ON app.external_references (project_id, sync_status, updated_at DESC);

CREATE INDEX external_references_tool_connection_id_idx ON app.external_references (tool_connection_id);

CREATE TRIGGER external_references_set_updated_at
  BEFORE UPDATE ON app.external_references
  FOR EACH ROW
  EXECUTE FUNCTION app.set_updated_at();

CREATE POLICY tenant_editor_insert ON app.external_references
  FOR INSERT
  WITH
    CHECK
    (((workspace_id = app.current_workspace_id()) AND ( SELECT app.has_workspace_role(external_references.workspace_id, ARRAY['owner'::text, 'editor'::text]) AS
    has_workspace_role)));

CREATE POLICY tenant_editor_update ON app.external_references
  FOR UPDATE
  USING
    (((workspace_id = app.current_workspace_id()) AND ( SELECT app.has_workspace_role(external_references.workspace_id, ARRAY['owner'::text, 'editor'::text]) AS
    has_workspace_role)))
  WITH
    CHECK
    (((workspace_id = app.current_workspace_id()) AND ( SELECT app.has_workspace_role(external_references.workspace_id, ARRAY['owner'::text, 'editor'::text]) AS
    has_workspace_role)));

CREATE POLICY tenant_member_select ON app.external_references
  FOR SELECT
  USING
    (((workspace_id = app.current_workspace_id()) AND ( SELECT app.has_workspace_role(external_references.workspace_id, ARRAY['owner'::text, 'editor'::text, 'viewer'::text]) AS
    has_workspace_role)));

ALTER TABLE app.gates
  FORCE ROW LEVEL SECURITY;

ALTER TABLE app.gates
  ADD CONSTRAINT gates_context_node_project_fkey FOREIGN KEY (context_node_id, project_id) REFERENCES app.context_nodes(id, project_id);

GRANT INSERT, SELECT, UPDATE ON app.gates TO ai_center_runtime;

CREATE POLICY tenant_editor_insert ON app.gates
  FOR INSERT
  WITH CHECK (((workspace_id = app.current_workspace_id()) AND ( SELECT app.has_workspace_role(gates.workspace_id, ARRAY['owner'::text, 'editor'::text]) AS has_workspace_role)));

CREATE POLICY tenant_editor_update ON app.gates
  FOR UPDATE
  USING (((workspace_id = app.current_workspace_id()) AND ( SELECT app.has_workspace_role(gates.workspace_id, ARRAY['owner'::text, 'editor'::text]) AS has_workspace_role)))
  WITH CHECK (((workspace_id = app.current_workspace_id()) AND ( SELECT app.has_workspace_role(gates.workspace_id, ARRAY['owner'::text, 'editor'::text]) AS has_workspace_role)));

CREATE POLICY tenant_member_select ON app.gates
  FOR SELECT
  USING
    (((workspace_id = app.current_workspace_id()) AND ( SELECT app.has_workspace_role(gates.workspace_id, ARRAY['owner'::text, 'editor'::text, 'viewer'::text]) AS
    has_workspace_role)));

ALTER TABLE app.handoffs
  FORCE ROW LEVEL SECURITY;

ALTER TABLE app.handoffs
  ADD CONSTRAINT handoffs_context_pack_project_fkey FOREIGN KEY (context_pack_id, project_id) REFERENCES app.context_packs(id, project_id);

GRANT INSERT, SELECT ON app.handoffs TO ai_center_runtime;

CREATE POLICY tenant_editor_insert ON app.handoffs
  FOR INSERT
  WITH CHECK (((workspace_id = app.current_workspace_id()) AND ( SELECT app.has_workspace_role(handoffs.workspace_id, ARRAY['owner'::text, 'editor'::text]) AS has_workspace_role)));

CREATE POLICY tenant_editor_update ON app.handoffs
  FOR UPDATE
  USING (((workspace_id = app.current_workspace_id()) AND ( SELECT app.has_workspace_role(handoffs.workspace_id, ARRAY['owner'::text, 'editor'::text]) AS has_workspace_role)))
  WITH CHECK (((workspace_id = app.current_workspace_id()) AND ( SELECT app.has_workspace_role(handoffs.workspace_id, ARRAY['owner'::text, 'editor'::text]) AS has_workspace_role)));

CREATE POLICY tenant_member_select ON app.handoffs
  FOR SELECT
  USING
    (((workspace_id = app.current_workspace_id()) AND ( SELECT app.has_workspace_role(handoffs.workspace_id, ARRAY['owner'::text, 'editor'::text, 'viewer'::text]) AS
    has_workspace_role)));

CREATE TABLE app.idempotency_records (
  id              bigint                   GENERATED ALWAYS AS IDENTITY NOT NULL,
  public_id       uuid                     DEFAULT gen_random_uuid() NOT NULL,
  workspace_id    bigint                   NOT NULL,
  project_id      bigint,
  actor_id        uuid                     NOT NULL,
  operation_key   text                     NOT NULL,
  idempotency_key text                     NOT NULL,
  request_hash    text                     NOT NULL,
  status          text                     DEFAULT 'processing'::text NOT NULL,
  response_status smallint,
  response_body   jsonb,
  error_code      text,
  locked_until    timestamp with time zone DEFAULT (now() + '00:00:30'::interval),
  expires_at      timestamp with time zone DEFAULT (now() + '24:00:00'::interval) NOT NULL,
  created_at      timestamp with time zone DEFAULT now() NOT NULL,
  updated_at      timestamp with time zone DEFAULT now() NOT NULL
);

ALTER TABLE app.idempotency_records
  ENABLE ROW LEVEL SECURITY;

ALTER TABLE app.idempotency_records
  FORCE ROW LEVEL SECURITY;

ALTER TABLE app.idempotency_records
  ADD CONSTRAINT idempotency_records_expiry_valid CHECK (expires_at > created_at);

ALTER TABLE app.idempotency_records
  ADD CONSTRAINT idempotency_records_key_not_blank CHECK (btrim(idempotency_key) <> ''::text);

ALTER TABLE app.idempotency_records
  ADD CONSTRAINT idempotency_records_operation_not_blank CHECK (btrim(operation_key) <> ''::text);

ALTER TABLE app.idempotency_records
  ADD CONSTRAINT idempotency_records_pkey PRIMARY KEY (id);

ALTER TABLE app.idempotency_records
  ADD CONSTRAINT idempotency_records_project_id_fkey FOREIGN KEY (project_id) REFERENCES app.projects(id) ON DELETE CASCADE;

ALTER TABLE app.idempotency_records
  ADD CONSTRAINT idempotency_records_public_id_key UNIQUE (public_id);

ALTER TABLE app.idempotency_records
  ADD CONSTRAINT idempotency_records_request_hash_valid CHECK (request_hash ~ '^[0-9a-f]{64}$'::text);

ALTER TABLE app.idempotency_records
  ADD CONSTRAINT idempotency_records_response_status_valid CHECK (response_status IS NULL OR response_status >= 100 AND response_status <= 599);

ALTER TABLE app.idempotency_records
  ADD CONSTRAINT idempotency_records_result_consistent CHECK (status = 'processing'::text AND response_status IS NULL AND error_code IS NULL AND locked_until IS
    NOT NULL OR status = 'completed'::text AND response_status IS NOT NULL AND error_code IS NULL AND locked_until IS NULL OR status = 'failed'::text AND error_code IS
    NOT NULL AND locked_until IS NULL);

ALTER TABLE app.idempotency_records
  ADD CONSTRAINT idempotency_records_scope_key_unique UNIQUE (workspace_id, actor_id, operation_key, idempotency_key);

ALTER TABLE app.idempotency_records
  ADD CONSTRAINT idempotency_records_status_valid CHECK (status = ANY (ARRAY['processing'::text, 'completed'::text, 'failed'::text]));

ALTER TABLE app.idempotency_records
  ADD CONSTRAINT idempotency_records_workspace_id_fkey FOREIGN KEY (workspace_id) REFERENCES app.workspaces(id) ON DELETE CASCADE;

GRANT INSERT, SELECT, UPDATE ON app.idempotency_records TO ai_center_runtime;

CREATE INDEX idempotency_records_reclaimable_idx ON app.idempotency_records (locked_until, created_at)
  WHERE status = 'processing'::text;

CREATE INDEX idempotency_records_project_id_idx ON app.idempotency_records (project_id)
  WHERE project_id IS NOT NULL;

CREATE INDEX idempotency_records_workspace_expiry_idx ON app.idempotency_records (workspace_id, expires_at);

CREATE TRIGGER idempotency_records_set_updated_at
  BEFORE UPDATE ON app.idempotency_records
  FOR EACH ROW
  EXECUTE FUNCTION app.set_updated_at();

CREATE POLICY tenant_editor_insert ON app.idempotency_records
  FOR INSERT
  WITH
    CHECK
    (((workspace_id = app.current_workspace_id()) AND ( SELECT app.has_workspace_role(idempotency_records.workspace_id, ARRAY['owner'::text, 'editor'::text]) AS
    has_workspace_role)));

CREATE POLICY tenant_editor_update ON app.idempotency_records
  FOR UPDATE
  USING
    (((workspace_id = app.current_workspace_id()) AND ( SELECT app.has_workspace_role(idempotency_records.workspace_id, ARRAY['owner'::text, 'editor'::text]) AS
    has_workspace_role)))
  WITH
    CHECK
    (((workspace_id = app.current_workspace_id()) AND ( SELECT app.has_workspace_role(idempotency_records.workspace_id, ARRAY['owner'::text, 'editor'::text]) AS
    has_workspace_role)));

CREATE POLICY tenant_member_select ON app.idempotency_records
  FOR SELECT
  USING
    (((workspace_id = app.current_workspace_id()) AND ( SELECT app.has_workspace_role(idempotency_records.workspace_id, ARRAY['owner'::text, 'editor'::text, 'viewer'::text]) AS
    has_workspace_role)));

CREATE TABLE app.insight_resolutions (
  id                         bigint                   GENERATED ALWAYS AS IDENTITY NOT NULL,
  public_id                  uuid                     DEFAULT gen_random_uuid() NOT NULL,
  workspace_id               bigint                   NOT NULL,
  project_id                 bigint                   NOT NULL,
  insight_id                 bigint                   NOT NULL,
  action                     text                     NOT NULL,
  knowledge_entry_version_id bigint,
  source_graph_version       bigint                   NOT NULL,
  resulting_graph_version    bigint                   NOT NULL,
  justification              text                     NOT NULL,
  resolved_by_actor_id       uuid                     NOT NULL,
  created_at                 timestamp with time zone DEFAULT now() NOT NULL
);

ALTER TABLE app.insight_resolutions
  ENABLE ROW LEVEL SECURITY;

ALTER TABLE app.insight_resolutions
  FORCE ROW LEVEL SECURITY;

ALTER TABLE app.insight_resolutions
  ADD CONSTRAINT insight_resolutions_action_valid CHECK (action = ANY (ARRAY['accept'::text, 'dismiss'::text, 'resolve'::text]));

ALTER TABLE app.insight_resolutions
  ADD CONSTRAINT insight_resolutions_graph_versions_valid CHECK (source_graph_version >= 0 AND resulting_graph_version >= source_graph_version);

ALTER TABLE app.insight_resolutions
  ADD CONSTRAINT insight_resolutions_insight_id_fkey FOREIGN KEY (insight_id) REFERENCES app.insights(id) ON DELETE CASCADE;

ALTER TABLE app.insight_resolutions
  ADD CONSTRAINT insight_resolutions_insight_unique UNIQUE (insight_id);

ALTER TABLE app.insight_resolutions
  ADD CONSTRAINT insight_resolutions_justification_not_blank CHECK (btrim(justification) <> ''::text);

ALTER TABLE app.insight_resolutions
  ADD CONSTRAINT insight_resolutions_knowledge_entry_version_id_fkey FOREIGN KEY (knowledge_entry_version_id) REFERENCES app.knowledge_entry_versions(id);

ALTER TABLE app.insight_resolutions
  ADD CONSTRAINT insight_resolutions_mutation_consistent CHECK (action = 'resolve'::text AND knowledge_entry_version_id IS
    NOT NULL AND resulting_graph_version > source_graph_version OR (action = ANY (ARRAY['accept'::text, 'dismiss'::text])) AND knowledge_entry_version_id IS NULL AND
    resulting_graph_version = source_graph_version);

ALTER TABLE app.insight_resolutions
  ADD CONSTRAINT insight_resolutions_pkey PRIMARY KEY (id);

ALTER TABLE app.insight_resolutions
  ADD CONSTRAINT insight_resolutions_project_id_fkey FOREIGN KEY (project_id) REFERENCES app.projects(id) ON DELETE CASCADE;

ALTER TABLE app.insight_resolutions
  ADD CONSTRAINT insight_resolutions_public_id_key UNIQUE (public_id);

ALTER TABLE app.insight_resolutions
  ADD CONSTRAINT insight_resolutions_workspace_id_fkey FOREIGN KEY (workspace_id) REFERENCES app.workspaces(id) ON DELETE CASCADE;

GRANT INSERT, SELECT ON app.insight_resolutions TO ai_center_runtime;

CREATE INDEX insight_resolutions_workspace_id_idx ON app.insight_resolutions (workspace_id);

CREATE INDEX insight_resolutions_project_id_idx ON app.insight_resolutions (project_id);

CREATE INDEX insight_resolutions_knowledge_version_id_idx ON app.insight_resolutions (knowledge_entry_version_id)
  WHERE knowledge_entry_version_id IS NOT NULL;

CREATE TRIGGER insight_resolutions_prevent_update
  BEFORE DELETE OR UPDATE ON app.insight_resolutions
  FOR EACH ROW
  EXECUTE FUNCTION app.prevent_append_only_mutation();

CREATE POLICY tenant_editor_insert ON app.insight_resolutions
  FOR INSERT
  WITH
    CHECK
    (((workspace_id = app.current_workspace_id()) AND ( SELECT app.has_workspace_role(insight_resolutions.workspace_id, ARRAY['owner'::text, 'editor'::text]) AS
    has_workspace_role)));

CREATE POLICY tenant_member_select ON app.insight_resolutions
  FOR SELECT
  USING
    (((workspace_id = app.current_workspace_id()) AND ( SELECT app.has_workspace_role(insight_resolutions.workspace_id, ARRAY['owner'::text, 'editor'::text, 'viewer'::text]) AS
    has_workspace_role)));

ALTER TABLE app.insight_sources
  FORCE ROW LEVEL SECURITY;

ALTER TABLE app.insight_sources
  ADD COLUMN workspace_id bigint;

ALTER TABLE app.insight_sources
  ADD CONSTRAINT insight_sources_workspace_id_fkey FOREIGN KEY (workspace_id) REFERENCES app.workspaces(id) ON DELETE CASCADE;

ALTER TABLE app.insight_sources
  ADD COLUMN project_id bigint;

UPDATE app.insight_sources source
SET workspace_id = insight.workspace_id,
    project_id = insight.project_id
FROM app.insights insight
WHERE insight.id = source.insight_id;

ALTER TABLE app.insight_sources
  ALTER COLUMN workspace_id SET NOT NULL,
  ALTER COLUMN project_id SET NOT NULL;

ALTER TABLE app.insight_sources
  ADD CONSTRAINT insight_sources_project_id_fkey FOREIGN KEY (project_id) REFERENCES app.projects(id) ON DELETE CASCADE;

GRANT INSERT, SELECT ON app.insight_sources TO ai_center_runtime;

CREATE INDEX insight_sources_project_id_idx ON app.insight_sources (project_id);

CREATE INDEX insight_sources_workspace_id_idx ON app.insight_sources (workspace_id);

CREATE TRIGGER insight_sources_prevent_update
  BEFORE DELETE OR UPDATE ON app.insight_sources
  FOR EACH ROW
  EXECUTE FUNCTION app.prevent_append_only_mutation();

CREATE POLICY tenant_editor_insert ON app.insight_sources
  FOR INSERT
  WITH
    CHECK
    (((workspace_id = app.current_workspace_id()) AND ( SELECT app.has_workspace_role(insight_sources.workspace_id, ARRAY['owner'::text, 'editor'::text]) AS has_workspace_role)));

CREATE POLICY tenant_member_select ON app.insight_sources
  FOR SELECT
  USING
    (((workspace_id = app.current_workspace_id()) AND ( SELECT app.has_workspace_role(insight_sources.workspace_id, ARRAY['owner'::text, 'editor'::text, 'viewer'::text]) AS
    has_workspace_role)));

ALTER TABLE app.insights
  FORCE ROW LEVEL SECURITY;

ALTER TABLE app.insights
  ADD CONSTRAINT insights_id_project_unique UNIQUE (id, project_id);

ALTER TABLE app.insight_resolutions
  ADD CONSTRAINT insight_resolutions_insight_project_fkey FOREIGN KEY (insight_id, project_id) REFERENCES app.insights(id, project_id);

ALTER TABLE app.insight_sources
  ADD CONSTRAINT insight_sources_insight_project_fkey FOREIGN KEY (insight_id, project_id) REFERENCES app.insights(id, project_id);

ALTER TABLE app.insights
  ADD COLUMN steward_assessment_id bigint;

ALTER TABLE app.insights
  ADD COLUMN resolved_at timestamp with time zone;

UPDATE app.insights
SET resolved_at = coalesce(resolved_at, updated_at, detected_at)
WHERE status IN ('accepted', 'resolved', 'dismissed');

ALTER TABLE app.insights
  ADD CONSTRAINT insights_resolution_consistent
    CHECK
    ((status = ANY (ARRAY['candidate'::text, 'open'::text])) AND resolution_justification IS NULL AND resolved_by_actor_id IS NULL AND resolved_at IS NULL OR (status = ANY
    (ARRAY['accepted'::text, 'resolved'::text, 'dismissed'::text])) AND resolution_justification IS NOT NULL AND resolved_by_actor_id IS NOT NULL AND resolved_at IS NOT NULL);

GRANT INSERT, SELECT, UPDATE ON app.insights TO ai_center_runtime;

CREATE UNIQUE INDEX insights_steward_assessment_id_idx ON app.insights (steward_assessment_id)
  WHERE steward_assessment_id IS NOT NULL;

CREATE POLICY tenant_editor_insert ON app.insights
  FOR INSERT
  WITH CHECK (((workspace_id = app.current_workspace_id()) AND ( SELECT app.has_workspace_role(insights.workspace_id, ARRAY['owner'::text, 'editor'::text]) AS has_workspace_role)));

CREATE POLICY tenant_editor_update ON app.insights
  FOR UPDATE
  USING (((workspace_id = app.current_workspace_id()) AND ( SELECT app.has_workspace_role(insights.workspace_id, ARRAY['owner'::text, 'editor'::text]) AS has_workspace_role)))
  WITH CHECK (((workspace_id = app.current_workspace_id()) AND ( SELECT app.has_workspace_role(insights.workspace_id, ARRAY['owner'::text, 'editor'::text]) AS has_workspace_role)));

CREATE POLICY tenant_member_select ON app.insights
  FOR SELECT
  USING
    (((workspace_id = app.current_workspace_id()) AND ( SELECT app.has_workspace_role(insights.workspace_id, ARRAY['owner'::text, 'editor'::text, 'viewer'::text]) AS
    has_workspace_role)));

ALTER TABLE app.knowledge_entries
  FORCE ROW LEVEL SECURITY;

ALTER TABLE app.knowledge_entries
  ADD CONSTRAINT knowledge_entries_context_node_project_fkey FOREIGN KEY (context_node_id, project_id) REFERENCES app.context_nodes(id, project_id);

ALTER TABLE app.knowledge_entries
  ADD CONSTRAINT knowledge_entries_id_project_unique UNIQUE (id, project_id);

ALTER TABLE app.context_pack_sources
  ADD CONSTRAINT context_pack_sources_entry_project_fkey FOREIGN KEY (knowledge_entry_id, project_id) REFERENCES app.knowledge_entries(id, project_id);

ALTER TABLE app.evidences
  ADD CONSTRAINT evidences_requirement_project_fkey FOREIGN KEY (requirement_entry_id, project_id) REFERENCES app.knowledge_entries(id, project_id);

GRANT INSERT, SELECT, UPDATE ON app.knowledge_entries TO ai_center_runtime;

CREATE POLICY tenant_editor_insert ON app.knowledge_entries
  FOR INSERT
  WITH
    CHECK
    (((workspace_id = app.current_workspace_id()) AND ( SELECT app.has_workspace_role(knowledge_entries.workspace_id, ARRAY['owner'::text, 'editor'::text]) AS has_workspace_role)));

CREATE POLICY tenant_editor_update ON app.knowledge_entries
  FOR UPDATE
  USING
    (((workspace_id = app.current_workspace_id()) AND ( SELECT app.has_workspace_role(knowledge_entries.workspace_id, ARRAY['owner'::text, 'editor'::text]) AS has_workspace_role)))
  WITH
    CHECK
    (((workspace_id = app.current_workspace_id()) AND ( SELECT app.has_workspace_role(knowledge_entries.workspace_id, ARRAY['owner'::text, 'editor'::text]) AS has_workspace_role)));

CREATE POLICY tenant_member_select ON app.knowledge_entries
  FOR SELECT
  USING
    (((workspace_id = app.current_workspace_id()) AND ( SELECT app.has_workspace_role(knowledge_entries.workspace_id, ARRAY['owner'::text, 'editor'::text, 'viewer'::text]) AS
    has_workspace_role)));

ALTER TABLE app.knowledge_entry_versions
  FORCE ROW LEVEL SECURITY;

ALTER TABLE app.knowledge_entry_versions
  ADD CONSTRAINT knowledge_entry_versions_context_node_project_fkey FOREIGN KEY (context_node_id, project_id) REFERENCES app.context_nodes(id, project_id);

ALTER TABLE app.knowledge_entry_versions
  ADD CONSTRAINT knowledge_entry_versions_entry_project_fkey FOREIGN KEY (knowledge_entry_id, project_id) REFERENCES app.knowledge_entries(id, project_id);

ALTER TABLE app.knowledge_entry_versions
  ADD CONSTRAINT knowledge_entry_versions_id_project_unique UNIQUE (id, project_id);

ALTER TABLE app.context_pack_selection_items
  ADD CONSTRAINT context_pack_selection_items_version_project_fkey FOREIGN KEY (knowledge_entry_version_id, project_id) REFERENCES app.knowledge_entry_versions(id, project_id);

ALTER TABLE app.context_pack_sources
  ADD CONSTRAINT context_pack_sources_version_project_fkey FOREIGN KEY (knowledge_entry_version_id, project_id) REFERENCES app.knowledge_entry_versions(id, project_id);

ALTER TABLE app.deliverable_sources
  ADD CONSTRAINT deliverable_sources_version_project_fkey FOREIGN KEY (knowledge_entry_version_id, project_id) REFERENCES app.knowledge_entry_versions(id, project_id);

ALTER TABLE app.evidences
  ADD CONSTRAINT evidences_requirement_version_project_fkey FOREIGN KEY (requirement_version_id, project_id) REFERENCES app.knowledge_entry_versions(id, project_id);

ALTER TABLE app.insight_resolutions
  ADD CONSTRAINT insight_resolutions_version_project_fkey FOREIGN KEY (knowledge_entry_version_id, project_id) REFERENCES app.knowledge_entry_versions(id, project_id);

ALTER TABLE app.insight_sources
  ADD CONSTRAINT insight_sources_version_project_fkey FOREIGN KEY (knowledge_entry_version_id, project_id) REFERENCES app.knowledge_entry_versions(id, project_id);

GRANT INSERT, SELECT ON app.knowledge_entry_versions TO ai_center_runtime;

CREATE TRIGGER knowledge_entry_versions_prevent_update
  BEFORE DELETE OR UPDATE ON app.knowledge_entry_versions
  FOR EACH ROW
  EXECUTE FUNCTION app.prevent_append_only_mutation();

CREATE POLICY tenant_editor_insert ON app.knowledge_entry_versions
  FOR INSERT
  WITH
    CHECK
    (((workspace_id = app.current_workspace_id()) AND ( SELECT app.has_workspace_role(knowledge_entry_versions.workspace_id, ARRAY['owner'::text, 'editor'::text]) AS
    has_workspace_role)));

CREATE POLICY tenant_member_select ON app.knowledge_entry_versions
  FOR SELECT
  USING
    (((workspace_id = app.current_workspace_id()) AND ( SELECT app.has_workspace_role(knowledge_entry_versions.workspace_id, ARRAY['owner'::text, 'editor'::text, 'viewer'::text])
    AS has_workspace_role)));

ALTER TABLE app.messages
  FORCE ROW LEVEL SECURITY;

ALTER TABLE app.messages
  ADD CONSTRAINT messages_id_project_unique UNIQUE (id, project_id);

ALTER TABLE app.messages
  ADD COLUMN client_message_id uuid;

GRANT INSERT, SELECT ON app.messages TO ai_center_runtime;

CREATE UNIQUE INDEX messages_session_role_client_id_idx ON app.messages (session_id, ROLE, client_message_id)
  WHERE client_message_id IS NOT NULL;

CREATE TRIGGER messages_prevent_update
  BEFORE DELETE OR UPDATE ON app.messages
  FOR EACH ROW
  EXECUTE FUNCTION app.prevent_append_only_mutation();

CREATE POLICY tenant_editor_insert ON app.messages
  FOR INSERT
  WITH CHECK (((workspace_id = app.current_workspace_id()) AND ( SELECT app.has_workspace_role(messages.workspace_id, ARRAY['owner'::text, 'editor'::text]) AS has_workspace_role)));

CREATE POLICY tenant_member_select ON app.messages
  FOR SELECT
  USING
    (((workspace_id = app.current_workspace_id()) AND ( SELECT app.has_workspace_role(messages.workspace_id, ARRAY['owner'::text, 'editor'::text, 'viewer'::text]) AS
    has_workspace_role)));

CREATE TABLE app.model_runs (
  id                   bigint                   GENERATED ALWAYS AS IDENTITY NOT NULL,
  public_id            uuid                     DEFAULT gen_random_uuid() NOT NULL,
  workspace_id         bigint                   NOT NULL,
  project_id           bigint,
  session_id           bigint,
  context_pack_id      bigint,
  operation            text                     NOT NULL,
  provider             text                     NOT NULL,
  model                text                     NOT NULL,
  prompt_version       text                     NOT NULL,
  schema_version       text                     NOT NULL,
  source_graph_version bigint,
  input_hash           text                     NOT NULL,
  source_public_ids    uuid[]                   DEFAULT '{}'::uuid[] NOT NULL,
  provider_response_id text,
  status               text                     DEFAULT 'pending'::text NOT NULL,
  output               jsonb,
  usage                jsonb                    DEFAULT '{}'::jsonb NOT NULL,
  input_tokens         integer,
  output_tokens        integer,
  estimated_cost       numeric(12,6),
  latency_ms           integer,
  attempt_count        integer                  DEFAULT 0 NOT NULL,
  error_class          text,
  error_message        text,
  started_at           timestamp with time zone,
  completed_at         timestamp with time zone,
  created_at           timestamp with time zone DEFAULT now() NOT NULL,
  updated_at           timestamp with time zone DEFAULT now() NOT NULL
);

ALTER TABLE app.model_runs
  ENABLE ROW LEVEL SECURITY;

ALTER TABLE app.model_runs
  FORCE ROW LEVEL SECURITY;

ALTER TABLE app.model_runs
  ADD CONSTRAINT model_runs_attempt_count_nonnegative CHECK (attempt_count >= 0);

ALTER TABLE app.model_runs
  ADD CONSTRAINT model_runs_completion_consistent
    CHECK ((status = ANY (ARRAY['pending'::text, 'running'::text])) AND completed_at IS NULL OR status = 'completed'::text AND started_at IS NOT NULL AND completed_at IS
    NOT NULL AND output IS NOT NULL AND error_class IS NULL OR status = 'failed'::text AND started_at IS NOT NULL AND completed_at IS NOT NULL AND error_class IS
    NOT NULL OR status = 'cancelled'::text AND completed_at IS NOT NULL);

ALTER TABLE app.model_runs
  ADD CONSTRAINT model_runs_context_pack_id_fkey FOREIGN KEY (context_pack_id) REFERENCES app.context_packs(id) ON DELETE SET NULL;

ALTER TABLE app.model_runs
  ADD CONSTRAINT model_runs_context_pack_project_fkey FOREIGN KEY (context_pack_id, project_id) REFERENCES app.context_packs(id, project_id);

ALTER TABLE app.model_runs
  ADD CONSTRAINT model_runs_estimated_cost_nonnegative CHECK (estimated_cost IS NULL OR estimated_cost >= 0::numeric);

ALTER TABLE app.model_runs
  ADD CONSTRAINT model_runs_id_project_unique UNIQUE (id, project_id);

ALTER TABLE app.deliverable_sources
  ADD CONSTRAINT deliverable_sources_model_run_project_fkey FOREIGN KEY (model_run_id, project_id) REFERENCES app.model_runs(id, project_id);

ALTER TABLE app.model_runs
  ADD CONSTRAINT model_runs_input_hash_valid CHECK (input_hash ~ '^[0-9a-f]{64}$'::text);

ALTER TABLE app.model_runs
  ADD CONSTRAINT model_runs_input_tokens_nonnegative CHECK (input_tokens IS NULL OR input_tokens >= 0);

ALTER TABLE app.model_runs
  ADD CONSTRAINT model_runs_latency_nonnegative CHECK (latency_ms IS NULL OR latency_ms >= 0);

ALTER TABLE app.model_runs
  ADD CONSTRAINT model_runs_model_not_blank CHECK (btrim(model) <> ''::text);

ALTER TABLE app.model_runs
  ADD CONSTRAINT model_runs_operation_valid
    CHECK (operation = ANY (ARRAY['extract_knowledge'::text, 'select_context'::text, 'generate_technical_plan'::text, 'assess_contradiction'::text, 'assess_coverage'::text]));

ALTER TABLE app.model_runs
  ADD CONSTRAINT model_runs_output_tokens_nonnegative CHECK (output_tokens IS NULL OR output_tokens >= 0);

ALTER TABLE app.model_runs
  ADD CONSTRAINT model_runs_pkey PRIMARY KEY (id);

ALTER TABLE app.deliverable_sources
  ADD CONSTRAINT deliverable_sources_model_run_id_fkey FOREIGN KEY (model_run_id) REFERENCES app.model_runs(id);

ALTER TABLE app.model_runs
  ADD CONSTRAINT model_runs_project_id_fkey FOREIGN KEY (project_id) REFERENCES app.projects(id) ON DELETE CASCADE;

ALTER TABLE app.model_runs
  ADD CONSTRAINT model_runs_project_links_consistent CHECK (project_id IS NOT NULL OR session_id IS NULL AND context_pack_id IS NULL);

ALTER TABLE app.model_runs
  ADD CONSTRAINT model_runs_prompt_version_not_blank CHECK (btrim(prompt_version) <> ''::text);

ALTER TABLE app.model_runs
  ADD CONSTRAINT model_runs_provider_not_blank CHECK (btrim(provider) <> ''::text);

ALTER TABLE app.model_runs
  ADD CONSTRAINT model_runs_public_id_key UNIQUE (public_id);

ALTER TABLE app.model_runs
  ADD CONSTRAINT model_runs_schema_version_not_blank CHECK (btrim(schema_version) <> ''::text);

ALTER TABLE app.model_runs
  ADD CONSTRAINT model_runs_session_id_fkey FOREIGN KEY (session_id) REFERENCES app.sessions(id) ON DELETE SET NULL;

ALTER TABLE app.model_runs
  ADD CONSTRAINT model_runs_source_graph_version_nonnegative CHECK (source_graph_version IS NULL OR source_graph_version >= 0);

ALTER TABLE app.model_runs
  ADD CONSTRAINT model_runs_status_valid CHECK (status = ANY (ARRAY['pending'::text, 'running'::text, 'completed'::text, 'failed'::text, 'cancelled'::text]));

ALTER TABLE app.model_runs
  ADD CONSTRAINT model_runs_workspace_id_fkey FOREIGN KEY (workspace_id) REFERENCES app.workspaces(id) ON DELETE CASCADE;

GRANT INSERT, SELECT, UPDATE ON app.model_runs TO ai_center_runtime;

CREATE INDEX model_runs_context_pack_id_idx ON app.model_runs (context_pack_id)
  WHERE context_pack_id IS NOT NULL;

CREATE INDEX model_runs_session_id_idx ON app.model_runs (session_id)
  WHERE session_id IS NOT NULL;

CREATE INDEX model_runs_workspace_status_created_idx ON app.model_runs (workspace_id, status, created_at DESC);

CREATE INDEX model_runs_project_operation_created_idx ON app.model_runs (project_id, operation, created_at DESC)
  WHERE project_id IS NOT NULL;

CREATE TRIGGER model_runs_set_updated_at
  BEFORE UPDATE ON app.model_runs
  FOR EACH ROW
  EXECUTE FUNCTION app.set_updated_at();

CREATE POLICY tenant_editor_insert ON app.model_runs
  FOR INSERT
  WITH
    CHECK (((workspace_id = app.current_workspace_id()) AND ( SELECT app.has_workspace_role(model_runs.workspace_id, ARRAY['owner'::text, 'editor'::text]) AS has_workspace_role)));

CREATE POLICY tenant_editor_update ON app.model_runs
  FOR UPDATE
  USING (((workspace_id = app.current_workspace_id()) AND ( SELECT app.has_workspace_role(model_runs.workspace_id, ARRAY['owner'::text, 'editor'::text]) AS has_workspace_role)))
  WITH
    CHECK (((workspace_id = app.current_workspace_id()) AND ( SELECT app.has_workspace_role(model_runs.workspace_id, ARRAY['owner'::text, 'editor'::text]) AS has_workspace_role)));

CREATE POLICY tenant_member_select ON app.model_runs
  FOR SELECT
  USING
    (((workspace_id = app.current_workspace_id()) AND ( SELECT app.has_workspace_role(model_runs.workspace_id, ARRAY['owner'::text, 'editor'::text, 'viewer'::text]) AS
    has_workspace_role)));

ALTER TABLE app.mutation_proposals
  FORCE ROW LEVEL SECURITY;

ALTER TABLE app.mutation_proposals
  ADD CONSTRAINT mutation_proposals_message_project_fkey FOREIGN KEY (source_message_id, project_id) REFERENCES app.messages(id, project_id);

GRANT INSERT, SELECT, UPDATE ON app.mutation_proposals TO ai_center_runtime;

CREATE POLICY tenant_editor_insert ON app.mutation_proposals
  FOR INSERT
  WITH
    CHECK
    (((workspace_id = app.current_workspace_id()) AND ( SELECT app.has_workspace_role(mutation_proposals.workspace_id, ARRAY['owner'::text, 'editor'::text]) AS
    has_workspace_role)));

CREATE POLICY tenant_editor_update ON app.mutation_proposals
  FOR UPDATE
  USING
    (((workspace_id = app.current_workspace_id()) AND ( SELECT app.has_workspace_role(mutation_proposals.workspace_id, ARRAY['owner'::text, 'editor'::text]) AS
    has_workspace_role)))
  WITH
    CHECK
    (((workspace_id = app.current_workspace_id()) AND ( SELECT app.has_workspace_role(mutation_proposals.workspace_id, ARRAY['owner'::text, 'editor'::text]) AS
    has_workspace_role)));

CREATE POLICY tenant_member_select ON app.mutation_proposals
  FOR SELECT
  USING
    (((workspace_id = app.current_workspace_id()) AND ( SELECT app.has_workspace_role(mutation_proposals.workspace_id, ARRAY['owner'::text, 'editor'::text, 'viewer'::text]) AS
    has_workspace_role)));

ALTER TABLE app.project_templates
  FORCE ROW LEVEL SECURITY;

GRANT SELECT ON app.project_templates TO ai_center_runtime;

CREATE POLICY system_catalog_member_select ON app.project_templates
  FOR SELECT
  USING (( SELECT app.has_workspace_role(app.current_workspace_id(), ARRAY['owner'::text, 'editor'::text, 'viewer'::text]) AS has_workspace_role));

ALTER TABLE app.projects
  FORCE ROW LEVEL SECURITY;

ALTER TABLE app.projects
  ADD CONSTRAINT projects_id_workspace_unique UNIQUE (id, workspace_id);

ALTER TABLE app.artifacts
  ADD CONSTRAINT artifacts_project_workspace_fkey FOREIGN KEY (project_id, workspace_id) REFERENCES app.projects(id, workspace_id) ON DELETE CASCADE;

ALTER TABLE app.audit_events
  ADD CONSTRAINT audit_events_project_workspace_fkey FOREIGN KEY (project_id, workspace_id) REFERENCES app.projects(id, workspace_id) ON DELETE CASCADE;

ALTER TABLE app.context_nodes
  ADD CONSTRAINT context_nodes_project_workspace_fkey FOREIGN KEY (project_id, workspace_id) REFERENCES app.projects(id, workspace_id) ON DELETE CASCADE;

ALTER TABLE app.context_pack_selection_items
  ADD CONSTRAINT context_pack_selection_items_project_workspace_fkey FOREIGN KEY (project_id, workspace_id) REFERENCES app.projects(id, workspace_id) ON DELETE CASCADE;

ALTER TABLE app.context_pack_sources
  ADD CONSTRAINT context_pack_sources_project_workspace_fkey FOREIGN KEY (project_id, workspace_id) REFERENCES app.projects(id, workspace_id) ON DELETE CASCADE;

ALTER TABLE app.context_packs
  ADD CONSTRAINT context_packs_project_workspace_fkey FOREIGN KEY (project_id, workspace_id) REFERENCES app.projects(id, workspace_id) ON DELETE CASCADE;

ALTER TABLE app.deliverable_sections
  ADD CONSTRAINT deliverable_sections_project_workspace_fkey FOREIGN KEY (project_id, workspace_id) REFERENCES app.projects(id, workspace_id) ON DELETE CASCADE;

ALTER TABLE app.deliverable_sources
  ADD CONSTRAINT deliverable_sources_project_workspace_fkey FOREIGN KEY (project_id, workspace_id) REFERENCES app.projects(id, workspace_id) ON DELETE CASCADE;

ALTER TABLE app.deliverables
  ADD CONSTRAINT deliverables_project_workspace_fkey FOREIGN KEY (project_id, workspace_id) REFERENCES app.projects(id, workspace_id) ON DELETE CASCADE;

ALTER TABLE app.domain_events
  ADD CONSTRAINT domain_events_project_workspace_fkey FOREIGN KEY (project_id, workspace_id) REFERENCES app.projects(id, workspace_id) ON DELETE CASCADE;

ALTER TABLE app.edges
  ADD CONSTRAINT edges_project_workspace_fkey FOREIGN KEY (project_id, workspace_id) REFERENCES app.projects(id, workspace_id) ON DELETE CASCADE;

ALTER TABLE app.evidences
  ADD CONSTRAINT evidences_project_workspace_fkey FOREIGN KEY (project_id, workspace_id) REFERENCES app.projects(id, workspace_id) ON DELETE CASCADE;

ALTER TABLE app.execution_events
  ADD CONSTRAINT execution_events_project_workspace_fkey FOREIGN KEY (project_id, workspace_id) REFERENCES app.projects(id, workspace_id) ON DELETE CASCADE;

ALTER TABLE app.executions
  ADD CONSTRAINT executions_project_workspace_fkey FOREIGN KEY (project_id, workspace_id) REFERENCES app.projects(id, workspace_id) ON DELETE CASCADE;

ALTER TABLE app.external_reference_observations
  ADD CONSTRAINT external_reference_observations_project_workspace_fkey FOREIGN KEY (project_id, workspace_id) REFERENCES app.projects(id, workspace_id) ON DELETE CASCADE;

ALTER TABLE app.external_references
  ADD CONSTRAINT external_references_project_workspace_fkey FOREIGN KEY (project_id, workspace_id) REFERENCES app.projects(id, workspace_id) ON DELETE CASCADE;

ALTER TABLE app.gates
  ADD CONSTRAINT gates_project_workspace_fkey FOREIGN KEY (project_id, workspace_id) REFERENCES app.projects(id, workspace_id) ON DELETE CASCADE;

ALTER TABLE app.handoffs
  ADD CONSTRAINT handoffs_project_workspace_fkey FOREIGN KEY (project_id, workspace_id) REFERENCES app.projects(id, workspace_id) ON DELETE CASCADE;

ALTER TABLE app.idempotency_records
  ADD CONSTRAINT idempotency_records_project_workspace_fkey FOREIGN KEY (project_id, workspace_id) REFERENCES app.projects(id, workspace_id) ON DELETE CASCADE;

ALTER TABLE app.insight_resolutions
  ADD CONSTRAINT insight_resolutions_project_workspace_fkey FOREIGN KEY (project_id, workspace_id) REFERENCES app.projects(id, workspace_id) ON DELETE CASCADE;

ALTER TABLE app.insight_sources
  ADD CONSTRAINT insight_sources_project_workspace_fkey FOREIGN KEY (project_id, workspace_id) REFERENCES app.projects(id, workspace_id) ON DELETE CASCADE;

ALTER TABLE app.insights
  ADD CONSTRAINT insights_project_workspace_fkey FOREIGN KEY (project_id, workspace_id) REFERENCES app.projects(id, workspace_id) ON DELETE CASCADE;

ALTER TABLE app.knowledge_entries
  ADD CONSTRAINT knowledge_entries_project_workspace_fkey FOREIGN KEY (project_id, workspace_id) REFERENCES app.projects(id, workspace_id) ON DELETE CASCADE;

ALTER TABLE app.knowledge_entry_versions
  ADD CONSTRAINT knowledge_entry_versions_project_workspace_fkey FOREIGN KEY (project_id, workspace_id) REFERENCES app.projects(id, workspace_id) ON DELETE CASCADE;

ALTER TABLE app.messages
  ADD CONSTRAINT messages_project_workspace_fkey FOREIGN KEY (project_id, workspace_id) REFERENCES app.projects(id, workspace_id) ON DELETE CASCADE;

ALTER TABLE app.model_runs
  ADD CONSTRAINT model_runs_project_workspace_fkey FOREIGN KEY (project_id, workspace_id) REFERENCES app.projects(id, workspace_id) ON DELETE CASCADE;

ALTER TABLE app.mutation_proposals
  ADD CONSTRAINT mutation_proposals_project_workspace_fkey FOREIGN KEY (project_id, workspace_id) REFERENCES app.projects(id, workspace_id) ON DELETE CASCADE;

GRANT INSERT, SELECT, UPDATE ON app.projects TO ai_center_runtime;

CREATE POLICY tenant_editor_insert ON app.projects
  FOR INSERT
  WITH CHECK (((workspace_id = app.current_workspace_id()) AND ( SELECT app.has_workspace_role(projects.workspace_id, ARRAY['owner'::text, 'editor'::text]) AS has_workspace_role)));

CREATE POLICY tenant_editor_update ON app.projects
  FOR UPDATE
  USING (((workspace_id = app.current_workspace_id()) AND ( SELECT app.has_workspace_role(projects.workspace_id, ARRAY['owner'::text, 'editor'::text]) AS has_workspace_role)))
  WITH CHECK (((workspace_id = app.current_workspace_id()) AND ( SELECT app.has_workspace_role(projects.workspace_id, ARRAY['owner'::text, 'editor'::text]) AS has_workspace_role)));

CREATE POLICY tenant_member_select ON app.projects
  FOR SELECT
  USING
    (((workspace_id = app.current_workspace_id()) AND ( SELECT app.has_workspace_role(projects.workspace_id, ARRAY['owner'::text, 'editor'::text, 'viewer'::text]) AS
    has_workspace_role)));

ALTER TABLE app.requirement_coverage
  FORCE ROW LEVEL SECURITY;

ALTER TABLE app.requirement_coverage
  ADD CONSTRAINT requirement_coverage_deliverable_project_fkey FOREIGN KEY (deliverable_id, project_id) REFERENCES app.deliverables(id, project_id);

ALTER TABLE app.requirement_coverage
  ADD CONSTRAINT requirement_coverage_evidence_project_fkey FOREIGN KEY (evidence_id, project_id) REFERENCES app.evidences(id, project_id);

ALTER TABLE app.requirement_coverage
  ADD CONSTRAINT requirement_coverage_requirement_project_fkey FOREIGN KEY (requirement_entry_id, project_id) REFERENCES app.knowledge_entries(id, project_id);

ALTER TABLE app.requirement_coverage
  ADD CONSTRAINT requirement_coverage_section_project_fkey FOREIGN KEY (deliverable_section_id, project_id) REFERENCES app.deliverable_sections(id, project_id);

ALTER TABLE app.requirement_coverage
  ADD CONSTRAINT requirement_coverage_version_project_fkey FOREIGN KEY (requirement_version_id, project_id) REFERENCES app.knowledge_entry_versions(id, project_id);

ALTER TABLE app.requirement_coverage
  ADD COLUMN workspace_id bigint;

UPDATE app.requirement_coverage coverage
SET workspace_id = project.workspace_id
FROM app.projects project
WHERE project.id = coverage.project_id;

ALTER TABLE app.requirement_coverage
  ALTER COLUMN workspace_id SET NOT NULL;

ALTER TABLE app.requirement_coverage
  ADD CONSTRAINT requirement_coverage_project_workspace_fkey FOREIGN KEY (project_id, workspace_id) REFERENCES app.projects(id, workspace_id) ON DELETE CASCADE;

ALTER TABLE app.requirement_coverage
  ADD CONSTRAINT requirement_coverage_workspace_id_fkey FOREIGN KEY (workspace_id) REFERENCES app.workspaces(id) ON DELETE CASCADE;

GRANT INSERT, SELECT, UPDATE ON app.requirement_coverage TO ai_center_runtime;

CREATE INDEX requirement_coverage_workspace_id_idx ON app.requirement_coverage (workspace_id);

CREATE POLICY tenant_editor_insert ON app.requirement_coverage
  FOR INSERT
  WITH
    CHECK
    (((workspace_id = app.current_workspace_id()) AND ( SELECT app.has_workspace_role(requirement_coverage.workspace_id, ARRAY['owner'::text, 'editor'::text]) AS
    has_workspace_role)));

CREATE POLICY tenant_editor_update ON app.requirement_coverage
  FOR UPDATE
  USING
    (((workspace_id = app.current_workspace_id()) AND ( SELECT app.has_workspace_role(requirement_coverage.workspace_id, ARRAY['owner'::text, 'editor'::text]) AS
    has_workspace_role)))
  WITH
    CHECK
    (((workspace_id = app.current_workspace_id()) AND ( SELECT app.has_workspace_role(requirement_coverage.workspace_id, ARRAY['owner'::text, 'editor'::text]) AS
    has_workspace_role)));

CREATE POLICY tenant_member_select ON app.requirement_coverage
  FOR SELECT
  USING
    (((workspace_id = app.current_workspace_id()) AND ( SELECT app.has_workspace_role(requirement_coverage.workspace_id, ARRAY['owner'::text, 'editor'::text, 'viewer'::text]) AS
    has_workspace_role)));

ALTER TABLE app.sessions
  FORCE ROW LEVEL SECURITY;

ALTER TABLE app.sessions
  ADD CONSTRAINT sessions_context_node_project_fkey FOREIGN KEY (context_node_id, project_id) REFERENCES app.context_nodes(id, project_id);

ALTER TABLE app.sessions
  ADD CONSTRAINT sessions_context_pack_project_fkey FOREIGN KEY (context_pack_id, project_id) REFERENCES app.context_packs(id, project_id);

ALTER TABLE app.sessions
  ADD CONSTRAINT sessions_id_project_unique UNIQUE (id, project_id);

ALTER TABLE app.deliverables
  ADD CONSTRAINT deliverables_source_session_project_fkey FOREIGN KEY (source_session_id, project_id) REFERENCES app.sessions(id, project_id);

ALTER TABLE app.handoffs
  ADD CONSTRAINT handoffs_source_session_project_fkey FOREIGN KEY (source_session_id, project_id) REFERENCES app.sessions(id, project_id);

ALTER TABLE app.handoffs
  ADD CONSTRAINT handoffs_target_session_project_fkey FOREIGN KEY (target_session_id, project_id) REFERENCES app.sessions(id, project_id);

ALTER TABLE app.messages
  ADD CONSTRAINT messages_session_project_fkey FOREIGN KEY (session_id, project_id) REFERENCES app.sessions(id, project_id);

ALTER TABLE app.model_runs
  ADD CONSTRAINT model_runs_session_project_fkey FOREIGN KEY (session_id, project_id) REFERENCES app.sessions(id, project_id);

ALTER TABLE app.mutation_proposals
  ADD CONSTRAINT mutation_proposals_session_project_fkey FOREIGN KEY (session_id, project_id) REFERENCES app.sessions(id, project_id);

ALTER TABLE app.sessions
  ADD CONSTRAINT sessions_project_workspace_fkey FOREIGN KEY (project_id, workspace_id) REFERENCES app.projects(id, workspace_id) ON DELETE CASCADE;

GRANT INSERT, SELECT, UPDATE ON app.sessions TO ai_center_runtime;

CREATE POLICY tenant_editor_insert ON app.sessions
  FOR INSERT
  WITH CHECK (((workspace_id = app.current_workspace_id()) AND ( SELECT app.has_workspace_role(sessions.workspace_id, ARRAY['owner'::text, 'editor'::text]) AS has_workspace_role)));

CREATE POLICY tenant_editor_update ON app.sessions
  FOR UPDATE
  USING (((workspace_id = app.current_workspace_id()) AND ( SELECT app.has_workspace_role(sessions.workspace_id, ARRAY['owner'::text, 'editor'::text]) AS has_workspace_role)))
  WITH CHECK (((workspace_id = app.current_workspace_id()) AND ( SELECT app.has_workspace_role(sessions.workspace_id, ARRAY['owner'::text, 'editor'::text]) AS has_workspace_role)));

CREATE POLICY tenant_member_select ON app.sessions
  FOR SELECT
  USING
    (((workspace_id = app.current_workspace_id()) AND ( SELECT app.has_workspace_role(sessions.workspace_id, ARRAY['owner'::text, 'editor'::text, 'viewer'::text]) AS
    has_workspace_role)));

CREATE TABLE app.steward_assessment_sources (
  id                         bigint GENERATED ALWAYS AS IDENTITY NOT NULL,
  workspace_id               bigint NOT NULL,
  project_id                 bigint NOT NULL,
  assessment_id              bigint NOT NULL,
  source_role                text   NOT NULL,
  knowledge_entry_version_id bigint NOT NULL
);

ALTER TABLE app.steward_assessment_sources
  ENABLE ROW LEVEL SECURITY;

ALTER TABLE app.steward_assessment_sources
  FORCE ROW LEVEL SECURITY;

ALTER TABLE app.steward_assessment_sources
  ADD CONSTRAINT steward_assessment_sources_knowledge_entry_version_id_fkey FOREIGN KEY (knowledge_entry_version_id) REFERENCES app.knowledge_entry_versions(id);

ALTER TABLE app.steward_assessment_sources
  ADD CONSTRAINT steward_assessment_sources_pkey PRIMARY KEY (id);

ALTER TABLE app.steward_assessment_sources
  ADD CONSTRAINT steward_assessment_sources_project_id_fkey FOREIGN KEY (project_id) REFERENCES app.projects(id) ON DELETE CASCADE;

ALTER TABLE app.steward_assessment_sources
  ADD CONSTRAINT steward_assessment_sources_project_workspace_fkey FOREIGN KEY (project_id, workspace_id) REFERENCES app.projects(id, workspace_id) ON DELETE CASCADE;

ALTER TABLE app.steward_assessment_sources
  ADD CONSTRAINT steward_assessment_sources_role_unique UNIQUE (assessment_id, source_role);

ALTER TABLE app.steward_assessment_sources
  ADD CONSTRAINT steward_assessment_sources_role_valid CHECK (source_role = ANY (ARRAY['left'::text, 'right'::text]));

ALTER TABLE app.steward_assessment_sources
  ADD CONSTRAINT steward_assessment_sources_version_project_fkey FOREIGN KEY (knowledge_entry_version_id, project_id) REFERENCES app.knowledge_entry_versions(id, project_id);

ALTER TABLE app.steward_assessment_sources
  ADD CONSTRAINT steward_assessment_sources_workspace_id_fkey FOREIGN KEY (workspace_id) REFERENCES app.workspaces(id) ON DELETE CASCADE;

GRANT INSERT, SELECT ON app.steward_assessment_sources TO ai_center_runtime;

CREATE INDEX steward_assessment_sources_project_id_idx ON app.steward_assessment_sources (project_id);

CREATE INDEX steward_assessment_sources_workspace_id_idx ON app.steward_assessment_sources (workspace_id);

CREATE INDEX steward_assessment_sources_assessment_id_idx ON app.steward_assessment_sources (assessment_id);

CREATE INDEX steward_assessment_sources_version_id_idx ON app.steward_assessment_sources (knowledge_entry_version_id);

CREATE TRIGGER steward_assessment_sources_prevent_update
  BEFORE DELETE OR UPDATE ON app.steward_assessment_sources
  FOR EACH ROW
  EXECUTE FUNCTION app.prevent_append_only_mutation();

CREATE POLICY tenant_editor_insert ON app.steward_assessment_sources
  FOR INSERT
  WITH
    CHECK
    (((workspace_id = app.current_workspace_id()) AND ( SELECT app.has_workspace_role(steward_assessment_sources.workspace_id, ARRAY['owner'::text, 'editor'::text]) AS
    has_workspace_role)));

CREATE POLICY tenant_member_select ON app.steward_assessment_sources
  FOR SELECT
  USING
    (((workspace_id = app.current_workspace_id()) AND ( SELECT app.has_workspace_role(steward_assessment_sources.workspace_id, ARRAY['owner'::text, 'editor'::text, 'viewer'::text])
    AS has_workspace_role)));

CREATE TABLE app.steward_assessments (
  id             bigint                   GENERATED ALWAYS AS IDENTITY NOT NULL,
  public_id      uuid                     DEFAULT gen_random_uuid() NOT NULL,
  workspace_id   bigint                   NOT NULL,
  project_id     bigint                   NOT NULL,
  model_run_id   bigint,
  graph_version  bigint                   NOT NULL,
  fingerprint    text                     NOT NULL,
  classification text                     NOT NULL,
  severity       text,
  confidence     numeric(4,3)             NOT NULL,
  title          text                     NOT NULL,
  explanation    text                     NOT NULL,
  created_at     timestamp with time zone DEFAULT now() NOT NULL
);

ALTER TABLE app.steward_assessments
  ENABLE ROW LEVEL SECURITY;

ALTER TABLE app.steward_assessments
  FORCE ROW LEVEL SECURITY;

ALTER TABLE app.steward_assessments
  ADD CONSTRAINT steward_assessments_classification_valid CHECK (classification = ANY (ARRAY['contradiction'::text, 'compatible'::text, 'ambiguous'::text]));

ALTER TABLE app.steward_assessments
  ADD CONSTRAINT steward_assessments_confidence_range CHECK (confidence >= 0::numeric AND confidence <= 1::numeric);

ALTER TABLE app.steward_assessments
  ADD CONSTRAINT steward_assessments_contradiction_severity_required CHECK (classification <> 'contradiction'::text OR severity IS NOT NULL);

ALTER TABLE app.steward_assessments
  ADD CONSTRAINT steward_assessments_explanation_not_blank CHECK (btrim(explanation) <> ''::text);

ALTER TABLE app.steward_assessments
  ADD CONSTRAINT steward_assessments_fingerprint_valid CHECK (fingerprint ~ '^[0-9a-f]{64}$'::text);

ALTER TABLE app.steward_assessments
  ADD CONSTRAINT steward_assessments_graph_version_nonnegative CHECK (graph_version >= 0);

ALTER TABLE app.steward_assessments
  ADD CONSTRAINT steward_assessments_id_project_unique UNIQUE (id, project_id);

ALTER TABLE app.insights
  ADD CONSTRAINT insights_assessment_project_fkey FOREIGN KEY (steward_assessment_id, project_id) REFERENCES app.steward_assessments(id, project_id);

ALTER TABLE app.steward_assessment_sources
  ADD CONSTRAINT steward_assessment_sources_assessment_project_fkey FOREIGN KEY (assessment_id, project_id) REFERENCES app.steward_assessments(id, project_id);

ALTER TABLE app.steward_assessments
  ADD CONSTRAINT steward_assessments_model_run_id_fkey FOREIGN KEY (model_run_id) REFERENCES app.model_runs(id) ON DELETE SET NULL;

ALTER TABLE app.steward_assessments
  ADD CONSTRAINT steward_assessments_pkey PRIMARY KEY (id);

ALTER TABLE app.insights
  ADD CONSTRAINT insights_steward_assessment_id_fkey FOREIGN KEY (steward_assessment_id) REFERENCES app.steward_assessments(id) ON DELETE SET NULL;

ALTER TABLE app.steward_assessment_sources
  ADD CONSTRAINT steward_assessment_sources_assessment_id_fkey FOREIGN KEY (assessment_id) REFERENCES app.steward_assessments(id) ON DELETE CASCADE;

ALTER TABLE app.steward_assessments
  ADD CONSTRAINT steward_assessments_project_fingerprint_graph_unique UNIQUE (project_id, fingerprint, graph_version);

ALTER TABLE app.steward_assessments
  ADD CONSTRAINT steward_assessments_project_id_fkey FOREIGN KEY (project_id) REFERENCES app.projects(id) ON DELETE CASCADE;

ALTER TABLE app.steward_assessments
  ADD CONSTRAINT steward_assessments_project_workspace_fkey FOREIGN KEY (project_id, workspace_id) REFERENCES app.projects(id, workspace_id) ON DELETE CASCADE;

ALTER TABLE app.steward_assessments
  ADD CONSTRAINT steward_assessments_public_id_key UNIQUE (public_id);

ALTER TABLE app.steward_assessments
  ADD CONSTRAINT steward_assessments_severity_valid CHECK (severity IS NULL OR (severity = ANY (ARRAY['notice'::text, 'warning'::text, 'blocking'::text])));

ALTER TABLE app.steward_assessments
  ADD CONSTRAINT steward_assessments_title_not_blank CHECK (btrim(title) <> ''::text);

ALTER TABLE app.steward_assessments
  ADD CONSTRAINT steward_assessments_workspace_id_fkey FOREIGN KEY (workspace_id) REFERENCES app.workspaces(id) ON DELETE CASCADE;

GRANT INSERT, SELECT ON app.steward_assessments TO ai_center_runtime;

CREATE INDEX steward_assessments_project_classification_created_idx ON app.steward_assessments (project_id, classification, created_at DESC);

CREATE INDEX steward_assessments_workspace_id_idx ON app.steward_assessments (workspace_id);

CREATE INDEX steward_assessments_model_run_id_idx ON app.steward_assessments (model_run_id)
  WHERE model_run_id IS NOT NULL;

CREATE TRIGGER steward_assessments_prevent_update
  BEFORE DELETE OR UPDATE ON app.steward_assessments
  FOR EACH ROW
  EXECUTE FUNCTION app.prevent_append_only_mutation();

CREATE POLICY tenant_editor_insert ON app.steward_assessments
  FOR INSERT
  WITH
    CHECK
    (((workspace_id = app.current_workspace_id()) AND ( SELECT app.has_workspace_role(steward_assessments.workspace_id, ARRAY['owner'::text, 'editor'::text]) AS
    has_workspace_role)));

CREATE POLICY tenant_member_select ON app.steward_assessments
  FOR SELECT
  USING
    (((workspace_id = app.current_workspace_id()) AND ( SELECT app.has_workspace_role(steward_assessments.workspace_id, ARRAY['owner'::text, 'editor'::text, 'viewer'::text]) AS
    has_workspace_role)));

ALTER TABLE app.tasks
  FORCE ROW LEVEL SECURITY;

ALTER TABLE app.tasks
  ADD CONSTRAINT tasks_context_pack_project_fkey FOREIGN KEY (context_pack_id, project_id) REFERENCES app.context_packs(id, project_id);

ALTER TABLE app.tasks
  ADD CONSTRAINT tasks_id_project_unique UNIQUE (id, project_id);

ALTER TABLE app.executions
  ADD CONSTRAINT executions_task_project_fkey FOREIGN KEY (task_id, project_id) REFERENCES app.tasks(id, project_id);

ALTER TABLE app.tasks
  ADD CONSTRAINT tasks_project_workspace_fkey FOREIGN KEY (project_id, workspace_id) REFERENCES app.projects(id, workspace_id) ON DELETE CASCADE;

CREATE POLICY tenant_editor_insert ON app.tasks
  FOR INSERT
  WITH CHECK (((workspace_id = app.current_workspace_id()) AND ( SELECT app.has_workspace_role(tasks.workspace_id, ARRAY['owner'::text, 'editor'::text]) AS has_workspace_role)));

CREATE POLICY tenant_editor_update ON app.tasks
  FOR UPDATE
  USING (((workspace_id = app.current_workspace_id()) AND ( SELECT app.has_workspace_role(tasks.workspace_id, ARRAY['owner'::text, 'editor'::text]) AS has_workspace_role)))
  WITH CHECK (((workspace_id = app.current_workspace_id()) AND ( SELECT app.has_workspace_role(tasks.workspace_id, ARRAY['owner'::text, 'editor'::text]) AS has_workspace_role)));

CREATE POLICY tenant_member_select ON app.tasks
  FOR SELECT
  USING
    (((workspace_id = app.current_workspace_id()) AND ( SELECT app.has_workspace_role(tasks.workspace_id, ARRAY['owner'::text, 'editor'::text, 'viewer'::text]) AS
    has_workspace_role)));

CREATE TABLE app.tool_connections (
  id                  bigint                   GENERATED ALWAYS AS IDENTITY NOT NULL,
  public_id           uuid                     DEFAULT gen_random_uuid() NOT NULL,
  workspace_id        bigint                   NOT NULL,
  provider            text                     NOT NULL,
  auth_mode           text                     NOT NULL,
  external_account_id text                     NOT NULL,
  display_name        text                     NOT NULL,
  capabilities        text[]                   DEFAULT '{}'::text[] NOT NULL,
  secret_reference    text                     NOT NULL,
  configuration       jsonb                    DEFAULT '{}'::jsonb NOT NULL,
  status              text                     DEFAULT 'pending'::text NOT NULL,
  created_by_actor_id uuid                     NOT NULL,
  last_verified_at    timestamp with time zone,
  created_at          timestamp with time zone DEFAULT now() NOT NULL,
  updated_at          timestamp with time zone DEFAULT now() NOT NULL
);

ALTER TABLE app.tool_connections
  ENABLE ROW LEVEL SECURITY;

ALTER TABLE app.tool_connections
  FORCE ROW LEVEL SECURITY;

ALTER TABLE app.tool_connections
  ADD CONSTRAINT tool_connections_auth_mode_valid CHECK (auth_mode = 'github_app'::text);

ALTER TABLE app.tool_connections
  ADD CONSTRAINT tool_connections_configuration_is_object CHECK (jsonb_typeof(configuration) = 'object'::text);

ALTER TABLE app.tool_connections
  ADD CONSTRAINT tool_connections_display_name_not_blank CHECK (btrim(display_name) <> ''::text);

ALTER TABLE app.tool_connections
  ADD CONSTRAINT tool_connections_external_account_not_blank CHECK (btrim(external_account_id) <> ''::text);

ALTER TABLE app.tool_connections
  ADD CONSTRAINT tool_connections_id_workspace_unique UNIQUE (id, workspace_id);

ALTER TABLE app.external_references
  ADD CONSTRAINT external_references_connection_workspace_fkey FOREIGN KEY (tool_connection_id, workspace_id) REFERENCES app.tool_connections(id, workspace_id);

ALTER TABLE app.tool_connections
  ADD CONSTRAINT tool_connections_pkey PRIMARY KEY (id);

ALTER TABLE app.external_references
  ADD CONSTRAINT external_references_tool_connection_id_fkey FOREIGN KEY (tool_connection_id) REFERENCES app.tool_connections(id);

ALTER TABLE app.tool_connections
  ADD CONSTRAINT tool_connections_provider_valid CHECK (provider = 'github'::text);

ALTER TABLE app.tool_connections
  ADD CONSTRAINT tool_connections_public_id_key UNIQUE (public_id);

ALTER TABLE app.tool_connections
  ADD CONSTRAINT tool_connections_secret_reference_not_blank CHECK (btrim(secret_reference) <> ''::text);

ALTER TABLE app.tool_connections
  ADD CONSTRAINT tool_connections_status_valid CHECK (status = ANY (ARRAY['pending'::text, 'active'::text, 'degraded'::text, 'revoked'::text]));

ALTER TABLE app.tool_connections
  ADD CONSTRAINT tool_connections_workspace_id_fkey FOREIGN KEY (workspace_id) REFERENCES app.workspaces(id) ON DELETE CASCADE;

ALTER TABLE app.tool_connections
  ADD CONSTRAINT tool_connections_workspace_provider_account_unique UNIQUE (workspace_id, PROVIDER, external_account_id);

GRANT SELECT ON app.tool_connections TO ai_center_runtime;

CREATE INDEX tool_connections_workspace_status_idx ON app.tool_connections (workspace_id, status, created_at DESC);

CREATE TRIGGER tool_connections_set_updated_at
  BEFORE UPDATE ON app.tool_connections
  FOR EACH ROW
  EXECUTE FUNCTION app.set_updated_at();

CREATE POLICY tenant_editor_insert ON app.tool_connections
  FOR INSERT
  WITH
    CHECK
    (((workspace_id = app.current_workspace_id()) AND ( SELECT app.has_workspace_role(tool_connections.workspace_id, ARRAY['owner'::text, 'editor'::text]) AS has_workspace_role)));

CREATE POLICY tenant_editor_update ON app.tool_connections
  FOR UPDATE
  USING
    (((workspace_id = app.current_workspace_id()) AND ( SELECT app.has_workspace_role(tool_connections.workspace_id, ARRAY['owner'::text, 'editor'::text]) AS has_workspace_role)))
  WITH
    CHECK
    (((workspace_id = app.current_workspace_id()) AND ( SELECT app.has_workspace_role(tool_connections.workspace_id, ARRAY['owner'::text, 'editor'::text]) AS has_workspace_role)));

CREATE POLICY tenant_member_select ON app.tool_connections
  FOR SELECT
  USING
    (((workspace_id = app.current_workspace_id()) AND ( SELECT app.has_workspace_role(tool_connections.workspace_id, ARRAY['owner'::text, 'editor'::text, 'viewer'::text]) AS
    has_workspace_role)));

CREATE TABLE app.workspace_members (
  id                  bigint                   GENERATED ALWAYS AS IDENTITY NOT NULL,
  public_id           uuid                     DEFAULT gen_random_uuid() NOT NULL,
  workspace_id        bigint                   NOT NULL,
  actor_id            uuid                     NOT NULL,
  role                text                     DEFAULT 'viewer'::text NOT NULL,
  invitation_status   text                     DEFAULT 'pending'::text NOT NULL,
  invited_by_actor_id uuid,
  accepted_at         timestamp with time zone,
  created_at          timestamp with time zone DEFAULT now() NOT NULL,
  updated_at          timestamp with time zone DEFAULT now() NOT NULL
);

-- Preserve access to every workspace created before membership-based auth.
-- The NOT EXISTS guard keeps this backfill replay-safe independently from the
-- seed, which is intentionally not part of production migration deployment.
INSERT INTO app.workspace_members (
  workspace_id,
  actor_id,
  role,
  invitation_status,
  invited_by_actor_id,
  accepted_at
)
SELECT
  workspace.id,
  workspace.owner_actor_id,
  'owner',
  'accepted',
  workspace.owner_actor_id,
  workspace.created_at
FROM app.workspaces workspace
WHERE NOT EXISTS (
  SELECT 1
  FROM app.workspace_members existing_member
  WHERE existing_member.workspace_id = workspace.id
    AND existing_member.actor_id = workspace.owner_actor_id
);

ALTER TABLE app.workspace_members
  ENABLE ROW LEVEL SECURITY;

ALTER TABLE app.workspace_members
  FORCE ROW LEVEL SECURITY;

ALTER TABLE app.workspace_members
  ADD CONSTRAINT workspace_members_acceptance_consistent CHECK (invitation_status = 'accepted'::text AND accepted_at IS
    NOT NULL OR invitation_status = 'pending'::text AND accepted_at IS NULL OR invitation_status = 'revoked'::text);

ALTER TABLE app.workspace_members
  ADD CONSTRAINT workspace_members_invitation_status_valid CHECK (invitation_status = ANY (ARRAY['pending'::text, 'accepted'::text, 'revoked'::text]));

ALTER TABLE app.workspace_members
  ADD CONSTRAINT workspace_members_pkey PRIMARY KEY (id);

ALTER TABLE app.workspace_members
  ADD CONSTRAINT workspace_members_public_id_key UNIQUE (public_id);

ALTER TABLE app.workspace_members
  ADD CONSTRAINT workspace_members_role_valid CHECK (role = ANY (ARRAY['owner'::text, 'editor'::text, 'viewer'::text]));

ALTER TABLE app.workspace_members
  ADD CONSTRAINT workspace_members_workspace_actor_unique UNIQUE (workspace_id, actor_id);

ALTER TABLE app.workspace_members
  ADD CONSTRAINT workspace_members_workspace_id_fkey FOREIGN KEY (workspace_id) REFERENCES app.workspaces(id) ON DELETE CASCADE;

GRANT INSERT, SELECT, UPDATE ON app.workspace_members TO ai_center_runtime;

GRANT USAGE ON SEQUENCE
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
TO ai_center_runtime;

CREATE INDEX workspace_members_actor_status_idx ON app.workspace_members (actor_id, invitation_status, workspace_id);

CREATE TRIGGER workspace_members_set_updated_at
  BEFORE UPDATE ON app.workspace_members
  FOR EACH ROW
  EXECUTE FUNCTION app.set_updated_at();

CREATE POLICY workspace_members_owner_insert ON app.workspace_members
  FOR INSERT
  WITH CHECK (((workspace_id = app.current_workspace_id()) AND ( SELECT app.has_workspace_role(workspace_members.workspace_id, ARRAY['owner'::text]) AS has_workspace_role)));

CREATE POLICY workspace_members_owner_update ON app.workspace_members
  FOR UPDATE
  USING (((workspace_id = app.current_workspace_id()) AND ( SELECT app.has_workspace_role(workspace_members.workspace_id, ARRAY['owner'::text]) AS has_workspace_role)))
  WITH CHECK (((workspace_id = app.current_workspace_id()) AND ( SELECT app.has_workspace_role(workspace_members.workspace_id, ARRAY['owner'::text]) AS has_workspace_role)));

CREATE POLICY workspace_members_scoped_select ON app.workspace_members
  FOR SELECT
  USING
    (((workspace_id = app.current_workspace_id()) AND ( SELECT app.has_workspace_role(workspace_members.workspace_id, ARRAY['owner'::text, 'editor'::text, 'viewer'::text]) AS
    has_workspace_role)));

ALTER TABLE app.workspaces
  FORCE ROW LEVEL SECURITY;

GRANT SELECT ON app.workspaces TO ai_center_runtime;

CREATE POLICY workspaces_member_select ON app.workspaces
  FOR SELECT
  USING (((id = app.current_workspace_id()) AND ( SELECT app.has_workspace_role(workspaces.id, ARRAY['owner'::text, 'editor'::text, 'viewer'::text]) AS has_workspace_role)));

CREATE POLICY workspaces_owner_update ON app.workspaces
  FOR UPDATE
  USING (((id = app.current_workspace_id()) AND ( SELECT app.has_workspace_role(workspaces.id, ARRAY['owner'::text]) AS has_workspace_role)))
  WITH CHECK (((id = app.current_workspace_id()) AND ( SELECT app.has_workspace_role(workspaces.id, ARRAY['owner'::text]) AS has_workspace_role)));

CREATE FUNCTION app.list_due_steward_workspaces (
  requested_limit integer DEFAULT 16
)
  RETURNS TABLE (
    workspace_id        bigint,
    workspace_public_id uuid,
    actor_id            uuid,
    workspace_role      text
  )
  LANGUAGE sql
  SECURITY DEFINER
  ROWS 128
  SET search_path TO ''
  SET row_security TO 'off'
  AS $function$
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
$function$;

REVOKE ALL ON FUNCTION app.list_due_steward_workspaces(integer)
  FROM PUBLIC, anon, authenticated, service_role, ai_center_runtime;

GRANT EXECUTE ON FUNCTION app.list_due_steward_workspaces(integer)
  TO ai_center_runtime;
