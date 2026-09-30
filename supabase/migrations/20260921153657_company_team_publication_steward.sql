alter table "app"."insights" drop constraint "insights_type_valid";


  create table "app"."publication_jobs" (
    "id" bigint generated always as identity not null,
    "public_id" uuid not null default gen_random_uuid(),
    "workspace_id" bigint not null,
    "project_id" bigint not null,
    "artifact_version_id" bigint not null,
    "connection_id" bigint not null,
    "connection_revision" integer not null,
    "requested_by_actor_id" uuid not null,
    "provider" text not null,
    "target_id" text not null,
    "title" text not null,
    "body_markdown" text not null,
    "content_hash" text not null,
    "status" text not null default 'queued'::text,
    "attempt_count" integer not null default 0,
    "lease_token" uuid,
    "lease_until" timestamp with time zone,
    "external_id" text,
    "external_url" text,
    "error_code" text,
    "created_at" timestamp with time zone not null default now(),
    "updated_at" timestamp with time zone not null default now()
      );


alter table "app"."publication_jobs" enable row level security;


  create table "app"."publication_observations" (
    "id" bigint generated always as identity not null,
    "public_id" uuid not null default gen_random_uuid(),
    "workspace_id" bigint not null,
    "publication_job_id" bigint not null,
    "observed_at" timestamp with time zone not null default now(),
    "observation_kind" text not null,
    "external_id" text not null,
    "external_url" text not null,
    "remote_updated_at" text,
    "snapshot" jsonb not null
      );


alter table "app"."publication_observations" enable row level security;


  create table "app"."steward_scope_sources" (
    "id" bigint generated always as identity not null,
    "workspace_id" bigint not null,
    "project_id" bigint not null,
    "assessment_id" bigint not null,
    "source_project_id" bigint not null,
    "source_role" text not null,
    "source_kind" text not null,
    "source_public_id" uuid not null,
    "knowledge_version_id" bigint,
    "artifact_version_id" bigint,
    "external_observation_id" bigint,
    "publication_observation_id" bigint,
    "source_snapshot" jsonb not null
      );


alter table "app"."steward_scope_sources" enable row level security;


  create table "app"."work_tool_connections" (
    "id" bigint generated always as identity not null,
    "public_id" uuid not null,
    "workspace_id" bigint not null,
    "provider" text not null,
    "name" text not null,
    "encrypted_credential" bytea not null,
    "credential_actor_id" uuid not null,
    "enabled" boolean not null default true,
    "revision" integer not null default 1,
    "created_at" timestamp with time zone not null default now(),
    "updated_at" timestamp with time zone not null default now()
      );


alter table "app"."work_tool_connections" enable row level security;


  create table "app"."workspace_invitations" (
    "id" bigint generated always as identity not null,
    "public_id" uuid not null,
    "workspace_id" bigint not null,
    "created_by_actor_id" uuid not null,
    "role" text not null,
    "label" text not null default ''::text,
    "token_hash" text not null,
    "request_hash" text not null,
    "status" text not null default 'pending'::text,
    "accepted_by_actor_id" uuid,
    "accepted_at" timestamp with time zone,
    "expires_at" timestamp with time zone not null,
    "created_at" timestamp with time zone not null default now(),
    "revoked_at" timestamp with time zone
      );


alter table "app"."workspace_invitations" enable row level security;

alter table "app"."workspace_members" add column "display_name" text not null default ''::text;

CREATE INDEX publication_jobs_connection_idx ON app.publication_jobs USING btree (connection_id);

CREATE UNIQUE INDEX publication_jobs_id_workspace_id_key ON app.publication_jobs USING btree (id, workspace_id);

CREATE INDEX publication_jobs_lease_idx ON app.publication_jobs USING btree (lease_until) WHERE (status = 'processing'::text);

CREATE UNIQUE INDEX publication_jobs_pkey ON app.publication_jobs USING btree (id);

CREATE INDEX publication_jobs_project_idx ON app.publication_jobs USING btree (project_id);

CREATE UNIQUE INDEX publication_jobs_public_id_key ON app.publication_jobs USING btree (public_id);

CREATE INDEX publication_jobs_queue_idx ON app.publication_jobs USING btree (created_at, id) WHERE (status = 'queued'::text);

CREATE INDEX publication_jobs_scope_idx ON app.publication_jobs USING btree (workspace_id, artifact_version_id, created_at DESC);

CREATE INDEX publication_jobs_version_idx ON app.publication_jobs USING btree (artifact_version_id);

CREATE UNIQUE INDEX publication_jobs_workspace_id_artifact_version_id_provider__key ON app.publication_jobs USING btree (workspace_id, artifact_version_id, provider, target_id);

CREATE UNIQUE INDEX publication_observations_id_workspace_id_key ON app.publication_observations USING btree (id, workspace_id);

CREATE INDEX publication_observations_job_idx ON app.publication_observations USING btree (publication_job_id, id DESC);

CREATE UNIQUE INDEX publication_observations_pkey ON app.publication_observations USING btree (id);

CREATE UNIQUE INDEX publication_observations_public_id_key ON app.publication_observations USING btree (public_id);

CREATE INDEX publication_observations_workspace_idx ON app.publication_observations USING btree (workspace_id);

CREATE INDEX steward_scope_sources_artifact_idx ON app.steward_scope_sources USING btree (artifact_version_id) WHERE (artifact_version_id IS NOT NULL);

CREATE UNIQUE INDEX steward_scope_sources_assessment_id_source_role_key ON app.steward_scope_sources USING btree (assessment_id, source_role);

CREATE INDEX steward_scope_sources_external_idx ON app.steward_scope_sources USING btree (external_observation_id) WHERE (external_observation_id IS NOT NULL);

CREATE INDEX steward_scope_sources_knowledge_idx ON app.steward_scope_sources USING btree (knowledge_version_id) WHERE (knowledge_version_id IS NOT NULL);

CREATE UNIQUE INDEX steward_scope_sources_pkey ON app.steward_scope_sources USING btree (id);

CREATE INDEX steward_scope_sources_project_idx ON app.steward_scope_sources USING btree (project_id);

CREATE INDEX steward_scope_sources_publication_idx ON app.steward_scope_sources USING btree (publication_observation_id) WHERE (publication_observation_id IS NOT NULL);

CREATE INDEX steward_scope_sources_source_project_idx ON app.steward_scope_sources USING btree (source_project_id);

CREATE INDEX steward_scope_sources_workspace_idx ON app.steward_scope_sources USING btree (workspace_id);

CREATE UNIQUE INDEX work_tool_connections_id_workspace_id_key ON app.work_tool_connections USING btree (id, workspace_id);

CREATE UNIQUE INDEX work_tool_connections_pkey ON app.work_tool_connections USING btree (id);

CREATE UNIQUE INDEX work_tool_connections_public_id_key ON app.work_tool_connections USING btree (public_id);

CREATE INDEX work_tool_connections_workspace_idx ON app.work_tool_connections USING btree (workspace_id);

CREATE INDEX workspace_invitations_creator_idx ON app.workspace_invitations USING btree (created_by_actor_id);

CREATE UNIQUE INDEX workspace_invitations_pkey ON app.workspace_invitations USING btree (id);

CREATE UNIQUE INDEX workspace_invitations_public_id_key ON app.workspace_invitations USING btree (public_id);

CREATE INDEX workspace_invitations_scope_idx ON app.workspace_invitations USING btree (workspace_id, status, expires_at);

alter table "app"."publication_jobs" add constraint "publication_jobs_pkey" PRIMARY KEY using index "publication_jobs_pkey";

alter table "app"."publication_observations" add constraint "publication_observations_pkey" PRIMARY KEY using index "publication_observations_pkey";

alter table "app"."steward_scope_sources" add constraint "steward_scope_sources_pkey" PRIMARY KEY using index "steward_scope_sources_pkey";

alter table "app"."work_tool_connections" add constraint "work_tool_connections_pkey" PRIMARY KEY using index "work_tool_connections_pkey";

alter table "app"."workspace_invitations" add constraint "workspace_invitations_pkey" PRIMARY KEY using index "workspace_invitations_pkey";

alter table "app"."publication_jobs" add constraint "publication_jobs_artifact_version_id_project_id_fkey" FOREIGN KEY (artifact_version_id, project_id) REFERENCES app.artifact_document_versions(id, project_id) not valid;

alter table "app"."publication_jobs" validate constraint "publication_jobs_artifact_version_id_project_id_fkey";

alter table "app"."publication_jobs" add constraint "publication_jobs_artifact_version_id_workspace_id_fkey" FOREIGN KEY (artifact_version_id, workspace_id) REFERENCES app.artifact_document_versions(id, workspace_id) not valid;

alter table "app"."publication_jobs" validate constraint "publication_jobs_artifact_version_id_workspace_id_fkey";

alter table "app"."publication_jobs" add constraint "publication_jobs_attempt_count_check" CHECK (((attempt_count >= 0) AND (attempt_count <= 1))) not valid;

alter table "app"."publication_jobs" validate constraint "publication_jobs_attempt_count_check";

alter table "app"."publication_jobs" add constraint "publication_jobs_body_markdown_check" CHECK ((octet_length(body_markdown) <= 61440)) not valid;

alter table "app"."publication_jobs" validate constraint "publication_jobs_body_markdown_check";

alter table "app"."publication_jobs" add constraint "publication_jobs_check" CHECK (((external_id IS NULL) = (external_url IS NULL))) not valid;

alter table "app"."publication_jobs" validate constraint "publication_jobs_check";

alter table "app"."publication_jobs" add constraint "publication_jobs_check1" CHECK (((status <> ALL (ARRAY['succeeded'::text, 'conflict'::text, 'unavailable'::text])) OR (external_id IS NOT NULL))) not valid;

alter table "app"."publication_jobs" validate constraint "publication_jobs_check1";

alter table "app"."publication_jobs" add constraint "publication_jobs_connection_id_workspace_id_fkey" FOREIGN KEY (connection_id, workspace_id) REFERENCES app.work_tool_connections(id, workspace_id) not valid;

alter table "app"."publication_jobs" validate constraint "publication_jobs_connection_id_workspace_id_fkey";

alter table "app"."publication_jobs" add constraint "publication_jobs_content_hash_check" CHECK ((content_hash ~ '^[0-9a-f]{64}$'::text)) not valid;

alter table "app"."publication_jobs" validate constraint "publication_jobs_content_hash_check";

alter table "app"."publication_jobs" add constraint "publication_jobs_error_code_check" CHECK ((length(error_code) <= 80)) not valid;

alter table "app"."publication_jobs" validate constraint "publication_jobs_error_code_check";

alter table "app"."publication_jobs" add constraint "publication_jobs_id_workspace_id_key" UNIQUE using index "publication_jobs_id_workspace_id_key";

alter table "app"."publication_jobs" add constraint "publication_jobs_project_id_workspace_id_fkey" FOREIGN KEY (project_id, workspace_id) REFERENCES app.projects(id, workspace_id) not valid;

alter table "app"."publication_jobs" validate constraint "publication_jobs_project_id_workspace_id_fkey";

alter table "app"."publication_jobs" add constraint "publication_jobs_provider_check" CHECK ((provider = ANY (ARRAY['notion'::text, 'linear'::text, 'github'::text]))) not valid;

alter table "app"."publication_jobs" validate constraint "publication_jobs_provider_check";

alter table "app"."publication_jobs" add constraint "publication_jobs_public_id_key" UNIQUE using index "publication_jobs_public_id_key";

alter table "app"."publication_jobs" add constraint "publication_jobs_status_check" CHECK ((status = ANY (ARRAY['queued'::text, 'processing'::text, 'succeeded'::text, 'failed'::text, 'needs_review'::text, 'conflict'::text, 'unavailable'::text, 'cancelled'::text]))) not valid;

alter table "app"."publication_jobs" validate constraint "publication_jobs_status_check";

alter table "app"."publication_jobs" add constraint "publication_jobs_target_id_check" CHECK (((length(target_id) >= 1) AND (length(target_id) <= 256))) not valid;

alter table "app"."publication_jobs" validate constraint "publication_jobs_target_id_check";

alter table "app"."publication_jobs" add constraint "publication_jobs_title_check" CHECK (((length(title) >= 1) AND (length(title) <= 200))) not valid;

alter table "app"."publication_jobs" validate constraint "publication_jobs_title_check";

alter table "app"."publication_jobs" add constraint "publication_jobs_workspace_id_artifact_version_id_provider__key" UNIQUE using index "publication_jobs_workspace_id_artifact_version_id_provider__key";

alter table "app"."publication_observations" add constraint "publication_observations_external_id_check" CHECK (((length(external_id) >= 1) AND (length(external_id) <= 256))) not valid;

alter table "app"."publication_observations" validate constraint "publication_observations_external_id_check";

alter table "app"."publication_observations" add constraint "publication_observations_external_url_check" CHECK ((length(external_url) <= 2048)) not valid;

alter table "app"."publication_observations" validate constraint "publication_observations_external_url_check";

alter table "app"."publication_observations" add constraint "publication_observations_id_workspace_id_key" UNIQUE using index "publication_observations_id_workspace_id_key";

alter table "app"."publication_observations" add constraint "publication_observations_observation_kind_check" CHECK ((observation_kind = ANY (ARRAY['created'::text, 'reconciled'::text, 'unchanged'::text, 'changed'::text, 'unavailable'::text]))) not valid;

alter table "app"."publication_observations" validate constraint "publication_observations_observation_kind_check";

alter table "app"."publication_observations" add constraint "publication_observations_public_id_key" UNIQUE using index "publication_observations_public_id_key";

alter table "app"."publication_observations" add constraint "publication_observations_publication_job_id_workspace_id_fkey" FOREIGN KEY (publication_job_id, workspace_id) REFERENCES app.publication_jobs(id, workspace_id) not valid;

alter table "app"."publication_observations" validate constraint "publication_observations_publication_job_id_workspace_id_fkey";

alter table "app"."publication_observations" add constraint "publication_observations_snapshot_check" CHECK (((jsonb_typeof(snapshot) = 'object'::text) AND (octet_length((snapshot)::text) <= 262144))) not valid;

alter table "app"."publication_observations" validate constraint "publication_observations_snapshot_check";

alter table "app"."steward_scope_sources" add constraint "steward_scope_sources_artifact_version_id_source_project_i_fkey" FOREIGN KEY (artifact_version_id, source_project_id) REFERENCES app.artifact_document_versions(id, project_id) not valid;

alter table "app"."steward_scope_sources" validate constraint "steward_scope_sources_artifact_version_id_source_project_i_fkey";

alter table "app"."steward_scope_sources" add constraint "steward_scope_sources_assessment_id_project_id_fkey" FOREIGN KEY (assessment_id, project_id) REFERENCES app.steward_assessments(id, project_id) not valid;

alter table "app"."steward_scope_sources" validate constraint "steward_scope_sources_assessment_id_project_id_fkey";

alter table "app"."steward_scope_sources" add constraint "steward_scope_sources_assessment_id_source_role_key" UNIQUE using index "steward_scope_sources_assessment_id_source_role_key";

alter table "app"."steward_scope_sources" add constraint "steward_scope_sources_check" CHECK ((num_nonnulls(knowledge_version_id, artifact_version_id, external_observation_id, publication_observation_id) = 1)) not valid;

alter table "app"."steward_scope_sources" validate constraint "steward_scope_sources_check";

alter table "app"."steward_scope_sources" add constraint "steward_scope_sources_check1" CHECK (((source_kind = 'knowledge_entry_version'::text) = (knowledge_version_id IS NOT NULL))) not valid;

alter table "app"."steward_scope_sources" validate constraint "steward_scope_sources_check1";

alter table "app"."steward_scope_sources" add constraint "steward_scope_sources_check2" CHECK (((source_kind = 'artifact_document_version'::text) = (artifact_version_id IS NOT NULL))) not valid;

alter table "app"."steward_scope_sources" validate constraint "steward_scope_sources_check2";

alter table "app"."steward_scope_sources" add constraint "steward_scope_sources_check3" CHECK (((source_kind = 'external_reference_observation'::text) = (external_observation_id IS NOT NULL))) not valid;

alter table "app"."steward_scope_sources" validate constraint "steward_scope_sources_check3";

alter table "app"."steward_scope_sources" add constraint "steward_scope_sources_check4" CHECK (((source_kind = 'publication_observation'::text) = (publication_observation_id IS NOT NULL))) not valid;

alter table "app"."steward_scope_sources" validate constraint "steward_scope_sources_check4";

alter table "app"."steward_scope_sources" add constraint "steward_scope_sources_external_observation_id_source_proje_fkey" FOREIGN KEY (external_observation_id, source_project_id) REFERENCES app.external_reference_observations(id, project_id) not valid;

alter table "app"."steward_scope_sources" validate constraint "steward_scope_sources_external_observation_id_source_proje_fkey";

alter table "app"."steward_scope_sources" add constraint "steward_scope_sources_knowledge_version_id_source_project__fkey" FOREIGN KEY (knowledge_version_id, source_project_id) REFERENCES app.knowledge_entry_versions(id, project_id) not valid;

alter table "app"."steward_scope_sources" validate constraint "steward_scope_sources_knowledge_version_id_source_project__fkey";

alter table "app"."steward_scope_sources" add constraint "steward_scope_sources_project_id_workspace_id_fkey" FOREIGN KEY (project_id, workspace_id) REFERENCES app.projects(id, workspace_id) not valid;

alter table "app"."steward_scope_sources" validate constraint "steward_scope_sources_project_id_workspace_id_fkey";

alter table "app"."steward_scope_sources" add constraint "steward_scope_sources_publication_observation_id_workspace_fkey" FOREIGN KEY (publication_observation_id, workspace_id) REFERENCES app.publication_observations(id, workspace_id) not valid;

alter table "app"."steward_scope_sources" validate constraint "steward_scope_sources_publication_observation_id_workspace_fkey";

alter table "app"."steward_scope_sources" add constraint "steward_scope_sources_source_kind_check" CHECK ((source_kind = ANY (ARRAY['knowledge_entry_version'::text, 'artifact_document_version'::text, 'external_reference_observation'::text, 'publication_observation'::text]))) not valid;

alter table "app"."steward_scope_sources" validate constraint "steward_scope_sources_source_kind_check";

alter table "app"."steward_scope_sources" add constraint "steward_scope_sources_source_project_id_workspace_id_fkey" FOREIGN KEY (source_project_id, workspace_id) REFERENCES app.projects(id, workspace_id) not valid;

alter table "app"."steward_scope_sources" validate constraint "steward_scope_sources_source_project_id_workspace_id_fkey";

alter table "app"."steward_scope_sources" add constraint "steward_scope_sources_source_role_check" CHECK ((source_role = ANY (ARRAY['left'::text, 'right'::text]))) not valid;

alter table "app"."steward_scope_sources" validate constraint "steward_scope_sources_source_role_check";

alter table "app"."steward_scope_sources" add constraint "steward_scope_sources_source_snapshot_check" CHECK (((jsonb_typeof(source_snapshot) = 'object'::text) AND (octet_length((source_snapshot)::text) <= 65536))) not valid;

alter table "app"."steward_scope_sources" validate constraint "steward_scope_sources_source_snapshot_check";

alter table "app"."steward_scope_sources" add constraint "steward_scope_sources_workspace_id_fkey" FOREIGN KEY (workspace_id) REFERENCES app.workspaces(id) not valid;

alter table "app"."steward_scope_sources" validate constraint "steward_scope_sources_workspace_id_fkey";

alter table "app"."work_tool_connections" add constraint "work_tool_connections_encrypted_credential_check" CHECK (((octet_length(encrypted_credential) >= 30) AND (octet_length(encrypted_credential) <= 8192))) not valid;

alter table "app"."work_tool_connections" validate constraint "work_tool_connections_encrypted_credential_check";

alter table "app"."work_tool_connections" add constraint "work_tool_connections_id_workspace_id_key" UNIQUE using index "work_tool_connections_id_workspace_id_key";

alter table "app"."work_tool_connections" add constraint "work_tool_connections_name_check" CHECK (((length(btrim(name)) >= 1) AND (length(btrim(name)) <= 120))) not valid;

alter table "app"."work_tool_connections" validate constraint "work_tool_connections_name_check";

alter table "app"."work_tool_connections" add constraint "work_tool_connections_provider_check" CHECK ((provider = ANY (ARRAY['notion'::text, 'linear'::text, 'github'::text]))) not valid;

alter table "app"."work_tool_connections" validate constraint "work_tool_connections_provider_check";

alter table "app"."work_tool_connections" add constraint "work_tool_connections_public_id_key" UNIQUE using index "work_tool_connections_public_id_key";

alter table "app"."work_tool_connections" add constraint "work_tool_connections_revision_check" CHECK ((revision > 0)) not valid;

alter table "app"."work_tool_connections" validate constraint "work_tool_connections_revision_check";

alter table "app"."work_tool_connections" add constraint "work_tool_connections_workspace_id_fkey" FOREIGN KEY (workspace_id) REFERENCES app.workspaces(id) not valid;

alter table "app"."work_tool_connections" validate constraint "work_tool_connections_workspace_id_fkey";

alter table "app"."workspace_invitations" add constraint "workspace_invitations_check" CHECK (((expires_at > created_at) AND (expires_at <= (created_at + '7 days'::interval)))) not valid;

alter table "app"."workspace_invitations" validate constraint "workspace_invitations_check";

alter table "app"."workspace_invitations" add constraint "workspace_invitations_check1" CHECK (((status = 'accepted'::text) = ((accepted_by_actor_id IS NOT NULL) AND (accepted_at IS NOT NULL)))) not valid;

alter table "app"."workspace_invitations" validate constraint "workspace_invitations_check1";

alter table "app"."workspace_invitations" add constraint "workspace_invitations_check2" CHECK (((status = 'revoked'::text) = (revoked_at IS NOT NULL))) not valid;

alter table "app"."workspace_invitations" validate constraint "workspace_invitations_check2";

alter table "app"."workspace_invitations" add constraint "workspace_invitations_label_check" CHECK ((length(label) <= 120)) not valid;

alter table "app"."workspace_invitations" validate constraint "workspace_invitations_label_check";

alter table "app"."workspace_invitations" add constraint "workspace_invitations_public_id_key" UNIQUE using index "workspace_invitations_public_id_key";

alter table "app"."workspace_invitations" add constraint "workspace_invitations_request_hash_check" CHECK ((request_hash ~ '^[0-9a-f]{64}$'::text)) not valid;

alter table "app"."workspace_invitations" validate constraint "workspace_invitations_request_hash_check";

alter table "app"."workspace_invitations" add constraint "workspace_invitations_role_check" CHECK ((role = ANY (ARRAY['editor'::text, 'viewer'::text]))) not valid;

alter table "app"."workspace_invitations" validate constraint "workspace_invitations_role_check";

alter table "app"."workspace_invitations" add constraint "workspace_invitations_status_check" CHECK ((status = ANY (ARRAY['pending'::text, 'accepted'::text, 'revoked'::text]))) not valid;

alter table "app"."workspace_invitations" validate constraint "workspace_invitations_status_check";

alter table "app"."workspace_invitations" add constraint "workspace_invitations_token_hash_check" CHECK ((token_hash ~ '^[0-9a-f]{64}$'::text)) not valid;

alter table "app"."workspace_invitations" validate constraint "workspace_invitations_token_hash_check";

alter table "app"."workspace_invitations" add constraint "workspace_invitations_workspace_id_fkey" FOREIGN KEY (workspace_id) REFERENCES app.workspaces(id) not valid;

alter table "app"."workspace_invitations" validate constraint "workspace_invitations_workspace_id_fkey";

alter table "app"."workspace_members" add constraint "workspace_members_display_name_check" CHECK ((length(display_name) <= 80)) not valid;

alter table "app"."workspace_members" validate constraint "workspace_members_display_name_check";

alter table "app"."insights" add constraint "insights_type_valid" CHECK ((insight_type = ANY (ARRAY['contradiction'::text, 'coverage_gap'::text, 'context_gap'::text]))) not valid;

alter table "app"."insights" validate constraint "insights_type_valid";

set check_function_bodies = off;

CREATE OR REPLACE FUNCTION app.accept_workspace_invitation(requested_id uuid, presented_hash text, requested_display_name text)
 RETURNS TABLE(public_id uuid, name text, role text)
 LANGUAGE plpgsql
 SECURITY DEFINER
 SET search_path TO ''
AS $function$
declare invited app.workspace_invitations%rowtype; actor uuid:=app.current_actor_id(); member_role text;
begin
  if actor is null or actor='00000000-0000-0000-0000-000000000000'::uuid then
    raise exception 'authentication required' using errcode='42501';
  end if;
  if requested_display_name is null or length(btrim(requested_display_name)) not between 1 and 80 then
    raise exception 'display name required' using errcode='22023';
  end if;
  select * into invited from app.workspace_invitations i where i.public_id=requested_id and i.token_hash=presented_hash;
  if invited.id is null then raise exception 'invitation unavailable' using errcode='P0002'; end if;
  -- Serialize with owner/member changes, then recheck the inviting authority.
  perform id from app.workspaces where id=invited.workspace_id for update;
  select * into strict invited from app.workspace_invitations i where i.id=invited.id for update;
  if invited.status='accepted' and invited.accepted_by_actor_id=actor then
    select m.role into member_role from app.workspace_members m
      where m.workspace_id=invited.workspace_id and m.actor_id=actor and m.invitation_status='accepted';
    if member_role is null then raise exception 'invitation unavailable' using errcode='P0002'; end if;
    return query select w.public_id,w.name,member_role from app.workspaces w where w.id=invited.workspace_id;
    return;
  end if;
  if invited.status<>'pending' or invited.expires_at<=now() or not exists(
    select 1 from app.workspace_members m where m.workspace_id=invited.workspace_id
      and m.actor_id=invited.created_by_actor_id and m.role='owner' and m.invitation_status='accepted'
  ) then raise exception 'invitation unavailable' using errcode='P0002'; end if;
  -- An already active member keeps their existing role; accepting an old link
  -- must not demote an owner or overwrite a deliberate role change.
  insert into app.workspace_members(workspace_id,actor_id,role,invitation_status,invited_by_actor_id,accepted_at,display_name)
  values(invited.workspace_id,actor,invited.role,'accepted',invited.created_by_actor_id,now(),btrim(requested_display_name))
  on conflict(workspace_id,actor_id) do update set
    role=case when app.workspace_members.invitation_status='accepted' then app.workspace_members.role else excluded.role end,
    invitation_status='accepted',invited_by_actor_id=excluded.invited_by_actor_id,
    accepted_at=coalesce(app.workspace_members.accepted_at,now()),display_name=excluded.display_name;
  update app.workspace_invitations set status='accepted',accepted_by_actor_id=actor,accepted_at=now() where id=invited.id;
  insert into app.audit_events(workspace_id,actor_id,action,object_kind,object_public_id,after_state)
  values(invited.workspace_id,actor,'team.invitation.accepted','workspace_invitation',invited.public_id,jsonb_build_object('role',invited.role));
  return query select w.public_id,w.name,m.role from app.workspaces w join app.workspace_members m on m.workspace_id=w.id
    where w.id=invited.workspace_id and m.actor_id=actor and m.invitation_status='accepted';
end;
$function$
;

CREATE OR REPLACE FUNCTION app.claim_publication_job()
 RETURNS TABLE(job_id uuid, workspace_id bigint, workspace_public_id uuid, actor_id uuid, actor_role text, lease uuid)
 LANGUAGE plpgsql
 SECURITY DEFINER
 SET search_path TO ''
 SET row_security TO 'off'
AS $function$
declare claimed app.publication_jobs; token uuid=gen_random_uuid();
begin
  update app.publication_jobs set status='needs_review',error_code='worker_lease_expired',updated_at=now()
    where id in (select j.id from app.publication_jobs j where j.status='processing' and j.lease_until<now()
      order by j.lease_until limit 100 for update skip locked);
  select * into claimed from app.publication_jobs j where j.status='queued' and j.attempt_count=0
    order by j.created_at,j.id limit 1 for update skip locked;
  if not found then return; end if;
  update app.publication_jobs set status='processing',attempt_count=1,lease_token=token,
    lease_until=now()+interval '2 minutes',updated_at=now() where id=claimed.id;
  return query select claimed.public_id,claimed.workspace_id,w.public_id,claimed.requested_by_actor_id,
    coalesce((select m.role from app.workspace_members m where m.workspace_id=claimed.workspace_id
      and m.actor_id=claimed.requested_by_actor_id and m.invitation_status='accepted'),'revoked'),token
    from app.workspaces w where w.id=claimed.workspace_id;
end;
$function$
;

CREATE OR REPLACE FUNCTION app.external_observed_context_event()
 RETURNS trigger
 LANGUAGE plpgsql
 SET search_path TO ''
AS $function$
begin
 if not exists(select 1 from app.projects where workspace_id=new.workspace_id and scope_kind='company') then return new; end if;
 update app.projects set graph_version=graph_version+1 where id=new.project_id;
 insert into app.domain_events(workspace_id,project_id,event_type,aggregate_kind,aggregate_public_id,payload)
 values(new.workspace_id,new.project_id,'external_reference.observed','external_reference_observation',new.public_id,
   jsonb_build_object('observation_public_id',new.public_id,'status',new.observation_status));
 return new;
end;
$function$
;

CREATE OR REPLACE FUNCTION app.finish_publication_job(requested_job uuid, requested_lease uuid, new_status text, safe_error text, receipt jsonb)
 RETURNS boolean
 LANGUAGE plpgsql
 SECURITY DEFINER
 SET search_path TO ''
 SET row_security TO 'off'
AS $function$
declare job app.publication_jobs;
begin
  if new_status not in ('succeeded','failed','needs_review','cancelled') or length(safe_error)>80 then
    raise exception 'Invalid publication settlement' using errcode='23514';
  end if;
  select * into job from app.publication_jobs where public_id=requested_job and lease_token=requested_lease
    and status in ('processing','needs_review') for update;
  if not found then return false; end if;
  if new_status='succeeded' then
    if receipt is null or jsonb_typeof(receipt)<>'object' or coalesce(length(receipt->>'external_id'),0) not between 1 and 256
      or coalesce(length(receipt->>'external_url'),0) not between 1 and 2048 or octet_length(receipt::text)>262144 then
      raise exception 'Invalid publication receipt' using errcode='23514';
    end if;
    insert into app.publication_observations(workspace_id,publication_job_id,observation_kind,external_id,external_url,remote_updated_at,snapshot)
      values(job.workspace_id,job.id,'created',receipt->>'external_id',receipt->>'external_url',receipt->>'remote_updated_at',receipt);
  end if;
  update app.publication_jobs set status=new_status,error_code=safe_error,
    external_id=case when new_status='succeeded' then receipt->>'external_id' else external_id end,
    external_url=case when new_status='succeeded' then receipt->>'external_url' else external_url end,
    updated_at=now() where id=job.id;
  return true;
end;
$function$
;

CREATE OR REPLACE FUNCTION app.preview_workspace_invitation(requested_id uuid, presented_hash text)
 RETURNS TABLE(public_id uuid, workspace_public_id uuid, company_name text, role text, expires_at timestamp with time zone)
 LANGUAGE sql
 STABLE SECURITY DEFINER
 SET search_path TO ''
AS $function$
  select i.public_id,w.public_id,w.name,i.role,i.expires_at
  from app.workspace_invitations i join app.workspaces w on w.id=i.workspace_id
  where i.public_id=requested_id and i.token_hash=presented_hash and i.status='pending' and i.expires_at>now()
    and exists(select 1 from app.workspace_members m where m.workspace_id=i.workspace_id
      and m.actor_id=i.created_by_actor_id and m.role='owner' and m.invitation_status='accepted');
$function$
;

CREATE OR REPLACE FUNCTION app.publication_observed_context_event()
 RETURNS trigger
 LANGUAGE plpgsql
 SET search_path TO ''
AS $function$
declare job app.publication_jobs;
begin
 select * into job from app.publication_jobs where id=new.publication_job_id;
 update app.projects set graph_version=graph_version+1 where id=job.project_id;
 insert into app.domain_events(workspace_id,project_id,event_type,aggregate_kind,aggregate_public_id,payload,requested_by_actor_id)
 values(job.workspace_id,job.project_id,'publication.observed','publication_observation',new.public_id,
   jsonb_build_object('observation_public_id',new.public_id,'observation_kind',new.observation_kind),job.requested_by_actor_id);
 return new;
end;
$function$
;

CREATE OR REPLACE FUNCTION app.publication_preserve_receipt()
 RETURNS trigger
 LANGUAGE plpgsql
 SET search_path TO ''
AS $function$
begin
  if old.external_id is not null and (new.external_id is distinct from old.external_id
    or new.external_url is distinct from old.external_url) then
    raise exception 'A canonical publication receipt cannot be replaced' using errcode='23514';
  end if;
  return new;
end;
$function$
;

CREATE OR REPLACE FUNCTION app.set_member_display_name(requested_name text)
 RETURNS uuid
 LANGUAGE plpgsql
 SECURITY DEFINER
 SET search_path TO ''
AS $function$
declare member_public_id uuid;
begin
  if requested_name is null or length(btrim(requested_name)) not between 1 and 80 then
    raise exception 'display name required' using errcode='22023';
  end if;
  update app.workspace_members set display_name=btrim(requested_name)
    where workspace_id=app.current_workspace_id() and actor_id=app.current_actor_id() and invitation_status='accepted'
    returning public_id into member_public_id;
  if member_public_id is null then raise exception 'accepted membership required' using errcode='42501'; end if;
  return member_public_id;
end;
$function$
;

CREATE OR REPLACE FUNCTION app.steward_scope_source_status(requested_assessment_id bigint)
 RETURNS text
 LANGUAGE sql
 STABLE
 SET search_path TO ''
AS $function$
 select case when not exists(select 1 from app.steward_scope_sources where assessment_id=requested_assessment_id) then null
   when not app.steward_scope_sources_current(requested_assessment_id) then 'stale'
   when exists(select 1 from app.steward_scope_sources where assessment_id=requested_assessment_id and source_snapshot->>'read_status'='insufficient') then 'unknown'
   else 'current' end;
$function$
;

CREATE OR REPLACE FUNCTION app.steward_scope_sources_current(requested_assessment_id bigint)
 RETURNS boolean
 LANGUAGE sql
 STABLE
 SET search_path TO ''
AS $function$
 select not exists(select 1 from app.steward_scope_sources s
   left join app.projects p on p.id=s.source_project_id
   left join app.knowledge_entry_versions v on v.id=s.knowledge_version_id
   left join app.knowledge_entries k on k.id=v.knowledge_entry_id
   left join app.artifact_document_versions a on a.id=s.artifact_version_id
   left join app.external_reference_observations e on e.id=s.external_observation_id
   left join app.publication_observations o on o.id=s.publication_observation_id
   where s.assessment_id=requested_assessment_id and (p.id is null or p.status<>'active'
     or (s.knowledge_version_id is not null and (v.version_number<>k.latest_version or k.status<>'confirmed'))
     or (s.artifact_version_id is not null and exists(select 1 from app.artifact_document_versions newer where newer.document_id=a.document_id and newer.status='validated' and newer.version>a.version))
     or (s.external_observation_id is not null and exists(select 1 from app.external_reference_observations newer where newer.external_reference_id=e.external_reference_id and newer.id>e.id))
     or (s.publication_observation_id is not null and exists(select 1 from app.publication_observations newer where newer.publication_job_id=o.publication_job_id and newer.id>o.id))));
$function$
;

CREATE OR REPLACE FUNCTION app.validate_steward_scope_source()
 RETURNS trigger
 LANGUAGE plpgsql
 SET search_path TO ''
AS $function$
declare actual_id uuid; actual_project bigint;
begin
 case new.source_kind
 when 'knowledge_entry_version' then select public_id,project_id into actual_id,actual_project from app.knowledge_entry_versions where id=new.knowledge_version_id;
 when 'artifact_document_version' then select public_id,project_id into actual_id,actual_project from app.artifact_document_versions where id=new.artifact_version_id and status='validated';
 when 'external_reference_observation' then select public_id,project_id into actual_id,actual_project from app.external_reference_observations where id=new.external_observation_id;
 when 'publication_observation' then select o.public_id,j.project_id into actual_id,actual_project from app.publication_observations o join app.publication_jobs j on j.id=o.publication_job_id where o.id=new.publication_observation_id;
 end case;
 if actual_id is null or actual_id<>new.source_public_id or actual_project<>new.source_project_id then
   raise exception 'Steward source does not match its immutable scope and version' using errcode='23514';
 end if;
 return new;
end;
$function$
;

CREATE OR REPLACE FUNCTION app.graph_endpoint_exists(endpoint_kind text, endpoint_id uuid, scope_id bigint, tenant_id bigint)
 RETURNS boolean
 LANGUAGE plpgsql
 STABLE
 SET search_path TO ''
AS $function$
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
    when 'task' then return exists(select 1 from app.tasks where public_id=endpoint_id and project_id=scope_id and workspace_id=tenant_id);
    when 'execution' then return exists(select 1 from app.executions where public_id=endpoint_id and project_id=scope_id and workspace_id=tenant_id);
    when 'insight' then return exists(select 1 from app.insights where public_id=endpoint_id and project_id=scope_id and workspace_id=tenant_id);
    else return false;
  end case;
end;
$function$
;

CREATE OR REPLACE FUNCTION app.list_due_steward_workspaces(requested_limit integer DEFAULT 16)
 RETURNS TABLE(workspace_id bigint, workspace_public_id uuid, actor_id uuid, workspace_role text)
 LANGUAGE sql
 SECURITY DEFINER ROWS 128
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
    where event.event_type in ('knowledge.committed','knowledge.revised','artifact.validated','graph.relationship_confirmed','external_reference.observed','publication.observed')
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
$function$
;

grant insert on table "app"."publication_jobs" to "ai_center_runtime";

grant select on table "app"."publication_jobs" to "ai_center_runtime";

grant insert on table "app"."publication_observations" to "ai_center_runtime";

grant select on table "app"."publication_observations" to "ai_center_runtime";

grant insert on table "app"."steward_scope_sources" to "ai_center_runtime";

grant select on table "app"."steward_scope_sources" to "ai_center_runtime";

grant insert on table "app"."work_tool_connections" to "ai_center_runtime";

grant select on table "app"."work_tool_connections" to "ai_center_runtime";

grant insert on table "app"."workspace_invitations" to "ai_center_runtime";

grant select on table "app"."workspace_invitations" to "ai_center_runtime";


  create policy "publications_editor_insert"
  on "app"."publication_jobs"
  as permissive
  for insert
  to public
with check (((workspace_id = ( SELECT app.current_workspace_id() AS current_workspace_id)) AND app.has_workspace_role(workspace_id, ARRAY['owner'::text, 'editor'::text]) AND (requested_by_actor_id = app.current_actor_id()) AND (status = 'queued'::text) AND (attempt_count = 0) AND (lease_token IS NULL) AND (external_id IS NULL)));



  create policy "publications_editor_update"
  on "app"."publication_jobs"
  as permissive
  for update
  to public
using (((workspace_id = ( SELECT app.current_workspace_id() AS current_workspace_id)) AND app.has_workspace_role(workspace_id, ARRAY['owner'::text, 'editor'::text])))
with check (((workspace_id = ( SELECT app.current_workspace_id() AS current_workspace_id)) AND app.has_workspace_role(workspace_id, ARRAY['owner'::text, 'editor'::text])));



  create policy "work_tools_member_select"
  on "app"."publication_jobs"
  as permissive
  for select
  to public
using (((workspace_id = ( SELECT app.current_workspace_id() AS current_workspace_id)) AND app.has_workspace_role(workspace_id, ARRAY['owner'::text, 'editor'::text, 'viewer'::text])));



  create policy "observations_editor_insert"
  on "app"."publication_observations"
  as permissive
  for insert
  to public
with check (((workspace_id = ( SELECT app.current_workspace_id() AS current_workspace_id)) AND app.has_workspace_role(workspace_id, ARRAY['owner'::text, 'editor'::text])));



  create policy "work_tools_member_select"
  on "app"."publication_observations"
  as permissive
  for select
  to public
using (((workspace_id = ( SELECT app.current_workspace_id() AS current_workspace_id)) AND app.has_workspace_role(workspace_id, ARRAY['owner'::text, 'editor'::text, 'viewer'::text])));



  create policy "steward_scope_sources_read"
  on "app"."steward_scope_sources"
  as permissive
  for select
  to public
using (((workspace_id = app.current_workspace_id()) AND app.has_workspace_role(workspace_id, ARRAY['owner'::text, 'editor'::text, 'viewer'::text])));



  create policy "steward_scope_sources_write"
  on "app"."steward_scope_sources"
  as permissive
  for insert
  to public
with check (((workspace_id = app.current_workspace_id()) AND app.has_workspace_role(workspace_id, ARRAY['owner'::text, 'editor'::text])));



  create policy "work_tools_member_select"
  on "app"."work_tool_connections"
  as permissive
  for select
  to public
using (((workspace_id = ( SELECT app.current_workspace_id() AS current_workspace_id)) AND app.has_workspace_role(workspace_id, ARRAY['owner'::text, 'editor'::text, 'viewer'::text])));



  create policy "work_tools_owner_write"
  on "app"."work_tool_connections"
  as permissive
  for all
  to public
using (((workspace_id = ( SELECT app.current_workspace_id() AS current_workspace_id)) AND app.has_workspace_role(workspace_id, ARRAY['owner'::text])))
with check (((workspace_id = ( SELECT app.current_workspace_id() AS current_workspace_id)) AND app.has_workspace_role(workspace_id, ARRAY['owner'::text])));



  create policy "invitation_owner_insert"
  on "app"."workspace_invitations"
  as permissive
  for insert
  to public
with check (((workspace_id = ( SELECT app.current_workspace_id() AS current_workspace_id)) AND (created_by_actor_id = ( SELECT app.current_actor_id() AS current_actor_id)) AND app.has_workspace_role(workspace_id, ARRAY['owner'::text])));



  create policy "invitation_owner_read"
  on "app"."workspace_invitations"
  as permissive
  for select
  to public
using (((workspace_id = ( SELECT app.current_workspace_id() AS current_workspace_id)) AND app.has_workspace_role(workspace_id, ARRAY['owner'::text])));



  create policy "invitation_owner_update"
  on "app"."workspace_invitations"
  as permissive
  for update
  to public
using (((workspace_id = ( SELECT app.current_workspace_id() AS current_workspace_id)) AND app.has_workspace_role(workspace_id, ARRAY['owner'::text])))
with check (((workspace_id = ( SELECT app.current_workspace_id() AS current_workspace_id)) AND app.has_workspace_role(workspace_id, ARRAY['owner'::text])));


CREATE TRIGGER external_observed_context AFTER INSERT ON app.external_reference_observations FOR EACH ROW EXECUTE FUNCTION app.external_observed_context_event();

CREATE TRIGGER publication_jobs_preserve_receipt BEFORE UPDATE ON app.publication_jobs FOR EACH ROW EXECUTE FUNCTION app.publication_preserve_receipt();

CREATE TRIGGER publication_observations_immutable BEFORE DELETE OR UPDATE ON app.publication_observations FOR EACH ROW EXECUTE FUNCTION app.prevent_append_only_mutation();

CREATE TRIGGER publication_observed_context AFTER INSERT ON app.publication_observations FOR EACH ROW EXECUTE FUNCTION app.publication_observed_context_event();

CREATE TRIGGER steward_scope_source_identity BEFORE INSERT ON app.steward_scope_sources FOR EACH ROW EXECUTE FUNCTION app.validate_steward_scope_source();

CREATE TRIGGER steward_scope_sources_immutable BEFORE DELETE OR UPDATE ON app.steward_scope_sources FOR EACH ROW EXECUTE FUNCTION app.prevent_append_only_mutation();



-- Privilege metadata and FORCE RLS are not fully emitted by migra.

revoke all on function app.publication_preserve_receipt() from public,anon,authenticated,service_role;
revoke all on function app.claim_publication_job() from public,anon,authenticated,service_role;
revoke all on function app.finish_publication_job(uuid,uuid,text,text,jsonb) from public,anon,authenticated,service_role;
grant select,insert on app.work_tool_connections,app.publication_jobs,app.publication_observations to ai_center_runtime;
grant update(name,encrypted_credential,credential_actor_id,enabled,revision,updated_at) on app.work_tool_connections to ai_center_runtime;
grant update(status,error_code,external_id,external_url,updated_at) on app.publication_jobs to ai_center_runtime;
grant usage on sequence app.work_tool_connections_id_seq,app.publication_jobs_id_seq,app.publication_observations_id_seq to ai_center_runtime;
grant execute on function app.claim_publication_job(),app.finish_publication_job(uuid,uuid,text,text,jsonb) to ai_center_runtime;

revoke all on app.workspace_invitations from public,anon,authenticated,service_role;
grant select,insert on app.workspace_invitations to ai_center_runtime;
grant update(status,revoked_at) on app.workspace_invitations to ai_center_runtime;
grant usage on sequence app.workspace_invitations_id_seq to ai_center_runtime;
revoke all on function app.preview_workspace_invitation(uuid,text) from public,anon,authenticated,service_role;
grant execute on function app.preview_workspace_invitation(uuid,text) to ai_center_runtime;
revoke all on function app.accept_workspace_invitation(uuid,text,text) from public,anon,authenticated,service_role;
grant execute on function app.accept_workspace_invitation(uuid,text,text) to ai_center_runtime;
revoke all on function app.set_member_display_name(text) from public,anon,authenticated,service_role;
grant execute on function app.set_member_display_name(text) to ai_center_runtime;
alter table app.workspace_invitations force row level security;

revoke all on app.steward_scope_sources from public,anon,authenticated,service_role;
grant select,insert on app.steward_scope_sources to ai_center_runtime;
grant usage on sequence app.steward_scope_sources_id_seq to ai_center_runtime;
revoke all on function app.validate_steward_scope_source() from public,anon,authenticated,service_role;
revoke all on function app.steward_scope_sources_current(bigint) from public,anon,authenticated,service_role;
grant execute on function app.steward_scope_sources_current(bigint) to ai_center_runtime;
revoke all on function app.publication_observed_context_event() from public,anon,authenticated,service_role;
revoke all on function app.external_observed_context_event() from public,anon,authenticated,service_role;
revoke all on function app.list_due_steward_workspaces(integer) from public,anon,authenticated,service_role;
grant execute on function app.list_due_steward_workspaces(integer) to ai_center_runtime;
revoke all on function app.steward_scope_source_status(bigint) from public,anon,authenticated,service_role;
grant execute on function app.steward_scope_source_status(bigint) to ai_center_runtime;
revoke all on function app.graph_endpoint_exists(text,uuid,bigint,bigint) from public,anon,authenticated,service_role;
grant execute on function app.graph_endpoint_exists(text,uuid,bigint,bigint) to ai_center_runtime;
alter table app.steward_scope_sources force row level security;

-- Schema06 uses a DO loop for RLS; preserve its FORCE flags explicitly.
alter table app.work_tool_connections force row level security;
alter table app.publication_jobs force row level security;
alter table app.publication_observations force row level security;
revoke all on app.work_tool_connections from public,anon,authenticated,service_role;
revoke all on app.publication_jobs from public,anon,authenticated,service_role;
revoke all on app.publication_observations from public,anon,authenticated,service_role;
