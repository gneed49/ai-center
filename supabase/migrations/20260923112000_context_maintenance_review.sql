-- Reviewed maintenance guards for the durable company steward.

create or replace function app.operator_purge_workspace(requested_public_id uuid, confirmation uuid, expected_receipt text)
returns jsonb language plpgsql security invoker set search_path='' set lock_timeout='2s' set statement_timeout='30s' as $$
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
  or exists(select 1 from app.steward_scan_progress where workspace_id=erasure_workspace_id and status='running')
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
$$;
revoke all on function app.operator_purge_workspace(uuid,uuid,text) from public,anon,authenticated,service_role,ai_center_runtime;

create or replace function app.operator_purge_project(requested_workspace uuid,requested_project uuid,confirmation uuid,expected_receipt text)
returns jsonb language plpgsql security invoker set search_path='' set lock_timeout='2s' set statement_timeout='30s' as $$
declare erasure_workspace_id bigint; erasure_project_id bigint; relation_name text; predicate text; manifest jsonb; retained_before text;
 rows_to_delete jsonb='{}'; ids jsonb; row_count bigint; deleted jsonb='{}'; remaining bigint;
begin
 if current_user<>'postgres' or session_user<>'postgres' then raise exception 'Direct operator maintenance connection required' using errcode='42501'; end if;
 if requested_project is null or confirmation is distinct from requested_project or expected_receipt is null then raise exception 'Exact project and receipt confirmation required' using errcode='22023'; end if;
 for relation_name in select unnest(app.operator_purge_tables()||array['project_templates','agent_profiles','deliverable_contracts']) order by 1 loop
  execute format('lock table app.%I in share row exclusive mode',relation_name);
 end loop;
 manifest=app.operator_project_manifest(requested_workspace,requested_project);
 if manifest->>'receipt' is distinct from expected_receipt then raise exception 'Project changed since preview' using errcode='40001'; end if;
 if not (manifest->>'eligible')::boolean then raise exception 'Project has dependencies outside its deletion scope; inspect manifest blockers' using errcode='55000'; end if;
 select p.id,p.workspace_id into erasure_project_id,erasure_workspace_id from app.projects p join app.workspaces w on w.id=p.workspace_id
  where p.public_id=requested_project and w.public_id=requested_workspace and p.scope_kind='project' and p.status='archived' for update of p;
 if erasure_project_id is null or not exists(select 1 from app.workspace_automation_controls c where c.workspace_id=erasure_workspace_id and not c.enabled) then
  raise exception 'Archive project and pause company automation before erasure' using errcode='55000'; end if;
 -- Shared reservations cannot be assigned to a project, so active work in the
 -- company conservatively prevents this maintenance operation.
 if exists(select 1 from app.ai_call_reservations r where r.workspace_id=erasure_workspace_id and r.status='running' and r.lease_until>clock_timestamp())
  or exists(select 1 from app.steward_scan_progress r where r.workspace_id=erasure_workspace_id and r.status='running')
  or exists(select 1 from app.model_runs r where r.workspace_id=erasure_workspace_id and r.status='running')
  or exists(select 1 from app.publication_jobs j where j.workspace_id=erasure_workspace_id and j.status in('queued','processing'))
  or exists(select 1 from app.domain_events e where e.workspace_id=erasure_workspace_id and e.status='processing')
  or exists(select 1 from app.idempotency_records c where c.workspace_id=erasure_workspace_id and c.status='processing') then
  raise exception 'Company still has active work' using errcode='55000'; end if;
 foreach relation_name in array app.operator_purge_tables() loop
  predicate=app.operator_project_predicate(relation_name);
  if predicate='false' then continue; end if;
  execute format('select coalesce(jsonb_agg(t.id order by t.id),''[]'') from app.%I t where t.workspace_id=$2 and (%s)',relation_name,predicate)
   into ids using erasure_project_id,erasure_workspace_id,requested_project;
  rows_to_delete=rows_to_delete||jsonb_build_object(relation_name,ids);
 end loop;
 retained_before=app.operator_project_preserved_fingerprint(rows_to_delete);
 perform set_config('app.operator_purge_workspace_id',erasure_workspace_id::text,true);
 perform set_config('app.operator_purge_project_id',erasure_project_id::text,true);
 perform set_config('app.operator_purge_transaction_id',pg_catalog.txid_current()::text,true);
 perform set_config('app.operator_purge_project_rows',rows_to_delete::text,true);
 foreach relation_name in array app.operator_purge_tables() loop
  if not (rows_to_delete ? relation_name) then continue; end if;
  execute format('delete from app.%I where workspace_id=$1 and id in(select value::bigint from jsonb_array_elements_text($2))',relation_name)
   using erasure_workspace_id,rows_to_delete->relation_name;
  get diagnostics row_count=row_count;
  deleted=deleted||jsonb_build_object(relation_name,row_count);
 end loop;
 foreach relation_name in array app.operator_purge_tables() loop
  if not (rows_to_delete ? relation_name) then continue; end if;
  execute format('select count(*) from app.%I where workspace_id=$1 and id in(select value::bigint from jsonb_array_elements_text($2))',relation_name)
   into remaining using erasure_workspace_id,rows_to_delete->relation_name;
  if remaining<>0 then raise exception 'Project rows remain after erasure' using errcode='55000'; end if;
 end loop;
 if app.operator_project_schema_fingerprint()<>app.operator_project_expected_schema()
  or app.operator_project_preserved_fingerprint(rows_to_delete) is distinct from retained_before then
  raise exception 'Retained application data changed; project erasure rolled back' using errcode='55000'; end if;
 perform set_config('app.operator_purge_workspace_id','',true);
 perform set_config('app.operator_purge_project_id','',true);
 perform set_config('app.operator_purge_transaction_id','',true);
 perform set_config('app.operator_purge_project_rows','',true);
 return jsonb_build_object('format','ai-center-project-erasure-receipt-v1','workspace_public_id',requested_workspace,'project_public_id',requested_project,
  'erased_at',clock_timestamp(),'deleted_rows',deleted,'manifest_receipt',expected_receipt,'retained_application_data_verified',true,
  'excluded',jsonb_build_array('shared company settings and identities','free-text mentions without canonical references','Auth accounts','remote destinations','backups'));
end;
$$;
revoke all on function app.operator_purge_project(uuid,uuid,uuid,text) from public,anon,authenticated,service_role,ai_center_runtime;

-- Reviewed: two steward tables and exact artifact version source FK.
create or replace function app.operator_project_expected_schema()
returns text language sql immutable security invoker set search_path='' as $$
 select 'b6d89bfc133c23645963435f4a261329'::text;
$$;
revoke all on function app.operator_project_expected_schema() from public,anon,authenticated,service_role,ai_center_runtime;
