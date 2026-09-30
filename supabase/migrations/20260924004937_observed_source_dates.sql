set check_function_bodies = off;

CREATE OR REPLACE FUNCTION app.observed_remote_time(value text)
 RETURNS timestamp with time zone
 LANGUAGE plpgsql
 STABLE STRICT
 SET search_path TO ''
AS $function$
begin
 if length(value)>80 or value !~ '^[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}(\.[0-9]+)?(Z|[+-][0-9]{2}:[0-9]{2})$' then return null; end if;
 begin
  return value::timestamptz;
 exception when invalid_datetime_format or datetime_field_overflow then return null;
 end;
end;
$function$
;

create or replace view "app"."publication_source_observations" as  WITH base AS (
         SELECT o.id,
            o.public_id,
            o.workspace_id,
            j.project_id,
            o.publication_job_id,
            j.public_id AS publication_public_id,
            j.provider,
                CASE
                    WHEN (j.provider = 'notion'::text) THEN 'page'::text
                    ELSE 'issue'::text
                END AS object_kind,
                CASE
                    WHEN (o.external_id ~* '^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$'::text) THEN (o.external_id)::uuid
                    ELSE NULL::uuid
                END AS external_id,
            o.external_url AS canonical_url,
            o.observed_at,
            o.remote_updated_at,
                CASE
                    WHEN (o.observation_kind = 'unavailable'::text) THEN o.external_id
                    ELSE app.context_utf8_prefix(COALESCE((o.snapshot ->> 'title'::text), j.title), 4096)
                END AS title,
                CASE
                    WHEN (o.observation_kind = 'unavailable'::text) THEN ''::text
                    ELSE app.context_utf8_prefix(COALESCE((o.snapshot ->> 'body_markdown'::text), ''::text), 61440)
                END AS body_markdown,
                CASE
                    WHEN (o.observation_kind = 'unavailable'::text) THEN 'unavailable'::text
                    ELSE 'available'::text
                END AS availability,
                CASE
                    WHEN (o.observation_kind = 'unavailable'::text) THEN 'none'::text
                    WHEN (((o.snapshot ->> 'complete'::text) = 'true'::text) AND (app.observed_remote_time(o.remote_updated_at) IS NOT NULL) AND (octet_length(COALESCE((o.snapshot ->> 'body_markdown'::text), ''::text)) <= 61440) AND (octet_length(COALESCE((o.snapshot ->> 'title'::text), j.title)) <= 4096)) THEN 'complete'::text
                    ELSE 'partial'::text
                END AS coverage,
                CASE
                    WHEN (o.observation_kind = 'unavailable'::text) THEN '[]'::jsonb
                    ELSE ((((jsonb_build_array('comments_not_read', 'attachments_not_read', 'related_objects_not_read') ||
                    CASE
                        WHEN (j.provider = 'notion'::text) THEN jsonb_build_array('properties_not_read', 'embedded_content_not_read', 'transcripts_not_read')
                        ELSE '[]'::jsonb
                    END) ||
                    CASE
                        WHEN (COALESCE((o.snapshot ->> 'complete'::text), 'false'::text) <> 'true'::text) THEN jsonb_build_array('provider_truncated')
                        ELSE '[]'::jsonb
                    END) ||
                    CASE
                        WHEN ((octet_length(COALESCE((o.snapshot ->> 'body_markdown'::text), ''::text)) > 61440) OR (octet_length(COALESCE((o.snapshot ->> 'title'::text), j.title)) > 4096)) THEN jsonb_build_array('local_text_limit')
                        ELSE '[]'::jsonb
                    END) ||
                    CASE
                        WHEN (app.observed_remote_time(o.remote_updated_at) IS NULL) THEN jsonb_build_array('remote_date_unavailable')
                        ELSE '[]'::jsonb
                    END)
                END AS omission_reasons,
            'publication-text-v1'::text AS projection_version,
            '{}'::jsonb AS metadata,
            o.connection_id,
            o.connection_revision
           FROM (app.publication_observations o
             JOIN app.publication_jobs j ON (((j.id = o.publication_job_id) AND (j.workspace_id = o.workspace_id))))
          WHERE (j.provider = ANY (ARRAY['notion'::text, 'linear'::text]))
        ), signatures AS (
         SELECT b.id,
            b.public_id,
            b.workspace_id,
            b.project_id,
            b.publication_job_id,
            b.publication_public_id,
            b.provider,
            b.object_kind,
            b.external_id,
            b.canonical_url,
            b.observed_at,
            b.remote_updated_at,
            b.title,
            b.body_markdown,
            b.availability,
            b.coverage,
            b.omission_reasons,
            b.projection_version,
            b.metadata,
            b.connection_id,
            b.connection_revision,
            jsonb_build_object('title', b.title, 'body_markdown', b.body_markdown) AS business,
            jsonb_build_object('provider', b.provider, 'external_id', b.external_id, 'canonical_url', b.canonical_url, 'availability', b.availability, 'coverage', b.coverage, 'omission_reasons', b.omission_reasons, 'projection_version', b.projection_version, 'metadata', b.metadata) AS evidence
           FROM base b
        ), boundaries AS (
         SELECT s.id,
            s.public_id,
            s.workspace_id,
            s.project_id,
            s.publication_job_id,
            s.publication_public_id,
            s.provider,
            s.object_kind,
            s.external_id,
            s.canonical_url,
            s.observed_at,
            s.remote_updated_at,
            s.title,
            s.body_markdown,
            s.availability,
            s.coverage,
            s.omission_reasons,
            s.projection_version,
            s.metadata,
            s.connection_id,
            s.connection_revision,
            s.business,
            s.evidence,
                CASE
                    WHEN (NOT (lag((s.business || s.evidence)) OVER (PARTITION BY s.publication_job_id ORDER BY s.id) IS DISTINCT FROM (s.business || s.evidence))) THEN 0
                    ELSE 1
                END AS boundary
           FROM signatures s
        ), groups AS (
         SELECT b.id,
            b.public_id,
            b.workspace_id,
            b.project_id,
            b.publication_job_id,
            b.publication_public_id,
            b.provider,
            b.object_kind,
            b.external_id,
            b.canonical_url,
            b.observed_at,
            b.remote_updated_at,
            b.title,
            b.body_markdown,
            b.availability,
            b.coverage,
            b.omission_reasons,
            b.projection_version,
            b.metadata,
            b.connection_id,
            b.connection_revision,
            b.business,
            b.evidence,
            b.boundary,
            (sum(b.boundary) OVER (PARTITION BY b.publication_job_id ORDER BY b.id))::integer AS version
           FROM boundaries b
        ), identified AS (
         SELECT g.id,
            g.public_id,
            g.workspace_id,
            g.project_id,
            g.publication_job_id,
            g.publication_public_id,
            g.provider,
            g.object_kind,
            g.external_id,
            g.canonical_url,
            g.observed_at,
            g.remote_updated_at,
            g.title,
            g.body_markdown,
            g.availability,
            g.coverage,
            g.omission_reasons,
            g.projection_version,
            g.metadata,
            g.connection_id,
            g.connection_revision,
            g.business,
            g.evidence,
            g.boundary,
            g.version,
            first_value(g.id) OVER (PARTITION BY g.publication_job_id, g.version ORDER BY g.id) AS canonical_observation_id,
            first_value(g.public_id) OVER (PARTITION BY g.publication_job_id, g.version ORDER BY g.id) AS canonical_observation_public_id,
            max(g.id) OVER (PARTITION BY g.publication_job_id, g.version) AS group_latest_id,
            max(g.id) OVER (PARTITION BY g.publication_job_id) AS latest_observation_id
           FROM groups g
        )
 SELECT i.id,
    i.public_id,
    i.workspace_id,
    i.project_id,
    i.publication_job_id,
    i.publication_public_id,
    i.provider,
    i.object_kind,
    i.external_id,
    i.canonical_url,
    i.version,
    i.canonical_observation_id,
    i.canonical_observation_public_id,
    i.latest_observation_id,
    (i.group_latest_id = i.latest_observation_id) AS is_current,
    i.observed_at,
    i.remote_updated_at,
    i.title,
    i.body_markdown,
    i.availability,
    i.coverage,
    i.omission_reasons,
    encode(sha256(convert_to((i.business)::text, 'UTF8'::name)), 'hex'::text) AS content_hash,
    encode(sha256(convert_to(((i.business || i.evidence))::text, 'UTF8'::name)), 'hex'::text) AS snapshot_hash,
    i.projection_version,
    i.metadata,
    authority.connection_id AS authority_connection_id,
    c.public_id AS authority_connection_public_id,
    authority.connection_revision AS authority_connection_revision
   FROM ((identified i
     JOIN app.publication_observations authority ON ((authority.id = i.group_latest_id)))
     LEFT JOIN app.work_tool_connections c ON (((c.id = authority.connection_id) AND (c.workspace_id = i.workspace_id))));




-- Reviewed ACL and caller-RLS options omitted by legacy diff.
revoke all on function app.observed_remote_time(text) from public,anon,authenticated,service_role;
grant execute on function app.observed_remote_time(text) to ai_center_runtime;
alter view app.publication_source_observations set (security_invoker=true);
