#[cfg(unix)]
use super::codex::collect_app_server_child_with_pgid;
use super::{
    broker::{CredentialBroker, CredentialLocations},
    claude::{parse_usage, ClaudeRequestSpec},
    codex::{
        classify_spawn_error, collect_app_server, parse_pgid_handshake, parse_rate_limits,
        resolve_codex_executable, CodexCollector, RpcSession,
    },
    fallback::{FallbackCollector, FallbackTrigger},
    wsl::{
        codex_cleanup_args, validate_system_wsl_path, ProcessOutput, WslCodexCollector,
        WslCommandFactory, WslCredentialSource, CLAUDE_CREDENTIAL_SCRIPT, CODEX_CLEANUP_SCRIPT,
        CODEX_LAUNCH_SCRIPT, CODEX_PROBE_SCRIPT,
    },
    Collector, CollectorError,
};
use crate::domain::{CollectionOutcome, FailureClass, Provider, ProviderUsageSnapshot, Source};
use secrecy::{ExposeSecret, SecretString};
use std::{
    collections::BTreeMap,
    ffi::OsStr,
    fs,
    path::PathBuf,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};
use time::OffsetDateTime;

static TEMP_DIR_SEQUENCE: AtomicUsize = AtomicUsize::new(0);

#[derive(Clone)]
struct FakeWslProcess {
    expected_args: Vec<String>,
    result: Result<ProcessOutput, CollectorError>,
}

impl super::wsl::WslProcess for FakeWslProcess {
    fn run<'a>(
        &'a self,
        args: &'a [&'a str],
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<ProcessOutput, CollectorError>> + Send + 'a>,
    > {
        assert_eq!(
            args,
            self.expected_args
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>()
        );
        let result = self.result.clone();
        Box::pin(async move { result })
    }
}

fn fake_wsl(result: Result<ProcessOutput, CollectorError>) -> WslCredentialSource {
    let process = FakeWslProcess {
        expected_args: vec![
            "--exec".into(),
            "sh".into(),
            "-c".into(),
            CLAUDE_CREDENTIAL_SCRIPT.into(),
        ],
        result,
    };
    WslCredentialSource::new(WslCommandFactory::with_process_for_test(Arc::new(process)))
}

#[test]
fn wsl_codex_arguments_are_fixed() {
    assert_eq!(
        CODEX_PROBE_SCRIPT,
        "bash -lc 'command -v codex >/dev/null 2>&1'"
    );
    assert!(CODEX_LAUNCH_SCRIPT.contains("exec codex -s read-only -a untrusted app-server"));
    assert!(CODEX_LAUNCH_SCRIPT.contains("setsid --wait bash -lc"));
    assert!(!CODEX_LAUNCH_SCRIPT.contains("\\nCACHEBITE_PGID"));
    assert!(CODEX_LAUNCH_SCRIPT.contains("CACHEBITE_PGID:%s"));
    assert!(CODEX_CLEANUP_SCRIPT.contains("*[!0-9]*"));
    assert!(CODEX_CLEANUP_SCRIPT.contains("kill -KILL -- \"-$1\""));
    assert!(CODEX_CLEANUP_SCRIPT.contains("kill -0 -- \"-$1\""));
    assert!(CODEX_CLEANUP_SCRIPT.contains("\"$i\" -lt 20"));
    assert_eq!(codex_cleanup_args(4242)[5], "4242");
}

#[test]
fn wsl_codex_missing_launcher_maps_to_cli_missing() {
    assert_eq!(
        classify_spawn_error(std::io::ErrorKind::NotFound),
        CollectorError::CliMissing
    );
    assert_eq!(
        classify_spawn_error(std::io::ErrorKind::PermissionDenied),
        CollectorError::Internal
    );
}

#[test]
fn wsl_codex_handshake_requires_bounded_numeric_pgid() {
    assert_eq!(parse_pgid_handshake(b"CACHEBITE_PGID:4242"), Ok(4242));
    // wsl.exe emits CRLF line endings; a trailing CR must not fail parsing.
    assert_eq!(parse_pgid_handshake(b"CACHEBITE_PGID:4242\r"), Ok(4242));
    let oversized = format!("CACHEBITE_PGID:{}", "1".repeat(80));
    for invalid in [
        b"invalid".as_slice(),
        b"CACHEBITE_PGID:0".as_slice(),
        b"CACHEBITE_PGID:not-a-number".as_slice(),
        oversized.as_bytes(),
    ] {
        assert_eq!(parse_pgid_handshake(invalid), Err(CollectorError::Protocol));
    }
}

#[derive(Default)]
struct RecordingWslProcess {
    calls: Mutex<Vec<Vec<String>>>,
}

impl super::wsl::WslProcess for RecordingWslProcess {
    fn run<'a>(
        &'a self,
        args: &'a [&'a str],
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<ProcessOutput, CollectorError>> + Send + 'a>,
    > {
        self.calls
            .lock()
            .unwrap()
            .push(args.iter().map(|arg| (*arg).to_owned()).collect());
        Box::pin(std::future::ready(Ok(ProcessOutput {
            status: 0,
            stdout: Vec::new(),
        })))
    }
}

#[tokio::test]
async fn wsl_codex_probes_before_reporting_missing_launcher() {
    let process = Arc::new(RecordingWslProcess::default());
    let collector =
        WslCodexCollector::new(WslCommandFactory::with_process_and_executable_for_test(
            process.clone(),
            PathBuf::from("Z:\\definitely-missing\\wsl.exe"),
        ));
    assert_eq!(collector.collect().await, CollectionOutcome::CliMissing);
    let calls = process.calls.lock().unwrap();
    assert_eq!(calls[0], ["--exec", "sh", "-c", CODEX_PROBE_SCRIPT]);
    assert_eq!(calls.len(), 1);
}

#[cfg(unix)]
fn fake_wsl_codex(script: &str) -> (TempDir, WslCodexCollector) {
    use std::os::unix::fs::PermissionsExt;
    let root = tempdir();
    let executable = root.path().join("fake-wsl.exe");
    fs::write(&executable, script).unwrap();
    fs::set_permissions(&executable, fs::Permissions::from_mode(0o700)).unwrap();
    let executable = fs::canonicalize(executable).unwrap();
    let factory = WslCommandFactory::with_executable_for_test(executable);
    (root, WslCodexCollector::new(factory))
}

#[cfg(unix)]
#[tokio::test]
async fn wsl_codex_uses_exact_fixed_arguments_and_shared_rpc() {
    let (root, collector) = fake_wsl_codex(
        "#!/bin/sh\nprintf '%s\\n' \"$@\" > \"$(dirname \"$0\")/args\"\nprintf 'CACHEBITE_PGID:%s\\n' $$\nread first\nprintf '%s\\n' '{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{}}'\nread second\nprintf '%s\\n' '{\"jsonrpc\":\"2.0\",\"id\":2,\"result\":{\"primary\":{\"usedPercent\":3}}}'\nsleep 30\n",
    );

    assert!(matches!(
        collector.collect().await,
        CollectionOutcome::Success { .. }
    ));
    assert_eq!(
        fs::read_to_string(root.path().join("args"))
            .unwrap()
            .as_str(),
        format!("--exec\nsh\n-c\n{CODEX_LAUNCH_SCRIPT}\n")
    );
}

#[cfg(unix)]
#[tokio::test]
async fn wsl_codex_maps_missing_launcher_to_cli_missing() {
    let collector = WslCodexCollector::new(WslCommandFactory::with_executable_for_test(
        PathBuf::from("/definitely/missing/wsl.exe"),
    ));
    assert_eq!(collector.collect().await, CollectionOutcome::CliMissing);
}

#[cfg(unix)]
#[tokio::test]
async fn wsl_codex_skips_login_noise_and_rejects_oversized_pgid() {
    // Login-shell profile output printed before the marker must be skipped.
    let (_noisy_root, noisy) = fake_wsl_codex(
        "#!/bin/sh\nprintf '%s\\n' 'Welcome to Ubuntu 24.04 LTS'\nprintf '%s' 'nvm partial line'\nprintf '\\nCACHEBITE_PGID:%s\\n' $$\nread first\nprintf '%s\\n' '{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{}}'\nread second\nprintf '%s\\n' '{\"jsonrpc\":\"2.0\",\"id\":2,\"result\":{\"primary\":{\"usedPercent\":3}}}'\nsleep 30\n",
    );
    assert!(matches!(
        noisy.collect().await,
        CollectionOutcome::Success { .. }
    ));

    // A marker that is present but malformed (oversized) still fails as Parse.
    let script = format!(
        "#!/bin/sh\nprintf 'CACHEBITE_PGID:%s\\n' '{}'\nsleep 30\n",
        "1".repeat(80)
    );
    let (_oversized_root, oversized) = fake_wsl_codex(&script);
    assert_eq!(
        oversized.collect().await,
        CollectionOutcome::Failed {
            class: FailureClass::Parse
        }
    );
}

#[cfg(unix)]
#[tokio::test]
async fn wsl_codex_cleanup_failure_is_propagated_after_child_reaping() {
    use std::os::unix::fs::PermissionsExt;
    let root = tempdir();
    let executable = root.path().join("cleanup-failure-wsl");
    let pid_file = root.path().join("pid");
    let script = format!(
        "#!/bin/sh\necho $$ > '{}'\nprintf 'CACHEBITE_PGID:%s\\n' $$\nread first\nprintf '%s\\n' '{{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{{}}}}'\nread second\nprintf '%s\\n' '{{\"jsonrpc\":\"2.0\",\"id\":2,\"result\":{{\"primary\":{{\"usedPercent\":3}}}}}}'\nsleep 30\n",
        pid_file.display()
    );
    fs::write(&executable, script).unwrap();
    fs::set_permissions(&executable, fs::Permissions::from_mode(0o700)).unwrap();
    let result = collect_app_server_child_with_pgid(
        tokio::process::Command::new(executable),
        OffsetDateTime::UNIX_EPOCH,
        Duration::from_secs(1),
        |_| async { Err(CollectorError::Internal) },
    )
    .await;
    assert_eq!(result, Err(CollectorError::Internal));
    let pid: i32 = fs::read_to_string(pid_file)
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    assert_eq!(unsafe { libc::kill(pid, 0) }, -1);
}

#[cfg(unix)]
#[tokio::test]
async fn wsl_codex_times_out_and_reaps_hanging_launcher() {
    let (root, _) =
        fake_wsl_codex("#!/bin/sh\necho $$ > \"$(dirname \"$0\")/pid\"\nprintf 'CACHEBITE_PGID:%s\\n' $$\nread first\nsleep 30\n");
    let executable = root.path().join("fake-wsl.exe");
    let collector = WslCodexCollector::with_timeout_for_test(
        WslCommandFactory::with_executable_for_test(executable),
        Duration::from_millis(100),
    );
    assert_eq!(
        collector.collect().await,
        CollectionOutcome::Failed {
            class: FailureClass::Network
        }
    );
    let pid: i32 = fs::read_to_string(root.path().join("pid"))
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    assert_eq!(unsafe { libc::kill(pid, 0) }, -1);
}

#[cfg(unix)]
#[tokio::test]
async fn wsl_codex_cancellation_kills_and_reaps_fake_launcher() {
    let (root, _) = fake_wsl_codex(
        "#!/bin/sh\necho $$ > \"$(dirname \"$0\")/pid\"\nsleep 30 &\necho $! > \"$(dirname \"$0\")/child-pid\"\nsleep 30\n",
    );
    let collector = WslCodexCollector::with_timeout_for_test(
        WslCommandFactory::with_executable_for_test(root.path().join("fake-wsl.exe")),
        Duration::from_millis(100),
    );
    let task = tokio::spawn(async move { collector.collect().await });
    let pid_file = root.path().join("pid");
    let child_pid_file = root.path().join("child-pid");
    for _ in 0..100 {
        if pid_file.exists() && child_pid_file.exists() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
    let pid: i32 = fs::read_to_string(pid_file)
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    let child_pid: i32 = fs::read_to_string(child_pid_file)
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    task.abort();
    let _ = task.await;
    for _ in 0..100 {
        if unsafe { libc::kill(pid, 0) } == -1 && unsafe { libc::kill(child_pid, 0) } == -1 {
            return;
        }
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
    panic!("cancelled WSL app-server launcher was not reaped");
}

#[test]
fn wsl_claude_script_is_fixed_and_checks_documented_locations() {
    assert_eq!(
        CLAUDE_CREDENTIAL_SCRIPT,
        "if [ -n \"${CLAUDE_CONFIG_DIR:-}\" ] && [ -f \"${CLAUDE_CONFIG_DIR}/.credentials.json\" ]; then cat -- \"${CLAUDE_CONFIG_DIR}/.credentials.json\"; elif [ -f \"${HOME}/.claude/.credentials.json\" ]; then cat -- \"${HOME}/.claude/.credentials.json\"; else exit 44; fi"
    );
}

#[tokio::test]
async fn wsl_claude_parses_secret_from_bounded_output() {
    let source = fake_wsl(Ok(ProcessOutput {
        status: 0,
        stdout: br#"{"claudeAiOauth":{"accessToken":"wsl-secret"}}"#.to_vec(),
    }));
    assert_eq!(
        source.claude_token().await.unwrap().expose_secret(),
        "wsl-secret"
    );
}

#[tokio::test]
async fn wsl_claude_maps_absence_and_rejects_invalid_or_oversized_credentials() {
    assert!(matches!(
        fake_wsl(Ok(ProcessOutput {
            status: 44,
            stdout: vec![]
        }))
        .claude_token()
        .await,
        Err(CollectorError::CredentialsMissing)
    ));
    assert!(matches!(
        fake_wsl(Err(CollectorError::CredentialsMissing))
            .claude_token()
            .await,
        Err(CollectorError::CredentialsMissing)
    ));
    assert!(matches!(
        fake_wsl(Err(CollectorError::Internal)).claude_token().await,
        Err(CollectorError::Internal)
    ));
    assert!(matches!(
        fake_wsl(Ok(ProcessOutput {
            status: 0,
            stdout: b"not-json".to_vec()
        }))
        .claude_token()
        .await,
        Err(CollectorError::CredentialFileInvalid)
    ));
    assert!(matches!(
        fake_wsl(Ok(ProcessOutput {
            status: 0,
            stdout: vec![b'x'; 64 * 1024 + 1]
        }))
        .claude_token()
        .await,
        Err(CollectorError::CredentialFileInvalid)
    ));
}

#[tokio::test]
async fn wsl_claude_diagnostics_do_not_expose_output() {
    let diagnostic_marker = "token-and-home-path-must-not-appear";
    let error = fake_wsl(Ok(ProcessOutput {
        status: 1,
        stdout: diagnostic_marker.as_bytes().to_vec(),
    }))
    .claude_token()
    .await
    .unwrap_err();
    assert_eq!(error, CollectorError::CredentialsMissing);
    assert!(!error.to_string().contains(diagnostic_marker));
}

#[test]
fn wsl_claude_resolver_accepts_only_regular_non_reparse_system_binary() {
    let root = tempdir();
    let system = root.path().join("System32");
    fs::create_dir(&system).unwrap();
    let executable = system.join("wsl.exe");
    fs::write(&executable, b"fixture").unwrap();
    assert_eq!(validate_system_wsl_path(&system).unwrap(), executable);

    fs::remove_file(&executable).unwrap();
    assert!(matches!(
        validate_system_wsl_path(&system),
        Err(CollectorError::CredentialsMissing)
    ));
}

#[cfg(unix)]
#[test]
fn wsl_claude_resolver_rejects_reparse_equivalent_binary() {
    use std::os::unix::fs::symlink;
    let root = tempdir();
    let system = root.path().join("System32");
    fs::create_dir(&system).unwrap();
    let target = root.path().join("target.exe");
    fs::write(&target, b"fixture").unwrap();
    symlink(target, system.join("wsl.exe")).unwrap();
    assert!(matches!(
        validate_system_wsl_path(&system),
        Err(CollectorError::CredentialsMissing)
    ));
}

struct CountingCollector {
    provider: Provider,
    outcome: CollectionOutcome,
    calls: Arc<AtomicUsize>,
}

impl Collector for CountingCollector {
    fn provider(&self) -> Provider {
        self.provider
    }

    fn collect(
        &self,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = CollectionOutcome> + Send + '_>> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Box::pin(std::future::ready(self.outcome.clone()))
    }
}

fn counting_collector(
    provider: Provider,
    outcome: CollectionOutcome,
) -> (CountingCollector, Arc<AtomicUsize>) {
    let calls = Arc::new(AtomicUsize::new(0));
    (
        CountingCollector {
            provider,
            outcome,
            calls: Arc::clone(&calls),
        },
        calls,
    )
}

fn fallback_success(provider: Provider) -> CollectionOutcome {
    CollectionOutcome::Success {
        snapshot: ProviderUsageSnapshot {
            provider,
            plan_type: Some("fallback".to_owned()),
            session: None,
            weekly: None,
            captured_at: OffsetDateTime::UNIX_EPOCH,
            source: if provider == Provider::Claude {
                Source::OauthApi
            } else {
                Source::CliRpc
            },
            is_cached: false,
            revision: 0,
        },
    }
}

#[tokio::test]
async fn fallback_native_success_is_returned_without_invoking_fallback() {
    let native = fallback_success(Provider::Claude);
    let (primary, primary_calls) = counting_collector(Provider::Claude, native.clone());
    let (secondary, secondary_calls) =
        counting_collector(Provider::Claude, CollectionOutcome::CredentialsMissing);
    let collector = FallbackCollector::new(
        Box::new(primary),
        Box::new(secondary),
        FallbackTrigger::CredentialsMissing,
    )
    .expect("matching providers should compose");

    assert_eq!(collector.collect().await, native);
    assert_eq!(primary_calls.load(Ordering::SeqCst), 1);
    assert_eq!(secondary_calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn fallback_configured_missing_outcome_invokes_fallback_once() {
    let expected = fallback_success(Provider::Codex);
    let (primary, primary_calls) =
        counting_collector(Provider::Codex, CollectionOutcome::CliMissing);
    let (secondary, secondary_calls) = counting_collector(Provider::Codex, expected.clone());
    let collector = FallbackCollector::new(
        Box::new(primary),
        Box::new(secondary),
        FallbackTrigger::CliMissing,
    )
    .expect("matching providers should compose");

    assert_eq!(collector.provider(), Provider::Codex);
    assert_eq!(collector.collect().await, expected);
    assert_eq!(primary_calls.load(Ordering::SeqCst), 1);
    assert_eq!(secondary_calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn fallback_failure_classes_and_other_missing_outcome_do_not_invoke_fallback() {
    let outcomes = [
        CollectionOutcome::Failed {
            class: FailureClass::Network,
        },
        CollectionOutcome::Failed {
            class: FailureClass::Provider,
        },
        CollectionOutcome::Failed {
            class: FailureClass::Parse,
        },
        CollectionOutcome::Failed {
            class: FailureClass::Internal,
        },
        CollectionOutcome::CliMissing,
    ];

    for outcome in outcomes {
        let (primary, primary_calls) = counting_collector(Provider::Claude, outcome.clone());
        let (secondary, secondary_calls) =
            counting_collector(Provider::Claude, fallback_success(Provider::Claude));
        let collector = FallbackCollector::new(
            Box::new(primary),
            Box::new(secondary),
            FallbackTrigger::CredentialsMissing,
        )
        .expect("matching providers should compose");

        assert_eq!(collector.collect().await, outcome);
        assert_eq!(primary_calls.load(Ordering::SeqCst), 1);
        assert_eq!(secondary_calls.load(Ordering::SeqCst), 0);
    }
}

#[test]
fn fallback_rejects_collectors_for_different_providers() {
    let (primary, _) = counting_collector(Provider::Claude, CollectionOutcome::CredentialsMissing);
    let (fallback, fallback_calls) =
        counting_collector(Provider::Codex, fallback_success(Provider::Codex));

    let result = FallbackCollector::new(
        Box::new(primary),
        Box::new(fallback),
        FallbackTrigger::CredentialsMissing,
    );

    assert!(result.is_err());
    assert_eq!(fallback_calls.load(Ordering::SeqCst), 0);
}

#[test]
fn documented_claude_locations_prefer_config_dir_then_default_home() {
    let locations =
        CredentialLocations::documented(Some("config-root".into()), Some("home-root".into()));
    assert_eq!(
        locations.claude,
        vec![
            std::path::PathBuf::from("config-root/.credentials.json"),
            std::path::PathBuf::from("home-root/.claude/.credentials.json")
        ]
    );
}

struct TempDir(std::path::PathBuf);

impl TempDir {
    fn path(&self) -> &std::path::Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn tempdir() -> TempDir {
    for _ in 0..100 {
        let sequence = TEMP_DIR_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "cachebite-test-{}-{}-{sequence}",
            std::process::id(),
            time::OffsetDateTime::now_utc().unix_timestamp_nanos()
        ));
        match fs::create_dir(&path) {
            Ok(()) => return TempDir(fs::canonicalize(path).unwrap()),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => panic!("failed to create temporary test directory: {error}"),
        }
    }
    panic!("failed to allocate a unique temporary test directory");
}

#[test]
fn codex_collector_rejects_bare_and_relative_executable_paths() {
    assert!(matches!(
        CodexCollector::new(PathBuf::from("codex")),
        Err(CollectorError::CliMissing)
    ));
    assert!(matches!(
        CodexCollector::new(PathBuf::from("bin/codex")),
        Err(CollectorError::CliMissing)
    ));
}

#[tokio::test]
async fn codex_process_launch_rejects_bare_and_relative_executable_paths() {
    let now = OffsetDateTime::UNIX_EPOCH;
    assert!(matches!(
        collect_app_server(std::path::Path::new("codex"), now).await,
        Err(CollectorError::CliMissing)
    ));
    assert!(matches!(
        collect_app_server(std::path::Path::new("bin/codex"), now).await,
        Err(CollectorError::CliMissing)
    ));
}

#[test]
fn codex_resolver_rejects_empty_path() {
    assert!(matches!(
        resolve_codex_executable(OsStr::new("")),
        Err(CollectorError::CliMissing)
    ));
}

#[cfg(unix)]
#[test]
fn codex_resolver_skips_empty_and_relative_path_segments() {
    use std::os::unix::fs::PermissionsExt;
    static CWD_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let _guard = CWD_LOCK.lock().unwrap();
    let root = tempdir();
    let valid = root.path().join("valid");
    fs::create_dir(&valid).unwrap();
    for executable in [root.path().join("codex"), valid.join("codex")] {
        fs::write(&executable, b"#!/bin/sh\n").unwrap();
        fs::set_permissions(&executable, fs::Permissions::from_mode(0o700)).unwrap();
    }
    let previous = std::env::current_dir().unwrap();
    std::env::set_current_dir(root.path()).unwrap();
    let path =
        std::env::join_paths([PathBuf::new(), PathBuf::from("relative"), valid.clone()]).unwrap();
    let resolved = resolve_codex_executable(&path);
    std::env::set_current_dir(previous).unwrap();
    assert_eq!(
        resolved.unwrap(),
        fs::canonicalize(valid.join("codex")).unwrap()
    );
}

#[test]
fn broker_uses_environment_before_file_and_never_writes() {
    let root = tempdir();
    let file = root.path().join("claude.json");
    fs::write(&file, r#"{"accessToken":"file-value"}"#).unwrap();
    let before = fs::read(&file).unwrap();
    let env = BTreeMap::from([("CLAUDE_CODE_OAUTH_TOKEN".into(), "env-value".into())]);
    let broker = CredentialBroker::new(
        env,
        CredentialLocations {
            claude: vec![file.clone()],
        },
    );
    let token = broker.claude_token().unwrap();
    assert_eq!(token.expose_secret(), "env-value");
    assert_eq!(fs::read(file).unwrap(), before);
    assert!(!format!("{token:?}").contains("env-value"));
}

#[test]
fn broker_maps_missing_and_rejects_oversize_credentials() {
    let root = tempdir();
    let missing = root.path().join("missing.json");
    let broker = CredentialBroker::new(
        BTreeMap::new(),
        CredentialLocations {
            claude: vec![missing],
        },
    );
    assert!(matches!(
        broker.claude_token(),
        Err(CollectorError::CredentialsMissing)
    ));
    let huge = root.path().join("huge.json");
    fs::write(&huge, vec![b'x'; 70_000]).unwrap();
    let broker = CredentialBroker::new(BTreeMap::new(), CredentialLocations { claude: vec![huge] });
    assert!(matches!(
        broker.claude_token(),
        Err(CollectorError::CredentialFileInvalid)
    ));
}

#[test]
fn broker_uses_first_available_read_only_location() {
    let root = tempdir();
    let first = root.path().join("active.json");
    let second = root.path().join("default.json");
    fs::write(&first, r#"{"claudeAiOauth":{"accessToken":"first"}}"#).unwrap();
    fs::write(&second, r#"{"accessToken":"second"}"#).unwrap();
    let broker = CredentialBroker::new(
        BTreeMap::new(),
        CredentialLocations {
            claude: vec![first, second],
        },
    );
    assert_eq!(broker.claude_token().unwrap().expose_secret(), "first");
}

#[test]
fn broker_skips_corrupt_candidate_but_reports_all_corrupt() {
    let root = tempdir();
    let corrupt = root.path().join("corrupt.json");
    let valid = root.path().join("valid.json");
    fs::write(&corrupt, b"not-json").unwrap();
    fs::write(&valid, r#"{"accessToken":"valid"}"#).unwrap();
    let broker = CredentialBroker::new(
        BTreeMap::new(),
        CredentialLocations {
            claude: vec![corrupt.clone(), valid],
        },
    );
    assert_eq!(broker.claude_token().unwrap().expose_secret(), "valid");
    let broker = CredentialBroker::new(
        BTreeMap::new(),
        CredentialLocations {
            claude: vec![corrupt],
        },
    );
    assert!(matches!(
        broker.claude_token(),
        Err(CollectorError::CredentialFileInvalid)
    ));
}

#[test]
fn errors_map_to_typed_outcomes_without_provider_cross_talk() {
    use crate::domain::{CollectionOutcome, FailureClass};
    assert_eq!(
        CollectorError::CredentialsMissing.into_outcome(),
        CollectionOutcome::CredentialsMissing
    );
    assert_eq!(
        CollectorError::CliMissing.into_outcome(),
        CollectionOutcome::CliMissing
    );
    assert_eq!(
        CollectorError::Timeout.into_outcome(),
        CollectionOutcome::Failed {
            class: FailureClass::Network
        }
    );
    let now = OffsetDateTime::UNIX_EPOCH;
    assert!(parse_usage(br#"{"five_hour":{"percent":1}}"#, now).is_ok());
    assert!(parse_rate_limits(br#"{"primary":{}}"#, now).is_err());
}

#[test]
fn claude_request_is_fixed_https_allowlisted_and_secret_safe() {
    let spec = ClaudeRequestSpec::new(SecretString::from("marker-secret".to_owned())).unwrap();
    assert_eq!(spec.url().scheme(), "https");
    assert_eq!(spec.url().host_str(), Some("api.anthropic.com"));
    assert_eq!(
        spec.header_names(),
        ["anthropic-beta", "authorization", "user-agent"]
    );
    assert!(!format!("{spec:?}").contains("marker-secret"));
    let request = spec.build_for_test().unwrap();
    assert_eq!(
        request.url().as_str(),
        "https://api.anthropic.com/api/oauth/usage"
    );
    assert_eq!(request.headers()["anthropic-beta"], "oauth-2025-04-20");
    assert_eq!(request.headers()["authorization"], "Bearer marker-secret");
    assert_eq!(
        request.headers()["user-agent"],
        "claude-code/2.0.0 (CacheBite)"
    );
}

#[test]
fn claude_parser_accepts_known_variants_and_rejects_bad_schema() {
    let now = OffsetDateTime::UNIX_EPOCH;
    let body = br#"{"five_hour":{"utilization":12.5,"resets_at":"2025-01-01T00:00:00Z"},"seven_day":{"percent":88,"reset_at":"2025-01-07T00:00:00Z"},"extra":true}"#;
    let snapshot = parse_usage(body, now).unwrap();
    let serialized = serde_json::to_string(&snapshot).unwrap();
    assert!(!serialized.contains("marker-secret"));
    assert_eq!(snapshot.provider, Provider::Claude);
    assert_eq!(snapshot.session.unwrap().used_percent, 12.5);
    assert_eq!(snapshot.weekly.unwrap().used_percent, 88.0);
    for bad in [
        br#"{}"#.as_slice(),
        br#"{"five_hour":{"utilization":"x"}}"#,
        br#"{"five_hour":{"utilization":null}}"#,
    ] {
        assert!(matches!(parse_usage(bad, now), Err(CollectorError::Parse)));
    }
    assert!(matches!(
        parse_usage(&vec![b'x'; 1_048_577], now),
        Err(CollectorError::ResponseTooLarge)
    ));
}

#[test]
fn codex_parser_accepts_nested_and_flat_variants() {
    let now = OffsetDateTime::UNIX_EPOCH;
    let nested = br#"{"primary":{"usedPercent":5,"windowDurationMins":300,"resetsAt":1735689600},"secondary":{"utilization":44,"window_minutes":10080}}"#;
    let snapshot = parse_rate_limits(nested, now).unwrap();
    assert_eq!(snapshot.session.unwrap().used_percent, 5.0);
    assert_eq!(snapshot.weekly.unwrap().used_percent, 44.0);
    let wrapped = br#"{"rateLimits":{"primary":{"used_percent":7}}}"#;
    assert_eq!(
        parse_rate_limits(wrapped, now)
            .unwrap()
            .session
            .unwrap()
            .used_percent,
        7.0
    );
    assert!(matches!(
        parse_rate_limits(br#"{"primary":{}}"#, now),
        Err(CollectorError::Parse)
    ));
    // codex-cli 0.144.5 reports the weekly window in `primary` with no
    // `secondary`; it must land in the weekly slot, leaving session empty.
    let weekly_in_primary =
        br#"{"primary":{"usedPercent":41,"windowDurationMins":10080,"resetsAt":1784876948},"secondary":null}"#;
    let snapshot = parse_rate_limits(weekly_in_primary, now).unwrap();
    assert!(snapshot.session.is_none());
    assert_eq!(snapshot.weekly.unwrap().used_percent, 41.0);
}

#[tokio::test]
async fn rpc_orders_handshake_and_read_and_rejects_wrong_ids() {
    let replies = concat!(
        "{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{}}\n",
        "{\"jsonrpc\":\"2.0\",\"id\":2,\"result\":{\"primary\":{\"usedPercent\":1}}}\n"
    );
    let mut output = Vec::new();
    let result = RpcSession::new(Duration::from_secs(1), 4096)
        .exchange(replies.as_bytes(), &mut output)
        .await
        .unwrap();
    let sent = String::from_utf8(output).unwrap();
    assert!(sent.find("initialize").unwrap() < sent.find("account/rateLimits/read").unwrap());
    assert!(sent.find("initialized").unwrap() < sent.find("account/rateLimits/read").unwrap());
    // Codex >=0.144 rejects `initialize` with -32600 unless clientInfo carries
    // both name and version, so the handshake must include them.
    let initialize_line = sent
        .lines()
        .find(|line| line.contains("initialize"))
        .unwrap();
    assert!(initialize_line.contains("clientInfo"));
    assert!(initialize_line.contains(env!("CARGO_PKG_NAME")));
    assert!(initialize_line.contains(env!("CARGO_PKG_VERSION")));
    assert!(result.primary.is_some());
    let wrong = b"{\"jsonrpc\":\"2.0\",\"id\":9,\"result\":{}}\n";
    assert!(matches!(
        RpcSession::new(Duration::from_secs(1), 4096)
            .exchange(wrong.as_slice(), &mut Vec::new())
            .await,
        Err(CollectorError::Protocol)
    ));
}

#[tokio::test]
async fn rpc_accepts_codex_responses_that_omit_jsonrpc_version() {
    // Codex CLI 0.144.5 app-server sends valid id/result envelopes without
    // echoing the optional JSON-RPC version field.
    let replies = concat!(
        "{\"id\":1,\"result\":{\"userAgent\":\"cachebite/0.144.5\"}}\n",
        "{\"id\":2,\"result\":{\"primary\":{\"usedPercent\":6}}}\n"
    );
    let result = RpcSession::new(Duration::from_secs(1), 4096)
        .exchange(replies.as_bytes(), &mut Vec::new())
        .await
        .unwrap();

    assert_eq!(result.primary.unwrap().used_percent(), Some(6.0));
}

#[tokio::test]
async fn rpc_rejects_an_explicitly_incompatible_jsonrpc_version() {
    let replies = b"{\"jsonrpc\":\"1.0\",\"id\":1,\"result\":{}}\n";
    assert!(matches!(
        RpcSession::new(Duration::from_secs(1), 4096)
            .exchange(replies.as_slice(), &mut Vec::new())
            .await,
        Err(CollectorError::Protocol)
    ));
}

#[tokio::test]
async fn rpc_tolerates_crlf_line_endings_and_blank_lines() {
    let replies = concat!(
        "{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{}}\r\n",
        "\r\n",
        "{\"jsonrpc\":\"2.0\",\"id\":2,\"result\":{\"primary\":{\"usedPercent\":8}}}\r\n"
    );
    let result = RpcSession::new(Duration::from_secs(1), 4096)
        .exchange(replies.as_bytes(), &mut Vec::new())
        .await
        .unwrap();
    assert_eq!(result.primary.unwrap().used_percent(), Some(8.0));
}

#[tokio::test]
async fn rpc_ignores_notifications_and_unmatched_ids_within_bounds() {
    let replies = concat!(
        "{\"jsonrpc\":\"2.0\",\"method\":\"server/ready\",\"params\":{}}\n",
        "{\"jsonrpc\":\"2.0\",\"id\":99,\"result\":{}}\n",
        "{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{}}\n",
        "{\"jsonrpc\":\"2.0\",\"method\":\"account/updated\",\"params\":{}}\n",
        "{\"jsonrpc\":\"2.0\",\"id\":2,\"result\":{\"primary\":{\"usedPercent\":2}}}\n"
    );
    let result = RpcSession::new(Duration::from_secs(1), 4096)
        .exchange(replies.as_bytes(), &mut Vec::new())
        .await
        .unwrap();
    assert_eq!(result.primary.unwrap().used_percent(), Some(2.0));
}

#[tokio::test]
async fn rpc_maps_not_signed_in_error_to_missing_credentials() {
    let replies = concat!(
        "{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{}}\n",
        "{\"jsonrpc\":\"2.0\",\"id\":2,\"error\":{\"code\":-32000,\"message\":\"Not signed in\"}}\n"
    );
    assert!(matches!(
        RpcSession::new(Duration::from_secs(1), 4096)
            .exchange(replies.as_bytes(), &mut Vec::new())
            .await,
        Err(CollectorError::CredentialsMissing)
    ));
}

#[tokio::test]
async fn rpc_enforces_timeout_malformed_and_size_limits() {
    let (pending, _open_peer) = tokio::io::duplex(8);
    assert!(matches!(
        RpcSession::new(Duration::from_millis(10), 32)
            .exchange(pending, &mut Vec::new())
            .await,
        Err(CollectorError::Timeout)
    ));
    for bytes in [b"not-json\n".as_slice(), b"{\"id\":1}\n"] {
        assert!(RpcSession::new(Duration::from_secs(1), 32)
            .exchange(bytes, &mut Vec::new())
            .await
            .is_err());
    }
    let huge = format!(
        "{{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{{\"x\":\"{}\"}}}}\n",
        "x".repeat(64)
    );
    assert!(matches!(
        RpcSession::new(Duration::from_secs(1), 32)
            .exchange(huge.as_bytes(), &mut Vec::new())
            .await,
        Err(CollectorError::ResponseTooLarge)
    ));
}

#[cfg(unix)]
#[tokio::test]
async fn app_server_child_is_reaped_after_success() {
    use super::codex::collect_app_server;
    use std::os::unix::fs::PermissionsExt;
    let root = tempdir();
    let executable = root.path().join("fake-codex");
    let pid_file = root.path().join("pid");
    let script = format!(
        "#!/bin/sh\necho $$ > '{}'\nread first\nprintf '%s\\n' '{{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{{}}}}'\nread second\nprintf '%s\\n' '{{\"jsonrpc\":\"2.0\",\"id\":2,\"result\":{{\"primary\":{{\"usedPercent\":3}}}}}}'\nsleep 30\n",
        pid_file.display()
    );
    fs::write(&executable, script).unwrap();
    fs::set_permissions(&executable, fs::Permissions::from_mode(0o700)).unwrap();
    let executable = fs::canonicalize(executable).unwrap();
    assert!(collect_app_server(&executable, OffsetDateTime::UNIX_EPOCH)
        .await
        .is_ok());
    let pid: i32 = fs::read_to_string(pid_file)
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    assert_eq!(unsafe { libc::kill(pid, 0) }, -1);
}

#[cfg(unix)]
#[tokio::test]
async fn cancelling_collection_kills_and_reaps_process_group() {
    use super::codex::collect_app_server;
    use std::os::unix::fs::PermissionsExt;
    let root = tempdir();
    let executable = root.path().join("hanging-codex");
    let pid_file = root.path().join("pid");
    let child_pid_file = root.path().join("child-pid");
    let script = format!(
        "#!/bin/sh\necho $$ > '{}'\nread first\nsleep 30 &\necho $! > '{}'\nwait\n",
        pid_file.display(),
        child_pid_file.display()
    );
    fs::write(&executable, script).unwrap();
    fs::set_permissions(&executable, fs::Permissions::from_mode(0o700)).unwrap();
    let executable = fs::canonicalize(executable).unwrap();
    let task =
        tokio::spawn(
            async move { collect_app_server(&executable, OffsetDateTime::UNIX_EPOCH).await },
        );
    for _ in 0..100 {
        if pid_file.exists() && child_pid_file.exists() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
    let pid: i32 = fs::read_to_string(&pid_file)
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    let child_pid: i32 = fs::read_to_string(&child_pid_file)
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    task.abort();
    let _ = task.await;
    for _ in 0..100 {
        if unsafe { libc::kill(pid, 0) } == -1 && unsafe { libc::kill(child_pid, 0) } == -1 {
            return;
        }
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
    panic!("cancelled app-server process was not reaped");
}
