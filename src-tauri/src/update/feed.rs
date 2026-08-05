use std::{
    future::Future,
    pin::Pin,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

use super::{
    channel::{channel_for_version, manifest_url},
    state::{truncate_notes, UpdateFailure},
};

/// A release the feed has confirmed is newer than the running build.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PendingUpdate {
    pub version: String,
    pub notes: Option<String>,
}

/// What an in-flight install reports back while it runs.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InstallProgress {
    Downloading {
        received: u64,
        total: Option<u64>,
    },
    /// The bytes are on disk and verified; the installer is about to run.
    Installing,
}

/// Whether the caller still has a process to restart once `install` returns.
///
/// Windows never reaches this: the NSIS path calls `std::process::exit(0)` from
/// inside the plugin. macOS and Linux replace the bundle in-process and hand
/// the restart back to us.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InstallOutcome {
    RestartRequired,
    /// The fixture feed, which must never take the E2E runner down with it.
    Completed,
}

pub type ProgressSink = Arc<dyn Fn(InstallProgress) + Send + Sync>;

pub type FeedFuture<'a, T> = Pin<Box<dyn Future<Output = Result<T, UpdateFailure>> + Send + 'a>>;

/// What the service needs from a release source, so tests and E2E never touch
/// GitHub. Mirrors `collectors::Collector`, which exists for the same reason.
pub trait ReleaseFeed: Send + Sync {
    fn check(&self) -> FeedFuture<'_, Option<PendingUpdate>>;
    fn install(
        &self,
        pending: PendingUpdate,
        on_progress: ProgressSink,
    ) -> FeedFuture<'_, InstallOutcome>;
}

/// The production feed: `tauri-plugin-updater` pointed at the channel manifest
/// this build is allowed to read.
///
/// The plugin owns its own HTTP client, which follows redirects — required,
/// because GitHub release asset URLs 302 to `objects.githubusercontent.com`.
/// The collectors' `redirect::Policy::none()` client cannot be reused here.
pub struct TauriUpdaterFeed {
    app: tauri::AppHandle,
    current_version: String,
    /// The `Update` handle produced by the last successful check, kept so
    /// `install` acts on exactly the release the user was shown rather than
    /// whatever the manifest says a moment later.
    pending: std::sync::Mutex<Option<tauri_plugin_updater::Update>>,
}

impl TauriUpdaterFeed {
    pub fn new(app: tauri::AppHandle, current_version: String) -> Self {
        Self {
            app,
            current_version,
            pending: std::sync::Mutex::new(None),
        }
    }

    fn build(&self) -> Result<tauri_plugin_updater::Updater, UpdateFailure> {
        use tauri_plugin_updater::UpdaterExt;

        let endpoint = manifest_url(channel_for_version(&self.current_version))
            .parse()
            .map_err(|_| UpdateFailure::MetadataInvalid)?;
        self.app
            .updater_builder()
            .endpoints(vec![endpoint])
            .map_err(|_| UpdateFailure::MetadataInvalid)?
            .timeout(std::time::Duration::from_secs(20))
            .on_before_exit(|| {
                eprintln!("[CacheBite:update] installer starting; the app will exit");
            })
            .build()
            .map_err(|_| UpdateFailure::MetadataInvalid)
    }
}

/// Maps a plugin error onto the renderer-safe failure classes.
///
/// `during_download` disambiguates the two phases that both surface transport
/// errors: a failed manifest fetch is `Offline`, a failed artifact fetch is
/// `DownloadFailed`, and the user-facing sentences differ accordingly.
pub(crate) fn classify(
    error: &tauri_plugin_updater::Error,
    during_download: bool,
) -> UpdateFailure {
    use tauri_plugin_updater::Error;

    match error {
        Error::Reqwest(inner) => {
            if inner.is_connect() || inner.is_timeout() {
                UpdateFailure::Offline
            } else if inner
                .status()
                .is_some_and(|status| status.as_u16() == 403 || status.as_u16() == 429)
            {
                UpdateFailure::RateLimited
            } else if during_download {
                UpdateFailure::DownloadFailed
            } else {
                UpdateFailure::Offline
            }
        }
        Error::Serialization(_) | Error::Semver(_) | Error::EmptyEndpoints => {
            UpdateFailure::MetadataInvalid
        }
        Error::TargetNotFound(_) | Error::TargetsNotFound(_) => UpdateFailure::ArtifactMissing,
        Error::Io(_) => UpdateFailure::DownloadFailed,
        Error::Minisign(_) | Error::Base64(_) => UpdateFailure::VerificationFailed,
        _ => {
            if during_download {
                UpdateFailure::InstallFailed
            } else {
                UpdateFailure::MetadataInvalid
            }
        }
    }
}

impl ReleaseFeed for TauriUpdaterFeed {
    fn check(&self) -> FeedFuture<'_, Option<PendingUpdate>> {
        Box::pin(async move {
            let updater = self.build()?;
            // `Updater::check` resolves the platform artifact *before* it
            // consumes the version comparison, so a manifest that omits this
            // platform's key fails here even when the running version already
            // matches. That reads as `ArtifactMissing`, never `UpToDate`.
            let found = updater
                .check()
                .await
                .map_err(|error| classify(&error, false))?;
            let Some(update) = found else {
                *self.pending.lock().expect("update feed mutex poisoned") = None;
                return Ok(None);
            };
            let pending = PendingUpdate {
                version: update.version.clone(),
                notes: update.body.as_deref().and_then(truncate_notes),
            };
            *self.pending.lock().expect("update feed mutex poisoned") = Some(update);
            Ok(Some(pending))
        })
    }

    fn install(
        &self,
        _pending: PendingUpdate,
        on_progress: ProgressSink,
    ) -> FeedFuture<'_, InstallOutcome> {
        Box::pin(async move {
            let update = self
                .pending
                .lock()
                .expect("update feed mutex poisoned")
                .take()
                .ok_or(UpdateFailure::InstallFailed)?;

            let chunk_sink = on_progress.clone();
            let received = std::sync::atomic::AtomicU64::new(0);
            let finish_sink = on_progress;

            update
                .download_and_install(
                    move |chunk, total| {
                        let seen =
                            received.fetch_add(chunk as u64, Ordering::SeqCst) + chunk as u64;
                        chunk_sink(InstallProgress::Downloading {
                            received: seen,
                            total,
                        });
                    },
                    move || finish_sink(InstallProgress::Installing),
                )
                .await
                .map_err(|error| classify(&error, true))?;

            Ok(InstallOutcome::RestartRequired)
        })
    }
}

/// Which deterministic outcome the fixture feed serves.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FixtureScenario {
    None,
    Available,
    Failed,
}

impl FixtureScenario {
    /// Unknown values fall back to `None` rather than failing the build: an
    /// E2E typo must not look like a broken updater.
    pub fn parse(raw: Option<&str>) -> Self {
        match raw {
            Some("available") => FixtureScenario::Available,
            Some("failed") => FixtureScenario::Failed,
            _ => FixtureScenario::None,
        }
    }
}

/// The deterministic feed used by unit tests and native E2E. Never touches the
/// network and never exits the process.
pub struct FixtureFeed {
    scenario: FixtureScenario,
    probes: AtomicUsize,
}

impl FixtureFeed {
    pub fn new(scenario: FixtureScenario) -> Self {
        Self {
            scenario,
            probes: AtomicUsize::new(0),
        }
    }

    pub fn from_env() -> Self {
        Self::new(FixtureScenario::parse(
            std::env::var("CACHEBITE_E2E_UPDATE").ok().as_deref(),
        ))
    }

    /// How many checks this feed has served. Surfaced through the
    /// webdriver-only probe command so the E2E can prove the panel-reveal
    /// floor actually suppresses a second check.
    pub fn probes(&self) -> usize {
        self.probes.load(Ordering::SeqCst)
    }
}

impl ReleaseFeed for FixtureFeed {
    fn check(&self) -> FeedFuture<'_, Option<PendingUpdate>> {
        Box::pin(async move {
            self.probes.fetch_add(1, Ordering::SeqCst);
            match self.scenario {
                FixtureScenario::None => Ok(None),
                FixtureScenario::Available => Ok(Some(PendingUpdate {
                    version: "9.9.9".to_owned(),
                    notes: Some("Fixture release notes.".to_owned()),
                })),
                FixtureScenario::Failed => Err(UpdateFailure::Offline),
            }
        })
    }

    fn install(
        &self,
        pending: PendingUpdate,
        on_progress: ProgressSink,
    ) -> FeedFuture<'_, InstallOutcome> {
        Box::pin(async move {
            if self.scenario == FixtureScenario::Failed {
                return Err(UpdateFailure::InstallFailed);
            }
            on_progress(InstallProgress::Downloading {
                received: 512,
                total: Some(1024),
            });
            on_progress(InstallProgress::Downloading {
                received: 1024,
                total: Some(1024),
            });
            on_progress(InstallProgress::Installing);
            // Deliberately stops here. Restarting would take the E2E runner
            // down with it, and the assertion is about the state, not the exit.
            let _ = pending;
            Ok(InstallOutcome::Completed)
        })
    }
}
