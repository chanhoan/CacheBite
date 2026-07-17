use super::{
    broker::{parse_token_bytes, ClaudeTokenSource, MAX_CREDENTIAL_BYTES},
    codex::collect_app_server_child_with_pgid,
    Collector, CollectorError,
};
use crate::domain::{CollectionOutcome, Provider};
use secrecy::SecretString;
#[cfg(any(windows, test))]
use std::path::Path;
#[cfg(windows)]
use std::time::Duration;
use std::{future::Future, path::PathBuf, pin::Pin, sync::Arc};
use zeroize::Zeroize;

pub const CLAUDE_CREDENTIAL_SCRIPT: &str = "if [ -n \"${CLAUDE_CONFIG_DIR:-}\" ] && [ -f \"${CLAUDE_CONFIG_DIR}/.credentials.json\" ]; then cat -- \"${CLAUDE_CONFIG_DIR}/.credentials.json\"; elif [ -f \"${HOME}/.claude/.credentials.json\" ]; then cat -- \"${HOME}/.claude/.credentials.json\"; else exit 44; fi";
pub const CODEX_PROBE_SCRIPT: &str = "bash -lc 'command -v codex >/dev/null 2>&1'";
pub const CODEX_LAUNCH_SCRIPT: &str = "exec setsid --wait bash -lc 'printf \"CACHEBITE_PGID:%s\\n\" \"$$\"; exec codex -s read-only -a untrusted app-server'";
pub const CODEX_CLEANUP_SCRIPT: &str = "case \"$1\" in ''|*[!0-9]*) exit 1;; esac; if ! kill -0 -- \"-$1\" 2>/dev/null; then exit 0; fi; kill -KILL -- \"-$1\" 2>/dev/null || exit 1; i=0; while kill -0 -- \"-$1\" 2>/dev/null; do i=$((i+1)); [ \"$i\" -lt 20 ] || exit 1; sleep 0.05 || exit 1; done";
#[cfg(windows)]
const PROCESS_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Clone, Eq, PartialEq)]
pub struct ProcessOutput {
    pub status: i32,
    pub stdout: Vec<u8>,
}

impl Drop for ProcessOutput {
    fn drop(&mut self) {
        self.stdout.zeroize();
    }
}

pub trait WslProcess: Send + Sync {
    fn run<'a>(
        &'a self,
        args: &'a [&'a str],
    ) -> Pin<Box<dyn Future<Output = Result<ProcessOutput, CollectorError>> + Send + 'a>>;
}

#[derive(Clone)]
pub struct WslCommandFactory {
    process: Arc<dyn WslProcess>,
    executable: PathBuf,
}

impl WslCommandFactory {
    #[cfg(windows)]
    pub fn from_system_directory() -> Result<Self, CollectorError> {
        let executable = validate_system_wsl_path(&windows_system_directory()?)?;
        Ok(Self {
            process: Arc::new(SystemWslProcess {
                executable: executable.clone(),
            }),
            executable,
        })
    }

    #[cfg(not(windows))]
    pub fn from_system_directory() -> Result<Self, CollectorError> {
        Err(CollectorError::CredentialsMissing)
    }

    #[cfg(test)]
    pub(crate) fn with_process_for_test(process: Arc<dyn WslProcess>) -> Self {
        Self {
            process,
            executable: PathBuf::new(),
        }
    }

    #[cfg(all(test, unix))]
    pub(crate) fn with_executable_for_test(executable: PathBuf) -> Self {
        Self {
            process: Arc::new(SuccessfulWslProcess),
            executable,
        }
    }

    #[cfg(test)]
    pub(crate) fn with_process_and_executable_for_test(
        process: Arc<dyn WslProcess>,
        executable: PathBuf,
    ) -> Self {
        Self {
            process,
            executable,
        }
    }
}

#[cfg(all(test, unix))]
struct SuccessfulWslProcess;

#[cfg(all(test, unix))]
impl WslProcess for SuccessfulWslProcess {
    fn run<'a>(
        &'a self,
        _args: &'a [&'a str],
    ) -> Pin<Box<dyn Future<Output = Result<ProcessOutput, CollectorError>> + Send + 'a>> {
        Box::pin(std::future::ready(Ok(ProcessOutput {
            status: 0,
            stdout: Vec::new(),
        })))
    }
}

pub struct WslCodexCollector {
    factory: WslCommandFactory,
    timeout: std::time::Duration,
}

impl WslCodexCollector {
    pub fn new(factory: WslCommandFactory) -> Self {
        Self {
            factory,
            timeout: std::time::Duration::from_secs(10),
        }
    }

    #[cfg(all(test, unix))]
    pub(crate) fn with_timeout_for_test(
        factory: WslCommandFactory,
        timeout: std::time::Duration,
    ) -> Self {
        Self { factory, timeout }
    }
}

impl Collector for WslCodexCollector {
    fn provider(&self) -> Provider {
        Provider::Codex
    }

    fn collect(&self) -> Pin<Box<dyn Future<Output = CollectionOutcome> + Send + '_>> {
        let factory = self.factory.clone();
        let timeout = self.timeout;
        Box::pin(async move {
            match tokio::spawn(async move { collect_wsl_codex(factory, timeout).await }).await {
                Ok(outcome) => outcome,
                Err(_) => CollectorError::Internal.into_outcome(),
            }
        })
    }
}

async fn collect_wsl_codex(
    factory: WslCommandFactory,
    timeout: std::time::Duration,
) -> CollectionOutcome {
    let probe = factory
        .process
        .run(&["--exec", "sh", "-c", CODEX_PROBE_SCRIPT])
        .await;
    #[cfg(debug_assertions)]
    eprintln!(
        "[CacheBite:codex] wsl probe status={:?}",
        probe.as_ref().map(|output| output.status)
    );
    if !matches!(probe, Ok(ProcessOutput { status: 0, .. })) {
        #[cfg(debug_assertions)]
        eprintln!("[CacheBite:codex] wsl probe failed -> CliMissing");
        return CollectionOutcome::CliMissing;
    }
    let mut command = tokio::process::Command::new(&factory.executable);
    command.args(["--exec", "sh", "-c", CODEX_LAUNCH_SCRIPT]);
    let cleanup_factory = factory.clone();
    match collect_app_server_child_with_pgid(
        command,
        time::OffsetDateTime::now_utc(),
        timeout,
        move |pgid| async move { run_codex_cleanup(&cleanup_factory, pgid).await },
    )
    .await
    {
        Ok(snapshot) => {
            #[cfg(debug_assertions)]
            eprintln!(
                "[CacheBite:codex] wsl success session={:?} weekly={:?} plan={:?}",
                snapshot
                    .session
                    .as_ref()
                    .map(|window| (window.used_percent, window.window_minutes)),
                snapshot
                    .weekly
                    .as_ref()
                    .map(|window| (window.used_percent, window.window_minutes)),
                snapshot.plan_type,
            );
            CollectionOutcome::Success { snapshot }
        }
        Err(error) => {
            #[cfg(debug_assertions)]
            eprintln!("[CacheBite:codex] wsl rpc error={error:?}");
            error.into_outcome()
        }
    }
}

async fn run_codex_cleanup(factory: &WslCommandFactory, pgid: u32) -> Result<(), CollectorError> {
    let args = codex_cleanup_args(pgid);
    let refs = args.iter().map(String::as_str).collect::<Vec<_>>();
    let output = factory
        .process
        .run(&refs)
        .await
        .map_err(|_| CollectorError::Internal)?;
    if output.status == 0 {
        Ok(())
    } else {
        Err(CollectorError::Internal)
    }
}

pub(crate) fn codex_cleanup_args(pgid: u32) -> [String; 6] {
    [
        "--exec".into(),
        "sh".into(),
        "-c".into(),
        CODEX_CLEANUP_SCRIPT.into(),
        "cachebite-cleanup".into(),
        pgid.to_string(),
    ]
}

pub struct WslCredentialSource {
    factory: WslCommandFactory,
}

impl WslCredentialSource {
    pub fn new(factory: WslCommandFactory) -> Self {
        Self { factory }
    }

    pub async fn claude_token(&self) -> Result<SecretString, CollectorError> {
        let mut output = self
            .factory
            .process
            .run(&["--exec", "sh", "-c", CLAUDE_CREDENTIAL_SCRIPT])
            .await?;
        if output.status != 0 {
            return Err(CollectorError::CredentialsMissing);
        }
        if output.stdout.len() as u64 > MAX_CREDENTIAL_BYTES {
            return Err(CollectorError::CredentialFileInvalid);
        }
        let result = match parse_token_bytes(&output.stdout) {
            Ok(Some(token)) => Ok(token),
            Ok(None) | Err(()) => Err(CollectorError::CredentialFileInvalid),
        };
        output.stdout.zeroize();
        result
    }
}

#[cfg(any(windows, test))]
pub(crate) fn validate_system_wsl_path(system_directory: &Path) -> Result<PathBuf, CollectorError> {
    if !system_directory.is_absolute() {
        return Err(CollectorError::CredentialsMissing);
    }
    let directory_metadata = std::fs::symlink_metadata(system_directory)
        .map_err(|_| CollectorError::CredentialsMissing)?;
    if !directory_metadata.is_dir() || is_reparse_point(&directory_metadata) {
        return Err(CollectorError::CredentialsMissing);
    }
    let executable = system_directory.join("wsl.exe");
    let executable_metadata =
        std::fs::symlink_metadata(&executable).map_err(|_| CollectorError::CredentialsMissing)?;
    if !executable_metadata.is_file() || is_reparse_point(&executable_metadata) {
        return Err(CollectorError::CredentialsMissing);
    }
    Ok(executable)
}

#[cfg(windows)]
fn is_reparse_point(metadata: &std::fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;
    metadata.file_attributes()
        & windows_sys::Win32::Storage::FileSystem::FILE_ATTRIBUTE_REPARSE_POINT
        != 0
}

#[cfg(all(not(windows), test))]
fn is_reparse_point(metadata: &std::fs::Metadata) -> bool {
    metadata.file_type().is_symlink()
}

#[cfg(windows)]
fn windows_system_directory() -> Result<PathBuf, CollectorError> {
    use windows_sys::Win32::System::SystemInformation::GetSystemDirectoryW;
    let required = unsafe { GetSystemDirectoryW(std::ptr::null_mut(), 0) };
    if required == 0 {
        return Err(CollectorError::Internal);
    }
    let mut buffer = vec![0u16; required as usize + 1];
    let written = unsafe { GetSystemDirectoryW(buffer.as_mut_ptr(), buffer.len() as u32) };
    if written == 0 || written as usize >= buffer.len() {
        return Err(CollectorError::Internal);
    }
    buffer.truncate(written as usize);
    Ok(PathBuf::from(
        String::from_utf16(&buffer).map_err(|_| CollectorError::Internal)?,
    ))
}

impl ClaudeTokenSource for WslCredentialSource {
    fn claude_token(
        &self,
    ) -> Pin<Box<dyn Future<Output = Result<SecretString, CollectorError>> + Send + '_>> {
        Box::pin(WslCredentialSource::claude_token(self))
    }
}

#[cfg(windows)]
struct SystemWslProcess {
    executable: PathBuf,
}

#[cfg(windows)]
impl WslProcess for SystemWslProcess {
    fn run<'a>(
        &'a self,
        args: &'a [&'a str],
    ) -> Pin<Box<dyn Future<Output = Result<ProcessOutput, CollectorError>> + Send + 'a>> {
        Box::pin(async move { run_bounded(&self.executable, args).await })
    }
}

#[cfg(windows)]
async fn run_bounded(
    executable: &std::path::Path,
    args: &[&str],
) -> Result<ProcessOutput, CollectorError> {
    use std::process::Stdio;
    use tokio::io::AsyncReadExt;

    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let mut command = tokio::process::Command::new(executable);
    command
        .args(args)
        .creation_flags(CREATE_NO_WINDOW)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true);
    let mut child = command.spawn().map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            CollectorError::CredentialsMissing
        } else {
            CollectorError::Internal
        }
    })?;
    let stdout_pipe = child.stdout.take().ok_or(CollectorError::Internal)?;
    let read_limit = MAX_CREDENTIAL_BYTES + 1;
    let operation = async {
        let mut stdout = Vec::new();
        stdout_pipe
            .take(read_limit)
            .read_to_end(&mut stdout)
            .await
            .map_err(|_| CollectorError::Internal)?;
        if stdout.len() as u64 > MAX_CREDENTIAL_BYTES {
            stdout.zeroize();
            let _ = child.kill().await;
            let _ = child.wait().await;
            return Err(CollectorError::CredentialFileInvalid);
        }
        let status = child.wait().await.map_err(|_| CollectorError::Internal)?;
        Ok::<_, CollectorError>(ProcessOutput {
            status: status.code().unwrap_or(-1),
            stdout,
        })
    };
    match tokio::time::timeout(PROCESS_TIMEOUT, operation).await {
        Ok(result) => result,
        Err(_) => {
            let _ = child.kill().await;
            let _ = child.wait().await;
            Err(CollectorError::Timeout)
        }
    }
}
