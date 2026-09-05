#!/usr/bin/env bash
set -Eeuo pipefail
# Do not leak connection strings or the optional runtime password when a caller
# accidentally invokes this operational script through `bash -x`.
set +x

runtime_script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
runtime_repo_dir="$(cd -- "${runtime_script_dir}/.." && pwd)"

usage() {
  cat <<'EOF'
AI Center — rôle PostgreSQL d'exécution

Usage:
  AI_CENTER_ADMIN_DATABASE_URL=postgresql://... ./scripts/runtime-db-role.sh apply
  AI_CENTER_ADMIN_DATABASE_URL=postgresql://... ./scripts/runtime-db-role.sh verify

Variables privées optionnelles pour `apply` :
  AI_CENTER_RUNTIME_DB_PASSWORD  Définit/renouvelle le mot de passe du rôle.

Le mot de passe n'est ni stocké dans le dépôt ni passé en argument à psql.
EOF
}

require_psql() {
  if ! command -v psql >/dev/null 2>&1; then
    printf 'Commande requise absente: psql\n' >&2
    exit 1
  fi
}

require_admin_url() {
  if [[ -z "${AI_CENTER_ADMIN_DATABASE_URL:-}" ]]; then
    printf 'AI_CENTER_ADMIN_DATABASE_URL doit viser le rôle de migration.\n' >&2
    exit 1
  fi
}

run_psql() {
  psql \
    --no-psqlrc \
    --set=ON_ERROR_STOP=1 \
    --dbname="${AI_CENTER_ADMIN_DATABASE_URL}" \
    "$@"
}

apply_role() {
  run_psql \
    --single-transaction \
    --file="${runtime_repo_dir}/supabase/roles.sql" \
    --file="${runtime_repo_dir}/supabase/schemas/02_runtime_access.sql"

  if [[ -n "${AI_CENTER_RUNTIME_DB_PASSWORD:-}" ]]; then
    run_psql --quiet <<'SQL'
\getenv runtime_password AI_CENTER_RUNTIME_DB_PASSWORD
select pg_catalog.format(
  'alter role ai_center_runtime password %L',
  :'runtime_password'
) \gexec
SQL
  else
    printf '%s\n' \
      'Mot de passe inchangé; un rôle créé à neuf ne pourra pas se connecter avant configuration privée.'
  fi
}

verify_role() {
  run_psql --file="${runtime_repo_dir}/scripts/sql/verify-runtime-db-role.sql"
}

case "${1:-}" in
  apply)
    require_psql
    require_admin_url
    apply_role
    verify_role
    ;;
  verify)
    require_psql
    require_admin_url
    verify_role
    ;;
  help | --help | -h)
    usage
    ;;
  *)
    usage >&2
    exit 2
    ;;
esac
