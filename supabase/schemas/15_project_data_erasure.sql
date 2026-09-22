-- Private maintenance of an isolated project. No runtime/API grant.
-- The reviewed catalog fingerprint is filled from the isolated migrated schema.
create or replace function app.operator_project_expected_schema()
returns text language sql immutable security invoker set search_path='' as $$
 select 'f17b7f9d7be7197bb88bb69e2fc9d942'::text;
$$;
revoke all on function app.operator_project_expected_schema() from public,anon,authenticated,service_role,ai_center_runtime;

create or replace function app.operator_project_schema_fingerprint()
returns text language sql stable security invoker set search_path='' as $$
 with inventory as (
 select 0 as category,c.relname as ordering,''::name secondary,'T|'||c.relname as item
 from pg_catalog.pg_class c join pg_catalog.pg_namespace n on n.oid=c.relnamespace where n.nspname='app' and c.relkind in('r','p')
 union all
 select 1,c.relname,a.attname,'C|'||c.relname||'|'||a.attname||'|'||pg_catalog.format_type(a.atttypid,a.atttypmod)
 from pg_catalog.pg_class c join pg_catalog.pg_namespace n on n.oid=c.relnamespace join pg_catalog.pg_attribute a on a.attrelid=c.oid and a.attnum>0 and not a.attisdropped
 where n.nspname='app' and c.relkind in('r','p')
 union all
 select 2,s.relname,c.conname,'F|'||s.relname||'|'||c.conname||'|'||tn.nspname||'|'||t.relname||'|'||
  (select string_agg(a.attname,',' order by k.ordinality) from unnest(c.conkey) with ordinality k(attnum,ordinality) join pg_catalog.pg_attribute a on a.attrelid=s.oid and a.attnum=k.attnum)||'|'||
  (select string_agg(a.attname,',' order by k.ordinality) from unnest(c.confkey) with ordinality k(attnum,ordinality) join pg_catalog.pg_attribute a on a.attrelid=t.oid and a.attnum=k.attnum)||'|'||
  c.confdeltype::text||'|'||case when c.condeferrable then '1' else '0' end||'|'||case when c.condeferred then '1' else '0' end
 from pg_catalog.pg_constraint c join pg_catalog.pg_class s on s.oid=c.conrelid join pg_catalog.pg_namespace sn on sn.oid=s.relnamespace
 join pg_catalog.pg_class t on t.oid=c.confrelid join pg_catalog.pg_namespace tn on tn.oid=t.relnamespace where sn.nspname='app' and c.contype='f'
 ) select md5(string_agg(item,E'\n' order by category,ordering,secondary)) from inventory;
$$;
revoke all on function app.operator_project_schema_fingerprint() from public,anon,authenticated,service_role,ai_center_runtime;

-- Trusted SQL fragments use t as row alias and $1=project/$2=workspace/$3=UUID.
-- Every tenant table is classified, including explicitly retained shared rows.
create or replace function app.operator_project_predicate(relation_name text)
returns text language plpgsql immutable security invoker set search_path='' as $$
begin
 case relation_name
 when 'ai_call_reservations','provider_connections','provider_selections','tool_connections','work_tool_connections',
      'workspace_automation_controls','workspace_invitations','workspace_members','workspaces' then return 'false';
 when 'projects' then return 't.id=$1';
 when 'artifact_version_sources' then return 'exists(select 1 from app.artifact_document_versions v where v.id=t.version_id and v.project_id=$1)';
 when 'publication_observations' then return 'exists(select 1 from app.publication_jobs j where j.id=t.publication_job_id and j.project_id=$1)';
 when 'audit_events' then return '(t.project_id=$1 or (t.project_id is null and t.object_kind=''project'' and t.object_public_id=$3))';
 when 'artifact_destination_settings','context_pack_scope_sources','context_pack_scope_versions','context_pack_selection_items',
      'context_pack_sources','deliverable_sources','domain_events','edges','execution_events','gates','handoffs',
      'insight_resolutions','insight_sources','mutation_proposals','requirement_coverage','steward_assessment_sources',
      'steward_scope_sources','evidences','external_reference_observations','github_code_file_observations','insights',
      'messages','idempotency_records','artifacts','deliverable_sections','github_code_corpora','knowledge_entry_versions',
      'publication_jobs','steward_assessments','artifact_document_versions','deliverables','executions','external_references',
      'knowledge_entries','model_runs','artifact_documents','sessions','tasks','context_packs','context_nodes' then return 't.project_id=$1';
 else raise exception 'Unclassified project erasure table' using errcode='55000';
 end case;
end;
$$;
revoke all on function app.operator_project_predicate(text) from public,anon,authenticated,service_role,ai_center_runtime;

create or replace function app.operator_project_manifest(requested_workspace uuid,requested_project uuid)
returns jsonb language plpgsql security invoker set search_path='' as $$
declare erasure_workspace_id bigint; erasure_project_id bigint; project_status text; relation_name text; predicate text;
 row_ids jsonb='{}'; counts jsonb='{}'; ids jsonb; found_count bigint; total_rows bigint=0; blockers jsonb='[]';
 owned_public_ids text[]='{}'; other_public_ids text[]='{}'; found_uuids text[]; workspace_column text;
 fk record; join_expression text; source_ids bigint[]; target_ids bigint[]; fingerprint text;
begin
 if current_user<>'postgres' or session_user<>'postgres' then raise exception 'Direct operator maintenance connection required' using errcode='42501'; end if;
 -- Reuses the closed tenant-table inventory and overall maintenance row budget.
 perform app.operator_workspace_manifest(requested_workspace);
 fingerprint=app.operator_project_schema_fingerprint();
 if fingerprint<>app.operator_project_expected_schema() then raise exception 'Project schema inventory changed; review all columns and foreign keys before erasure' using errcode='55000'; end if;
 select p.workspace_id,p.id,p.status into erasure_workspace_id,erasure_project_id,project_status
  from app.projects p join app.workspaces w on w.id=p.workspace_id
  where w.public_id=requested_workspace and p.public_id=requested_project and p.scope_kind='project';
 if erasure_project_id is null then raise exception 'Unknown exact business project in workspace' using errcode='22023'; end if;
 foreach relation_name in array app.operator_purge_tables() loop
  predicate=app.operator_project_predicate(relation_name);
  if predicate='false' then row_ids=row_ids||jsonb_build_object(relation_name,'[]'::jsonb); counts=counts||jsonb_build_object(relation_name,0); continue; end if;
  execute format('select coalesce(jsonb_agg(t.id order by t.id),''[]''),count(*),coalesce(array_agg(to_jsonb(t)->>''public_id'') filter(where to_jsonb(t)->>''public_id'' is not null),''{}'') from app.%I t where t.workspace_id=$2 and (%s)',relation_name,predicate)
   into ids,found_count,found_uuids using erasure_project_id,erasure_workspace_id,requested_project;
  row_ids=row_ids||jsonb_build_object(relation_name,ids); counts=counts||jsonb_build_object(relation_name,found_count);
  total_rows=total_rows+found_count; owned_public_ids=owned_public_ids||found_uuids;
  -- Other project identities support conservative detection of outbound JSON links.
  -- Shared company identities are also scopes and deliberately count as links.
  execute format('select coalesce(array_agg(to_jsonb(t)->>''public_id'') filter(where to_jsonb(t)->>''public_id'' is not null),''{}'') from app.%I t where t.workspace_id=$2 and not coalesce((%s),false)',relation_name,predicate)
   into found_uuids using erasure_project_id,erasure_workspace_id,requested_project;
  other_public_ids=other_public_ids||found_uuids;
 end loop;
 if total_rows>100000 then raise exception 'Project exceeds maintenance row budget' using errcode='54000'; end if;
 -- Every FK is examined, including CASCADE and SET NULL: they may never alter a
 -- row outside this project's explicit deletion set as a side effect.
 for fk in select c.conname,c.conrelid,c.confrelid,c.conkey,c.confkey,s.relname source_table,t.relname target_table,
   sn.nspname source_schema,tn.nspname target_schema
  from pg_catalog.pg_constraint c join pg_catalog.pg_class s on s.oid=c.conrelid join pg_catalog.pg_namespace sn on sn.oid=s.relnamespace
  join pg_catalog.pg_class t on t.oid=c.confrelid join pg_catalog.pg_namespace tn on tn.oid=t.relnamespace
  where c.contype='f' and (sn.nspname='app' or tn.nspname='app')
 loop
  if fk.source_schema<>'app' or fk.target_schema<>'app' then raise exception 'Cross-schema foreign key requires project erasure review' using errcode='55000'; end if;
  if not(row_ids ? fk.source_table) or not(row_ids ? fk.target_table) then
   -- Global catalogs can be referenced, but can never reference project data.
   if fk.source_table=any(array['project_templates','agent_profiles','deliverable_contracts']) or fk.target_table=any(array['project_templates','agent_profiles','deliverable_contracts']) then continue; end if;
   raise exception 'Unclassified foreign key table' using errcode='55000';
  end if;
  select coalesce(array_agg(value::bigint),'{}') into source_ids from jsonb_array_elements_text(row_ids->fk.source_table);
  select coalesce(array_agg(value::bigint),'{}') into target_ids from jsonb_array_elements_text(row_ids->fk.target_table);
  if cardinality(source_ids)=0 and cardinality(target_ids)=0 then continue; end if;
  select string_agg(format('s.%I=t.%I',sa.attname,ta.attname),' and ' order by k.ordinality) into join_expression
   from unnest(fk.conkey,fk.confkey) with ordinality k(source_attnum,target_attnum,ordinality)
   join pg_catalog.pg_attribute sa on sa.attrelid=fk.conrelid and sa.attnum=k.source_attnum
   join pg_catalog.pg_attribute ta on ta.attrelid=fk.confrelid and ta.attnum=k.target_attnum;
  execute format('select count(*) from app.%I s join app.%I t on %s where coalesce((to_jsonb(t)->>''id'')::bigint=any($2),false) and not coalesce((to_jsonb(s)->>''id'')::bigint=any($1),false)',fk.source_table,fk.target_table,join_expression)
   into found_count using source_ids,target_ids;
  if found_count>0 then blockers=blockers||jsonb_build_array(jsonb_build_object('kind','incoming_foreign_key','table',fk.source_table,'constraint',fk.conname,'count',found_count)); end if;
  if app.operator_project_predicate(fk.target_table)<>'false' then
   execute format('select count(*) from app.%I s join app.%I t on %s where coalesce((to_jsonb(s)->>''id'')::bigint=any($1),false) and not coalesce((to_jsonb(t)->>''id'')::bigint=any($2),false)',fk.source_table,fk.target_table,join_expression)
    into found_count using source_ids,target_ids;
   if found_count>0 then blockers=blockers||jsonb_build_array(jsonb_build_object('kind','outgoing_foreign_key','table',fk.source_table,'constraint',fk.conname,'count',found_count)); end if;
  end if;
 end loop;
 -- Covers polymorphic UUIDs and UUID arrays, including model run sources,
 -- structured snapshots, message provenance and cached command responses.
 foreach relation_name in array app.operator_purge_tables() loop
  workspace_column=case when relation_name='workspaces' then 'id' else 'workspace_id' end;
  select coalesce(array_agg(value::bigint),'{}') into source_ids from jsonb_array_elements_text(row_ids->relation_name);
  execute format('select count(*) from app.%I t where t.%I=$1 and not coalesce((to_jsonb(t)->>''id'')::bigint=any($2),false) and exists(select 1 from jsonb_path_query(to_jsonb(t),''$.**'') atom where jsonb_typeof(atom)=''string'' and atom#>>''{}''=any($3))',relation_name,workspace_column)
   into found_count using erasure_workspace_id,source_ids,owned_public_ids;
  if found_count>0 then blockers=blockers||jsonb_build_array(jsonb_build_object('kind','incoming_structured_reference','table',relation_name,'count',found_count)); end if;
  if cardinality(source_ids)>0 then
   execute format('select count(*) from app.%I t where t.workspace_id=$1 and t.id=any($2) and exists(select 1 from jsonb_path_query(to_jsonb(t),''$.**'') atom where jsonb_typeof(atom)=''string'' and atom#>>''{}''=any($3))',relation_name)
    into found_count using erasure_workspace_id,source_ids,other_public_ids;
   if found_count>0 then blockers=blockers||jsonb_build_array(jsonb_build_object('kind','outgoing_structured_reference','table',relation_name,'count',found_count)); end if;
  end if;
 end loop;
 return jsonb_build_object('format','ai-center-project-erasure-manifest-v1','workspace_public_id',requested_workspace,'project_public_id',requested_project,
  'project_status',project_status,'row_counts',counts,'total_rows',total_rows,'blockers',blockers,'eligible',jsonb_array_length(blockers)=0,
  'schema_fingerprint',fingerprint,'receipt',md5(requested_workspace::text||requested_project::text||counts::text||blockers::text||fingerprint));
end;
$$;
revoke all on function app.operator_project_manifest(uuid,uuid) from public,anon,authenticated,service_role,ai_center_runtime;

-- Defense against an unexpected DELETE trigger changing a retained row. This
-- guard covers every application tenant and global catalog, not only neighbors
-- in the same company. Its strict global budget is an explicit V1 limitation.
create or replace function app.operator_project_preserved_fingerprint(deletion_rows jsonb)
returns text language plpgsql security invoker set search_path='' as $$
declare relation_name text; excluded_ids bigint[]; row_count bigint; row_bytes bigint;
 total_rows bigint=0; total_bytes bigint=0; table_hash text; hashes jsonb='{}';
begin
 if current_user<>'postgres' or session_user<>'postgres' then raise exception 'Direct operator maintenance connection required' using errcode='42501'; end if;
 foreach relation_name in array app.operator_purge_tables()||array['project_templates','agent_profiles','deliverable_contracts'] loop
  select coalesce(array_agg(value::bigint),'{}') into excluded_ids from jsonb_array_elements_text(coalesce(deletion_rows->relation_name,'[]'));
  execute format('select count(*),coalesce(sum(octet_length(to_jsonb(t)::text)),0)::bigint from app.%I t where not coalesce((to_jsonb(t)->>''id'')::bigint=any($1),false)',relation_name)
   into row_count,row_bytes using excluded_ids;
  total_rows=total_rows+row_count;total_bytes=total_bytes+row_bytes;
  if total_rows>100000 or total_bytes>67108864 then raise exception 'Project erasure retained-data verification exceeds 100000 rows or 64 MiB across the instance' using errcode='54000'; end if;
  execute format('select md5(coalesce(jsonb_agg(to_jsonb(t) order by to_jsonb(t)::text)::text,''[]'')) from app.%I t where not coalesce((to_jsonb(t)->>''id'')::bigint=any($1),false)',relation_name)
   into table_hash using excluded_ids;
  hashes=hashes||jsonb_build_object(relation_name,table_hash);
 end loop;
 return md5(hashes::text);
end;
$$;
revoke all on function app.operator_project_preserved_fingerprint(jsonb) from public,anon,authenticated,service_role,ai_center_runtime;

-- The caller is a direct maintenance session, and the exact deletion set is
-- captured before any delete. The immutable-row exception cannot spread to a
-- second project or be enabled by runtime-set custom GUCs.
create or replace function app.operator_erasure_row_allowed(relation_name text,previous_row jsonb)
returns boolean language plpgsql stable security invoker set search_path='' as $$
declare allowed_rows jsonb;
begin
 if current_user<>'postgres' or session_user<>'postgres'
  or nullif(current_setting('app.operator_purge_workspace_id',true),'') is distinct from previous_row->>'workspace_id'
  or nullif(current_setting('app.operator_purge_transaction_id',true),'') is distinct from pg_catalog.txid_current()::text then return false; end if;
 if nullif(current_setting('app.operator_purge_project_id',true),'') is null then return true; end if;
 allowed_rows=nullif(current_setting('app.operator_purge_project_rows',true),'')::jsonb;
 return coalesce(allowed_rows->relation_name @> jsonb_build_array((previous_row->>'id')::bigint),false);
end;
$$;
revoke all on function app.operator_erasure_row_allowed(text,jsonb) from public,anon,authenticated,service_role,ai_center_runtime;

create or replace function app.prevent_append_only_mutation()
returns trigger language plpgsql security invoker set search_path='' as $$
begin
 if tg_op='DELETE' and current_user='postgres' and session_user='postgres'
  and app.operator_erasure_row_allowed(tg_table_name,to_jsonb(old)) then return old; end if;
 raise exception '% rows are append-only',tg_table_name;
end;
$$;
revoke all on function app.prevent_append_only_mutation() from public,anon,authenticated,service_role,ai_center_runtime;
create or replace function app.prevent_audit_mutation()
returns trigger language plpgsql security invoker set search_path='' as $$
begin
 if tg_op='DELETE' and current_user='postgres' and session_user='postgres'
  and app.operator_erasure_row_allowed(tg_table_name,to_jsonb(old)) then return old; end if;
 raise exception 'audit_events are immutable';
end;
$$;
revoke all on function app.prevent_audit_mutation() from public,anon,authenticated,service_role,ai_center_runtime;

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
