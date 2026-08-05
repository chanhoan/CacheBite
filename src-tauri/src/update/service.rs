use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::time::Instant;

use tokio::sync::watch;

use super::{
    channel::{
        should_check, AUTOMATIC_CHECK_INTERVAL, PANEL_OPEN_CHECK_FLOOR, STARTUP_CHECK_DELAY,
    },
    feed::{InstallOutcome, InstallProgress, PendingUpdate, ProgressSink, ReleaseFeed},
    state::{UpdateStateDto, UpdateStatus},
};

struct Inner {
    status: watch::Sender<UpdateStatus>,
    feed: Arc<dyn ReleaseFeed>,
    current_version: String,
    last_checked: Mutex<Option<Instant>>,
    in_flight: AtomicBool,
}

/// Releases the re-entrancy latch on every exit path, including the early
/// returns an `?` would take. Without a `Drop` guard a single failed check
/// would wedge the checker permanently.
struct InFlightGuard(Arc<Inner>);

impl InFlightGuard {
    fn acquire(inner: &Arc<Inner>) -> Option<Self> {
        if inner.in_flight.swap(true, Ordering::SeqCst) {
            None
        } else {
            Some(Self(inner.clone()))
        }
    }
}

impl Drop for InFlightGuard {
    fn drop(&mut self) {
        self.0.in_flight.store(false, Ordering::SeqCst);
    }
}

/// Owns the update state machine and the cadence that drives it.
///
/// Cloning shares one `Inner`, which is what lets the scheduler task, the panel
/// nudge and the IPC commands all act on the same state without the service
/// having to be wrapped in a second `Arc` at the `manage` boundary.
#[derive(Clone)]
pub struct UpdateService {
    inner: Arc<Inner>,
}

impl UpdateService {
    pub fn new(feed: Arc<dyn ReleaseFeed>, current_version: String) -> Self {
        let (status, _) = watch::channel(UpdateStatus::Idle);
        Self {
            inner: Arc::new(Inner {
                status,
                feed,
                current_version,
                last_checked: Mutex::new(None),
                in_flight: AtomicBool::new(false),
            }),
        }
    }

    pub fn current_version(&self) -> &str {
        &self.inner.current_version
    }

    pub fn subscribe(&self) -> watch::Receiver<UpdateStatus> {
        self.inner.status.subscribe()
    }

    pub fn snapshot(&self) -> UpdateStateDto {
        UpdateStateDto {
            current_version: self.inner.current_version.clone(),
            status: self.inner.status.borrow().clone(),
        }
    }

    /// `send_replace` rather than `send`: the state must keep advancing even in
    /// the window before `emit_update_state` has subscribed, and a dropped
    /// renderer must never turn a state transition into an error.
    fn publish(&self, status: UpdateStatus) {
        self.inner.status.send_replace(status);
    }

    fn last_checked(&self) -> Option<Instant> {
        *self
            .inner
            .last_checked
            .lock()
            .expect("update service mutex poisoned")
    }

    /// Startup delay, then a slow background sweep. The sweep consults
    /// `should_check`, so a check triggered by a panel reveal a minute ago
    /// correctly suppresses this round.
    pub fn spawn_scheduler(&self) {
        let service = self.clone();
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(STARTUP_CHECK_DELAY).await;
            loop {
                if should_check(
                    service.last_checked(),
                    Instant::now(),
                    AUTOMATIC_CHECK_INTERVAL,
                ) {
                    service.run_check().await;
                }
                tokio::time::sleep(AUTOMATIC_CHECK_INTERVAL).await;
            }
        });
    }

    /// Fire-and-forget nudge from the panel reveal.
    ///
    /// Returns immediately and never reports failure: it is called from the
    /// `toggle_panel` command path, and an update check has no business
    /// delaying or breaking the gesture that opens the panel.
    pub fn check_on_panel_reveal(&self) {
        if !should_check(self.last_checked(), Instant::now(), PANEL_OPEN_CHECK_FLOOR) {
            return;
        }
        let service = self.clone();
        tauri::async_runtime::spawn(async move {
            service.run_check().await;
        });
    }

    /// The manual `Check for updates` button. Deliberately skips the throttle —
    /// the user asking is the signal.
    pub async fn check_now(&self) {
        self.run_check().await;
    }

    async fn run_check(&self) {
        // A background check must not overwrite progress the user is watching.
        if self.inner.status.borrow().is_installing() {
            return;
        }
        let Some(_guard) = InFlightGuard::acquire(&self.inner) else {
            return;
        };
        self.publish(UpdateStatus::Checking);
        let outcome = self.inner.feed.check().await;
        *self
            .inner
            .last_checked
            .lock()
            .expect("update service mutex poisoned") = Some(Instant::now());
        self.publish(match outcome {
            Ok(None) => UpdateStatus::UpToDate,
            Ok(Some(pending)) => UpdateStatus::Available {
                version: pending.version,
                notes: pending.notes,
            },
            Err(reason) => UpdateStatus::Failed { reason },
        });
    }

    /// Downloads, verifies and installs the release the user was shown.
    ///
    /// Only valid from `Available`: installing anything else would act on a
    /// release the user never saw. On Windows this never returns — the plugin
    /// hands off to NSIS and exits the process — which is why `Installing` is
    /// published by the feed before the install begins rather than after.
    pub async fn install(&self, app: &tauri::AppHandle) {
        let pending = match &*self.inner.status.borrow() {
            UpdateStatus::Available { version, notes } => PendingUpdate {
                version: version.clone(),
                notes: notes.clone(),
            },
            _ => return,
        };
        let Some(_guard) = InFlightGuard::acquire(&self.inner) else {
            return;
        };
        self.publish(UpdateStatus::Downloading {
            received: 0,
            total: None,
        });

        let progress_service = self.clone();
        let installing_version = pending.version.clone();
        let sink: ProgressSink = Arc::new(move |progress| match progress {
            InstallProgress::Downloading { received, total } => {
                progress_service.publish(UpdateStatus::Downloading { received, total });
            }
            InstallProgress::Installing => {
                progress_service.publish(UpdateStatus::Installing {
                    version: installing_version.clone(),
                });
            }
        });

        match self.inner.feed.install(pending, sink).await {
            Ok(InstallOutcome::RestartRequired) => app.restart(),
            // The fixture feed stops here on purpose so the E2E runner survives.
            Ok(InstallOutcome::Completed) => {}
            Err(reason) => self.publish(UpdateStatus::Failed { reason }),
        }
    }
}
