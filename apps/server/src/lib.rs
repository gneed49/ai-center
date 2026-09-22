#![forbid(unsafe_code)]

use std::sync::Arc;

use anyhow::{Context, Result};
use secrecy::ExposeSecret;
use sqlx::{PgPool, postgres::PgPoolOptions};

use agent::{AgentEngine, DeterministicEngine, OpenAiEngine};
use auth::AuthRuntime;
use config::{AgentMode, Config};
use integrations::{
    GitHubRuntime,
    github::{GitHubAppCredentials, GitHubAppTokenProvider, GitHubClient},
};
use service::AppState;
use steward::StewardDrainSupervisor;

const SUPABASE_RUNTIME_ROLE: &str = "ai_center_runtime";

// This is an exact database posture snapshot, not mutable application state;
// keeping one boolean per independently audited privilege makes fail-closed
// validation and its diagnostics explicit.
#[allow(clippy::struct_excessive_bools)]
#[derive(Clone, Debug, sqlx::FromRow)]
struct RuntimeRolePosture {
    login_role_name: String,
    effective_role_name: String,
    can_login: bool,
    is_superuser: bool,
    bypasses_rls: bool,
    can_create_databases: bool,
    can_create_roles: bool,
    can_replicate: bool,
    inherits_privileges: bool,
    has_role_memberships: bool,
    can_create_in_database: bool,
    can_create_in_app_schema: bool,
    can_create_in_public_schema: bool,
    owns_app_schema: bool,
    owns_app_objects: bool,
}

pub mod agent;
pub mod artifacts;
pub mod auth;
pub mod automation;
pub mod company;
pub mod config;
pub mod context;
pub mod error;
pub mod external_references;
pub mod idempotency;
pub mod integrations;
pub mod models;
pub mod outbox;
pub mod provider_subscriptions;
pub mod providers;
pub mod routes;
mod scope_context;
pub mod service;
pub mod steward;
pub mod team;
pub mod work_tools;

// Keep the catalog inspection in one statement so every property describes the
// same connected PostgreSQL session and role snapshot.
#[allow(clippy::too_many_lines)]
async fn verify_supabase_runtime_role(pool: &PgPool) -> Result<()> {
    let posture = sqlx::query_as::<_, RuntimeRolePosture>(
        "select
           session_user::text as login_role_name,
           current_user::text as effective_role_name,
           role.rolcanlogin as can_login,
           role.rolsuper as is_superuser,
           role.rolbypassrls as bypasses_rls,
           role.rolcreatedb as can_create_databases,
           role.rolcreaterole as can_create_roles,
           role.rolreplication as can_replicate,
           role.rolinherit as inherits_privileges,
           exists (
             select 1 from pg_catalog.pg_auth_members membership
             where membership.member = role.oid
           ) as has_role_memberships,
           pg_catalog.has_database_privilege(
             role.rolname, current_database(), 'CREATE'
           ) as can_create_in_database,
           pg_catalog.has_schema_privilege(
             role.rolname, 'app', 'CREATE'
           ) as can_create_in_app_schema,
           pg_catalog.has_schema_privilege(
             role.rolname, 'public', 'CREATE'
           ) as can_create_in_public_schema,
           exists (
             select 1 from pg_catalog.pg_namespace namespace
             where namespace.nspname = 'app'
               and namespace.nspowner = role.oid
           ) as owns_app_schema,
           (
             exists (
               select 1
               from pg_catalog.pg_class relation_row
               join pg_catalog.pg_namespace namespace_row
                 on namespace_row.oid = relation_row.relnamespace
               where namespace_row.nspname = 'app'
                 and relation_row.relowner = role.oid
             )
             or exists (
               select 1
               from pg_catalog.pg_proc procedure_row
               join pg_catalog.pg_namespace namespace_row
                 on namespace_row.oid = procedure_row.pronamespace
               where namespace_row.nspname = 'app'
                 and procedure_row.proowner = role.oid
             )
             or exists (
               select 1
               from pg_catalog.pg_type type_row
               join pg_catalog.pg_namespace namespace_row
                 on namespace_row.oid = type_row.typnamespace
               where namespace_row.nspname = 'app'
                 and type_row.typowner = role.oid
             )
             or exists (
               select 1
               from pg_catalog.pg_collation collation_row
               join pg_catalog.pg_namespace namespace_row
                 on namespace_row.oid = collation_row.collnamespace
               where namespace_row.nspname = 'app'
                 and collation_row.collowner = role.oid
             )
             or exists (
               select 1
               from pg_catalog.pg_conversion conversion_row
               join pg_catalog.pg_namespace namespace_row
                 on namespace_row.oid = conversion_row.connamespace
               where namespace_row.nspname = 'app'
                 and conversion_row.conowner = role.oid
             )
             or exists (
               select 1
               from pg_catalog.pg_operator operator_row
               join pg_catalog.pg_namespace namespace_row
                 on namespace_row.oid = operator_row.oprnamespace
               where namespace_row.nspname = 'app'
                 and operator_row.oprowner = role.oid
             )
             or exists (
               select 1
               from pg_catalog.pg_opclass operator_class_row
               join pg_catalog.pg_namespace namespace_row
                 on namespace_row.oid = operator_class_row.opcnamespace
               where namespace_row.nspname = 'app'
                 and operator_class_row.opcowner = role.oid
             )
             or exists (
               select 1
               from pg_catalog.pg_opfamily operator_family_row
               join pg_catalog.pg_namespace namespace_row
                 on namespace_row.oid = operator_family_row.opfnamespace
               where namespace_row.nspname = 'app'
                 and operator_family_row.opfowner = role.oid
             )
             or exists (
               select 1
               from pg_catalog.pg_statistic_ext statistic_row
               join pg_catalog.pg_namespace namespace_row
                 on namespace_row.oid = statistic_row.stxnamespace
               where namespace_row.nspname = 'app'
                 and statistic_row.stxowner = role.oid
             )
             or exists (
               select 1
               from pg_catalog.pg_ts_config configuration_row
               join pg_catalog.pg_namespace namespace_row
                 on namespace_row.oid = configuration_row.cfgnamespace
               where namespace_row.nspname = 'app'
                 and configuration_row.cfgowner = role.oid
             )
             or exists (
               select 1
               from pg_catalog.pg_ts_dict dictionary_row
               join pg_catalog.pg_namespace namespace_row
                 on namespace_row.oid = dictionary_row.dictnamespace
               where namespace_row.nspname = 'app'
                 and dictionary_row.dictowner = role.oid
             )
             or exists (
               select 1
               from pg_catalog.pg_extension extension_row
               join pg_catalog.pg_namespace namespace_row
                 on namespace_row.oid = extension_row.extnamespace
               where namespace_row.nspname = 'app'
                 and extension_row.extowner = role.oid
             )
           ) as owns_app_objects
         from pg_catalog.pg_roles role
         where role.rolname = session_user",
    )
    .fetch_one(pool)
    .await
    .context("failed to inspect the PostgreSQL runtime role")?;

    validate_supabase_runtime_role(&posture)
}

fn validate_supabase_runtime_role(posture: &RuntimeRolePosture) -> Result<()> {
    let mut violations = Vec::new();
    if posture.login_role_name != SUPABASE_RUNTIME_ROLE {
        violations.push("the connected login is not ai_center_runtime");
    }
    if posture.effective_role_name != SUPABASE_RUNTIME_ROLE {
        violations.push("the effective role is not ai_center_runtime");
    }
    if !posture.can_login {
        violations.push("the dedicated role is NOLOGIN");
    }
    if posture.is_superuser {
        violations.push("the dedicated role is SUPERUSER");
    }
    if posture.bypasses_rls {
        violations.push("the dedicated role has BYPASSRLS");
    }
    if posture.can_create_databases {
        violations.push("the dedicated role has CREATEDB");
    }
    if posture.can_create_roles {
        violations.push("the dedicated role has CREATEROLE");
    }
    if posture.can_replicate {
        violations.push("the dedicated role has REPLICATION");
    }
    if posture.inherits_privileges {
        violations.push("the dedicated role is INHERIT");
    }
    if posture.has_role_memberships {
        violations.push("the dedicated role belongs to another role");
    }
    if posture.can_create_in_database {
        violations.push("the dedicated role has CREATE on the database");
    }
    if posture.can_create_in_app_schema {
        violations.push("the dedicated role has CREATE on schema app");
    }
    if posture.can_create_in_public_schema {
        violations.push("the dedicated role has CREATE on schema public");
    }
    if posture.owns_app_schema {
        violations.push("the dedicated role owns schema app");
    }
    if posture.owns_app_objects {
        violations.push("the dedicated role owns objects in schema app");
    }

    anyhow::ensure!(
        violations.is_empty(),
        "AI_CENTER_AUTH_MODE=supabase refuses PostgreSQL login {}: {}",
        posture.login_role_name,
        violations.join("; ")
    );
    Ok(())
}

async fn verify_company_schema(pool: &PgPool) -> Result<()> {
    let compatible: bool = sqlx::query_scalar(
        "select not exists (
           select 1 from unnest(array[
             'app.projects','app.workspace_invitations','app.artifact_documents',
             'app.artifact_document_versions','app.context_pack_scope_sources',
             'app.work_tool_connections','app.publication_jobs','app.publication_observations',
             'app.steward_scope_sources','app.workspace_automation_controls','app.ai_call_reservations'
             ,'app.github_code_corpora','app.github_code_file_observations'
           ]) required(name) where to_regclass(name) is null
         ) and not exists (
           select 1 from unnest(array[
             'app.ensure_scope_agents(uuid)',
             'app.accept_workspace_invitation(uuid,text,text)',
             'app.context_pack_scopes_current(bigint)',
             'app.claim_publication_job()',
             'app.steward_scope_source_status(bigint)'
           ]) required(name) where to_regprocedure(name) is null
         ) and not exists (
           select 1 from (values ('messages','author_actor_id'),('messages','command_public_id'),
             ('messages','submitted_content'),('domain_events','deferred_count')) required(table_name,column_name)
           where not exists (select 1 from information_schema.columns c where c.table_schema='app'
             and c.table_name=required.table_name and c.column_name=required.column_name)
         )",
    )
    .fetch_one(pool)
    .await
    .context("failed to inspect the application schema")?;
    anyhow::ensure!(
        compatible,
        "Company Context database migration required before server startup"
    );
    Ok(())
}

/// Connects the application services and returns the configured HTTP router.
///
/// # Errors
///
/// Returns an error when `PostgreSQL` cannot be reached or the selected agent
/// engine is missing required server-side configuration.
pub async fn build(
    config: Config,
) -> Result<(
    std::net::SocketAddr,
    axum::Router,
    Option<StewardDrainSupervisor>,
)> {
    let pool = PgPoolOptions::new()
        .max_connections(10)
        .acquire_timeout(std::time::Duration::from_secs(5))
        .connect(config.database_url.expose_secret())
        .await
        .context("failed to connect to PostgreSQL")?;
    if config.auth_mode == config::AuthMode::Supabase {
        verify_supabase_runtime_role(&pool).await?;
    }
    verify_company_schema(&pool).await?;
    let cipher = if let Some(key) = &config.credential_encryption_key {
        providers::encryption::CredentialCipher::from_hex(key).ok()
    } else if let Some(directory) = &config.credential_directory {
        providers::encryption::CredentialCipher::local(directory).ok()
    } else {
        None
    };
    let subscriptions = config
        .subscription_runtime
        .map(|(root, executable, enabled)| {
            Arc::new(provider_subscriptions::SubscriptionRuntime::new(
                root, executable, enabled,
            ))
        });
    let providers = Some(Arc::new(providers::ProviderRuntime {
        cipher,
        subscriptions,
    }));
    let (engine, agent_mode): (Arc<dyn AgentEngine>, &'static str) = match config.agent_mode {
        AgentMode::Deterministic => (Arc::new(DeterministicEngine), "deterministic"),
        AgentMode::OpenAi => (
            Arc::new(OpenAiEngine::new(
                config.openai_api_key.context("OPENAI_API_KEY is missing")?,
                config.openai_model,
            )?),
            "openai",
        ),
    };
    let bind = config.bind;
    let auth = Arc::new(AuthRuntime::new(
        config.auth_mode,
        config.workspace_id,
        config.actor_id,
        config.supabase_url,
    )?);
    let github = match (
        config.github_app_id,
        config.github_app_installation_id,
        config.github_app_private_key_path,
    ) {
        (Some(app_id), Some(installation_id), Some(private_key_path)) => {
            let private_key = std::fs::read_to_string(&private_key_path).with_context(|| {
                format!(
                    "failed to read GitHub App private key from {}",
                    private_key_path.display()
                )
            })?;
            let provider = Arc::new(GitHubAppTokenProvider::new(GitHubAppCredentials {
                app_id,
                installation_id,
                private_key_pem: secrecy::SecretString::from(private_key),
            })?);
            Some(Arc::new(GitHubRuntime {
                client: Arc::new(GitHubClient::new(provider)?),
                installation_id,
            }))
        }
        (None, None, None) => None,
        _ => unreachable!("GitHub configuration is validated by Config::from_env"),
    };
    let application_state = AppState {
        pool,
        engine,
        providers,
        workspace_id: config.workspace_id,
        workspace_internal_id: None,
        workspace_role: "owner".into(),
        actor_id: config.actor_id,
        agent_mode,
        steward_trigger: None,
    };
    let (state, steward_supervisor) =
        start_background_workers(application_state, config.agent_mode)?;
    Ok((
        bind,
        routes::router(state, auth, github, config.cors_origins),
        steward_supervisor,
    ))
}

fn start_background_workers(
    mut application_state: AppState,
    mode: AgentMode,
) -> Result<(Arc<AppState>, Option<StewardDrainSupervisor>)> {
    let mut supervisor = if application_state.providers.is_some() || mode == AgentMode::OpenAi {
        let (trigger, supervisor) = StewardDrainSupervisor::start(application_state.clone())
            .context("failed to initialize the Steward outbox supervisor")?;
        application_state.steward_trigger = Some(trigger);
        Some(supervisor)
    } else {
        None
    };
    let state = Arc::new(application_state);
    if let Some(supervisor) = supervisor.as_mut() {
        supervisor.attach_background_task(work_tools::worker::start(state.clone()));
    }
    Ok((state, supervisor))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_runtime_role_posture() -> RuntimeRolePosture {
        RuntimeRolePosture {
            login_role_name: SUPABASE_RUNTIME_ROLE.into(),
            effective_role_name: SUPABASE_RUNTIME_ROLE.into(),
            can_login: true,
            is_superuser: false,
            bypasses_rls: false,
            can_create_databases: false,
            can_create_roles: false,
            can_replicate: false,
            inherits_privileges: false,
            has_role_memberships: false,
            can_create_in_database: false,
            can_create_in_app_schema: false,
            can_create_in_public_schema: false,
            owns_app_schema: false,
            owns_app_objects: false,
        }
    }

    #[test]
    fn accepts_only_the_dedicated_least_privilege_posture() {
        validate_supabase_runtime_role(&valid_runtime_role_posture())
            .expect("the dedicated least-privilege runtime role should be accepted");
    }

    #[test]
    fn rejects_a_privileged_or_owner_runtime_role() {
        let mut posture = valid_runtime_role_posture();
        posture.can_login = false;
        posture.is_superuser = true;
        posture.bypasses_rls = true;
        posture.can_create_databases = true;
        posture.can_create_roles = true;
        posture.can_replicate = true;
        posture.inherits_privileges = true;
        posture.has_role_memberships = true;
        posture.can_create_in_database = true;
        posture.can_create_in_app_schema = true;
        posture.can_create_in_public_schema = true;
        posture.owns_app_schema = true;
        posture.owns_app_objects = true;

        let error = validate_supabase_runtime_role(&posture)
            .expect_err("every elevated posture flag must fail closed")
            .to_string();
        for expected in [
            "NOLOGIN",
            "SUPERUSER",
            "BYPASSRLS",
            "CREATEDB",
            "CREATEROLE",
            "REPLICATION",
            "INHERIT",
            "belongs to another role",
            "CREATE on the database",
            "CREATE on schema app",
            "CREATE on schema public",
            "owns schema app",
            "owns objects in schema app",
        ] {
            assert!(error.contains(expected), "missing violation: {expected}");
        }
    }

    #[test]
    fn rejects_login_or_effective_role_substitution() {
        let mut substituted_login = valid_runtime_role_posture();
        substituted_login.login_role_name = "migration_owner".into();
        let login_error = validate_supabase_runtime_role(&substituted_login)
            .expect_err("a different connected login must fail closed")
            .to_string();
        assert!(login_error.contains("connected login is not ai_center_runtime"));

        let mut substituted_effective_role = valid_runtime_role_posture();
        substituted_effective_role.effective_role_name = "migration_owner".into();
        let effective_error = validate_supabase_runtime_role(&substituted_effective_role)
            .expect_err("SET ROLE substitution must fail closed")
            .to_string();
        assert!(effective_error.contains("effective role is not ai_center_runtime"));
    }
}
