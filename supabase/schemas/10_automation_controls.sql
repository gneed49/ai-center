create table app.workspace_automation_controls (
  id bigint generated always as identity primary key,
  workspace_id bigint not null unique references app.workspaces(id) on delete cascade,
  enabled boolean not null default true,
  generation bigint not null default 1 check(generation>0),
  updated_by_actor_id uuid not null,
  updated_at timestamptz not null default now()
);
create table app.ai_call_reservations (
  id bigint generated always as identity primary key,
  public_id uuid not null unique,
  workspace_id bigint not null references app.workspaces(id) on delete cascade,
  actor_id uuid not null,
  operation text not null check(operation in ('respond','select_context','technical_plan','coverage','steward')),
  control_generation bigint not null check(control_generation>=0),
  status text not null default 'running' check(status in ('running','completed','failed','cancelled')),
  created_at timestamptz not null default now(),
  lease_until timestamptz not null,
  finished_at timestamptz,
  check(lease_until>created_at and lease_until<=created_at+interval '11 minutes'),
  check((status='running')=(finished_at is null))
);
create index ai_call_reservations_scope_time_idx on app.ai_call_reservations(workspace_id,created_at desc);
create index ai_call_reservations_active_idx on app.ai_call_reservations(workspace_id,lease_until) where status='running';
alter table app.workspace_automation_controls enable row level security;
alter table app.workspace_automation_controls force row level security;
alter table app.ai_call_reservations enable row level security;
alter table app.ai_call_reservations force row level security;
create policy automation_controls_read on app.workspace_automation_controls for select using
 (workspace_id=(select app.current_workspace_id()) and app.has_workspace_role(workspace_id,array['owner','editor','viewer']));
create policy automation_controls_owner on app.workspace_automation_controls for all using
 (workspace_id=(select app.current_workspace_id()) and app.has_workspace_role(workspace_id,array['owner'])) with check
 (workspace_id=(select app.current_workspace_id()) and updated_by_actor_id=(select app.current_actor_id()) and app.has_workspace_role(workspace_id,array['owner']));
create policy ai_reservations_read on app.ai_call_reservations for select using
 (workspace_id=(select app.current_workspace_id()) and app.has_workspace_role(workspace_id,array['owner','editor','viewer']));
create policy ai_reservations_insert on app.ai_call_reservations for insert with check
 (workspace_id=(select app.current_workspace_id()) and actor_id=(select app.current_actor_id()) and app.has_workspace_role(workspace_id,array['owner','editor']));
create policy ai_reservations_update on app.ai_call_reservations for update using
 (workspace_id=(select app.current_workspace_id()) and actor_id=(select app.current_actor_id()) and app.has_workspace_role(workspace_id,array['owner','editor'])) with check
 (workspace_id=(select app.current_workspace_id()) and actor_id=(select app.current_actor_id()) and app.has_workspace_role(workspace_id,array['owner','editor']));
revoke all on app.workspace_automation_controls,app.ai_call_reservations from public,anon,authenticated,service_role;
grant select,insert on app.workspace_automation_controls,app.ai_call_reservations to ai_center_runtime;
grant update(enabled,generation,updated_by_actor_id,updated_at) on app.workspace_automation_controls to ai_center_runtime;
grant update(status,finished_at) on app.ai_call_reservations to ai_center_runtime;
grant usage on sequence app.workspace_automation_controls_id_seq,app.ai_call_reservations_id_seq to ai_center_runtime;

-- Revoking membership stops the provider future but must not strand its lease.
-- This function only settles an existing reservation owned by the same actor;
-- it can neither create work nor expose data or change company controls.
create function app.finish_ai_call_reservation(requested_id uuid,requested_status text)
returns boolean language plpgsql security definer set search_path='' as $$
begin
  if requested_status not in ('completed','failed','cancelled') then
    raise exception 'invalid reservation settlement' using errcode='22023';
  end if;
  update app.ai_call_reservations set status=requested_status,finished_at=now()
    where public_id=requested_id and workspace_id=app.current_workspace_id()
      and actor_id=app.current_actor_id() and status='running';
  return found;
end;
$$;
revoke all on function app.finish_ai_call_reservation(uuid,text) from public,anon,authenticated,service_role;
grant execute on function app.finish_ai_call_reservation(uuid,text) to ai_center_runtime;

-- A paused company keeps queued publications untouched until explicit resume.
create or replace function app.claim_publication_job()
returns table(job_id uuid,workspace_id bigint,workspace_public_id uuid,actor_id uuid,actor_role text,lease uuid)
language plpgsql security definer set search_path='' set row_security=off as $$
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
$$;
revoke all on function app.claim_publication_job() from public,anon,authenticated,service_role;


-- Skip paused companies before the bounded recovery scan chooses its scopes.
create or replace function app.list_due_steward_workspaces(
  requested_limit integer default 16
)
returns table(
  workspace_id bigint,
  workspace_public_id uuid,
  actor_id uuid,
  workspace_role text
)
language sql
-- Intentional narrow SECURITY DEFINER recovery bootstrap. The function is in
-- the private app schema, hardcodes the Steward event allowlist, returns at
-- most 128 scopes, and selects only an accepted owner/editor. It never claims
-- or reads event payloads; all processing still runs as that member under the
-- runtime role, request GUCs and forced RLS.
security definer
set search_path = ''
set row_security = off
rows 128
as $$
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
$$;

