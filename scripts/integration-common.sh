#!/usr/bin/env bash
# Sourced only by desktop integration scripts. Never source a .env file.

integration_repo_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
integration_target_tool="${integration_repo_dir}/scripts/integration-target.py"

integration_environment() {
  python3 "${integration_target_tool}" environment || return
  export AI_CENTER_INTEGRATION_WORKDIR="${integration_repo_dir}/.run/integration-stack"
  export AI_CENTER_AGENT_MODE=deterministic
  export AI_CENTER_LOCAL_POSTGRES_PORT=55322
  export AI_CENTER_AUTH_SMOKE_BIND=127.0.0.1:4618
  export AI_CENTER_REAL_E2E_API_URL=http://127.0.0.1:4617
  export AI_CENTER_REAL_E2E_WEB_URL=http://127.0.0.1:5183
  # Deterministic tests never inherit an operator's live-provider credentials.
  unset OPENAI_API_KEY OPENAI_MODEL GITHUB_APP_ID GITHUB_APP_INSTALLATION_ID
  unset GITHUB_APP_PRIVATE_KEY_PATH
  export SUPABASE_TELEMETRY_DISABLED=1 DO_NOT_TRACK=1
  if [[ -n "${CARGO_TARGET_DIR:-}" ]]; then
    CARGO_TARGET_DIR="$(realpath -m -- "${CARGO_TARGET_DIR}")" || return
    export CARGO_TARGET_DIR
  fi
  integration_project_id="$(python3 "${integration_target_tool}" project-id)" || return
  export AI_CENTER_SUPABASE_DB_CONTAINER="supabase_db_${integration_project_id}"
}

integration_supabase() {
  python3 "${integration_target_tool}" guard || return
  # An explicit workdir overrides SUPABASE_WORKDIR and never loads the repo .env.
  (cd -- "${AI_CENTER_INTEGRATION_WORKDIR}" && \
    "${integration_repo_dir}/node_modules/.bin/supabase" \
      --workdir "${AI_CENTER_INTEGRATION_WORKDIR}" "$@")
}

integration_require_stack() {
  integration_environment || return
  python3 "${integration_target_tool}" guard || return
  integration_supabase status -o json 2>/dev/null \
    | python3 "${integration_target_tool}" status
}

integration_stop() {
  python3 "${integration_target_tool}" guard || return
  local integration_stop_log="${AI_CENTER_INTEGRATION_WORKDIR}/stop-cli.log"
  if integration_supabase stop --project-id "${integration_project_id}" --no-backup \
    >"${integration_stop_log}" 2>&1; then
    printf 'Stack CI arrêtée et volumes supprimés.\n'
    return 0
  fi
  python3 "${integration_target_tool}" podman-cleanup || return
  printf 'Stack CI arrêtée ; suppression ciblée des seuls volumes Podman confirmée.\n'
}
