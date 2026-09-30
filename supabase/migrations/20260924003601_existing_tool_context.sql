alter table "app"."artifact_version_sources" drop constraint "artifact_version_sources_check";

alter table "app"."artifact_version_sources" drop constraint "artifact_version_sources_source_kind_check";

alter table "app"."context_pack_scope_sources" drop constraint "context_pack_scope_sources_check";

alter table "app"."context_pack_scope_sources" drop constraint "context_pack_scope_sources_source_kind_check";

alter table "app"."steward_scope_sources" drop constraint "steward_scope_sources_check";

alter table "app"."steward_scope_sources" drop constraint "steward_scope_sources_source_kind_check";


  create table "app"."tool_source_observations" (
    "id" bigint generated always as identity not null,
    "public_id" uuid not null default gen_random_uuid(),
    "workspace_id" bigint not null,
    "project_id" bigint not null,
    "reference_id" bigint not null,
    "version" integer not null,
    "provider" text not null,
    "object_kind" text not null,
    "external_id" uuid not null,
    "canonical_url" text not null,
    "connection_id" bigint not null,
    "connection_revision" integer not null,
    "observed_at" timestamp with time zone not null default now(),
    "remote_updated_at" timestamp with time zone,
    "title" text not null,
    "body_markdown" text not null,
    "availability" text not null,
    "coverage" text not null,
    "omission_reasons" jsonb not null,
    "projection_version" text not null,
    "content_hash" text not null,
    "snapshot_hash" text not null,
    "metadata" jsonb not null
      );


alter table "app"."tool_source_observations" enable row level security;


  create table "app"."tool_source_references" (
    "id" bigint generated always as identity not null,
    "public_id" uuid not null default gen_random_uuid(),
    "workspace_id" bigint not null,
    "project_id" bigint not null,
    "provider" text not null,
    "object_kind" text not null,
    "external_id" uuid not null,
    "canonical_url" text not null,
    "connection_id" bigint not null,
    "connection_revision" integer not null,
    "created_by_actor_id" uuid not null,
    "created_at" timestamp with time zone not null default now(),
    "updated_at" timestamp with time zone not null default now(),
    "status" text not null default 'active'::text,
    "revision" integer not null default 1,
    "current_observation_id" bigint,
    "last_attempt_at" timestamp with time zone,
    "last_checked_at" timestamp with time zone,
    "last_check_status" text,
    "last_check_error_code" text
      );


alter table "app"."tool_source_references" enable row level security;

alter table "app"."artifact_version_sources" add column "publication_observation_id" bigint;

alter table "app"."artifact_version_sources" add column "tool_source_observation_id" bigint;

alter table "app"."context_pack_scope_sources" add column "publication_observation_id" bigint;

alter table "app"."context_pack_scope_sources" add column "tool_source_observation_id" bigint;

alter table "app"."publication_observations" add column "connection_id" bigint;

alter table "app"."publication_observations" add column "connection_revision" integer;

alter table "app"."steward_scope_sources" add column "tool_source_observation_id" bigint;

alter table "app"."work_tool_connections" add column "allow_existing_reads" boolean not null default false;

CREATE INDEX artifact_sources_publication_idx ON app.artifact_version_sources USING btree (publication_observation_id) WHERE (publication_observation_id IS NOT NULL);

CREATE INDEX artifact_sources_tool_idx ON app.artifact_version_sources USING btree (tool_source_observation_id) WHERE (tool_source_observation_id IS NOT NULL);

CREATE INDEX audit_remote_read_window_idx ON app.audit_events USING btree (workspace_id, action, occurred_at DESC) WHERE (action ~~ 'work_tool.remote_read.%'::text);

CREATE INDEX pack_sources_publication_idx ON app.context_pack_scope_sources USING btree (publication_observation_id) WHERE (publication_observation_id IS NOT NULL);

CREATE INDEX pack_sources_tool_idx ON app.context_pack_scope_sources USING btree (tool_source_observation_id) WHERE (tool_source_observation_id IS NOT NULL);

CREATE INDEX publication_observation_connection_idx ON app.publication_observations USING btree (connection_id) WHERE (connection_id IS NOT NULL);

CREATE INDEX steward_sources_tool_idx ON app.steward_scope_sources USING btree (tool_source_observation_id) WHERE (tool_source_observation_id IS NOT NULL);

CREATE INDEX tool_source_observations_connection_idx ON app.tool_source_observations USING btree (connection_id);

CREATE UNIQUE INDEX tool_source_observations_id_project_id_key ON app.tool_source_observations USING btree (id, project_id);

CREATE UNIQUE INDEX tool_source_observations_id_reference_id_key ON app.tool_source_observations USING btree (id, reference_id);

CREATE UNIQUE INDEX tool_source_observations_id_workspace_id_key ON app.tool_source_observations USING btree (id, workspace_id);

CREATE UNIQUE INDEX tool_source_observations_pkey ON app.tool_source_observations USING btree (id);

CREATE INDEX tool_source_observations_project_idx ON app.tool_source_observations USING btree (project_id);

CREATE UNIQUE INDEX tool_source_observations_public_id_key ON app.tool_source_observations USING btree (public_id);

CREATE UNIQUE INDEX tool_source_observations_reference_id_version_key ON app.tool_source_observations USING btree (reference_id, version);

CREATE INDEX tool_source_observations_workspace_idx ON app.tool_source_observations USING btree (workspace_id);

CREATE UNIQUE INDEX tool_source_references_id_project_id_key ON app.tool_source_references USING btree (id, project_id);

CREATE UNIQUE INDEX tool_source_references_id_workspace_id_key ON app.tool_source_references USING btree (id, workspace_id);

CREATE UNIQUE INDEX tool_source_references_pkey ON app.tool_source_references USING btree (id);

CREATE UNIQUE INDEX tool_source_references_public_id_key ON app.tool_source_references USING btree (public_id);

CREATE UNIQUE INDEX tool_source_references_workspace_id_project_id_provider_ext_key ON app.tool_source_references USING btree (workspace_id, project_id, provider, external_id);

CREATE INDEX tool_sources_connection_idx ON app.tool_source_references USING btree (connection_id);

CREATE INDEX tool_sources_head_idx ON app.tool_source_references USING btree (current_observation_id);

CREATE INDEX tool_sources_project_idx ON app.tool_source_references USING btree (project_id, status, created_at DESC, id DESC);

CREATE INDEX tool_sources_workspace_idx ON app.tool_source_references USING btree (workspace_id);

alter table "app"."tool_source_observations" add constraint "tool_source_observations_pkey" PRIMARY KEY using index "tool_source_observations_pkey";

alter table "app"."tool_source_references" add constraint "tool_source_references_pkey" PRIMARY KEY using index "tool_source_references_pkey";

alter table "app"."artifact_version_sources" add constraint "artifact_sources_publication_scope_fkey" FOREIGN KEY (publication_observation_id, workspace_id) REFERENCES app.publication_observations(id, workspace_id) not valid;

alter table "app"."artifact_version_sources" validate constraint "artifact_sources_publication_scope_fkey";

alter table "app"."artifact_version_sources" add constraint "artifact_sources_tool_scope_fkey" FOREIGN KEY (tool_source_observation_id, source_project_id) REFERENCES app.tool_source_observations(id, project_id) not valid;

alter table "app"."artifact_version_sources" validate constraint "artifact_sources_tool_scope_fkey";

alter table "app"."context_pack_scope_sources" add constraint "pack_sources_publication_scope_fkey" FOREIGN KEY (publication_observation_id, workspace_id) REFERENCES app.publication_observations(id, workspace_id) not valid;

alter table "app"."context_pack_scope_sources" validate constraint "pack_sources_publication_scope_fkey";

alter table "app"."context_pack_scope_sources" add constraint "pack_sources_tool_scope_fkey" FOREIGN KEY (tool_source_observation_id, source_project_id) REFERENCES app.tool_source_observations(id, project_id) not valid;

alter table "app"."context_pack_scope_sources" validate constraint "pack_sources_tool_scope_fkey";

alter table "app"."publication_observations" add constraint "publication_observation_attestation_check" CHECK ((((connection_id IS NULL) AND (connection_revision IS NULL)) OR ((connection_id IS NOT NULL) AND (connection_revision IS NOT NULL) AND (connection_revision > 0)))) not valid;

alter table "app"."publication_observations" validate constraint "publication_observation_attestation_check";

alter table "app"."publication_observations" add constraint "publication_observation_connection_fkey" FOREIGN KEY (connection_id, workspace_id) REFERENCES app.work_tool_connections(id, workspace_id) not valid;

alter table "app"."publication_observations" validate constraint "publication_observation_connection_fkey";

alter table "app"."steward_scope_sources" add constraint "steward_scope_sources_tool_kind_check" CHECK (((source_kind = 'tool_source_observation'::text) = (tool_source_observation_id IS NOT NULL))) not valid;

alter table "app"."steward_scope_sources" validate constraint "steward_scope_sources_tool_kind_check";

alter table "app"."steward_scope_sources" add constraint "steward_sources_tool_scope_fkey" FOREIGN KEY (tool_source_observation_id, source_project_id) REFERENCES app.tool_source_observations(id, project_id) not valid;

alter table "app"."steward_scope_sources" validate constraint "steward_sources_tool_scope_fkey";

alter table "app"."tool_source_observations" add constraint "tool_source_observations_availability_check" CHECK ((availability = ANY (ARRAY['available'::text, 'unavailable'::text]))) not valid;

alter table "app"."tool_source_observations" validate constraint "tool_source_observations_availability_check";

alter table "app"."tool_source_observations" add constraint "tool_source_observations_canonical_url_check" CHECK (((octet_length(canonical_url) >= 1) AND (octet_length(canonical_url) <= 2048))) not valid;

alter table "app"."tool_source_observations" validate constraint "tool_source_observations_canonical_url_check";

alter table "app"."tool_source_observations" add constraint "tool_source_observations_check" CHECK ((((provider = 'linear'::text) AND (object_kind = 'issue'::text)) OR ((provider = 'notion'::text) AND (object_kind = 'page'::text)))) not valid;

alter table "app"."tool_source_observations" validate constraint "tool_source_observations_check";

alter table "app"."tool_source_observations" add constraint "tool_source_observations_check1" CHECK (((octet_length(title) + octet_length(body_markdown)) <= 65536)) not valid;

alter table "app"."tool_source_observations" validate constraint "tool_source_observations_check1";

alter table "app"."tool_source_observations" add constraint "tool_source_observations_check2" CHECK (((((octet_length((metadata)::text) + octet_length((omission_reasons)::text)) + octet_length(title)) + octet_length(body_markdown)) <= 131072)) not valid;

alter table "app"."tool_source_observations" validate constraint "tool_source_observations_check2";

alter table "app"."tool_source_observations" add constraint "tool_source_observations_check3" CHECK ((((availability = 'unavailable'::text) AND (body_markdown = ''::text) AND (coverage = 'none'::text)) OR ((availability = 'available'::text) AND (coverage = ANY (ARRAY['complete'::text, 'partial'::text]))))) not valid;

alter table "app"."tool_source_observations" validate constraint "tool_source_observations_check3";

alter table "app"."tool_source_observations" add constraint "tool_source_observations_connection_id_workspace_id_fkey" FOREIGN KEY (connection_id, workspace_id) REFERENCES app.work_tool_connections(id, workspace_id) not valid;

alter table "app"."tool_source_observations" validate constraint "tool_source_observations_connection_id_workspace_id_fkey";

alter table "app"."tool_source_observations" add constraint "tool_source_observations_connection_revision_check" CHECK ((connection_revision > 0)) not valid;

alter table "app"."tool_source_observations" validate constraint "tool_source_observations_connection_revision_check";

alter table "app"."tool_source_observations" add constraint "tool_source_observations_content_hash_check" CHECK ((content_hash ~ '^[0-9a-f]{64}$'::text)) not valid;

alter table "app"."tool_source_observations" validate constraint "tool_source_observations_content_hash_check";

alter table "app"."tool_source_observations" add constraint "tool_source_observations_coverage_check" CHECK ((coverage = ANY (ARRAY['complete'::text, 'partial'::text, 'none'::text]))) not valid;

alter table "app"."tool_source_observations" validate constraint "tool_source_observations_coverage_check";

alter table "app"."tool_source_observations" add constraint "tool_source_observations_id_project_id_key" UNIQUE using index "tool_source_observations_id_project_id_key";

alter table "app"."tool_source_observations" add constraint "tool_source_observations_id_reference_id_key" UNIQUE using index "tool_source_observations_id_reference_id_key";

alter table "app"."tool_source_observations" add constraint "tool_source_observations_id_workspace_id_key" UNIQUE using index "tool_source_observations_id_workspace_id_key";

alter table "app"."tool_source_observations" add constraint "tool_source_observations_metadata_check" CHECK ((jsonb_typeof(metadata) = 'object'::text)) not valid;

alter table "app"."tool_source_observations" validate constraint "tool_source_observations_metadata_check";

alter table "app"."tool_source_observations" add constraint "tool_source_observations_omission_reasons_check" CHECK ((jsonb_typeof(omission_reasons) = 'array'::text)) not valid;

alter table "app"."tool_source_observations" validate constraint "tool_source_observations_omission_reasons_check";

alter table "app"."tool_source_observations" add constraint "tool_source_observations_project_id_workspace_id_fkey" FOREIGN KEY (project_id, workspace_id) REFERENCES app.projects(id, workspace_id) not valid;

alter table "app"."tool_source_observations" validate constraint "tool_source_observations_project_id_workspace_id_fkey";

alter table "app"."tool_source_observations" add constraint "tool_source_observations_projection_version_check" CHECK (((length(projection_version) >= 1) AND (length(projection_version) <= 80))) not valid;

alter table "app"."tool_source_observations" validate constraint "tool_source_observations_projection_version_check";

alter table "app"."tool_source_observations" add constraint "tool_source_observations_public_id_key" UNIQUE using index "tool_source_observations_public_id_key";

alter table "app"."tool_source_observations" add constraint "tool_source_observations_reference_id_project_id_fkey" FOREIGN KEY (reference_id, project_id) REFERENCES app.tool_source_references(id, project_id) not valid;

alter table "app"."tool_source_observations" validate constraint "tool_source_observations_reference_id_project_id_fkey";

alter table "app"."tool_source_observations" add constraint "tool_source_observations_reference_id_version_key" UNIQUE using index "tool_source_observations_reference_id_version_key";

alter table "app"."tool_source_observations" add constraint "tool_source_observations_reference_id_workspace_id_fkey" FOREIGN KEY (reference_id, workspace_id) REFERENCES app.tool_source_references(id, workspace_id) not valid;

alter table "app"."tool_source_observations" validate constraint "tool_source_observations_reference_id_workspace_id_fkey";

alter table "app"."tool_source_observations" add constraint "tool_source_observations_snapshot_hash_check" CHECK ((snapshot_hash ~ '^[0-9a-f]{64}$'::text)) not valid;

alter table "app"."tool_source_observations" validate constraint "tool_source_observations_snapshot_hash_check";

alter table "app"."tool_source_observations" add constraint "tool_source_observations_version_check" CHECK ((version > 0)) not valid;

alter table "app"."tool_source_observations" validate constraint "tool_source_observations_version_check";

alter table "app"."tool_source_references" add constraint "tool_source_head_reference_fkey" FOREIGN KEY (current_observation_id, id) REFERENCES app.tool_source_observations(id, reference_id) DEFERRABLE INITIALLY DEFERRED not valid;

alter table "app"."tool_source_references" validate constraint "tool_source_head_reference_fkey";

alter table "app"."tool_source_references" add constraint "tool_source_references_canonical_url_check" CHECK (((octet_length(canonical_url) >= 1) AND (octet_length(canonical_url) <= 2048))) not valid;

alter table "app"."tool_source_references" validate constraint "tool_source_references_canonical_url_check";

alter table "app"."tool_source_references" add constraint "tool_source_references_check" CHECK ((((provider = 'linear'::text) AND (object_kind = 'issue'::text)) OR ((provider = 'notion'::text) AND (object_kind = 'page'::text)))) not valid;

alter table "app"."tool_source_references" validate constraint "tool_source_references_check";

alter table "app"."tool_source_references" add constraint "tool_source_references_connection_id_workspace_id_fkey" FOREIGN KEY (connection_id, workspace_id) REFERENCES app.work_tool_connections(id, workspace_id) not valid;

alter table "app"."tool_source_references" validate constraint "tool_source_references_connection_id_workspace_id_fkey";

alter table "app"."tool_source_references" add constraint "tool_source_references_connection_revision_check" CHECK ((connection_revision > 0)) not valid;

alter table "app"."tool_source_references" validate constraint "tool_source_references_connection_revision_check";

alter table "app"."tool_source_references" add constraint "tool_source_references_id_project_id_key" UNIQUE using index "tool_source_references_id_project_id_key";

alter table "app"."tool_source_references" add constraint "tool_source_references_id_workspace_id_key" UNIQUE using index "tool_source_references_id_workspace_id_key";

alter table "app"."tool_source_references" add constraint "tool_source_references_last_check_error_code_check" CHECK (((length(last_check_error_code) >= 1) AND (length(last_check_error_code) <= 80))) not valid;

alter table "app"."tool_source_references" validate constraint "tool_source_references_last_check_error_code_check";

alter table "app"."tool_source_references" add constraint "tool_source_references_last_check_status_check" CHECK ((last_check_status = ANY (ARRAY['available'::text, 'partial'::text, 'unavailable'::text, 'failed'::text]))) not valid;

alter table "app"."tool_source_references" validate constraint "tool_source_references_last_check_status_check";

alter table "app"."tool_source_references" add constraint "tool_source_references_project_id_workspace_id_fkey" FOREIGN KEY (project_id, workspace_id) REFERENCES app.projects(id, workspace_id) not valid;

alter table "app"."tool_source_references" validate constraint "tool_source_references_project_id_workspace_id_fkey";

alter table "app"."tool_source_references" add constraint "tool_source_references_public_id_key" UNIQUE using index "tool_source_references_public_id_key";

alter table "app"."tool_source_references" add constraint "tool_source_references_revision_check" CHECK ((revision > 0)) not valid;

alter table "app"."tool_source_references" validate constraint "tool_source_references_revision_check";

alter table "app"."tool_source_references" add constraint "tool_source_references_status_check" CHECK ((status = ANY (ARRAY['active'::text, 'detached'::text]))) not valid;

alter table "app"."tool_source_references" validate constraint "tool_source_references_status_check";

alter table "app"."tool_source_references" add constraint "tool_source_references_workspace_id_project_id_provider_ext_key" UNIQUE using index "tool_source_references_workspace_id_project_id_provider_ext_key";

alter table "app"."artifact_version_sources" add constraint "artifact_version_sources_check" CHECK (((num_nonnulls(knowledge_version_id, context_pack_id, deliverable_id, session_id, source_artifact_version_id, tool_source_observation_id, publication_observation_id) = 1) AND ((source_kind = 'knowledge'::text) = (knowledge_version_id IS NOT NULL)) AND ((source_kind = 'context_pack'::text) = (context_pack_id IS NOT NULL)) AND ((source_kind = 'deliverable'::text) = (deliverable_id IS NOT NULL)) AND ((source_kind = 'session'::text) = (session_id IS NOT NULL)) AND ((source_kind = 'artifact_version'::text) = (source_artifact_version_id IS NOT NULL)) AND ((source_kind = 'tool_source_observation'::text) = (tool_source_observation_id IS NOT NULL)) AND ((source_kind = 'publication_observation'::text) = (publication_observation_id IS NOT NULL)))) not valid;

alter table "app"."artifact_version_sources" validate constraint "artifact_version_sources_check";

alter table "app"."artifact_version_sources" add constraint "artifact_version_sources_source_kind_check" CHECK ((source_kind = ANY (ARRAY['knowledge'::text, 'context_pack'::text, 'deliverable'::text, 'session'::text, 'artifact_version'::text, 'tool_source_observation'::text, 'publication_observation'::text]))) not valid;

alter table "app"."artifact_version_sources" validate constraint "artifact_version_sources_source_kind_check";

alter table "app"."context_pack_scope_sources" add constraint "context_pack_scope_sources_check" CHECK (((num_nonnulls(knowledge_version_id, artifact_version_id, tool_source_observation_id, publication_observation_id) = 1) AND ((source_kind = 'knowledge_entry_version'::text) = (knowledge_version_id IS NOT NULL)) AND ((source_kind = 'artifact_document_version'::text) = (artifact_version_id IS NOT NULL)) AND ((source_kind = 'tool_source_observation'::text) = (tool_source_observation_id IS NOT NULL)) AND ((source_kind = 'publication_observation'::text) = (publication_observation_id IS NOT NULL)) AND ((source_kind <> ALL (ARRAY['tool_source_observation'::text, 'publication_observation'::text])) OR (NOT is_mandatory)))) not valid;

alter table "app"."context_pack_scope_sources" validate constraint "context_pack_scope_sources_check";

alter table "app"."context_pack_scope_sources" add constraint "context_pack_scope_sources_source_kind_check" CHECK ((source_kind = ANY (ARRAY['knowledge_entry_version'::text, 'artifact_document_version'::text, 'tool_source_observation'::text, 'publication_observation'::text]))) not valid;

alter table "app"."context_pack_scope_sources" validate constraint "context_pack_scope_sources_source_kind_check";

alter table "app"."steward_scope_sources" add constraint "steward_scope_sources_check" CHECK ((num_nonnulls(knowledge_version_id, artifact_version_id, external_observation_id, publication_observation_id, github_code_file_observation_id, tool_source_observation_id) = 1)) not valid;

alter table "app"."steward_scope_sources" validate constraint "steward_scope_sources_check";

alter table "app"."steward_scope_sources" add constraint "steward_scope_sources_source_kind_check" CHECK ((source_kind = ANY (ARRAY['knowledge_entry_version'::text, 'artifact_document_version'::text, 'external_reference_observation'::text, 'publication_observation'::text, 'github_code_file_observation'::text, 'tool_source_observation'::text]))) not valid;

alter table "app"."steward_scope_sources" validate constraint "steward_scope_sources_source_kind_check";

set check_function_bodies = off;

CREATE OR REPLACE FUNCTION app.context_utf8_prefix(value text, max_bytes integer)
 RETURNS text
 LANGUAGE plpgsql
 IMMUTABLE STRICT
 SET search_path TO ''
AS $function$
declare low_bound integer=0; high_bound integer=length(value); midpoint integer;
begin
 if max_bytes<0 then raise exception 'Invalid text budget' using errcode='22023'; end if;
 if octet_length(value)<=max_bytes then return value; end if;
 while low_bound<high_bound loop
  midpoint=(low_bound+high_bound+1)/2;
  if octet_length(left(value,midpoint))<=max_bytes then low_bound=midpoint; else high_bound=midpoint-1; end if;
 end loop;
 return left(value,low_bound);
end;
$function$
;

CREATE OR REPLACE FUNCTION app.publication_observation_current(observation_id bigint)
 RETURNS boolean
 LANGUAGE sql
 STABLE
 SET search_path TO ''
AS $function$
 select exists(select 1 from app.publication_source_observations o
 join app.projects p on p.id=o.project_id and p.workspace_id=o.workspace_id
 join app.work_tool_connections c on c.id=o.authority_connection_id and c.workspace_id=o.workspace_id and c.provider=o.provider
 where o.id=observation_id and o.workspace_id=app.current_workspace_id() and o.is_current
  and o.external_id is not null and o.availability='available' and p.status='active'
  and c.enabled and c.revision=o.authority_connection_revision);
$function$
;

create or replace view "app"."publication_source_observations" as  WITH base AS (
         SELECT o.id,
            o.public_id,
            o.workspace_id,
            j.project_id,
            o.publication_job_id,
            j.public_id AS publication_public_id,
            j.provider,
                CASE
                    WHEN (j.provider = 'notion'::text) THEN 'page'::text
                    ELSE 'issue'::text
                END AS object_kind,
                CASE
                    WHEN (o.external_id ~* '^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$'::text) THEN (o.external_id)::uuid
                    ELSE NULL::uuid
                END AS external_id,
            o.external_url AS canonical_url,
            o.observed_at,
            o.remote_updated_at,
                CASE
                    WHEN (o.observation_kind = 'unavailable'::text) THEN o.external_id
                    ELSE app.context_utf8_prefix(COALESCE((o.snapshot ->> 'title'::text), j.title), 4096)
                END AS title,
                CASE
                    WHEN (o.observation_kind = 'unavailable'::text) THEN ''::text
                    ELSE app.context_utf8_prefix(COALESCE((o.snapshot ->> 'body_markdown'::text), ''::text), 61440)
                END AS body_markdown,
                CASE
                    WHEN (o.observation_kind = 'unavailable'::text) THEN 'unavailable'::text
                    ELSE 'available'::text
                END AS availability,
                CASE
                    WHEN (o.observation_kind = 'unavailable'::text) THEN 'none'::text
                    WHEN (((o.snapshot ->> 'complete'::text) = 'true'::text) AND (octet_length(COALESCE((o.snapshot ->> 'body_markdown'::text), ''::text)) <= 61440) AND (octet_length(COALESCE((o.snapshot ->> 'title'::text), j.title)) <= 4096)) THEN 'complete'::text
                    ELSE 'partial'::text
                END AS coverage,
                CASE
                    WHEN (o.observation_kind = 'unavailable'::text) THEN '[]'::jsonb
                    ELSE ((((jsonb_build_array('comments_not_read', 'attachments_not_read', 'related_objects_not_read') ||
                    CASE
                        WHEN (j.provider = 'notion'::text) THEN jsonb_build_array('properties_not_read', 'embedded_content_not_read', 'transcripts_not_read')
                        ELSE '[]'::jsonb
                    END) ||
                    CASE
                        WHEN (COALESCE((o.snapshot ->> 'complete'::text), 'false'::text) <> 'true'::text) THEN jsonb_build_array('provider_truncated')
                        ELSE '[]'::jsonb
                    END) ||
                    CASE
                        WHEN ((octet_length(COALESCE((o.snapshot ->> 'body_markdown'::text), ''::text)) > 61440) OR (octet_length(COALESCE((o.snapshot ->> 'title'::text), j.title)) > 4096)) THEN jsonb_build_array('local_text_limit')
                        ELSE '[]'::jsonb
                    END) ||
                    CASE
                        WHEN (o.remote_updated_at IS NULL) THEN jsonb_build_array('remote_date_unavailable')
                        ELSE '[]'::jsonb
                    END)
                END AS omission_reasons,
            'publication-text-v1'::text AS projection_version,
            '{}'::jsonb AS metadata,
            o.connection_id,
            o.connection_revision
           FROM (app.publication_observations o
             JOIN app.publication_jobs j ON (((j.id = o.publication_job_id) AND (j.workspace_id = o.workspace_id))))
          WHERE (j.provider = ANY (ARRAY['notion'::text, 'linear'::text]))
        ), signatures AS (
         SELECT b.id,
            b.public_id,
            b.workspace_id,
            b.project_id,
            b.publication_job_id,
            b.publication_public_id,
            b.provider,
            b.object_kind,
            b.external_id,
            b.canonical_url,
            b.observed_at,
            b.remote_updated_at,
            b.title,
            b.body_markdown,
            b.availability,
            b.coverage,
            b.omission_reasons,
            b.projection_version,
            b.metadata,
            b.connection_id,
            b.connection_revision,
            jsonb_build_object('title', b.title, 'body_markdown', b.body_markdown) AS business,
            jsonb_build_object('provider', b.provider, 'external_id', b.external_id, 'canonical_url', b.canonical_url, 'availability', b.availability, 'coverage', b.coverage, 'omission_reasons', b.omission_reasons, 'projection_version', b.projection_version, 'metadata', b.metadata) AS evidence
           FROM base b
        ), boundaries AS (
         SELECT s.id,
            s.public_id,
            s.workspace_id,
            s.project_id,
            s.publication_job_id,
            s.publication_public_id,
            s.provider,
            s.object_kind,
            s.external_id,
            s.canonical_url,
            s.observed_at,
            s.remote_updated_at,
            s.title,
            s.body_markdown,
            s.availability,
            s.coverage,
            s.omission_reasons,
            s.projection_version,
            s.metadata,
            s.connection_id,
            s.connection_revision,
            s.business,
            s.evidence,
                CASE
                    WHEN (NOT (lag((s.business || s.evidence)) OVER (PARTITION BY s.publication_job_id ORDER BY s.id) IS DISTINCT FROM (s.business || s.evidence))) THEN 0
                    ELSE 1
                END AS boundary
           FROM signatures s
        ), groups AS (
         SELECT b.id,
            b.public_id,
            b.workspace_id,
            b.project_id,
            b.publication_job_id,
            b.publication_public_id,
            b.provider,
            b.object_kind,
            b.external_id,
            b.canonical_url,
            b.observed_at,
            b.remote_updated_at,
            b.title,
            b.body_markdown,
            b.availability,
            b.coverage,
            b.omission_reasons,
            b.projection_version,
            b.metadata,
            b.connection_id,
            b.connection_revision,
            b.business,
            b.evidence,
            b.boundary,
            (sum(b.boundary) OVER (PARTITION BY b.publication_job_id ORDER BY b.id))::integer AS version
           FROM boundaries b
        ), identified AS (
         SELECT g.id,
            g.public_id,
            g.workspace_id,
            g.project_id,
            g.publication_job_id,
            g.publication_public_id,
            g.provider,
            g.object_kind,
            g.external_id,
            g.canonical_url,
            g.observed_at,
            g.remote_updated_at,
            g.title,
            g.body_markdown,
            g.availability,
            g.coverage,
            g.omission_reasons,
            g.projection_version,
            g.metadata,
            g.connection_id,
            g.connection_revision,
            g.business,
            g.evidence,
            g.boundary,
            g.version,
            first_value(g.id) OVER (PARTITION BY g.publication_job_id, g.version ORDER BY g.id) AS canonical_observation_id,
            first_value(g.public_id) OVER (PARTITION BY g.publication_job_id, g.version ORDER BY g.id) AS canonical_observation_public_id,
            max(g.id) OVER (PARTITION BY g.publication_job_id, g.version) AS group_latest_id,
            max(g.id) OVER (PARTITION BY g.publication_job_id) AS latest_observation_id
           FROM groups g
        )
 SELECT i.id,
    i.public_id,
    i.workspace_id,
    i.project_id,
    i.publication_job_id,
    i.publication_public_id,
    i.provider,
    i.object_kind,
    i.external_id,
    i.canonical_url,
    i.version,
    i.canonical_observation_id,
    i.canonical_observation_public_id,
    i.latest_observation_id,
    (i.group_latest_id = i.latest_observation_id) AS is_current,
    i.observed_at,
    i.remote_updated_at,
    i.title,
    i.body_markdown,
    i.availability,
    i.coverage,
    i.omission_reasons,
    encode(sha256(convert_to((i.business)::text, 'UTF8'::name)), 'hex'::text) AS content_hash,
    encode(sha256(convert_to(((i.business || i.evidence))::text, 'UTF8'::name)), 'hex'::text) AS snapshot_hash,
    i.projection_version,
    i.metadata,
    authority.connection_id AS authority_connection_id,
    c.public_id AS authority_connection_public_id,
    authority.connection_revision AS authority_connection_revision
   FROM ((identified i
     JOIN app.publication_observations authority ON ((authority.id = i.group_latest_id)))
     LEFT JOIN app.work_tool_connections c ON (((c.id = authority.connection_id) AND (c.workspace_id = i.workspace_id))));


CREATE OR REPLACE FUNCTION app.tool_source_context_event()
 RETURNS trigger
 LANGUAGE plpgsql
 SET search_path TO ''
AS $function$
begin
 if tg_op='INSERT' and new.current_observation_id is null then return new; end if;
 if tg_op='UPDATE' and (new.current_observation_id,new.status,new.connection_id,new.connection_revision)
  is not distinct from (old.current_observation_id,old.status,old.connection_id,old.connection_revision) then return new; end if;
 update app.projects set graph_version=graph_version+1 where id=new.project_id;
 with changed as (
  update app.context_packs p set status='stale',invalidated_at=now(),stale_reason='included external source changed or became unavailable'
  where p.workspace_id=new.workspace_id and p.status='current' and exists(
   select 1 from app.context_pack_scope_sources s join app.tool_source_observations o on o.id=s.tool_source_observation_id
   where s.context_pack_id=p.id and s.decision='included' and o.reference_id=new.id and not app.tool_source_observation_current(o.id)) returning p.id
 ) update app.deliverables d set status='stale',stale_at=now() where d.source_context_pack_id in(select id from changed) and d.status in('draft','committed');
 insert into app.domain_events(workspace_id,project_id,event_type,aggregate_kind,aggregate_public_id,payload,requested_by_actor_id)
 values(new.workspace_id,new.project_id,'tool_source.observed','tool_source_reference',new.public_id,
  jsonb_build_object('reference_public_id',new.public_id,'status',new.status),app.current_actor_id());
 return new;
end;
$function$
;

CREATE OR REPLACE FUNCTION app.tool_source_observation_current(observation_id bigint)
 RETURNS boolean
 LANGUAGE sql
 STABLE
 SET search_path TO ''
AS $function$
 select exists(select 1 from app.tool_source_observations o
 join app.tool_source_references r on r.id=o.reference_id and r.workspace_id=o.workspace_id and r.project_id=o.project_id
 join app.projects p on p.id=r.project_id and p.workspace_id=r.workspace_id
 join app.work_tool_connections c on c.id=r.connection_id and c.workspace_id=r.workspace_id and c.provider=r.provider
 where o.id=observation_id and o.workspace_id=app.current_workspace_id()
  and r.current_observation_id=o.id and r.status='active' and p.status='active'
  and o.availability='available' and c.enabled and c.allow_existing_reads and c.revision=r.connection_revision);
$function$
;

CREATE OR REPLACE FUNCTION app.validate_external_source_provenance()
 RETURNS trigger
 LANGUAGE plpgsql
 SET search_path TO ''
AS $function$
declare actual_uuid uuid; actual_project bigint;
begin
 case new.source_kind
 when 'tool_source_observation' then select public_id,project_id into actual_uuid,actual_project from app.tool_source_observations where id=new.tool_source_observation_id and workspace_id=new.workspace_id;
 when 'publication_observation' then select o.public_id,j.project_id into actual_uuid,actual_project from app.publication_observations o join app.publication_jobs j on j.id=o.publication_job_id where o.id=new.publication_observation_id and o.workspace_id=new.workspace_id;
 else return new;
 end case;
 if actual_uuid is distinct from new.source_public_id or actual_project is distinct from new.source_project_id then
  raise exception 'External source must identify the exact observation and scope' using errcode='23514';
 end if;
 return new;
end;
$function$
;

CREATE OR REPLACE FUNCTION app.validate_tool_source_head()
 RETURNS trigger
 LANGUAGE plpgsql
 SET search_path TO ''
AS $function$
declare reference app.tool_source_references; head app.tool_source_observations; target_reference_id bigint;
begin
 if tg_table_name='tool_source_observations' then target_reference_id=new.reference_id; else target_reference_id=new.id; end if;
 select * into reference from app.tool_source_references where id=target_reference_id;
 if not found then return new; end if;
 select * into head from app.tool_source_observations where id=reference.current_observation_id and reference_id=reference.id;
 if not found or exists(select 1 from app.tool_source_observations where reference_id=reference.id and version>head.version) then
  raise exception 'A source reference requires its latest exact observation' using errcode='23514';
 end if;
 return new;
end;
$function$
;

CREATE OR REPLACE FUNCTION app.validate_tool_source_identity()
 RETURNS trigger
 LANGUAGE plpgsql
 SET search_path TO ''
AS $function$
declare reference app.tool_source_references; connection_provider text;
begin
 if tg_table_name='tool_source_references' then
  if tg_op='UPDATE' and (new.public_id,new.workspace_id,new.project_id,new.provider,new.object_kind,new.external_id,new.created_by_actor_id,new.created_at)
    is distinct from (old.public_id,old.workspace_id,old.project_id,old.provider,old.object_kind,old.external_id,old.created_by_actor_id,old.created_at) then
   raise exception 'A source reference identity is immutable' using errcode='23514';
  end if;
 else
  select * into reference from app.tool_source_references where id=new.reference_id;
  if not found or (new.workspace_id,new.project_id,new.provider,new.object_kind,new.external_id)
    is distinct from (reference.workspace_id,reference.project_id,reference.provider,reference.object_kind,reference.external_id) then
   raise exception 'Observation does not match its reference identity' using errcode='23514';
  end if;
 end if;
 select provider into connection_provider from app.work_tool_connections where id=new.connection_id and workspace_id=new.workspace_id;
 if connection_provider is distinct from new.provider then
  raise exception 'Observation connection provider does not match' using errcode='23514';
 end if;
 return new;
end;
$function$
;

CREATE OR REPLACE FUNCTION app.context_pack_scopes_current(requested_pack_id bigint)
 RETURNS boolean
 LANGUAGE sql
 STABLE
 SET search_path TO ''
AS $function$
  select exists(select 1 from app.context_packs p where p.id=requested_pack_id and p.workspace_id=app.current_workspace_id())
    -- Graph stamps are receipts and optimistic concurrency guards, not a
    -- blanket invalidation rule. Only included exact sources govern freshness.
    and not exists(select 1 from app.context_pack_scope_versions s left join app.projects p on p.id=s.source_project_id
      where s.context_pack_id=requested_pack_id and (p.id is null or p.status<>'active'))
    and not exists(select 1 from app.context_pack_sources s
      join app.knowledge_entry_versions v on v.id=s.knowledge_entry_version_id
      join app.knowledge_entries k on k.id=v.knowledge_entry_id
      where s.context_pack_id=requested_pack_id and (v.version_number<>k.latest_version or k.status<>'confirmed'))
    and not exists(select 1 from app.context_pack_scope_sources s
      left join app.knowledge_entry_versions v on v.id=s.knowledge_version_id
      left join app.knowledge_entries k on k.id=v.knowledge_entry_id
      left join app.artifact_document_versions a on a.id=s.artifact_version_id
      where s.context_pack_id=requested_pack_id and s.decision='included' and
        ((s.source_kind='tool_source_observation' and not app.tool_source_observation_current(s.tool_source_observation_id))
          or (s.source_kind='publication_observation' and not app.publication_observation_current(s.publication_observation_id))
          or (s.source_kind='knowledge_entry_version' and (v.id is null or v.version_number<>k.latest_version or k.status<>'confirmed'))
          or (s.source_kind='artifact_document_version' and (a.id is null or a.status<>'validated' or exists(
            select 1 from app.artifact_document_versions newer where newer.document_id=a.document_id
              and newer.status='validated' and newer.version>a.version)))));
$function$
;

CREATE OR REPLACE FUNCTION app.finish_publication_job(requested_job uuid, requested_lease uuid, new_status text, safe_error text, receipt jsonb)
 RETURNS boolean
 LANGUAGE plpgsql
 SECURITY DEFINER
 SET search_path TO ''
 SET row_security TO 'off'
AS $function$
declare job app.publication_jobs; connection app.work_tool_connections; authority_id bigint; authority_revision integer;
begin
 if new_status not in('succeeded','failed','needs_review','cancelled') or length(safe_error)>80 then
  raise exception 'Invalid publication settlement' using errcode='23514';
 end if;
 select * into job from app.publication_jobs where public_id=requested_job and lease_token=requested_lease and status in('processing','needs_review');
 if not found then return false; end if;
 perform id from app.projects where id=job.project_id for no key update;
 select * into connection from app.work_tool_connections where id=job.connection_id;
 perform pg_advisory_xact_lock_shared(hashtextextended(job.workspace_id::text||':work-tool:'||connection.public_id::text,0));
 select * into job from app.publication_jobs where public_id=requested_job and lease_token=requested_lease and status in('processing','needs_review') for update;
 if not found then return false; end if;
 select * into connection from app.work_tool_connections where id=job.connection_id;
 if connection.enabled and connection.workspace_id=job.workspace_id and connection.provider=job.provider and connection.revision=job.connection_revision
  and exists(select 1 from app.workspace_members m where m.workspace_id=job.workspace_id and m.actor_id=job.requested_by_actor_id and m.invitation_status='accepted' and m.role in('owner','editor')) then
  authority_id=connection.id; authority_revision=connection.revision;
 end if;
 if new_status='succeeded' then
  if receipt is null or jsonb_typeof(receipt)<>'object' or coalesce(length(receipt->>'external_id'),0) not between 1 and 256
   or coalesce(length(receipt->>'external_url'),0) not between 1 and 2048 or octet_length(receipt::text)>262144 then
   raise exception 'Invalid publication receipt' using errcode='23514';
  end if;
  insert into app.publication_observations(workspace_id,publication_job_id,observation_kind,external_id,external_url,remote_updated_at,snapshot,connection_id,connection_revision)
   values(job.workspace_id,job.id,'created',receipt->>'external_id',receipt->>'external_url',receipt->>'remote_updated_at',receipt,authority_id,authority_revision);
 end if;
 update app.publication_jobs set status=new_status,error_code=safe_error,
  external_id=case when new_status='succeeded' then receipt->>'external_id' else external_id end,
  external_url=case when new_status='succeeded' then receipt->>'external_url' else external_url end,
  updated_at=now() where id=job.id;
 return true;
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
    when 'tool_source_reference' then return exists(select 1 from app.tool_source_references where public_id=endpoint_id and project_id=scope_id and workspace_id=tenant_id);
    when 'tool_source_observation' then return exists(select 1 from app.tool_source_observations where public_id=endpoint_id and project_id=scope_id and workspace_id=tenant_id);
    when 'session' then return exists(select 1 from app.sessions where public_id=endpoint_id and project_id=scope_id and workspace_id=tenant_id);
    when 'task' then return exists(select 1 from app.tasks where public_id=endpoint_id and project_id=scope_id and workspace_id=tenant_id);
    when 'execution' then return exists(select 1 from app.executions where public_id=endpoint_id and project_id=scope_id and workspace_id=tenant_id);
    when 'insight' then return exists(select 1 from app.insights where public_id=endpoint_id and project_id=scope_id and workspace_id=tenant_id);
    when 'github_code_file_observation' then return exists(select 1 from app.github_code_file_observations where public_id=endpoint_id and project_id=scope_id and workspace_id=tenant_id);
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
    where event.event_type in ('knowledge.committed','knowledge.revised','artifact.validated','graph.relationship_confirmed','external_reference.observed','publication.observed','github_code.observed','tool_source.observed','steward.continue')
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
      and not exists(select 1 from app.workspace_automation_controls c where c.workspace_id=event.workspace_id and not c.enabled)
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

CREATE OR REPLACE FUNCTION app.operator_project_predicate(relation_name text)
 RETURNS text
 LANGUAGE plpgsql
 IMMUTABLE
 SET search_path TO ''
AS $function$
begin
 case relation_name
 when 'ai_call_reservations','provider_connections','provider_selections','tool_connections','work_tool_connections',
      'steward_scan_progress','workspace_automation_controls','workspace_invitations','workspace_members','workspaces' then return 'false';
 when 'projects' then return 't.id=$1';
 when 'artifact_version_sources' then return 'exists(select 1 from app.artifact_document_versions v where v.id=t.version_id and v.project_id=$1)';
 when 'publication_observations' then return 'exists(select 1 from app.publication_jobs j where j.id=t.publication_job_id and j.project_id=$1)';
 when 'audit_events' then return '(t.project_id=$1 or (t.project_id is null and t.object_kind=''project'' and t.object_public_id=$3))';
 when 'artifact_destination_settings','context_pack_scope_sources','context_pack_scope_versions','context_pack_selection_items',
      'context_pack_sources','deliverable_sources','domain_events','edges','execution_events','gates','handoffs',
      'insight_resolutions','insight_sources','mutation_proposals','requirement_coverage','steward_assessment_sources',
      'steward_scan_sources','steward_scope_sources','evidences','external_reference_observations','github_code_file_observations','insights',
      'messages','idempotency_records','artifacts','deliverable_sections','github_code_corpora','knowledge_entry_versions',
      'tool_source_references','tool_source_observations','publication_jobs','steward_assessments','artifact_document_versions','deliverables','executions','external_references',
      'knowledge_entries','model_runs','artifact_documents','sessions','tasks','context_packs','context_nodes' then return 't.project_id=$1';
 else raise exception 'Unclassified project erasure table' using errcode='55000';
 end case;
end;
$function$
;

CREATE OR REPLACE FUNCTION app.operator_purge_tables()
 RETURNS text[]
 LANGUAGE sql
 IMMUTABLE
 SET search_path TO ''
AS $function$
 select array[
  'ai_call_reservations',
  'artifact_destination_settings',
  'artifact_version_sources',
  'audit_events',
  'context_pack_scope_sources',
  'context_pack_scope_versions',
  'context_pack_selection_items',
  'context_pack_sources',
  'deliverable_sources',
  'domain_events',
  'edges',
  'execution_events',
  'gates',
  'handoffs',
  'insight_resolutions',
  'insight_sources',
  'mutation_proposals',
  'provider_selections',
  'requirement_coverage',
  'steward_assessment_sources',
  'steward_scope_sources',
  'steward_scan_sources',
  'steward_scan_progress',
  'workspace_automation_controls',
  'workspace_invitations',
  'workspace_members',
  'evidences',
  'external_reference_observations',
  'github_code_file_observations',
  'insights',
  'messages',
  'idempotency_records',
  'provider_connections',
  'publication_observations',
  'tool_source_observations',
  'tool_source_references',
  'artifacts',
  'deliverable_sections',
  'github_code_corpora',
  'knowledge_entry_versions',
  'publication_jobs',
  'steward_assessments',
  'artifact_document_versions',
  'deliverables',
  'executions',
  'external_references',
  'knowledge_entries',
  'model_runs',
  'work_tool_connections',
  'artifact_documents',
  'sessions',
  'tasks',
  'tool_connections',
  'context_packs',
  'context_nodes',
  'projects',
  'workspaces'
 ]::text[];
$function$
;

CREATE OR REPLACE FUNCTION app.publication_observed_context_event()
 RETURNS trigger
 LANGUAGE plpgsql
 SET search_path TO ''
AS $function$
declare job app.publication_jobs; previous app.publication_observations; previous_group bigint; current_group bigint;
begin
 select * into job from app.publication_jobs where id=new.publication_job_id;
 select * into previous from app.publication_observations where publication_job_id=new.publication_job_id and id<new.id order by id desc limit 1;
 if job.provider in('linear','notion') then
  select canonical_observation_id into previous_group from app.publication_source_observations where id=previous.id;
  select canonical_observation_id into current_group from app.publication_source_observations where id=new.id;
  if previous_group is not distinct from current_group and (new.connection_id,new.connection_revision) is not distinct from (previous.connection_id,previous.connection_revision) then return new; end if;
 end if;
 update app.projects set graph_version=graph_version+1 where id=job.project_id;
 with changed as (
  update app.context_packs p set status='stale',invalidated_at=now(),stale_reason='included published observation changed or became unavailable'
  where p.workspace_id=new.workspace_id and p.status='current' and exists(
   select 1 from app.context_pack_scope_sources s join app.publication_observations o on o.id=s.publication_observation_id
   where s.context_pack_id=p.id and s.decision='included' and o.publication_job_id=new.publication_job_id and not app.publication_observation_current(o.id)) returning p.id
 ) update app.deliverables d set status='stale',stale_at=now() where d.source_context_pack_id in(select id from changed) and d.status in('draft','committed');
 insert into app.domain_events(workspace_id,project_id,event_type,aggregate_kind,aggregate_public_id,payload,requested_by_actor_id)
 values(job.workspace_id,job.project_id,'publication.observed','publication_observation',new.public_id,
  jsonb_build_object('observation_public_id',new.public_id,'observation_kind',new.observation_kind),job.requested_by_actor_id);
 return new;
end;
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
   left join app.github_code_file_observations code on code.id=s.github_code_file_observation_id
   left join app.github_code_corpora corpus on corpus.id=code.corpus_id
   where s.assessment_id=requested_assessment_id and (p.id is null or p.status<>'active'
     or (s.knowledge_version_id is not null and (v.version_number<>k.latest_version or k.status<>'confirmed'))
     or (s.artifact_version_id is not null and exists(select 1 from app.artifact_document_versions newer where newer.document_id=a.document_id and newer.status='validated' and newer.version>a.version))
     or (s.external_observation_id is not null and exists(select 1 from app.external_reference_observations newer where newer.external_reference_id=e.external_reference_id and newer.id>e.id))
     or (s.tool_source_observation_id is not null and not app.tool_source_observation_current(s.tool_source_observation_id))
     or (s.publication_observation_id is not null and (case when exists(select 1 from app.publication_jobs j where j.id=o.publication_job_id and j.provider in('notion','linear')) then not app.publication_observation_current(s.publication_observation_id) else exists(select 1 from app.publication_observations newer where newer.publication_job_id=o.publication_job_id and newer.id>o.id) end))
     or (s.github_code_file_observation_id is not null and exists(select 1 from app.github_code_file_observations newer join app.github_code_corpora newer_corpus on newer_corpus.id=newer.corpus_id where newer.project_id=code.project_id and newer.path=code.path and newer_corpus.repository=corpus.repository and newer.id>code.id))));
$function$
;

CREATE OR REPLACE FUNCTION app.validate_scope_source_identity()
 RETURNS trigger
 LANGUAGE plpgsql
 SET search_path TO ''
AS $function$
declare actual_id uuid; actual_project bigint;
begin
 case new.source_kind
 when 'knowledge_entry_version' then select public_id,project_id into actual_id,actual_project from app.knowledge_entry_versions where id=new.knowledge_version_id;
 when 'artifact_document_version' then select public_id,project_id into actual_id,actual_project from app.artifact_document_versions where id=new.artifact_version_id and status='validated';
 when 'tool_source_observation' then select public_id,project_id into actual_id,actual_project from app.tool_source_observations where id=new.tool_source_observation_id;
 when 'publication_observation' then select o.public_id,j.project_id into actual_id,actual_project from app.publication_observations o join app.publication_jobs j on j.id=o.publication_job_id where o.id=new.publication_observation_id;
 else raise exception 'Unknown context source' using errcode='23514';
 end case;
 if actual_id is null or actual_id<>new.source_public_id or actual_project<>new.source_project_id then
  raise exception 'Scoped source identity does not match its immutable version' using errcode='23514';
 end if;
 return new;
end;
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
 when 'tool_source_observation' then select public_id,project_id into actual_id,actual_project from app.tool_source_observations where id=new.tool_source_observation_id;
 when 'github_code_file_observation' then select public_id,project_id into actual_id,actual_project from app.github_code_file_observations where id=new.github_code_file_observation_id;
 end case;
 if actual_id is null or actual_id<>new.source_public_id or actual_project<>new.source_project_id then
   raise exception 'Steward source does not match its immutable scope and version' using errcode='23514';
 end if;
 return new;
end;
$function$
;

grant insert on table "app"."tool_source_observations" to "ai_center_runtime";

grant select on table "app"."tool_source_observations" to "ai_center_runtime";

grant insert on table "app"."tool_source_references" to "ai_center_runtime";

grant select on table "app"."tool_source_references" to "ai_center_runtime";


  create policy "tool_source_editor_insert"
  on "app"."tool_source_observations"
  as permissive
  for insert
  to public
with check (((workspace_id = ( SELECT app.current_workspace_id() AS current_workspace_id)) AND app.has_workspace_role(workspace_id, ARRAY['owner'::text, 'editor'::text])));



  create policy "tool_source_member_select"
  on "app"."tool_source_observations"
  as permissive
  for select
  to public
using (((workspace_id = ( SELECT app.current_workspace_id() AS current_workspace_id)) AND app.has_workspace_role(workspace_id, ARRAY['owner'::text, 'editor'::text, 'viewer'::text])));



  create policy "tool_source_editor_insert"
  on "app"."tool_source_references"
  as permissive
  for insert
  to public
with check (((workspace_id = ( SELECT app.current_workspace_id() AS current_workspace_id)) AND app.has_workspace_role(workspace_id, ARRAY['owner'::text, 'editor'::text])));



  create policy "tool_source_editor_update"
  on "app"."tool_source_references"
  as permissive
  for update
  to public
using (((workspace_id = ( SELECT app.current_workspace_id() AS current_workspace_id)) AND app.has_workspace_role(workspace_id, ARRAY['owner'::text, 'editor'::text])))
with check (((workspace_id = ( SELECT app.current_workspace_id() AS current_workspace_id)) AND app.has_workspace_role(workspace_id, ARRAY['owner'::text, 'editor'::text])));



  create policy "tool_source_member_select"
  on "app"."tool_source_references"
  as permissive
  for select
  to public
using (((workspace_id = ( SELECT app.current_workspace_id() AS current_workspace_id)) AND app.has_workspace_role(workspace_id, ARRAY['owner'::text, 'editor'::text, 'viewer'::text])));


CREATE TRIGGER artifact_external_source_identity BEFORE INSERT ON app.artifact_version_sources FOR EACH ROW EXECUTE FUNCTION app.validate_external_source_provenance();

CREATE CONSTRAINT TRIGGER tool_source_observation_head_required AFTER INSERT ON app.tool_source_observations DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION app.validate_tool_source_head();

CREATE TRIGGER tool_source_observation_identity BEFORE INSERT ON app.tool_source_observations FOR EACH ROW EXECUTE FUNCTION app.validate_tool_source_identity();

CREATE TRIGGER tool_source_observations_immutable BEFORE DELETE OR UPDATE ON app.tool_source_observations FOR EACH ROW EXECUTE FUNCTION app.prevent_append_only_mutation();

CREATE CONSTRAINT TRIGGER tool_source_head_required AFTER INSERT OR UPDATE ON app.tool_source_references DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION app.validate_tool_source_head();

CREATE TRIGGER tool_source_reference_context AFTER INSERT OR UPDATE ON app.tool_source_references FOR EACH ROW EXECUTE FUNCTION app.tool_source_context_event();

CREATE TRIGGER tool_source_reference_identity BEFORE INSERT OR UPDATE ON app.tool_source_references FOR EACH ROW EXECUTE FUNCTION app.validate_tool_source_identity();



-- Reviewed privilege/forced-RLS repair omitted by the legacy diff engine.
alter table app.tool_source_references force row level security;
alter table app.tool_source_observations force row level security;
revoke all on function app.validate_tool_source_identity() from public,anon,authenticated,service_role;
revoke all on function app.validate_tool_source_head() from public,anon,authenticated,service_role;
revoke all on function app.tool_source_observation_current(bigint) from public,anon,authenticated,service_role;
revoke all on function app.context_utf8_prefix(text,integer) from public,anon,authenticated,service_role;
revoke all on app.publication_source_observations from public,anon,authenticated,service_role;
revoke all on function app.publication_observation_current(bigint) from public,anon,authenticated,service_role;
revoke all on function app.validate_external_source_provenance() from public,anon,authenticated,service_role;
revoke all on function app.validate_scope_source_identity() from public,anon,authenticated,service_role;
revoke all on function app.context_pack_scopes_current(bigint) from public,anon,authenticated,service_role;
revoke all on function app.graph_endpoint_exists(text,uuid,bigint,bigint) from public,anon,authenticated,service_role;
revoke all on function app.tool_source_context_event() from public,anon,authenticated,service_role;
revoke all on function app.finish_publication_job(uuid,uuid,text,text,jsonb) from public,anon,authenticated,service_role;
revoke all on function app.publication_observed_context_event() from public,anon,authenticated,service_role;
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

-- Existing-tool context, same company roles and immutable observations.
grant update(allow_existing_reads) on app.work_tool_connections to ai_center_runtime;
grant select,insert on app.tool_source_references,app.tool_source_observations to ai_center_runtime;
grant update(canonical_url,connection_id,connection_revision,updated_at,status,revision,current_observation_id,last_attempt_at,last_checked_at,last_check_status,last_check_error_code)
 on app.tool_source_references to ai_center_runtime;
grant usage on sequence app.tool_source_references_id_seq,app.tool_source_observations_id_seq to ai_center_runtime;
grant select on app.publication_source_observations to ai_center_runtime;
grant execute on function app.context_utf8_prefix(text,integer),app.tool_source_observation_current(bigint),app.publication_observation_current(bigint) to ai_center_runtime;

-- The legacy diff omits view security options; preserve caller RLS.
alter view app.publication_source_observations set (security_invoker=true);
