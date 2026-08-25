do $verify$
declare
  runtime_role pg_catalog.pg_roles%rowtype;
  unexpected_membership text;
  destructive_table text;
  missing_privilege text;
  unexpected_select_table text;
  unexpected_insert_table text;
  unexpected_update_table text;
  unexpected_sequence text;
  unexpected_function text;
  expected_select_tables constant text[] := array[
    'workspaces', 'workspace_members', 'project_templates', 'agent_profiles',
    'deliverable_contracts', 'projects', 'context_nodes',
    'knowledge_entries', 'knowledge_entry_versions', 'edges', 'context_packs',
    'context_pack_sources', 'context_pack_selection_items', 'sessions',
    'model_runs', 'messages', 'idempotency_records', 'mutation_proposals',
    'gates', 'handoffs', 'deliverables', 'deliverable_sections',
    'deliverable_sources', 'tool_connections', 'external_references',
    'external_reference_observations', 'artifacts', 'evidences',
    'requirement_coverage', 'steward_assessments',
    'steward_assessment_sources', 'insights', 'insight_sources',
    'insight_resolutions', 'domain_events', 'audit_events'
  ];
  expected_insert_tables constant text[] := array[
    'workspace_members', 'projects', 'context_nodes', 'knowledge_entries',
    'knowledge_entry_versions', 'edges', 'context_packs',
    'context_pack_sources', 'context_pack_selection_items', 'sessions',
    'model_runs', 'messages', 'idempotency_records', 'mutation_proposals',
    'gates', 'handoffs', 'deliverables', 'deliverable_sections',
    'deliverable_sources', 'external_references',
    'external_reference_observations', 'evidences', 'requirement_coverage',
    'steward_assessments', 'steward_assessment_sources', 'insights',
    'insight_sources', 'insight_resolutions', 'domain_events', 'audit_events'
  ];
  expected_update_tables constant text[] := array[
    'workspace_members', 'projects', 'sessions', 'idempotency_records', 'mutation_proposals',
    'context_packs', 'gates', 'model_runs', 'deliverables',
    'external_references', 'evidences', 'requirement_coverage', 'insights',
    'knowledge_entries', 'domain_events'
  ];
  allowed_function_oids constant oid[] := array[
    'app.current_actor_id()'::regprocedure::oid,
    'app.current_workspace_id()'::regprocedure::oid,
    'app.current_workspace_role()'::regprocedure::oid,
    'app.has_workspace_role(bigint,text[])'::regprocedure::oid,
    'app.authorize_workspace_member(uuid,uuid)'::regprocedure::oid,
    'app.list_actor_workspaces()'::regprocedure::oid,
    'app.list_due_steward_workspaces(integer)'::regprocedure::oid
  ];
begin
  select role_row.*
  into runtime_role
  from pg_catalog.pg_roles role_row
  where role_row.rolname = 'ai_center_runtime';

  if not found then
    raise exception 'ai_center_runtime does not exist';
  end if;

  if runtime_role.rolsuper
    or runtime_role.rolbypassrls
    or runtime_role.rolcreatedb
    or runtime_role.rolcreaterole
    or runtime_role.rolreplication
  then
    raise exception 'ai_center_runtime has an elevated role attribute';
  end if;

  if not runtime_role.rolcanlogin then
    raise exception 'ai_center_runtime cannot be used as the API login role';
  end if;

  if runtime_role.rolinherit then
    raise exception 'ai_center_runtime must be NOINHERIT';
  end if;

  if not pg_catalog.has_database_privilege(
    'ai_center_runtime', current_database(), 'CONNECT'
  ) then
    raise exception 'ai_center_runtime cannot connect to the current database';
  end if;

  if pg_catalog.has_database_privilege(
    'ai_center_runtime', current_database(), 'CREATE'
  ) then
    raise exception 'ai_center_runtime must not create schemas in the database';
  end if;

  select parent.rolname
  into unexpected_membership
  from pg_catalog.pg_auth_members membership
  join pg_catalog.pg_roles parent on parent.oid = membership.roleid
  where membership.member = runtime_role.oid
  limit 1;

  if unexpected_membership is not null then
    raise exception 'ai_center_runtime unexpectedly belongs to role %',
      unexpected_membership;
  end if;

  if not pg_catalog.has_schema_privilege(
    'ai_center_runtime', 'app', 'USAGE'
  ) then
    raise exception 'ai_center_runtime lacks USAGE on schema app';
  end if;

  if pg_catalog.has_schema_privilege(
    'ai_center_runtime', 'app', 'CREATE'
  ) then
    raise exception 'ai_center_runtime must not CREATE in schema app';
  end if;

  if pg_catalog.has_schema_privilege(
    'ai_center_runtime', 'public', 'CREATE'
  ) then
    raise exception 'ai_center_runtime must not CREATE in schema public';
  end if;

  select relation.relname
  into destructive_table
  from pg_catalog.pg_class relation
  join pg_catalog.pg_namespace namespace on namespace.oid = relation.relnamespace
  where namespace.nspname = 'app'
    and relation.relkind in ('r', 'p')
    and (
      pg_catalog.has_table_privilege(
        'ai_center_runtime', relation.oid, 'DELETE'
      )
      or pg_catalog.has_table_privilege(
        'ai_center_runtime', relation.oid, 'TRUNCATE'
      )
      or pg_catalog.has_table_privilege(
        'ai_center_runtime', relation.oid, 'REFERENCES'
      )
      or pg_catalog.has_table_privilege(
        'ai_center_runtime', relation.oid, 'TRIGGER'
      )
      or pg_catalog.has_table_privilege(
        'ai_center_runtime', relation.oid, 'MAINTAIN'
      )
    )
  limit 1;

  if destructive_table is not null then
    raise exception 'ai_center_runtime has a destructive privilege on app.%',
      destructive_table;
  end if;

  select table_name
  into missing_privilege
  from unnest(expected_select_tables) as expected(table_name)
  where not pg_catalog.has_table_privilege(
    'ai_center_runtime', pg_catalog.format('app.%I', table_name), 'SELECT'
  )
  limit 1;

  if missing_privilege is not null then
    raise exception 'ai_center_runtime lacks SELECT on app.%',
      missing_privilege;
  end if;

  select relation.relname
  into unexpected_select_table
  from pg_catalog.pg_class relation
  join pg_catalog.pg_namespace namespace on namespace.oid = relation.relnamespace
  where namespace.nspname = 'app'
    and relation.relkind in ('r', 'p')
    and pg_catalog.has_table_privilege(
      'ai_center_runtime', relation.oid, 'SELECT'
    )
    and relation.relname <> all(expected_select_tables)
  limit 1;

  if unexpected_select_table is not null then
    raise exception 'ai_center_runtime has unexpected SELECT on app.%',
      unexpected_select_table;
  end if;

  missing_privilege := null;
  select table_name
  into missing_privilege
  from unnest(expected_insert_tables) as expected(table_name)
  where not pg_catalog.has_table_privilege(
    'ai_center_runtime', pg_catalog.format('app.%I', table_name), 'INSERT'
  )
  limit 1;

  if missing_privilege is not null then
    raise exception 'ai_center_runtime lacks INSERT on app.%',
      missing_privilege;
  end if;

  select relation.relname
  into unexpected_insert_table
  from pg_catalog.pg_class relation
  join pg_catalog.pg_namespace namespace on namespace.oid = relation.relnamespace
  where namespace.nspname = 'app'
    and relation.relkind in ('r', 'p')
    and pg_catalog.has_table_privilege(
      'ai_center_runtime', relation.oid, 'INSERT'
    )
    and relation.relname <> all(expected_insert_tables)
  limit 1;

  if unexpected_insert_table is not null then
    raise exception 'ai_center_runtime has unexpected INSERT on app.%',
      unexpected_insert_table;
  end if;

  missing_privilege := null;
  select table_name
  into missing_privilege
  from unnest(expected_insert_tables) as expected(table_name)
  where not pg_catalog.has_sequence_privilege(
    'ai_center_runtime',
    pg_catalog.format('app.%I', table_name || '_id_seq'),
    'USAGE'
  )
  limit 1;

  if missing_privilege is not null then
    raise exception 'ai_center_runtime lacks sequence USAGE for app.%',
      missing_privilege;
  end if;

  select relation.relname
  into unexpected_sequence
  from pg_catalog.pg_class relation
  join pg_catalog.pg_namespace namespace on namespace.oid = relation.relnamespace
  where namespace.nspname = 'app'
    and relation.relkind = 'S'
    and pg_catalog.has_sequence_privilege(
      'ai_center_runtime',
      pg_catalog.format('%I.%I', namespace.nspname, relation.relname),
      'UPDATE'
    )
  limit 1;

  if unexpected_sequence is not null then
    raise exception 'ai_center_runtime can set sequence app.%',
      unexpected_sequence;
  end if;

  missing_privilege := null;
  select table_name
  into missing_privilege
  from unnest(expected_update_tables) as expected(table_name)
  where not pg_catalog.has_table_privilege(
    'ai_center_runtime', pg_catalog.format('app.%I', table_name), 'UPDATE'
  )
  limit 1;

  if missing_privilege is not null then
    raise exception 'ai_center_runtime lacks UPDATE on app.%',
      missing_privilege;
  end if;

  select relation.relname
  into unexpected_update_table
  from pg_catalog.pg_class relation
  join pg_catalog.pg_namespace namespace on namespace.oid = relation.relnamespace
  where namespace.nspname = 'app'
    and relation.relkind in ('r', 'p')
    and pg_catalog.has_table_privilege(
      'ai_center_runtime', relation.oid, 'UPDATE'
    )
    and relation.relname <> all(expected_update_tables)
  limit 1;

  if unexpected_update_table is not null then
    raise exception 'ai_center_runtime has unexpected UPDATE on app.%',
      unexpected_update_table;
  end if;

  select pg_catalog.format('%I(%s)', procedure.proname,
    pg_catalog.pg_get_function_identity_arguments(procedure.oid))
  into unexpected_function
  from pg_catalog.pg_proc procedure
  join pg_catalog.pg_namespace namespace on namespace.oid = procedure.pronamespace
  where namespace.nspname = 'app'
    and pg_catalog.has_function_privilege(
      'ai_center_runtime', procedure.oid, 'EXECUTE'
    )
    and procedure.oid <> all(allowed_function_oids)
  limit 1;

  if unexpected_function is not null then
    raise exception 'ai_center_runtime can execute unexpected function app.%',
      unexpected_function;
  end if;

  select procedure_oid::regprocedure::text
  into missing_privilege
  from unnest(allowed_function_oids) as expected(procedure_oid)
  where not pg_catalog.has_function_privilege(
    'ai_center_runtime', procedure_oid, 'EXECUTE'
  )
  limit 1;

  if missing_privilege is not null then
    raise exception 'ai_center_runtime cannot execute required function %',
      missing_privilege;
  end if;

  if not pg_catalog.has_function_privilege(
    'ai_center_runtime',
    'app.authorize_workspace_member(uuid,uuid)',
    'EXECUTE'
  ) then
    raise exception 'ai_center_runtime cannot authorize workspace membership';
  end if;

  if exists (
    select 1
    from pg_catalog.aclexplode(
      coalesce(
        (
          select procedure.proacl
          from pg_catalog.pg_proc procedure
          where procedure.oid =
            'app.list_due_steward_workspaces(integer)'::regprocedure
        ),
        pg_catalog.acldefault(
          'f',
          (
            select procedure.proowner
            from pg_catalog.pg_proc procedure
            where procedure.oid =
              'app.list_due_steward_workspaces(integer)'::regprocedure
          )
        )
      )
    ) privilege
    where privilege.grantee = 0
      and privilege.privilege_type = 'EXECUTE'
  ) then
    raise exception 'PUBLIC can execute the Steward workspace scanner';
  end if;

  if exists (
    select 1
    from pg_catalog.pg_class relation
    join pg_catalog.pg_namespace namespace on namespace.oid = relation.relnamespace
    where namespace.nspname = 'app'
      and relation.relkind in ('r', 'p')
      and pg_catalog.has_table_privilege(
        'ai_center_runtime', relation.oid, 'SELECT'
      )
      and (not relation.relrowsecurity or not relation.relforcerowsecurity)
  ) then
    raise exception 'ai_center_runtime can read an app table without forced RLS';
  end if;
end;
$verify$;

select
  rolname,
  rolcanlogin,
  rolsuper,
  rolbypassrls,
  rolinherit
from pg_catalog.pg_roles
where rolname = 'ai_center_runtime';
