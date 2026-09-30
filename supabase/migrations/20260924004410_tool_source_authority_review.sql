set check_function_bodies = off;

CREATE OR REPLACE FUNCTION app.validate_publication_attestation()
 RETURNS trigger
 LANGUAGE plpgsql
 SET search_path TO ''
AS $function$
begin
 if new.connection_id is not null and not exists(
  select 1 from app.publication_jobs j join app.work_tool_connections c on c.id=j.connection_id and c.workspace_id=j.workspace_id
  where j.id=new.publication_job_id and j.workspace_id=new.workspace_id and c.id=new.connection_id
   and c.revision=new.connection_revision and c.provider=j.provider and c.enabled) then
  raise exception 'Publication observation authority does not match its connection' using errcode='23514';
 end if;
 return new;
end;
$function$
;

CREATE OR REPLACE FUNCTION app.operator_project_expected_schema()
 RETURNS text
 LANGUAGE sql
 IMMUTABLE
 SET search_path TO ''
AS $function$
 select '0e4ba7e13c76db41cfb4677a5d465c6c'::text;
$function$
;

CREATE TRIGGER publication_observation_attestation BEFORE INSERT ON app.publication_observations FOR EACH ROW EXECUTE FUNCTION app.validate_publication_attestation();



-- Preserve private trigger and operator-only maintenance ACLs.
revoke all on function app.validate_publication_attestation() from public,anon,authenticated,service_role;
revoke all on function app.operator_project_expected_schema() from public,anon,authenticated,service_role,ai_center_runtime;
