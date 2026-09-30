-- Exact, selected GitHub files at an immutable commit; never repository certification.
create table app.github_code_corpora (
  id bigint generated always as identity primary key,
  public_id uuid not null default gen_random_uuid() unique,
  workspace_id bigint not null,
  project_id bigint not null,
  connection_id bigint not null,
  repository text not null check(length(repository) between 3 and 201),
  commit_sha text not null check(commit_sha ~ '^[0-9a-f]{40}$'),
  commit_verified boolean not null,
  requested_paths jsonb not null check(jsonb_typeof(requested_paths)='array' and jsonb_array_length(requested_paths) between 1 and 10),
  requested_by_actor_id uuid not null,
  observed_at timestamptz not null default now(),
  unique(id,workspace_id,project_id),
  foreign key(project_id,workspace_id) references app.projects(id,workspace_id),
  foreign key(connection_id,workspace_id) references app.work_tool_connections(id,workspace_id)
);
create index github_code_corpora_scope_idx on app.github_code_corpora(workspace_id,project_id,id desc);
create index github_code_corpora_project_idx on app.github_code_corpora(project_id);
create index github_code_corpora_connection_idx on app.github_code_corpora(connection_id);
create table app.github_code_file_observations (
  id bigint generated always as identity primary key,
  public_id uuid not null default gen_random_uuid() unique,
  workspace_id bigint not null,
  project_id bigint not null,
  corpus_id bigint not null,
  path text not null check(length(path) between 1 and 512),
  status text not null check(status in ('code_read','missing','inaccessible','too_large','binary','unsupported','unavailable')),
  reason_code text,
  blob_sha text check(blob_sha ~ '^[0-9a-f]{40}$'),
  content_hash text check(content_hash ~ '^[0-9a-f]{64}$'),
  content_text text check(octet_length(content_text)<=65536),
  line_count integer not null default 0 check(line_count>=0),
  observed_at timestamptz not null default now(),
  unique(corpus_id,path),
  unique(id,workspace_id),
  unique(id,project_id),
  foreign key(corpus_id,workspace_id,project_id) references app.github_code_corpora(id,workspace_id,project_id),
  check((status='code_read')=(content_text is not null and content_hash is not null and blob_sha is not null)),
  check(status='code_read' or (content_text is null and content_hash is null and line_count=0))
);
create index github_code_files_workspace_idx on app.github_code_file_observations(workspace_id);
create index github_code_files_project_idx on app.github_code_file_observations(project_id);
create or replace function app.github_code_file_scope()
returns trigger language plpgsql security invoker set search_path='' as $$
declare corpus app.github_code_corpora;
begin
 select * into corpus from app.github_code_corpora where id=new.corpus_id and workspace_id=new.workspace_id and project_id=new.project_id;
 if corpus.id is null or not (corpus.requested_paths ? new.path) or (new.status='code_read' and not corpus.commit_verified) then
  raise exception 'Code evidence requires the requested path and a verified commit' using errcode='23514';
 end if;
 return new;
end;
$$;
revoke all on function app.github_code_file_scope() from public,anon,authenticated,service_role;
create trigger github_code_file_scope before insert on app.github_code_file_observations
 for each row execute function app.github_code_file_scope();
do $$
declare relation_name text;
begin
 foreach relation_name in array array['github_code_corpora','github_code_file_observations'] loop
  execute format('alter table app.%I enable row level security',relation_name);
  execute format('alter table app.%I force row level security',relation_name);
  execute format('create policy github_code_member_select on app.%I for select using(workspace_id=app.current_workspace_id() and app.has_workspace_role(workspace_id,array[''owner'',''editor'',''viewer'']))',relation_name);
  execute format('create policy github_code_editor_insert on app.%I for insert with check(workspace_id=app.current_workspace_id() and app.has_workspace_role(workspace_id,array[''owner'',''editor'']))',relation_name);
  execute format('create trigger github_code_immutable before update or delete on app.%I for each row execute function app.prevent_append_only_mutation()',relation_name);
  execute format('revoke all on app.%I from public,anon,authenticated,service_role',relation_name);
 end loop;
end;
$$;
grant select,insert on app.github_code_corpora,app.github_code_file_observations to ai_center_runtime;
grant usage on sequence app.github_code_corpora_id_seq,app.github_code_file_observations_id_seq to ai_center_runtime;

alter table app.steward_scope_sources add column github_code_file_observation_id bigint;
alter table app.steward_scope_sources add foreign key(github_code_file_observation_id,source_project_id)
 references app.github_code_file_observations(id,project_id);
create index steward_scope_sources_code_idx on app.steward_scope_sources(github_code_file_observation_id) where github_code_file_observation_id is not null;
alter table app.steward_scope_sources drop constraint steward_scope_sources_source_kind_check;
alter table app.steward_scope_sources add constraint steward_scope_sources_source_kind_check
 check(source_kind in ('knowledge_entry_version','artifact_document_version','external_reference_observation','publication_observation','github_code_file_observation'));
alter table app.steward_scope_sources drop constraint steward_scope_sources_check;
alter table app.steward_scope_sources add constraint steward_scope_sources_check
 check(num_nonnulls(knowledge_version_id,artifact_version_id,external_observation_id,publication_observation_id,github_code_file_observation_id)=1);
alter table app.steward_scope_sources add constraint steward_scope_sources_code_kind_check
 check((source_kind='github_code_file_observation')=(github_code_file_observation_id is not null));

create or replace function app.github_code_observed_context_event()
returns trigger language plpgsql security invoker set search_path='' as $$
declare corpus app.github_code_corpora;
begin
 select * into corpus from app.github_code_corpora where id=new.corpus_id;
 update app.projects set graph_version=graph_version+1 where id=new.project_id;
 insert into app.domain_events(workspace_id,project_id,event_type,aggregate_kind,aggregate_public_id,payload,requested_by_actor_id)
 values(new.workspace_id,new.project_id,'github_code.observed','github_code_file_observation',new.public_id,
   jsonb_build_object('observation_public_id',new.public_id,'corpus_public_id',corpus.public_id,'status',new.status),corpus.requested_by_actor_id);
 return new;
end;
$$;
revoke all on function app.github_code_observed_context_event() from public,anon,authenticated,service_role;
create trigger github_code_observed_context after insert on app.github_code_file_observations
 for each row execute function app.github_code_observed_context_event();


create or replace function app.validate_steward_scope_source()
returns trigger language plpgsql security invoker set search_path='' as $$
declare actual_id uuid; actual_project bigint;
begin
 case new.source_kind
 when 'knowledge_entry_version' then select public_id,project_id into actual_id,actual_project from app.knowledge_entry_versions where id=new.knowledge_version_id;
 when 'artifact_document_version' then select public_id,project_id into actual_id,actual_project from app.artifact_document_versions where id=new.artifact_version_id and status='validated';
 when 'external_reference_observation' then select public_id,project_id into actual_id,actual_project from app.external_reference_observations where id=new.external_observation_id;
 when 'publication_observation' then select o.public_id,j.project_id into actual_id,actual_project from app.publication_observations o join app.publication_jobs j on j.id=o.publication_job_id where o.id=new.publication_observation_id;
 when 'github_code_file_observation' then select public_id,project_id into actual_id,actual_project from app.github_code_file_observations where id=new.github_code_file_observation_id;
 end case;
 if actual_id is null or actual_id<>new.source_public_id or actual_project<>new.source_project_id then
   raise exception 'Steward source does not match its immutable scope and version' using errcode='23514';
 end if;
 return new;
end;
$$;

create or replace function app.steward_scope_sources_current(requested_assessment_id bigint)
returns boolean language sql stable security invoker set search_path='' as $$
 select not exists(select 1 from app.steward_scope_sources s
   left join app.projects p on p.id=s.source_project_id
   left join app.knowledge_entry_versions v on v.id=s.knowledge_version_id
   left join app.knowledge_entries k on k.id=v.knowledge_entry_id
   left join app.artifact_document_versions a on a.id=s.artifact_version_id
   left join app.external_reference_observations e on e.id=s.external_observation_id
   left join app.publication_observations o on o.id=s.publication_observation_id
   left join app.github_code_file_observations code on code.id=s.github_code_file_observation_id
   left join app.github_code_corpora corpus on corpus.id=code.corpus_id
   where s.assessment_id=requested_assessment_id and (p.id is null or p.status<>'active'
     or (s.knowledge_version_id is not null and (v.version_number<>k.latest_version or k.status<>'confirmed'))
     or (s.artifact_version_id is not null and exists(select 1 from app.artifact_document_versions newer where newer.document_id=a.document_id and newer.status='validated' and newer.version>a.version))
     or (s.external_observation_id is not null and exists(select 1 from app.external_reference_observations newer where newer.external_reference_id=e.external_reference_id and newer.id>e.id))
     or (s.publication_observation_id is not null and exists(select 1 from app.publication_observations newer where newer.publication_job_id=o.publication_job_id and newer.id>o.id))
     or (s.github_code_file_observation_id is not null and exists(select 1 from app.github_code_file_observations newer join app.github_code_corpora newer_corpus on newer_corpus.id=newer.corpus_id where newer.project_id=code.project_id and newer.path=code.path and newer_corpus.repository=corpus.repository and newer.id>code.id))));
$$;

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

create or replace function app.graph_endpoint_exists(endpoint_kind text, endpoint_id uuid, scope_id bigint, tenant_id bigint)
returns boolean language plpgsql stable security invoker set search_path = '' as $$
begin
  case endpoint_kind
    when 'project' then return exists(select 1 from app.projects where id=scope_id and public_id=endpoint_id and workspace_id=tenant_id);
    when 'context_node' then return exists(select 1 from app.context_nodes where public_id=endpoint_id and project_id=scope_id and workspace_id=tenant_id);
    when 'knowledge_entry' then return exists(select 1 from app.knowledge_entries where public_id=endpoint_id and project_id=scope_id and workspace_id=tenant_id);
    when 'knowledge_entry_version' then return exists(select 1 from app.knowledge_entry_versions where public_id=endpoint_id and project_id=scope_id and workspace_id=tenant_id);
    when 'deliverable' then return exists(select 1 from app.deliverables where public_id=endpoint_id and project_id=scope_id and workspace_id=tenant_id);
    when 'context_pack' then return exists(select 1 from app.context_packs where public_id=endpoint_id and project_id=scope_id and workspace_id=tenant_id);
    when 'artifact' then return exists(select 1 from app.artifacts where public_id=endpoint_id and project_id=scope_id and workspace_id=tenant_id);
    when 'artifact_document_version' then return exists(select 1 from app.artifact_document_versions where public_id=endpoint_id and project_id=scope_id and workspace_id=tenant_id);
    when 'external_reference' then return exists(select 1 from app.external_references where public_id=endpoint_id and project_id=scope_id and workspace_id=tenant_id);
    when 'external_reference_observation' then return exists(select 1 from app.external_reference_observations where public_id=endpoint_id and project_id=scope_id and workspace_id=tenant_id);
    when 'publication_observation' then return exists(select 1 from app.publication_observations o join app.publication_jobs j on j.id=o.publication_job_id where o.public_id=endpoint_id and j.project_id=scope_id and o.workspace_id=tenant_id);
    when 'task' then return exists(select 1 from app.tasks where public_id=endpoint_id and project_id=scope_id and workspace_id=tenant_id);
    when 'execution' then return exists(select 1 from app.executions where public_id=endpoint_id and project_id=scope_id and workspace_id=tenant_id);
    when 'insight' then return exists(select 1 from app.insights where public_id=endpoint_id and project_id=scope_id and workspace_id=tenant_id);
    when 'github_code_file_observation' then return exists(select 1 from app.github_code_file_observations where public_id=endpoint_id and project_id=scope_id and workspace_id=tenant_id);
    else return false;
  end case;
end;
$$;
