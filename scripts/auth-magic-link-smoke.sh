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

require_command cargo
require_command curl
require_command npx
require_command python3

: "${AI_CENTER_ADMIN_DATABASE_URL:?AI_CENTER_ADMIN_DATABASE_URL doit viser le PostgreSQL Supabase local avec le rôle de migration}"
: "${AI_CENTER_RUNTIME_DATABASE_URL:?AI_CENTER_RUNTIME_DATABASE_URL doit viser le PostgreSQL Supabase local avec ai_center_runtime}"

alpha_bind="${AI_CENTER_AUTH_SMOKE_BIND}"
if [[ ! "${alpha_bind}" =~ ^127\.0\.0\.1:([0-9]{2,5})$ ]]; then
  printf 'AI_CENTER_AUTH_SMOKE_BIND doit être une adresse loopback IPv4 explicite.\n' >&2
  exit 1
fi
alpha_api_url="http://${alpha_bind}"
alpha_workspace_id="10000000-0000-0000-0000-000000000001"
alpha_forged_workspace_id="ffffffff-ffff-4fff-8fff-ffffffffffff"
alpha_db_container="${AI_CENTER_SUPABASE_DB_CONTAINER}"
alpha_failure_stage="${AI_CENTER_AUTH_SMOKE_FAILURE_STAGE:-}"
if [[ -n "${alpha_failure_stage}" && "${alpha_failure_stage}" != "after-membership" ]]; then
  printf 'AI_CENTER_AUTH_SMOKE_FAILURE_STAGE accepte uniquement after-membership.\n' >&2
  exit 1
fi

alpha_status_output=""
alpha_supabase_url=""
alpha_anon_key=""
alpha_service_role_key=""
alpha_server_pid=""
alpha_server_log=""
alpha_actor_id=""
alpha_access_token=""
alpha_email_otp=""
alpha_http_body=""
alpha_http_status=""

run_admin_sql() {
  if command -v psql >/dev/null 2>&1; then
    psql --no-psqlrc --set=ON_ERROR_STOP=1 \
      --quiet --dbname="${AI_CENTER_ADMIN_DATABASE_URL}" "$@"
    return
  fi
  if command -v docker >/dev/null 2>&1; then
    docker exec -i "${alpha_db_container}" \
      psql --no-psqlrc --set=ON_ERROR_STOP=1 \
      --quiet --username=postgres --dbname=postgres "$@"
    return
  fi
  if command -v podman >/dev/null 2>&1; then
    podman exec -i "${alpha_db_container}" \
      psql --no-psqlrc --set=ON_ERROR_STOP=1 \
      --quiet --username=postgres --dbname=postgres "$@"
    return
  fi
  printf 'psql, docker ou podman est requis pour créer le membership Auth éphémère.\n' >&2
  return 1
}

auth_admin_request() {
  local alpha_method="$1"
  local alpha_path="$2"
  local alpha_payload="${3:-}"
  local alpha_response
  local alpha_args=(
    --silent
    --show-error
    --request "${alpha_method}"
    --header "apikey: ${alpha_service_role_key}"
    --header "Authorization: Bearer ${alpha_service_role_key}"
    --write-out $'\n%{http_code}'
  )
  if [[ -n "${alpha_payload}" ]]; then
    alpha_args+=(
      --header 'Content-Type: application/json'
      --data "${alpha_payload}"
    )
  fi
  alpha_response="$(curl "${alpha_args[@]}" "${alpha_supabase_url}/auth/v1${alpha_path}")"
  alpha_http_status="${alpha_response##*$'\n'}"
  alpha_http_body="${alpha_response%$'\n'*}"
}

auth_verify_otp() {
  local alpha_payload="$1"
  local alpha_response
  alpha_response="$(
    curl --silent --show-error \
      --request POST \
      --header "apikey: ${alpha_anon_key}" \
      --header 'Content-Type: application/json' \
      --data "${alpha_payload}" \
      --write-out $'\n%{http_code}' \
      "${alpha_supabase_url}/auth/v1/verify"
  )"
  alpha_http_status="${alpha_response##*$'\n'}"
  alpha_http_body="${alpha_response%$'\n'*}"
}

app_request() {
  local alpha_method="$1"
  local alpha_path="$2"
  local alpha_token="${3:-}"
  local alpha_workspace="${4:-}"
  local alpha_payload="${5:-}"
  local alpha_idempotency_key="${6:-}"
  local alpha_response
  local alpha_args=(
    --silent
    --show-error
    --request "${alpha_method}"
    --write-out $'\n%{http_code}'
  )
  if [[ -n "${alpha_token}" ]]; then
    alpha_args+=(--header "Authorization: Bearer ${alpha_token}")
  fi
  if [[ -n "${alpha_workspace}" ]]; then
    alpha_args+=(--header "X-AI-Center-Workspace-Id: ${alpha_workspace}")
  fi
  if [[ -n "${alpha_idempotency_key}" ]]; then
    alpha_args+=(--header "Idempotency-Key: ${alpha_idempotency_key}")
  fi
  if [[ -n "${alpha_payload}" ]]; then
    alpha_args+=(
      --header 'Content-Type: application/json'
      --data "${alpha_payload}"
    )
  fi
  alpha_response="$(curl "${alpha_args[@]}" "${alpha_api_url}${alpha_path}")"
  alpha_http_status="${alpha_response##*$'\n'}"
  alpha_http_body="${alpha_response%$'\n'*}"
}

expect_status() {
  local alpha_expected="$1"
  local alpha_label="$2"
  if [[ "${alpha_http_status}" != "${alpha_expected}" ]]; then
    printf '%s: statut attendu %s, reçu %s.\n' \
      "${alpha_label}" "${alpha_expected}" "${alpha_http_status}" >&2
    return 1
  fi
  printf 'OK  %-42s HTTP %s\n' "${alpha_label}" "${alpha_expected}"
}

cleanup() {
  local alpha_exit_code="$?"
  local alpha_cleanup_failed=0
  local alpha_delete_response=""
  local alpha_delete_status=""
  local alpha_remaining_memberships=""
  trap - EXIT INT TERM
  set +e
  if [[ -n "${alpha_actor_id}" ]]; then
    if ! run_admin_sql \
      --set="actor_id=${alpha_actor_id}" \
      --set="workspace_public_id=${alpha_workspace_id}" \
      >/dev/null 2>&1 <<'SQL'
delete from app.workspace_members member
using app.workspaces workspace
where member.workspace_id = workspace.id
  and workspace.public_id = :'workspace_public_id'::uuid
  and member.actor_id = :'actor_id'::uuid;
SQL
    then
      printf 'Le nettoyage du membership Auth éphémère a échoué.\n' >&2
      alpha_cleanup_failed=1
    fi
    alpha_remaining_memberships="$(
      run_admin_sql \
        --tuples-only --no-align \
        --set="actor_id=${alpha_actor_id}" \
        --set="workspace_public_id=${alpha_workspace_id}" \
        2>/dev/null <<'SQL'
select count(*)
from app.workspace_members member
join app.workspaces workspace on workspace.id = member.workspace_id
where workspace.public_id = :'workspace_public_id'::uuid
  and member.actor_id = :'actor_id'::uuid;
SQL
    )"
    alpha_remaining_memberships="${alpha_remaining_memberships//[[:space:]]/}"
    if [[ "${alpha_remaining_memberships}" != "0" ]]; then
      printf 'Le membership Auth éphémère subsiste après nettoyage.\n' >&2
      alpha_cleanup_failed=1
    fi
    if [[ -n "${alpha_service_role_key}" && -n "${alpha_supabase_url}" ]]; then
      alpha_delete_response="$(
        curl --silent --show-error \
          --request DELETE \
          --header "apikey: ${alpha_service_role_key}" \
          --header "Authorization: Bearer ${alpha_service_role_key}" \
          --write-out $'\n%{http_code}' \
          "${alpha_supabase_url}/auth/v1/admin/users/${alpha_actor_id}" \
          2>/dev/null
      )"
      alpha_delete_status="${alpha_delete_response##*$'\n'}"
      if [[ ! "${alpha_delete_status}" =~ ^20[04]$ ]]; then
        printf 'Le nettoyage de l’utilisateur Auth éphémère a échoué.\n' >&2
        alpha_cleanup_failed=1
      fi
    fi
  fi
  if [[ -n "${alpha_server_pid}" ]]; then
    kill "${alpha_server_pid}" >/dev/null 2>&1
    wait "${alpha_server_pid}" >/dev/null 2>&1
  fi
  if [[ -n "${alpha_server_log}" ]]; then
    rm -f -- "${alpha_server_log}"
  fi
  unset alpha_status_output alpha_anon_key alpha_service_role_key
  unset alpha_access_token alpha_email_otp alpha_http_body
  if [[ "${alpha_exit_code}" == "0" && "${alpha_cleanup_failed}" != "0" ]]; then
    alpha_exit_code=1
  fi
  exit "${alpha_exit_code}"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

if ! alpha_status_output="$(integration_supabase status -o env 2>/dev/null)"; then
  printf 'Impossible de lire les identifiants éphémères de la stack Supabase locale.\n' >&2
  exit 1
fi
while IFS='=' read -r alpha_status_name alpha_status_value; do
  alpha_status_value="${alpha_status_value%\"}"
  alpha_status_value="${alpha_status_value#\"}"
  case "${alpha_status_name}" in
    API_URL) alpha_supabase_url="${alpha_status_value}" ;;
    ANON_KEY) alpha_anon_key="${alpha_status_value}" ;;
    SERVICE_ROLE_KEY) alpha_service_role_key="${alpha_status_value}" ;;
  esac
done <<< "${alpha_status_output}"
unset alpha_status_output alpha_status_value

case "${alpha_supabase_url}" in
  http://127.0.0.1:55321) ;;
  *)
    printf 'La CLI Supabase doit retourner l’API CI locale sur le port 55321.\n' >&2
    exit 1
    ;;
esac
if [[ -z "${alpha_anon_key}" || -z "${alpha_service_role_key}" ]]; then
  printf 'Les clés publiques et service_role de la stack locale sont absentes.\n' >&2
  exit 1
fi

cargo build -p ai-center-server --bin ai-center-server

alpha_server_log="$(mktemp -t ai-center-auth-smoke.XXXXXX.log)"
(cd -- "${AI_CENTER_INTEGRATION_WORKDIR}" && \
exec env DATABASE_URL="${AI_CENTER_RUNTIME_DATABASE_URL}" \
  AI_CENTER_BIND="${alpha_bind}" \
  AI_CENTER_AGENT_MODE=deterministic \
  AI_CENTER_AUTH_MODE=supabase \
  SUPABASE_URL="${alpha_supabase_url}" \
  RUST_LOG=ai_center_server=warn,tower_http=warn \
  "${CARGO_TARGET_DIR:-${alpha_repo_dir}/target}/debug/ai-center-server") >"${alpha_server_log}" 2>&1 &
alpha_server_pid="$!"

alpha_server_ready=false
for ((alpha_attempt = 1; alpha_attempt <= 50; alpha_attempt += 1)); do
  if ! kill -0 "${alpha_server_pid}" >/dev/null 2>&1; then
    printf 'Le serveur Auth éphémère s’est arrêté pendant son démarrage.\n' >&2
    exit 1
  fi
  if curl --silent --show-error --fail "${alpha_api_url}/api/health" >/dev/null 2>&1; then
    alpha_server_ready=true
    break
  fi
  sleep 0.2
done
if [[ "${alpha_server_ready}" != "true" ]]; then
  printf 'Le serveur Auth éphémère n’est pas devenu prêt.\n' >&2
  exit 1
fi

alpha_email="ai-center-auth-smoke-$(python3 -c 'import secrets; print(secrets.token_hex(12))')@example.invalid"
alpha_create_user_payload="$(
  EMAIL="${alpha_email}" python3 -c \
    'import json, os; print(json.dumps({"email": os.environ["EMAIL"], "email_confirm": True}))'
)"
auth_admin_request POST /admin/users "${alpha_create_user_payload}"
expect_status 200 'création utilisateur Auth éphémère'
alpha_actor_id="$(
  python3 -c \
    'import json, sys, uuid; value=json.load(sys.stdin)["id"]; uuid.UUID(value); print(value)' \
    <<< "${alpha_http_body}"
)"

run_admin_sql \
  --set="actor_id=${alpha_actor_id}" \
  --set="workspace_public_id=${alpha_workspace_id}" \
  >/dev/null <<'SQL'
insert into app.workspace_members (
  workspace_id,
  actor_id,
  role,
  invitation_status,
  invited_by_actor_id,
  accepted_at
)
select
  workspace.id,
  :'actor_id'::uuid,
  'viewer',
  'accepted',
  workspace.owner_actor_id,
  now()
from app.workspaces workspace
where workspace.public_id = :'workspace_public_id'::uuid
on conflict (workspace_id, actor_id) do update
set role = excluded.role,
    invitation_status = excluded.invitation_status,
    invited_by_actor_id = excluded.invited_by_actor_id,
    accepted_at = excluded.accepted_at,
    updated_at = now();
SQL

if [[ "${alpha_failure_stage}" == "after-membership" ]]; then
  printf 'Échec contrôlé après création du membership; validation du trap de nettoyage.\n' >&2
  exit 97
fi

alpha_generate_link_payload="$(
  EMAIL="${alpha_email}" python3 -c \
    'import json, os; print(json.dumps({"type": "magiclink", "email": os.environ["EMAIL"]}))'
)"
auth_admin_request POST /admin/generate_link "${alpha_generate_link_payload}"
expect_status 200 'generateLink magic-link réel'
alpha_email_otp="$(
  python3 -c \
    'import json, sys; value=json.load(sys.stdin); print(value.get("email_otp") or value.get("properties", {}).get("email_otp") or "")' \
    <<< "${alpha_http_body}"
)"
if [[ -z "${alpha_email_otp}" ]]; then
  printf 'generateLink n’a pas retourné d’OTP vérifiable.\n' >&2
  exit 1
fi

alpha_verify_payload="$(
  EMAIL="${alpha_email}" OTP="${alpha_email_otp}" python3 -c \
    'import json, os; print(json.dumps({"type": "magiclink", "email": os.environ["EMAIL"], "token": os.environ["OTP"]}))'
)"
auth_verify_otp "${alpha_verify_payload}"
expect_status 200 'verifyOtp magic-link réel'
alpha_access_token="$(
  python3 -c \
    'import json, sys; value=json.load(sys.stdin)["access_token"]; assert value; print(value)' \
    <<< "${alpha_http_body}"
)"
unset alpha_email_otp alpha_verify_payload

app_request GET /api/workspaces "${alpha_access_token}"
expect_status 200 'GET /api/workspaces authentifié'
WORKSPACE_ID="${alpha_workspace_id}" python3 -c '
import json, os, sys
workspaces = json.load(sys.stdin)
matches = [item for item in workspaces if item.get("public_id") == os.environ["WORKSPACE_ID"]]
if len(matches) != 1 or matches[0].get("role") != "viewer":
    raise SystemExit("le workspace seed avec rôle viewer est absent")
' <<< "${alpha_http_body}"
printf 'OK  %-42s rôle viewer\n' 'membership retourné par /api/workspaces'

app_request GET /api/projects "${alpha_access_token}" "${alpha_workspace_id}"
expect_status 200 'GET protégé autorisé pour viewer'

alpha_idempotency_key="$(python3 -c 'import uuid; print(uuid.uuid4())')"
app_request POST /api/projects "${alpha_access_token}" "${alpha_workspace_id}" \
  '{"name":"interdit au viewer","objective":"smoke Auth"}' \
  "${alpha_idempotency_key}"
expect_status 403 'mutation viewer refusée'

app_request GET /api/projects "${alpha_access_token}" "${alpha_forged_workspace_id}"
expect_status 403 'workspace forgé refusé'

app_request GET /api/workspaces
expect_status 401 'absence de bearer refusée'

printf 'Certification Supabase magic-link desktop réussie; nettoyage Auth et membership en cours.\n'
