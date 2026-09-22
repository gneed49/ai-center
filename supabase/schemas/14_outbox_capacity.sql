-- Claims remain monotonic fencing tokens. Deferring admission because of a
-- company quota must not consume the independent processing-failure budget.
alter table app.domain_events add column deferred_count integer not null default 0;
alter table app.domain_events add constraint domain_events_deferred_count_valid
  check(deferred_count>=0 and deferred_count<=attempt_count);
