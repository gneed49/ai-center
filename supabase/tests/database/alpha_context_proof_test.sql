begin;

create extension if not exists pgtap with schema extensions;

select plan(32);

select has_table('app', 'workspace_members', 'workspace members are durable');
select has_table('app', 'idempotency_records', 'idempotency records are durable');
select has_table('app', 'model_runs', 'model runs are durable');
select has_table(
  'app',
  'context_pack_selection_items',
  'context pack selection decisions are durable'
);
select has_table('app', 'deliverable_sources', 'deliverable lineage sources are durable');
select has_table('app', 'steward_assessments', 'all steward classifications are durable');
select has_table('app', 'tool_connections', 'tool connections are durable');
select has_table('app', 'external_references', 'external references are durable');
select has_table(
  'app',
  'external_reference_observations',
  'external reference observations are durable'
);
select has_table('app', 'insight_resolutions', 'insight resolutions are durable');

select has_column(
  'app',
  'context_packs',
  'source_graph_version',
  'context packs pin their source graph version'
);
select has_column(
  'app',
  'artifacts',
  'external_reference_id',
  'artifacts can point to an external reference'
);
select has_column(
  'app',
  'evidences',
  'external_reference_id',
  'evidences can point to an external reference'
);
select has_column(
  'app',
  'requirement_coverage',
  'workspace_id',
  'coverage rows carry their tenant scope'
);

select has_trigger(
  'app',
  'external_reference_observations',
  'external_reference_observations_prevent_update',
  'external observations are append-only'
);
select has_trigger(
  'app',
  'context_packs',
  'context_packs_enforce_immutability',
  'context pack payloads are immutable'
);
select has_index(
  'app',
  'domain_events',
  'domain_events_pending_available_idx',
  'the outbox has a pending-work claim index'
);

select ok(
  to_regprocedure('app.list_due_steward_workspaces(integer)') is not null,
  'the private Steward recovery scanner exists'
);

select ok(
  not exists (
    select 1
    from pg_catalog.pg_proc procedure
    cross join lateral pg_catalog.aclexplode(
      coalesce(
        procedure.proacl,
        pg_catalog.acldefault('f', procedure.proowner)
      )
    ) privilege
    where procedure.oid =
      'app.list_due_steward_workspaces(integer)'::regprocedure
      and privilege.grantee = 0
      and privilege.privilege_type = 'EXECUTE'
  ),
  'PUBLIC cannot execute the private Steward recovery scanner'
);

select ok(
  has_function_privilege(
    'ai_center_runtime',
    'app.list_due_steward_workspaces(integer)',
    'EXECUTE'
  ),
  'the runtime role can execute only the allowlisted recovery scanner'
);

select ok(
  not has_function_privilege(
    'anon',
    'app.list_due_steward_workspaces(integer)',
    'EXECUTE'
  )
  and not has_function_privilege(
    'authenticated',
    'app.list_due_steward_workspaces(integer)',
    'EXECUTE'
  )
  and not has_function_privilege(
    'service_role',
    'app.list_due_steward_workspaces(integer)',
    'EXECUTE'
  ),
  'no Supabase client role can execute the recovery scanner'
);

select ok(
  (
    select pg_get_constraintdef(constraint_row.oid) like '%tracked_by%'
    from pg_constraint constraint_row
    where constraint_row.conrelid = 'app.edges'::regclass
      and constraint_row.conname = 'edges_type_valid'
  ),
  'tracked_by is part of the edge vocabulary contract'
);

select extensions.results_eq(
  $$
    select relrowsecurity and relforcerowsecurity
    from pg_class relation
    join pg_namespace namespace on namespace.oid = relation.relnamespace
    where namespace.nspname = 'app'
      and relation.relname = 'external_references'
  $$,
  array[true],
  'external references have RLS enabled and forced'
);

insert into app.workspace_members (
  public_id,
  workspace_id,
  actor_id,
  role,
  invitation_status,
  invited_by_actor_id,
  accepted_at
)
select
  '11000000-0000-0000-0000-000000000099',
  workspace.id,
  '00000000-0000-0000-0000-000000000099',
  'viewer',
  'accepted',
  workspace.owner_actor_id,
  now()
from app.workspaces workspace
where workspace.public_id = '10000000-0000-0000-0000-000000000001';

select set_config(
  'test.alpha_workspace_id',
  (
    select id::text
    from app.workspaces
    where public_id = '10000000-0000-0000-0000-000000000001'
  ),
  true
);

-- Supabase local grants its non-superuser `postgres` role ADMIN membership
-- with SET disabled. Enable SET only inside this rolled-back test transaction
-- so policies are exercised as the real NOBYPASSRLS API login.
grant ai_center_runtime to postgres with set true, inherit false;
grant usage on schema extensions to ai_center_runtime;
grant execute on all functions in schema extensions to ai_center_runtime;

set local role ai_center_runtime;
select set_config('app.current_actor_id', '00000000-0000-0000-0000-000000000001', true);
select set_config(
  'app.current_workspace_id',
  current_setting('test.alpha_workspace_id'),
  true
);
select set_config('app.current_workspace_role', 'owner', true);

insert into app.projects (
  public_id,
  workspace_id,
  template_id,
  name,
  objective,
  created_by_actor_id
)
select
  '82000000-0000-0000-0000-000000000001',
  workspace.id,
  template.id,
  'Steward recovery pgTAP project',
  'Prove bounded private outbox discovery',
  '00000000-0000-0000-0000-000000000001'
from app.workspaces workspace
cross join app.project_templates template
where workspace.public_id = '10000000-0000-0000-0000-000000000001'
  and template.template_key = 'software-product-delivery';

insert into app.domain_events (
  workspace_id,
  project_id,
  event_type,
  aggregate_kind,
  aggregate_public_id,
  available_at
)
select
  workspace.id,
  project.id,
  source.event_type,
  'knowledge_entry',
  source.aggregate_public_id,
  source.available_at
from app.workspaces workspace
join app.projects project on project.workspace_id = workspace.id
cross join (
  values
    (
      'knowledge.committed'::text,
      '81000000-0000-0000-0000-000000000001'::uuid,
      clock_timestamp() - interval '1 second'
    ),
    (
      'connector.refresh.requested'::text,
      '81000000-0000-0000-0000-000000000002'::uuid,
      clock_timestamp() - interval '1 second'
    ),
    (
      'knowledge.revised'::text,
      '81000000-0000-0000-0000-000000000003'::uuid,
      clock_timestamp() + interval '1 hour'
    )
) source(event_type, aggregate_public_id, available_at)
where workspace.public_id = '10000000-0000-0000-0000-000000000001'
  and project.public_id = '82000000-0000-0000-0000-000000000001';

select extensions.results_eq(
  $$
    select workspace_id, actor_id, workspace_role
    from app.list_due_steward_workspaces(8)
  $$,
  $$
    select
      current_setting('test.alpha_workspace_id')::bigint,
      '00000000-0000-0000-0000-000000000001'::uuid,
      'owner'::text
  $$,
  'the scanner returns one due scope with an accepted mutating member'
);

select extensions.is_empty(
  $$ select * from app.list_due_steward_workspaces(-1) $$,
  'an invalid scanner bound fails closed'
);

update app.domain_events
set status = 'processed', processed_at = clock_timestamp()
where aggregate_public_id = '81000000-0000-0000-0000-000000000001';

select extensions.is_empty(
  $$ select * from app.list_due_steward_workspaces(8) $$,
  'unsupported and future events are excluded from recovery discovery'
);

select extensions.results_eq(
  $$
    select count(*)::bigint
    from app.workspaces
    where public_id = '10000000-0000-0000-0000-000000000001'
  $$,
  array[1::bigint],
  'the seeded owner can read their workspace'
);

select set_config('app.current_workspace_id', '9223372036854775807', true);
select extensions.results_eq(
  $$
    select count(*)::bigint
    from app.workspaces
    where public_id = '10000000-0000-0000-0000-000000000001'
  $$,
  array[0::bigint],
  'a mismatched workspace GUC cannot read the workspace'
);

select set_config(
  'app.current_workspace_id',
  current_setting('test.alpha_workspace_id'),
  true
);
select set_config('app.current_actor_id', '00000000-0000-0000-0000-000000000098', true);
select extensions.results_eq(
  $$
    select count(*)::bigint
    from app.workspaces
    where public_id = '10000000-0000-0000-0000-000000000001'
  $$,
  array[0::bigint],
  'a non-member actor cannot read the workspace'
);

select extensions.results_eq(
  $$
    select count(*)::bigint
    from app.workspace_members
    where workspace_id = app.current_workspace_id()
  $$,
  array[0::bigint],
  'a non-member actor cannot inspect workspace membership'
);

select set_config('app.current_actor_id', '00000000-0000-0000-0000-000000000001', true);
select extensions.results_eq(
  $$
    select count(*)::bigint
    from app.workspace_members
    where workspace_id = app.current_workspace_id()
  $$,
  array[2::bigint],
  'an owner can inspect workspace membership'
);

select set_config('app.current_actor_id', '00000000-0000-0000-0000-000000000099', true);
select set_config('app.current_workspace_role', 'viewer', true);
select extensions.is_empty(
  $$
    update app.workspace_members
    set role = 'editor'
    where actor_id = '00000000-0000-0000-0000-000000000099'
    returning role
  $$,
  'a viewer cannot promote their own membership'
);

reset role;

select * from finish();
rollback;
