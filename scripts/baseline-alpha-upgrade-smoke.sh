#!/usr/bin/env bash
set -Eeuo pipefail
set +x

alpha_script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
alpha_repo_dir="$(cd -- "${alpha_script_dir}/.." && pwd)"

cd -- "${alpha_repo_dir}"
source "${alpha_script_dir}/integration-common.sh"
integration_require_stack

require_command() {
  if ! command -v "$1" >/dev/null 2>&1; then
    printf 'Commande requise absente: %s\n' "$1" >&2
    exit 1
  fi
}

require_command createdb
require_command dropdb
require_command psql

: "${AI_CENTER_ADMIN_DATABASE_URL:?AI_CENTER_ADMIN_DATABASE_URL doit viser le PostgreSQL Supabase local avec le rôle de migration}"

if [[ ! "${AI_CENTER_ADMIN_DATABASE_URL}" =~ ^postgresql://[^[:space:]]+@(127\.0\.0\.1|localhost):[0-9]{2,5}/postgres$ ]]; then
  printf 'La preuve baseline→alpha exige une base administrateur locale explicite nommée postgres.\n' >&2
  exit 1
fi

alpha_upgrade_database="ai_center_upgrade_${BASHPID}_${RANDOM}"
if [[ ! "${alpha_upgrade_database}" =~ ^ai_center_upgrade_[0-9]+_[0-9]+$ ]]; then
  printf 'Nom de base de test inattendu; abandon du smoke de migration.\n' >&2
  exit 1
fi
alpha_upgrade_database_url="${AI_CENTER_ADMIN_DATABASE_URL%/postgres}/${alpha_upgrade_database}"

cleanup() {
  local alpha_exit_code="$?"
  trap - EXIT INT TERM
  set +e
  if [[ "${alpha_upgrade_database}" =~ ^ai_center_upgrade_[0-9]+_[0-9]+$ ]]; then
    dropdb \
      --maintenance-db="${AI_CENTER_ADMIN_DATABASE_URL}" \
      --force \
      --if-exists \
      "${alpha_upgrade_database}" >/dev/null 2>&1
  fi
  exit "${alpha_exit_code}"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

createdb \
  --maintenance-db="${AI_CENTER_ADMIN_DATABASE_URL}" \
  --template=template0 \
  "${alpha_upgrade_database}"

psql --no-psqlrc --set=ON_ERROR_STOP=1 --quiet \
  --dbname="${alpha_upgrade_database_url}" <<'SQL'
create schema if not exists extensions;
create extension if not exists pgcrypto with schema extensions;
SQL

psql --no-psqlrc --set=ON_ERROR_STOP=1 --quiet \
  --dbname="${alpha_upgrade_database_url}" \
  --file=supabase/roles.sql
psql --no-psqlrc --set=ON_ERROR_STOP=1 --quiet \
  --dbname="${alpha_upgrade_database_url}" \
  --file=supabase/migrations/20260818003330_initial_domain.sql

# Reproduce the durable state that exists before Alpha: multiple workspaces,
# but no workspace_members table yet.
psql --no-psqlrc --set=ON_ERROR_STOP=1 --quiet \
  --dbname="${alpha_upgrade_database_url}" <<'SQL'
insert into app.workspaces (public_id, owner_actor_id, name)
values
  (
    '91000000-0000-0000-0000-000000000001',
    '92000000-0000-0000-0000-000000000001',
    'Legacy workspace A'
  ),
  (
    '91000000-0000-0000-0000-000000000002',
    '92000000-0000-0000-0000-000000000002',
    'Legacy workspace B'
  );
SQL

psql --no-psqlrc --set=ON_ERROR_STOP=1 --quiet \
  --dbname="${alpha_upgrade_database_url}" \
  --file=supabase/migrations/20260825121915_alpha_context_proof.sql

alpha_unrecoverable_workspace_count="$(
  psql --no-psqlrc --set=ON_ERROR_STOP=1 \
    --tuples-only --no-align \
    --dbname="${alpha_upgrade_database_url}" <<'SQL'
select count(*)
from app.workspaces workspace
where not exists (
  select 1
  from app.authorize_workspace_member(
    workspace.public_id,
    workspace.owner_actor_id
  ) authorized_membership
  where authorized_membership.workspace_id = workspace.id
    and authorized_membership.role = 'owner'
);
SQL
)"
alpha_unrecoverable_workspace_count="${alpha_unrecoverable_workspace_count//[[:space:]]/}"

if [[ "${alpha_unrecoverable_workspace_count}" != "0" ]]; then
  printf '%s workspace(s) legacy ne possèdent pas de membership owner accepté après migration.\n' \
    "${alpha_unrecoverable_workspace_count}" >&2
  exit 1
fi

printf 'Upgrade baseline→alpha vérifié: chaque workspace legacy conserve un owner accepté.\n'
