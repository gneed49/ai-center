-- The default -1 must preserve identities, marker, receipt, observation and content.
do $$
declare baseline app.audit_events; job app.publication_jobs; observation app.publication_observations;
begin
  select * into strict baseline from app.audit_events where action='fixture.upgrade.publication';
  select * into strict job from app.publication_jobs where public_id=baseline.object_public_id;
  select * into strict observation from app.publication_observations where publication_job_id=job.id;
  if job.source_ticket_index<>-1 or (to_jsonb(job)-'source_ticket_index')<>baseline.after_state->'job'
    or to_jsonb(observation)<>baseline.after_state->'observation' then
    raise exception 'Ticket migration changed a legacy publication or receipt';
  end if;
  if has_function_privilege('authenticated','app.publication_validate_source_ticket()','EXECUTE')
    or has_function_privilege('anon','app.publication_validate_source_ticket()','EXECUTE') then
    raise exception 'Publication trigger function must remain private';
  end if;
end;
$$;
