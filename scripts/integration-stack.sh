#!/usr/bin/env bash
set -Eeuo pipefail
set +x
umask 077

integration_script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
source "${integration_script_dir}/integration-common.sh"
cd -- "${integration_repo_dir}"

lock_integration() {
  integration_environment
  mkdir -p -- "${integration_repo_dir}/.run"
  exec 9>>"${integration_repo_dir}/.run/integration-stack.lock"
  flock -n 9 || { printf 'Cette stack CI est déjà utilisée.\n' >&2; exit 1; }
}

case "${1:-help}" in
  prepare)
    lock_integration
    python3 "${integration_target_tool}" prepare
    printf 'Stack CI préparée : %s ; PostgreSQL 55322, Supabase 55321, API 4617, web 5183.\n' "${integration_project_id}"
    ;;
  stop)
    lock_integration
    integration_stop
    ;;
  run)
    shift
    if (( $# == 0 )); then set -- integration backup-restore auth-smoke real-e2e; fi
    for integration_phase in "$@"; do
      case "${integration_phase}" in
        integration | backup-restore | auth-smoke | real-e2e | native-e2e) ;;
        *) printf 'Phase CI inconnue.\n' >&2; exit 2 ;;
      esac
    done
    lock_integration
    python3 "${integration_target_tool}" prepare
    export AI_CENTER_RUNTIME_DB_PASSWORD="$(python3 -c 'import secrets; print(secrets.token_hex(24))')"
    export AI_CENTER_ADMIN_DATABASE_URL=postgresql://postgres:postgres@127.0.0.1:55322/postgres
    export AI_CENTER_RUNTIME_DATABASE_URL="postgresql://ai_center_runtime:${AI_CENTER_RUNTIME_DB_PASSWORD}@127.0.0.1:55322/postgres"
    integration_log="${AI_CENTER_INTEGRATION_WORKDIR}/startup.log"
    umask 077
    integration_started=0
    cleanup_integration() {
      local integration_exit=$?
      trap - EXIT INT TERM
      if [[ "${integration_started}" == 1 ]]; then
        if ! integration_stop \
          >"${AI_CENTER_INTEGRATION_WORKDIR}/teardown.log" 2>&1; then
          printf 'Nettoyage CI incomplet ; consulter le journal privé de teardown.\n' >&2
          integration_exit=1
        fi
      fi
      exit "${integration_exit}"
    }
    trap cleanup_integration EXIT
    trap 'exit 130' INT
    trap 'exit 143' TERM
    # Refuse an occupied port before starting, so another checkout/service cannot
    # become the target of a later SQL command or an already-running web server.
    python3 - <<'PY'
import socket
sockets = []
try:
    for port in (55320, 55321, 55322, 55323, 55324, 55327, 55329, 4617, 4618, 5183):
        handle = socket.socket()
        handle.bind(("127.0.0.1", port))
        sockets.append(handle)
except OSError:
    raise SystemExit("Un port de la stack CI est occupé ; aucune stack existante ne sera arrêtée.")
finally:
    for handle in sockets:
        handle.close()
PY
    integration_started=1
    if ! integration_supabase start --yes >"${integration_log}" 2>&1; then
      printf 'Démarrage CI échoué ; diagnostic privé : %s\n' "${integration_log}" >&2
      exit 1
    fi
    integration_require_stack
    for integration_phase in "$@"; do
      ./scripts/ci-desktop.sh "${integration_phase}"
    done
    ;;
  *)
    printf '%s\n' \
      'Usage: ./scripts/integration-stack.sh prepare|run [integration backup-restore auth-smoke real-e2e native-e2e]|stop' \
      'run prépare une stack jetable, exécute les contrôles puis supprime ses seuls volumes.'
    ;;
esac
