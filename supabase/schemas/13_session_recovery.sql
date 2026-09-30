-- Legacy rows intentionally keep an unknown author and no recoverable command.
alter table app.messages add column author_actor_id uuid;
alter table app.messages add column command_public_id uuid;
alter table app.messages add column submitted_content text;
alter table app.messages add constraint messages_user_author_valid
  check(author_actor_id is null or role='user');
alter table app.messages add constraint messages_recovery_fields_valid
  check(command_public_id is null or
    (role='user' and author_actor_id is not null and client_message_id is not null and submitted_content is not null));
alter table app.idempotency_records add constraint idempotency_message_scope_unique
  unique(public_id,workspace_id,project_id,actor_id);
alter table app.messages add constraint messages_command_scope_fkey
  foreign key(command_public_id,workspace_id,project_id,author_actor_id)
  references app.idempotency_records(public_id,workspace_id,project_id,actor_id)
  on delete set null(command_public_id);
create index messages_author_session_idx on app.messages(session_id,author_actor_id,id desc)
  where role='user' and command_public_id is not null;
create index messages_command_scope_idx on app.messages(command_public_id,workspace_id,project_id,author_actor_id)
  where command_public_id is not null;

-- This restrictive policy composes with existing tenant/editor checks.
-- Server-only legacy fixtures may omit authors; a claimed author must be real.
create policy messages_author_insert on app.messages as restrictive for insert
  with check(author_actor_id is null or author_actor_id=(select app.current_actor_id()));
