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
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
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
            write_request(writer, 1, "initialize").await?;
            let mut remaining = self.max_response_bytes;
            let initialized = read_matching_response(&mut reader, 1, &mut remaining).await?;
            validate_response(&initialized)?;
            write_notification(writer, "initialized").await?;
            write_request(writer, 2, "account/rateLimits/read").await?;
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
        .arg("app-server")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true);
    #[cfg(unix)]
    command.process_group(0);
    let child = command.spawn().map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            CollectorError::CliMissing
        } else {
            CollectorError::Internal
        }
    })?;
    let mut child = ManagedChild::new(child);
    let mut stdin = child.stdin().ok_or(CollectorError::Internal)?;
    let stdout = child.stdout().ok_or(CollectorError::Internal)?;
    let result = RpcSession::new(Duration::from_secs(10), MAX_RESPONSE_BYTES)
        .exchange(stdout, &mut stdin)
        .await;
    drop(stdin);
    child.terminate().await;
    let limits = result?;
    normalize(limits, now)
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

fn normalize(
    limits: RateLimitResult,
    now: OffsetDateTime,
) -> Result<ProviderUsageSnapshot, CollectorError> {
    let session = limits.primary.map(|wire| wire.normalize(300)).transpose()?;
    let weekly = limits
        .secondary
        .map(|wire| wire.normalize(10_080))
        .transpose()?;
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

async fn write_request<W: AsyncWrite + Unpin>(
    writer: &mut W,
    id: u64,
    method: &str,
) -> Result<(), CollectorError> {
    let request = serde_json::json!({"jsonrpc":"2.0", "id":id, "method":method, "params":{}});
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
    serde_json::from_slice(&bytes).map_err(|_| CollectorError::Protocol)
}

fn validate_response(response: &RpcResponse) -> Result<(), CollectorError> {
    if response.jsonrpc != "2.0" {
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
    jsonrpc: String,
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
