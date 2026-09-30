//! Explicit durable source commands: admission, bounded network, fenced commit.
use super::{
    admission,
    locator::{LinearIssueKey, SourceLocator, parse_locator},
    models::{
        AttachToolSource, ExistingReadErrorCode, ExistingReadFailure, ExistingReadOutcome,
        ExpectedSource, RebindToolSource, SourceCommandResult, ToolProvider,
    },
    persistence::{self, Reference},
    reader::ExistingToolReader,
};
use crate::{
    artifacts,
    error::{AppError, AppResult},
    idempotency::{self, IdempotencyLease},
    service::AppState,
    work_tools::{self, Credential},
};
use serde_json::{Value, json};
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

#[derive(Clone)]
pub enum Command {
    Attach(AttachToolSource),
    Refresh(Uuid, ExpectedSource),
    Detach(Uuid, ExpectedSource),
    Rebind(Uuid, RebindToolSource),
}
impl Command {
    #[must_use]
    pub const fn action(&self) -> &'static str {
        match self {
            Self::Attach(_) => "attach",
            Self::Refresh(..) => "refresh",
            Self::Detach(..) => "detach",
            Self::Rebind(..) => "rebind",
        }
    }
    pub fn normalize(self) -> AppResult<Self> {
        match self {
            Self::Attach(mut input) => {
                let locator = parse_locator(input.provider, &input.source)
                    .map_err(|e| AppError::Invalid(e.to_string()))?;
                input.source = locator.canonical_input();
                if !input.confirm_scope_sharing
                    || input.connection_id.is_nil()
                    || input.expected_connection_revision < 1
                {
                    return Err(AppError::Invalid("Confirmez le partage de cette source dans le projet et choisissez une connexion active.".into()));
                }
                Ok(Self::Attach(input))
            }
            Self::Rebind(_, ref input)
                if !input.confirm_scope_sharing
                    || input.connection_id.is_nil()
                    || input.expected_connection_revision < 1 =>
            {
                Err(AppError::Invalid(
                    "Confirmez le partage et choisissez une connexion active.".into(),
                ))
            }
            other => Ok(other),
        }
    }
    #[must_use]
    pub fn request(&self, project: Uuid) -> Value {
        match self {
            Self::Attach(input) => {
                json!({"action":self.action(),"project_id":project,"input":input})
            }
            Self::Refresh(id, input) | Self::Detach(id, input) => {
                json!({"action":self.action(),"project_id":project,"reference_id":id,"input":input})
            }
            Self::Rebind(id, input) => {
                json!({"action":self.action(),"project_id":project,"reference_id":id,"input":input})
            }
        }
    }
    fn expected(&self) -> Option<(i32, Uuid)> {
        match self {
            Self::Attach(_) => None,
            Self::Refresh(_, v) | Self::Detach(_, v) => {
                Some((v.expected_revision, v.expected_observation_id))
            }
            Self::Rebind(_, v) => Some((v.expected_revision, v.expected_observation_id)),
        }
    }
}
pub fn operation(action: &str, project: Uuid) -> AppResult<String> {
    if !matches!(action, "attach" | "refresh" | "detach" | "rebind") || project.is_nil() {
        return Err(AppError::Invalid("Opération de source invalide.".into()));
    }
    Ok(format!("tool_source.{action}:{project}"))
}
pub async fn project_for_reference(state: &AppState, id: Uuid) -> AppResult<Uuid> {
    let mut tx = state.begin_request().await?;
    let project=sqlx::query_scalar("select p.public_id from app.tool_source_references r join app.projects p on p.id=r.project_id where r.public_id=$1 and r.workspace_id=app.current_workspace_id()").bind(id).fetch_optional(&mut *tx).await?.ok_or(AppError::NotFound)?;
    tx.commit().await?;
    Ok(project)
}
struct Connection {
    id: i64,
    revision: i32,
}
async fn require_owner(tx: &mut Transaction<'_, Postgres>) -> AppResult<()> {
    let owner: bool = sqlx::query_scalar(
        "select app.has_workspace_role(app.current_workspace_id(),array['owner'])",
    )
    .fetch_one(&mut **tx)
    .await?;
    if owner {
        Ok(())
    } else {
        Err(AppError::Forbidden)
    }
}
async fn connection(
    tx: &mut Transaction<'_, Postgres>,
    id: Uuid,
    provider: ToolProvider,
    expected: Option<i32>,
) -> AppResult<Connection> {
    work_tools::connection_lock(tx, id, false).await?;
    let row:Option<(i64,i32)>=sqlx::query_as("select id,revision from app.work_tool_connections where public_id=$1 and workspace_id=app.current_workspace_id() and provider=$2 and enabled and allow_existing_reads and app.has_workspace_role(workspace_id,array['owner','editor'])")
        .bind(id).bind(provider.as_str()).fetch_optional(&mut **tx).await?;
    let (id, revision) = row.ok_or_else(|| {
        AppError::Conflict("Activez la lecture des sources sur une connexion compatible.".into())
    })?;
    if expected.is_some_and(|n| n != revision) {
        return Err(AppError::Conflict(
            "La connexion a changé. Rechargez ses réglages avant de réessayer.".into(),
        ));
    }
    Ok(Connection { id, revision })
}
async fn ensure_expected(
    tx: &mut Transaction<'_, Postgres>,
    command: &Command,
    r: &Reference,
    lease: &IdempotencyLease,
) -> AppResult<()> {
    if command.expected() == Some((r.revision, r.observation_public_id)) {
        return Ok(());
    }
    // A retry may reuse its own failed verification, never another command's edit.
    let own_failure:bool=sqlx::query_scalar("select exists(select 1 from app.audit_events where workspace_id=app.current_workspace_id() and actor_id=app.current_actor_id() and action='tool_source.verification_failed' and object_public_id=$1 and after_state->>'command_public_id'=$2 and after_state->>'revision'=$3 and after_state->>'observation_id'=$4 and after_state->>'connection_id'=$5 and after_state->>'retryable'='true')")
        .bind(r.public_id).bind(lease.record_public_id.to_string()).bind(r.revision.to_string()).bind(r.observation_public_id.to_string()).bind(r.connection_public_id.to_string()).fetch_one(&mut **tx).await?;
    if own_failure
        && command
            .expected()
            .is_some_and(|(_, head)| head == r.observation_public_id)
    {
        return Ok(());
    }
    Err(AppError::Conflict(
        "La source a changé. Rechargez-la avant de réessayer.".into(),
    ))
}
fn provider(r: &Reference) -> AppResult<ToolProvider> {
    match r.provider.as_str() {
        "linear" => Ok(ToolProvider::Linear),
        "notion" => Ok(ToolProvider::Notion),
        _ => Err(AppError::Internal("Invalid source provider".into())),
    }
}
fn locator_for(r: &Reference) -> AppResult<SourceLocator> {
    parse_locator(provider(r)?, &r.external_id.to_string())
        .map_err(|_| AppError::Internal("Invalid source identity".into()))
}
async fn existing_result(
    tx: &mut Transaction<'_, Postgres>,
    r: &Reference,
    connection: Uuid,
    action: &str,
) -> AppResult<SourceCommandResult> {
    if r.connection_public_id != connection {
        return Err(AppError::Conflict("Cette source utilise une autre connexion. Un propriétaire doit la reconnecter explicitement.".into()));
    }
    if r.status != "active" {
        return Err(AppError::Conflict(
            "Cette source a été retirée. Un propriétaire doit la reconnecter explicitement.".into(),
        ));
    }
    persistence::result(tx, r.public_id, action, "existing", "not_performed").await
}
struct Prepared {
    project: i64,
    provider: ToolProvider,
    connection_public_id: Uuid,
    connection_revision: i32,
    reference: Option<Reference>,
    locator: SourceLocator,
    credential: Credential,
    attempt: admission::Attempt,
}
enum Admission {
    Existing(Box<SourceCommandResult>),
    Read(Box<Prepared>),
}
async fn prepare(
    state: &AppState,
    project: Uuid,
    command: &Command,
    lease: &IdempotencyLease,
) -> AppResult<Admission> {
    artifacts::editor(state)?;
    if matches!(command, Command::Rebind(..)) {
        work_tools::owner(state)?;
    }
    let mut tx = state.begin_request().await?;
    let project_internal = artifacts::project_id(&mut tx, project).await?;
    crate::company::data::require_active(&mut tx, project_internal).await?;
    if matches!(command, Command::Rebind(..)) {
        require_owner(&mut tx).await?;
    }
    if lease.project_id != Some(project_internal)
        || lease.operation_key != operation(command.action(), project)?
    {
        return Err(AppError::Conflict(
            "Cette commande appartient à un autre contexte.".into(),
        ));
    }
    let before = match command {
        Command::Attach(_) => None,
        Command::Refresh(id, _) | Command::Detach(id, _) | Command::Rebind(id, _) => {
            let r = persistence::reference(&mut tx, *id, false).await?;
            if r.project_id != project_internal {
                return Err(AppError::NotFound);
            }
            Some(r)
        }
    };
    // Detach neither needs a working credential nor performs any remote read.
    if let Command::Detach(..) = command {
        let before = before.as_ref().ok_or(AppError::NotFound)?;
        work_tools::connection_lock(&mut tx, before.connection_public_id, false).await?;
        let r = persistence::reference(&mut tx, before.public_id, true).await?;
        ensure_expected(&mut tx, command, &r, lease).await?;
        sqlx::query("update app.tool_source_references set status='detached',revision=revision+1,updated_at=clock_timestamp() where id=$1").bind(r.id).execute(&mut *tx).await?;
        let result =
            persistence::result(&mut tx, r.public_id, "detach", "detached", "not_performed")
                .await?;
        artifacts::complete(&mut tx, Some(lease), &result).await?;
        tx.commit().await?;
        return Ok(Admission::Existing(Box::new(result)));
    }
    let (connection_id, expected, locator) = match command {
        Command::Attach(v) => (
            v.connection_id,
            Some(v.expected_connection_revision),
            parse_locator(v.provider, &v.source).map_err(|e| AppError::Invalid(e.to_string()))?,
        ),
        Command::Rebind(_, v) => (
            v.connection_id,
            Some(v.expected_connection_revision),
            locator_for(before.as_ref().ok_or(AppError::NotFound)?)?,
        ),
        Command::Refresh(..) => {
            let r = before.as_ref().ok_or(AppError::NotFound)?;
            (r.connection_public_id, None, locator_for(r)?)
        }
        Command::Detach(..) => unreachable!(),
    };
    let provider = locator.provider();
    let conn = connection(&mut tx, connection_id, provider, expected).await?;
    let reference = if let Some(before) = before {
        let r = persistence::reference(&mut tx, before.public_id, true).await?;
        ensure_expected(&mut tx, command, &r, lease).await?;
        if r.status != "active" && !matches!(command, Command::Rebind(..)) {
            return Err(AppError::Conflict(
                "Cette source a été retirée. Reconnectez-la pour la vérifier.".into(),
            ));
        }
        if r.connection_public_id != before.connection_public_id {
            return Err(AppError::Conflict(
                "La connexion de la source a changé.".into(),
            ));
        }
        Some(r)
    } else {
        let existing = match &locator {
            SourceLocator::NotionPage { id }
            | SourceLocator::LinearIssue {
                key: LinearIssueKey::Id(id),
                ..
            } => persistence::identity(&mut tx, project_internal, provider, *id).await?,
            SourceLocator::LinearIssue {
                key: LinearIssueKey::Identifier(identifier),
                ..
            } => persistence::alias(&mut tx, project_internal, connection_id, identifier).await?,
        };
        if let Some(r) = existing {
            // A URL's workspace is part of its identity, including locally known aliases.
            if let SourceLocator::LinearIssue {
                expected_workspace_slug: Some(slug),
                ..
            } = &locator
                && !r
                    .canonical_url
                    .starts_with(&format!("https://linear.app/{slug}/issue/"))
            {
                return Err(admission::remote_error(&ExistingReadFailure::new(
                    ExistingReadErrorCode::RemoteIdentityMismatch,
                )));
            }
            let result = existing_result(&mut tx, &r, connection_id, "attach").await?;
            artifacts::complete(&mut tx, Some(lease), &result).await?;
            tx.commit().await?;
            return Ok(Admission::Existing(Box::new(result)));
        }
        None
    };
    if reference.as_ref().is_none_or(|r| r.status != "active") {
        persistence::capacity(&mut tx, project_internal).await?;
    }
    let credential = work_tools::credential_in_tx(state, &mut tx, connection_id).await?;
    let fingerprint = idempotency::hash_json(
        &json!({"project_id":project,"connection_id":connection_id,"provider":provider,"source":locator.canonical_input()}),
    );
    let attempt = admission::admit(
        &mut tx,
        state,
        connection_id,
        &fingerprint,
        reference.as_ref().and_then(|r| r.last_attempt_at),
        lease,
    )
    .await?;
    if let Some(r) = &reference {
        sqlx::query("update app.tool_source_references set last_attempt_at=$2 where id=$1")
            .bind(r.id)
            .bind(attempt.admitted_at)
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;
    Ok(Admission::Read(Box::new(Prepared {
        project: project_internal,
        provider,
        connection_public_id: connection_id,
        connection_revision: conn.revision,
        reference,
        locator,
        credential,
        attempt,
    })))
}
/// Production entry point always uses official HTTPS endpoints.
pub async fn execute(
    state: &AppState,
    project: Uuid,
    command: Command,
    lease: &IdempotencyLease,
) -> AppResult<SourceCommandResult> {
    let reader = ExistingToolReader::official().map_err(|e| admission::remote_error(&e))?;
    execute_inner(state, project, command, lease, &reader).await
}
/// Synthetic integration tests may inject only the reader's restricted loopback client.
#[cfg(debug_assertions)]
pub async fn execute_with_reader(
    state: &AppState,
    project: Uuid,
    command: Command,
    lease: &IdempotencyLease,
    reader: &ExistingToolReader,
) -> AppResult<SourceCommandResult> {
    execute_inner(state, project, command, lease, reader).await
}
async fn execute_inner(
    state: &AppState,
    project: Uuid,
    command: Command,
    lease: &IdempotencyLease,
    reader: &ExistingToolReader,
) -> AppResult<SourceCommandResult> {
    let command = command.normalize()?;
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(45);
    let prepared = tokio::time::timeout_at(deadline, prepare(state, project, &command, lease))
        .await
        .map_err(|_| {
            admission::remote_error(&ExistingReadFailure::new(
                ExistingReadErrorCode::RemoteTimeout,
            ))
        })??;
    let Admission::Read(prepared) = prepared else {
        let Admission::Existing(result) = prepared else {
            unreachable!()
        };
        return Ok(*result);
    };
    let remote = tokio::time::timeout_at(
        deadline,
        reader.read_existing(&prepared.credential.secret, &prepared.locator),
    )
    .await
    .unwrap_or_else(|_| {
        Err(ExistingReadFailure::new(
            ExistingReadErrorCode::RemoteTimeout,
        ))
    });
    // Network is stopped before this best-effort cleanup. Lost rights leave a 60 s slot.
    let _ = admission::finished(state, &prepared.attempt, remote.as_ref().err()).await;
    finalize(state, &command, lease, *prepared, remote).await
}
async fn finalize(
    state: &AppState,
    command: &Command,
    lease: &IdempotencyLease,
    before: Prepared,
    remote: Result<ExistingReadOutcome, ExistingReadFailure>,
) -> AppResult<SourceCommandResult> {
    let mut tx = state.begin_request().await?;
    crate::company::data::require_active(&mut tx, before.project).await?;
    if matches!(command, Command::Rebind(..)) {
        require_owner(&mut tx).await?;
    }
    let conn = connection(
        &mut tx,
        before.connection_public_id,
        before.provider,
        Some(before.connection_revision),
    )
    .await?;
    let existing = if let Some(r) = before.reference {
        let current = persistence::reference(&mut tx, r.public_id, true).await?;
        if current.revision != r.revision
            || current.observation_public_id != r.observation_public_id
            || current.connection_public_id != r.connection_public_id
            || current.status != r.status
        {
            return Err(AppError::Conflict(
                "La source a changé pendant la lecture. Rechargez-la.".into(),
            ));
        }
        Some(current)
    } else {
        None
    };
    if existing.as_ref().is_some_and(|r| r.status == "detached") {
        persistence::capacity(&mut tx, before.project).await?;
    }
    admission::enabled(&mut tx).await?;
    let (observation, error_code) = match remote {
        Ok(ExistingReadOutcome::Available(snapshot)) => {
            if existing
                .as_ref()
                .is_some_and(|r| r.external_id != snapshot.external_id)
            {
                return Err(admission::remote_error(&ExistingReadFailure::new(
                    ExistingReadErrorCode::RemoteIdentityMismatch,
                )));
            }
            if existing.is_none()
                && let Some(found) = persistence::identity(
                    &mut tx,
                    before.project,
                    before.provider,
                    snapshot.external_id,
                )
                .await?
            {
                let result =
                    existing_result(&mut tx, &found, before.connection_public_id, "attach").await?;
                artifacts::complete(&mut tx, Some(lease), &result).await?;
                tx.commit().await?;
                return Ok(result);
            }
            (
                persistence::Observation::available(*snapshot, before.provider)?,
                None,
            )
        }
        Ok(ExistingReadOutcome::Unavailable { code })
            if existing.is_some() && !matches!(command, Command::Rebind(..)) =>
        {
            (
                persistence::Observation::unavailable(
                    existing.as_ref().ok_or(AppError::NotFound)?,
                    before.provider,
                )?,
                Some(code.as_str()),
            )
        }
        other => {
            let failure = match other {
                Err(failure) => failure,
                Ok(ExistingReadOutcome::Unavailable { code }) => ExistingReadFailure::new(code),
                Ok(ExistingReadOutcome::Available(_)) => unreachable!(),
            };
            let error = admission::remote_error(&failure);
            if let Some(r) = &existing {
                sqlx::query("update app.tool_source_references set revision=revision+1,updated_at=clock_timestamp(),last_checked_at=clock_timestamp(),last_check_status='failed',last_check_error_code=$2 where id=$1").bind(r.id).bind(failure.code.as_str()).execute(&mut *tx).await?;
                work_tools::audit(&mut tx,state,"tool_source.verification_failed",r.public_id,json!({"command_public_id":lease.record_public_id,"revision":r.revision+1,"observation_id":r.observation_public_id,"connection_id":r.connection_public_id,"retryable":failure.retryable})).await?;
            }
            let retry_after = match &error {
                AppError::ToolSource { retry_after, .. } => *retry_after,
                _ => None,
            };
            idempotency::fail(&mut tx,lease,error.status_code().as_u16(),error.public_code(),if failure.retryable{idempotency::FailureDisposition::Retryable}else{idempotency::FailureDisposition::Permanent},json!({"code":error.public_code(),"message":error.public_message(),"retry_after":retry_after})).await?;
            tx.commit().await?;
            return Err(error);
        }
    };
    let result = persistence::persist(
        &mut tx,
        state,
        before.project,
        before.provider,
        conn.id,
        conn.revision,
        existing.as_ref(),
        observation,
        before.attempt.admitted_at,
        command.action(),
        error_code,
    )
    .await?;
    artifacts::complete(&mut tx, Some(lease), &result).await?;
    tx.commit().await?;
    Ok(result)
}
/// Read only: historical response bodies are returned exactly as stored.
pub async fn receipt(state: &AppState, project: Uuid, key: Uuid, action: &str) -> AppResult<Value> {
    let op = operation(action, project)?;
    let mut tx = state.begin_request().await?;
    let row:Option<(i64,String)>=sqlx::query_as("select id,status from app.projects where public_id=$1 and workspace_id=app.current_workspace_id()").bind(project).fetch_optional(&mut *tx).await?;
    let (project, status) = row.ok_or(AppError::NotFound)?;
    let result:Option<Value>=sqlx::query_scalar("select jsonb_build_object('status',case when expires_at<=clock_timestamp() then 'expired' when status='completed' then 'completed' when status='processing' and locked_until>clock_timestamp() then 'processing' when status='processing' then 'interrupted' when status='failed' and response_body->>'retryable'='true' then 'retryable' else 'failed' end,'can_retry',coalesce(expires_at>clock_timestamp() and ((status='processing' and locked_until<=clock_timestamp()) or (status='failed' and response_body->>'retryable'='true')),false),'retry_after',response_body->'retry_after','result',case when status='completed' then response_body else null end,'error',case when status='failed' then jsonb_build_object('code',response_body->'code','message',response_body->'message') else null end) from app.idempotency_records where workspace_id=app.current_workspace_id() and actor_id=app.current_actor_id() and project_id=$1 and operation_key=$2 and idempotency_key=$3")
        .bind(project).bind(op).bind(key.to_string()).fetch_optional(&mut *tx).await?;
    tx.commit().await?;
    let mut result=result.unwrap_or_else(||json!({"status":"not_received","can_retry":true,"retry_after":null,"result":null,"error":null}));
    if !matches!(state.workspace_role.as_str(), "owner" | "editor")
        || status != "active"
        || (action == "rebind" && state.workspace_role != "owner")
    {
        result["can_retry"] = json!(false);
    }
    Ok(result)
}
