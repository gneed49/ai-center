\set ON_ERROR_STOP on

begin;

insert into app.tool_connections (
  workspace_id,
  provider,
  auth_mode,
  external_account_id,
  display_name,
  capabilities,
  secret_reference,
  configuration,
  status,
  created_by_actor_id,
  last_verified_at
)
select
  workspace.id,
  'github',
  'github_app',
  :'installation_id',
  :'display_name',
  array['metadata:read', 'contents:read', 'pull_requests:read', 'checks:read', 'statuses:read'],
  :'secret_reference',
  jsonb_build_object('installation_id', :'installation_id'),
  :'connection_status',
  :'actor_id'::uuid,
  case when :'connection_status' = 'active' then now() else null end
from app.workspaces workspace
where workspace.public_id = :'workspace_public_id'::uuid
on conflict (workspace_id, provider, external_account_id) do update
set display_name = excluded.display_name,
    capabilities = excluded.capabilities,
    secret_reference = excluded.secret_reference,
    configuration = excluded.configuration,
    status = excluded.status,
    last_verified_at = excluded.last_verified_at,
    updated_at = now()
returning public_id as provisioned_public_id \gset

\if :{?provisioned_public_id}
\else
  \echo 'Workspace not found; GitHub connection was not provisioned.'
  \quit 3
\endif

commit;

select connection.public_id, connection.provider, connection.external_account_id,
       connection.display_name, connection.capabilities, connection.secret_reference,
       connection.status, connection.last_verified_at
from app.tool_connections connection
join app.workspaces workspace on workspace.id = connection.workspace_id
where workspace.public_id = :'workspace_public_id'::uuid
  and connection.provider = 'github'
  and connection.external_account_id = :'installation_id';
