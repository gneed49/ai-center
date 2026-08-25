#!/usr/bin/env bash
set -euo pipefail
set +x

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

required=(
  AI_CENTER_ADMIN_DATABASE_URL
  AI_CENTER_GITHUB_INSTALLATION_ID
  AI_CENTER_GITHUB_CONNECTION_STATUS
  AI_CENTER_WORKSPACE_ID
  AI_CENTER_ACTOR_ID
)
for variable in "${required[@]}"; do
  if [[ -z "${!variable:-}" ]]; then
    echo "Missing required environment variable: ${variable}" >&2
    exit 2
  fi
done

if [[ ! "${AI_CENTER_GITHUB_INSTALLATION_ID}" =~ ^[1-9][0-9]*$ ]]; then
  echo "AI_CENTER_GITHUB_INSTALLATION_ID must be a positive integer" >&2
  exit 2
fi
if [[ "${AI_CENTER_GITHUB_CONNECTION_STATUS}" != "pending" && "${AI_CENTER_GITHUB_CONNECTION_STATUS}" != "active" ]]; then
  echo "AI_CENTER_GITHUB_CONNECTION_STATUS must be pending or active" >&2
  exit 2
fi

display_name="${AI_CENTER_GITHUB_CONNECTION_NAME:-AI Center GitHub App}"
secret_reference="${AI_CENTER_GITHUB_SECRET_REFERENCE:-env:GITHUB_APP_PRIVATE_KEY_PATH}"

psql "${AI_CENTER_ADMIN_DATABASE_URL}" \
  --no-psqlrc \
  --set=ON_ERROR_STOP=1 \
  --set=workspace_public_id="${AI_CENTER_WORKSPACE_ID}" \
  --set=actor_id="${AI_CENTER_ACTOR_ID}" \
  --set=installation_id="${AI_CENTER_GITHUB_INSTALLATION_ID}" \
  --set=connection_status="${AI_CENTER_GITHUB_CONNECTION_STATUS}" \
  --set=display_name="${display_name}" \
  --set=secret_reference="${secret_reference}" \
  --file "${repo_root}/scripts/sql/provision-github-connection.sql"
