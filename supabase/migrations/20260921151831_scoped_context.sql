-- Generated from the isolated declarative schema; FORCE RLS and ACL metadata
-- below are reviewed supplements omitted by the migra diff engine.

  create table "app"."context_pack_scope_sources" (
    "id" bigint generated always as identity not null,
    "workspace_id" bigint not null,
    "project_id" bigint not null,
    "context_pack_id" bigint not null,
    "source_project_id" bigint not null,
    "source_kind" text not null,
    "source_public_id" uuid not null,
    "knowledge_version_id" bigint,
    "artifact_version_id" bigint,
    "decision" text not null,
    "reason_code" text not null,
    "explanation" text not null,
    "rank" integer not null,
    "estimated_tokens" integer not null,
    "is_mandatory" boolean not null
      );


alter table "app"."context_pack_scope_sources" enable row level security;


  create table "app"."context_pack_scope_versions" (
    "id" bigint generated always as identity not null,
    "workspace_id" bigint not null,
    "project_id" bigint not null,
    "context_pack_id" bigint not null,
    "source_project_id" bigint not null,
    "graph_version" bigint not null
      );


alter table "app"."context_pack_scope_versions" enable row level security;

CREATE INDEX context_pack_scope_sources_artifact_idx ON app.context_pack_scope_sources USING btree (artifact_version_id) WHERE (artifact_version_id IS NOT NULL);

CREATE UNIQUE INDEX context_pack_scope_sources_context_pack_id_source_kind_sour_key ON app.context_pack_scope_sources USING btree (context_pack_id, source_kind, source_public_id);

CREATE INDEX context_pack_scope_sources_knowledge_idx ON app.context_pack_scope_sources USING btree (knowledge_version_id) WHERE (knowledge_version_id IS NOT NULL);

CREATE UNIQUE INDEX context_pack_scope_sources_pkey ON app.context_pack_scope_sources USING btree (id);

CREATE INDEX context_pack_scope_sources_project_idx ON app.context_pack_scope_sources USING btree (project_id);

CREATE INDEX context_pack_scope_sources_scope_idx ON app.context_pack_scope_sources USING btree (source_project_id);

CREATE INDEX context_pack_scope_sources_workspace_idx ON app.context_pack_scope_sources USING btree (workspace_id);

CREATE UNIQUE INDEX context_pack_scope_versions_context_pack_id_source_project__key ON app.context_pack_scope_versions USING btree (context_pack_id, source_project_id);

CREATE UNIQUE INDEX context_pack_scope_versions_pkey ON app.context_pack_scope_versions USING btree (id);

CREATE INDEX context_pack_scope_versions_project_idx ON app.context_pack_scope_versions USING btree (project_id);

CREATE INDEX context_pack_scope_versions_source_idx ON app.context_pack_scope_versions USING btree (source_project_id);

CREATE INDEX context_pack_scope_versions_workspace_idx ON app.context_pack_scope_versions USING btree (workspace_id);

alter table "app"."context_pack_scope_sources" add constraint "context_pack_scope_sources_pkey" PRIMARY KEY using index "context_pack_scope_sources_pkey";

alter table "app"."context_pack_scope_versions" add constraint "context_pack_scope_versions_pkey" PRIMARY KEY using index "context_pack_scope_versions_pkey";

alter table "app"."context_pack_scope_sources" add constraint "context_pack_scope_sources_artifact_version_id_source_proj_fkey" FOREIGN KEY (artifact_version_id, source_project_id) REFERENCES app.artifact_document_versions(id, project_id) not valid;

alter table "app"."context_pack_scope_sources" validate constraint "context_pack_scope_sources_artifact_version_id_source_proj_fkey";

alter table "app"."context_pack_scope_sources" add constraint "context_pack_scope_sources_check" CHECK ((((source_kind = 'knowledge_entry_version'::text) AND (knowledge_version_id IS NOT NULL) AND (artifact_version_id IS NULL)) OR ((source_kind = 'artifact_document_version'::text) AND (knowledge_version_id IS NULL) AND (artifact_version_id IS NOT NULL)))) not valid;

alter table "app"."context_pack_scope_sources" validate constraint "context_pack_scope_sources_check";

alter table "app"."context_pack_scope_sources" add constraint "context_pack_scope_sources_check1" CHECK (((NOT is_mandatory) OR (decision = 'included'::text))) not valid;

alter table "app"."context_pack_scope_sources" validate constraint "context_pack_scope_sources_check1";

alter table "app"."context_pack_scope_sources" add constraint "context_pack_scope_sources_context_pack_id_project_id_fkey" FOREIGN KEY (context_pack_id, project_id) REFERENCES app.context_packs(id, project_id) not valid;

alter table "app"."context_pack_scope_sources" validate constraint "context_pack_scope_sources_context_pack_id_project_id_fkey";

alter table "app"."context_pack_scope_sources" add constraint "context_pack_scope_sources_context_pack_id_source_kind_sour_key" UNIQUE using index "context_pack_scope_sources_context_pack_id_source_kind_sour_key";

alter table "app"."context_pack_scope_sources" add constraint "context_pack_scope_sources_decision_check" CHECK ((decision = ANY (ARRAY['included'::text, 'excluded'::text]))) not valid;

alter table "app"."context_pack_scope_sources" validate constraint "context_pack_scope_sources_decision_check";

alter table "app"."context_pack_scope_sources" add constraint "context_pack_scope_sources_estimated_tokens_check" CHECK ((estimated_tokens >= 0)) not valid;

alter table "app"."context_pack_scope_sources" validate constraint "context_pack_scope_sources_estimated_tokens_check";

alter table "app"."context_pack_scope_sources" add constraint "context_pack_scope_sources_knowledge_version_id_source_pro_fkey" FOREIGN KEY (knowledge_version_id, source_project_id) REFERENCES app.knowledge_entry_versions(id, project_id) not valid;

alter table "app"."context_pack_scope_sources" validate constraint "context_pack_scope_sources_knowledge_version_id_source_pro_fkey";

alter table "app"."context_pack_scope_sources" add constraint "context_pack_scope_sources_project_id_workspace_id_fkey" FOREIGN KEY (project_id, workspace_id) REFERENCES app.projects(id, workspace_id) not valid;

alter table "app"."context_pack_scope_sources" validate constraint "context_pack_scope_sources_project_id_workspace_id_fkey";

alter table "app"."context_pack_scope_sources" add constraint "context_pack_scope_sources_rank_check" CHECK ((rank > 0)) not valid;

alter table "app"."context_pack_scope_sources" validate constraint "context_pack_scope_sources_rank_check";

alter table "app"."context_pack_scope_sources" add constraint "context_pack_scope_sources_reason_code_check" CHECK ((btrim(reason_code) <> ''::text)) not valid;

alter table "app"."context_pack_scope_sources" validate constraint "context_pack_scope_sources_reason_code_check";

alter table "app"."context_pack_scope_sources" add constraint "context_pack_scope_sources_source_kind_check" CHECK ((source_kind = ANY (ARRAY['knowledge_entry_version'::text, 'artifact_document_version'::text]))) not valid;

alter table "app"."context_pack_scope_sources" validate constraint "context_pack_scope_sources_source_kind_check";

alter table "app"."context_pack_scope_sources" add constraint "context_pack_scope_sources_source_project_id_workspace_id_fkey" FOREIGN KEY (source_project_id, workspace_id) REFERENCES app.projects(id, workspace_id) not valid;

alter table "app"."context_pack_scope_sources" validate constraint "context_pack_scope_sources_source_project_id_workspace_id_fkey";

alter table "app"."context_pack_scope_sources" add constraint "context_pack_scope_sources_workspace_id_fkey" FOREIGN KEY (workspace_id) REFERENCES app.workspaces(id) not valid;

alter table "app"."context_pack_scope_sources" validate constraint "context_pack_scope_sources_workspace_id_fkey";

alter table "app"."context_pack_scope_versions" add constraint "context_pack_scope_versions_context_pack_id_project_id_fkey" FOREIGN KEY (context_pack_id, project_id) REFERENCES app.context_packs(id, project_id) not valid;

alter table "app"."context_pack_scope_versions" validate constraint "context_pack_scope_versions_context_pack_id_project_id_fkey";

alter table "app"."context_pack_scope_versions" add constraint "context_pack_scope_versions_context_pack_id_source_project__key" UNIQUE using index "context_pack_scope_versions_context_pack_id_source_project__key";

alter table "app"."context_pack_scope_versions" add constraint "context_pack_scope_versions_graph_version_check" CHECK ((graph_version >= 0)) not valid;

alter table "app"."context_pack_scope_versions" validate constraint "context_pack_scope_versions_graph_version_check";

alter table "app"."context_pack_scope_versions" add constraint "context_pack_scope_versions_project_id_workspace_id_fkey" FOREIGN KEY (project_id, workspace_id) REFERENCES app.projects(id, workspace_id) not valid;

alter table "app"."context_pack_scope_versions" validate constraint "context_pack_scope_versions_project_id_workspace_id_fkey";

alter table "app"."context_pack_scope_versions" add constraint "context_pack_scope_versions_source_project_id_workspace_id_fkey" FOREIGN KEY (source_project_id, workspace_id) REFERENCES app.projects(id, workspace_id) not valid;

alter table "app"."context_pack_scope_versions" validate constraint "context_pack_scope_versions_source_project_id_workspace_id_fkey";

alter table "app"."context_pack_scope_versions" add constraint "context_pack_scope_versions_workspace_id_fkey" FOREIGN KEY (workspace_id) REFERENCES app.workspaces(id) not valid;

alter table "app"."context_pack_scope_versions" validate constraint "context_pack_scope_versions_workspace_id_fkey";

set check_function_bodies = off;

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
        ((s.source_kind='knowledge_entry_version' and (v.id is null or v.version_number<>k.latest_version or k.status<>'confirmed'))
          or (s.source_kind='artifact_document_version' and (a.id is null or a.status<>'validated' or exists(
            select 1 from app.artifact_document_versions newer where newer.document_id=a.document_id
              and newer.status='validated' and newer.version>a.version)))));
$function$
;

CREATE OR REPLACE FUNCTION app.invalidate_scoped_artifact()
 RETURNS trigger
 LANGUAGE plpgsql
 SET search_path TO ''
AS $function$
declare head_status text;
begin
  if new.current_version_id is not distinct from old.current_version_id then return new; end if;
  select status into head_status from app.artifact_document_versions where id=new.current_version_id;
  -- Unvalidated drafts never replace a previously confirmed source in retrieval.
  if head_status<>'validated' then return new; end if;
  update app.projects set graph_version=graph_version+1 where id=new.project_id;
  with changed as (
    update app.context_packs p set status='stale',invalidated_at=now(),stale_reason='included artifact version was superseded'
    where p.workspace_id=new.workspace_id and p.status='current' and exists(
      select 1 from app.context_pack_scope_sources s join app.artifact_document_versions v on v.id=s.artifact_version_id
      where s.context_pack_id=p.id and s.decision='included' and v.document_id=new.id and v.id<>new.current_version_id)
      returning p.id
  ) update app.deliverables d set status='stale',stale_at=now()
    where d.source_context_pack_id in(select id from changed) and d.status in ('draft','committed');
  return new;
end;
$function$
;

CREATE OR REPLACE FUNCTION app.invalidate_scoped_knowledge()
 RETURNS trigger
 LANGUAGE plpgsql
 SET search_path TO ''
AS $function$
begin
  if row(new.latest_version,new.status) is not distinct from row(old.latest_version,old.status) then return new; end if;
  with changed as (
    update app.context_packs p set status='stale',invalidated_at=now(),stale_reason='included cross-scope knowledge was revised'
    where p.workspace_id=new.workspace_id and p.status='current' and exists(
      select 1 from app.context_pack_scope_sources s join app.knowledge_entry_versions v on v.id=s.knowledge_version_id
      where s.context_pack_id=p.id and s.decision='included' and v.knowledge_entry_id=new.id
        and (v.version_number<>new.latest_version or new.status<>'confirmed')) returning p.id
  ) update app.deliverables d set status='stale',stale_at=now()
    where d.source_context_pack_id in (select id from changed) and d.status in ('draft','committed');
  return new;
end;
$function$
;

CREATE OR REPLACE FUNCTION app.validate_scope_source_identity()
 RETURNS trigger
 LANGUAGE plpgsql
 SET search_path TO ''
AS $function$
declare actual_id uuid;
begin
  if new.source_kind='knowledge_entry_version' then
    select public_id into actual_id from app.knowledge_entry_versions where id=new.knowledge_version_id;
  else
    select public_id into actual_id from app.artifact_document_versions where id=new.artifact_version_id and status='validated';
  end if;
  if actual_id is null or actual_id<>new.source_public_id then
    raise exception 'scoped source identity does not match its immutable version' using errcode='23514';
  end if;
  return new;
end;
$function$
;

grant insert on table "app"."context_pack_scope_sources" to "ai_center_runtime";

grant select on table "app"."context_pack_scope_sources" to "ai_center_runtime";

grant insert on table "app"."context_pack_scope_versions" to "ai_center_runtime";

grant select on table "app"."context_pack_scope_versions" to "ai_center_runtime";


  create policy "scope_sources_read"
  on "app"."context_pack_scope_sources"
  as permissive
  for select
  to public
using (((workspace_id = app.current_workspace_id()) AND app.has_workspace_role(workspace_id, ARRAY['owner'::text, 'editor'::text, 'viewer'::text])));



  create policy "scope_sources_write"
  on "app"."context_pack_scope_sources"
  as permissive
  for insert
  to public
with check (((workspace_id = app.current_workspace_id()) AND app.has_workspace_role(workspace_id, ARRAY['owner'::text, 'editor'::text])));



  create policy "scope_versions_read"
  on "app"."context_pack_scope_versions"
  as permissive
  for select
  to public
using (((workspace_id = app.current_workspace_id()) AND app.has_workspace_role(workspace_id, ARRAY['owner'::text, 'editor'::text, 'viewer'::text])));



  create policy "scope_versions_write"
  on "app"."context_pack_scope_versions"
  as permissive
  for insert
  to public
with check (((workspace_id = app.current_workspace_id()) AND app.has_workspace_role(workspace_id, ARRAY['owner'::text, 'editor'::text])));


CREATE TRIGGER artifact_invalidate_scoped_sources AFTER UPDATE ON app.artifact_documents FOR EACH ROW EXECUTE FUNCTION app.invalidate_scoped_artifact();

CREATE TRIGGER scope_sources_append_only BEFORE DELETE OR UPDATE ON app.context_pack_scope_sources FOR EACH ROW EXECUTE FUNCTION app.prevent_append_only_mutation();

CREATE TRIGGER scope_sources_validate_identity BEFORE INSERT ON app.context_pack_scope_sources FOR EACH ROW EXECUTE FUNCTION app.validate_scope_source_identity();

CREATE TRIGGER scope_versions_append_only BEFORE DELETE OR UPDATE ON app.context_pack_scope_versions FOR EACH ROW EXECUTE FUNCTION app.prevent_append_only_mutation();

CREATE TRIGGER knowledge_invalidate_scoped_sources AFTER UPDATE ON app.knowledge_entries FOR EACH ROW EXECUTE FUNCTION app.invalidate_scoped_knowledge();



-- Preserve the full security contract of schema05.
revoke all on app.context_pack_scope_versions,app.context_pack_scope_sources from public,anon,authenticated,service_role;
grant select,insert on app.context_pack_scope_versions,app.context_pack_scope_sources to ai_center_runtime;
grant usage on sequence app.context_pack_scope_versions_id_seq,app.context_pack_scope_sources_id_seq to ai_center_runtime;
revoke all on function app.validate_scope_source_identity() from public,anon,authenticated,service_role;
revoke all on function app.context_pack_scopes_current(bigint) from public,anon,authenticated,service_role;
grant execute on function app.context_pack_scopes_current(bigint) to ai_center_runtime;
revoke all on function app.invalidate_scoped_knowledge() from public,anon,authenticated,service_role;
revoke all on function app.invalidate_scoped_artifact() from public,anon,authenticated,service_role;
alter table app.context_pack_scope_versions force row level security;
alter table app.context_pack_scope_sources force row level security;
