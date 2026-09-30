-- Additive cross-scope provenance. Historical local source tables retain all
-- their original same-project foreign keys and append-only records.
create table app.context_pack_scope_versions (
  id bigint generated always as identity primary key,
  workspace_id bigint not null references app.workspaces(id),
  project_id bigint not null,
  context_pack_id bigint not null,
  source_project_id bigint not null,
  graph_version bigint not null check(graph_version>=0),
  unique(context_pack_id,source_project_id),
  foreign key(project_id,workspace_id) references app.projects(id,workspace_id),
  foreign key(context_pack_id,project_id) references app.context_packs(id,project_id),
  foreign key(source_project_id,workspace_id) references app.projects(id,workspace_id)
);
create index context_pack_scope_versions_workspace_idx on app.context_pack_scope_versions(workspace_id);
create index context_pack_scope_versions_source_idx on app.context_pack_scope_versions(source_project_id);
create index context_pack_scope_versions_project_idx on app.context_pack_scope_versions(project_id);

-- One immutable receipt per offered cross-scope knowledge/artifact candidate.
-- Included and excluded decisions both retain exact, typed provenance.
create table app.context_pack_scope_sources (
  id bigint generated always as identity primary key,
  workspace_id bigint not null references app.workspaces(id),
  project_id bigint not null,
  context_pack_id bigint not null,
  source_project_id bigint not null,
  source_kind text not null check(source_kind in ('knowledge_entry_version','artifact_document_version')),
  source_public_id uuid not null,
  knowledge_version_id bigint,
  artifact_version_id bigint,
  decision text not null check(decision in ('included','excluded')),
  reason_code text not null check(btrim(reason_code)<>''),
  explanation text not null,
  rank integer not null check(rank>0),
  estimated_tokens integer not null check(estimated_tokens>=0),
  is_mandatory boolean not null,
  unique(context_pack_id,source_kind,source_public_id),
  foreign key(project_id,workspace_id) references app.projects(id,workspace_id),
  foreign key(context_pack_id,project_id) references app.context_packs(id,project_id),
  foreign key(source_project_id,workspace_id) references app.projects(id,workspace_id),
  foreign key(knowledge_version_id,source_project_id) references app.knowledge_entry_versions(id,project_id),
  foreign key(artifact_version_id,source_project_id) references app.artifact_document_versions(id,project_id),
  check((source_kind='knowledge_entry_version' and knowledge_version_id is not null and artifact_version_id is null)
    or (source_kind='artifact_document_version' and knowledge_version_id is null and artifact_version_id is not null)),
  check(not is_mandatory or decision='included')
);
create index context_pack_scope_sources_workspace_idx on app.context_pack_scope_sources(workspace_id);
create index context_pack_scope_sources_project_idx on app.context_pack_scope_sources(project_id);
create index context_pack_scope_sources_scope_idx on app.context_pack_scope_sources(source_project_id);
create index context_pack_scope_sources_knowledge_idx on app.context_pack_scope_sources(knowledge_version_id) where knowledge_version_id is not null;
create index context_pack_scope_sources_artifact_idx on app.context_pack_scope_sources(artifact_version_id) where artifact_version_id is not null;

alter table app.context_pack_scope_versions enable row level security;
alter table app.context_pack_scope_versions force row level security;
alter table app.context_pack_scope_sources enable row level security;
alter table app.context_pack_scope_sources force row level security;
create policy scope_versions_read on app.context_pack_scope_versions for select using
  (workspace_id=app.current_workspace_id() and app.has_workspace_role(workspace_id,array['owner','editor','viewer']));
create policy scope_versions_write on app.context_pack_scope_versions for insert with check
  (workspace_id=app.current_workspace_id() and app.has_workspace_role(workspace_id,array['owner','editor']));
create policy scope_sources_read on app.context_pack_scope_sources for select using
  (workspace_id=app.current_workspace_id() and app.has_workspace_role(workspace_id,array['owner','editor','viewer']));
create policy scope_sources_write on app.context_pack_scope_sources for insert with check
  (workspace_id=app.current_workspace_id() and app.has_workspace_role(workspace_id,array['owner','editor']));
revoke all on app.context_pack_scope_versions,app.context_pack_scope_sources from public,anon,authenticated,service_role;
grant select,insert on app.context_pack_scope_versions,app.context_pack_scope_sources to ai_center_runtime;
grant usage on sequence app.context_pack_scope_versions_id_seq,app.context_pack_scope_sources_id_seq to ai_center_runtime;
create trigger scope_versions_append_only before update or delete on app.context_pack_scope_versions
  for each row execute function app.prevent_append_only_mutation();
create trigger scope_sources_append_only before update or delete on app.context_pack_scope_sources
  for each row execute function app.prevent_append_only_mutation();

create or replace function app.validate_scope_source_identity()
returns trigger language plpgsql security invoker set search_path='' as $$
declare actual_id uuid;
begin
  if new.source_kind='knowledge_entry_version' then
    select public_id into actual_id from app.knowledge_entry_versions where id=new.knowledge_version_id;
  else
    select public_id into actual_id from app.artifact_document_versions where id=new.artifact_version_id and status='validated';
  end if;
  if actual_id is null or actual_id<>new.source_public_id then
    raise exception 'scoped source identity does not match its immutable version' using errcode='23514';
  end if;
  return new;
end;
$$;
revoke all on function app.validate_scope_source_identity() from public,anon,authenticated,service_role;
create trigger scope_sources_validate_identity before insert on app.context_pack_scope_sources
  for each row execute function app.validate_scope_source_identity();

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
        ((s.source_kind='knowledge_entry_version' and (v.id is null or v.version_number<>k.latest_version or k.status<>'confirmed'))
          or (s.source_kind='artifact_document_version' and (a.id is null or a.status<>'validated' or exists(
            select 1 from app.artifact_document_versions newer where newer.document_id=a.document_id
              and newer.status='validated' and newer.version>a.version)))));
$$;
revoke all on function app.context_pack_scopes_current(bigint) from public,anon,authenticated,service_role;
grant execute on function app.context_pack_scopes_current(bigint) to ai_center_runtime;

create or replace function app.invalidate_scoped_knowledge()
returns trigger language plpgsql security invoker set search_path='' as $$
begin
  if row(new.latest_version,new.status) is not distinct from row(old.latest_version,old.status) then return new; end if;
  with changed as (
    update app.context_packs p set status='stale',invalidated_at=now(),stale_reason='included cross-scope knowledge was revised'
    where p.workspace_id=new.workspace_id and p.status='current' and exists(
      select 1 from app.context_pack_scope_sources s join app.knowledge_entry_versions v on v.id=s.knowledge_version_id
      where s.context_pack_id=p.id and s.decision='included' and v.knowledge_entry_id=new.id
        and (v.version_number<>new.latest_version or new.status<>'confirmed')) returning p.id
  ) update app.deliverables d set status='stale',stale_at=now()
    where d.source_context_pack_id in (select id from changed) and d.status in ('draft','committed');
  return new;
end;
$$;
revoke all on function app.invalidate_scoped_knowledge() from public,anon,authenticated,service_role;
create trigger knowledge_invalidate_scoped_sources after update on app.knowledge_entries
  for each row execute function app.invalidate_scoped_knowledge();

create or replace function app.invalidate_scoped_artifact()
returns trigger language plpgsql security invoker set search_path='' as $$
declare head_status text;
begin
  if new.current_version_id is not distinct from old.current_version_id then return new; end if;
  select status into head_status from app.artifact_document_versions where id=new.current_version_id;
  -- Unvalidated drafts never replace a previously confirmed source in retrieval.
  if head_status<>'validated' then return new; end if;
  update app.projects set graph_version=graph_version+1 where id=new.project_id;
  with changed as (
    update app.context_packs p set status='stale',invalidated_at=now(),stale_reason='included artifact version was superseded'
    where p.workspace_id=new.workspace_id and p.status='current' and exists(
      select 1 from app.context_pack_scope_sources s join app.artifact_document_versions v on v.id=s.artifact_version_id
      where s.context_pack_id=p.id and s.decision='included' and v.document_id=new.id and v.id<>new.current_version_id)
      returning p.id
  ) update app.deliverables d set status='stale',stale_at=now()
    where d.source_context_pack_id in(select id from changed) and d.status in ('draft','committed');
  return new;
end;
$$;
revoke all on function app.invalidate_scoped_artifact() from public,anon,authenticated,service_role;
create trigger artifact_invalidate_scoped_sources after update on app.artifact_documents
  for each row execute function app.invalidate_scoped_artifact();
