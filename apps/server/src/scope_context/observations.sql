select o.id source_id,o.public_id, o.project_id,p.public_id project_public_id,p.graph_version,p.scope_kind,
 'tool_source_observation'::text source_kind,r.public_id object_public_id,o.version,o.provider,o.external_id,
 o.title,o.body_markdown,o.observed_at
from app.tool_source_observations o join app.tool_source_references r on r.id=o.reference_id
join app.projects p on p.id=o.project_id
where o.workspace_id=app.current_workspace_id() and app.tool_source_observation_current(o.id)
union all
select o.id,o.public_id,o.project_id,p.public_id,p.graph_version,p.scope_kind,
 'publication_observation',o.publication_public_id,o.version,o.provider,o.external_id,
 o.title,o.body_markdown,o.observed_at
from app.publication_source_observations o join app.projects p on p.id=o.project_id
join app.work_tool_connections c on c.id=o.authority_connection_id and c.workspace_id=o.workspace_id and c.provider=o.provider
where o.workspace_id=app.current_workspace_id() and o.provider in ('notion','linear')
 and o.external_id is not null and o.id=o.canonical_observation_id
 and o.is_current and o.availability='available' and p.status='active'
 and c.enabled and c.revision=o.authority_connection_revision
