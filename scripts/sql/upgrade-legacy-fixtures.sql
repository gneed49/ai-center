-- Test data only, applied to the disposable baseline before any upgrade.
do $$
declare
  template bigint;
  profile bigint;
  project bigint;
  node bigint;
  knowledge bigint;
  workspace record;
begin
  insert into app.project_templates(template_key,name,version,definition)
    values ('upgrade-legacy','Legacy template',1,'{}') returning id into template;
  insert into app.agent_profiles(template_id,profile_key,name,scope_kind,instructions)
    values (template,'legacy-product','Legacy product','product','Preserve confirmed intent')
    returning id into profile;
  for workspace in select * from app.workspaces order by id loop
    insert into app.projects(workspace_id,template_id,name,objective,graph_version,created_by_actor_id)
      values (workspace.id,template,'Legacy project','Preserve confirmed knowledge',7,workspace.owner_actor_id)
      returning id into project;
    insert into app.context_nodes(workspace_id,project_id,agent_profile_id,node_key,title)
      values (workspace.id,project,profile,'product','Legacy product') returning id into node;
    insert into app.knowledge_entries(workspace_id,project_id,context_node_id,entry_type)
      values (workspace.id,project,node,'requirement') returning id into knowledge;
    insert into app.knowledge_entry_versions(knowledge_entry_id,workspace_id,project_id,context_node_id,
      version_number,entry_type,title,statement,author_actor_id,origin_type,created_at)
      values (knowledge,workspace.id,project,node,1,'requirement','Durable requirement',
        'Confirmed knowledge must survive upgrades.',workspace.owner_actor_id,'manual','2026-08-18T12:00:00Z');
    insert into app.domain_events(workspace_id,project_id,event_type,aggregate_kind,aggregate_public_id,payload)
      select workspace.id,project,'knowledge.committed','project',public_id,'{"legacy":true}'
      from app.projects where id=project;
  end loop;
end;
$$;
