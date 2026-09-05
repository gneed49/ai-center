-- Derived from supabase/schemas/02_runtime_access.sql (ACP-T08).
-- External tracking describes read-only GitHub observations; no runner.
grant select, insert on table app.tasks, app.executions, app.execution_events, app.artifacts to ai_center_runtime;
grant update on table app.tasks, app.executions to ai_center_runtime;
grant usage on sequence app.tasks_id_seq, app.executions_id_seq, app.execution_events_id_seq, app.artifacts_id_seq to ai_center_runtime;
