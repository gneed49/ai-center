alter table "app"."external_reference_observations" drop constraint "external_reference_observations_reference_hash_unique";

drop index if exists "app"."external_reference_observations_reference_hash_unique";

drop index if exists "app"."external_reference_observations_reference_observed_idx";

alter table "app"."idempotency_records" add column "lease_generation" uuid not null default gen_random_uuid();

CREATE INDEX external_reference_observations_reference_observed_idx ON app.external_reference_observations USING btree (external_reference_id, id DESC);
