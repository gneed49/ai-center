alter table "app"."domain_events" add column "deferred_count" integer not null default 0;

alter table "app"."messages" add column "author_actor_id" uuid;

alter table "app"."messages" add column "command_public_id" uuid;

alter table "app"."messages" add column "submitted_content" text;

CREATE UNIQUE INDEX idempotency_message_scope_unique ON app.idempotency_records USING btree (public_id, workspace_id, project_id, actor_id);

CREATE INDEX messages_author_session_idx ON app.messages USING btree (session_id, author_actor_id, id DESC) WHERE ((role = 'user'::text) AND (command_public_id IS NOT NULL));

CREATE INDEX messages_command_scope_idx ON app.messages USING btree (command_public_id, workspace_id, project_id, author_actor_id) WHERE (command_public_id IS NOT NULL);

alter table "app"."domain_events" add constraint "domain_events_deferred_count_valid" CHECK (((deferred_count >= 0) AND (deferred_count <= attempt_count))) not valid;

alter table "app"."domain_events" validate constraint "domain_events_deferred_count_valid";

alter table "app"."idempotency_records" add constraint "idempotency_message_scope_unique" UNIQUE using index "idempotency_message_scope_unique";

alter table "app"."messages" add constraint "messages_command_scope_fkey" FOREIGN KEY (command_public_id, workspace_id, project_id, author_actor_id) REFERENCES app.idempotency_records(public_id, workspace_id, project_id, actor_id) ON DELETE SET NULL (command_public_id) not valid;

alter table "app"."messages" validate constraint "messages_command_scope_fkey";

alter table "app"."messages" add constraint "messages_recovery_fields_valid" CHECK (((command_public_id IS NULL) OR ((role = 'user'::text) AND (author_actor_id IS NOT NULL) AND (client_message_id IS NOT NULL) AND (submitted_content IS NOT NULL)))) not valid;

alter table "app"."messages" validate constraint "messages_recovery_fields_valid";

alter table "app"."messages" add constraint "messages_user_author_valid" CHECK (((author_actor_id IS NULL) OR (role = 'user'::text))) not valid;

alter table "app"."messages" validate constraint "messages_user_author_valid";

set check_function_bodies = off;

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

CREATE OR REPLACE FUNCTION app.operator_purge_workspace(requested_public_id uuid, confirmation uuid, expected_receipt text)
 RETURNS jsonb
 LANGUAGE plpgsql
 SET search_path TO ''
 SET lock_timeout TO '2s'
 SET statement_timeout TO '30s'
AS $function$
declare erasure_workspace_id bigint; relation_name text; manifest jsonb; row_count bigint; deleted jsonb='{}'; remaining bigint;
begin
 if current_user<>'postgres' or session_user<>'postgres' then
  raise exception 'Direct operator maintenance connection required' using errcode='42501';
 end if;
 if requested_public_id is null or confirmation is distinct from requested_public_id or expected_receipt is null then
  raise exception 'Exact workspace and manifest confirmation required' using errcode='22023';
 end if;
 -- Fixed order and a short lock timeout provide an atomic maintenance window.
 -- Readers remain available. Other tenants are never deletion predicates.
 for relation_name in select unnest(app.operator_purge_tables()) order by 1 loop
  execute format('lock table app.%I in share row exclusive mode',relation_name);
 end loop;
 manifest=app.operator_workspace_manifest(requested_public_id);
 if manifest->>'receipt' is distinct from expected_receipt then
  raise exception 'Workspace changed since preview; inspect a new manifest' using errcode='40001';
 end if;
 select id into erasure_workspace_id from app.workspaces where public_id=requested_public_id for update;
 if not exists(select 1 from app.workspace_automation_controls where workspace_id=erasure_workspace_id and not enabled)
  or exists(select 1 from app.projects where workspace_id=erasure_workspace_id and scope_kind='project' and status<>'archived') then
  raise exception 'Pause automation and archive all business projects before erasure' using errcode='55000';
 end if;
 if exists(select 1 from app.ai_call_reservations where workspace_id=erasure_workspace_id and status='running' and lease_until>clock_timestamp())
  or exists(select 1 from app.model_runs where workspace_id=erasure_workspace_id and status='running')
  or exists(select 1 from app.publication_jobs where workspace_id=erasure_workspace_id and status in('queued','processing'))
  or exists(select 1 from app.domain_events where workspace_id=erasure_workspace_id and status='processing')
  or exists(select 1 from app.idempotency_records where workspace_id=erasure_workspace_id and status='processing') then
  raise exception 'Company still has active work; settle it before erasure' using errcode='55000';
 end if;
 perform pg_catalog.set_config('app.operator_purge_workspace_id',erasure_workspace_id::text,true);
 perform pg_catalog.set_config('app.operator_purge_transaction_id',pg_catalog.txid_current()::text,true);
 foreach relation_name in array app.operator_purge_tables() loop
  execute format('delete from app.%I where %I=$1',relation_name,case when relation_name='workspaces' then 'id' else 'workspace_id' end) using erasure_workspace_id;
  get diagnostics row_count=row_count;
  deleted=deleted||jsonb_build_object(relation_name,row_count);
 end loop;
 foreach relation_name in array app.operator_purge_tables() loop
  execute format('select count(*) from app.%I where %I=$1',relation_name,case when relation_name='workspaces' then 'id' else 'workspace_id' end) into remaining using erasure_workspace_id;
  if remaining<>0 then raise exception 'Tenant rows remain after erasure' using errcode='55000'; end if;
 end loop;
 perform pg_catalog.set_config('app.operator_purge_workspace_id','',true);
 perform pg_catalog.set_config('app.operator_purge_transaction_id','',true);
 return jsonb_build_object('workspace_public_id',requested_public_id,'erased_at',clock_timestamp(),'deleted_rows',deleted,
   'manifest_receipt',expected_receipt,'format','ai-center-erasure-receipt-v1',
   'excluded',jsonb_build_array('Auth accounts','external destinations','backups and their retention','personal local provider subscriptions'));
end;
$function$
;

CREATE OR REPLACE FUNCTION app.operator_workspace_manifest(requested_public_id uuid)
 RETURNS jsonb
 LANGUAGE plpgsql
 SET search_path TO ''
AS $function$
declare erasure_workspace_id bigint; relation_name text; actual_tables text[]; counts jsonb='{}'; row_count bigint; total_rows bigint=0;
begin
 if current_user<>'postgres' or session_user<>'postgres' then
  raise exception 'Direct operator maintenance connection required' using errcode='42501';
 end if;
 select id into erasure_workspace_id from app.workspaces where public_id=requested_public_id;
 if erasure_workspace_id is null then raise exception 'Unknown exact workspace identifier' using errcode='22023'; end if;
 select array_agg(c.relname order by c.relname) into actual_tables from pg_catalog.pg_class c
  join pg_catalog.pg_namespace n on n.oid=c.relnamespace
  where n.nspname='app' and c.relkind in('r','p') and (c.relname='workspaces' or exists(
   select 1 from pg_catalog.pg_attribute a where a.attrelid=c.oid and a.attname='workspace_id' and not a.attisdropped));
 if actual_tables is distinct from (select array_agg(name order by name) from unnest(app.operator_purge_tables()) name) then
  raise exception 'Tenant table inventory changed; operator erasure needs review' using errcode='55000';
 end if;
 foreach relation_name in array app.operator_purge_tables() loop
  execute format('select count(*) from app.%I where %I=$1',relation_name,case when relation_name='workspaces' then 'id' else 'workspace_id' end)
   into row_count using erasure_workspace_id;
  counts=counts||jsonb_build_object(relation_name,row_count);
  total_rows=total_rows+row_count;
 end loop;
 if total_rows>100000 then raise exception 'Operator erasure exceeds its 100000-row maintenance budget' using errcode='54000'; end if;
 return jsonb_build_object('workspace_public_id',requested_public_id,'row_counts',counts,'total_rows',total_rows,
  'receipt',md5(requested_public_id::text||counts::text),'format','ai-center-erasure-manifest-v1');
end;
$function$
;

CREATE OR REPLACE FUNCTION app.prevent_append_only_mutation()
 RETURNS trigger
 LANGUAGE plpgsql
 SET search_path TO ''
AS $function$
begin
 if tg_op='DELETE' and current_user='postgres' and session_user='postgres'
  and nullif(current_setting('app.operator_purge_workspace_id',true),'')=old.workspace_id::text
  and nullif(current_setting('app.operator_purge_transaction_id',true),'')=pg_catalog.txid_current()::text then return old; end if;
 raise exception '% rows are append-only',tg_table_name;
end;
$function$
;

CREATE OR REPLACE FUNCTION app.prevent_audit_mutation()
 RETURNS trigger
 LANGUAGE plpgsql
 SET search_path TO ''
AS $function$
begin
 if tg_op='DELETE' and current_user='postgres' and session_user='postgres'
  and nullif(current_setting('app.operator_purge_workspace_id',true),'')=old.workspace_id::text
  and nullif(current_setting('app.operator_purge_transaction_id',true),'')=pg_catalog.txid_current()::text then return old; end if;
 raise exception 'audit_events are immutable';
end;
$function$
;


  create policy "messages_author_insert"
  on "app"."messages"
  as restrictive
  for insert
  to public
with check (((author_actor_id IS NULL) OR (author_actor_id = ( SELECT app.current_actor_id() AS current_actor_id))));




-- Function ACLs are explicit: erasure remains direct-operator maintenance only.
revoke all on function app.operator_purge_tables() from public,anon,authenticated,service_role,ai_center_runtime;
revoke all on function app.operator_workspace_manifest(uuid) from public,anon,authenticated,service_role,ai_center_runtime;
revoke all on function app.operator_purge_workspace(uuid,uuid,text) from public,anon,authenticated,service_role,ai_center_runtime;
revoke all on function app.prevent_append_only_mutation() from public,anon,authenticated,service_role,ai_center_runtime;
revoke all on function app.prevent_audit_mutation() from public,anon,authenticated,service_role,ai_center_runtime;
