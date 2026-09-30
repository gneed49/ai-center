#!/usr/bin/env bash
set -Eeuo pipefail

ai_center_script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
ai_center_repo_dir="$(cd -- "${ai_center_script_dir}/.." && pwd)"
ai_center_run_dir="${ai_center_repo_dir}/.run/ai-center"
ai_center_server_pid_file="${ai_center_run_dir}/server.pid"
ai_center_app_pid_file="${ai_center_run_dir}/app.pid"
ai_center_server_log="${ai_center_run_dir}/server.log"
ai_center_app_log="${ai_center_run_dir}/app.log"
ai_center_supabase_log="${ai_center_run_dir}/supabase.log"
ai_center_health_url="http://127.0.0.1:4317/api/health"
ai_center_agent_mode="deterministic"
ai_center_json=false
ai_center_verbose=false
ai_center_server_started=false
ai_center_app_started=false

export SUPABASE_TELEMETRY_DISABLED=1
export DO_NOT_TRACK=1

mkdir -p -- "${ai_center_run_dir}"
touch -- "${ai_center_server_log}" "${ai_center_app_log}" "${ai_center_supabase_log}"
cd -- "${ai_center_repo_dir}"

usage() {
  cat <<'EOF'
AI Center — environnement de développement local

Usage:
  ./dev [linux] [--agent deterministic|openai] [--verbose]
  ./dev doctor [--json]
  ./dev status [--json]
  ./dev logs [server|app|all]
  ./dev stop
  ./dev help

Sans sous-commande, ./dev équivaut à ./dev linux.

Commandes:
  linux   Démarre Supabase, l'API Rust et l'application Tauri Linux.
  doctor  Vérifie les prérequis sans modifier l'environnement.
  status  Affiche l'état de Supabase, de l'API et de l'application.
  logs    Suit les journaux locaux (all par défaut).
  stop    Arrête l'application, l'API et Supabase sans effacer les données.

Options:
  --agent deterministic  Moteur local reproductible (défaut).
  --agent openai         Moteur OpenAI configuré dans .env.local.
  --json                 Sortie structurée pour doctor et status.
  --verbose              Active les journaux détaillés de Tauri.
EOF
}

info() {
  if [[ "${ai_center_json}" != true ]]; then
    printf '[ai-center] %s\n' "$*"
  fi
}

fail() {
  printf '[ai-center] erreur: %s\n' "$*" >&2
  exit 1
}

command_exists() {
  command -v "$1" >/dev/null 2>&1
}

bool_json() {
  if "$@" >/dev/null 2>&1; then
    printf 'true'
  else
    printf 'false'
  fi
}

pid_is_running() {
  local ai_center_pid_file="$1"
  local ai_center_expected="${2:-}"
  local ai_center_pid
  local ai_center_cmdline

  [[ -f "${ai_center_pid_file}" ]] || return 1
  read -r ai_center_pid < "${ai_center_pid_file}" || return 1
  [[ "${ai_center_pid}" =~ ^[0-9]+$ ]] || return 1
  kill -0 "${ai_center_pid}" 2>/dev/null || return 1

  if [[ -n "${ai_center_expected}" ]]; then
    [[ -r "/proc/${ai_center_pid}/cmdline" ]] || return 1
    ai_center_cmdline="$(tr '\0' ' ' < "/proc/${ai_center_pid}/cmdline")"
    [[ "${ai_center_cmdline}" == *"${ai_center_expected}"* ]] || return 1
  fi
}

remove_stale_pid() {
  local ai_center_pid_file="$1"
  local ai_center_expected="${2:-}"
  if [[ -f "${ai_center_pid_file}" ]] && ! pid_is_running "${ai_center_pid_file}" "${ai_center_expected}"; then
    rm -f -- "${ai_center_pid_file}"
  fi
}

stop_managed_process() {
  local ai_center_name="$1"
  local ai_center_pid_file="$2"
  local ai_center_expected="$3"
  local ai_center_pid

  if ! pid_is_running "${ai_center_pid_file}" "${ai_center_expected}"; then
    rm -f -- "${ai_center_pid_file}"
    return 0
  fi

  read -r ai_center_pid < "${ai_center_pid_file}"
  info "Arrêt de ${ai_center_name} (PID ${ai_center_pid})…"
  kill -TERM -- "-${ai_center_pid}" 2>/dev/null || kill -TERM "${ai_center_pid}" 2>/dev/null || true

  for _ in {1..30}; do
    if ! kill -0 "${ai_center_pid}" 2>/dev/null; then
      rm -f -- "${ai_center_pid_file}"
      return 0
    fi
    sleep 0.2
  done

  kill -KILL -- "-${ai_center_pid}" 2>/dev/null || kill -KILL "${ai_center_pid}" 2>/dev/null || true
  rm -f -- "${ai_center_pid_file}"
}

server_is_healthy() {
  local ai_center_health
  ai_center_health="$(curl --fail --silent --show-error --max-time 2 "${ai_center_health_url}" 2>/dev/null)" || return 1
  [[ "${ai_center_health}" == *'"service":"ai-center-server"'* ]] &&
    [[ "${ai_center_health}" == *'"database":"connected"'* ]]
}

supabase_is_running() {
  [[ -x "${ai_center_repo_dir}/node_modules/.bin/supabase" ]] || return 1
  npx --no-install supabase status --output json >/dev/null 2>&1
}

doctor() {
  local ai_center_node ai_center_npm ai_center_cargo ai_center_rust
  local ai_center_docker ai_center_docker_daemon ai_center_curl ai_center_setsid
  local ai_center_pkg_config ai_center_gtk ai_center_webkit ai_center_supabase ai_center_tauri
  local ai_center_ok=true

  ai_center_node="$(bool_json command_exists node)"
  ai_center_npm="$(bool_json command_exists npm)"
  ai_center_cargo="$(bool_json command_exists cargo)"
  ai_center_rust="$(bool_json command_exists rustc)"
  ai_center_docker="$(bool_json command_exists docker)"
  ai_center_docker_daemon="$(bool_json docker info)"
  ai_center_curl="$(bool_json command_exists curl)"
  ai_center_setsid="$(bool_json command_exists setsid)"
  ai_center_pkg_config="$(bool_json command_exists pkg-config)"
  ai_center_gtk="$(bool_json pkg-config --exists gtk+-3.0)"
  ai_center_webkit="$(bool_json pkg-config --exists webkit2gtk-4.1)"
  ai_center_supabase="$(bool_json test -x "${ai_center_repo_dir}/node_modules/.bin/supabase")"
  ai_center_tauri="$(bool_json test -x "${ai_center_repo_dir}/node_modules/.bin/tauri")"

  for ai_center_value in \
    "${ai_center_node}" "${ai_center_npm}" "${ai_center_cargo}" "${ai_center_rust}" \
    "${ai_center_docker}" "${ai_center_docker_daemon}" "${ai_center_curl}" \
    "${ai_center_setsid}" "${ai_center_pkg_config}" "${ai_center_gtk}" \
    "${ai_center_webkit}" "${ai_center_supabase}" "${ai_center_tauri}"; do
    if [[ "${ai_center_value}" != true ]]; then
      ai_center_ok=false
    fi
  done

  if [[ "${ai_center_json}" == true ]]; then
    printf '{"ok":%s,"node":%s,"npm":%s,"cargo":%s,"rustc":%s,"docker":%s,"dockerDaemon":%s,"curl":%s,"setsid":%s,"pkgConfig":%s,"gtk3":%s,"webkitgtk41":%s,"supabaseCli":%s,"tauriCli":%s}\n' \
      "${ai_center_ok}" "${ai_center_node}" "${ai_center_npm}" "${ai_center_cargo}" \
      "${ai_center_rust}" "${ai_center_docker}" "${ai_center_docker_daemon}" \
      "${ai_center_curl}" "${ai_center_setsid}" "${ai_center_pkg_config}" \
      "${ai_center_gtk}" "${ai_center_webkit}" "${ai_center_supabase}" "${ai_center_tauri}"
  else
    printf 'Diagnostic AI Center\n'
    printf '  Node.js              %s\n' "${ai_center_node}"
    printf '  npm                  %s\n' "${ai_center_npm}"
    printf '  Cargo / Rust         %s / %s\n' "${ai_center_cargo}" "${ai_center_rust}"
    printf '  Docker / daemon      %s / %s\n' "${ai_center_docker}" "${ai_center_docker_daemon}"
    printf '  curl / setsid        %s / %s\n' "${ai_center_curl}" "${ai_center_setsid}"
    printf '  GTK 3 / WebKitGTK    %s / %s\n' "${ai_center_gtk}" "${ai_center_webkit}"
    printf '  Supabase / Tauri CLI %s / %s\n' "${ai_center_supabase}" "${ai_center_tauri}"
  fi

  [[ "${ai_center_ok}" == true ]]
}

require_core_tools() {
  local ai_center_missing=()
  local ai_center_tool

  for ai_center_tool in node npm cargo rustc docker curl setsid sha256sum; do
    if ! command_exists "${ai_center_tool}"; then
      ai_center_missing+=("${ai_center_tool}")
    fi
  done

  if (( ${#ai_center_missing[@]} > 0 )); then
    fail "prérequis manquants: ${ai_center_missing[*]}. Lancez ./dev doctor."
  fi

}

ensure_docker_daemon() {
  if docker info >/dev/null 2>&1; then
    return 0
  fi

  info "Le daemon Docker est arrêté; tentative de démarrage non interactif…"
  if [[ "$(id -u)" == 0 ]] && command_exists systemctl; then
    systemctl start docker >/dev/null 2>&1 || true
  elif command_exists sudo && command_exists systemctl; then
    sudo -n systemctl start docker >/dev/null 2>&1 || true
  fi

  for _ in {1..20}; do
    if docker info >/dev/null 2>&1; then
      info "Daemon Docker prêt."
      return 0
    fi
    sleep 0.5
  done

  fail "le daemon Docker est arrêté. Démarrez-le avec 'sudo systemctl start docker', puis relancez ./dev."
}

ensure_npm_dependencies() {
  local ai_center_lock_hash ai_center_saved_hash=""
  local ai_center_hash_file="${ai_center_run_dir}/package-lock.sha256"

  ai_center_lock_hash="$(sha256sum package-lock.json | awk '{print $1}')"
  if [[ -f "${ai_center_hash_file}" ]]; then
    read -r ai_center_saved_hash < "${ai_center_hash_file}" || true
  fi

  if [[ "${ai_center_saved_hash}" == "${ai_center_lock_hash}" ]] &&
    [[ -x node_modules/.bin/supabase ]] && [[ -x node_modules/.bin/tauri ]]; then
    info "Dépendances npm déjà à jour."
    return 0
  fi

  if [[ -x node_modules/.bin/supabase ]] && [[ -x node_modules/.bin/tauri ]] &&
    npm ls --depth=0 --silent >/dev/null 2>&1; then
    printf '%s\n' "${ai_center_lock_hash}" > "${ai_center_hash_file}"
    info "Dépendances npm existantes validées."
    return 0
  fi

  info "Installation reproductible des dépendances npm…"
  npm ci
  printf '%s\n' "${ai_center_lock_hash}" > "${ai_center_hash_file}"
}

ensure_local_env() {
  if [[ -f .env.local ]]; then
    info ".env.local existant conservé."
    return 0
  fi

  cp -- .env.example .env.local
  chmod 600 .env.local
  info ".env.local créé depuis .env.example (mode déterministe imposé par ./dev)."
}

start_supabase() {
  if supabase_is_running; then
    info "Supabase local déjà actif."
    return 0
  fi

  info "Démarrage de Supabase local…"
  if ! npx --no-install supabase start --yes > "${ai_center_supabase_log}" 2>&1; then
    printf '[ai-center] échec Supabase; consultez %s\n' "${ai_center_supabase_log}" >&2
    tail -n 30 "${ai_center_supabase_log}" >&2 || true
    return 1
  fi
  supabase_is_running || fail "Supabase n'est pas sain après son démarrage."
  info "Supabase prêt (PostgreSQL 127.0.0.1:54322)."
}

start_server() {
  remove_stale_pid "${ai_center_server_pid_file}" "dev:server"

  if server_is_healthy; then
    info "API AI Center déjà saine sur 127.0.0.1:4317."
    return 0
  fi

  if pid_is_running "${ai_center_server_pid_file}" "dev:server"; then
    fail "un serveur géré est actif mais ne répond pas; lancez ./dev stop puis réessayez."
  fi

  info "Démarrage du serveur Rust (agent: ${ai_center_agent_mode})…"
  setsid env AI_CENTER_AGENT_MODE="${ai_center_agent_mode}" \
    npm run dev:server >> "${ai_center_server_log}" 2>&1 &
  ai_center_server_pid=$!
  printf '%s\n' "${ai_center_server_pid}" > "${ai_center_server_pid_file}"
  ai_center_server_started=true

  for _ in {1..120}; do
    if server_is_healthy; then
      info "API prête: ${ai_center_health_url}"
      return 0
    fi
    if ! kill -0 "${ai_center_server_pid}" 2>/dev/null; then
      tail -n 50 "${ai_center_server_log}" >&2 || true
      fail "le serveur Rust s'est arrêté avant d'être prêt."
    fi
    sleep 0.5
  done

  tail -n 50 "${ai_center_server_log}" >&2 || true
  fail "délai dépassé en attendant l'API Rust."
}

cleanup_session() {
  local ai_center_exit_code=$?
  trap - EXIT INT TERM

  if [[ "${ai_center_app_started}" == true ]]; then
    stop_managed_process "l'application Tauri" "${ai_center_app_pid_file}" "dev:desktop"
  fi
  if [[ "${ai_center_server_started}" == true ]]; then
    stop_managed_process "le serveur Rust" "${ai_center_server_pid_file}" "dev:server"
  fi
  exit "${ai_center_exit_code}"
}

start_linux() {
  require_core_tools
  ensure_docker_daemon
  ensure_npm_dependencies
  ensure_local_env

  if ! doctor; then
    fail "diagnostic incomplet; corrigez les prérequis marqués false."
  fi

  trap cleanup_session EXIT INT TERM
  start_supabase
  start_server

  remove_stale_pid "${ai_center_app_pid_file}" "dev:desktop"
  if pid_is_running "${ai_center_app_pid_file}" "dev:desktop"; then
    fail "l'application Tauri est déjà gérée par un autre lancement."
  fi

  info "Lancement de l'application Tauri Linux…"
  info "Journaux: ${ai_center_run_dir}"

  local ai_center_tauri_args=()
  if [[ "${ai_center_verbose}" == true ]]; then
    ai_center_tauri_args+=(--verbose)
  fi

  setsid npm run dev:desktop -- "${ai_center_tauri_args[@]}" \
    > >(tee -a "${ai_center_app_log}") 2>&1 &
  ai_center_app_pid=$!
  printf '%s\n' "${ai_center_app_pid}" > "${ai_center_app_pid_file}"
  ai_center_app_started=true

  set +e
  wait "${ai_center_app_pid}"
  ai_center_app_exit=$?
  set -e
  rm -f -- "${ai_center_app_pid_file}"
  ai_center_app_started=false
  return "${ai_center_app_exit}"
}

show_status() {
  local ai_center_supabase ai_center_server ai_center_app
  ai_center_supabase="$(bool_json supabase_is_running)"
  ai_center_server="$(bool_json server_is_healthy)"
  ai_center_app="$(bool_json pid_is_running "${ai_center_app_pid_file}" "dev:desktop")"

  if [[ "${ai_center_json}" == true ]]; then
    printf '{"supabase":%s,"server":%s,"app":%s,"healthUrl":"%s"}\n' \
      "${ai_center_supabase}" "${ai_center_server}" "${ai_center_app}" "${ai_center_health_url}"
  else
    printf 'État AI Center\n'
    printf '  Supabase  %s\n' "${ai_center_supabase}"
    printf '  Serveur   %s\n' "${ai_center_server}"
    printf '  App Linux %s\n' "${ai_center_app}"
    printf '  Logs      %s\n' "${ai_center_run_dir}"
  fi
}

show_logs() {
  local ai_center_target="${1:-all}"
  case "${ai_center_target}" in
    server) tail -n 100 -F "${ai_center_server_log}" ;;
    app) tail -n 100 -F "${ai_center_app_log}" ;;
    all) tail -n 100 -F "${ai_center_server_log}" "${ai_center_app_log}" ;;
    *) fail "cible de logs inconnue: ${ai_center_target} (server, app ou all)." ;;
  esac
}

stop_all() {
  stop_managed_process "l'application Tauri" "${ai_center_app_pid_file}" "dev:desktop"
  stop_managed_process "le serveur Rust" "${ai_center_server_pid_file}" "dev:server"

  if supabase_is_running; then
    info "Arrêt de Supabase (les données sont conservées)…"
    npx --no-install supabase stop > "${ai_center_supabase_log}" 2>&1
  else
    info "Supabase est déjà arrêté."
  fi
  info "Environnement AI Center arrêté."
}

ai_center_command="linux"
if [[ $# -gt 0 && "$1" != -* ]]; then
  ai_center_command="$1"
  shift
fi

ai_center_log_target="all"
if [[ "${ai_center_command}" == logs && $# -gt 0 && "$1" != -* ]]; then
  ai_center_log_target="$1"
  shift
fi

while [[ $# -gt 0 ]]; do
  case "$1" in
    --agent)
      [[ $# -ge 2 ]] || fail "--agent attend deterministic ou openai."
      ai_center_agent_mode="$2"
      shift 2
      ;;
    --json)
      ai_center_json=true
      shift
      ;;
    --verbose)
      ai_center_verbose=true
      shift
      ;;
    -h|--help)
      ai_center_command="help"
      shift
      ;;
    *)
      fail "option inconnue: $1"
      ;;
  esac
done

case "${ai_center_agent_mode}" in
  deterministic|openai) ;;
  *) fail "mode agent inconnu: ${ai_center_agent_mode}" ;;
esac

case "${ai_center_command}" in
  linux) start_linux ;;
  doctor) doctor ;;
  status) show_status ;;
  logs) show_logs "${ai_center_log_target}" ;;
  stop) stop_all ;;
  help) usage ;;
  *) fail "commande inconnue: ${ai_center_command}. Lancez ./dev help." ;;
esac
