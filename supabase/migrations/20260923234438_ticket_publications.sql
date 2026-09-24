-- Generated from schemas/06_work_tools.sql with the guarded legacy migra engine.
-- Review: replace only document-level uniqueness; preserve every legacy job at index -1.
alter table "app"."publication_jobs" drop constraint "publication_jobs_workspace_id_artifact_version_id_provider__key";

drop index if exists "app"."publication_jobs_workspace_id_artifact_version_id_provider__key";

alter table "app"."publication_jobs" add column "source_ticket_index" smallint not null default '-1'::integer;

CREATE UNIQUE INDEX publication_jobs_source_destination_unique ON app.publication_jobs USING btree (workspace_id, artifact_version_id, provider, target_id, source_ticket_index);

alter table "app"."publication_jobs" add constraint "publication_jobs_source_destination_unique" UNIQUE using index "publication_jobs_source_destination_unique";

alter table "app"."publication_jobs" add constraint "publication_jobs_source_ticket_index_check" CHECK (((source_ticket_index >= '-1'::integer) AND (source_ticket_index <= 29))) not valid;

alter table "app"."publication_jobs" validate constraint "publication_jobs_source_ticket_index_check";

set check_function_bodies = off;

CREATE OR REPLACE FUNCTION app.publication_validate_source_ticket()
 RETURNS trigger
 LANGUAGE plpgsql
 SET search_path TO ''
AS $function$
declare source_kind text; source_content jsonb; tickets jsonb;
begin
  if tg_op='UPDATE' then
    if (new.public_id,new.workspace_id,new.project_id,new.artifact_version_id,
        new.source_ticket_index,new.connection_id,new.connection_revision,
        new.requested_by_actor_id,new.provider,new.target_id,new.title,new.body_markdown,new.content_hash)
      is distinct from
       (old.public_id,old.workspace_id,old.project_id,old.artifact_version_id,
        old.source_ticket_index,old.connection_id,old.connection_revision,
        old.requested_by_actor_id,old.provider,old.target_id,old.title,old.body_markdown,old.content_hash) then
      raise exception 'A publication source and prepared content are immutable' using errcode='23514';
    end if;
    return new;
  end if;
  if new.source_ticket_index>=0 then
    select d.artifact_type,v.structured_content into source_kind,source_content
      from app.artifact_document_versions v join app.artifact_documents d on d.id=v.document_id
      where v.id=new.artifact_version_id and v.workspace_id=new.workspace_id and v.project_id=new.project_id;
    if not found or new.provider not in ('linear','github')
      or source_kind not in ('product_tickets','technical_tickets')
      or source_content->>'format' is distinct from 'agent-artifact-v1'
      or source_content->>'artifact_type' is distinct from source_kind then
      raise exception 'An indexed publication requires an exact structured ticket source' using errcode='23514';
    end if;
    tickets=source_content->'draft'->'tickets';
    if jsonb_typeof(tickets) is distinct from 'array' then
      raise exception 'The ticket source has no structured entries' using errcode='23514';
    end if;
    if jsonb_array_length(tickets) not between 1 and 30
      or jsonb_typeof(tickets->new.source_ticket_index::integer) is distinct from 'object' then
      raise exception 'The selected ticket does not exist in this version' using errcode='23514';
    end if;
  end if;
  return new;
end;
$function$
;

CREATE TRIGGER publication_jobs_validate_source_ticket BEFORE INSERT OR UPDATE ON app.publication_jobs FOR EACH ROW EXECUTE FUNCTION app.publication_validate_source_ticket();

-- migra does not preserve these declared privilege revocations.
revoke all on function app.publication_validate_source_ticket() from public,anon,authenticated,service_role;
set check_function_bodies = on;
