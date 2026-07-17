use super::{Collector, CollectorError, MAX_RESPONSE_BYTES};
use crate::domain::{CollectionOutcome, Provider, ProviderUsageSnapshot, Source, UsageWindow};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
    process::Stdio,
    time::Duration,
};
use time::OffsetDateTime;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};

pub struct CodexCollector {
    executable: std::path::PathBuf,
}

impl CodexCollector {
    pub fn new(executable: PathBuf) -> Result<Self, CollectorError> {
        validate_executable(&executable)?;
        Ok(Self { executable })
    }
}

pub fn resolve_codex_executable(path: &OsStr) -> Result<PathBuf, CollectorError> {
    for directory in std::env::split_paths(path) {
        // Empty PATH entries mean the current directory on Unix, and relative
        // entries defer selection to the launcher's working directory. Neither
        // is a trusted startup search root.
        if !directory.is_absolute() {
            continue;
        }
        for name in codex_executable_names() {
            let candidate = directory.join(name);
            let Ok(canonical) = std::fs::canonicalize(candidate) else {
                continue;
            };
            if validate_executable(&canonical).is_ok() {
                return Ok(canonical);
            }
        }
    }
    Err(CollectorError::CliMissing)
}

#[cfg(windows)]
fn codex_executable_names() -> Vec<String> {
    let extensions = std::env::var_os("PATHEXT")
        .and_then(|value| value.into_string().ok())
        .unwrap_or_else(|| ".COM;.EXE;.BAT;.CMD".to_owned());
    extensions
        .split(';')
        .filter(|extension| !extension.is_empty())
        .map(|extension| format!("codex{}", extension.to_ascii_lowercase()))
        .collect()
}

#[cfg(not(windows))]
fn codex_executable_names() -> Vec<&'static str> {
    vec!["codex"]
}

fn validate_executable(executable: &Path) -> Result<(), CollectorError> {
    if !executable.is_absolute() {
        return Err(CollectorError::CliMissing);
    }
    let canonical = std::fs::canonicalize(executable).map_err(|_| CollectorError::CliMissing)?;
    if canonical != executable {
        return Err(CollectorError::CliMissing);
    }
    // This deliberately validates a pathname, not an open executable handle.
    // Replacement between validation and spawn remains possible; signature or
    // handle pinning is outside the approved remediation scope.
    let metadata = executable
        .metadata()
        .map_err(|_| CollectorError::CliMissing)?;
    if !metadata.is_file() || !platform_is_executable(executable, &metadata) {
        return Err(CollectorError::CliMissing);
    }
    Ok(())
}

#[cfg(unix)]
fn platform_is_executable(_path: &Path, metadata: &std::fs::Metadata) -> bool {
    use std::os::unix::fs::PermissionsExt;
    metadata.permissions().mode() & 0o111 != 0
}

#[cfg(windows)]
fn platform_is_executable(path: &Path, _metadata: &std::fs::Metadata) -> bool {
    path.extension()
        .and_then(OsStr::to_str)
        .is_some_and(|extension| {
            matches!(
                extension.to_ascii_lowercase().as_str(),
                "com" | "exe" | "bat" | "cmd"
            )
        })
}

#[cfg(not(any(unix, windows)))]
fn platform_is_executable(_path: &Path, _metadata: &std::fs::Metadata) -> bool {
    true
}

impl Collector for CodexCollector {
    fn provider(&self) -> Provider {
        Provider::Codex
    }

    fn collect(
        &self,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = CollectionOutcome> + Send + '_>> {
        Box::pin(async move {
            match collect_app_server(&self.executable, OffsetDateTime::now_utc()).await {
                Ok(snapshot) => CollectionOutcome::Success { snapshot },
                Err(error) => error.into_outcome(),
            }
        })
    }
}

pub struct RpcSession {
    timeout: Duration,
    max_response_bytes: usize,
}

impl RpcSession {
    pub fn new(timeout: Duration, max_response_bytes: usize) -> Self {
        Self {
            timeout,
            max_response_bytes,
        }
    }

    pub async fn exchange<R, W>(
        &self,
        mut reader: R,
        writer: &mut W,
    ) -> Result<RateLimitResult, CollectorError>
    where
        R: AsyncRead + Unpin,
        W: AsyncWrite + Unpin,
    {
        tokio::time::timeout(self.timeout, async {
            write_request(writer, 1, "initialize", initialize_params()).await?;
            let mut remaining = self.max_response_bytes;
            let initialized = read_matching_response(&mut reader, 1, &mut remaining).await?;
            validate_response(&initialized)?;
            write_notification(writer, "initialized").await?;
            write_request(
                writer,
                2,
                "account/rateLimits/read",
                Value::Object(Default::default()),
            )
            .await?;
            let response = read_matching_response(&mut reader, 2, &mut remaining).await?;
            validate_response(&response)?;
            let envelope: RateLimitEnvelope =
                serde_json::from_value(response.result.ok_or(CollectorError::Protocol)?)
                    .map_err(|_| CollectorError::Parse)?;
            Ok(envelope.into_result())
        })
        .await
        .map_err(|_| CollectorError::Timeout)?
    }
}

pub async fn collect_app_server(
    executable: &Path,
    now: OffsetDateTime,
) -> Result<ProviderUsageSnapshot, CollectorError> {
    validate_executable(executable)?;
    let mut command = Command::new(executable);
    command
        .args(["-s", "read-only", "-a", "untrusted"])
        .arg("app-server");
    collect_app_server_child(command, now).await
}

pub async fn collect_app_server_child(
    command: Command,
    now: OffsetDateTime,
) -> Result<ProviderUsageSnapshot, CollectorError> {
    collect_app_server_child_with_options(command, now, Duration::from_secs(10), false).await
}

pub(crate) async fn collect_app_server_child_with_options(
    command: Command,
    now: OffsetDateTime,
    timeout: Duration,
    exit_127_is_cli_missing: bool,
) -> Result<ProviderUsageSnapshot, CollectorError> {
    let mut child = spawn_managed_app_server(command)?;
    let mut stdin = child.stdin().ok_or(CollectorError::Internal)?;
    let stdout = child.stdout().ok_or(CollectorError::Internal)?;
    let result = RpcSession::new(timeout, MAX_RESPONSE_BYTES)
        .exchange(stdout, &mut stdin)
        .await;
    drop(stdin);
    let missing_cli = exit_127_is_cli_missing
        && result.is_err()
        && child.exit_code_with_grace().await == Some(127);
    child.terminate().await;
    if missing_cli {
        return Err(CollectorError::CliMissing);
    }
    let limits = result?;
    normalize(limits, now)
}

pub(crate) async fn collect_app_server_child_with_pgid<F, Fut>(
    command: Command,
    now: OffsetDateTime,
    timeout: Duration,
    cleanup: F,
) -> Result<ProviderUsageSnapshot, CollectorError>
where
    F: FnOnce(u32) -> Fut,
    Fut: std::future::Future<Output = Result<(), CollectorError>>,
{
    let mut child = spawn_managed_app_server(command)?;
    let mut stdin = child.stdin().ok_or(CollectorError::Internal)?;
    let stdout = child.stdout().ok_or(CollectorError::Internal)?;
    let mut reader = BufReader::new(stdout);
    let pgid = tokio::time::timeout(timeout, read_pgid_handshake(&mut reader))
        .await
        .map_err(|_| CollectorError::Timeout)
        .and_then(|result| result)?;
    let result = RpcSession::new(timeout, MAX_RESPONSE_BYTES)
        .exchange(reader, &mut stdin)
        .await;
    drop(stdin);
    let cleanup_result = cleanup(pgid).await;
    child.terminate().await;
    cleanup_result?;
    let limits = result?;
    normalize(limits, now)
}

fn configure_app_server_command(command: &mut Command) {
    command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true);
    #[cfg(unix)]
    command.process_group(0);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.as_std_mut().creation_flags(CREATE_NO_WINDOW);
    }
}

fn spawn_managed_app_server(mut command: Command) -> Result<ManagedChild, CollectorError> {
    configure_app_server_command(&mut command);
    command
        .spawn()
        .map(ManagedChild::new)
        .map_err(|error| classify_spawn_error(error.kind()))
}

async fn read_pgid_handshake<R: AsyncRead + Unpin>(
    reader: &mut BufReader<R>,
) -> Result<u32, CollectorError> {
    const PREFIX: &[u8] = b"CACHEBITE_PGID:";
    // A login shell (`bash -lc`) may print profile output (MOTD, version
    // managers, etc.) to stdout before the marker line. Skip leading noise lines
    // until the marker appears, within a bounded byte budget. The marker is
    // emitted before `exec codex`, so no JSON-RPC output is discarded here.
    const MAX_SCAN_BYTES: usize = 64 * 1024;
    const MAX_LINE_BYTES: usize = 128;
    let mut scanned = 0usize;
    let mut line = Vec::new();
    loop {
        let byte = match reader.read_u8().await {
            Ok(byte) => byte,
            Err(error) => {
                #[cfg(debug_assertions)]
                eprintln!(
                    "[CacheBite:codex] handshake stream ended after {scanned} bytes: {error}"
                );
                return Err(CollectorError::Protocol);
            }
        };
        scanned += 1;
        if scanned > MAX_SCAN_BYTES {
            return Err(CollectorError::Protocol);
        }
        if byte == b'\n' {
            #[cfg(debug_assertions)]
            eprintln!(
                "[CacheBite:codex] handshake line ({} bytes): {}",
                line.len(),
                escape_bytes(&line)
            );
            // wsl.exe pipes stdout to Windows with CRLF line endings; drop a
            // trailing carriage return so the numeric marker parses.
            if line.last() == Some(&b'\r') {
                line.pop();
            }
            if line.starts_with(PREFIX) {
                return parse_pgid_handshake(&line);
            }
            line.clear();
            continue;
        }
        // The marker is far shorter than MAX_LINE_BYTES; longer content cannot be
        // the marker, so stop growing the buffer while still consuming the line.
        if line.len() < MAX_LINE_BYTES {
            line.push(byte);
        }
    }
}

/// Escapes non-printable bytes for diagnostic logging so the exact wire stream
/// (CRLF, control chars, encoding surprises) is visible in debug builds.
#[cfg(debug_assertions)]
pub(crate) fn escape_bytes(bytes: &[u8]) -> String {
    bytes
        .iter()
        .take(200)
        .map(|byte| match byte {
            0x20..=0x7e => (*byte as char).to_string(),
            _ => format!("\\x{byte:02x}"),
        })
        .collect()
}

pub(crate) fn parse_pgid_handshake(line: &[u8]) -> Result<u32, CollectorError> {
    const PREFIX: &[u8] = b"CACHEBITE_PGID:";
    const MAX_HANDSHAKE_BYTES: usize = 64;
    // Defensively tolerate a trailing CR from CRLF line endings.
    let line = line.strip_suffix(b"\r").unwrap_or(line);
    if line.len() > MAX_HANDSHAKE_BYTES {
        return Err(CollectorError::Protocol);
    }
    let digits = line.strip_prefix(PREFIX).ok_or(CollectorError::Protocol)?;
    if digits.is_empty() || !digits.iter().all(u8::is_ascii_digit) {
        return Err(CollectorError::Protocol);
    }
    let digits = std::str::from_utf8(digits).map_err(|_| CollectorError::Protocol)?;
    let pgid = digits
        .parse::<u32>()
        .map_err(|_| CollectorError::Protocol)?;
    if pgid == 0 {
        return Err(CollectorError::Protocol);
    }
    Ok(pgid)
}

pub(crate) fn classify_spawn_error(kind: std::io::ErrorKind) -> CollectorError {
    if kind == std::io::ErrorKind::NotFound {
        CollectorError::CliMissing
    } else {
        CollectorError::Internal
    }
}

struct ManagedChild {
    child: Option<Child>,
}

impl ManagedChild {
    fn new(child: Child) -> Self {
        Self { child: Some(child) }
    }

    fn stdin(&mut self) -> Option<tokio::process::ChildStdin> {
        self.child.as_mut()?.stdin.take()
    }

    fn stdout(&mut self) -> Option<tokio::process::ChildStdout> {
        self.child.as_mut()?.stdout.take()
    }

    async fn terminate(&mut self) {
        if let Some(mut child) = self.child.take() {
            terminate_process_group(&child);
            let _ = child.kill().await;
            let _ = child.wait().await;
        }
    }

    async fn exit_code_with_grace(&mut self) -> Option<i32> {
        let child = self.child.as_mut()?;
        if let Some(status) = child.try_wait().ok().flatten() {
            return status.code();
        }
        tokio::time::timeout(Duration::from_millis(100), child.wait())
            .await
            .ok()?
            .ok()?
            .code()
    }
}

impl Drop for ManagedChild {
    fn drop(&mut self) {
        let Some(mut child) = self.child.take() else {
            return;
        };
        terminate_process_group(&child);
        let _ = child.start_kill();
        if let Ok(runtime) = tokio::runtime::Handle::try_current() {
            runtime.spawn(async move {
                let _ = child.wait().await;
            });
        }
    }
}

#[cfg(unix)]
fn terminate_process_group(child: &Child) {
    if let Some(pid) = child.id() {
        unsafe {
            libc::kill(-(pid as i32), libc::SIGKILL);
        }
    }
}

#[cfg(not(unix))]
fn terminate_process_group(_child: &Child) {}

pub fn parse_rate_limits(
    body: &[u8],
    now: OffsetDateTime,
) -> Result<ProviderUsageSnapshot, CollectorError> {
    if body.len() > MAX_RESPONSE_BYTES {
        return Err(CollectorError::ResponseTooLarge);
    }
    let limits: RateLimitEnvelope =
        serde_json::from_slice(body).map_err(|_| CollectorError::Parse)?;
    normalize(limits.into_result(), now)
}

/// Windows at or above this duration are treated as the weekly limit. Codex
/// reports a 5-hour session window near 300 minutes and a weekly window near
/// 10080 minutes, so any threshold between the two separates them.
const WEEKLY_WINDOW_THRESHOLD_MINS: u32 = 1440;

fn normalize(
    limits: RateLimitResult,
    now: OffsetDateTime,
) -> Result<ProviderUsageSnapshot, CollectorError> {
    // Codex does not guarantee that `primary` is the session window and
    // `secondary` the weekly one. As of codex-cli 0.144.5 an account can report
    // the weekly window in `primary` with `secondary` absent, so classify each
    // window by its declared duration instead of trusting slot order.
    let mut session = None;
    let mut weekly = None;
    for wire in [limits.primary, limits.secondary].into_iter().flatten() {
        let is_weekly = wire
            .window_minutes
            .is_some_and(|minutes| minutes >= WEEKLY_WINDOW_THRESHOLD_MINS);
        if is_weekly {
            let normalized = wire.normalize(10_080)?;
            if weekly.is_none() {
                weekly = Some(normalized);
            }
        } else {
            let normalized = wire.normalize(300)?;
            if session.is_none() {
                session = Some(normalized);
            }
        }
    }
    if session.is_none() && weekly.is_none() {
        return Err(CollectorError::Parse);
    }
    Ok(ProviderUsageSnapshot {
        provider: Provider::Codex,
        plan_type: limits.plan_type,
        session,
        weekly,
        captured_at: now,
        source: Source::CliRpc,
        is_cached: false,
        revision: 0,
    })
}

/// Parameters for the app-server `initialize` handshake. Codex rejects the
/// request with JSON-RPC -32600 unless `clientInfo.name` and `clientInfo.version`
/// are both present.
fn initialize_params() -> Value {
    serde_json::json!({
        "clientInfo": {
            "name": env!("CARGO_PKG_NAME"),
            "version": env!("CARGO_PKG_VERSION"),
        }
    })
}

async fn write_request<W: AsyncWrite + Unpin>(
    writer: &mut W,
    id: u64,
    method: &str,
    params: Value,
) -> Result<(), CollectorError> {
    let request = serde_json::json!({"jsonrpc":"2.0", "id":id, "method":method, "params":params});
    writer
        .write_all(request.to_string().as_bytes())
        .await
        .map_err(|_| CollectorError::Protocol)?;
    writer
        .write_all(b"\n")
        .await
        .map_err(|_| CollectorError::Protocol)?;
    writer.flush().await.map_err(|_| CollectorError::Protocol)
}

async fn write_notification<W: AsyncWrite + Unpin>(
    writer: &mut W,
    method: &str,
) -> Result<(), CollectorError> {
    let notification = serde_json::json!({"jsonrpc":"2.0", "method":method, "params":{}});
    writer
        .write_all(notification.to_string().as_bytes())
        .await
        .map_err(|_| CollectorError::Protocol)?;
    writer
        .write_all(b"\n")
        .await
        .map_err(|_| CollectorError::Protocol)?;
    writer.flush().await.map_err(|_| CollectorError::Protocol)
}

async fn read_matching_response<R: AsyncRead + Unpin>(
    reader: &mut R,
    wanted_id: u64,
    remaining: &mut usize,
) -> Result<RpcResponse, CollectorError> {
    for _ in 0..64 {
        let response = read_response(reader, remaining).await?;
        if response.id == Some(wanted_id) {
            return Ok(response);
        }
        if response.id.is_none() && response.method.is_none() {
            return Err(CollectorError::Protocol);
        }
    }
    Err(CollectorError::Protocol)
}

async fn read_response<R: AsyncRead + Unpin>(
    reader: &mut R,
    remaining: &mut usize,
) -> Result<RpcResponse, CollectorError> {
    loop {
        let mut bytes = Vec::with_capacity((*remaining).min(4096));
        loop {
            let byte = reader
                .read_u8()
                .await
                .map_err(|_| CollectorError::Protocol)?;
            if byte == b'\n' {
                break;
            }
            if *remaining == 0 {
                return Err(CollectorError::ResponseTooLarge);
            }
            *remaining -= 1;
            bytes.push(byte);
        }
        // Tolerate CRLF line endings from wsl.exe and skip blank separator lines.
        if bytes.last() == Some(&b'\r') {
            bytes.pop();
        }
        if bytes.is_empty() {
            continue;
        }
        #[cfg(debug_assertions)]
        eprintln!(
            "[CacheBite:codex] rpc line ({} bytes): {}",
            bytes.len(),
            escape_bytes(&bytes)
        );
        return serde_json::from_slice(&bytes).map_err(|_| CollectorError::Protocol);
    }
}

fn validate_response(response: &RpcResponse) -> Result<(), CollectorError> {
    // Codex CLI 0.144.5 app-server omits this otherwise standard field in
    // responses. Keep rejecting an explicitly incompatible version.
    if response
        .jsonrpc
        .as_deref()
        .is_some_and(|version| version != "2.0")
    {
        return Err(CollectorError::Protocol);
    }
    if let Some(error) = &response.error {
        let message = error.message.to_ascii_lowercase();
        if error.code == -32000
            && (message.contains("not signed in")
                || message.contains("authentication")
                || message.contains("log in"))
        {
            return Err(CollectorError::CredentialsMissing);
        }
        return Err(CollectorError::Provider);
    }
    Ok(())
}

#[derive(Deserialize)]
struct RpcResponse {
    jsonrpc: Option<String>,
    id: Option<u64>,
    method: Option<String>,
    result: Option<Value>,
    error: Option<RpcError>,
}

#[derive(Deserialize)]
struct RpcError {
    code: i64,
    message: String,
}

#[derive(Debug, Deserialize)]
pub struct RateLimitResult {
    pub primary: Option<RateWindow>,
    pub secondary: Option<RateWindow>,
    #[serde(rename = "planType")]
    plan_type: Option<String>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum RateLimitEnvelope {
    Nested {
        #[serde(rename = "rateLimits", alias = "rate_limits")]
        rate_limits: RateLimitResult,
    },
    Direct(RateLimitResult),
}

impl RateLimitEnvelope {
    fn into_result(self) -> RateLimitResult {
        match self {
            Self::Nested { rate_limits } => rate_limits,
            Self::Direct(result) => result,
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RateWindow {
    #[serde(rename = "usedPercent", alias = "utilization", alias = "used_percent")]
    used_percent: Option<f64>,
    #[serde(rename = "windowDurationMins", alias = "window_minutes")]
    window_minutes: Option<u32>,
    #[serde(rename = "resetsAt", alias = "reset_at")]
    resets_at: Option<i64>,
}

impl RateWindow {
    #[cfg(test)]
    pub(crate) fn used_percent(&self) -> Option<f64> {
        self.used_percent
    }

    fn normalize(self, fallback_minutes: u32) -> Result<UsageWindow, CollectorError> {
        let percent = self.used_percent.ok_or(CollectorError::Parse)?;
        let reset = self
            .resets_at
            .map(|timestamp| {
                OffsetDateTime::from_unix_timestamp(timestamp).map_err(|_| CollectorError::Parse)
            })
            .transpose()?;
        UsageWindow::new(
            percent,
            self.window_minutes.unwrap_or(fallback_minutes),
            reset,
        )
        .map_err(|_| CollectorError::Parse)
    }
}
