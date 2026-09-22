-- Company scopes reuse the existing versioned project container. They are
-- explicitly typed and never presented as a customer project.
alter table app.workspaces add column description text not null default '';
alter table app.workspaces add column bootstrap_request_hash text;
alter table app.workspaces add constraint workspaces_description_length
  check (length(description) <= 4000);
alter table app.workspaces add constraint workspaces_bootstrap_hash_valid
  check (bootstrap_request_hash is null or bootstrap_request_hash ~ '^[0-9a-f]{64}$');
create index workspaces_owner_actor_idx on app.workspaces(owner_actor_id);
grant update(name,description) on app.workspaces to ai_center_runtime;

alter table app.projects add column scope_kind text not null default 'project';
alter table app.projects add constraint projects_scope_kind_valid
  check (scope_kind in ('company', 'project'));
create unique index projects_one_company_scope on app.projects(workspace_id)
  where scope_kind = 'company';

alter table app.agent_profiles drop constraint agent_profiles_scope_kind_valid;
alter table app.agent_profiles add constraint agent_profiles_scope_kind_valid
  check (scope_kind in ('general', 'product', 'sales', 'tech', 'steward'));

-- Catalog installation is deterministic data owned by the application. There
-- is no caller-provided prompt, role or privilege in this privileged helper.
create or replace function app.ensure_company_catalog()
returns void language plpgsql security definer set search_path = '' as $$
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
$$;
revoke all on function app.ensure_company_catalog() from public, anon, authenticated, service_role, ai_center_runtime;

create or replace function app.ensure_scope_agents(requested_project_public_id uuid)
returns void language plpgsql security definer set search_path = '' as $$
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
$$;
revoke all on function app.ensure_scope_agents(uuid) from public, anon, authenticated, service_role;
grant execute on function app.ensure_scope_agents(uuid) to ai_center_runtime;

-- Narrow bootstrap helper: only the authenticated actor installed by the API
-- is used. It serializes creation per actor before checking the ten-company cap.
create or replace function app.create_company_workspace(
  requested_public_id uuid, requested_name text, requested_description text, requested_hash text
)
returns uuid language plpgsql security definer set search_path = '' as $$
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
$$;
revoke all on function app.create_company_workspace(uuid,text,text,text) from public, anon, authenticated, service_role;
grant execute on function app.create_company_workspace(uuid,text,text,text) to ai_center_runtime;

-- Existing project producers omit target_project_id; the trigger keeps that
-- backward-compatible while checking both endpoints against their real scope.
alter table app.edges add column target_project_id bigint;
update app.edges set target_project_id=project_id;
alter table app.edges alter column target_project_id set not null;
alter table app.edges add constraint edges_target_project_workspace_fkey
  foreign key(target_project_id,workspace_id) references app.projects(id,workspace_id);
create index edges_target_project_target_idx on app.edges(target_project_id,target_public_id);

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
    when 'task' then return exists(select 1 from app.tasks where public_id=endpoint_id and project_id=scope_id and workspace_id=tenant_id);
    when 'execution' then return exists(select 1 from app.executions where public_id=endpoint_id and project_id=scope_id and workspace_id=tenant_id);
    when 'insight' then return exists(select 1 from app.insights where public_id=endpoint_id and project_id=scope_id and workspace_id=tenant_id);
    else return false;
  end case;
end;
$$;
revoke all on function app.graph_endpoint_exists(text,uuid,bigint,bigint) from public, anon, authenticated, service_role;
grant execute on function app.graph_endpoint_exists(text,uuid,bigint,bigint) to ai_center_runtime;

create or replace function app.validate_graph_edge_scope()
returns trigger language plpgsql security invoker set search_path = '' as $$
begin
  new.target_project_id := coalesce(new.target_project_id,new.project_id);
  if not app.graph_endpoint_exists(new.source_kind,new.source_public_id,new.project_id,new.workspace_id)
    or not app.graph_endpoint_exists(new.target_kind,new.target_public_id,new.target_project_id,new.workspace_id) then
    raise exception 'graph endpoint outside declared scope' using errcode = '23503';
  end if;
  return new;
end;
$$;
revoke all on function app.validate_graph_edge_scope() from public, anon, authenticated, service_role;
create trigger edges_validate_scope before insert or update on app.edges
  for each row execute function app.validate_graph_edge_scope();

create or replace function app.preserve_company_owner()
returns trigger language plpgsql security invoker set search_path = '' as $$
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
$$;
revoke all on function app.preserve_company_owner() from public,anon,authenticated,service_role;
create trigger workspace_members_preserve_owner before update on app.workspace_members
  for each row execute function app.preserve_company_owner();

-- The runtime still cannot insert workspaces or modify the global catalog.
-- New columns inherit existing FORCE RLS, and bootstrap is the only exception.
