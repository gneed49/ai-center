-- Targeted existing-tool reads. References express attachment; observations are
-- immutable external evidence, never confirmed company rules.
alter table app.work_tool_connections add column allow_existing_reads boolean not null default false;
create index audit_remote_read_window_idx on app.audit_events(workspace_id,action,occurred_at desc)
 where action like 'work_tool.remote_read.%';

create table app.tool_source_references (
 id bigint generated always as identity primary key,
 public_id uuid not null default gen_random_uuid() unique,
 workspace_id bigint not null,
 project_id bigint not null,
 provider text not null,
 object_kind text not null,
 external_id uuid not null,
 canonical_url text not null check(octet_length(canonical_url) between 1 and 2048),
 connection_id bigint not null,
 connection_revision integer not null check(connection_revision>0),
 created_by_actor_id uuid not null,
 created_at timestamptz not null default now(),
 updated_at timestamptz not null default now(),
 status text not null default 'active' check(status in('active','detached')),
 revision integer not null default 1 check(revision>0),
 current_observation_id bigint,
 last_attempt_at timestamptz,
 last_checked_at timestamptz,
 last_check_status text check(last_check_status in('available','partial','unavailable','failed')),
 last_check_error_code text check(length(last_check_error_code) between 1 and 80),
 unique(id,workspace_id),
 unique(id,project_id),
 unique(workspace_id,project_id,provider,external_id),
 check((provider='linear' and object_kind='issue') or (provider='notion' and object_kind='page')),
 foreign key(project_id,workspace_id) references app.projects(id,workspace_id),
 foreign key(connection_id,workspace_id) references app.work_tool_connections(id,workspace_id)
);
create index tool_sources_project_idx on app.tool_source_references(project_id,status,created_at desc,id desc);
create index tool_sources_workspace_idx on app.tool_source_references(workspace_id);
create index tool_sources_connection_idx on app.tool_source_references(connection_id);
create index tool_sources_head_idx on app.tool_source_references(current_observation_id);

create table app.tool_source_observations (
 id bigint generated always as identity primary key,
 public_id uuid not null default gen_random_uuid() unique,
 workspace_id bigint not null,
 project_id bigint not null,
 reference_id bigint not null,
 version integer not null check(version>0),
 provider text not null,
 object_kind text not null,
 external_id uuid not null,
 canonical_url text not null check(octet_length(canonical_url) between 1 and 2048),
 connection_id bigint not null,
 connection_revision integer not null check(connection_revision>0),
 observed_at timestamptz not null default now(),
 remote_updated_at timestamptz,
 title text not null,
 body_markdown text not null,
 availability text not null check(availability in('available','unavailable')),
 coverage text not null check(coverage in('complete','partial','none')),
 omission_reasons jsonb not null check(jsonb_typeof(omission_reasons)='array'),
 projection_version text not null check(length(projection_version) between 1 and 80),
 content_hash text not null check(content_hash ~ '^[0-9a-f]{64}$'),
 snapshot_hash text not null check(snapshot_hash ~ '^[0-9a-f]{64}$'),
 metadata jsonb not null check(jsonb_typeof(metadata)='object'),
 unique(id,workspace_id),unique(id,project_id),unique(id,reference_id),unique(reference_id,version),
 check((provider='linear' and object_kind='issue') or (provider='notion' and object_kind='page')),
 check(octet_length(title)+octet_length(body_markdown)<=65536),
 check(octet_length(metadata::text)+octet_length(omission_reasons::text)+octet_length(title)+octet_length(body_markdown)<=131072),
 check((availability='unavailable' and body_markdown='' and coverage='none') or (availability='available' and coverage in('complete','partial'))),
 foreign key(project_id,workspace_id) references app.projects(id,workspace_id),
 foreign key(reference_id,workspace_id) references app.tool_source_references(id,workspace_id),
 foreign key(reference_id,project_id) references app.tool_source_references(id,project_id),
 foreign key(connection_id,workspace_id) references app.work_tool_connections(id,workspace_id)
);
create index tool_source_observations_workspace_idx on app.tool_source_observations(workspace_id);
create index tool_source_observations_project_idx on app.tool_source_observations(project_id);
create index tool_source_observations_connection_idx on app.tool_source_observations(connection_id);
-- Deferred circular head allows insert and operator-governed erasure in one tx.
alter table app.tool_source_references add constraint tool_source_head_reference_fkey
 foreign key(current_observation_id,id) references app.tool_source_observations(id,reference_id) deferrable initially deferred;

create or replace function app.validate_tool_source_identity()
returns trigger language plpgsql security invoker set search_path='' as $$
declare reference app.tool_source_references; connection_provider text;
begin
 if tg_table_name='tool_source_references' then
  if tg_op='UPDATE' and (new.public_id,new.workspace_id,new.project_id,new.provider,new.object_kind,new.external_id,new.created_by_actor_id,new.created_at)
    is distinct from (old.public_id,old.workspace_id,old.project_id,old.provider,old.object_kind,old.external_id,old.created_by_actor_id,old.created_at) then
   raise exception 'A source reference identity is immutable' using errcode='23514';
  end if;
 else
  select * into reference from app.tool_source_references where id=new.reference_id;
  if not found or (new.workspace_id,new.project_id,new.provider,new.object_kind,new.external_id)
    is distinct from (reference.workspace_id,reference.project_id,reference.provider,reference.object_kind,reference.external_id) then
   raise exception 'Observation does not match its reference identity' using errcode='23514';
  end if;
 end if;
 select provider into connection_provider from app.work_tool_connections where id=new.connection_id and workspace_id=new.workspace_id;
 if connection_provider is distinct from new.provider then
  raise exception 'Observation connection provider does not match' using errcode='23514';
 end if;
 return new;
end;
$$;
revoke all on function app.validate_tool_source_identity() from public,anon,authenticated,service_role;
create trigger tool_source_reference_identity before insert or update on app.tool_source_references
 for each row execute function app.validate_tool_source_identity();
create trigger tool_source_observation_identity before insert on app.tool_source_observations
 for each row execute function app.validate_tool_source_identity();
create trigger tool_source_observations_immutable before update or delete on app.tool_source_observations
 for each row execute function app.prevent_append_only_mutation();

create or replace function app.validate_tool_source_head()
returns trigger language plpgsql security invoker set search_path='' as $$
declare reference app.tool_source_references; head app.tool_source_observations; target_reference_id bigint;
begin
 if tg_table_name='tool_source_observations' then target_reference_id=new.reference_id; else target_reference_id=new.id; end if;
 select * into reference from app.tool_source_references where id=target_reference_id;
 if not found then return new; end if;
 select * into head from app.tool_source_observations where id=reference.current_observation_id and reference_id=reference.id;
 if not found or exists(select 1 from app.tool_source_observations where reference_id=reference.id and version>head.version) then
  raise exception 'A source reference requires its latest exact observation' using errcode='23514';
 end if;
 return new;
end;
$$;
revoke all on function app.validate_tool_source_head() from public,anon,authenticated,service_role;
create constraint trigger tool_source_head_required after insert or update on app.tool_source_references
 deferrable initially deferred for each row execute function app.validate_tool_source_head();
create constraint trigger tool_source_observation_head_required after insert on app.tool_source_observations
 deferrable initially deferred for each row execute function app.validate_tool_source_head();

do $$
declare relation_name text;
begin
 foreach relation_name in array array['tool_source_references','tool_source_observations'] loop
  execute format('alter table app.%I enable row level security',relation_name);
  execute format('alter table app.%I force row level security',relation_name);
  execute format('create policy tool_source_member_select on app.%I for select using(workspace_id=(select app.current_workspace_id()) and app.has_workspace_role(workspace_id,array[''owner'',''editor'',''viewer'']::text[]))',relation_name);
  execute format('create policy tool_source_editor_insert on app.%I for insert with check(workspace_id=(select app.current_workspace_id()) and app.has_workspace_role(workspace_id,array[''owner'',''editor'']::text[]))',relation_name);
  execute format('revoke all on app.%I from public,anon,authenticated,service_role',relation_name);
 end loop;
end;
$$;
create policy tool_source_editor_update on app.tool_source_references for update
 using(workspace_id=(select app.current_workspace_id()) and app.has_workspace_role(workspace_id,array['owner','editor']))
 with check(workspace_id=(select app.current_workspace_id()) and app.has_workspace_role(workspace_id,array['owner','editor']));

create or replace function app.tool_source_observation_current(observation_id bigint)
returns boolean language sql stable security invoker set search_path='' as $$
 select exists(select 1 from app.tool_source_observations o
 join app.tool_source_references r on r.id=o.reference_id and r.workspace_id=o.workspace_id and r.project_id=o.project_id
 join app.projects p on p.id=r.project_id and p.workspace_id=r.workspace_id
 join app.work_tool_connections c on c.id=r.connection_id and c.workspace_id=r.workspace_id and c.provider=r.provider
 where o.id=observation_id and o.workspace_id=app.current_workspace_id()
  and r.current_observation_id=o.id and r.status='active' and p.status='active'
  and o.availability='available' and c.enabled and c.allow_existing_reads and c.revision=r.connection_revision);
$$;
revoke all on function app.tool_source_observation_current(bigint) from public,anon,authenticated,service_role;

-- A receipt's authority is independent from its immutable business snapshot.
-- Null on historical receipts: no credential authority is reconstructed.
alter table app.publication_observations
 add column connection_id bigint,
 add column connection_revision integer,
 add constraint publication_observation_connection_fkey foreign key(connection_id,workspace_id) references app.work_tool_connections(id,workspace_id),
 add constraint publication_observation_attestation_check check((connection_id is null and connection_revision is null) or (connection_id is not null and connection_revision is not null and connection_revision>0));
create index publication_observation_connection_idx on app.publication_observations(connection_id) where connection_id is not null;

-- Bound by bytes without splitting a Unicode code point.
create or replace function app.context_utf8_prefix(value text,max_bytes integer)
returns text language plpgsql immutable strict security invoker set search_path='' as $$
declare low_bound integer=0; high_bound integer=length(value); midpoint integer;
begin
 if max_bytes<0 then raise exception 'Invalid text budget' using errcode='22023'; end if;
 if octet_length(value)<=max_bytes then return value; end if;
 while low_bound<high_bound loop
  midpoint=(low_bound+high_bound+1)/2;
  if octet_length(left(value,midpoint))<=max_bytes then low_bound=midpoint; else high_bound=midpoint-1; end if;
 end loop;
 return left(value,low_bound);
end;
$$;
revoke all on function app.context_utf8_prefix(text,integer) from public,anon,authenticated,service_role;

-- Provider dates are evidence only when strict ISO time can be parsed. Invalid
-- legacy text remains historical raw metadata, not an invented timestamp.
create or replace function app.observed_remote_time(value text)
returns timestamptz language plpgsql stable strict security invoker set search_path='' as $$
begin
 if length(value)>80 or value !~ '^[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}(\.[0-9]+)?(Z|[+-][0-9]{2}:[0-9]{2})$' then return null; end if;
 begin
  return value::timestamptz;
 exception when invalid_datetime_format or datetime_field_overflow then return null;
 end;
end;
$$;
revoke all on function app.observed_remote_time(text) from public,anon,authenticated,service_role;

create view app.publication_source_observations with(security_invoker=true) as
with base as (
 select o.id,o.public_id,o.workspace_id,j.project_id,o.publication_job_id,j.public_id as publication_public_id,j.provider,
  case when j.provider='notion' then 'page' else 'issue' end as object_kind,
  case when o.external_id ~* '^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$' then o.external_id::uuid else null end as external_id,
  o.external_url as canonical_url,o.observed_at,o.remote_updated_at,
  case when o.observation_kind='unavailable' then o.external_id else app.context_utf8_prefix(coalesce(o.snapshot->>'title',j.title),4096) end as title,
  case when o.observation_kind='unavailable' then '' else app.context_utf8_prefix(coalesce(o.snapshot->>'body_markdown',''),61440) end as body_markdown,
  case when o.observation_kind='unavailable' then 'unavailable' else 'available' end as availability,
  case when o.observation_kind='unavailable' then 'none'
   when o.snapshot->>'complete'='true' and app.observed_remote_time(o.remote_updated_at) is not null and octet_length(coalesce(o.snapshot->>'body_markdown',''))<=61440 and octet_length(coalesce(o.snapshot->>'title',j.title))<=4096 then 'complete' else 'partial' end as coverage,
  case when o.observation_kind='unavailable' then '[]'::jsonb else
   jsonb_build_array('comments_not_read','attachments_not_read','related_objects_not_read')
    ||case when j.provider='notion' then jsonb_build_array('properties_not_read','embedded_content_not_read','transcripts_not_read') else '[]'::jsonb end
    ||case when coalesce(o.snapshot->>'complete','false')<>'true' then jsonb_build_array('provider_truncated') else '[]'::jsonb end
    ||case when octet_length(coalesce(o.snapshot->>'body_markdown',''))>61440 or octet_length(coalesce(o.snapshot->>'title',j.title))>4096 then jsonb_build_array('local_text_limit') else '[]'::jsonb end
    ||case when app.observed_remote_time(o.remote_updated_at) is null then jsonb_build_array('remote_date_unavailable') else '[]'::jsonb end end as omission_reasons,
  'publication-text-v1'::text as projection_version,'{}'::jsonb as metadata,
  o.connection_id,o.connection_revision
 from app.publication_observations o join app.publication_jobs j on j.id=o.publication_job_id and j.workspace_id=o.workspace_id
 where j.provider in('notion','linear')
), signatures as (
 select b.*, jsonb_build_object('title',title,'body_markdown',body_markdown) as business,
  jsonb_build_object('provider',provider,'external_id',external_id,'canonical_url',canonical_url,'availability',availability,
   'coverage',coverage,'omission_reasons',omission_reasons,'projection_version',projection_version,'metadata',metadata) as evidence
 from base b
), boundaries as (
 select s.*, case when lag(business||evidence) over(partition by publication_job_id order by id) is not distinct from business||evidence then 0 else 1 end as boundary
 from signatures s
), groups as (
 select b.*,sum(boundary) over(partition by publication_job_id order by id)::integer as version from boundaries b
), identified as (
 select g.*,first_value(id) over(partition by publication_job_id,version order by id) as canonical_observation_id,
 first_value(public_id) over(partition by publication_job_id,version order by id) as canonical_observation_public_id,
 max(id) over(partition by publication_job_id,version) as group_latest_id,
 max(id) over(partition by publication_job_id) as latest_observation_id
 from groups g
)
select i.id,i.public_id,i.workspace_id,i.project_id,i.publication_job_id,i.publication_public_id,i.provider,i.object_kind,i.external_id,i.canonical_url,
 i.version,i.canonical_observation_id,i.canonical_observation_public_id,i.latest_observation_id,
 i.group_latest_id=i.latest_observation_id as is_current,i.observed_at,i.remote_updated_at,i.title,i.body_markdown,i.availability,i.coverage,i.omission_reasons,
 encode(sha256(convert_to(i.business::text,'UTF8')),'hex') as content_hash,
 encode(sha256(convert_to((i.business||i.evidence)::text,'UTF8')),'hex') as snapshot_hash,
 i.projection_version,i.metadata,authority.connection_id as authority_connection_id,c.public_id as authority_connection_public_id,
 authority.connection_revision as authority_connection_revision
from identified i join app.publication_observations authority on authority.id=i.group_latest_id
left join app.work_tool_connections c on c.id=authority.connection_id and c.workspace_id=i.workspace_id;
revoke all on app.publication_source_observations from public,anon,authenticated,service_role;

create or replace function app.publication_observation_current(observation_id bigint)
returns boolean language sql stable security invoker set search_path='' as $$
 select exists(select 1 from app.publication_source_observations o
 join app.projects p on p.id=o.project_id and p.workspace_id=o.workspace_id
 join app.work_tool_connections c on c.id=o.authority_connection_id and c.workspace_id=o.workspace_id and c.provider=o.provider
 where o.id=observation_id and o.workspace_id=app.current_workspace_id() and o.is_current
  and o.external_id is not null and o.availability='available' and p.status='active'
  and c.enabled and c.revision=o.authority_connection_revision);
$$;
revoke all on function app.publication_observation_current(bigint) from public,anon,authenticated,service_role;

-- Typed, scope-exact provenance. No external observation can become mandatory.
alter table app.artifact_version_sources
 add column tool_source_observation_id bigint,
 add column publication_observation_id bigint,
 drop constraint artifact_version_sources_source_kind_check,
 drop constraint artifact_version_sources_check,
 add constraint artifact_version_sources_source_kind_check check(source_kind in('knowledge','context_pack','deliverable','session','artifact_version','tool_source_observation','publication_observation')),
 add constraint artifact_version_sources_check check(
  num_nonnulls(knowledge_version_id,context_pack_id,deliverable_id,session_id,source_artifact_version_id,tool_source_observation_id,publication_observation_id)=1
  and (source_kind='knowledge')=(knowledge_version_id is not null)
  and (source_kind='context_pack')=(context_pack_id is not null)
  and (source_kind='deliverable')=(deliverable_id is not null)
  and (source_kind='session')=(session_id is not null)
  and (source_kind='artifact_version')=(source_artifact_version_id is not null)
  and (source_kind='tool_source_observation')=(tool_source_observation_id is not null)
  and (source_kind='publication_observation')=(publication_observation_id is not null)),
 add constraint artifact_sources_tool_scope_fkey foreign key(tool_source_observation_id,source_project_id) references app.tool_source_observations(id,project_id),
 add constraint artifact_sources_publication_scope_fkey foreign key(publication_observation_id,workspace_id) references app.publication_observations(id,workspace_id);
create index artifact_sources_tool_idx on app.artifact_version_sources(tool_source_observation_id) where tool_source_observation_id is not null;
create index artifact_sources_publication_idx on app.artifact_version_sources(publication_observation_id) where publication_observation_id is not null;

alter table app.context_pack_scope_sources
 add column tool_source_observation_id bigint,
 add column publication_observation_id bigint,
 drop constraint context_pack_scope_sources_source_kind_check,
 drop constraint context_pack_scope_sources_check,
 add constraint context_pack_scope_sources_source_kind_check check(source_kind in('knowledge_entry_version','artifact_document_version','tool_source_observation','publication_observation')),
 add constraint context_pack_scope_sources_check check(
  num_nonnulls(knowledge_version_id,artifact_version_id,tool_source_observation_id,publication_observation_id)=1
  and (source_kind='knowledge_entry_version')=(knowledge_version_id is not null)
  and (source_kind='artifact_document_version')=(artifact_version_id is not null)
  and (source_kind='tool_source_observation')=(tool_source_observation_id is not null)
  and (source_kind='publication_observation')=(publication_observation_id is not null)
  and (source_kind not in('tool_source_observation','publication_observation') or not is_mandatory)),
 add constraint pack_sources_tool_scope_fkey foreign key(tool_source_observation_id,source_project_id) references app.tool_source_observations(id,project_id),
 add constraint pack_sources_publication_scope_fkey foreign key(publication_observation_id,workspace_id) references app.publication_observations(id,workspace_id);
create index pack_sources_tool_idx on app.context_pack_scope_sources(tool_source_observation_id) where tool_source_observation_id is not null;
create index pack_sources_publication_idx on app.context_pack_scope_sources(publication_observation_id) where publication_observation_id is not null;

create or replace function app.validate_external_source_provenance()
returns trigger language plpgsql security invoker set search_path='' as $$
declare actual_uuid uuid; actual_project bigint;
begin
 case new.source_kind
 when 'tool_source_observation' then select public_id,project_id into actual_uuid,actual_project from app.tool_source_observations where id=new.tool_source_observation_id and workspace_id=new.workspace_id;
 when 'publication_observation' then select o.public_id,j.project_id into actual_uuid,actual_project from app.publication_observations o join app.publication_jobs j on j.id=o.publication_job_id where o.id=new.publication_observation_id and o.workspace_id=new.workspace_id;
 else return new;
 end case;
 if actual_uuid is distinct from new.source_public_id or actual_project is distinct from new.source_project_id then
  raise exception 'External source must identify the exact observation and scope' using errcode='23514';
 end if;
 return new;
end;
$$;
revoke all on function app.validate_external_source_provenance() from public,anon,authenticated,service_role;
create trigger artifact_external_source_identity before insert on app.artifact_version_sources
 for each row execute function app.validate_external_source_provenance();

create or replace function app.validate_scope_source_identity()
returns trigger language plpgsql security invoker set search_path='' as $$
declare actual_id uuid; actual_project bigint;
begin
 case new.source_kind
 when 'knowledge_entry_version' then select public_id,project_id into actual_id,actual_project from app.knowledge_entry_versions where id=new.knowledge_version_id;
 when 'artifact_document_version' then select public_id,project_id into actual_id,actual_project from app.artifact_document_versions where id=new.artifact_version_id and status='validated';
 when 'tool_source_observation' then select public_id,project_id into actual_id,actual_project from app.tool_source_observations where id=new.tool_source_observation_id;
 when 'publication_observation' then select o.public_id,j.project_id into actual_id,actual_project from app.publication_observations o join app.publication_jobs j on j.id=o.publication_job_id where o.id=new.publication_observation_id;
 else raise exception 'Unknown context source' using errcode='23514';
 end case;
 if actual_id is null or actual_id<>new.source_public_id or actual_project<>new.source_project_id then
  raise exception 'Scoped source identity does not match its immutable version' using errcode='23514';
 end if;
 return new;
end;
$$;
revoke all on function app.validate_scope_source_identity() from public,anon,authenticated,service_role;

create or replace function app.context_pack_scopes_current(requested_pack_id bigint)
returns boolean language sql stable security invoker set search_path='' as $$
  select exists(select 1 from app.context_packs p where p.id=requested_pack_id and p.workspace_id=app.current_workspace_id())
    -- Graph stamps are receipts and optimistic concurrency guards, not a
    -- blanket invalidation rule. Only included exact sources govern freshness.
    and not exists(select 1 from app.context_pack_scope_versions s left join app.projects p on p.id=s.source_project_id
      where s.context_pack_id=requested_pack_id and (p.id is null or p.status<>'active'))
    and not exists(select 1 from app.context_pack_sources s
      join app.knowledge_entry_versions v on v.id=s.knowledge_entry_version_id
      join app.knowledge_entries k on k.id=v.knowledge_entry_id
      where s.context_pack_id=requested_pack_id and (v.version_number<>k.latest_version or k.status<>'confirmed'))
    and not exists(select 1 from app.context_pack_scope_sources s
      left join app.knowledge_entry_versions v on v.id=s.knowledge_version_id
      left join app.knowledge_entries k on k.id=v.knowledge_entry_id
      left join app.artifact_document_versions a on a.id=s.artifact_version_id
      where s.context_pack_id=requested_pack_id and s.decision='included' and
        ((s.source_kind='tool_source_observation' and not app.tool_source_observation_current(s.tool_source_observation_id))
          or (s.source_kind='publication_observation' and not app.publication_observation_current(s.publication_observation_id))
          or (s.source_kind='knowledge_entry_version' and (v.id is null or v.version_number<>k.latest_version or k.status<>'confirmed'))
          or (s.source_kind='artifact_document_version' and (a.id is null or a.status<>'validated' or exists(
            select 1 from app.artifact_document_versions newer where newer.document_id=a.document_id
              and newer.status='validated' and newer.version>a.version)))));
$$;
revoke all on function app.context_pack_scopes_current(bigint) from public,anon,authenticated,service_role;
grant execute on function app.context_pack_scopes_current(bigint) to ai_center_runtime;


-- Conversations participate in validated scoped graph relationships.
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
    when 'tool_source_reference' then return exists(select 1 from app.tool_source_references where public_id=endpoint_id and project_id=scope_id and workspace_id=tenant_id);
    when 'tool_source_observation' then return exists(select 1 from app.tool_source_observations where public_id=endpoint_id and project_id=scope_id and workspace_id=tenant_id);
    when 'session' then return exists(select 1 from app.sessions where public_id=endpoint_id and project_id=scope_id and workspace_id=tenant_id);
    when 'task' then return exists(select 1 from app.tasks where public_id=endpoint_id and project_id=scope_id and workspace_id=tenant_id);
    when 'execution' then return exists(select 1 from app.executions where public_id=endpoint_id and project_id=scope_id and workspace_id=tenant_id);
    when 'insight' then return exists(select 1 from app.insights where public_id=endpoint_id and project_id=scope_id and workspace_id=tenant_id);
    when 'github_code_file_observation' then return exists(select 1 from app.github_code_file_observations where public_id=endpoint_id and project_id=scope_id and workspace_id=tenant_id);
    else return false;
  end case;
end;
$$;
revoke all on function app.graph_endpoint_exists(text,uuid,bigint,bigint) from public,anon,authenticated,service_role;
grant execute on function app.graph_endpoint_exists(text,uuid,bigint,bigint) to ai_center_runtime;

-- References emit only actual context or authority changes, never mere checks.
create or replace function app.tool_source_context_event()
returns trigger language plpgsql security invoker set search_path='' as $$
begin
 if tg_op='INSERT' and new.current_observation_id is null then return new; end if;
 if tg_op='UPDATE' and (new.current_observation_id,new.status,new.connection_id,new.connection_revision)
  is not distinct from (old.current_observation_id,old.status,old.connection_id,old.connection_revision) then return new; end if;
 update app.projects set graph_version=graph_version+1 where id=new.project_id;
 with changed as (
  update app.context_packs p set status='stale',invalidated_at=now(),stale_reason='included external source changed or became unavailable'
  where p.workspace_id=new.workspace_id and p.status='current' and exists(
   select 1 from app.context_pack_scope_sources s join app.tool_source_observations o on o.id=s.tool_source_observation_id
   where s.context_pack_id=p.id and s.decision='included' and o.reference_id=new.id and not app.tool_source_observation_current(o.id)) returning p.id
 ) update app.deliverables d set status='stale',stale_at=now() where d.source_context_pack_id in(select id from changed) and d.status in('draft','committed');
 insert into app.domain_events(workspace_id,project_id,event_type,aggregate_kind,aggregate_public_id,payload,requested_by_actor_id)
 values(new.workspace_id,new.project_id,'tool_source.observed','tool_source_reference',new.public_id,
  jsonb_build_object('reference_public_id',new.public_id,'status',new.status),app.current_actor_id());
 return new;
end;
$$;
revoke all on function app.tool_source_context_event() from public,anon,authenticated,service_role;
create trigger tool_source_reference_context after insert or update on app.tool_source_references
 for each row execute function app.tool_source_context_event();

-- Keep receipts even if their creator loses access. Only attest current authority
-- after acquiring project, connection advisory, then the exact lease/job lock.
create or replace function app.finish_publication_job(requested_job uuid,requested_lease uuid,new_status text,
 safe_error text,receipt jsonb)
returns boolean language plpgsql security definer set search_path='' set row_security=off as $$
declare job app.publication_jobs; connection app.work_tool_connections; authority_id bigint; authority_revision integer;
begin
 if new_status not in('succeeded','failed','needs_review','cancelled') or length(safe_error)>80 then
  raise exception 'Invalid publication settlement' using errcode='23514';
 end if;
 select * into job from app.publication_jobs where public_id=requested_job and lease_token=requested_lease and status in('processing','needs_review');
 if not found then return false; end if;
 perform id from app.projects where id=job.project_id for no key update;
 select * into connection from app.work_tool_connections where id=job.connection_id;
 perform pg_advisory_xact_lock_shared(hashtextextended(job.workspace_id::text||':work-tool:'||connection.public_id::text,0));
 select * into job from app.publication_jobs where public_id=requested_job and lease_token=requested_lease and status in('processing','needs_review') for update;
 if not found then return false; end if;
 select * into connection from app.work_tool_connections where id=job.connection_id;
 if connection.enabled and connection.workspace_id=job.workspace_id and connection.provider=job.provider and connection.revision=job.connection_revision
  and exists(select 1 from app.workspace_members m where m.workspace_id=job.workspace_id and m.actor_id=job.requested_by_actor_id and m.invitation_status='accepted' and m.role in('owner','editor')) then
  authority_id=connection.id; authority_revision=connection.revision;
 end if;
 if new_status='succeeded' then
  if receipt is null or jsonb_typeof(receipt)<>'object' or coalesce(length(receipt->>'external_id'),0) not between 1 and 256
   or coalesce(length(receipt->>'external_url'),0) not between 1 and 2048 or octet_length(receipt::text)>262144 then
   raise exception 'Invalid publication receipt' using errcode='23514';
  end if;
  insert into app.publication_observations(workspace_id,publication_job_id,observation_kind,external_id,external_url,remote_updated_at,snapshot,connection_id,connection_revision)
   values(job.workspace_id,job.id,'created',receipt->>'external_id',receipt->>'external_url',receipt->>'remote_updated_at',receipt,authority_id,authority_revision);
 end if;
 update app.publication_jobs set status=new_status,error_code=safe_error,
  external_id=case when new_status='succeeded' then receipt->>'external_id' else external_id end,
  external_url=case when new_status='succeeded' then receipt->>'external_url' else external_url end,
  updated_at=now() where id=job.id;
 return true;
end;
$$;
revoke all on function app.finish_publication_job(uuid,uuid,text,text,jsonb) from public,anon,authenticated,service_role;

create or replace function app.publication_observed_context_event()
returns trigger language plpgsql security invoker set search_path='' as $$
declare job app.publication_jobs; previous app.publication_observations; previous_group bigint; current_group bigint;
begin
 select * into job from app.publication_jobs where id=new.publication_job_id;
 select * into previous from app.publication_observations where publication_job_id=new.publication_job_id and id<new.id order by id desc limit 1;
 if job.provider in('linear','notion') then
  select canonical_observation_id into previous_group from app.publication_source_observations where id=previous.id;
  select canonical_observation_id into current_group from app.publication_source_observations where id=new.id;
  if previous_group is not distinct from current_group and (new.connection_id,new.connection_revision) is not distinct from (previous.connection_id,previous.connection_revision) then return new; end if;
 end if;
 update app.projects set graph_version=graph_version+1 where id=job.project_id;
 with changed as (
  update app.context_packs p set status='stale',invalidated_at=now(),stale_reason='included published observation changed or became unavailable'
  where p.workspace_id=new.workspace_id and p.status='current' and exists(
   select 1 from app.context_pack_scope_sources s join app.publication_observations o on o.id=s.publication_observation_id
   where s.context_pack_id=p.id and s.decision='included' and o.publication_job_id=new.publication_job_id and not app.publication_observation_current(o.id)) returning p.id
 ) update app.deliverables d set status='stale',stale_at=now() where d.source_context_pack_id in(select id from changed) and d.status in('draft','committed');
 insert into app.domain_events(workspace_id,project_id,event_type,aggregate_kind,aggregate_public_id,payload,requested_by_actor_id)
 values(job.workspace_id,job.project_id,'publication.observed','publication_observation',new.public_id,
  jsonb_build_object('observation_public_id',new.public_id,'observation_kind',new.observation_kind),job.requested_by_actor_id);
 return new;
end;
$$;
revoke all on function app.publication_observed_context_event() from public,anon,authenticated,service_role;

alter table app.steward_scope_sources
 add column tool_source_observation_id bigint,
 drop constraint steward_scope_sources_source_kind_check,
 drop constraint steward_scope_sources_check,
 add constraint steward_scope_sources_source_kind_check check(source_kind in('knowledge_entry_version','artifact_document_version','external_reference_observation','publication_observation','github_code_file_observation','tool_source_observation')),
 add constraint steward_scope_sources_check check(num_nonnulls(knowledge_version_id,artifact_version_id,external_observation_id,publication_observation_id,github_code_file_observation_id,tool_source_observation_id)=1),
 add constraint steward_scope_sources_tool_kind_check check((source_kind='tool_source_observation')=(tool_source_observation_id is not null)),
 add constraint steward_sources_tool_scope_fkey foreign key(tool_source_observation_id,source_project_id) references app.tool_source_observations(id,project_id);
create index steward_sources_tool_idx on app.steward_scope_sources(tool_source_observation_id) where tool_source_observation_id is not null;

create or replace function app.validate_steward_scope_source()
returns trigger language plpgsql security invoker set search_path='' as $$
declare actual_id uuid; actual_project bigint;
begin
 case new.source_kind
 when 'knowledge_entry_version' then select public_id,project_id into actual_id,actual_project from app.knowledge_entry_versions where id=new.knowledge_version_id;
 when 'artifact_document_version' then select public_id,project_id into actual_id,actual_project from app.artifact_document_versions where id=new.artifact_version_id and status='validated';
 when 'external_reference_observation' then select public_id,project_id into actual_id,actual_project from app.external_reference_observations where id=new.external_observation_id;
 when 'publication_observation' then select o.public_id,j.project_id into actual_id,actual_project from app.publication_observations o join app.publication_jobs j on j.id=o.publication_job_id where o.id=new.publication_observation_id;
 when 'tool_source_observation' then select public_id,project_id into actual_id,actual_project from app.tool_source_observations where id=new.tool_source_observation_id;
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
     or (s.tool_source_observation_id is not null and not app.tool_source_observation_current(s.tool_source_observation_id))
     or (s.publication_observation_id is not null and (case when exists(select 1 from app.publication_jobs j where j.id=o.publication_job_id and j.provider in('notion','linear')) then not app.publication_observation_current(s.publication_observation_id) else exists(select 1 from app.publication_observations newer where newer.publication_job_id=o.publication_job_id and newer.id>o.id) end))
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
    where event.event_type in ('knowledge.committed','knowledge.revised','artifact.validated','graph.relationship_confirmed','external_reference.observed','publication.observed','github_code.observed','tool_source.observed','steward.continue')
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


-- An attested receipt belongs to the publication's actual connector. Historical
-- unverified receipts remain readable with a null attestation.
create or replace function app.validate_publication_attestation()
returns trigger language plpgsql security invoker set search_path='' as $$
begin
 if new.connection_id is not null and not exists(
  select 1 from app.publication_jobs j join app.work_tool_connections c on c.id=j.connection_id and c.workspace_id=j.workspace_id
  where j.id=new.publication_job_id and j.workspace_id=new.workspace_id and c.id=new.connection_id
   and c.revision=new.connection_revision and c.provider=j.provider and c.enabled) then
  raise exception 'Publication observation authority does not match its connection' using errcode='23514';
 end if;
 return new;
end;
$$;
revoke all on function app.validate_publication_attestation() from public,anon,authenticated,service_role;
create trigger publication_observation_attestation before insert on app.publication_observations
 for each row execute function app.validate_publication_attestation();
