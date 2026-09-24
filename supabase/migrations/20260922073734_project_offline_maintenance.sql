set check_function_bodies = off;

CREATE OR REPLACE FUNCTION app.operator_abandoned_manifest(requested_workspace uuid)
 RETURNS jsonb
 LANGUAGE plpgsql
 SET search_path TO ''
AS $function$
declare target_workspace_id bigint; paused_at timestamptz; control_generation bigint; counts jsonb; blockers jsonb='[]';
 candidate_fingerprint text; candidate_count bigint; unsafe_count bigint;
begin
 if current_user<>'postgres' or session_user<>'postgres' then raise exception 'Direct operator maintenance connection required' using errcode='42501'; end if;
 perform app.operator_workspace_manifest(requested_workspace);
 select w.id,c.updated_at,c.generation into target_workspace_id,paused_at,control_generation
  from app.workspaces w left join app.workspace_automation_controls c on c.workspace_id=w.id and not c.enabled
  where w.public_id=requested_workspace;
 if paused_at is null or paused_at>clock_timestamp()-interval '12 minutes' then
  blockers=blockers||jsonb_build_array(jsonb_build_object('kind','continuous_pause_under_12_minutes'));
 end if;
 select count(*),coalesce(md5(jsonb_agg(to_jsonb(r) order by r.table_name,r.row_id)::text),md5('[]'))
  into candidate_count,candidate_fingerprint from app.operator_abandoned_rows(target_workspace_id) r;
 if candidate_count>10000 then raise exception 'Abandoned maintenance exceeds 10000-operation budget' using errcode='54000'; end if;
 select coalesce(jsonb_object_agg(table_name,total),'{}') into counts from (
  select table_name,count(*) total from app.operator_abandoned_rows(target_workspace_id) group by table_name
 ) grouped;
 select count(*) into unsafe_count from app.operator_abandoned_rows(target_workspace_id) r where r.lease_until>clock_timestamp();
 if unsafe_count>0 then blockers=blockers||jsonb_build_array(jsonb_build_object('kind','active_leases','count',unsafe_count)); end if;
 -- Even completed activity after the pause requires a fresh offline window.
 select count(*) into unsafe_count from (
  select greatest(r.created_at,r.finished_at) activity from app.ai_call_reservations r where r.workspace_id=target_workspace_id
  union all select greatest(r.created_at,r.updated_at,r.started_at,r.completed_at) from app.model_runs r where r.workspace_id=target_workspace_id
  union all select greatest(r.created_at,r.updated_at) from app.idempotency_records r where r.workspace_id=target_workspace_id
  union all select greatest(r.occurred_at,r.locked_at,r.processed_at,r.failed_at) from app.domain_events r where r.workspace_id=target_workspace_id
  union all select greatest(r.created_at,r.updated_at) from app.publication_jobs r where r.workspace_id=target_workspace_id
 ) activity where activity.activity>paused_at;
 if unsafe_count>0 then blockers=blockers||jsonb_build_array(jsonb_build_object('kind','activity_after_pause','count',unsafe_count)); end if;
 return jsonb_build_object('format','ai-center-abandoned-manifest-v1','workspace_public_id',requested_workspace,
  'paused_at',paused_at,'control_generation',control_generation,'counts',counts,'total_operations',candidate_count,
  'blockers',blockers,'eligible',jsonb_array_length(blockers)=0,'remote_outcome','unknown',
  'receipt',md5(requested_workspace::text||coalesce(paused_at::text,'')||coalesce(control_generation::text,'')||candidate_fingerprint||blockers::text));
end;
$function$
;

CREATE OR REPLACE FUNCTION app.operator_abandoned_rows(requested_workspace_id bigint)
 RETURNS TABLE(table_name text, row_id bigint, public_id uuid, status text, created_at timestamp with time zone, last_activity_at timestamp with time zone, lease_until timestamp with time zone, lease_marker text)
 LANGUAGE sql
 STABLE
 SET search_path TO ''
AS $function$
 select 'ai_call_reservations',r.id,r.public_id,r.status,r.created_at,r.created_at,r.lease_until,r.control_generation::text
 from app.ai_call_reservations r where r.workspace_id=requested_workspace_id and r.status='running'
 union all
 select 'model_runs',r.id,r.public_id,r.status,r.created_at,greatest(r.created_at,r.updated_at,r.started_at),null,r.attempt_count::text
 from app.model_runs r where r.workspace_id=requested_workspace_id and r.status in('pending','running')
 union all
 select 'idempotency_records',r.id,r.public_id,r.status,r.created_at,greatest(r.created_at,r.updated_at),r.locked_until,r.lease_generation::text
 from app.idempotency_records r where r.workspace_id=requested_workspace_id and r.status='processing'
 union all
 select 'domain_events',r.id,r.public_id,r.status,r.occurred_at,greatest(r.occurred_at,r.locked_at),r.locked_until,r.attempt_count::text||':'||coalesce(r.locked_by,'')
 from app.domain_events r where r.workspace_id=requested_workspace_id and r.status='processing'
 union all
 select 'publication_jobs',r.id,r.public_id,r.status,r.created_at,greatest(r.created_at,r.updated_at),r.lease_until,coalesce(r.lease_token::text,'')
 from app.publication_jobs r where r.workspace_id=requested_workspace_id
  and (r.status in('queued','processing') or (r.status='needs_review' and r.lease_token is not null));
$function$
;

CREATE OR REPLACE FUNCTION app.operator_close_abandoned(requested_workspace uuid, confirmation uuid, expected_receipt text, workers_stopped boolean, operator_actor uuid)
 RETURNS jsonb
 LANGUAGE plpgsql
 SET search_path TO ''
 SET lock_timeout TO '2s'
 SET statement_timeout TO '30s'
AS $function$
declare target_workspace_id bigint; relation_name text; manifest jsonb; closed_counts jsonb='{}'; affected_count bigint; audit_id uuid=gen_random_uuid();
begin
 if current_user<>'postgres' or session_user<>'postgres' then raise exception 'Direct operator maintenance connection required' using errcode='42501'; end if;
 if requested_workspace is null or confirmation is distinct from requested_workspace or expected_receipt is null
  or workers_stopped is distinct from true or operator_actor is null or operator_actor='00000000-0000-0000-0000-000000000000'::uuid then
  raise exception 'Exact target, receipt, operator identity and explicit stopped-process confirmation required' using errcode='22023';
 end if;
 for relation_name in select unnest(array['ai_call_reservations','audit_events','domain_events','idempotency_records','model_runs','publication_jobs','workspace_automation_controls','workspaces']) order by 1 loop
  execute format('lock table app.%I in share row exclusive mode',relation_name);
 end loop;
 manifest=app.operator_abandoned_manifest(requested_workspace);
 if manifest->>'receipt' is distinct from expected_receipt then raise exception 'Abandoned work changed since preview' using errcode='40001'; end if;
 if not (manifest->>'eligible')::boolean then raise exception 'Offline maintenance conditions are not satisfied; inspect preview blockers' using errcode='55000'; end if;
 select id into target_workspace_id from app.workspaces where public_id=requested_workspace for update;
 update app.ai_call_reservations set status='cancelled',finished_at=clock_timestamp()
  where workspace_id=target_workspace_id and status='running';
 get diagnostics affected_count=row_count;
 closed_counts=closed_counts||jsonb_build_object('ai_call_reservations',affected_count);
 update app.model_runs set status='cancelled',completed_at=clock_timestamp(),error_class='operator_abandoned_closed',
  error_message='Closed during confirmed offline maintenance; remote outcome unknown.'
  where workspace_id=target_workspace_id and status in('pending','running');
 get diagnostics affected_count=row_count;
 closed_counts=closed_counts||jsonb_build_object('model_runs',affected_count);
 update app.idempotency_records set status='failed',locked_until=null,lease_generation=gen_random_uuid(),
  error_code='operator_abandoned_closed',response_status=409,
  response_body=jsonb_build_object('code','operator_abandoned_closed','message','Traitement clôturé pendant une maintenance. Le résultat distant reste inconnu. Aucun nouvel appel automatique.','retryable',false)
  where workspace_id=target_workspace_id and status='processing';
 get diagnostics affected_count=row_count;
 closed_counts=closed_counts||jsonb_build_object('idempotency_records',affected_count);
 update app.domain_events set status='dead_letter',locked_at=null,locked_until=null,locked_by=null,processed_at=null,failed_at=clock_timestamp(),
  last_error_code='operator_abandoned_closed',last_error_message='Closed during confirmed offline maintenance; remote outcome unknown.'
  where workspace_id=target_workspace_id and status='processing';
 get diagnostics affected_count=row_count;
 closed_counts=closed_counts||jsonb_build_object('domain_events',affected_count);
 -- Invalidating the capability is essential: late settlement accepts needs_review.
 update app.publication_jobs set status=case when status='queued' then 'cancelled' else 'needs_review' end,
  lease_token=null,lease_until=null,updated_at=clock_timestamp(),error_code='operator_abandoned_closed'
  where workspace_id=target_workspace_id and (status in('queued','processing') or (status='needs_review' and lease_token is not null));
 get diagnostics affected_count=row_count;
 closed_counts=closed_counts||jsonb_build_object('publication_jobs',affected_count);
 insert into app.audit_events(public_id,workspace_id,actor_id,action,object_kind,object_public_id,after_state)
  values(audit_id,target_workspace_id,operator_actor,'maintenance.abandoned_closed','workspace',requested_workspace,
   jsonb_build_object('closed_counts',closed_counts,'remote_outcome','unknown','workers_stopped_confirmed',true,'operator_identity','operator_supplied_reference'));
 return jsonb_build_object('format','ai-center-abandoned-receipt-v1','workspace_public_id',requested_workspace,
  'closed_at',clock_timestamp(),'closed_counts',closed_counts,'audit_public_id',audit_id,'remote_outcome','unknown','manifest_receipt',expected_receipt);
end;
$function$
;

CREATE OR REPLACE FUNCTION app.operator_erasure_row_allowed(relation_name text, previous_row jsonb)
 RETURNS boolean
 LANGUAGE plpgsql
 STABLE
 SET search_path TO ''
AS $function$
declare allowed_rows jsonb;
begin
 if current_user<>'postgres' or session_user<>'postgres'
  or nullif(current_setting('app.operator_purge_workspace_id',true),'') is distinct from previous_row->>'workspace_id'
  or nullif(current_setting('app.operator_purge_transaction_id',true),'') is distinct from pg_catalog.txid_current()::text then return false; end if;
 if nullif(current_setting('app.operator_purge_project_id',true),'') is null then return true; end if;
 allowed_rows=nullif(current_setting('app.operator_purge_project_rows',true),'')::jsonb;
 return coalesce(allowed_rows->relation_name @> jsonb_build_array((previous_row->>'id')::bigint),false);
end;
$function$
;

CREATE OR REPLACE FUNCTION app.operator_project_expected_schema()
 RETURNS text
 LANGUAGE sql
 IMMUTABLE
 SET search_path TO ''
AS $function$
 select 'f17b7f9d7be7197bb88bb69e2fc9d942'::text;
$function$
;

CREATE OR REPLACE FUNCTION app.operator_project_manifest(requested_workspace uuid, requested_project uuid)
 RETURNS jsonb
 LANGUAGE plpgsql
 SET search_path TO ''
AS $function$
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
$function$
;

CREATE OR REPLACE FUNCTION app.operator_project_preserved_fingerprint(deletion_rows jsonb)
 RETURNS text
 LANGUAGE plpgsql
 SET search_path TO ''
AS $function$
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
$function$
;

CREATE OR REPLACE FUNCTION app.operator_project_schema_fingerprint()
 RETURNS text
 LANGUAGE sql
 STABLE
 SET search_path TO ''
AS $function$
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
$function$
;

CREATE OR REPLACE FUNCTION app.operator_purge_project(requested_workspace uuid, requested_project uuid, confirmation uuid, expected_receipt text)
 RETURNS jsonb
 LANGUAGE plpgsql
 SET search_path TO ''
 SET lock_timeout TO '2s'
 SET statement_timeout TO '30s'
AS $function$
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
$function$
;

CREATE OR REPLACE FUNCTION app.prevent_append_only_mutation()
 RETURNS trigger
 LANGUAGE plpgsql
 SET search_path TO ''
AS $function$
begin
 if tg_op='DELETE' and current_user='postgres' and session_user='postgres'
  and app.operator_erasure_row_allowed(tg_table_name,to_jsonb(old)) then return old; end if;
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
  and app.operator_erasure_row_allowed(tg_table_name,to_jsonb(old)) then return old; end if;
 raise exception 'audit_events are immutable';
end;
$function$
;



-- Explicit private ACLs omitted by the schema diff.
revoke all on function app.operator_project_expected_schema() from public,anon,authenticated,service_role,ai_center_runtime;
revoke all on function app.operator_project_schema_fingerprint() from public,anon,authenticated,service_role,ai_center_runtime;
revoke all on function app.operator_project_predicate(text) from public,anon,authenticated,service_role,ai_center_runtime;
revoke all on function app.operator_project_manifest(uuid,uuid) from public,anon,authenticated,service_role,ai_center_runtime;
revoke all on function app.operator_project_preserved_fingerprint(jsonb) from public,anon,authenticated,service_role,ai_center_runtime;
revoke all on function app.operator_erasure_row_allowed(text,jsonb) from public,anon,authenticated,service_role,ai_center_runtime;
revoke all on function app.prevent_append_only_mutation() from public,anon,authenticated,service_role,ai_center_runtime;
revoke all on function app.prevent_audit_mutation() from public,anon,authenticated,service_role,ai_center_runtime;
revoke all on function app.operator_purge_project(uuid,uuid,uuid,text) from public,anon,authenticated,service_role,ai_center_runtime;

-- Explicit private ACLs omitted by the schema diff.
revoke all on function app.operator_abandoned_rows(bigint) from public,anon,authenticated,service_role,ai_center_runtime;
revoke all on function app.operator_abandoned_manifest(uuid) from public,anon,authenticated,service_role,ai_center_runtime;
revoke all on function app.operator_close_abandoned(uuid,uuid,text,boolean,uuid) from public,anon,authenticated,service_role,ai_center_runtime;
