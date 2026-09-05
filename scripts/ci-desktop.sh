#!/usr/bin/env bash
set -Eeuo pipefail

alpha_script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
alpha_repo_dir="$(cd -- "${alpha_script_dir}/.." && pwd)"

cd -- "${alpha_repo_dir}"
source "${alpha_script_dir}/integration-common.sh"

usage() {
  cat <<'EOF'
AI Center — contrôles CI desktop-only

Usage:
  ./scripts/ci-desktop.sh quality
  ./scripts/ci-desktop.sh integration
  ./scripts/ci-desktop.sh backup-restore
  ./scripts/ci-desktop.sh auth-smoke
  ./scripts/ci-desktop.sh e2e
  ./scripts/ci-desktop.sh real-e2e
  ./scripts/ci-desktop.sh desktop
  ./scripts/ci-desktop.sh secret-scan
  ./scripts/integration-stack.sh run

Les commandes Android, iOS et mobile sont volontairement absentes.
Les évaluations IA réelles ne sont jamais lancées par ce script.
EOF
}

require_command() {
  if ! command -v "$1" >/dev/null 2>&1; then
    printf 'Commande requise absente: %s\n' "$1" >&2
    exit 1
  fi
}

quality() {
  require_command npm
  require_command cargo
  require_command python3

  cargo fmt --all -- --check
  python3 scripts/check-markdown-links.py README.md docs specs
  npx --no-install prettier --check \
    .github/workflows docs/architecture specs/alpha-context-proof
  npm run lint -w @ai-center/web
  cargo clippy --workspace --all-targets -- -D warnings
  npm run test -w @ai-center/web
  cargo test --workspace --lib
  python3 -m py_compile scripts/alpha-eval.py scripts/alpha-live-eval.py
  python3 -m unittest discover -s scripts/tests -p 'test_alpha*.py'
  python3 -m unittest discover -s scripts/tests -p 'test_integration_target.py'
  npm run build:web
  cargo build -p ai-center-server
}

integration() {
  integration_require_stack
  require_command npm
  require_command cargo
  require_command psql

  : "${AI_CENTER_ADMIN_DATABASE_URL:?AI_CENTER_ADMIN_DATABASE_URL doit viser la base CI locale avec le rôle de migration}"
  : "${AI_CENTER_RUNTIME_DATABASE_URL:?AI_CENTER_RUNTIME_DATABASE_URL doit viser la base CI locale avec ai_center_runtime}"
  : "${AI_CENTER_RUNTIME_DB_PASSWORD:?AI_CENTER_RUNTIME_DB_PASSWORD doit définir le mot de passe local éphémère du rôle runtime}"
  if [[ "${AI_CENTER_AGENT_MODE:-deterministic}" != "deterministic" ]]; then
    printf 'Les tests d’intégration CI exigent AI_CENTER_AGENT_MODE=deterministic.\n' >&2
    exit 1
  fi

  ./scripts/baseline-alpha-upgrade-smoke.sh
  integration_supabase db reset --local --yes
  integration_supabase test db

  AI_CENTER_ADMIN_DATABASE_URL="${AI_CENTER_ADMIN_DATABASE_URL}" \
    ./scripts/runtime-db-role.sh verify
  AI_CENTER_ADMIN_DATABASE_URL="${AI_CENTER_ADMIN_DATABASE_URL}" \
    AI_CENTER_RUNTIME_DB_PASSWORD="${AI_CENTER_RUNTIME_DB_PASSWORD}" \
    ./scripts/runtime-db-role.sh apply

  local alpha_runtime_role
  alpha_runtime_role="$(
    psql --no-psqlrc --set=ON_ERROR_STOP=1 \
      --tuples-only --no-align \
      --dbname="${AI_CENTER_RUNTIME_DATABASE_URL}" \
      --command='select current_user'
  )"
  if [[ "${alpha_runtime_role}" != "ai_center_runtime" ]]; then
    printf 'La connexion d’intégration utilise le rôle inattendu: %s\n' \
      "${alpha_runtime_role}" >&2
    exit 1
  fi

  DATABASE_URL="${AI_CENTER_RUNTIME_DATABASE_URL}" \
    AI_CENTER_EXPECT_DATABASE_ROLE=ai_center_runtime \
    AI_CENTER_AGENT_MODE=deterministic \
    cargo test -p ai-center-server --test mvp_flow -- --test-threads=1
  DATABASE_URL="${AI_CENTER_RUNTIME_DATABASE_URL}" \
    AI_CENTER_ADMIN_DATABASE_URL="${AI_CENTER_ADMIN_DATABASE_URL}" \
    AI_CENTER_EXPECT_DATABASE_ROLE=ai_center_runtime \
    AI_CENTER_AGENT_MODE=deterministic \
    cargo test -p ai-center-server --test context_pack_concurrency -- --test-threads=1
  DATABASE_URL="${AI_CENTER_RUNTIME_DATABASE_URL}" \
    AI_CENTER_EXPECT_DATABASE_ROLE=ai_center_runtime \
    AI_CENTER_AGENT_MODE=deterministic \
    cargo test -p ai-center-server --test idempotency_atomicity -- --test-threads=1
  DATABASE_URL="${AI_CENTER_RUNTIME_DATABASE_URL}" \
    AI_CENTER_EXPECT_DATABASE_ROLE=ai_center_runtime \
    AI_CENTER_AGENT_MODE=deterministic \
    cargo test -p ai-center-server --test model_run_lifecycle -- --test-threads=1
  DATABASE_URL="${AI_CENTER_RUNTIME_DATABASE_URL}" \
    AI_CENTER_ADMIN_DATABASE_URL="${AI_CENTER_ADMIN_DATABASE_URL}" \
    AI_CENTER_EXPECT_DATABASE_ROLE=ai_center_runtime \
    AI_CENTER_AGENT_MODE=deterministic \
    cargo test -p ai-center-server --test targeted_invalidation -- --test-threads=1
  DATABASE_URL="${AI_CENTER_RUNTIME_DATABASE_URL}" \
    AI_CENTER_EXPECT_DATABASE_ROLE=ai_center_runtime \
    AI_CENTER_AGENT_MODE=deterministic \
    cargo test -p ai-center-server --lib \
      github_persistence_preserves_proofs_under_rate_limits_and_binds_idempotence_to_target \
      -- --ignored --test-threads=1
}

backup_restore() {
  : "${AI_CENTER_ADMIN_DATABASE_URL:?AI_CENTER_ADMIN_DATABASE_URL doit viser la base CI locale avec le rôle de migration}"
  ./scripts/postgres-backup-restore-smoke.sh
}

auth_smoke() {
  require_command npm
  require_command cargo
  require_command curl
  require_command python3

  : "${AI_CENTER_ADMIN_DATABASE_URL:?AI_CENTER_ADMIN_DATABASE_URL doit viser la base CI locale avec le rôle de migration}"
  : "${AI_CENTER_RUNTIME_DATABASE_URL:?AI_CENTER_RUNTIME_DATABASE_URL doit viser la base CI locale avec ai_center_runtime}"
  ./scripts/auth-magic-link-smoke.sh
}

e2e() {
  require_command npm
  if [[ -n "${AI_CENTER_PLAYWRIGHT_PROJECT:-}" ]]; then
    npm run test:e2e -w @ai-center/web -- \
      --project="${AI_CENTER_PLAYWRIGHT_PROJECT}"
  else
    npm run test:e2e -w @ai-center/web
  fi
}

real_e2e() {
  integration_require_stack
  require_command npm
  require_command cargo
  require_command curl

  : "${AI_CENTER_RUNTIME_DATABASE_URL:?AI_CENTER_RUNTIME_DATABASE_URL doit viser la base CI locale avec ai_center_runtime}"
  cargo build -p ai-center-server --bin ai-center-server

  # Le trap EXIT s'exécute après la sortie de cette fonction en cas d'échec.
  # Ces valeurs doivent donc survivre à la portée locale de real_e2e.
  declare -g alpha_real_e2e_api_log alpha_real_e2e_web_log
  declare -g alpha_real_e2e_api_pid alpha_real_e2e_web_pid
  alpha_real_e2e_api_log="$(mktemp)"
  alpha_real_e2e_web_log="$(mktemp)"
  alpha_real_e2e_api_pid=""
  alpha_real_e2e_web_pid=""
  cleanup_real_e2e() {
    if [[ -n "${alpha_real_e2e_web_pid:-}" ]]; then kill "${alpha_real_e2e_web_pid}" 2>/dev/null || true; fi
    if [[ -n "${alpha_real_e2e_api_pid:-}" ]]; then kill "${alpha_real_e2e_api_pid}" 2>/dev/null || true; fi
    wait "${alpha_real_e2e_web_pid:-}" 2>/dev/null || true
    wait "${alpha_real_e2e_api_pid:-}" 2>/dev/null || true
    rm -f -- "${alpha_real_e2e_api_log:-}" "${alpha_real_e2e_web_log:-}"
    alpha_real_e2e_api_log=""
    alpha_real_e2e_web_log=""
    alpha_real_e2e_api_pid=""
    alpha_real_e2e_web_pid=""
  }
  trap cleanup_real_e2e EXIT

  (cd -- "${AI_CENTER_INTEGRATION_WORKDIR}" && \
  exec env DATABASE_URL="${AI_CENTER_RUNTIME_DATABASE_URL}" \
    AI_CENTER_BIND=127.0.0.1:4617 \
    AI_CENTER_CORS_ORIGINS="${AI_CENTER_REAL_E2E_WEB_URL}" \
    AI_CENTER_AUTH_MODE=local \
    AI_CENTER_AGENT_MODE=deterministic \
    AI_CENTER_WORKSPACE_ID=10000000-0000-0000-0000-000000000001 \
    AI_CENTER_ACTOR_ID=00000000-0000-0000-0000-000000000001 \
    "${CARGO_TARGET_DIR:-${alpha_repo_dir}/target}/debug/ai-center-server") >"${alpha_real_e2e_api_log}" 2>&1 &
  alpha_real_e2e_api_pid=$!

  VITE_API_URL="${AI_CENTER_REAL_E2E_API_URL}" \
    VITE_WORKSPACE_ID=10000000-0000-0000-0000-000000000001 \
    VITE_SUPABASE_URL= VITE_SUPABASE_ANON_KEY= TAURI_DEV_HOST= \
    node "${alpha_repo_dir}/node_modules/vite/bin/vite.js" \
      --config apps/web/vite.integration.config.ts apps/web --host 127.0.0.1 --port 5183 --strictPort \
      >"${alpha_real_e2e_web_log}" 2>&1 &
  alpha_real_e2e_web_pid=$!

  local alpha_attempt
  for alpha_attempt in {1..60}; do
    if curl --fail --silent "${AI_CENTER_REAL_E2E_API_URL}/api/health" >/dev/null 2>&1 \
      && curl --fail --silent "${AI_CENTER_REAL_E2E_WEB_URL}/" >/dev/null 2>&1; then
      break
    fi
    if ! kill -0 "${alpha_real_e2e_api_pid}" 2>/dev/null || ! kill -0 "${alpha_real_e2e_web_pid}" 2>/dev/null; then
      printf 'Le serveur API ou Vite s’est arrêté avant le smoke réel.\n' >&2
      return 1
    fi
    sleep 1
  done
  if [[ "${alpha_attempt}" == 60 ]]; then
    printf 'Timeout au démarrage de la chaîne navigateur réelle.\n' >&2
    return 1
  fi

  npx playwright test \
    --config=apps/web/playwright.real.config.ts \
    --project=chromium-desktop-real-api
  cleanup_real_e2e
  trap - EXIT
}

desktop() {
  require_command npm
  require_command cargo

  npm run build:web
  npm run tauri -- build \
    --config apps/desktop/src-tauri/tauri.conf.json \
    --no-bundle
}

secret_scan() {
  require_command git

  # Les fragments empêchent le scanner de se détecter lui-même.
  local alpha_openai_prefix alpha_github_prefix alpha_github_fine_prefix
  local alpha_token_pattern alpha_private_key_pattern
  alpha_openai_prefix='s''k-'
  alpha_github_prefix='g''h[pousr]_'
  alpha_github_fine_prefix='github''_pat_'
  alpha_token_pattern="(${alpha_openai_prefix}(proj-)?[A-Za-z0-9_-]{20,}|${alpha_github_prefix}[A-Za-z0-9]{20,}|${alpha_github_fine_prefix}[A-Za-z0-9_]{20,})"
  alpha_private_key_pattern='BEGIN [A-Z0-9 ]*PRIVATE KEY'

  if git grep --untracked -nIE "${alpha_token_pattern}" -- . ':!package-lock.json'; then
    printf 'Secret potentiel détecté dans un fichier suivi.\n' >&2
    exit 1
  fi

  if git grep --untracked -nIE "${alpha_private_key_pattern}" -- . ':!scripts/ci-desktop.sh'; then
    printf 'Clé privée potentielle détectée dans un fichier suivi.\n' >&2
    exit 1
  fi
}

case "${1:-}" in
  quality)
    quality
    ;;
  integration)
    integration
    ;;
  backup-restore)
    backup_restore
    ;;
  auth-smoke)
    auth_smoke
    ;;
  e2e)
    e2e
    ;;
  real-e2e)
    real_e2e
    ;;
  desktop)
    desktop
    ;;
  secret-scan)
    secret_scan
    ;;
  help | --help | -h)
    usage
    ;;
  *)
    usage >&2
    exit 2
    ;;
esac
