use super::{
    broker::{CredentialBroker, CredentialLocations},
    claude::{parse_usage, redirects_are_disabled, ClaudeRequestSpec},
    codex::{
        collect_app_server, parse_rate_limits, resolve_codex_executable, CodexCollector, RpcSession,
    },
    CollectorError,
};
use crate::domain::Provider;
use secrecy::{ExposeSecret, SecretString};
use std::{collections::BTreeMap, ffi::OsStr, fs, path::PathBuf, time::Duration};
use time::OffsetDateTime;

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
    let path = std::env::temp_dir().join(format!(
        "cachebite-test-{}-{}",
        std::process::id(),
        time::OffsetDateTime::now_utc().unix_timestamp_nanos()
    ));
    fs::create_dir(&path).unwrap();
    TempDir(path)
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
    assert!(redirects_are_disabled());
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
    let pending = tokio::io::duplex(8).0;
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
