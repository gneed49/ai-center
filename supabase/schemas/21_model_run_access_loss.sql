-- Durable ownership for narrow cancellation after loss of normal RLS access.
-- Existing rows stay NULL: migration must never invent a starting actor.
alter table app.model_runs add column started_by_actor_id uuid;
alter table app.model_runs alter column started_by_actor_id set default app.current_actor_id();

create policy model_runs_starting_actor_insert on app.model_runs as restrictive for insert
 with check (started_by_actor_id=(select app.current_actor_id()));

create function app.prevent_model_run_actor_change()
returns trigger language plpgsql security invoker set search_path='' as $$
begin
 if new.started_by_actor_id is distinct from old.started_by_actor_id then
  raise exception 'Model run starting actor is immutable' using errcode='23514';
 end if;
 return new;
end;
$$;
create trigger model_runs_starting_actor_immutable before update of started_by_actor_id on app.model_runs
 for each row execute function app.prevent_model_run_actor_change();
revoke all on function app.prevent_model_run_actor_change() from public,anon,authenticated,service_role,ai_center_runtime;

-- Authenticated server GUCs identify the caller; normal model data is never read
-- back to that caller. This is not a general cancellation or impersonation API.
-- Even a writer-to-writer role change invalidates the captured request authority.
create function app.cancel_model_run_after_access_loss(requested_run_id bigint)
returns boolean language plpgsql security definer set search_path='' set row_security=off as $$
declare actor uuid=app.current_actor_id(); workspace bigint=app.current_workspace_id();
begin
 if actor is null or actor='00000000-0000-0000-0000-000000000000'::uuid or workspace is null then
  return false;
 end if;
 update app.model_runs set status='cancelled',completed_at=clock_timestamp(),
   error_class='access_changed',error_message='Stopped after the starting actor workspace authorization changed.'
  where id=requested_run_id and workspace_id=workspace and started_by_actor_id=actor and status='running'
    and not exists(select 1 from app.workspace_members m where m.workspace_id=workspace
      and m.actor_id=actor and m.invitation_status='accepted' and m.role in ('owner','editor')
      and m.role=app.current_workspace_role());
 return found;
end;
$$;
revoke all on function app.cancel_model_run_after_access_loss(bigint) from public,anon,authenticated,service_role;
grant execute on function app.cancel_model_run_after_access_loss(bigint) to ai_center_runtime;
