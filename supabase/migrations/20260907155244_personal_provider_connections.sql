-- Migration unit 1: schema_changes
-- Transaction mode: transactional
-- Boundary reason: default

ALTER TABLE app.domain_events
  ADD COLUMN requested_by_actor_id uuid DEFAULT (NULLIF(current_setting('app.current_actor_id'::text, true), ''::text))::uuid;

CREATE TABLE app.provider_connections (
  public_id      uuid                     NOT NULL,
  workspace_id   bigint                   NOT NULL,
  actor_id       uuid                     NOT NULL,
  provider       text                     NOT NULL,
  name           text                     NOT NULL,
  model          text                     NOT NULL,
  key_ciphertext bytea,
  key_hint       text,
  created_at     timestamp with time zone DEFAULT now() NOT NULL,
  updated_at     timestamp with time zone DEFAULT now() NOT NULL
);

ALTER TABLE app.provider_connections
  ENABLE ROW LEVEL SECURITY;

ALTER TABLE app.provider_connections
  FORCE ROW LEVEL SECURITY;

ALTER TABLE app.provider_connections
  ADD CONSTRAINT provider_connections_check
    CHECK
    (provider = 'claude_subscription'::text AND key_ciphertext IS NULL AND key_hint IS NULL OR provider <> 'claude_subscription'::text AND octet_length(key_ciphertext) >= 30 AND
    key_ciphertext IS NOT NULL AND key_hint IS NOT NULL);

ALTER TABLE app.provider_connections
  ADD CONSTRAINT provider_connections_model_check CHECK (length(model) >= 1 AND length(model) <= 200);

ALTER TABLE app.provider_connections
  ADD CONSTRAINT provider_connections_name_check CHECK (length(name) >= 1 AND length(name) <= 120);

ALTER TABLE app.provider_connections
  ADD CONSTRAINT provider_connections_pkey PRIMARY KEY (public_id);

ALTER TABLE app.provider_connections
  ADD CONSTRAINT provider_connections_provider_check
    CHECK (provider = ANY (ARRAY['openai'::text, 'anthropic'::text, 'kimi'::text, 'deepseek'::text, 'openrouter'::text, 'claude_subscription'::text]));

ALTER TABLE app.provider_connections
  ADD CONSTRAINT provider_connections_workspace_id_actor_id_public_id_key UNIQUE (workspace_id, actor_id, public_id);

ALTER TABLE app.provider_connections
  ADD CONSTRAINT provider_connections_workspace_id_fkey FOREIGN KEY (workspace_id) REFERENCES app.workspaces(id) ON DELETE CASCADE;

GRANT DELETE, INSERT, SELECT, UPDATE ON app.provider_connections TO ai_center_runtime;

CREATE POLICY provider_connections_private ON app.provider_connections
  USING
    (((workspace_id = ( SELECT app.current_workspace_id() AS current_workspace_id)) AND (actor_id = ( SELECT app.current_actor_id() AS current_actor_id)) AND
    app.has_workspace_role(workspace_id, ARRAY['owner'::text, 'editor'::text])))
  WITH
    CHECK
    (((workspace_id = ( SELECT app.current_workspace_id() AS current_workspace_id)) AND (actor_id = ( SELECT app.current_actor_id() AS current_actor_id)) AND
    app.has_workspace_role(workspace_id, ARRAY['owner'::text, 'editor'::text])));

CREATE TABLE app.provider_selections (
  workspace_id  bigint                   NOT NULL,
  actor_id      uuid                     NOT NULL,
  mode          text                     NOT NULL,
  connection_id uuid,
  updated_at    timestamp with time zone DEFAULT now() NOT NULL
);

ALTER TABLE app.provider_selections
  ENABLE ROW LEVEL SECURITY;

ALTER TABLE app.provider_selections
  FORCE ROW LEVEL SECURITY;

ALTER TABLE app.provider_selections
  ADD CONSTRAINT provider_selections_check CHECK ((mode = 'connection'::text) = (connection_id IS NOT NULL));

ALTER TABLE app.provider_selections
  ADD CONSTRAINT provider_selections_mode_check CHECK (mode = ANY (ARRAY['server_default'::text, 'deterministic'::text, 'connection'::text]));

ALTER TABLE app.provider_selections
  ADD CONSTRAINT provider_selections_pkey PRIMARY KEY (workspace_id, actor_id);

ALTER TABLE app.provider_selections
  ADD CONSTRAINT provider_selections_workspace_id_actor_id_connection_id_fkey FOREIGN KEY (workspace_id, actor_id, connection_id)
    REFERENCES app.provider_connections(workspace_id, actor_id, public_id);

ALTER TABLE app.provider_selections
  ADD CONSTRAINT provider_selections_workspace_id_fkey FOREIGN KEY (workspace_id) REFERENCES app.workspaces(id) ON DELETE CASCADE;

GRANT DELETE, INSERT, SELECT, UPDATE ON app.provider_selections TO ai_center_runtime;

CREATE POLICY provider_selections_private ON app.provider_selections
  USING
    (((workspace_id = ( SELECT app.current_workspace_id() AS current_workspace_id)) AND (actor_id = ( SELECT app.current_actor_id() AS current_actor_id)) AND
    app.has_workspace_role(workspace_id, ARRAY['owner'::text, 'editor'::text])))
  WITH
    CHECK
    (((workspace_id = ( SELECT app.current_workspace_id() AS current_workspace_id)) AND (actor_id = ( SELECT app.current_actor_id() AS current_actor_id)) AND
    app.has_workspace_role(workspace_id, ARRAY['owner'::text, 'editor'::text])));