-- Generated from schemas 01/02/03/04/99 on the checkout-isolated Supabase stack.
-- Reviewed additions restore data backfill, FORCE RLS and ACLs omitted by migra.

alter table "app"."agent_profiles" drop constraint "agent_profiles_scope_kind_valid";


  create table "app"."artifact_destination_settings" (
    "id" bigint generated always as identity not null,
    "workspace_id" bigint not null,
    "project_id" bigint,
    "artifact_type" text not null,
    "provider" text not null,
    "target_id" text,
    "label" text not null default ''::text,
    "enabled" boolean not null default true,
    "revision" integer not null default 1,
    "updated_by_actor_id" uuid not null,
    "updated_at" timestamp with time zone not null default now()
      );


alter table "app"."artifact_destination_settings" enable row level security;


  create table "app"."artifact_document_versions" (
    "id" bigint generated always as identity not null,
    "public_id" uuid not null default gen_random_uuid(),
    "document_id" bigint not null,
    "workspace_id" bigint not null,
    "project_id" bigint not null,
    "version" integer not null,
    "title" text not null,
    "body_markdown" text not null,
    "structured_content" jsonb not null default '{}'::jsonb,
    "status" text not null,
    "created_by_actor_id" uuid not null,
    "created_at" timestamp with time zone not null default now(),
    "validated_at" timestamp with time zone,
    "content_hash" text not null
      );


alter table "app"."artifact_document_versions" enable row level security;


  create table "app"."artifact_documents" (
    "id" bigint generated always as identity not null,
    "public_id" uuid not null default gen_random_uuid(),
    "workspace_id" bigint not null,
    "project_id" bigint not null,
    "artifact_type" text not null,
    "current_version_id" bigint,
    "created_by_actor_id" uuid not null,
    "created_at" timestamp with time zone not null default now(),
    "updated_at" timestamp with time zone not null default now()
      );


alter table "app"."artifact_documents" enable row level security;


  create table "app"."artifact_version_sources" (
    "id" bigint generated always as identity not null,
    "workspace_id" bigint not null,
    "version_id" bigint not null,
    "source_project_id" bigint not null,
    "source_kind" text not null,
    "knowledge_version_id" bigint,
    "context_pack_id" bigint,
    "deliverable_id" bigint,
    "session_id" bigint,
    "source_public_id" uuid not null,
    "snapshot" jsonb not null
      );


alter table "app"."artifact_version_sources" enable row level security;

-- Backfill before enforcing NOT NULL; the declarative diff omits data changes.
alter table "app"."edges" add column "target_project_id" bigint;
update app.edges set target_project_id=project_id where target_project_id is null;
alter table app.edges alter column target_project_id set not null;

alter table "app"."projects" add column "scope_kind" text not null default 'project'::text;

alter table "app"."workspaces" add column "bootstrap_request_hash" text;

alter table "app"."workspaces" add column "description" text not null default ''::text;

CREATE UNIQUE INDEX artifact_destination_settings_pkey ON app.artifact_destination_settings USING btree (id);

CREATE UNIQUE INDEX artifact_destinations_company_unique ON app.artifact_destination_settings USING btree (workspace_id, artifact_type) WHERE (project_id IS NULL);

CREATE INDEX artifact_destinations_project_idx ON app.artifact_destination_settings USING btree (project_id);

CREATE UNIQUE INDEX artifact_destinations_project_unique ON app.artifact_destination_settings USING btree (workspace_id, project_id, artifact_type) WHERE (project_id IS NOT NULL);

CREATE UNIQUE INDEX artifact_document_versions_document_id_id_key ON app.artifact_document_versions USING btree (document_id, id);

CREATE UNIQUE INDEX artifact_document_versions_document_id_version_key ON app.artifact_document_versions USING btree (document_id, version);

CREATE UNIQUE INDEX artifact_document_versions_id_project_id_key ON app.artifact_document_versions USING btree (id, project_id);

CREATE UNIQUE INDEX artifact_document_versions_id_workspace_id_key ON app.artifact_document_versions USING btree (id, workspace_id);

CREATE UNIQUE INDEX artifact_document_versions_pkey ON app.artifact_document_versions USING btree (id);

CREATE INDEX artifact_document_versions_project_idx ON app.artifact_document_versions USING btree (project_id);

CREATE UNIQUE INDEX artifact_document_versions_public_id_key ON app.artifact_document_versions USING btree (public_id);

CREATE INDEX artifact_document_versions_workspace_idx ON app.artifact_document_versions USING btree (workspace_id);

CREATE INDEX artifact_documents_current_version_idx ON app.artifact_documents USING btree (current_version_id);

CREATE UNIQUE INDEX artifact_documents_id_workspace_id_project_id_key ON app.artifact_documents USING btree (id, workspace_id, project_id);

CREATE UNIQUE INDEX artifact_documents_pkey ON app.artifact_documents USING btree (id);

CREATE INDEX artifact_documents_project_idx ON app.artifact_documents USING btree (project_id);

CREATE UNIQUE INDEX artifact_documents_public_id_key ON app.artifact_documents USING btree (public_id);

CREATE INDEX artifact_documents_scope_updated_idx ON app.artifact_documents USING btree (workspace_id, project_id, updated_at DESC, id DESC);

CREATE INDEX artifact_version_sources_deliverable_idx ON app.artifact_version_sources USING btree (deliverable_id) WHERE (deliverable_id IS NOT NULL);

CREATE INDEX artifact_version_sources_knowledge_idx ON app.artifact_version_sources USING btree (knowledge_version_id) WHERE (knowledge_version_id IS NOT NULL);

CREATE INDEX artifact_version_sources_pack_idx ON app.artifact_version_sources USING btree (context_pack_id) WHERE (context_pack_id IS NOT NULL);

CREATE UNIQUE INDEX artifact_version_sources_pkey ON app.artifact_version_sources USING btree (id);

CREATE INDEX artifact_version_sources_project_idx ON app.artifact_version_sources USING btree (source_project_id);

CREATE INDEX artifact_version_sources_session_idx ON app.artifact_version_sources USING btree (session_id) WHERE (session_id IS NOT NULL);

CREATE UNIQUE INDEX artifact_version_sources_version_id_source_kind_source_publ_key ON app.artifact_version_sources USING btree (version_id, source_kind, source_public_id);

CREATE INDEX artifact_version_sources_workspace_idx ON app.artifact_version_sources USING btree (workspace_id);

CREATE INDEX edges_target_project_target_idx ON app.edges USING btree (target_project_id, target_public_id);

CREATE UNIQUE INDEX projects_one_company_scope ON app.projects USING btree (workspace_id) WHERE (scope_kind = 'company'::text);

CREATE INDEX workspaces_owner_actor_idx ON app.workspaces USING btree (owner_actor_id);

alter table "app"."artifact_destination_settings" add constraint "artifact_destination_settings_pkey" PRIMARY KEY using index "artifact_destination_settings_pkey";

alter table "app"."artifact_document_versions" add constraint "artifact_document_versions_pkey" PRIMARY KEY using index "artifact_document_versions_pkey";

alter table "app"."artifact_documents" add constraint "artifact_documents_pkey" PRIMARY KEY using index "artifact_documents_pkey";

alter table "app"."artifact_version_sources" add constraint "artifact_version_sources_pkey" PRIMARY KEY using index "artifact_version_sources_pkey";

alter table "app"."artifact_destination_settings" add constraint "artifact_destination_settings_artifact_type_check" CHECK ((artifact_type = ANY (ARRAY['kickoff'::text, 'specification'::text, 'product_tickets'::text, 'technical_plan'::text, 'technical_tickets'::text]))) not valid;

alter table "app"."artifact_destination_settings" validate constraint "artifact_destination_settings_artifact_type_check";

alter table "app"."artifact_destination_settings" add constraint "artifact_destination_settings_check" CHECK ((((provider = 'internal'::text) AND (target_id IS NULL)) OR ((provider <> 'internal'::text) AND (target_id IS NOT NULL)))) not valid;

alter table "app"."artifact_destination_settings" validate constraint "artifact_destination_settings_check";

alter table "app"."artifact_destination_settings" add constraint "artifact_destination_settings_label_check" CHECK ((length(label) <= 120)) not valid;

alter table "app"."artifact_destination_settings" validate constraint "artifact_destination_settings_label_check";

alter table "app"."artifact_destination_settings" add constraint "artifact_destination_settings_project_id_workspace_id_fkey" FOREIGN KEY (project_id, workspace_id) REFERENCES app.projects(id, workspace_id) not valid;

alter table "app"."artifact_destination_settings" validate constraint "artifact_destination_settings_project_id_workspace_id_fkey";

alter table "app"."artifact_destination_settings" add constraint "artifact_destination_settings_provider_check" CHECK ((provider = ANY (ARRAY['internal'::text, 'notion'::text, 'linear'::text, 'github'::text]))) not valid;

alter table "app"."artifact_destination_settings" validate constraint "artifact_destination_settings_provider_check";

alter table "app"."artifact_destination_settings" add constraint "artifact_destination_settings_revision_check" CHECK ((revision > 0)) not valid;

alter table "app"."artifact_destination_settings" validate constraint "artifact_destination_settings_revision_check";

alter table "app"."artifact_destination_settings" add constraint "artifact_destination_settings_target_id_check" CHECK (((length(target_id) >= 1) AND (length(target_id) <= 256))) not valid;

alter table "app"."artifact_destination_settings" validate constraint "artifact_destination_settings_target_id_check";

alter table "app"."artifact_destination_settings" add constraint "artifact_destination_settings_workspace_id_fkey" FOREIGN KEY (workspace_id) REFERENCES app.workspaces(id) not valid;

alter table "app"."artifact_destination_settings" validate constraint "artifact_destination_settings_workspace_id_fkey";

alter table "app"."artifact_document_versions" add constraint "artifact_document_versions_body_markdown_check" CHECK ((octet_length(body_markdown) <= 262144)) not valid;

alter table "app"."artifact_document_versions" validate constraint "artifact_document_versions_body_markdown_check";

alter table "app"."artifact_document_versions" add constraint "artifact_document_versions_check" CHECK (((status = 'validated'::text) = (validated_at IS NOT NULL))) not valid;

alter table "app"."artifact_document_versions" validate constraint "artifact_document_versions_check";

alter table "app"."artifact_document_versions" add constraint "artifact_document_versions_check1" CHECK (((status = 'draft'::text) OR (btrim(body_markdown) <> ''::text) OR (structured_content <> '{}'::jsonb))) not valid;

alter table "app"."artifact_document_versions" validate constraint "artifact_document_versions_check1";

alter table "app"."artifact_document_versions" add constraint "artifact_document_versions_content_hash_check" CHECK ((content_hash ~ '^[0-9a-f]{64}$'::text)) not valid;

alter table "app"."artifact_document_versions" validate constraint "artifact_document_versions_content_hash_check";

alter table "app"."artifact_document_versions" add constraint "artifact_document_versions_document_id_id_key" UNIQUE using index "artifact_document_versions_document_id_id_key";

alter table "app"."artifact_document_versions" add constraint "artifact_document_versions_document_id_version_key" UNIQUE using index "artifact_document_versions_document_id_version_key";

alter table "app"."artifact_document_versions" add constraint "artifact_document_versions_document_id_workspace_id_projec_fkey" FOREIGN KEY (document_id, workspace_id, project_id) REFERENCES app.artifact_documents(id, workspace_id, project_id) not valid;

alter table "app"."artifact_document_versions" validate constraint "artifact_document_versions_document_id_workspace_id_projec_fkey";

alter table "app"."artifact_document_versions" add constraint "artifact_document_versions_id_project_id_key" UNIQUE using index "artifact_document_versions_id_project_id_key";

alter table "app"."artifact_document_versions" add constraint "artifact_document_versions_id_workspace_id_key" UNIQUE using index "artifact_document_versions_id_workspace_id_key";

alter table "app"."artifact_document_versions" add constraint "artifact_document_versions_public_id_key" UNIQUE using index "artifact_document_versions_public_id_key";

alter table "app"."artifact_document_versions" add constraint "artifact_document_versions_status_check" CHECK ((status = ANY (ARRAY['draft'::text, 'validated'::text]))) not valid;

alter table "app"."artifact_document_versions" validate constraint "artifact_document_versions_status_check";

alter table "app"."artifact_document_versions" add constraint "artifact_document_versions_structured_content_check" CHECK (((jsonb_typeof(structured_content) = 'object'::text) AND (octet_length((structured_content)::text) <= 262144))) not valid;

alter table "app"."artifact_document_versions" validate constraint "artifact_document_versions_structured_content_check";

alter table "app"."artifact_document_versions" add constraint "artifact_document_versions_title_check" CHECK (((length(btrim(title)) >= 1) AND (length(btrim(title)) <= 200))) not valid;

alter table "app"."artifact_document_versions" validate constraint "artifact_document_versions_title_check";

alter table "app"."artifact_document_versions" add constraint "artifact_document_versions_version_check" CHECK ((version > 0)) not valid;

alter table "app"."artifact_document_versions" validate constraint "artifact_document_versions_version_check";

alter table "app"."artifact_documents" add constraint "artifact_documents_artifact_type_check" CHECK ((artifact_type = ANY (ARRAY['kickoff'::text, 'specification'::text, 'product_tickets'::text, 'technical_plan'::text, 'technical_tickets'::text]))) not valid;

alter table "app"."artifact_documents" validate constraint "artifact_documents_artifact_type_check";

alter table "app"."artifact_documents" add constraint "artifact_documents_head_fkey" FOREIGN KEY (id, current_version_id) REFERENCES app.artifact_document_versions(document_id, id) DEFERRABLE INITIALLY DEFERRED not valid;

alter table "app"."artifact_documents" validate constraint "artifact_documents_head_fkey";

alter table "app"."artifact_documents" add constraint "artifact_documents_id_workspace_id_project_id_key" UNIQUE using index "artifact_documents_id_workspace_id_project_id_key";

alter table "app"."artifact_documents" add constraint "artifact_documents_project_id_workspace_id_fkey" FOREIGN KEY (project_id, workspace_id) REFERENCES app.projects(id, workspace_id) not valid;

alter table "app"."artifact_documents" validate constraint "artifact_documents_project_id_workspace_id_fkey";

alter table "app"."artifact_documents" add constraint "artifact_documents_public_id_key" UNIQUE using index "artifact_documents_public_id_key";

alter table "app"."artifact_documents" add constraint "artifact_documents_workspace_id_fkey" FOREIGN KEY (workspace_id) REFERENCES app.workspaces(id) not valid;

alter table "app"."artifact_documents" validate constraint "artifact_documents_workspace_id_fkey";

alter table "app"."artifact_version_sources" add constraint "artifact_version_sources_check" CHECK ((((source_kind = 'knowledge'::text) AND (knowledge_version_id IS NOT NULL) AND (context_pack_id IS NULL) AND (deliverable_id IS NULL) AND (session_id IS NULL)) OR ((source_kind = 'context_pack'::text) AND (knowledge_version_id IS NULL) AND (context_pack_id IS NOT NULL) AND (deliverable_id IS NULL) AND (session_id IS NULL)) OR ((source_kind = 'deliverable'::text) AND (knowledge_version_id IS NULL) AND (context_pack_id IS NULL) AND (deliverable_id IS NOT NULL) AND (session_id IS NULL)) OR ((source_kind = 'session'::text) AND (knowledge_version_id IS NULL) AND (context_pack_id IS NULL) AND (deliverable_id IS NULL) AND (session_id IS NOT NULL)))) not valid;

alter table "app"."artifact_version_sources" validate constraint "artifact_version_sources_check";

alter table "app"."artifact_version_sources" add constraint "artifact_version_sources_context_pack_id_source_project_id_fkey" FOREIGN KEY (context_pack_id, source_project_id) REFERENCES app.context_packs(id, project_id) not valid;

alter table "app"."artifact_version_sources" validate constraint "artifact_version_sources_context_pack_id_source_project_id_fkey";

alter table "app"."artifact_version_sources" add constraint "artifact_version_sources_deliverable_id_source_project_id_fkey" FOREIGN KEY (deliverable_id, source_project_id) REFERENCES app.deliverables(id, project_id) not valid;

alter table "app"."artifact_version_sources" validate constraint "artifact_version_sources_deliverable_id_source_project_id_fkey";

alter table "app"."artifact_version_sources" add constraint "artifact_version_sources_knowledge_version_id_source_proje_fkey" FOREIGN KEY (knowledge_version_id, source_project_id) REFERENCES app.knowledge_entry_versions(id, project_id) not valid;

alter table "app"."artifact_version_sources" validate constraint "artifact_version_sources_knowledge_version_id_source_proje_fkey";

alter table "app"."artifact_version_sources" add constraint "artifact_version_sources_session_id_source_project_id_fkey" FOREIGN KEY (session_id, source_project_id) REFERENCES app.sessions(id, project_id) not valid;

alter table "app"."artifact_version_sources" validate constraint "artifact_version_sources_session_id_source_project_id_fkey";

alter table "app"."artifact_version_sources" add constraint "artifact_version_sources_snapshot_check" CHECK ((jsonb_typeof(snapshot) = 'object'::text)) not valid;

alter table "app"."artifact_version_sources" validate constraint "artifact_version_sources_snapshot_check";

alter table "app"."artifact_version_sources" add constraint "artifact_version_sources_source_kind_check" CHECK ((source_kind = ANY (ARRAY['knowledge'::text, 'context_pack'::text, 'deliverable'::text, 'session'::text]))) not valid;

alter table "app"."artifact_version_sources" validate constraint "artifact_version_sources_source_kind_check";

alter table "app"."artifact_version_sources" add constraint "artifact_version_sources_source_project_id_workspace_id_fkey" FOREIGN KEY (source_project_id, workspace_id) REFERENCES app.projects(id, workspace_id) not valid;

alter table "app"."artifact_version_sources" validate constraint "artifact_version_sources_source_project_id_workspace_id_fkey";

alter table "app"."artifact_version_sources" add constraint "artifact_version_sources_version_id_source_kind_source_publ_key" UNIQUE using index "artifact_version_sources_version_id_source_kind_source_publ_key";

alter table "app"."artifact_version_sources" add constraint "artifact_version_sources_version_id_workspace_id_fkey" FOREIGN KEY (version_id, workspace_id) REFERENCES app.artifact_document_versions(id, workspace_id) not valid;

alter table "app"."artifact_version_sources" validate constraint "artifact_version_sources_version_id_workspace_id_fkey";

alter table "app"."edges" add constraint "edges_target_project_workspace_fkey" FOREIGN KEY (target_project_id, workspace_id) REFERENCES app.projects(id, workspace_id) not valid;

alter table "app"."edges" validate constraint "edges_target_project_workspace_fkey";

alter table "app"."projects" add constraint "projects_scope_kind_valid" CHECK ((scope_kind = ANY (ARRAY['company'::text, 'project'::text]))) not valid;

alter table "app"."projects" validate constraint "projects_scope_kind_valid";

alter table "app"."workspaces" add constraint "workspaces_bootstrap_hash_valid" CHECK (((bootstrap_request_hash IS NULL) OR (bootstrap_request_hash ~ '^[0-9a-f]{64}$'::text))) not valid;

alter table "app"."workspaces" validate constraint "workspaces_bootstrap_hash_valid";

alter table "app"."workspaces" add constraint "workspaces_description_length" CHECK ((length(description) <= 4000)) not valid;

alter table "app"."workspaces" validate constraint "workspaces_description_length";

alter table "app"."agent_profiles" add constraint "agent_profiles_scope_kind_valid" CHECK ((scope_kind = ANY (ARRAY['general'::text, 'product'::text, 'sales'::text, 'tech'::text, 'steward'::text]))) not valid;

alter table "app"."agent_profiles" validate constraint "agent_profiles_scope_kind_valid";

set check_function_bodies = off;

CREATE OR REPLACE FUNCTION app.artifact_head_is_current()
 RETURNS trigger
 LANGUAGE plpgsql
 SET search_path TO ''
AS $function$
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
$function$
;

CREATE OR REPLACE FUNCTION app.artifact_source_before_head()
 RETURNS trigger
 LANGUAGE plpgsql
 SET search_path TO ''
AS $function$
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
$function$
;

CREATE OR REPLACE FUNCTION app.create_company_workspace(requested_public_id uuid, requested_name text, requested_description text, requested_hash text)
 RETURNS uuid
 LANGUAGE plpgsql
 SECURITY DEFINER
 SET search_path TO ''
AS $function$
declare actor uuid := app.current_actor_id(); existing app.workspaces%rowtype;
  workspace_id_value bigint; project_public_id_value uuid; template_id_value bigint;
begin
  if actor is null or actor = '00000000-0000-0000-0000-000000000000'::uuid then
    raise exception 'authenticated actor required' using errcode = '42501';
  end if;
  if requested_public_id is null or requested_public_id = '00000000-0000-0000-0000-000000000000'::uuid
    or requested_name is null or length(btrim(requested_name)) not between 1 and 120
    or requested_description is null or length(requested_description) > 4000
    or requested_hash is null or requested_hash !~ '^[0-9a-f]{64}$' then
    raise exception 'invalid company input' using errcode = '22023';
  end if;
  perform pg_catalog.pg_advisory_xact_lock(pg_catalog.hashtextextended(actor::text, 94731));
  select * into existing from app.workspaces where public_id = requested_public_id;
  if existing.id is not null then
    if existing.owner_actor_id <> actor or existing.bootstrap_request_hash is distinct from requested_hash then
      raise exception 'company identity is already used' using errcode = '23505';
    end if;
    -- A revoked membership must never be recreated by replaying bootstrap.
    if not exists(select 1 from app.workspace_members where workspace_id=existing.id
      and actor_id=actor and invitation_status='accepted') then
      raise exception 'company access has been revoked' using errcode = '42501';
    end if;
    return existing.public_id;
  end if;
  if (select count(*) from app.workspaces where owner_actor_id=actor) >= 10 then
    raise exception 'company creation limit reached' using errcode = '54000';
  end if;
  perform app.ensure_company_catalog();
  select id into strict template_id_value from app.project_templates where template_key='software-product-delivery';
  insert into app.workspaces(public_id, owner_actor_id, name, description, bootstrap_request_hash)
    values(requested_public_id, actor, btrim(requested_name), requested_description, requested_hash)
    returning id into workspace_id_value;
  insert into app.workspace_members(workspace_id, actor_id, role, invitation_status, invited_by_actor_id, accepted_at)
    values(workspace_id_value, actor, 'owner', 'accepted', actor, now());
  perform pg_catalog.set_config('app.current_workspace_id', workspace_id_value::text, true);
  perform pg_catalog.set_config('app.current_workspace_role', 'owner', true);
  insert into app.projects(workspace_id,template_id,name,objective,scope_kind,created_by_actor_id)
    values(workspace_id_value,template_id_value,'Contexte société',requested_description,'company',actor)
    returning public_id into project_public_id_value;
  perform app.ensure_scope_agents(project_public_id_value);
  insert into app.audit_events(workspace_id,actor_id,action,object_kind,object_public_id,after_state)
    values(workspace_id_value,actor,'company.created','workspace',requested_public_id,
      pg_catalog.jsonb_build_object('company_scope_public_id',project_public_id_value));
  return requested_public_id;
end;
$function$
;

CREATE OR REPLACE FUNCTION app.ensure_company_catalog()
 RETURNS void
 LANGUAGE plpgsql
 SECURITY DEFINER
 SET search_path TO ''
AS $function$
declare template_id_value bigint;
begin
  insert into app.project_templates(template_key, name, definition)
  values ('software-product-delivery', 'Software Product Delivery', '{}'::jsonb)
  on conflict (template_key) do nothing;
  select id into strict template_id_value from app.project_templates
    where template_key = 'software-product-delivery';
  insert into app.agent_profiles(template_id, profile_key, name, scope_kind, instructions, retrieval_policy)
  select template_id_value, profile_key, name, scope_kind, instructions, policy
  from (values
    ('company-general', 'Assistant général', 'general',
     'Aide à comprendre, synthétiser et relier le contexte de la société. Cite les sources reçues et propose les changements sans les confirmer.',
     '{"role":"general","context_pack_required":false}'::jsonb),
    ('product-agent', 'Product Manager', 'product',
     'Clarifie les besoins, règles métier, exigences et critères. Prépare des spécifications et tickets produit sourcés sans confirmer les mutations.',
     '{"role":"product_manager","context_pack_required":false}'::jsonb),
    ('sales-agent', 'Commercial', 'sales',
     'Analyse le besoin client et prépare le cadrage commercial à partir des sources autorisées. Ne promets pas de fonctionnalité sans preuve et propose les décisions pour validation.',
     '{"role":"sales","context_pack_required":false}'::jsonb),
    ('tech-agent', 'Lead technique', 'tech',
     'Transforme le ContextPack en architecture, risques, plan et tickets techniques sourcés. Ne modifie aucun dépôt et ne confirme aucune connaissance.',
     '{"role":"tech_lead","context_pack_required":true}'::jsonb),
    ('developer-agent', 'Développeur conseil', 'tech',
     'Analyse le ContextPack, explique le code observé et prépare des tickets techniques et critères de vérification. Ne code pas et ne lance aucune exécution.',
     '{"role":"developer","context_pack_required":true}'::jsonb)
  ) as catalog(profile_key, name, scope_kind, instructions, policy)
  on conflict (template_id, profile_key) do nothing;
  insert into app.deliverable_contracts(template_id,contract_key,name,required_sections,accepted_evidence_types,completion_rules,human_validation_required)
  values
    (template_id_value,'feature-brief','Feature Brief',array['objective','business_rules','requirements','acceptance_criteria','open_questions'],
      array['knowledge_entry','human_validation'],'{"gate":"product-ready","all_sections_required":true}'::jsonb,true),
    (template_id_value,'technical-delivery-plan','Technical Delivery Plan',array['architecture','delivery_slices','risks','validation','coverage'],
      array['deliverable_section','analysis','human_validation'],'{"all_requirements_have_coverage_state":true}'::jsonb,true)
  on conflict (template_id,contract_key) do nothing;
end;
$function$
;

CREATE OR REPLACE FUNCTION app.ensure_scope_agents(requested_project_public_id uuid)
 RETURNS void
 LANGUAGE plpgsql
 SECURITY DEFINER
 SET search_path TO ''
AS $function$
declare scope_row app.projects%rowtype;
begin
  select * into scope_row from app.projects
    where public_id = requested_project_public_id
      and workspace_id = app.current_workspace_id();
  if scope_row.id is null or not app.has_workspace_role(scope_row.workspace_id, array['owner','editor']) then
    raise exception 'scope is not authorized' using errcode = '42501';
  end if;
  perform app.ensure_company_catalog();
  insert into app.context_nodes(workspace_id, project_id, agent_profile_id, node_key, title, description,
    preferred_entry_types, preferred_deliverable_types)
  select scope_row.workspace_id, scope_row.id, profile.id, node.node_key, node.title, node.description,
    array['decision','business_rule','requirement','constraint','open_question'], node.deliverables
  from (values
    ('general', 'company-general', 'Général', 'Contexte transversal, compréhension et synthèse.', array['kickoff']::text[]),
    ('product', 'product-agent', 'Produit', 'Besoins, règles métier et critères d’acceptation.', array['feature-brief','product-tickets']::text[]),
    ('sales', 'sales-agent', 'Commercial', 'Besoin client, cadrage et engagements sourcés.', array['kickoff','sales-brief']::text[]),
    ('tech', 'tech-agent', 'Lead technique', 'Architecture et plan technique à partir d’un ContextPack.', array['technical-delivery-plan']::text[]),
    ('dev', 'developer-agent', 'Développement', 'Compréhension du code et tickets techniques, sans exécution.', array['technical-tickets']::text[])
  ) as node(node_key, profile_key, title, description, deliverables)
  join app.agent_profiles profile on profile.template_id = scope_row.template_id and profile.profile_key = node.profile_key
  on conflict (project_id, node_key) do nothing;
end;
$function$
;

CREATE OR REPLACE FUNCTION app.graph_endpoint_exists(endpoint_kind text, endpoint_id uuid, scope_id bigint, tenant_id bigint)
 RETURNS boolean
 LANGUAGE plpgsql
 STABLE
 SET search_path TO ''
AS $function$
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
    when 'task' then return exists(select 1 from app.tasks where public_id=endpoint_id and project_id=scope_id and workspace_id=tenant_id);
    when 'execution' then return exists(select 1 from app.executions where public_id=endpoint_id and project_id=scope_id and workspace_id=tenant_id);
    when 'insight' then return exists(select 1 from app.insights where public_id=endpoint_id and project_id=scope_id and workspace_id=tenant_id);
    else return false;
  end case;
end;
$function$
;

CREATE OR REPLACE FUNCTION app.preserve_company_owner()
 RETURNS trigger
 LANGUAGE plpgsql
 SET search_path TO ''
AS $function$
begin
  if old.role='owner' and old.invitation_status='accepted'
    and (new.role<>'owner' or new.invitation_status<>'accepted') then
    perform id from app.workspaces where id=old.workspace_id for update;
    if not exists(select 1 from app.workspace_members where workspace_id=old.workspace_id
      and id<>old.id and role='owner' and invitation_status='accepted') then
      raise exception 'the last accepted company owner cannot be removed' using errcode='23514';
    end if;
  end if;
  return new;
end;
$function$
;

CREATE OR REPLACE FUNCTION app.validate_graph_edge_scope()
 RETURNS trigger
 LANGUAGE plpgsql
 SET search_path TO ''
AS $function$
begin
  new.target_project_id := coalesce(new.target_project_id,new.project_id);
  if not app.graph_endpoint_exists(new.source_kind,new.source_public_id,new.project_id,new.workspace_id)
    or not app.graph_endpoint_exists(new.target_kind,new.target_public_id,new.target_project_id,new.workspace_id) then
    raise exception 'graph endpoint outside declared scope' using errcode = '23503';
  end if;
  return new;
end;
$function$
;

grant insert on table "app"."artifact_destination_settings" to "ai_center_runtime";

grant select on table "app"."artifact_destination_settings" to "ai_center_runtime";

grant update on table "app"."artifact_destination_settings" to "ai_center_runtime";

grant insert on table "app"."artifact_document_versions" to "ai_center_runtime";

grant select on table "app"."artifact_document_versions" to "ai_center_runtime";

grant insert on table "app"."artifact_documents" to "ai_center_runtime";

grant select on table "app"."artifact_documents" to "ai_center_runtime";

grant insert on table "app"."artifact_version_sources" to "ai_center_runtime";

grant select on table "app"."artifact_version_sources" to "ai_center_runtime";


  create policy "artifact_destination_write"
  on "app"."artifact_destination_settings"
  as permissive
  for all
  to public
using (((workspace_id = ( SELECT app.current_workspace_id() AS current_workspace_id)) AND app.has_workspace_role(workspace_id,
CASE
    WHEN (project_id IS NULL) THEN ARRAY['owner'::text]
    ELSE ARRAY['owner'::text, 'editor'::text]
END)))
with check (((workspace_id = ( SELECT app.current_workspace_id() AS current_workspace_id)) AND app.has_workspace_role(workspace_id,
CASE
    WHEN (project_id IS NULL) THEN ARRAY['owner'::text]
    ELSE ARRAY['owner'::text, 'editor'::text]
END)));



  create policy "artifact_member_select"
  on "app"."artifact_destination_settings"
  as permissive
  for select
  to public
using (((workspace_id = ( SELECT app.current_workspace_id() AS current_workspace_id)) AND app.has_workspace_role(workspace_id, ARRAY['owner'::text, 'editor'::text, 'viewer'::text])));



  create policy "artifact_editor_insert"
  on "app"."artifact_document_versions"
  as permissive
  for insert
  to public
with check (((workspace_id = ( SELECT app.current_workspace_id() AS current_workspace_id)) AND app.has_workspace_role(workspace_id, ARRAY['owner'::text, 'editor'::text])));



  create policy "artifact_member_select"
  on "app"."artifact_document_versions"
  as permissive
  for select
  to public
using (((workspace_id = ( SELECT app.current_workspace_id() AS current_workspace_id)) AND app.has_workspace_role(workspace_id, ARRAY['owner'::text, 'editor'::text, 'viewer'::text])));



  create policy "artifact_editor_insert"
  on "app"."artifact_documents"
  as permissive
  for insert
  to public
with check (((workspace_id = ( SELECT app.current_workspace_id() AS current_workspace_id)) AND app.has_workspace_role(workspace_id, ARRAY['owner'::text, 'editor'::text])));



  create policy "artifact_editor_update"
  on "app"."artifact_documents"
  as permissive
  for update
  to public
using (((workspace_id = ( SELECT app.current_workspace_id() AS current_workspace_id)) AND app.has_workspace_role(workspace_id, ARRAY['owner'::text, 'editor'::text])))
with check (((workspace_id = ( SELECT app.current_workspace_id() AS current_workspace_id)) AND app.has_workspace_role(workspace_id, ARRAY['owner'::text, 'editor'::text])));



  create policy "artifact_member_select"
  on "app"."artifact_documents"
  as permissive
  for select
  to public
using (((workspace_id = ( SELECT app.current_workspace_id() AS current_workspace_id)) AND app.has_workspace_role(workspace_id, ARRAY['owner'::text, 'editor'::text, 'viewer'::text])));



  create policy "artifact_editor_insert"
  on "app"."artifact_version_sources"
  as permissive
  for insert
  to public
with check (((workspace_id = ( SELECT app.current_workspace_id() AS current_workspace_id)) AND app.has_workspace_role(workspace_id, ARRAY['owner'::text, 'editor'::text])));



  create policy "artifact_member_select"
  on "app"."artifact_version_sources"
  as permissive
  for select
  to public
using (((workspace_id = ( SELECT app.current_workspace_id() AS current_workspace_id)) AND app.has_workspace_role(workspace_id, ARRAY['owner'::text, 'editor'::text, 'viewer'::text])));


CREATE TRIGGER artifact_document_versions_immutable BEFORE DELETE OR UPDATE ON app.artifact_document_versions FOR EACH ROW EXECUTE FUNCTION app.prevent_append_only_mutation();

CREATE CONSTRAINT TRIGGER artifact_versions_head_required AFTER INSERT ON app.artifact_document_versions DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION app.artifact_head_is_current();

CREATE CONSTRAINT TRIGGER artifact_documents_head_required AFTER INSERT OR UPDATE ON app.artifact_documents DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION app.artifact_head_is_current();

CREATE TRIGGER artifact_sources_before_head BEFORE INSERT ON app.artifact_version_sources FOR EACH ROW EXECUTE FUNCTION app.artifact_source_before_head();

CREATE TRIGGER artifact_version_sources_immutable BEFORE DELETE OR UPDATE ON app.artifact_version_sources FOR EACH ROW EXECUTE FUNCTION app.prevent_append_only_mutation();

CREATE TRIGGER edges_validate_scope BEFORE INSERT OR UPDATE ON app.edges FOR EACH ROW EXECUTE FUNCTION app.validate_graph_edge_scope();

CREATE TRIGGER workspace_members_preserve_owner BEFORE UPDATE ON app.workspace_members FOR EACH ROW EXECUTE FUNCTION app.preserve_company_owner();



-- Explicit security metadata is part of the desired declarative state.
revoke all on function app.ensure_company_catalog() from public, anon, authenticated, service_role, ai_center_runtime;
revoke all on function app.ensure_scope_agents(uuid) from public, anon, authenticated, service_role;
revoke all on function app.create_company_workspace(uuid,text,text,text) from public, anon, authenticated, service_role;
revoke all on function app.graph_endpoint_exists(text,uuid,bigint,bigint) from public, anon, authenticated, service_role;
revoke all on function app.validate_graph_edge_scope() from public, anon, authenticated, service_role;
revoke all on function app.preserve_company_owner() from public,anon,authenticated,service_role;
revoke execute on function app.artifact_head_is_current() from public,anon,authenticated;
revoke execute on function app.artifact_source_before_head() from public,anon,authenticated;
alter table app.artifact_documents force row level security;
revoke all on app.artifact_documents from public, anon, authenticated, service_role;
alter table app.artifact_document_versions force row level security;
revoke all on app.artifact_document_versions from public, anon, authenticated, service_role;
alter table app.artifact_version_sources force row level security;
revoke all on app.artifact_version_sources from public, anon, authenticated, service_role;
alter table app.artifact_destination_settings force row level security;
revoke all on app.artifact_destination_settings from public, anon, authenticated, service_role;
-- Extensions are replayed after the baseline role reset by runtime-db-role.sh.
-- Keep this file privileges-only: role repair must not rerun schema DDL or data.
grant update(name, description) on app.workspaces to ai_center_runtime;
grant execute on function app.ensure_scope_agents(uuid) to ai_center_runtime;
grant execute on function app.create_company_workspace(uuid,text,text,text) to ai_center_runtime;
grant execute on function app.graph_endpoint_exists(text,uuid,bigint,bigint) to ai_center_runtime;

grant select, insert on app.artifact_documents, app.artifact_document_versions,
  app.artifact_version_sources to ai_center_runtime;
grant update(current_version_id, updated_at) on app.artifact_documents to ai_center_runtime;
grant select, insert, update on app.artifact_destination_settings to ai_center_runtime;
grant usage on sequence app.artifact_documents_id_seq, app.artifact_document_versions_id_seq,
  app.artifact_version_sources_id_seq, app.artifact_destination_settings_id_seq to ai_center_runtime;
