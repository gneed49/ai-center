#!/usr/bin/env bash
set -Eeuo pipefail
set +x

export LC_ALL=C
export PGCONNECT_TIMEOUT="${PGCONNECT_TIMEOUT:-5}"

require_command() {
  if ! command -v "$1" >/dev/null 2>&1; then
    printf 'Commande requise absente pour le smoke backup/restore: %s\n' "$1" >&2
    exit 1
  fi
}

: "${AI_CENTER_ADMIN_DATABASE_URL:?AI_CENTER_ADMIN_DATABASE_URL doit viser la base Supabase locale isolée}"
: "${AI_CENTER_LOCAL_POSTGRES_PORT:=54322}"

if [[ ! "${AI_CENTER_LOCAL_POSTGRES_PORT}" =~ ^[0-9]{4,5}$ ]] \
  || (( AI_CENTER_LOCAL_POSTGRES_PORT < 1024 || AI_CENTER_LOCAL_POSTGRES_PORT > 65535 )); then
  printf '%s\n' \
    'Refus: AI_CENTER_LOCAL_POSTGRES_PORT doit être un port local non privilégié valide.' >&2
  exit 1
fi

# La cible est figée avant toute commande destructive. Les caractères qui
# pourraient changer l'interprétation de l'autorité URI sont interdits dans les
# credentials. `localhost` est ensuite réécrit en IPv4 loopback afin de ne pas
# dépendre d'une résolution DNS ou /etc/hosts.
if [[ ! "${AI_CENTER_ADMIN_DATABASE_URL}" =~ ^postgresql://[A-Za-z0-9_.~-]+:[A-Za-z0-9_.~%-]+@(127\.0\.0\.1|localhost):([0-9]{4,5})/postgres$ ]] \
  || [[ "${BASH_REMATCH[2]:-}" != "${AI_CENTER_LOCAL_POSTGRES_PORT}" ]]; then
  printf '%s\n' \
    'Refus: le smoke backup/restore accepte uniquement PostgreSQL loopback sur le port local explicitement autorisé.' >&2
  exit 1
fi
alpha_admin_url="${AI_CENTER_ADMIN_DATABASE_URL}"
if [[ "${alpha_admin_url}" == *"@localhost:${AI_CENTER_LOCAL_POSTGRES_PORT}/postgres" ]]; then
  alpha_admin_url="${alpha_admin_url%@localhost:${AI_CENTER_LOCAL_POSTGRES_PORT}/postgres}@127.0.0.1:${AI_CENTER_LOCAL_POSTGRES_PORT}/postgres"
fi

for alpha_command in \
  cmp createdb diff dropdb mktemp pg_dump pg_restore psql sed sha256sum sort; do
  require_command "${alpha_command}"
done

alpha_source_database="$({
  psql --no-psqlrc --set=ON_ERROR_STOP=1 --tuples-only --no-align \
    --dbname="${alpha_admin_url}" \
    --command="select current_database()"
} 2>/dev/null)"
if [[ "${alpha_source_database}" != "postgres" ]]; then
  printf '%s\n' 'Refus: la connexion locale n’aboutit pas à la base PostgreSQL attendue.' >&2
  exit 1
fi

alpha_work_dir="$(mktemp -d "${TMPDIR:-/tmp}/ai-center-backup-restore.XXXXXXXXXX")"
alpha_suffix="${alpha_work_dir##*.}"
alpha_restore_database="ai_center_restore_${alpha_suffix,,}"
alpha_restore_database_created=0

validate_restore_database_name() {
  [[ "$1" =~ ^ai_center_restore_[a-z0-9]{10}$ ]]
}

if ! validate_restore_database_name "${alpha_restore_database}"; then
  printf '%s\n' 'Refus: le nom de base temporaire généré ne respecte pas le contrat de sûreté.' >&2
  rmdir -- "${alpha_work_dir}"
  exit 1
fi

alpha_archive="${alpha_work_dir}/app.dump"
alpha_source_counts="${alpha_work_dir}/source-counts.tsv"
alpha_restore_counts="${alpha_work_dir}/restore-counts.tsv"
alpha_source_objects="${alpha_work_dir}/source-objects.tsv"
alpha_restore_objects="${alpha_work_dir}/restore-objects.tsv"
alpha_source_data="${alpha_work_dir}/source-data.tsv"
alpha_restore_data="${alpha_work_dir}/restore-data.tsv"
alpha_source_schema="${alpha_work_dir}/source-schema.sql"
alpha_restore_schema="${alpha_work_dir}/restore-schema.sql"

cleanup() {
  local alpha_status=$?
  trap - EXIT INT TERM

  if [[ "${alpha_restore_database_created}" == "1" ]]; then
    if validate_restore_database_name "${alpha_restore_database}"; then
      if ! dropdb --if-exists --force \
        --maintenance-db="${alpha_admin_url}" \
        -- "${alpha_restore_database}" >/dev/null 2>&1; then
        printf '%s\n' \
          'Échec du nettoyage de la base temporaire backup/restore; intervention locale requise.' >&2
        alpha_status=1
      fi
    else
      printf '%s\n' 'Nettoyage refusé: le nom de base temporaire n’est plus valide.' >&2
      alpha_status=1
    fi
  fi

  rm -f -- \
    "${alpha_archive}" \
    "${alpha_source_counts}" "${alpha_restore_counts}" \
    "${alpha_source_objects}" "${alpha_restore_objects}" \
    "${alpha_source_data}" "${alpha_restore_data}" \
    "${alpha_source_schema}" "${alpha_restore_schema}"
  if ! rmdir -- "${alpha_work_dir}" 2>/dev/null; then
    printf '%s\n' 'Le répertoire temporaire du smoke n’a pas pu être supprimé proprement.' >&2
    alpha_status=1
  fi

  exit "${alpha_status}"
}

trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

database_exists="$(
  psql --no-psqlrc --set=ON_ERROR_STOP=1 --tuples-only --no-align \
    --dbname="${alpha_admin_url}" \
    --set=restore_database="${alpha_restore_database}" <<'SQL'
select exists(select 1 from pg_database where datname = :'restore_database');
SQL
)"
if [[ "${database_exists}" != "f" ]]; then
  printf '%s\n' 'Refus: la base temporaire aléatoire existe déjà.' >&2
  exit 1
fi

write_table_counts() {
  local alpha_url="$1"
  local alpha_output="$2"
  psql --no-psqlrc --set=ON_ERROR_STOP=1 --quiet --tuples-only --no-align \
    --dbname="${alpha_url}" >"${alpha_output}" <<'SQL'
select format(
  'select %L || E''\t'' || count(*)::text from %I.%I;',
  relation.relname,
  namespace.nspname,
  relation.relname
)
from pg_class relation
join pg_namespace namespace on namespace.oid = relation.relnamespace
where namespace.nspname = 'app'
  and relation.relkind in ('r', 'p')
order by relation.relname
\gexec
SQL
  sort -o "${alpha_output}" "${alpha_output}"
}

write_object_inventory() {
  local alpha_url="$1"
  local alpha_output="$2"
  psql --no-psqlrc --set=ON_ERROR_STOP=1 --quiet --tuples-only --no-align \
    --field-separator=$'\t' --dbname="${alpha_url}" >"${alpha_output}" <<'SQL'
select 'schema', namespace.nspname, pg_get_userbyid(namespace.nspowner),
       coalesce(
         (
           select string_agg(acl_item::text, ',' order by acl_item::text)
           from unnest(namespace.nspacl) acl_item
         ),
         ''
       )
from pg_namespace namespace
where namespace.nspname = 'app';

select 'relation', relation.relname, relation.relkind::text,
       relation.relrowsecurity::text, relation.relforcerowsecurity::text,
       coalesce(
         (
           select string_agg(acl_item::text, ',' order by acl_item::text)
           from unnest(relation.relacl) acl_item
         ),
         ''
       )
from pg_class relation
join pg_namespace namespace on namespace.oid = relation.relnamespace
where namespace.nspname = 'app';

select 'column', relation.relname, attribute.attnum::text, attribute.attname,
       pg_catalog.format_type(attribute.atttypid, attribute.atttypmod),
       attribute.attnotnull::text, attribute.attidentity::text,
       attribute.attgenerated::text,
       coalesce(pg_get_expr(default_value.adbin, default_value.adrelid), '')
from pg_attribute attribute
join pg_class relation on relation.oid = attribute.attrelid
join pg_namespace namespace on namespace.oid = relation.relnamespace
left join pg_attrdef default_value
  on default_value.adrelid = attribute.attrelid
 and default_value.adnum = attribute.attnum
where namespace.nspname = 'app'
  and attribute.attnum > 0
  and not attribute.attisdropped;

select 'constraint', relation.relname, constraint_value.conname,
       constraint_value.contype::text,
       pg_get_constraintdef(constraint_value.oid, true)
from pg_constraint constraint_value
join pg_class relation on relation.oid = constraint_value.conrelid
join pg_namespace namespace on namespace.oid = relation.relnamespace
where namespace.nspname = 'app';

select 'index', table_relation.relname, index_relation.relname,
       pg_get_indexdef(index_relation.oid)
from pg_index index_value
join pg_class table_relation on table_relation.oid = index_value.indrelid
join pg_class index_relation on index_relation.oid = index_value.indexrelid
join pg_namespace namespace on namespace.oid = table_relation.relnamespace
where namespace.nspname = 'app';

select 'policy', policy.tablename, policy.policyname, policy.permissive,
       coalesce(
         (
           select string_agg(role_name::text, ',' order by role_name::text)
           from unnest(policy.roles) role_name
         ),
         ''
       ),
       policy.cmd, coalesce(policy.qual, ''),
       coalesce(policy.with_check, '')
from pg_policies policy
where policy.schemaname = 'app';

select 'function', procedure.proname,
       pg_get_function_identity_arguments(procedure.oid),
       pg_get_function_result(procedure.oid), language.lanname,
       procedure.prokind::text, procedure.provolatile::text,
       procedure.prosecdef::text, procedure.proleakproof::text,
       procedure.proparallel::text,
       coalesce(
         (
           select string_agg(acl_item::text, ',' order by acl_item::text)
           from unnest(procedure.proacl) acl_item
         ),
         ''
       )
from pg_proc procedure
join pg_namespace namespace on namespace.oid = procedure.pronamespace
join pg_language language on language.oid = procedure.prolang
where namespace.nspname = 'app';

select 'trigger', relation.relname, trigger_value.tgname,
       pg_get_triggerdef(trigger_value.oid, true)
from pg_trigger trigger_value
join pg_class relation on relation.oid = trigger_value.tgrelid
join pg_namespace namespace on namespace.oid = relation.relnamespace
where namespace.nspname = 'app'
  and not trigger_value.tgisinternal;

select 'enum', type_value.typname, enum_value.enumsortorder::text,
       enum_value.enumlabel
from pg_type type_value
join pg_namespace namespace on namespace.oid = type_value.typnamespace
join pg_enum enum_value on enum_value.enumtypid = type_value.oid
where namespace.nspname = 'app';
SQL
  sort -o "${alpha_output}" "${alpha_output}"
}

write_normalized_data() {
  local alpha_url="$1"
  local alpha_output="$2"
  psql --no-psqlrc --set=ON_ERROR_STOP=1 --quiet --tuples-only --no-align \
    --dbname="${alpha_url}" >"${alpha_output}" <<'SQL'
select format(
  'select %L || E''\trow\t'' || to_jsonb(alpha_row)::text from %I.%I alpha_row order by to_jsonb(alpha_row)::text;',
  relation.relname,
  namespace.nspname,
  relation.relname
)
from pg_class relation
join pg_namespace namespace on namespace.oid = relation.relnamespace
where namespace.nspname = 'app'
  and relation.relkind in ('r', 'p')
order by relation.relname
\gexec

select format(
  'select %L || E''\tsequence\t'' || last_value::text || E''\t'' || is_called::text from %I.%I;',
  relation.relname,
  namespace.nspname,
  relation.relname
)
from pg_class relation
join pg_namespace namespace on namespace.oid = relation.relnamespace
where namespace.nspname = 'app'
  and relation.relkind = 'S'
order by relation.relname
\gexec
SQL
  sort -o "${alpha_output}" "${alpha_output}"
}

write_normalized_schema() {
  local alpha_url="$1"
  local alpha_output="$2"
  pg_dump --schema-only --schema=app --no-owner --no-privileges \
    --dbname="${alpha_url}" \
    | sed \
      -e '/^-- Dumped from database version /d' \
      -e '/^-- Dumped by pg_dump version /d' \
      -e '/^\\restrict /d' \
      -e '/^\\unrestrict /d' \
      >"${alpha_output}"
}

schema_stats() {
  local alpha_url="$1"
  psql --no-psqlrc --set=ON_ERROR_STOP=1 --tuples-only --no-align \
    --field-separator=$'\t' --dbname="${alpha_url}" --command="
      select
        count(*)::text,
        count(*) filter (where relation.relrowsecurity)::text,
        count(*) filter (where relation.relforcerowsecurity)::text,
        (select count(*)::text from pg_policies where schemaname = 'app')
      from pg_class relation
      join pg_namespace namespace on namespace.oid = relation.relnamespace
      where namespace.nspname = 'app'
        and relation.relkind in ('r', 'p')
    "
}

pg_dump --format=custom --schema=app --no-owner \
  --file="${alpha_archive}" --dbname="${alpha_admin_url}"

write_table_counts "${alpha_admin_url}" "${alpha_source_counts}"
write_object_inventory "${alpha_admin_url}" "${alpha_source_objects}"
write_normalized_data "${alpha_admin_url}" "${alpha_source_data}"
write_normalized_schema "${alpha_admin_url}" "${alpha_source_schema}"

# Le drapeau est armé avant CREATE DATABASE afin que le trap traite aussi une
# interruption après création côté serveur mais avant retour du client.
alpha_restore_database_created=1
createdb --maintenance-db="${alpha_admin_url}" --template=template0 \
  -- "${alpha_restore_database}"
alpha_restore_url="${alpha_admin_url%/postgres}/${alpha_restore_database}"

pg_restore --exit-on-error --no-owner \
  --dbname="${alpha_restore_url}" "${alpha_archive}"

write_table_counts "${alpha_restore_url}" "${alpha_restore_counts}"
write_object_inventory "${alpha_restore_url}" "${alpha_restore_objects}"
write_normalized_data "${alpha_restore_url}" "${alpha_restore_data}"
write_normalized_schema "${alpha_restore_url}" "${alpha_restore_schema}"

alpha_source_stats="$(schema_stats "${alpha_admin_url}")"
alpha_restore_stats="$(schema_stats "${alpha_restore_url}")"
if [[ "${alpha_source_stats}" != "${alpha_restore_stats}" ]]; then
  printf '%s\n' 'Échec: les comptes de tables, RLS ou policies diffèrent après restauration.' >&2
  exit 1
fi

IFS=$'\t' read -r alpha_table_count alpha_rls_count alpha_forced_rls_count alpha_policy_count \
  <<<"${alpha_restore_stats}"
if (( alpha_table_count == 0 || alpha_policy_count == 0 )); then
  printf '%s\n' 'Échec: la restauration ne contient pas le schéma applicatif attendu.' >&2
  exit 1
fi
if (( alpha_rls_count != alpha_table_count || alpha_forced_rls_count != alpha_table_count )); then
  printf '%s\n' 'Échec: au moins une table applicative restaurée n’impose pas la RLS.' >&2
  exit 1
fi

if ! cmp -s "${alpha_source_counts}" "${alpha_restore_counts}"; then
  printf '%s\n' 'Échec: les nombres de lignes par table diffèrent après restauration.' >&2
  exit 1
fi
if ! cmp -s "${alpha_source_objects}" "${alpha_restore_objects}"; then
  printf '%s\n' 'Échec: l’inventaire des objets, grants, policies ou drapeaux RLS diffère.' >&2
  # L’inventaire ne contient ni donnée métier ni secret. Le diff rend le smoke
  # actionnable sur un runner distant sans conserver le dump ou les fichiers
  # temporaires en artefacts.
  diff --unified=0 \
    --label source-object-inventory \
    --label restored-object-inventory \
    "${alpha_source_objects}" "${alpha_restore_objects}" >&2 || true
  exit 1
fi
if ! cmp -s "${alpha_source_schema}" "${alpha_restore_schema}"; then
  printf '%s\n' 'Échec: le schéma PostgreSQL normalisé diffère après restauration.' >&2
  exit 1
fi

alpha_source_data_hash="$(sha256sum "${alpha_source_data}")"
alpha_source_data_hash="${alpha_source_data_hash%% *}"
alpha_restore_data_hash="$(sha256sum "${alpha_restore_data}")"
alpha_restore_data_hash="${alpha_restore_data_hash%% *}"
if [[ "${alpha_source_data_hash}" != "${alpha_restore_data_hash}" ]]; then
  printf '%s\n' 'Échec: le hash logique normalisé des données diffère après restauration.' >&2
  exit 1
fi

printf 'Backup/restore PostgreSQL local vérifié: %s tables, %s policies, données %s.\n' \
  "${alpha_table_count}" "${alpha_policy_count}" "${alpha_source_data_hash}"
