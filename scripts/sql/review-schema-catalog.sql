-- Read-only inventory for an explicitly guarded, disposable migrated database.
-- The fingerprint is reviewed by the integrator, never accepted automatically.
select app.operator_project_schema_fingerprint() as migrated_fingerprint,
  app.operator_project_expected_schema() as reviewed_fingerprint;
select c.relname as table_name,a.attname as column_name,pg_catalog.format_type(a.atttypid,a.atttypmod) as column_type
from pg_catalog.pg_class c join pg_catalog.pg_namespace n on n.oid=c.relnamespace
join pg_catalog.pg_attribute a on a.attrelid=c.oid and a.attnum>0 and not a.attisdropped
where n.nspname='app' and c.relkind in('r','p') order by c.relname,a.attnum;
select c.relname as table_name,k.conname,pg_catalog.pg_get_constraintdef(k.oid) as constraint_definition
from pg_catalog.pg_constraint k join pg_catalog.pg_class c on c.oid=k.conrelid
join pg_catalog.pg_namespace n on n.oid=c.relnamespace
where n.nspname='app' and k.contype='f' order by c.relname,k.conname;
