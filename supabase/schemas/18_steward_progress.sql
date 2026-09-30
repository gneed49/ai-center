-- One fenced company worker; durable per-version receipts avoid frontier resets.
create table app.steward_scan_progress (
 id bigint generated always as identity primary key,
 workspace_id bigint not null unique references app.workspaces(id),
 lease_token uuid,
 lease_until timestamptz,
 status text not null default 'pending' check(status in ('pending','running','idle','blocked')),
 last_error_code text,
 updated_at timestamptz not null default now(),
 check ((lease_token is null)=(lease_until is null)),
 check ((status='running')=(lease_token is not null))
);
create table app.steward_scan_sources (
 id bigint generated always as identity primary key,
 workspace_id bigint not null references app.workspaces(id),
 project_id bigint not null,
 source_public_id uuid not null,
 source_kind text not null check(source_kind in ('knowledge_entry_version','artifact_document_version','external_reference_observation','publication_observation','github_code_file_observation')),
 examined_pairs integer not null check(examined_pairs>=0),
 omitted_neighbors bigint not null check(omitted_neighbors>=0),
 completed_at timestamptz not null default now(),
 unique(workspace_id,source_public_id),
 foreign key(project_id,workspace_id) references app.projects(id,workspace_id)
);
create index steward_scan_sources_project_idx on app.steward_scan_sources(project_id);
create index steward_scope_sources_workspace_source_idx on app.steward_scope_sources(workspace_id,source_public_id,assessment_id);
create index steward_assessments_workspace_fingerprint_idx on app.steward_assessments(workspace_id,fingerprint);
alter table app.steward_scan_progress enable row level security;
alter table app.steward_scan_progress force row level security;
alter table app.steward_scan_sources enable row level security;
alter table app.steward_scan_sources force row level security;
create policy steward_scan_progress_read on app.steward_scan_progress for select using
 (workspace_id=(select app.current_workspace_id()) and app.has_workspace_role(workspace_id,array['owner','editor','viewer']));
create policy steward_scan_progress_write on app.steward_scan_progress for all using
 (workspace_id=(select app.current_workspace_id()) and app.has_workspace_role(workspace_id,array['owner','editor'])) with check
 (workspace_id=(select app.current_workspace_id()) and app.has_workspace_role(workspace_id,array['owner','editor']));
create policy steward_scan_sources_read on app.steward_scan_sources for select using
 (workspace_id=(select app.current_workspace_id()) and app.has_workspace_role(workspace_id,array['owner','editor','viewer']));
create policy steward_scan_sources_write on app.steward_scan_sources for insert with check
 (workspace_id=(select app.current_workspace_id()) and app.has_workspace_role(workspace_id,array['owner','editor']));
create trigger steward_scan_sources_immutable before update or delete on app.steward_scan_sources
 for each row execute function app.prevent_append_only_mutation();
revoke all on app.steward_scan_progress,app.steward_scan_sources from public,anon,authenticated,service_role;
grant select,insert,update on app.steward_scan_progress to ai_center_runtime;
grant select,insert on app.steward_scan_sources to ai_center_runtime;
grant usage on sequence app.steward_scan_progress_id_seq,app.steward_scan_sources_id_seq to ai_center_runtime;

-- Explicit offline maintenance only. This never retries or certifies remote work.
create or replace function app.operator_abandoned_rows(requested_workspace_id bigint)
returns table(table_name text,row_id bigint,public_id uuid,status text,created_at timestamptz,last_activity_at timestamptz,lease_until timestamptz,lease_marker text)
language sql stable security invoker set search_path='' as $$
 select 'ai_call_reservations',r.id,r.public_id,r.status,r.created_at,r.created_at,r.lease_until,r.control_generation::text
 from app.ai_call_reservations r where r.workspace_id=requested_workspace_id and r.status='running'
 union all
 select 'steward_scan_progress',r.id,null::uuid,r.status,r.updated_at,r.updated_at,r.lease_until,r.lease_token::text
 from app.steward_scan_progress r where r.workspace_id=requested_workspace_id and r.status='running'
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
$$;
revoke all on function app.operator_abandoned_rows(bigint) from public,anon,authenticated,service_role,ai_center_runtime;

create or replace function app.operator_abandoned_manifest(requested_workspace uuid)
returns jsonb language plpgsql security invoker set search_path='' as $$
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
  union all select r.updated_at from app.steward_scan_progress r where r.workspace_id=target_workspace_id
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
$$;
revoke all on function app.operator_abandoned_manifest(uuid) from public,anon,authenticated,service_role,ai_center_runtime;

create or replace function app.operator_close_abandoned(requested_workspace uuid,confirmation uuid,expected_receipt text,workers_stopped boolean,operator_actor uuid)
returns jsonb language plpgsql security invoker set search_path='' set lock_timeout='2s' set statement_timeout='30s' as $$
declare target_workspace_id bigint; relation_name text; manifest jsonb; closed_counts jsonb='{}'; affected_count bigint; audit_id uuid=gen_random_uuid();
begin
 if current_user<>'postgres' or session_user<>'postgres' then raise exception 'Direct operator maintenance connection required' using errcode='42501'; end if;
 if requested_workspace is null or confirmation is distinct from requested_workspace or expected_receipt is null
  or workers_stopped is distinct from true or operator_actor is null or operator_actor='00000000-0000-0000-0000-000000000000'::uuid then
  raise exception 'Exact target, receipt, operator identity and explicit stopped-process confirmation required' using errcode='22023';
 end if;
 for relation_name in select unnest(array['ai_call_reservations','audit_events','domain_events','idempotency_records','model_runs','publication_jobs','steward_scan_progress','workspace_automation_controls','workspaces']) order by 1 loop
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
 update app.steward_scan_progress set status='pending',lease_token=null,lease_until=null,last_error_code='operator_abandoned_closed',updated_at=clock_timestamp()
  where workspace_id=target_workspace_id and status='running';
 get diagnostics affected_count=row_count;
 closed_counts=closed_counts||jsonb_build_object('steward_scan_progress',affected_count);
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
$$;
revoke all on function app.operator_close_abandoned(uuid,uuid,text,boolean,uuid) from public,anon,authenticated,service_role,ai_center_runtime;
