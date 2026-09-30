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
  -- Withdrawal supersedes every link issued before that membership change.
  -- A new owner-issued invitation may deliberately readmit the former member.
  -- The workspace lock above serializes this check with removal and creation.
  if exists(select 1 from app.workspace_members m
    where m.workspace_id=invited.workspace_id and m.actor_id=actor
      and m.invitation_status='revoked' and invited.created_at<=m.updated_at
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



revoke all on function app.accept_workspace_invitation(uuid,text,text), app.list_due_steward_workspaces(integer) from public,anon,authenticated,service_role;
grant execute on function app.accept_workspace_invitation(uuid,text,text), app.list_due_steward_workspaces(integer) to ai_center_runtime;
