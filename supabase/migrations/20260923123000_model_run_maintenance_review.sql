-- Reviewed immutable starting actor: tenant-owned model row, no new foreign keys.
create or replace function app.operator_project_expected_schema()
returns text language sql immutable security invoker set search_path='' as $$
 select '9d19679eb7bedeb15663096a6a515ddc'::text;
$$;
revoke all on function app.operator_project_expected_schema() from public,anon,authenticated,service_role,ai_center_runtime;
