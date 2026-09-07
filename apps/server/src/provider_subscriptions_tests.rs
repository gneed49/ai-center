use super::*;
use std::{fs, os::unix::fs::PermissionsExt};

struct Fixture {
    directory: PrivateDirectory,
    runtime: SubscriptionRuntime,
    events: PathBuf,
}

impl Fixture {
    fn new(mode: &str) -> Self {
        let directory = PrivateDirectory(
            std::env::temp_dir().join(format!("ai-center-subscription-test-{}", Uuid::new_v4())),
        );
        private_directory(&directory.0).unwrap();
        let executable = directory.0.join("fake-claude");
        let events = directory.0.join("events.jsonl");
        let settings = json!({"mode": mode, "events": events, "signal": directory.0.join("started"), "child_signal": directory.0.join("child-survived")});
        let literal = serde_json::to_string(&serde_json::to_string(&settings).unwrap()).unwrap();
        let script = format!(
            "#!/usr/bin/python3\nimport os, sys, json, time, subprocess\nfrom pathlib import Path\nCONFIG=json.loads({literal})\nFLAGS={}\n{}",
            serde_json::to_string(&REQUIRED_FLAGS).unwrap(),
            FAKE_CLI
        );
        fs::write(&executable, script).unwrap();
        fs::set_permissions(&executable, fs::Permissions::from_mode(0o700)).unwrap();
        let runtime = SubscriptionRuntime::new(directory.0.join("sessions"), executable, true);
        Self {
            directory,
            runtime,
            events,
        }
    }

    fn calls(&self) -> Vec<Value> {
        fs::read_to_string(&self.events)
            .unwrap_or_default()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect()
    }

    async fn started(&self) {
        tokio::time::timeout(Duration::from_secs(3), async {
            while !self.directory.0.join("started").exists() {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap();
    }
}

const FAKE_CLI: &str = r#"
args=sys.argv[1:]
config=Path(os.environ['CLAUDE_CONFIG_DIR'])
event={'args':args,'cwd':os.getcwd(),'config':str(config),'home':os.environ['HOME'],'env_keys':list(os.environ.keys())}
with open(CONFIG['events'],'a') as log: log.write(json.dumps(event)+'\n')
if '--version' in args:
    print('2.0.0 (Claude Code)' if CONFIG['mode']=='old' else '2.1.221 (Claude Code)' if CONFIG['mode']=='future' else '2.1.220 (Claude Code)')
    sys.exit(0)
if '--help' in args:
    print(' '.join(FLAGS[:-1] if CONFIG['mode']=='missing_flag' else FLAGS))
    sys.exit(0)
if 'auth' in args:
    state=config/'fake-state'
    if 'status' in args:
        state_value=state.read_text() if state.exists() else None
        connected=state_value!='disconnected' if state.exists() else CONFIG['mode'] not in ['disconnected','login','login_wait','login_api','login_team']
        api=CONFIG['mode'] in ['api','stuck_api'] or (CONFIG['mode']=='login_api' and state_value=='unsupported')
        team=CONFIG['mode'] in ['team','stuck_team'] or (CONFIG['mode']=='login_team' and state_value=='unsupported')
        print(json.dumps({'loggedIn':connected,'authMethod':'api_key' if api else 'claude.ai','apiProvider':'firstParty','subscriptionType':'team' if team else 'pro','email':'private-test@example.invalid','orgId':'private-test-org'}))
        sys.exit(0 if connected else 1)
    if 'logout' in args:
        if CONFIG['mode'] not in ['stuck_api','stuck_team']: state.write_text('disconnected')
        print('private logout message that must not be exposed')
        sys.exit(0)
    if 'login' in args:
        print('https://claude.com/cai/oauth/authorize?code=true&response_type=code&state=fake-state&client_id=fake-client',flush=True)
        Path(CONFIG['signal']).write_text(str(os.getpid()))
        if CONFIG['mode']=='login_wait':
            child=subprocess.Popen(['/usr/bin/python3','-c',"import time;from pathlib import Path;time.sleep(0.7);Path("+repr(CONFIG['child_signal'])+").write_text('survived');time.sleep(30)"])
            time.sleep(30)
        else:
            time.sleep(0.06)
            previous_login=config/'previous-login'
            state.write_text('unsupported' if CONFIG['mode'] in ['login_api','login_team'] and not previous_login.exists() else 'connected')
            previous_login.touch()
        sys.exit(0)
data=json.loads(sys.stdin.read())
with open(CONFIG['events'],'a') as log: log.write(json.dumps({'stdin':data})+'\n')
if CONFIG['mode']=='hang':
    child=subprocess.Popen(['/usr/bin/python3','-c',"import time;from pathlib import Path;time.sleep(0.7);Path("+repr(CONFIG['child_signal'])+").write_text('survived');time.sleep(30)"])
    Path(CONFIG['signal']).write_text(str(os.getpid()))
    time.sleep(30)
if CONFIG['mode']=='large':
    print('x'*2200000)
    time.sleep(30)
if CONFIG['mode']=='error':
    print(json.dumps({'type':'result','subtype':'error_during_execution','is_error':True,'result':'PRIVATE-PROMPT-AND-FAKE-KEY'}))
    print('PRIVATE-STDERR-FAKE-KEY',file=sys.stderr)
    sys.exit(1)
print(json.dumps({'type':'result','subtype':'success','is_error':False,'stop_reason':'tool_use','structured_output':{'summary':'fake summary'},'usage':{'input_tokens':17,'output_tokens':9},'modelUsage':{'claude-sonnet-4-6':{'inputTokens':17}},'permission_denials':[],'session_id':'private-cli-session','total_cost_usd':0.5}))
"#;

fn scope() -> SubscriptionScope {
    SubscriptionScope {
        workspace_id: Uuid::new_v4(),
        actor_id: Uuid::new_v4(),
        connection_id: Uuid::new_v4(),
    }
}

fn schema() -> Value {
    json!({"type":"object","properties":{"summary":{"type":"string"}},"required":["summary"],"additionalProperties":false})
}

async fn generate(
    runtime: &SubscriptionRuntime,
    scope: SubscriptionScope,
) -> AppResult<StructuredResponse> {
    runtime
        .transport(scope)
        .generate(
            "sonnet",
            "test",
            "PRIVATE-PROMPT",
            &json!({"private":"PRIVATE-CONTEXT"}),
            &schema(),
        )
        .await
}

#[tokio::test]
async fn disabled_missing_old_and_incomplete_clients_have_explicit_capabilities() {
    let disabled = SubscriptionRuntime::new(PathBuf::new(), PathBuf::new(), false);
    assert!(!disabled.capabilities().await.available);
    assert_eq!(
        disabled.status(scope()).await.unwrap().status,
        "unavailable"
    );
    let missing = SubscriptionRuntime::new(
        PathBuf::from("/tmp/example-subscription"),
        PathBuf::from("/missing/claude"),
        true,
    );
    assert!(!missing.capabilities().await.available);
    for mode in ["old", "future", "missing_flag"] {
        let fixture = Fixture::new(mode);
        assert!(!fixture.runtime.capabilities().await.available);
        assert!(fixture.calls().iter().all(|event| {
            !event["args"]
                .as_array()
                .unwrap()
                .iter()
                .any(|arg| arg == "auth")
        }));
    }
}

#[tokio::test]
async fn generation_uses_stdin_private_directories_and_removes_action_tools() {
    let fixture = Fixture::new("success");
    let scope = scope();
    let result = generate(&fixture.runtime, scope).await.unwrap();
    assert_eq!(result.output["summary"], "fake summary");
    assert_eq!(result.metadata.provider, PROVIDER);
    assert_eq!(
        result.metadata.served_model.as_deref(),
        Some("claude-sonnet-4-6")
    );
    assert_eq!(result.metadata.input_tokens, Some(17));
    assert_eq!(result.metadata.output_tokens, Some(9));
    assert!(result.metadata.provider_response_id.is_none());
    assert!(result.metadata.estimated_cost.is_none());
    let events = fixture.calls();
    let invocation = events
        .iter()
        .find(|event| {
            event["args"]
                .as_array()
                .is_some_and(|args| args.iter().any(|arg| arg == "-p"))
        })
        .unwrap();
    let args = invocation["args"].as_array().unwrap();
    for (flag, value) in [
        ("--tools", ""),
        ("--disallowedTools", "mcp__*"),
        ("--mcp-config", "{\"mcpServers\":{}}"),
        ("--setting-sources", ""),
        ("--permission-mode", "dontAsk"),
    ] {
        let index = args.iter().position(|arg| arg == flag).unwrap();
        assert_eq!(args[index + 1], value);
    }
    assert!(!invocation.to_string().contains("PRIVATE-PROMPT"));
    assert!(!invocation.to_string().contains("PRIVATE-CONTEXT"));
    let environment = invocation["env_keys"].as_array().unwrap();
    assert!(
        !environment.iter().any(|key| key
            .as_str()
            .is_some_and(|key| key.starts_with("ANTHROPIC_")
                || key.starts_with("OPENAI_")
                || key.contains("TOKEN")))
    );
    let expected = fixture
        .runtime
        .inner
        .root
        .join(scope.workspace_id.to_string())
        .join(scope.actor_id.to_string())
        .join(scope.connection_id.to_string());
    assert!(Path::new(invocation["config"].as_str().unwrap()).starts_with(&expected));
    assert!(Path::new(invocation["home"].as_str().unwrap()).starts_with(&expected));
    assert_eq!(
        fs::metadata(&expected).unwrap().permissions().mode() & 0o777,
        0o700
    );
    assert_eq!(
        events.iter().find_map(|event| event.get("stdin")).unwrap()["instructions"],
        "PRIVATE-PROMPT"
    );
}

#[tokio::test]
async fn status_never_exposes_account_fields_and_refuses_api_or_managed_accounts() {
    let fixture = Fixture::new("success");
    let status = fixture.runtime.status(scope()).await.unwrap();
    assert!(status.connected);
    assert_eq!(status.authenticated, Some(true));
    let public = serde_json::to_string(&status).unwrap();
    assert!(!public.contains("email"));
    assert!(!public.contains("private-test"));
    for mode in ["api", "team", "disconnected"] {
        let fixture = Fixture::new(mode);
        let status = fixture.runtime.status(scope()).await.unwrap();
        assert!(!status.connected);
        assert_eq!(status.authenticated, Some(mode != "disconnected"));
        assert!(generate(&fixture.runtime, scope()).await.is_err());
        assert!(
            !fixture
                .calls()
                .iter()
                .any(|event| event.get("stdin").is_some())
        );
    }
}

#[tokio::test]
async fn login_can_be_polled_only_by_its_exact_scope_and_logout_is_verified() {
    let fixture = Fixture::new("login");
    let owner = scope();
    let login = fixture.runtime.start_login(owner).await.unwrap();
    assert_eq!(login.status, "pending");
    assert!(login.auth_url.is_none());
    for intruder in [
        SubscriptionScope {
            actor_id: Uuid::new_v4(),
            ..owner
        },
        SubscriptionScope {
            workspace_id: Uuid::new_v4(),
            ..owner
        },
        SubscriptionScope {
            connection_id: Uuid::new_v4(),
            ..owner
        },
    ] {
        assert!(matches!(
            fixture.runtime.login_status(intruder, login.login_id).await,
            Err(AppError::NotFound)
        ));
        assert!(matches!(
            fixture.runtime.cancel_login(intruder, login.login_id).await,
            Err(AppError::NotFound)
        ));
    }
    let complete = tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            let status = fixture
                .runtime
                .login_status(owner, login.login_id)
                .await
                .unwrap();
            if status.status != "pending" {
                break status;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    assert_eq!(complete.status, "connected");
    assert!(complete.auth_url.is_none());
    assert_eq!(
        fixture.runtime.logout(owner).await.unwrap().status,
        "disconnected"
    );
}

#[tokio::test]
async fn cancelling_login_kills_its_process_group_and_hides_the_url() {
    let fixture = Fixture::new("login_wait");
    let owner = scope();
    let login = fixture.runtime.start_login(owner).await.unwrap();
    fixture.started().await;
    let pending = fixture
        .runtime
        .login_status(owner, login.login_id)
        .await
        .unwrap();
    assert!(
        pending
            .auth_url
            .as_ref()
            .is_some_and(|url| url.starts_with("https://claude.com/cai/oauth/authorize"))
    );
    assert!(fixture.runtime.start_login(owner).await.is_err());
    let cancelled = fixture
        .runtime
        .cancel_login(owner, login.login_id)
        .await
        .unwrap();
    assert_eq!(cancelled.status, "cancelled");
    assert!(cancelled.auth_url.is_none());
    tokio::time::sleep(Duration::from_millis(850)).await;
    assert!(!fixture.directory.0.join("child-survived").exists());
}

#[tokio::test]
async fn dropping_the_last_runtime_cancels_owned_login_processes() {
    let fixture = Fixture::new("login_wait");
    fixture.runtime.start_login(scope()).await.unwrap();
    fixture.started().await;
    drop(fixture.runtime);
    tokio::time::sleep(Duration::from_millis(850)).await;
    assert!(!fixture.directory.0.join("child-survived").exists());
}

#[tokio::test]
async fn deadline_and_future_cancellation_kill_generation_descendants() {
    let mut fixture = Fixture::new("hang");
    Arc::get_mut(&mut fixture.runtime.inner)
        .unwrap()
        .generation_timeout = Duration::from_millis(160);
    let error = generate(&fixture.runtime, scope()).await.unwrap_err();
    assert!(
        matches!(error, AppError::Provider(error) if error.class() == ProviderErrorClass::Timeout)
    );
    tokio::time::sleep(Duration::from_millis(750)).await;
    assert!(!fixture.directory.0.join("child-survived").exists());
    let fixture = Fixture::new("hang");
    let runtime = fixture.runtime.clone();
    let task = tokio::spawn(async move { generate(&runtime, scope()).await });
    fixture.started().await;
    task.abort();
    let _ = task.await;
    tokio::time::sleep(Duration::from_millis(850)).await;
    assert!(!fixture.directory.0.join("child-survived").exists());
}

#[tokio::test]
async fn logout_cancels_a_running_generation_before_disconnecting() {
    let fixture = Fixture::new("hang");
    let runtime = fixture.runtime.clone();
    let owner = scope();
    let task = tokio::spawn(async move { generate(&runtime, owner).await });
    fixture.started().await;
    assert_eq!(
        fixture.runtime.logout(owner).await.unwrap().status,
        "disconnected"
    );
    assert!(task.await.unwrap().is_err());
    tokio::time::sleep(Duration::from_millis(850)).await;
    assert!(!fixture.directory.0.join("child-survived").exists());
}

#[tokio::test]
async fn provider_errors_and_excessive_output_are_bounded_and_sanitized() {
    for mode in ["error", "large"] {
        let fixture = Fixture::new(mode);
        let error =
            tokio::time::timeout(Duration::from_secs(3), generate(&fixture.runtime, scope()))
                .await
                .unwrap()
                .unwrap_err();
        assert!(!error.to_string().contains("PRIVATE"));
        assert!(!error.public_message().contains("FAKE-KEY"));
    }
}

#[tokio::test]
async fn separate_connections_never_share_the_official_session_directory() {
    let fixture = Fixture::new("success");
    let first = scope();
    let second = SubscriptionScope {
        connection_id: Uuid::new_v4(),
        ..first
    };
    fixture.runtime.logout(first).await.unwrap();
    assert!(!fixture.runtime.status(first).await.unwrap().connected);
    assert!(fixture.runtime.status(second).await.unwrap().connected);
}

#[test]
fn authorization_url_accepts_only_official_complete_noncredential_urls() {
    let allowed = "https://claude.com/cai/oauth/authorize?code=true&state=fake-state\n";
    assert!(official_auth_url(allowed.as_bytes()).is_some());
    assert!(official_auth_url(allowed.trim().as_bytes()).is_none());
    for url in [
        "https://claude.com.attacker.invalid/cai/oauth/authorize",
        "http://claude.com/cai/oauth/authorize",
        "https://claude.com/cai/oauth/authorize#fragment",
        "https://user:password@claude.com/cai/oauth/authorize",
        "https://claude.com:444/cai/oauth/authorize",
        "https://claude.com/cai/oauth/authorize?access_token=fake-secret",
        "https://claude.com/cai/oauth/authorize?code=fake-secret",
        "https://claude.com/other-path",
    ] {
        assert!(
            official_auth_url(format!("{url}\n").as_bytes()).is_none(),
            "{url}"
        );
    }
}

#[test]
fn unexpected_tool_envelopes_truncation_and_model_substitution_are_rejected() {
    use std::os::unix::process::ExitStatusExt;
    let base = json!({"type":"result","subtype":"success","is_error":false,"structured_output":{"summary":"ok"},"modelUsage":{"claude-sonnet-4-6":{}},"stop_reason":"tool_use","permission_denials":[]});
    for (field, value) in [
        ("type", json!("assistant")),
        ("subtype", json!("error_max_structured_output_retries")),
        ("is_error", json!(true)),
        ("stop_reason", json!("max_tokens")),
        ("structured_output", Value::Null),
        ("tool_calls", json!([])),
        ("content", json!([])),
        ("permission_denials", json!([{"tool_name":"Bash"}])),
        ("stop_reason", json!({"unexpected":"contract"})),
        ("modelUsage", json!({"claude-opus-4-6":{}})),
        (
            "modelUsage",
            json!({"claude-sonnet-4-6":{},"claude-opus-4-6":{}}),
        ),
    ] {
        let mut body = base.clone();
        body[field] = value;
        let output = ProcessOutput {
            status: ExitStatus::from_raw(0),
            stdout: serde_json::to_vec(&body).unwrap(),
        };
        assert!(
            parse_generation(&output, "sonnet", Duration::ZERO).is_err(),
            "{field}"
        );
    }
}

#[test]
fn symlinked_private_directories_are_refused() {
    let fixture = Fixture::new("success");
    let linked = fixture.directory.0.join("linked");
    std::os::unix::fs::symlink(&fixture.directory.0, &linked).unwrap();
    assert!(private_directory(&linked.join("child")).is_err());
    assert!(private_directory(&fixture.directory.0.join("../elsewhere")).is_err());
}

#[test]
fn scope_locks_never_conflict_between_distinct_connections() {
    let runtime = SubscriptionRuntime::new(PathBuf::new(), PathBuf::new(), false);
    let scopes = (0..128).map(|_| scope()).collect::<Vec<_>>();
    let guards = scopes
        .iter()
        .map(|scope| runtime.scope_lock(*scope).unwrap())
        .collect::<Vec<_>>();
    assert!(runtime.scope_lock(scopes[0]).is_err());
    drop(guards);
    assert!(runtime.scope_lock(scopes[0]).is_ok());
    assert_eq!(runtime.inner.locks.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn forgetting_private_storage_needs_no_client_and_retires_the_scope() {
    let fixture = Fixture::new("success");
    let owner = scope();
    let other = scope();
    let private = fixture.runtime.paths(owner).unwrap();
    let preserved = fixture.runtime.paths(other).unwrap();
    fs::write(private.config.join("fake-credential"), "fixture-only").unwrap();
    fs::write(preserved.config.join("fake-credential"), "fixture-only").unwrap();
    fs::remove_file(&fixture.runtime.inner.executable).unwrap();
    fixture.runtime.forget(owner).await.unwrap();
    assert!(!fixture.runtime.scope_directory(owner).exists());
    assert!(preserved.config.join("fake-credential").exists());
    assert!(fixture.calls().is_empty());
    assert!(matches!(
        fixture.runtime.active_scope_lock(owner),
        Err(AppError::NotFound)
    ));
    fixture.runtime.forget(owner).await.unwrap();
}

#[tokio::test]
async fn forgetting_stops_login_and_blocks_late_attempts() {
    let fixture = Fixture::new("login_wait");
    let owner = scope();
    fixture.runtime.start_login(owner).await.unwrap();
    fixture.started().await;
    fixture.runtime.forget(owner).await.unwrap();
    assert!(!fixture.runtime.scope_directory(owner).exists());
    assert!(matches!(
        fixture.runtime.start_login(owner).await,
        Err(AppError::NotFound)
    ));
    tokio::time::sleep(Duration::from_millis(850)).await;
    assert!(!fixture.directory.0.join("child-survived").exists());
}

async fn complete_login(fixture: &Fixture, owner: SubscriptionScope) -> SubscriptionLogin {
    let login = fixture.runtime.start_login(owner).await.unwrap();
    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            let status = fixture
                .runtime
                .login_status(owner, login.login_id)
                .await
                .unwrap();
            if status.status != "pending" {
                break status;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap()
}

#[tokio::test]
async fn unsupported_authenticated_accounts_can_logout_and_change_to_personal_subscription() {
    for mode in ["login_api", "login_team"] {
        let fixture = Fixture::new(mode);
        let owner = scope();
        assert_eq!(complete_login(&fixture, owner).await.status, "failed");
        let status = fixture.runtime.status(owner).await.unwrap();
        assert_eq!(status.authenticated, Some(true));
        assert!(!status.connected && !status.available);
        assert!(generate(&fixture.runtime, owner).await.is_err());
        assert!(
            !fixture
                .calls()
                .iter()
                .any(|event| event.get("stdin").is_some())
        );
        let disconnected = fixture.runtime.logout(owner).await.unwrap();
        assert_eq!(disconnected.authenticated, Some(false));
        assert_eq!(disconnected.status, "disconnected");
        assert_eq!(complete_login(&fixture, owner).await.status, "connected");
        assert!(fixture.runtime.status(owner).await.unwrap().connected);
        assert!(generate(&fixture.runtime, owner).await.is_ok());
    }
}

#[tokio::test]
async fn logout_rejects_a_client_that_keeps_an_unsupported_account_authenticated() {
    for mode in ["stuck_api", "stuck_team"] {
        let fixture = Fixture::new(mode);
        let owner = scope();
        let error = fixture.runtime.logout(owner).await.unwrap_err();
        assert_eq!(error.model_run_error_class(), "provider_response_contract");
        assert_eq!(
            fixture.runtime.status(owner).await.unwrap().authenticated,
            Some(true)
        );
    }
}
