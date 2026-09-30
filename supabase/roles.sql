-- Custom cluster roles are provisioned by the Supabase CLI before migrations.
-- Keep passwords out of this file. The operational provisioning script can set
-- the runtime password from a private environment variable when required.

do $$
begin
  if not exists (
    select 1
    from pg_catalog.pg_roles
    where rolname = 'ai_center_runtime'
  ) then
    -- LOGIN and NOINHERIT are the only non-default attributes needed here.
    -- NOSUPERUSER, NOCREATEDB, NOCREATEROLE, NOREPLICATION and NOBYPASSRLS
    -- are PostgreSQL's safe CREATE ROLE defaults and are asserted by the
    -- post-start verifier. Supabase applies roles.sql as `supabase_admin`,
    -- which intentionally cannot alter superuser-only role attributes.
    create role ai_center_runtime login noinherit;
  end if;
end;
$$;

alter role ai_center_runtime with
  login
  noinherit;

alter role ai_center_runtime set row_security = on;
alter role ai_center_runtime set search_path = pg_catalog;

do $$
begin
  execute pg_catalog.format(
    'grant connect on database %I to ai_center_runtime',
    current_database()
  );
end;
$$;

comment on role ai_center_runtime is
  'AI Center API runtime login; non-privileged and always subject to RLS';
