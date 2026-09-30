-- Bearer invitation tokens never enter durable storage; only their SHA-256
-- digest does. Auth identity and explicit acceptance still gate membership.
alter table app.workspace_members add column display_name text not null default ''
  check (length(display_name)<=80);

create table app.workspace_invitations (
  id bigint generated always as identity primary key,
  public_id uuid not null unique,
  workspace_id bigint not null references app.workspaces(id),
  created_by_actor_id uuid not null,
  role text not null check(role in ('editor','viewer')),
  label text not null default '' check(length(label)<=120),
  token_hash text not null check(token_hash ~ '^[0-9a-f]{64}$'),
  request_hash text not null check(request_hash ~ '^[0-9a-f]{64}$'),
  status text not null default 'pending' check(status in ('pending','accepted','revoked')),
  accepted_by_actor_id uuid,
  accepted_at timestamptz,
  expires_at timestamptz not null,
  created_at timestamptz not null default now(),
  revoked_at timestamptz,
  check(expires_at>created_at and expires_at<=created_at+interval '7 days'),
  check((status='accepted')=(accepted_by_actor_id is not null and accepted_at is not null)),
  check((status='revoked')=(revoked_at is not null))
);
create index workspace_invitations_scope_idx on app.workspace_invitations(workspace_id,status,expires_at);
create index workspace_invitations_creator_idx on app.workspace_invitations(created_by_actor_id);
alter table app.workspace_invitations enable row level security;
alter table app.workspace_invitations force row level security;
create policy invitation_owner_read on app.workspace_invitations for select using
  (workspace_id=(select app.current_workspace_id()) and app.has_workspace_role(workspace_id,array['owner']));
create policy invitation_owner_insert on app.workspace_invitations for insert with check
  (workspace_id=(select app.current_workspace_id()) and created_by_actor_id=(select app.current_actor_id())
   and app.has_workspace_role(workspace_id,array['owner']));
create policy invitation_owner_update on app.workspace_invitations for update using
  (workspace_id=(select app.current_workspace_id()) and app.has_workspace_role(workspace_id,array['owner']))
  with check(workspace_id=(select app.current_workspace_id()) and app.has_workspace_role(workspace_id,array['owner']));
revoke all on app.workspace_invitations from public,anon,authenticated,service_role;
grant select,insert on app.workspace_invitations to ai_center_runtime;
grant update(status,revoked_at) on app.workspace_invitations to ai_center_runtime;
grant usage on sequence app.workspace_invitations_id_seq to ai_center_runtime;

-- A narrow preview takes the digest computed by the server from the presented
-- token. The HTTP API never accepts this digest as an acceptance credential.
create function app.preview_workspace_invitation(requested_id uuid,presented_hash text)
returns table(public_id uuid,workspace_public_id uuid,company_name text,role text,expires_at timestamptz)
language sql stable security definer set search_path='' as $$
  select i.public_id,w.public_id,w.name,i.role,i.expires_at
  from app.workspace_invitations i join app.workspaces w on w.id=i.workspace_id
  where i.public_id=requested_id and i.token_hash=presented_hash and i.status='pending' and i.expires_at>now()
    and exists(select 1 from app.workspace_members m where m.workspace_id=i.workspace_id
      and m.actor_id=i.created_by_actor_id and m.role='owner' and m.invitation_status='accepted');
$$;
revoke all on function app.preview_workspace_invitation(uuid,text) from public,anon,authenticated,service_role;
grant execute on function app.preview_workspace_invitation(uuid,text) to ai_center_runtime;

create function app.accept_workspace_invitation(requested_id uuid,presented_hash text,requested_display_name text)
returns table(public_id uuid,name text,role text)
language plpgsql security definer set search_path='' as $$
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
$$;
revoke all on function app.accept_workspace_invitation(uuid,text,text) from public,anon,authenticated,service_role;
grant execute on function app.accept_workspace_invitation(uuid,text,text) to ai_center_runtime;

create function app.set_member_display_name(requested_name text)
returns uuid language plpgsql security definer set search_path='' as $$
declare member_public_id uuid;
begin
  if requested_name is null or length(btrim(requested_name)) not between 1 and 80 then
    raise exception 'display name required' using errcode='22023';
  end if;
  update app.workspace_members set display_name=btrim(requested_name)
    where workspace_id=app.current_workspace_id() and actor_id=app.current_actor_id() and invitation_status='accepted'
    returning public_id into member_public_id;
  if member_public_id is null then raise exception 'accepted membership required' using errcode='42501'; end if;
  return member_public_id;
end;
$$;
revoke all on function app.set_member_display_name(text) from public,anon,authenticated,service_role;
grant execute on function app.set_member_display_name(text) to ai_center_runtime;
