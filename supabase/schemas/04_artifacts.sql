-- Authored documents are distinct from immutable external execution artifacts.
-- A draft save appends a version; validation appends a validated version.
create table app.artifact_documents (
  id bigint generated always as identity primary key,
  public_id uuid not null default gen_random_uuid() unique,
  workspace_id bigint not null references app.workspaces(id),
  project_id bigint not null,
  artifact_type text not null check (artifact_type in ('kickoff','specification','product_tickets','technical_plan','technical_tickets')),
  current_version_id bigint,
  created_by_actor_id uuid not null,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  unique (id,workspace_id,project_id),
  foreign key (project_id,workspace_id) references app.projects(id,workspace_id)
);
create index artifact_documents_scope_updated_idx on app.artifact_documents(workspace_id,project_id,updated_at desc,id desc);
create index artifact_documents_project_idx on app.artifact_documents(project_id);
create index artifact_documents_current_version_idx on app.artifact_documents(current_version_id);

create table app.artifact_document_versions (
  id bigint generated always as identity primary key,
  public_id uuid not null default gen_random_uuid() unique,
  document_id bigint not null,
  workspace_id bigint not null,
  project_id bigint not null,
  version integer not null check (version > 0),
  title text not null check (length(btrim(title)) between 1 and 200),
  body_markdown text not null check (octet_length(body_markdown) <= 262144),
  structured_content jsonb not null default '{}'::jsonb check (jsonb_typeof(structured_content)='object' and octet_length(structured_content::text) <= 262144),
  status text not null check (status in ('draft','validated')),
  created_by_actor_id uuid not null,
  created_at timestamptz not null default now(),
  validated_at timestamptz,
  content_hash text not null check (content_hash ~ '^[0-9a-f]{64}$'),
  unique (document_id,version),
  unique (document_id,id),
  unique (id,workspace_id),
  unique (id,project_id),
  foreign key (document_id,workspace_id,project_id) references app.artifact_documents(id,workspace_id,project_id),
  check ((status='validated')=(validated_at is not null)),
  check (status='draft' or btrim(body_markdown)<>'' or structured_content<>'{}'::jsonb)
);
create index artifact_document_versions_workspace_idx on app.artifact_document_versions(workspace_id);
create index artifact_document_versions_project_idx on app.artifact_document_versions(project_id);
alter table app.artifact_documents add constraint artifact_documents_head_fkey
  foreign key (id,current_version_id) references app.artifact_document_versions(document_id,id)
  deferrable initially deferred;

create table app.artifact_version_sources (
  id bigint generated always as identity primary key,
  workspace_id bigint not null,
  version_id bigint not null,
  source_project_id bigint not null,
  source_kind text not null check (source_kind in ('knowledge','context_pack','deliverable','session','artifact_version')),
  knowledge_version_id bigint,
  context_pack_id bigint,
  deliverable_id bigint,
  session_id bigint,
  source_artifact_version_id bigint,
  source_public_id uuid not null,
  snapshot jsonb not null check (jsonb_typeof(snapshot)='object'),
  unique (version_id,source_kind,source_public_id),
  foreign key (version_id,workspace_id) references app.artifact_document_versions(id,workspace_id),
  foreign key (source_project_id,workspace_id) references app.projects(id,workspace_id),
  foreign key (knowledge_version_id,source_project_id) references app.knowledge_entry_versions(id,project_id),
  foreign key (context_pack_id,source_project_id) references app.context_packs(id,project_id),
  foreign key (deliverable_id,source_project_id) references app.deliverables(id,project_id),
  foreign key (session_id,source_project_id) references app.sessions(id,project_id),
  constraint artifact_sources_artifact_project_fkey foreign key (source_artifact_version_id,source_project_id) references app.artifact_document_versions(id,project_id),
  check (
    (source_kind='knowledge' and knowledge_version_id is not null and context_pack_id is null and deliverable_id is null and session_id is null and source_artifact_version_id is null)
    or (source_kind='context_pack' and knowledge_version_id is null and context_pack_id is not null and deliverable_id is null and session_id is null and source_artifact_version_id is null)
    or (source_kind='deliverable' and knowledge_version_id is null and context_pack_id is null and deliverable_id is not null and session_id is null and source_artifact_version_id is null)
    or (source_kind='session' and knowledge_version_id is null and context_pack_id is null and deliverable_id is null and session_id is not null and source_artifact_version_id is null)
    or (source_kind='artifact_version' and knowledge_version_id is null and context_pack_id is null and deliverable_id is null and session_id is null and source_artifact_version_id is not null)
  )
);
create index artifact_version_sources_workspace_idx on app.artifact_version_sources(workspace_id);
create index artifact_version_sources_project_idx on app.artifact_version_sources(source_project_id);
create index artifact_version_sources_knowledge_idx on app.artifact_version_sources(knowledge_version_id) where knowledge_version_id is not null;
create index artifact_version_sources_pack_idx on app.artifact_version_sources(context_pack_id) where context_pack_id is not null;
create index artifact_version_sources_deliverable_idx on app.artifact_version_sources(deliverable_id) where deliverable_id is not null;
create index artifact_version_sources_artifact_idx on app.artifact_version_sources(source_artifact_version_id) where source_artifact_version_id is not null;
create index artifact_version_sources_session_idx on app.artifact_version_sources(session_id) where session_id is not null;

create table app.artifact_destination_settings (
  id bigint generated always as identity primary key,
  workspace_id bigint not null references app.workspaces(id),
  project_id bigint,
  artifact_type text not null check (artifact_type in ('kickoff','specification','product_tickets','technical_plan','technical_tickets')),
  provider text not null check (provider in ('internal','notion','linear','github')),
  target_id text check (length(target_id) between 1 and 256),
  label text not null default '' check (length(label)<=120),
  enabled boolean not null default true,
  revision integer not null default 1 check (revision>0),
  updated_by_actor_id uuid not null,
  updated_at timestamptz not null default now(),
  foreign key (project_id,workspace_id) references app.projects(id,workspace_id),
  check ((provider='internal' and target_id is null) or (provider<>'internal' and target_id is not null))
);
create unique index artifact_destinations_company_unique on app.artifact_destination_settings(workspace_id,artifact_type) where project_id is null;
create unique index artifact_destinations_project_unique on app.artifact_destination_settings(workspace_id,project_id,artifact_type) where project_id is not null;
create index artifact_destinations_project_idx on app.artifact_destination_settings(project_id);

create or replace function app.artifact_head_is_current()
returns trigger language plpgsql set search_path='' as $$
declare head bigint; latest bigint; document bigint;
begin
  if tg_table_name='artifact_document_versions' then document=new.document_id; else document=new.id; end if;
  select current_version_id into head from app.artifact_documents where id=document;
  select id into latest from app.artifact_document_versions where document_id=document order by version desc limit 1;
  if head is null or latest is null or head<>latest then
    raise exception 'Artifact head must identify its latest immutable version' using errcode='23514';
  end if;
  return null;
end;
$$;
revoke execute on function app.artifact_head_is_current() from public,anon,authenticated;
create constraint trigger artifact_documents_head_required
after insert or update on app.artifact_documents deferrable initially deferred
for each row execute function app.artifact_head_is_current();
create constraint trigger artifact_versions_head_required
after insert on app.artifact_document_versions deferrable initially deferred
for each row execute function app.artifact_head_is_current();
create or replace function app.artifact_source_before_head()
returns trigger language plpgsql set search_path='' as $$
declare source_version integer; head_version integer;
begin
  select v.version,h.version into source_version,head_version
    from app.artifact_document_versions v join app.artifact_documents d on d.id=v.document_id
    left join app.artifact_document_versions h on h.id=d.current_version_id where v.id=new.version_id;
  if source_version is null or source_version<=coalesce(head_version,0) then
    raise exception 'Cannot append provenance to a committed artifact version' using errcode='23514';
  end if;
  return new;
end;
$$;
revoke execute on function app.artifact_source_before_head() from public,anon,authenticated;
create trigger artifact_sources_before_head before insert on app.artifact_version_sources
for each row execute function app.artifact_source_before_head();
create trigger artifact_document_versions_immutable before update or delete on app.artifact_document_versions
for each row execute function app.prevent_append_only_mutation();
create trigger artifact_version_sources_immutable before update or delete on app.artifact_version_sources
for each row execute function app.prevent_append_only_mutation();

do $$
declare relation_name text;
begin
  foreach relation_name in array array['artifact_documents','artifact_document_versions','artifact_version_sources','artifact_destination_settings'] loop
    execute format('alter table app.%I enable row level security',relation_name);
    execute format('alter table app.%I force row level security',relation_name);
    execute format('create policy artifact_member_select on app.%I for select using (workspace_id=(select app.current_workspace_id()) and app.has_workspace_role(workspace_id,array[''owner'',''editor'',''viewer'']::text[]))',relation_name);
    execute format('revoke all on app.%I from public,anon,authenticated,service_role',relation_name);
  end loop;
  foreach relation_name in array array['artifact_documents','artifact_document_versions','artifact_version_sources'] loop
    execute format('create policy artifact_editor_insert on app.%I for insert with check (workspace_id=(select app.current_workspace_id()) and app.has_workspace_role(workspace_id,array[''owner'',''editor'']::text[]))',relation_name);
  end loop;
end;
$$;
create policy artifact_editor_update on app.artifact_documents for update
using (workspace_id=(select app.current_workspace_id()) and app.has_workspace_role(workspace_id,array['owner','editor']::text[]))
with check (workspace_id=(select app.current_workspace_id()) and app.has_workspace_role(workspace_id,array['owner','editor']::text[]));
create policy artifact_destination_write on app.artifact_destination_settings for all
using (workspace_id=(select app.current_workspace_id()) and app.has_workspace_role(workspace_id,case when project_id is null then array['owner'] else array['owner','editor'] end::text[]))
with check (workspace_id=(select app.current_workspace_id()) and app.has_workspace_role(workspace_id,case when project_id is null then array['owner'] else array['owner','editor'] end::text[]));
grant select,insert on app.artifact_documents,app.artifact_document_versions,app.artifact_version_sources to ai_center_runtime;
grant update(current_version_id,updated_at) on app.artifact_documents to ai_center_runtime;
grant select,insert,update on app.artifact_destination_settings to ai_center_runtime;
grant usage on sequence app.artifact_documents_id_seq,app.artifact_document_versions_id_seq,app.artifact_version_sources_id_seq,app.artifact_destination_settings_id_seq to ai_center_runtime;
