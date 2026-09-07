//! Personal subscription access through an unmodified, explicitly configured CLI.
//!
//! The runtime never reads its credential files. Every subprocess has a private
//! environment, no project context, a deadline and bounded output. A process
//! group guard also covers cancellation while a future is being dropped.

use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    process::{ExitStatus, Stdio},
    sync::{Arc, Mutex as StdMutex, Weak},
    time::{Duration, Instant},
};

use async_trait::async_trait;
use serde::Serialize;
use serde_json::{Value, json};
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWriteExt},
    process::{Child, Command},
    sync::{Mutex, OwnedMutexGuard, Semaphore, watch},
    task::JoinHandle,
};
use url::Url;
use uuid::Uuid;

use crate::{
    agent::AgentRunMetadata,
    error::{AppError, AppResult, ProviderError, ProviderErrorClass},
    integrations::providers::{ProviderModel, StructuredResponse, StructuredTransport},
};

const PROVIDER: &str = "claude_subscription";
const OUTPUT_LIMIT: usize = 2 * 1024 * 1024;
const AUTH_OUTPUT_LIMIT: usize = 64 * 1024;
const INPUT_LIMIT: usize = 2 * 1024 * 1024;
const SCHEMA_LIMIT: usize = 64 * 1024;
const MAX_LOGIN_RECORDS: usize = 64;
const CLI_TIMEOUT: Duration = Duration::from_secs(10);
const LOGIN_TIMEOUT: Duration = Duration::from_secs(180);
const GENERATION_TIMEOUT: Duration = Duration::from_secs(120);
const REQUIRED_FLAGS: &[&str] = &[
    "--safe-mode",
    "--tools",
    "--disallowedTools",
    "--strict-mcp-config",
    "--mcp-config",
    "--disable-slash-commands",
    "--no-chrome",
    "--setting-sources",
    "--settings",
    "--permission-mode",
    "--no-session-persistence",
    "--json-schema",
    "--output-format",
    "--system-prompt",
];

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SubscriptionScope {
    pub workspace_id: Uuid,
    pub actor_id: Uuid,
    pub connection_id: Uuid,
}

#[derive(Clone, Debug, Serialize)]
pub struct SubscriptionCapability {
    pub available: bool,
    pub reason: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct SubscriptionStatus {
    pub status: &'static str,
    pub connected: bool,
    pub available: bool,
    pub message: Option<String>,
}

#[derive(Clone, Serialize)]
pub struct SubscriptionLogin {
    pub login_id: Uuid,
    pub status: &'static str,
    /// Transient official authorization URL; never persisted or logged.
    pub auth_url: Option<String>,
}

impl std::fmt::Debug for SubscriptionLogin {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("SubscriptionLogin")
            .field("login_id", &self.login_id)
            .field("status", &self.status)
            .field("auth_url", &self.auth_url.as_ref().map(|_| "[REDACTED]"))
            .finish()
    }
}

#[derive(Clone)]
pub struct SubscriptionRuntime {
    inner: Arc<RuntimeInner>,
}

struct RuntimeInner {
    root: PathBuf,
    executable: PathBuf,
    enabled: bool,
    locks: StdMutex<HashMap<SubscriptionScope, Weak<Mutex<()>>>>,
    forgotten: StdMutex<HashSet<SubscriptionScope>>,
    slots: Arc<Semaphore>,
    logins: StdMutex<HashMap<Uuid, LoginRecord>>,
    generations: StdMutex<HashMap<SubscriptionScope, watch::Sender<bool>>>,
    #[cfg(test)]
    generation_timeout: Duration,
}

struct LoginRecord {
    scope: SubscriptionScope,
    state: Arc<StdMutex<SubscriptionLogin>>,
    task: JoinHandle<()>,
    created: Instant,
}

impl Drop for LoginRecord {
    fn drop(&mut self) {
        self.task.abort();
    }
}

struct ScopePaths {
    home: PathBuf,
    config: PathBuf,
    cwd: PathBuf,
}

struct PrivateDirectory(PathBuf);

impl Drop for PrivateDirectory {
    fn drop(&mut self) {
        // Only our UUID-named, non-credential scratch directory is removed.
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

struct ProcessGuard {
    child: Child,
    #[cfg(unix)]
    group: nix::unistd::Pid,
}

impl ProcessGuard {
    fn spawn(command: &mut Command) -> AppResult<Self> {
        command.kill_on_drop(true);
        #[cfg(unix)]
        command.process_group(0);
        let child = command
            .spawn()
            .map_err(|_| failure(ProviderErrorClass::Transport))?;
        #[cfg(unix)]
        let group = nix::unistd::Pid::from_raw(
            i32::try_from(
                child
                    .id()
                    .ok_or_else(|| failure(ProviderErrorClass::Transport))?,
            )
            .map_err(|_| failure(ProviderErrorClass::Transport))?,
        );
        Ok(Self {
            child,
            #[cfg(unix)]
            group,
        })
    }
}

impl Drop for ProcessGuard {
    fn drop(&mut self) {
        #[cfg(unix)]
        let _ = nix::sys::signal::killpg(self.group, nix::sys::signal::Signal::SIGKILL);
        let _ = self.child.start_kill();
    }
}

struct ProcessOutput {
    status: ExitStatus,
    stdout: Vec<u8>,
}

impl SubscriptionRuntime {
    #[must_use]
    pub fn new(root: PathBuf, executable: PathBuf, enabled: bool) -> Self {
        Self {
            inner: Arc::new(RuntimeInner {
                root,
                executable,
                enabled,
                locks: StdMutex::new(HashMap::new()),
                forgotten: StdMutex::new(HashSet::new()),
                slots: Arc::new(Semaphore::new(4)),
                logins: StdMutex::new(HashMap::new()),
                generations: StdMutex::new(HashMap::new()),
                #[cfg(test)]
                generation_timeout: GENERATION_TIMEOUT,
            }),
        }
    }

    /// Checks only the installed client's help/version, never its account.
    pub async fn capabilities(&self) -> SubscriptionCapability {
        let result = self.check_capability().await;
        match result {
            Ok(()) => SubscriptionCapability {
                available: true,
                reason: None,
            },
            Err(reason) => SubscriptionCapability {
                available: false,
                reason: Some(reason.into()),
            },
        }
    }

    async fn check_capability(&self) -> Result<(), &'static str> {
        if !self.inner.enabled {
            return Err("Les abonnements locaux sont désactivés sur ce serveur.");
        }
        if !cfg!(target_os = "linux") {
            return Err(
                "Le confinement du client abonnement est actuellement vérifié sur Linux uniquement.",
            );
        }
        if !self.inner.root.is_absolute() || !self.inner.executable.is_absolute() {
            return Err(
                "Configurer le chemin absolu du client officiel Claude Code et de son stockage privé.",
            );
        }
        if !self.inner.executable.is_file() {
            return Err(
                "Le client officiel Claude Code n’est pas installé à l’emplacement configuré.",
            );
        }
        if managed_policy_present() {
            return Err(
                "Une politique administrée nécessite de vérifier le confinement des outils et hooks avant activation.",
            );
        }
        private_directory(&self.inner.root)
            .map_err(|_| "Le stockage privé du client officiel est indisponible.")?;
        let probe = PrivateDirectory(self.inner.root.join(format!("probe-{}", Uuid::new_v4())));
        let paths = ScopePaths::new(&probe.0)
            .map_err(|_| "Le stockage privé du client officiel est indisponible.")?;
        let _slot = tokio::time::timeout(CLI_TIMEOUT, self.inner.slots.clone().acquire_owned())
            .await
            .map_err(|_| "Le client abonnement est occupé.")?
            .map_err(|_| "Le client abonnement est indisponible.")?;
        let version = run_command(
            &self.inner.executable,
            &paths,
            &["--version"],
            None,
            CLI_TIMEOUT,
            AUTH_OUTPUT_LIMIT,
            None,
        )
        .await
        .map_err(|_| "Impossible de vérifier la version du client officiel.")?;
        if !version.status.success() || !supported_version(&version.stdout) {
            return Err(
                "Cette version du client Claude Code n’a pas encore été vérifiée. Le contrat actuel prend en charge la version 2.1.220.",
            );
        }
        let help = run_command(
            &self.inner.executable,
            &paths,
            &["--help"],
            None,
            CLI_TIMEOUT,
            AUTH_OUTPUT_LIMIT,
            None,
        )
        .await
        .map_err(|_| "Impossible de vérifier les options de sécurité du client officiel.")?;
        if !help.status.success() {
            return Err("Impossible de vérifier les options de sécurité du client officiel.");
        }
        let help = std::str::from_utf8(&help.stdout)
            .map_err(|_| "Le client officiel utilise un contrat inconnu.")?;
        if !REQUIRED_FLAGS.iter().all(|flag| help.contains(flag)) {
            return Err(
                "Le client installé ne fournit pas toutes les options requises pour retirer les outils.",
            );
        }
        Ok(())
    }

    async fn ensure_available(&self) -> AppResult<()> {
        let capability = self.capabilities().await;
        if !capability.available {
            return Err(AppError::Agent(capability.reason.unwrap_or_default()));
        }
        Ok(())
    }

    fn scope_lock(&self, scope: SubscriptionScope) -> AppResult<OwnedMutexGuard<()>> {
        let lock = {
            let mut locks = self
                .inner
                .locks
                .lock()
                .map_err(|_| failure(ProviderErrorClass::Transport))?;
            locks.retain(|_, lock| lock.strong_count() > 0);
            if let Some(lock) = locks.get(&scope).and_then(Weak::upgrade) {
                lock
            } else {
                let lock = Arc::new(Mutex::new(()));
                locks.insert(scope, Arc::downgrade(&lock));
                lock
            }
        };
        lock.try_lock_owned().map_err(|_| {
            AppError::Conflict("Une opération utilise déjà cette connexion abonnement.".into())
        })
    }

    fn active_scope_lock(&self, scope: SubscriptionScope) -> AppResult<OwnedMutexGuard<()>> {
        let lock = self.scope_lock(scope)?;
        if self
            .inner
            .forgotten
            .lock()
            .map_err(|_| failure(ProviderErrorClass::Transport))?
            .contains(&scope)
        {
            return Err(AppError::NotFound);
        }
        Ok(lock)
    }

    fn scope_directory(&self, scope: SubscriptionScope) -> PathBuf {
        self.inner
            .root
            .join(scope.workspace_id.to_string())
            .join(scope.actor_id.to_string())
            .join(scope.connection_id.to_string())
    }

    fn paths(&self, scope: SubscriptionScope) -> AppResult<ScopePaths> {
        ScopePaths::new(&self.scope_directory(scope))
    }

    /// Returns only connection state. No email, account identifier or raw CLI output.
    ///
    /// # Errors
    /// Returns a sanitized error for an occupied connection, subprocess failure,
    /// deadline or an unsupported account response.
    pub async fn status(&self, scope: SubscriptionScope) -> AppResult<SubscriptionStatus> {
        let capability = self.capabilities().await;
        if !capability.available {
            return Ok(SubscriptionStatus {
                status: "unavailable",
                connected: false,
                available: false,
                message: capability.reason,
            });
        }
        let _lock = self.active_scope_lock(scope)?;
        let _slot = self
            .inner
            .slots
            .clone()
            .try_acquire_owned()
            .map_err(|_| failure(ProviderErrorClass::Transport))?;
        read_status(&self.inner.executable, &self.paths(scope)?).await
    }

    /// Starts the vendor's own flow. The host never accepts OAuth codes or tokens.
    ///
    /// # Errors
    /// Rejects unavailable clients, occupied scopes, excessive pending logins,
    /// inaccessible private storage or a failed subprocess launch.
    pub async fn start_login(&self, scope: SubscriptionScope) -> AppResult<SubscriptionLogin> {
        self.ensure_available().await?;
        let lock = self.active_scope_lock(scope)?;
        let slot = self
            .inner
            .slots
            .clone()
            .try_acquire_owned()
            .map_err(|_| failure(ProviderErrorClass::Transport))?;
        let paths = self.paths(scope)?;
        let login_id = Uuid::new_v4();
        let state = Arc::new(StdMutex::new(SubscriptionLogin {
            login_id,
            status: "pending",
            auth_url: None,
        }));
        let mut records = self
            .inner
            .logins
            .lock()
            .map_err(|_| failure(ProviderErrorClass::Transport))?;
        records.retain(|_, record| {
            !record.task.is_finished() || record.created.elapsed() < Duration::from_secs(300)
        });
        if records.len() >= MAX_LOGIN_RECORDS {
            return Err(AppError::Conflict(
                "Trop de connexions sont en cours.".into(),
            ));
        }
        let task_state = state.clone();
        let executable = self.inner.executable.clone();
        let task = tokio::spawn(async move {
            let (_lock, _slot) = (lock, slot);
            let result = run_command(
                &executable,
                &paths,
                &["--safe-mode", "auth", "login"],
                None,
                LOGIN_TIMEOUT,
                AUTH_OUTPUT_LIMIT,
                Some(task_state.clone()),
            )
            .await;
            let outcome = match result {
                Ok(output) if output.status.success() => {
                    match read_status(&executable, &paths).await {
                        Ok(status) if status.connected => "connected",
                        _ => "failed",
                    }
                }
                Err(AppError::Provider(error)) if error.class() == ProviderErrorClass::Timeout => {
                    "expired"
                }
                _ => "failed",
            };
            if let Ok(mut state) = task_state.lock() {
                state.status = outcome;
                state.auth_url = None;
            }
        });
        records.insert(
            login_id,
            LoginRecord {
                scope,
                state,
                task,
                created: Instant::now(),
            },
        );
        Ok(SubscriptionLogin {
            login_id,
            status: "pending",
            auth_url: None,
        })
    }

    /// A login id is not authority: all three scope identities must match.
    /// Returns the transient state of a login owned by this exact scope.
    ///
    /// # Errors
    /// Returns not found for an unknown login or another scope, and a sanitized
    /// error if the runtime state is inaccessible.
    #[allow(clippy::unused_async)] // Keep the polling interface async like the other runtime operations.
    pub async fn login_status(
        &self,
        scope: SubscriptionScope,
        login_id: Uuid,
    ) -> AppResult<SubscriptionLogin> {
        let records = self
            .inner
            .logins
            .lock()
            .map_err(|_| failure(ProviderErrorClass::Transport))?;
        let record = records
            .get(&login_id)
            .filter(|record| record.scope == scope)
            .ok_or(AppError::NotFound)?;
        record
            .state
            .lock()
            .map(|state| state.clone())
            .map_err(|_| failure(ProviderErrorClass::Transport))
    }

    /// Cancels an owned login and waits for its process group to terminate.
    ///
    /// # Errors
    /// Returns not found for an unknown login or another scope, and a sanitized
    /// error if the runtime state is inaccessible.
    pub async fn cancel_login(
        &self,
        scope: SubscriptionScope,
        login_id: Uuid,
    ) -> AppResult<SubscriptionLogin> {
        let record = {
            let mut records = self
                .inner
                .logins
                .lock()
                .map_err(|_| failure(ProviderErrorClass::Transport))?;
            if !records
                .get(&login_id)
                .is_some_and(|record| record.scope == scope)
            {
                return Err(AppError::NotFound);
            }
            records.remove(&login_id).ok_or(AppError::NotFound)?
        };
        record.task.abort();
        // Awaiting the aborted task ensures its process guard has run.
        let mut record = record;
        let _ = (&mut record.task).await;
        Ok(SubscriptionLogin {
            login_id,
            status: "cancelled",
            auth_url: None,
        })
    }

    /// Stops active work, invokes official logout and verifies disconnection.
    ///
    /// # Errors
    /// Returns a sanitized error when cancellation, client availability, private
    /// storage, the subprocess or its disconnection response cannot be verified.
    pub async fn logout(&self, scope: SubscriptionScope) -> AppResult<SubscriptionStatus> {
        let _lock = self.stop_and_lock(scope).await?;
        self.ensure_available().await?;
        let _slot = self
            .inner
            .slots
            .clone()
            .try_acquire_owned()
            .map_err(|_| failure(ProviderErrorClass::Transport))?;
        let paths = self.paths(scope)?;
        let result = run_command(
            &self.inner.executable,
            &paths,
            &["--safe-mode", "auth", "logout"],
            None,
            CLI_TIMEOUT,
            AUTH_OUTPUT_LIMIT,
            None,
        )
        .await?;
        if !result.status.success() {
            return Err(failure(ProviderErrorClass::Authentication));
        }
        let state = read_status(&self.inner.executable, &paths).await?;
        if state.connected {
            return Err(failure(ProviderErrorClass::ResponseContract));
        }
        Ok(state)
    }

    /// Removes only this connection's private CLI storage, even without a client.
    /// The connection UUID is permanently retired for this runtime instance.
    ///
    /// # Errors
    /// Returns a sanitized error if active work cannot be stopped or the private
    /// directory cannot be safely removed. The connection remains retired.
    pub async fn forget(&self, scope: SubscriptionScope) -> AppResult<()> {
        if !self.inner.root.is_absolute() {
            return Err(failure(ProviderErrorClass::Transport));
        }
        self.inner
            .forgotten
            .lock()
            .map_err(|_| failure(ProviderErrorClass::Transport))?
            .insert(scope);
        let _lock = self.stop_and_lock(scope).await?;
        let directory = self.scope_directory(scope);
        match std::fs::symlink_metadata(&directory) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(_) => return Err(failure(ProviderErrorClass::Transport)),
            Ok(_) => {}
        }
        private_directory(&directory)?;
        std::fs::remove_dir_all(directory).map_err(|_| failure(ProviderErrorClass::Transport))
    }

    async fn stop_and_lock(&self, scope: SubscriptionScope) -> AppResult<OwnedMutexGuard<()>> {
        tokio::time::timeout(CLI_TIMEOUT, async {
            loop {
                let ids = {
                    let records = self
                        .inner
                        .logins
                        .lock()
                        .map_err(|_| failure(ProviderErrorClass::Transport))?;
                    records
                        .iter()
                        .filter(|(_, record)| record.scope == scope)
                        .map(|(id, _)| *id)
                        .collect::<Vec<_>>()
                };
                for id in ids {
                    // Another cancellation may have removed the same record.
                    match self.cancel_login(scope, id).await {
                        Ok(_) | Err(AppError::NotFound) => {}
                        Err(error) => return Err(error),
                    }
                }
                if let Ok(generations) = self.inner.generations.lock()
                    && let Some(cancellation) = generations.get(&scope)
                {
                    cancellation.send_replace(true);
                }
                if let Ok(lock) = self.scope_lock(scope) {
                    return Ok(lock);
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .map_err(|_| failure(ProviderErrorClass::Timeout))?
    }

    #[must_use]
    pub fn transport(&self, scope: SubscriptionScope) -> Arc<dyn StructuredTransport> {
        Arc::new(ClaudeSubscriptionTransport {
            runtime: self.clone(),
            scope,
        })
    }
}

impl ScopePaths {
    fn new(base: &Path) -> AppResult<Self> {
        private_directory(base)?;
        let home = base.join("home");
        let config = base.join("claude");
        let cwd = base.join("work");
        for path in [
            &home,
            &config,
            &cwd,
            &home.join(".config"),
            &home.join(".cache"),
        ] {
            private_directory(path)?;
        }
        Ok(Self { home, config, cwd })
    }
}

#[cfg(unix)]
fn private_directory(path: &Path) -> AppResult<()> {
    use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
    // Walk each existing component to reject symlink redirection, including parents.
    let mut current = PathBuf::new();
    for component in path.components() {
        if matches!(component, std::path::Component::ParentDir) {
            return Err(failure(ProviderErrorClass::Transport));
        }
        current.push(component);
        match std::fs::symlink_metadata(&current) {
            Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                let mut builder = std::fs::DirBuilder::new();
                builder.mode(0o700);
                match builder.create(&current) {
                    Ok(()) => {}
                    Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
                    Err(_) => return Err(failure(ProviderErrorClass::Transport)),
                }
                let metadata = std::fs::symlink_metadata(&current)
                    .map_err(|_| failure(ProviderErrorClass::Transport))?;
                if !metadata.is_dir() || metadata.file_type().is_symlink() {
                    return Err(failure(ProviderErrorClass::Transport));
                }
            }
            Ok(_) | Err(_) => return Err(failure(ProviderErrorClass::Transport)),
        }
    }
    let metadata = std::fs::metadata(path).map_err(|_| failure(ProviderErrorClass::Transport))?;
    if metadata.permissions().mode() & 0o077 != 0 {
        return Err(failure(ProviderErrorClass::Transport));
    }
    Ok(())
}

#[cfg(not(unix))]
fn private_directory(_path: &Path) -> AppResult<()> {
    Err(failure(ProviderErrorClass::Transport))
}

fn managed_policy_present() -> bool {
    [
        "/etc/claude-code/managed-settings.json",
        "/etc/claude-code/managed-mcp.json",
        "/etc/claude-code/managed-settings.d",
        "/etc/claude-code/CLAUDE.md",
    ]
    .iter()
    .any(|path| match std::fs::symlink_metadata(path) {
        Ok(_) => true,
        Err(error) => error.kind() != std::io::ErrorKind::NotFound,
    })
}

fn supported_version(bytes: &[u8]) -> bool {
    // Updating this list requires auditing the CLI's no-action-tools contract.
    const AUDITED_VERSIONS: &[&str] = &["2.1.220"];
    let Ok(text) = std::str::from_utf8(bytes) else {
        return false;
    };
    let Some(version) = text.trim().strip_suffix(" (Claude Code)") else {
        return false;
    };
    AUDITED_VERSIONS.contains(&version)
}

fn configured_command(
    executable: &Path,
    paths: &ScopePaths,
    arguments: &[&str],
    has_input: bool,
) -> Command {
    let mut command = Command::new(executable);
    command
        .args(arguments)
        .current_dir(&paths.cwd)
        .env_clear()
        // Child-local paths, never mutations of the server's HOME or CODEX_HOME.
        .env("HOME", &paths.home)
        .env("CLAUDE_CONFIG_DIR", &paths.config)
        .env("XDG_CONFIG_HOME", paths.home.join(".config"))
        .env("XDG_CACHE_HOME", paths.home.join(".cache"))
        .env("TMPDIR", &paths.cwd)
        .env("PATH", "/usr/bin:/bin")
        .env("LANG", "C.UTF-8")
        .env("NO_COLOR", "1")
        // The UI presents the validated URL. Do not start a browser descendant.
        .env("BROWSER", "/bin/false")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .stdin(if has_input {
            Stdio::piped()
        } else {
            Stdio::null()
        });
    command
}

async fn run_command(
    executable: &Path,
    paths: &ScopePaths,
    arguments: &[&str],
    input: Option<&[u8]>,
    timeout: Duration,
    limit: usize,
    login: Option<Arc<StdMutex<SubscriptionLogin>>>,
) -> AppResult<ProcessOutput> {
    let mut command = configured_command(executable, paths, arguments, input.is_some());
    let mut process = ProcessGuard::spawn(&mut command)?;
    let stdout = process
        .child
        .stdout
        .take()
        .ok_or_else(|| failure(ProviderErrorClass::Transport))?;
    let stderr = process
        .child
        .stderr
        .take()
        .ok_or_else(|| failure(ProviderErrorClass::Transport))?;
    let stdin = process.child.stdin.take();
    let result = tokio::time::timeout(timeout, async {
        let (stdout, _, (), status) = tokio::try_join!(
            read_bounded(stdout, limit, login.clone()),
            read_bounded(stderr, AUTH_OUTPUT_LIMIT, login),
            async {
                if let (Some(mut pipe), Some(bytes)) = (stdin, input) {
                    pipe.write_all(bytes)
                        .await
                        .map_err(|_| failure(ProviderErrorClass::Transport))?;
                    pipe.shutdown()
                        .await
                        .map_err(|_| failure(ProviderErrorClass::Transport))?;
                }
                Ok(())
            },
            async {
                process
                    .child
                    .wait()
                    .await
                    .map_err(|_| failure(ProviderErrorClass::Transport))
            },
        )?;
        Ok(ProcessOutput { status, stdout })
    })
    .await;
    match result {
        Ok(result) => result,
        Err(_) => Err(failure(ProviderErrorClass::Timeout)),
    }
}

async fn read_bounded(
    mut pipe: impl AsyncRead + Unpin,
    limit: usize,
    login: Option<Arc<StdMutex<SubscriptionLogin>>>,
) -> AppResult<Vec<u8>> {
    let mut output = Vec::new();
    let mut buffer = [0_u8; 4096];
    loop {
        let read = pipe
            .read(&mut buffer)
            .await
            .map_err(|_| failure(ProviderErrorClass::Transport))?;
        if read == 0 {
            return Ok(output);
        }
        if output.len() + read > limit {
            return Err(failure(ProviderErrorClass::ResponseContract));
        }
        output.extend_from_slice(&buffer[..read]);
        if let Some(state) = &login
            && let Some(url) = official_auth_url(&output)
            && let Ok(mut state) = state.lock()
        {
            state.auth_url = Some(url);
        }
    }
}

fn official_auth_url(output: &[u8]) -> Option<String> {
    let text = std::str::from_utf8(output).ok()?;
    for part in text.split_whitespace() {
        // URLs are accepted only when a complete line/token has arrived.
        if !text.ends_with(char::is_whitespace) && text.ends_with(part) {
            continue;
        }
        let url = Url::parse(part).ok();
        if let Some(url) = url
            && url.scheme() == "https"
            && matches!(
                (url.host_str(), url.path()),
                (Some("claude.com"), "/cai/oauth/authorize")
                    | (
                        Some("claude.ai" | "platform.claude.com" | "console.anthropic.com"),
                        "/oauth/authorize" | "/oauth/authorize/"
                    )
            )
            && url.port().is_none()
            && url.username().is_empty()
            && url.password().is_none()
            && url.fragment().is_none()
            && !url.query_pairs().any(|(key, _)| {
                matches!(key.as_ref(), "access_token" | "refresh_token" | "id_token")
            })
            && !url
                .query_pairs()
                .any(|(key, value)| key == "code" && value != "true")
        {
            return Some(url.to_string());
        }
    }
    None
}

async fn read_status(executable: &Path, paths: &ScopePaths) -> AppResult<SubscriptionStatus> {
    let result = run_command(
        executable,
        paths,
        &["--safe-mode", "auth", "status", "--json"],
        None,
        CLI_TIMEOUT,
        AUTH_OUTPUT_LIMIT,
        None,
    )
    .await?;
    parse_status(&result)
}

fn parse_status(result: &ProcessOutput) -> AppResult<SubscriptionStatus> {
    let value: Value = serde_json::from_slice(&result.stdout)
        .map_err(|_| failure(ProviderErrorClass::ResponseContract))?;
    let logged_in = value
        .get("loggedIn")
        .and_then(Value::as_bool)
        .ok_or_else(|| failure(ProviderErrorClass::ResponseContract))?;
    if !logged_in && result.status.code() == Some(1) {
        return Ok(SubscriptionStatus {
            status: "disconnected",
            connected: false,
            available: true,
            message: None,
        });
    }
    if !logged_in || !result.status.success() {
        return Err(failure(ProviderErrorClass::ResponseContract));
    }
    if value.get("authMethod").and_then(Value::as_str) != Some("claude.ai") {
        return Ok(SubscriptionStatus { status: "unavailable", connected: false, available: false, message: Some("Cette connexion utilise une facturation API. Choisir une clé API ou un abonnement personnel dans le client officiel.".into()) });
    }
    // Team/Enterprise can fetch managed hooks at startup. Do not attempt to
    // override or bypass those policies through a local subprocess environment.
    if !matches!(
        value.get("subscriptionType").and_then(Value::as_str),
        Some("pro" | "max")
    ) {
        return Ok(SubscriptionStatus { status: "unavailable", connected: false, available: false, message: Some("Ce type de compte nécessite une vérification de ses politiques administrées avant utilisation sans outils.".into()) });
    }
    Ok(SubscriptionStatus {
        status: "connected",
        connected: true,
        available: true,
        message: None,
    })
}

fn failure(class: ProviderErrorClass) -> AppError {
    ProviderError::new(PROVIDER, class, 1, None).into()
}

struct GenerationRegistration {
    inner: Arc<RuntimeInner>,
    scope: SubscriptionScope,
}

impl Drop for GenerationRegistration {
    fn drop(&mut self) {
        if let Ok(mut generations) = self.inner.generations.lock() {
            generations.remove(&self.scope);
        }
    }
}

struct ClaudeSubscriptionTransport {
    runtime: SubscriptionRuntime,
    scope: SubscriptionScope,
}

#[async_trait]
impl StructuredTransport for ClaudeSubscriptionTransport {
    fn provider_name(&self) -> &'static str {
        PROVIDER
    }

    async fn list_models(&self) -> AppResult<Vec<ProviderModel>> {
        let status = self.runtime.status(self.scope).await?;
        if !status.connected {
            return Err(failure(ProviderErrorClass::Authentication));
        }
        Ok([
            ("sonnet", "Sonnet — alias Claude Code"),
            ("opus", "Opus — alias Claude Code"),
            ("haiku", "Haiku — alias Claude Code"),
        ]
        .into_iter()
        .map(|(id, name)| ProviderModel {
            id: id.into(),
            name: name.into(),
        })
        .collect())
    }

    async fn generate(
        &self,
        model: &str,
        operation_name: &str,
        instructions: &str,
        input: &Value,
        schema: &Value,
    ) -> AppResult<StructuredResponse> {
        if !model
            .as_bytes()
            .first()
            .is_some_and(u8::is_ascii_alphanumeric)
            || model.len() > 160
            || !model
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || b"-._[]".contains(&byte))
        {
            return Err(AppError::Invalid(
                "Identifiant de modèle abonnement invalide.".into(),
            ));
        }
        let payload = serde_json::to_vec(
            &json!({"operation": operation_name, "instructions": instructions, "input": input}),
        )
        .map_err(|_| failure(ProviderErrorClass::Request))?;
        let schema_text =
            serde_json::to_string(schema).map_err(|_| failure(ProviderErrorClass::Request))?;
        if payload.len() > INPUT_LIMIT || schema_text.len() > SCHEMA_LIMIT || !schema.is_object() {
            return Err(failure(ProviderErrorClass::Request));
        }
        self.runtime.ensure_available().await?;
        let _lock = self.runtime.active_scope_lock(self.scope)?;
        let _slot = self
            .runtime
            .inner
            .slots
            .clone()
            .try_acquire_owned()
            .map_err(|_| failure(ProviderErrorClass::Transport))?;
        let paths = self.runtime.paths(self.scope)?;
        if !read_status(&self.runtime.inner.executable, &paths)
            .await?
            .connected
        {
            return Err(failure(ProviderErrorClass::Authentication));
        }
        let arguments = [
            "--safe-mode",
            "-p",
            "--tools",
            "",
            "--disallowedTools",
            "mcp__*",
            "--strict-mcp-config",
            "--mcp-config",
            "{\"mcpServers\":{}}",
            "--disable-slash-commands",
            "--no-chrome",
            "--setting-sources",
            "",
            "--settings",
            "{\"disableAllHooks\":true}",
            "--permission-mode",
            "dontAsk",
            "--no-session-persistence",
            "--output-format",
            "json",
            "--model",
            model,
            "--json-schema",
            &schema_text,
            "--system-prompt",
            "Produce only the requested structured result from the supplied JSON input. Do not access tools, files, commands, external resources, or other context.",
        ];
        #[cfg(test)]
        let timeout = self.runtime.inner.generation_timeout;
        #[cfg(not(test))]
        let timeout = GENERATION_TIMEOUT;
        let started = Instant::now();
        let (sender, mut cancelled) = watch::channel(false);
        self.runtime
            .inner
            .generations
            .lock()
            .map_err(|_| failure(ProviderErrorClass::Transport))?
            .insert(self.scope, sender);
        let _registration = GenerationRegistration {
            inner: self.runtime.inner.clone(),
            scope: self.scope,
        };
        tokio::select! {
            result = run_command(&self.runtime.inner.executable, &paths, &arguments, Some(&payload), timeout, OUTPUT_LIMIT, None) => {
                parse_generation(&result?, model, started.elapsed())
            },
            _ = cancelled.changed() => Err(failure(ProviderErrorClass::Authentication)),
        }
    }
}

fn parse_generation(
    result: &ProcessOutput,
    model: &str,
    elapsed: Duration,
) -> AppResult<StructuredResponse> {
    let value: Value = serde_json::from_slice(&result.stdout)
        .map_err(|_| failure(ProviderErrorClass::ResponseContract))?;
    if !result.status.success() || value.get("is_error").and_then(Value::as_bool) == Some(true) {
        return Err(failure(ProviderErrorClass::Request));
    }
    if value.get("type").and_then(Value::as_str) != Some("result")
        || value.get("subtype").and_then(Value::as_str) != Some("success")
        || value.get("is_error").and_then(Value::as_bool) != Some(false)
        || value.get("stop_reason").is_some_and(|reason| {
            !reason.is_null()
                && !matches!(
                    reason.as_str(),
                    Some("end_turn" | "stop_sequence" | "tool_use")
                )
        })
        || value
            .get("permission_denials")
            .is_some_and(|denials| denials.as_array().is_none_or(|denials| !denials.is_empty()))
        || ["tool_use", "tool_calls", "messages", "content"]
            .iter()
            .any(|key| value.get(key).is_some())
    {
        return Err(failure(ProviderErrorClass::ResponseContract));
    }
    let output = value
        .get("structured_output")
        .filter(|output| output.is_object())
        .cloned()
        .ok_or_else(|| failure(ProviderErrorClass::ResponseContract))?;
    let served_model = match value.get("modelUsage") {
        Some(Value::Object(models)) if models.len() == 1 => models.keys().next().cloned(),
        _ => return Err(failure(ProviderErrorClass::ResponseContract)),
    };
    if let Some(served) = &served_model {
        let expected = match model {
            "sonnet" => served.starts_with("claude-sonnet-"),
            "opus" => served.starts_with("claude-opus-"),
            "haiku" => served.starts_with("claude-haiku-"),
            "fable" => served.starts_with("claude-fable-"),
            _ => served == model,
        };
        if !expected {
            return Err(failure(ProviderErrorClass::ResponseContract));
        }
    }
    let tokens = |key: &str| {
        value
            .get("usage")
            .and_then(|usage| usage.get(key))
            .and_then(Value::as_i64)
            .filter(|tokens| *tokens >= 0)
    };
    Ok(StructuredResponse {
        output,
        metadata: AgentRunMetadata {
            provider: PROVIDER.into(),
            requested_model: model.into(),
            served_model,
            provider_response_id: None,
            provider_request_id: None,
            status: Some("completed".into()),
            input_tokens: tokens("input_tokens"),
            output_tokens: tokens("output_tokens"),
            // CLI's total_cost_usd is an estimate, not a subscription charge.
            estimated_cost: None,
            latency_ms: i64::try_from(elapsed.as_millis()).unwrap_or(i64::MAX),
            attempts: 1,
        },
    })
}

#[cfg(all(test, unix))]
#[path = "provider_subscriptions_tests.rs"]
mod tests;
