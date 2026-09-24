-- Cross-scope Steward receipts preserve the historical local-source FKs.
create table app.steward_scope_sources (
  id bigint generated always as identity primary key,
  workspace_id bigint not null references app.workspaces(id),
  project_id bigint not null,
  assessment_id bigint not null,
  source_project_id bigint not null,
  source_role text not null check(source_role in ('left','right')),
  source_kind text not null check(source_kind in ('knowledge_entry_version','artifact_document_version','external_reference_observation','publication_observation')),
  source_public_id uuid not null,
  knowledge_version_id bigint,
  artifact_version_id bigint,
  external_observation_id bigint,
  publication_observation_id bigint,
  source_snapshot jsonb not null check(jsonb_typeof(source_snapshot)='object' and octet_length(source_snapshot::text)<=65536),
  unique(assessment_id,source_role),
  foreign key(project_id,workspace_id) references app.projects(id,workspace_id),
  foreign key(assessment_id,project_id) references app.steward_assessments(id,project_id),
  foreign key(source_project_id,workspace_id) references app.projects(id,workspace_id),
  foreign key(knowledge_version_id,source_project_id) references app.knowledge_entry_versions(id,project_id),
  foreign key(artifact_version_id,source_project_id) references app.artifact_document_versions(id,project_id),
  foreign key(external_observation_id,source_project_id) references app.external_reference_observations(id,project_id),
  foreign key(publication_observation_id,workspace_id) references app.publication_observations(id,workspace_id),
  check(num_nonnulls(knowledge_version_id,artifact_version_id,external_observation_id,publication_observation_id)=1),
  check((source_kind='knowledge_entry_version')=(knowledge_version_id is not null)),
  check((source_kind='artifact_document_version')=(artifact_version_id is not null)),
  check((source_kind='external_reference_observation')=(external_observation_id is not null)),
  check((source_kind='publication_observation')=(publication_observation_id is not null))
);
create index steward_scope_sources_workspace_idx on app.steward_scope_sources(workspace_id);
create index steward_scope_sources_project_idx on app.steward_scope_sources(project_id);
create index steward_scope_sources_source_project_idx on app.steward_scope_sources(source_project_id);
create index steward_scope_sources_knowledge_idx on app.steward_scope_sources(knowledge_version_id) where knowledge_version_id is not null;
create index steward_scope_sources_artifact_idx on app.steward_scope_sources(artifact_version_id) where artifact_version_id is not null;
create index steward_scope_sources_external_idx on app.steward_scope_sources(external_observation_id) where external_observation_id is not null;
create index steward_scope_sources_publication_idx on app.steward_scope_sources(publication_observation_id) where publication_observation_id is not null;
alter table app.steward_scope_sources enable row level security;
alter table app.steward_scope_sources force row level security;
create policy steward_scope_sources_read on app.steward_scope_sources for select using
 (workspace_id=app.current_workspace_id() and app.has_workspace_role(workspace_id,array['owner','editor','viewer']));
create policy steward_scope_sources_write on app.steward_scope_sources for insert with check
 (workspace_id=app.current_workspace_id() and app.has_workspace_role(workspace_id,array['owner','editor']));
create trigger steward_scope_sources_immutable before update or delete on app.steward_scope_sources
 for each row execute function app.prevent_append_only_mutation();
revoke all on app.steward_scope_sources from public,anon,authenticated,service_role;
grant select,insert on app.steward_scope_sources to ai_center_runtime;
grant usage on sequence app.steward_scope_sources_id_seq to ai_center_runtime;

create or replace function app.validate_steward_scope_source()
returns trigger language plpgsql security invoker set search_path='' as $$
declare actual_id uuid; actual_project bigint;
begin
 case new.source_kind
 when 'knowledge_entry_version' then select public_id,project_id into actual_id,actual_project from app.knowledge_entry_versions where id=new.knowledge_version_id;
 when 'artifact_document_version' then select public_id,project_id into actual_id,actual_project from app.artifact_document_versions where id=new.artifact_version_id and status='validated';
 when 'external_reference_observation' then select public_id,project_id into actual_id,actual_project from app.external_reference_observations where id=new.external_observation_id;
 when 'publication_observation' then select o.public_id,j.project_id into actual_id,actual_project from app.publication_observations o join app.publication_jobs j on j.id=o.publication_job_id where o.id=new.publication_observation_id;
 end case;
 if actual_id is null or actual_id<>new.source_public_id or actual_project<>new.source_project_id then
   raise exception 'Steward source does not match its immutable scope and version' using errcode='23514';
 end if;
 return new;
end;
$$;
revoke all on function app.validate_steward_scope_source() from public,anon,authenticated,service_role;
create trigger steward_scope_source_identity before insert on app.steward_scope_sources
 for each row execute function app.validate_steward_scope_source();

-- Human lifecycle stays intact. Freshness is a computed projection of exact
-- sources, so a historical accepted/dismissed conclusion is never rewritten.
create or replace function app.steward_scope_sources_current(requested_assessment_id bigint)
returns boolean language sql stable security invoker set search_path='' as $$
 select not exists(select 1 from app.steward_scope_sources s
   left join app.projects p on p.id=s.source_project_id
   left join app.knowledge_entry_versions v on v.id=s.knowledge_version_id
   left join app.knowledge_entries k on k.id=v.knowledge_entry_id
   left join app.artifact_document_versions a on a.id=s.artifact_version_id
   left join app.external_reference_observations e on e.id=s.external_observation_id
   left join app.publication_observations o on o.id=s.publication_observation_id
   where s.assessment_id=requested_assessment_id and (p.id is null or p.status<>'active'
     or (s.knowledge_version_id is not null and (v.version_number<>k.latest_version or k.status<>'confirmed'))
     or (s.artifact_version_id is not null and exists(select 1 from app.artifact_document_versions newer where newer.document_id=a.document_id and newer.status='validated' and newer.version>a.version))
     or (s.external_observation_id is not null and exists(select 1 from app.external_reference_observations newer where newer.external_reference_id=e.external_reference_id and newer.id>e.id))
     or (s.publication_observation_id is not null and exists(select 1 from app.publication_observations newer where newer.publication_job_id=o.publication_job_id and newer.id>o.id))));
$$;
revoke all on function app.steward_scope_sources_current(bigint) from public,anon,authenticated,service_role;
grant execute on function app.steward_scope_sources_current(bigint) to ai_center_runtime;

create or replace function app.publication_observed_context_event()
returns trigger language plpgsql security invoker set search_path='' as $$
declare job app.publication_jobs;
begin
 select * into job from app.publication_jobs where id=new.publication_job_id;
 update app.projects set graph_version=graph_version+1 where id=job.project_id;
 insert into app.domain_events(workspace_id,project_id,event_type,aggregate_kind,aggregate_public_id,payload,requested_by_actor_id)
 values(job.workspace_id,job.project_id,'publication.observed','publication_observation',new.public_id,
   jsonb_build_object('observation_public_id',new.public_id,'observation_kind',new.observation_kind),job.requested_by_actor_id);
 return new;
end;
$$;
revoke all on function app.publication_observed_context_event() from public,anon,authenticated,service_role;
create trigger publication_observed_context after insert on app.publication_observations
 for each row execute function app.publication_observed_context_event();

create or replace function app.external_observed_context_event()
returns trigger language plpgsql security invoker set search_path='' as $$
begin
 if not exists(select 1 from app.projects where workspace_id=new.workspace_id and scope_kind='company') then return new; end if;
 update app.projects set graph_version=graph_version+1 where id=new.project_id;
 insert into app.domain_events(workspace_id,project_id,event_type,aggregate_kind,aggregate_public_id,payload)
 values(new.workspace_id,new.project_id,'external_reference.observed','external_reference_observation',new.public_id,
   jsonb_build_object('observation_public_id',new.public_id,'status',new.observation_status));
 return new;
end;
$$;
revoke all on function app.external_observed_context_event() from public,anon,authenticated,service_role;
create trigger external_observed_context after insert on app.external_reference_observations
 for each row execute function app.external_observed_context_event();

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
    where event.event_type in ('knowledge.committed','knowledge.revised','artifact.validated','graph.relationship_confirmed','external_reference.observed','publication.observed')
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
revoke all on function app.list_due_steward_workspaces(integer) from public,anon,authenticated,service_role;
grant execute on function app.list_due_steward_workspaces(integer) to ai_center_runtime;

alter table app.insights drop constraint insights_type_valid;
alter table app.insights add constraint insights_type_valid check(insight_type in ('contradiction','coverage_gap','context_gap'));

create or replace function app.steward_scope_source_status(requested_assessment_id bigint)
returns text language sql stable security invoker set search_path='' as $$
 select case when not exists(select 1 from app.steward_scope_sources where assessment_id=requested_assessment_id) then null
   when not app.steward_scope_sources_current(requested_assessment_id) then 'stale'
   when exists(select 1 from app.steward_scope_sources where assessment_id=requested_assessment_id and source_snapshot->>'read_status'='insufficient') then 'unknown'
   else 'current' end;
$$;
revoke all on function app.steward_scope_source_status(bigint) from public,anon,authenticated,service_role;
grant execute on function app.steward_scope_source_status(bigint) to ai_center_runtime;

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
    else return false;
  end case;
end;
$$;
revoke all on function app.graph_endpoint_exists(text,uuid,bigint,bigint) from public,anon,authenticated,service_role;
grant execute on function app.graph_endpoint_exists(text,uuid,bigint,bigint) to ai_center_runtime;
