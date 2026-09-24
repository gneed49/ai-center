-- Explicit publications of immutable artifacts; external tools stay canonical.
create table app.work_tool_connections (
  id bigint generated always as identity primary key,
  public_id uuid not null unique,
  workspace_id bigint not null references app.workspaces(id),
  provider text not null check (provider in ('notion','linear','github')),
  name text not null check (length(btrim(name)) between 1 and 120),
  encrypted_credential bytea not null check (octet_length(encrypted_credential) between 30 and 8192),
  credential_actor_id uuid not null,
  enabled boolean not null default true,
  revision integer not null default 1 check (revision>0),
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  unique(id,workspace_id)
);
create index work_tool_connections_workspace_idx on app.work_tool_connections(workspace_id);

create table app.publication_jobs (
  id bigint generated always as identity primary key,
  public_id uuid not null default gen_random_uuid() unique,
  workspace_id bigint not null,
  project_id bigint not null,
  artifact_version_id bigint not null,
  connection_id bigint not null,
  connection_revision integer not null,
  requested_by_actor_id uuid not null,
  provider text not null check (provider in ('notion','linear','github')),
  target_id text not null check (length(target_id) between 1 and 256),
  title text not null check (length(title) between 1 and 200),
  body_markdown text not null check (octet_length(body_markdown)<=61440),
  content_hash text not null check (content_hash ~ '^[0-9a-f]{64}$'),
  status text not null default 'queued' check (status in ('queued','processing','succeeded','failed','needs_review','conflict','unavailable','cancelled')),
  attempt_count integer not null default 0 check (attempt_count between 0 and 1),
  lease_token uuid,
  lease_until timestamptz,
  external_id text,
  external_url text,
  error_code text check (length(error_code)<=80),
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  source_ticket_index smallint not null default -1 check (source_ticket_index between -1 and 29),
  unique (id,workspace_id),
  constraint publication_jobs_source_destination_unique unique (workspace_id,artifact_version_id,provider,target_id,source_ticket_index),
  foreign key (artifact_version_id,workspace_id) references app.artifact_document_versions(id,workspace_id),
  foreign key (artifact_version_id,project_id) references app.artifact_document_versions(id,project_id),
  foreign key (project_id,workspace_id) references app.projects(id,workspace_id),
  foreign key (connection_id,workspace_id) references app.work_tool_connections(id,workspace_id),
  check ((external_id is null)=(external_url is null)),
  check (status not in ('succeeded','conflict','unavailable') or external_id is not null)
);
create index publication_jobs_scope_idx on app.publication_jobs(workspace_id,artifact_version_id,created_at desc);
create index publication_jobs_project_idx on app.publication_jobs(project_id);
create index publication_jobs_version_idx on app.publication_jobs(artifact_version_id);
create index publication_jobs_connection_idx on app.publication_jobs(connection_id);
create index publication_jobs_queue_idx on app.publication_jobs(created_at,id) where status='queued';
create index publication_jobs_lease_idx on app.publication_jobs(lease_until) where status='processing';

-- The index is scoped to an immutable version, never matched across versions.
-- Invoker rights preserve the same source RLS as the request that queues a job.
create or replace function app.publication_validate_source_ticket()
returns trigger language plpgsql set search_path='' as $$
declare source_kind text; source_content jsonb; tickets jsonb;
begin
  if tg_op='UPDATE' then
    if (new.public_id,new.workspace_id,new.project_id,new.artifact_version_id,
        new.source_ticket_index,new.connection_id,new.connection_revision,
        new.requested_by_actor_id,new.provider,new.target_id,new.title,new.body_markdown,new.content_hash)
      is distinct from
       (old.public_id,old.workspace_id,old.project_id,old.artifact_version_id,
        old.source_ticket_index,old.connection_id,old.connection_revision,
        old.requested_by_actor_id,old.provider,old.target_id,old.title,old.body_markdown,old.content_hash) then
      raise exception 'A publication source and prepared content are immutable' using errcode='23514';
    end if;
    return new;
  end if;
  if new.source_ticket_index>=0 then
    select d.artifact_type,v.structured_content into source_kind,source_content
      from app.artifact_document_versions v join app.artifact_documents d on d.id=v.document_id
      where v.id=new.artifact_version_id and v.workspace_id=new.workspace_id and v.project_id=new.project_id;
    if not found or new.provider not in ('linear','github')
      or source_kind not in ('product_tickets','technical_tickets')
      or source_content->>'format' is distinct from 'agent-artifact-v1'
      or source_content->>'artifact_type' is distinct from source_kind then
      raise exception 'An indexed publication requires an exact structured ticket source' using errcode='23514';
    end if;
    tickets=source_content->'draft'->'tickets';
    if jsonb_typeof(tickets) is distinct from 'array' then
      raise exception 'The ticket source has no structured entries' using errcode='23514';
    end if;
    if jsonb_array_length(tickets) not between 1 and 30
      or jsonb_typeof(tickets->new.source_ticket_index::integer) is distinct from 'object' then
      raise exception 'The selected ticket does not exist in this version' using errcode='23514';
    end if;
  end if;
  return new;
end;
$$;
revoke all on function app.publication_validate_source_ticket() from public,anon,authenticated,service_role;
create trigger publication_jobs_validate_source_ticket before insert or update on app.publication_jobs
for each row execute function app.publication_validate_source_ticket();

create or replace function app.publication_preserve_receipt()
returns trigger language plpgsql set search_path='' as $$
begin
  if old.external_id is not null and (new.external_id is distinct from old.external_id
    or new.external_url is distinct from old.external_url) then
    raise exception 'A canonical publication receipt cannot be replaced' using errcode='23514';
  end if;
  return new;
end;
$$;
revoke all on function app.publication_preserve_receipt() from public,anon,authenticated,service_role;
create trigger publication_jobs_preserve_receipt before update on app.publication_jobs
for each row execute function app.publication_preserve_receipt();

create table app.publication_observations (
  id bigint generated always as identity primary key,
  public_id uuid not null default gen_random_uuid() unique,
  workspace_id bigint not null,
  publication_job_id bigint not null,
  observed_at timestamptz not null default now(),
  observation_kind text not null check (observation_kind in ('created','reconciled','unchanged','changed','unavailable')),
  external_id text not null check (length(external_id) between 1 and 256),
  external_url text not null check (length(external_url)<=2048),
  remote_updated_at text,
  snapshot jsonb not null check (jsonb_typeof(snapshot)='object' and octet_length(snapshot::text)<=262144),
  unique(id,workspace_id),
  foreign key (publication_job_id,workspace_id) references app.publication_jobs(id,workspace_id)
);
create index publication_observations_job_idx on app.publication_observations(publication_job_id,id desc);
create index publication_observations_workspace_idx on app.publication_observations(workspace_id);
create trigger publication_observations_immutable before update or delete on app.publication_observations
for each row execute function app.prevent_append_only_mutation();

do $$
declare relation_name text;
begin
  foreach relation_name in array array['work_tool_connections','publication_jobs','publication_observations'] loop
    execute format('alter table app.%I enable row level security',relation_name);
    execute format('alter table app.%I force row level security',relation_name);
    execute format('create policy work_tools_member_select on app.%I for select using (workspace_id=(select app.current_workspace_id()) and app.has_workspace_role(workspace_id,array[''owner'',''editor'',''viewer'']::text[]))',relation_name);
    execute format('revoke all on app.%I from public,anon,authenticated,service_role',relation_name);
  end loop;
end;
$$;
create policy work_tools_owner_write on app.work_tool_connections for all
using (workspace_id=(select app.current_workspace_id()) and app.has_workspace_role(workspace_id,array['owner']::text[]))
with check (workspace_id=(select app.current_workspace_id()) and app.has_workspace_role(workspace_id,array['owner']::text[]));
create policy publications_editor_insert on app.publication_jobs for insert
with check (workspace_id=(select app.current_workspace_id()) and app.has_workspace_role(workspace_id,array['owner','editor']::text[])
  and requested_by_actor_id=app.current_actor_id() and status='queued' and attempt_count=0 and lease_token is null and external_id is null);
create policy observations_editor_insert on app.publication_observations for insert
with check (workspace_id=(select app.current_workspace_id()) and app.has_workspace_role(workspace_id,array['owner','editor']::text[]));
create policy publications_editor_update on app.publication_jobs for update
using (workspace_id=(select app.current_workspace_id()) and app.has_workspace_role(workspace_id,array['owner','editor']::text[]))
with check (workspace_id=(select app.current_workspace_id()) and app.has_workspace_role(workspace_id,array['owner','editor']::text[]));

-- The runtime alone may take a single queue lease across tenants. No content or
-- credential is returned. A lost create outcome is NEVER automatically retried.
create or replace function app.claim_publication_job()
returns table(job_id uuid,workspace_id bigint,workspace_public_id uuid,actor_id uuid,actor_role text,lease uuid)
language plpgsql security definer set search_path='' set row_security=off as $$
declare claimed app.publication_jobs; token uuid=gen_random_uuid();
begin
  update app.publication_jobs set status='needs_review',error_code='worker_lease_expired',updated_at=now()
    where id in (select j.id from app.publication_jobs j where j.status='processing' and j.lease_until<now()
      order by j.lease_until limit 100 for update skip locked);
  select * into claimed from app.publication_jobs j where j.status='queued' and j.attempt_count=0
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

-- A random lease is a capability to settle exactly this attempt, even if its
-- initiating member was revoked while the external request was in flight.
-- This avoids discarding a successful receipt and accidentally creating twice.
create or replace function app.finish_publication_job(requested_job uuid,requested_lease uuid,new_status text,
  safe_error text,receipt jsonb)
returns boolean language plpgsql security definer set search_path='' set row_security=off as $$
declare job app.publication_jobs;
begin
  if new_status not in ('succeeded','failed','needs_review','cancelled') or length(safe_error)>80 then
    raise exception 'Invalid publication settlement' using errcode='23514';
  end if;
  select * into job from app.publication_jobs where public_id=requested_job and lease_token=requested_lease
    and status in ('processing','needs_review') for update;
  if not found then return false; end if;
  if new_status='succeeded' then
    if receipt is null or jsonb_typeof(receipt)<>'object' or coalesce(length(receipt->>'external_id'),0) not between 1 and 256
      or coalesce(length(receipt->>'external_url'),0) not between 1 and 2048 or octet_length(receipt::text)>262144 then
      raise exception 'Invalid publication receipt' using errcode='23514';
    end if;
    insert into app.publication_observations(workspace_id,publication_job_id,observation_kind,external_id,external_url,remote_updated_at,snapshot)
      values(job.workspace_id,job.id,'created',receipt->>'external_id',receipt->>'external_url',receipt->>'remote_updated_at',receipt);
  end if;
  update app.publication_jobs set status=new_status,error_code=safe_error,
    external_id=case when new_status='succeeded' then receipt->>'external_id' else external_id end,
    external_url=case when new_status='succeeded' then receipt->>'external_url' else external_url end,
    updated_at=now() where id=job.id;
  return true;
end;
$$;
revoke all on function app.finish_publication_job(uuid,uuid,text,text,jsonb) from public,anon,authenticated,service_role;

grant select,insert on app.work_tool_connections,app.publication_jobs,app.publication_observations to ai_center_runtime;
grant update(name,encrypted_credential,credential_actor_id,enabled,revision,updated_at) on app.work_tool_connections to ai_center_runtime;
grant update(status,error_code,external_id,external_url,updated_at) on app.publication_jobs to ai_center_runtime;
grant usage on sequence app.work_tool_connections_id_seq,app.publication_jobs_id_seq,app.publication_observations_id_seq to ai_center_runtime;
grant execute on function app.claim_publication_job(),app.finish_publication_job(uuid,uuid,text,text,jsonb) to ai_center_runtime;
