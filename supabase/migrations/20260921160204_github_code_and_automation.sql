alter table "app"."steward_scope_sources" drop constraint "steward_scope_sources_check";

alter table "app"."steward_scope_sources" drop constraint "steward_scope_sources_source_kind_check";


  create table "app"."ai_call_reservations" (
    "id" bigint generated always as identity not null,
    "public_id" uuid not null,
    "workspace_id" bigint not null,
    "actor_id" uuid not null,
    "operation" text not null,
    "control_generation" bigint not null,
    "status" text not null default 'running'::text,
    "created_at" timestamp with time zone not null default now(),
    "lease_until" timestamp with time zone not null,
    "finished_at" timestamp with time zone
      );


alter table "app"."ai_call_reservations" enable row level security;


  create table "app"."github_code_corpora" (
    "id" bigint generated always as identity not null,
    "public_id" uuid not null default gen_random_uuid(),
    "workspace_id" bigint not null,
    "project_id" bigint not null,
    "connection_id" bigint not null,
    "repository" text not null,
    "commit_sha" text not null,
    "commit_verified" boolean not null,
    "requested_paths" jsonb not null,
    "requested_by_actor_id" uuid not null,
    "observed_at" timestamp with time zone not null default now()
      );


alter table "app"."github_code_corpora" enable row level security;


  create table "app"."github_code_file_observations" (
    "id" bigint generated always as identity not null,
    "public_id" uuid not null default gen_random_uuid(),
    "workspace_id" bigint not null,
    "project_id" bigint not null,
    "corpus_id" bigint not null,
    "path" text not null,
    "status" text not null,
    "reason_code" text,
    "blob_sha" text,
    "content_hash" text,
    "content_text" text,
    "line_count" integer not null default 0,
    "observed_at" timestamp with time zone not null default now()
      );


alter table "app"."github_code_file_observations" enable row level security;


  create table "app"."workspace_automation_controls" (
    "id" bigint generated always as identity not null,
    "workspace_id" bigint not null,
    "enabled" boolean not null default true,
    "generation" bigint not null default 1,
    "updated_by_actor_id" uuid not null,
    "updated_at" timestamp with time zone not null default now()
      );


alter table "app"."workspace_automation_controls" enable row level security;

alter table "app"."steward_scope_sources" add column "github_code_file_observation_id" bigint;

CREATE INDEX ai_call_reservations_active_idx ON app.ai_call_reservations USING btree (workspace_id, lease_until) WHERE (status = 'running'::text);

CREATE UNIQUE INDEX ai_call_reservations_pkey ON app.ai_call_reservations USING btree (id);

CREATE UNIQUE INDEX ai_call_reservations_public_id_key ON app.ai_call_reservations USING btree (public_id);

CREATE INDEX ai_call_reservations_scope_time_idx ON app.ai_call_reservations USING btree (workspace_id, created_at DESC);

CREATE INDEX github_code_corpora_connection_idx ON app.github_code_corpora USING btree (connection_id);

CREATE UNIQUE INDEX github_code_corpora_id_workspace_id_project_id_key ON app.github_code_corpora USING btree (id, workspace_id, project_id);

CREATE UNIQUE INDEX github_code_corpora_pkey ON app.github_code_corpora USING btree (id);

CREATE INDEX github_code_corpora_project_idx ON app.github_code_corpora USING btree (project_id);

CREATE UNIQUE INDEX github_code_corpora_public_id_key ON app.github_code_corpora USING btree (public_id);

CREATE INDEX github_code_corpora_scope_idx ON app.github_code_corpora USING btree (workspace_id, project_id, id DESC);

CREATE UNIQUE INDEX github_code_file_observations_corpus_id_path_key ON app.github_code_file_observations USING btree (corpus_id, path);

CREATE UNIQUE INDEX github_code_file_observations_id_project_id_key ON app.github_code_file_observations USING btree (id, project_id);

CREATE UNIQUE INDEX github_code_file_observations_id_workspace_id_key ON app.github_code_file_observations USING btree (id, workspace_id);

CREATE UNIQUE INDEX github_code_file_observations_pkey ON app.github_code_file_observations USING btree (id);

CREATE UNIQUE INDEX github_code_file_observations_public_id_key ON app.github_code_file_observations USING btree (public_id);

CREATE INDEX github_code_files_project_idx ON app.github_code_file_observations USING btree (project_id);

CREATE INDEX github_code_files_workspace_idx ON app.github_code_file_observations USING btree (workspace_id);

CREATE INDEX steward_scope_sources_code_idx ON app.steward_scope_sources USING btree (github_code_file_observation_id) WHERE (github_code_file_observation_id IS NOT NULL);

CREATE UNIQUE INDEX workspace_automation_controls_pkey ON app.workspace_automation_controls USING btree (id);

CREATE UNIQUE INDEX workspace_automation_controls_workspace_id_key ON app.workspace_automation_controls USING btree (workspace_id);

alter table "app"."ai_call_reservations" add constraint "ai_call_reservations_pkey" PRIMARY KEY using index "ai_call_reservations_pkey";

alter table "app"."github_code_corpora" add constraint "github_code_corpora_pkey" PRIMARY KEY using index "github_code_corpora_pkey";

alter table "app"."github_code_file_observations" add constraint "github_code_file_observations_pkey" PRIMARY KEY using index "github_code_file_observations_pkey";

alter table "app"."workspace_automation_controls" add constraint "workspace_automation_controls_pkey" PRIMARY KEY using index "workspace_automation_controls_pkey";

alter table "app"."ai_call_reservations" add constraint "ai_call_reservations_check" CHECK (((lease_until > created_at) AND (lease_until <= (created_at + '00:11:00'::interval)))) not valid;

alter table "app"."ai_call_reservations" validate constraint "ai_call_reservations_check";

alter table "app"."ai_call_reservations" add constraint "ai_call_reservations_check1" CHECK (((status = 'running'::text) = (finished_at IS NULL))) not valid;

alter table "app"."ai_call_reservations" validate constraint "ai_call_reservations_check1";

alter table "app"."ai_call_reservations" add constraint "ai_call_reservations_control_generation_check" CHECK ((control_generation >= 0)) not valid;

alter table "app"."ai_call_reservations" validate constraint "ai_call_reservations_control_generation_check";

alter table "app"."ai_call_reservations" add constraint "ai_call_reservations_operation_check" CHECK ((operation = ANY (ARRAY['respond'::text, 'select_context'::text, 'technical_plan'::text, 'coverage'::text, 'steward'::text]))) not valid;

alter table "app"."ai_call_reservations" validate constraint "ai_call_reservations_operation_check";

alter table "app"."ai_call_reservations" add constraint "ai_call_reservations_public_id_key" UNIQUE using index "ai_call_reservations_public_id_key";

alter table "app"."ai_call_reservations" add constraint "ai_call_reservations_status_check" CHECK ((status = ANY (ARRAY['running'::text, 'completed'::text, 'failed'::text, 'cancelled'::text]))) not valid;

alter table "app"."ai_call_reservations" validate constraint "ai_call_reservations_status_check";

alter table "app"."ai_call_reservations" add constraint "ai_call_reservations_workspace_id_fkey" FOREIGN KEY (workspace_id) REFERENCES app.workspaces(id) ON DELETE CASCADE not valid;

alter table "app"."ai_call_reservations" validate constraint "ai_call_reservations_workspace_id_fkey";

alter table "app"."github_code_corpora" add constraint "github_code_corpora_commit_sha_check" CHECK ((commit_sha ~ '^[0-9a-f]{40}$'::text)) not valid;

alter table "app"."github_code_corpora" validate constraint "github_code_corpora_commit_sha_check";

alter table "app"."github_code_corpora" add constraint "github_code_corpora_connection_id_workspace_id_fkey" FOREIGN KEY (connection_id, workspace_id) REFERENCES app.work_tool_connections(id, workspace_id) not valid;

alter table "app"."github_code_corpora" validate constraint "github_code_corpora_connection_id_workspace_id_fkey";

alter table "app"."github_code_corpora" add constraint "github_code_corpora_id_workspace_id_project_id_key" UNIQUE using index "github_code_corpora_id_workspace_id_project_id_key";

alter table "app"."github_code_corpora" add constraint "github_code_corpora_project_id_workspace_id_fkey" FOREIGN KEY (project_id, workspace_id) REFERENCES app.projects(id, workspace_id) not valid;

alter table "app"."github_code_corpora" validate constraint "github_code_corpora_project_id_workspace_id_fkey";

alter table "app"."github_code_corpora" add constraint "github_code_corpora_public_id_key" UNIQUE using index "github_code_corpora_public_id_key";

alter table "app"."github_code_corpora" add constraint "github_code_corpora_repository_check" CHECK (((length(repository) >= 3) AND (length(repository) <= 201))) not valid;

alter table "app"."github_code_corpora" validate constraint "github_code_corpora_repository_check";

alter table "app"."github_code_corpora" add constraint "github_code_corpora_requested_paths_check" CHECK (((jsonb_typeof(requested_paths) = 'array'::text) AND ((jsonb_array_length(requested_paths) >= 1) AND (jsonb_array_length(requested_paths) <= 10)))) not valid;

alter table "app"."github_code_corpora" validate constraint "github_code_corpora_requested_paths_check";

alter table "app"."github_code_file_observations" add constraint "github_code_file_observations_blob_sha_check" CHECK ((blob_sha ~ '^[0-9a-f]{40}$'::text)) not valid;

alter table "app"."github_code_file_observations" validate constraint "github_code_file_observations_blob_sha_check";

alter table "app"."github_code_file_observations" add constraint "github_code_file_observations_check" CHECK (((status = 'code_read'::text) = ((content_text IS NOT NULL) AND (content_hash IS NOT NULL) AND (blob_sha IS NOT NULL)))) not valid;

alter table "app"."github_code_file_observations" validate constraint "github_code_file_observations_check";

alter table "app"."github_code_file_observations" add constraint "github_code_file_observations_check1" CHECK (((status = 'code_read'::text) OR ((content_text IS NULL) AND (content_hash IS NULL) AND (line_count = 0)))) not valid;

alter table "app"."github_code_file_observations" validate constraint "github_code_file_observations_check1";

alter table "app"."github_code_file_observations" add constraint "github_code_file_observations_content_hash_check" CHECK ((content_hash ~ '^[0-9a-f]{64}$'::text)) not valid;

alter table "app"."github_code_file_observations" validate constraint "github_code_file_observations_content_hash_check";

alter table "app"."github_code_file_observations" add constraint "github_code_file_observations_content_text_check" CHECK ((octet_length(content_text) <= 65536)) not valid;

alter table "app"."github_code_file_observations" validate constraint "github_code_file_observations_content_text_check";

alter table "app"."github_code_file_observations" add constraint "github_code_file_observations_corpus_id_path_key" UNIQUE using index "github_code_file_observations_corpus_id_path_key";

alter table "app"."github_code_file_observations" add constraint "github_code_file_observations_corpus_id_workspace_id_proje_fkey" FOREIGN KEY (corpus_id, workspace_id, project_id) REFERENCES app.github_code_corpora(id, workspace_id, project_id) not valid;

alter table "app"."github_code_file_observations" validate constraint "github_code_file_observations_corpus_id_workspace_id_proje_fkey";

alter table "app"."github_code_file_observations" add constraint "github_code_file_observations_id_project_id_key" UNIQUE using index "github_code_file_observations_id_project_id_key";

alter table "app"."github_code_file_observations" add constraint "github_code_file_observations_id_workspace_id_key" UNIQUE using index "github_code_file_observations_id_workspace_id_key";

alter table "app"."github_code_file_observations" add constraint "github_code_file_observations_line_count_check" CHECK ((line_count >= 0)) not valid;

alter table "app"."github_code_file_observations" validate constraint "github_code_file_observations_line_count_check";

alter table "app"."github_code_file_observations" add constraint "github_code_file_observations_path_check" CHECK (((length(path) >= 1) AND (length(path) <= 512))) not valid;

alter table "app"."github_code_file_observations" validate constraint "github_code_file_observations_path_check";

alter table "app"."github_code_file_observations" add constraint "github_code_file_observations_public_id_key" UNIQUE using index "github_code_file_observations_public_id_key";

alter table "app"."github_code_file_observations" add constraint "github_code_file_observations_status_check" CHECK ((status = ANY (ARRAY['code_read'::text, 'missing'::text, 'inaccessible'::text, 'too_large'::text, 'binary'::text, 'unsupported'::text, 'unavailable'::text]))) not valid;

alter table "app"."github_code_file_observations" validate constraint "github_code_file_observations_status_check";

alter table "app"."steward_scope_sources" add constraint "steward_scope_sources_code_kind_check" CHECK (((source_kind = 'github_code_file_observation'::text) = (github_code_file_observation_id IS NOT NULL))) not valid;

alter table "app"."steward_scope_sources" validate constraint "steward_scope_sources_code_kind_check";

alter table "app"."steward_scope_sources" add constraint "steward_scope_sources_github_code_file_observation_id_sour_fkey" FOREIGN KEY (github_code_file_observation_id, source_project_id) REFERENCES app.github_code_file_observations(id, project_id) not valid;

alter table "app"."steward_scope_sources" validate constraint "steward_scope_sources_github_code_file_observation_id_sour_fkey";

alter table "app"."workspace_automation_controls" add constraint "workspace_automation_controls_generation_check" CHECK ((generation > 0)) not valid;

alter table "app"."workspace_automation_controls" validate constraint "workspace_automation_controls_generation_check";

alter table "app"."workspace_automation_controls" add constraint "workspace_automation_controls_workspace_id_fkey" FOREIGN KEY (workspace_id) REFERENCES app.workspaces(id) ON DELETE CASCADE not valid;

alter table "app"."workspace_automation_controls" validate constraint "workspace_automation_controls_workspace_id_fkey";

alter table "app"."workspace_automation_controls" add constraint "workspace_automation_controls_workspace_id_key" UNIQUE using index "workspace_automation_controls_workspace_id_key";

alter table "app"."steward_scope_sources" add constraint "steward_scope_sources_check" CHECK ((num_nonnulls(knowledge_version_id, artifact_version_id, external_observation_id, publication_observation_id, github_code_file_observation_id) = 1)) not valid;

alter table "app"."steward_scope_sources" validate constraint "steward_scope_sources_check";

alter table "app"."steward_scope_sources" add constraint "steward_scope_sources_source_kind_check" CHECK ((source_kind = ANY (ARRAY['knowledge_entry_version'::text, 'artifact_document_version'::text, 'external_reference_observation'::text, 'publication_observation'::text, 'github_code_file_observation'::text]))) not valid;

alter table "app"."steward_scope_sources" validate constraint "steward_scope_sources_source_kind_check";

set check_function_bodies = off;

CREATE OR REPLACE FUNCTION app.finish_ai_call_reservation(requested_id uuid, requested_status text)
 RETURNS boolean
 LANGUAGE plpgsql
 SECURITY DEFINER
 SET search_path TO ''
AS $function$
begin
  if requested_status not in ('completed','failed','cancelled') then
    raise exception 'invalid reservation settlement' using errcode='22023';
  end if;
  update app.ai_call_reservations set status=requested_status,finished_at=now()
    where public_id=requested_id and workspace_id=app.current_workspace_id()
      and actor_id=app.current_actor_id() and status='running';
  return found;
end;
$function$
;

CREATE OR REPLACE FUNCTION app.github_code_file_scope()
 RETURNS trigger
 LANGUAGE plpgsql
 SET search_path TO ''
AS $function$
declare corpus app.github_code_corpora;
begin
 select * into corpus from app.github_code_corpora where id=new.corpus_id and workspace_id=new.workspace_id and project_id=new.project_id;
 if corpus.id is null or not (corpus.requested_paths ? new.path) or (new.status='code_read' and not corpus.commit_verified) then
  raise exception 'Code evidence requires the requested path and a verified commit' using errcode='23514';
 end if;
 return new;
end;
$function$
;

CREATE OR REPLACE FUNCTION app.github_code_observed_context_event()
 RETURNS trigger
 LANGUAGE plpgsql
 SET search_path TO ''
AS $function$
declare corpus app.github_code_corpora;
begin
 select * into corpus from app.github_code_corpora where id=new.corpus_id;
 update app.projects set graph_version=graph_version+1 where id=new.project_id;
 insert into app.domain_events(workspace_id,project_id,event_type,aggregate_kind,aggregate_public_id,payload,requested_by_actor_id)
 values(new.workspace_id,new.project_id,'github_code.observed','github_code_file_observation',new.public_id,
   jsonb_build_object('observation_public_id',new.public_id,'corpus_public_id',corpus.public_id,'status',new.status),corpus.requested_by_actor_id);
 return new;
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
    and not exists(select 1 from app.workspace_automation_controls c where c.workspace_id=j.workspace_id and not c.enabled)
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
    where event.event_type in ('knowledge.committed','knowledge.revised','artifact.validated','graph.relationship_confirmed','external_reference.observed','publication.observed','github_code.observed')
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
     or (s.publication_observation_id is not null and exists(select 1 from app.publication_observations newer where newer.publication_job_id=o.publication_job_id and newer.id>o.id))
     or (s.github_code_file_observation_id is not null and exists(select 1 from app.github_code_file_observations newer join app.github_code_corpora newer_corpus on newer_corpus.id=newer.corpus_id where newer.project_id=code.project_id and newer.path=code.path and newer_corpus.repository=corpus.repository and newer.id>code.id))));
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
 when 'github_code_file_observation' then select public_id,project_id into actual_id,actual_project from app.github_code_file_observations where id=new.github_code_file_observation_id;
 end case;
 if actual_id is null or actual_id<>new.source_public_id or actual_project<>new.source_project_id then
   raise exception 'Steward source does not match its immutable scope and version' using errcode='23514';
 end if;
 return new;
end;
$function$
;

grant insert on table "app"."ai_call_reservations" to "ai_center_runtime";

grant select on table "app"."ai_call_reservations" to "ai_center_runtime";

grant insert on table "app"."github_code_corpora" to "ai_center_runtime";

grant select on table "app"."github_code_corpora" to "ai_center_runtime";

grant insert on table "app"."github_code_file_observations" to "ai_center_runtime";

grant select on table "app"."github_code_file_observations" to "ai_center_runtime";

grant insert on table "app"."workspace_automation_controls" to "ai_center_runtime";

grant select on table "app"."workspace_automation_controls" to "ai_center_runtime";


  create policy "ai_reservations_insert"
  on "app"."ai_call_reservations"
  as permissive
  for insert
  to public
with check (((workspace_id = ( SELECT app.current_workspace_id() AS current_workspace_id)) AND (actor_id = ( SELECT app.current_actor_id() AS current_actor_id)) AND app.has_workspace_role(workspace_id, ARRAY['owner'::text, 'editor'::text])));



  create policy "ai_reservations_read"
  on "app"."ai_call_reservations"
  as permissive
  for select
  to public
using (((workspace_id = ( SELECT app.current_workspace_id() AS current_workspace_id)) AND app.has_workspace_role(workspace_id, ARRAY['owner'::text, 'editor'::text, 'viewer'::text])));



  create policy "ai_reservations_update"
  on "app"."ai_call_reservations"
  as permissive
  for update
  to public
using (((workspace_id = ( SELECT app.current_workspace_id() AS current_workspace_id)) AND (actor_id = ( SELECT app.current_actor_id() AS current_actor_id)) AND app.has_workspace_role(workspace_id, ARRAY['owner'::text, 'editor'::text])))
with check (((workspace_id = ( SELECT app.current_workspace_id() AS current_workspace_id)) AND (actor_id = ( SELECT app.current_actor_id() AS current_actor_id)) AND app.has_workspace_role(workspace_id, ARRAY['owner'::text, 'editor'::text])));



  create policy "github_code_editor_insert"
  on "app"."github_code_corpora"
  as permissive
  for insert
  to public
with check (((workspace_id = app.current_workspace_id()) AND app.has_workspace_role(workspace_id, ARRAY['owner'::text, 'editor'::text])));



  create policy "github_code_member_select"
  on "app"."github_code_corpora"
  as permissive
  for select
  to public
using (((workspace_id = app.current_workspace_id()) AND app.has_workspace_role(workspace_id, ARRAY['owner'::text, 'editor'::text, 'viewer'::text])));



  create policy "github_code_editor_insert"
  on "app"."github_code_file_observations"
  as permissive
  for insert
  to public
with check (((workspace_id = app.current_workspace_id()) AND app.has_workspace_role(workspace_id, ARRAY['owner'::text, 'editor'::text])));



  create policy "github_code_member_select"
  on "app"."github_code_file_observations"
  as permissive
  for select
  to public
using (((workspace_id = app.current_workspace_id()) AND app.has_workspace_role(workspace_id, ARRAY['owner'::text, 'editor'::text, 'viewer'::text])));



  create policy "automation_controls_owner"
  on "app"."workspace_automation_controls"
  as permissive
  for all
  to public
using (((workspace_id = ( SELECT app.current_workspace_id() AS current_workspace_id)) AND app.has_workspace_role(workspace_id, ARRAY['owner'::text])))
with check (((workspace_id = ( SELECT app.current_workspace_id() AS current_workspace_id)) AND (updated_by_actor_id = ( SELECT app.current_actor_id() AS current_actor_id)) AND app.has_workspace_role(workspace_id, ARRAY['owner'::text])));



  create policy "automation_controls_read"
  on "app"."workspace_automation_controls"
  as permissive
  for select
  to public
using (((workspace_id = ( SELECT app.current_workspace_id() AS current_workspace_id)) AND app.has_workspace_role(workspace_id, ARRAY['owner'::text, 'editor'::text, 'viewer'::text])));


CREATE TRIGGER github_code_immutable BEFORE DELETE OR UPDATE ON app.github_code_corpora FOR EACH ROW EXECUTE FUNCTION app.prevent_append_only_mutation();

CREATE TRIGGER github_code_file_scope BEFORE INSERT ON app.github_code_file_observations FOR EACH ROW EXECUTE FUNCTION app.github_code_file_scope();

CREATE TRIGGER github_code_immutable BEFORE DELETE OR UPDATE ON app.github_code_file_observations FOR EACH ROW EXECUTE FUNCTION app.prevent_append_only_mutation();

CREATE TRIGGER github_code_observed_context AFTER INSERT ON app.github_code_file_observations FOR EACH ROW EXECUTE FUNCTION app.github_code_observed_context_event();



-- Reviewed security metadata omitted by the schema diff tool.
revoke all on function app.github_code_file_scope() from public,anon,authenticated,service_role;
grant select,insert on app.github_code_corpora,app.github_code_file_observations to ai_center_runtime;
grant usage on sequence app.github_code_corpora_id_seq,app.github_code_file_observations_id_seq to ai_center_runtime;
revoke all on function app.github_code_observed_context_event() from public,anon,authenticated,service_role;
revoke all on app.workspace_automation_controls,app.ai_call_reservations from public,anon,authenticated,service_role;
grant select,insert on app.workspace_automation_controls,app.ai_call_reservations to ai_center_runtime;
grant update(enabled,generation,updated_by_actor_id,updated_at) on app.workspace_automation_controls to ai_center_runtime;
grant update(status,finished_at) on app.ai_call_reservations to ai_center_runtime;
grant usage on sequence app.workspace_automation_controls_id_seq,app.ai_call_reservations_id_seq to ai_center_runtime;
revoke all on function app.finish_ai_call_reservation(uuid,text) from public,anon,authenticated,service_role;
grant execute on function app.finish_ai_call_reservation(uuid,text) to ai_center_runtime;
revoke all on function app.claim_publication_job() from public,anon,authenticated,service_role;
alter table app.github_code_corpora force row level security;
revoke all on app.github_code_corpora from public,anon,authenticated,service_role;
alter table app.github_code_file_observations force row level security;
revoke all on app.github_code_file_observations from public,anon,authenticated,service_role;
alter table app.workspace_automation_controls force row level security;
revoke all on app.workspace_automation_controls from public,anon,authenticated,service_role;
alter table app.ai_call_reservations force row level security;
revoke all on app.ai_call_reservations from public,anon,authenticated,service_role;
