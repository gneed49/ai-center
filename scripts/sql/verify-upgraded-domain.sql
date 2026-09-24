-- Every assertion is evaluated against the upgraded legacy database, not a reset.
do $$
begin
  if (select count(*) from app.workspaces) <> 2
    or (select count(*) from app.projects where objective='Preserve confirmed knowledge' and graph_version=7) <> 2
    or (select count(*) from app.knowledge_entries where latest_version=1 and status='confirmed') <> 2
    or (select count(*) from app.knowledge_entry_versions v
      join app.workspaces w on w.id=v.workspace_id
      join app.knowledge_entries k on k.id=v.knowledge_entry_id and k.project_id=v.project_id
      where v.statement='Confirmed knowledge must survive upgrades.' and v.version_number=1
        and v.author_actor_id=w.owner_actor_id and v.created_at='2026-08-18T12:00:00Z') <> 2
    or (select count(*) from app.domain_events where payload='{"legacy":true}' and requested_by_actor_id is null) <> 2
  then
    raise exception 'Upgraded legacy data lost content, ownership, versions or event provenance';
  end if;

  -- Effects of the September migrations: grants, observation cycles/leases,
  -- personal connection tables, RLS and the event's originating actor.
  if not has_table_privilege('ai_center_runtime','app.tasks','INSERT')
    or not has_table_privilege('ai_center_runtime','app.tasks','UPDATE')
    or not has_table_privilege('ai_center_runtime','app.execution_events','INSERT')
    or not has_sequence_privilege('ai_center_runtime','app.artifacts_id_seq','USAGE')
    or not exists(select 1 from information_schema.columns
      where table_schema='app' and table_name='idempotency_records' and column_name='lease_generation' and is_nullable='NO')
    or exists(select 1 from pg_constraint where conrelid='app.external_reference_observations'::regclass
      and conname='external_reference_observations_reference_hash_unique')
    or not exists(select 1 from pg_indexes where schemaname='app'
      and indexname='external_reference_observations_reference_observed_idx' and indexdef like '%id DESC%')
    or (select count(*) from pg_class c join pg_namespace n on n.oid=c.relnamespace
      where n.nspname='app' and c.relname in ('provider_connections','provider_selections')
      and c.relrowsecurity and c.relforcerowsecurity) <> 2
    or (select count(*) from pg_policies where schemaname='app'
      and tablename in ('provider_connections','provider_selections')) <> 2
    or not has_table_privilege('ai_center_runtime','app.provider_connections','DELETE')
    or has_table_privilege('authenticated','app.provider_connections','SELECT')
  then
    raise exception 'Upgrade did not reach the current application schema';
  end if;
end;
$$;
