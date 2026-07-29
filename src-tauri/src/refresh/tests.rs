use std::{
    collections::VecDeque,
    fs,
    future::Future,
    path::PathBuf,
    pin::Pin,
    sync::{Arc, Mutex},
};

use ::time::{Duration as TimeDuration, OffsetDateTime};
use tokio::{
    sync::Notify,
    time::{self, Duration},
};

use super::*;
use crate::{
    collectors::Collector,
    domain::{
        CollectionOutcome, FailureClass, Provider, ProviderUsageSnapshot, Source, UsageWindow,
    },
    store::{HistoryRepository, ProviderRecord, SnapshotRepository, SnapshotStore},
};

#[derive(Default)]
struct FailOnceSnapshots {
    attempts: Mutex<usize>,
}

impl SnapshotPersistence for FailOnceSnapshots {
    fn load(&self) -> std::io::Result<SnapshotStore> {
        Ok(SnapshotStore::default())
    }

    fn save_provider(&self, _record: ProviderRecord) -> std::io::Result<()> {
        let mut attempts = self.attempts.lock().expect("snapshot attempts");
        *attempts += 1;
        if *attempts == 1 {
            Err(std::io::Error::other("/secret/snapshots.json payload"))
        } else {
            Ok(())
        }
    }
}

#[derive(Default)]
struct FailOnceHistory {
    attempts: Mutex<usize>,
}

#[derive(Default)]
struct NoopSnapshots;

impl SnapshotPersistence for NoopSnapshots {
    fn load(&self) -> std::io::Result<SnapshotStore> {
        Ok(SnapshotStore::default())
    }
    fn save_provider(&self, _record: ProviderRecord) -> std::io::Result<()> {
        Ok(())
    }
}

#[derive(Default)]
struct ControlledHistory {
    failing: Mutex<bool>,
    captured: Mutex<Vec<OffsetDateTime>>,
}

impl HistoryPersistence for ControlledHistory {
    fn append_success(
        &self,
        snapshot: &ProviderUsageSnapshot,
        _now: OffsetDateTime,
    ) -> std::io::Result<bool> {
        if *self.failing.lock().expect("failing") {
            return Err(std::io::Error::other("still failing"));
        }
        self.captured
            .lock()
            .expect("captured")
            .push(snapshot.captured_at);
        Ok(true)
    }
}

struct GatedCollector {
    ready: Arc<Notify>,
}

impl Collector for GatedCollector {
    fn provider(&self) -> Provider {
        Provider::Claude
    }
    fn collect(&self) -> Pin<Box<dyn Future<Output = CollectionOutcome> + Send + '_>> {
        Box::pin(async {
            self.ready.notified().await;
            success(Provider::Claude, OffsetDateTime::now_utc(), None)
        })
    }
}

#[derive(Default)]
struct PublicationProbeSnapshots {
    state: Mutex<Option<tokio::sync::watch::Receiver<ProviderState>>>,
    revision_seen_during_write: Mutex<Option<u64>>,
}

impl SnapshotPersistence for PublicationProbeSnapshots {
    fn load(&self) -> std::io::Result<SnapshotStore> {
        Ok(SnapshotStore::default())
    }
    fn save_provider(&self, _record: ProviderRecord) -> std::io::Result<()> {
        let revision = self
            .state
            .lock()
            .expect("state probe")
            .as_ref()
            .map(|state| state.borrow().revision);
        *self
            .revision_seen_during_write
            .lock()
            .expect("revision probe") = revision;
        Ok(())
    }
}

impl HistoryPersistence for FailOnceHistory {
    fn append_success(
        &self,
        _snapshot: &ProviderUsageSnapshot,
        _now: OffsetDateTime,
    ) -> std::io::Result<bool> {
        let mut attempts = self.attempts.lock().expect("history attempts");
        *attempts += 1;
        if *attempts == 1 {
            Err(std::io::Error::other("credential=secret history payload"))
        } else {
            Ok(true)
        }
    }
}

fn persistence_dir() -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "cachebite-refresh-persistence-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&path);
    fs::create_dir(&path).expect("temp persistence dir");
    path
}

struct FakeCollector {
    provider: Provider,
    calls: Arc<Mutex<usize>>,
    outcomes: Arc<Mutex<VecDeque<CollectionOutcome>>>,
}

impl FakeCollector {
    fn new(provider: Provider, outcomes: Vec<CollectionOutcome>) -> (Arc<Self>, Arc<Mutex<usize>>) {
        let calls = Arc::new(Mutex::new(0));
        (
            Arc::new(Self {
                provider,
                calls: Arc::clone(&calls),
                outcomes: Arc::new(Mutex::new(outcomes.into())),
            }),
            calls,
        )
    }
}

impl Collector for FakeCollector {
    fn provider(&self) -> Provider {
        self.provider
    }
    fn collect(&self) -> Pin<Box<dyn Future<Output = CollectionOutcome> + Send + '_>> {
        Box::pin(async {
            *self.calls.lock().expect("calls") += 1;
            self.outcomes
                .lock()
                .expect("outcomes")
                .pop_front()
                .unwrap_or(CollectionOutcome::Failed {
                    class: FailureClass::Internal,
                })
        })
    }
}

fn success(
    provider: Provider,
    captured_at: OffsetDateTime,
    resets_at: Option<OffsetDateTime>,
) -> CollectionOutcome {
    CollectionOutcome::Success {
        snapshot: ProviderUsageSnapshot {
            provider,
            plan_type: None,
            session: Some(UsageWindow::new(25.0, 300, resets_at).expect("window")),
            weekly: None,
            captured_at,
            source: if provider == Provider::Claude {
                Source::OauthApi
            } else {
                Source::CliRpc
            },
            is_cached: false,
            revision: 999,
        },
    }
}

fn config() -> SchedulerConfig {
    SchedulerConfig {
        poll_interval: Duration::from_secs(900),
        debounce: Duration::from_millis(500),
        ttl: Duration::from_secs(1800),
        backoff_base: Duration::from_secs(2),
        backoff_cap: Duration::from_secs(8),
        jitter: Duration::ZERO,
    }
}

async fn settle() {
    for _ in 0..5 {
        tokio::task::yield_now().await;
    }
}

#[cfg(feature = "tauri-ipc")]
#[test]
fn actor_can_start_without_an_entered_tokio_runtime() {
    let (collector, _calls) = FakeCollector::new(Provider::Claude, Vec::new());

    let handle = RefreshHandle::spawn(collector, config());

    assert_eq!(handle.provider(), Provider::Claude);
}

#[tokio::test(start_paused = true)]
async fn startup_fetches_immediately_and_actor_owns_revision() {
    let (collector, calls) = FakeCollector::new(
        Provider::Claude,
        vec![success(Provider::Claude, OffsetDateTime::now_utc(), None)],
    );
    let handle = RefreshHandle::spawn(collector, config());
    settle().await;
    assert_eq!(*calls.lock().expect("calls"), 1);
    let state = handle.current();
    assert_eq!(state.revision, 1);
    assert_eq!(state.snapshot.expect("snapshot").revision, 1);
}

#[tokio::test(start_paused = true)]
async fn manual_focus_and_resume_triggers_are_debounced_together() {
    let now = OffsetDateTime::now_utc();
    let (collector, calls) = FakeCollector::new(
        Provider::Claude,
        vec![
            success(Provider::Claude, now, None),
            success(Provider::Claude, now, None),
        ],
    );
    let handle = RefreshHandle::spawn(collector, config());
    settle().await;
    handle.trigger(RefreshReason::Manual).await.expect("manual");
    handle.trigger(RefreshReason::Focus).await.expect("focus");
    handle.trigger(RefreshReason::Resume).await.expect("resume");
    settle().await;
    time::advance(Duration::from_millis(499)).await;
    settle().await;
    assert_eq!(*calls.lock().expect("calls"), 1);
    time::advance(Duration::from_millis(1)).await;
    settle().await;
    assert_eq!(*calls.lock().expect("calls"), 2);
}

#[tokio::test(start_paused = true)]
async fn poll_runs_every_fifteen_minutes() {
    let now = OffsetDateTime::now_utc();
    let (collector, calls) = FakeCollector::new(
        Provider::Claude,
        vec![
            success(Provider::Claude, now, None),
            success(Provider::Claude, now, None),
        ],
    );
    let _handle = RefreshHandle::spawn(collector, config());
    settle().await;
    time::advance(Duration::from_secs(899)).await;
    settle().await;
    assert_eq!(*calls.lock().expect("calls"), 1);
    time::advance(Duration::from_secs(1)).await;
    settle().await;
    assert_eq!(*calls.lock().expect("calls"), 2);
}

#[tokio::test(start_paused = true)]
async fn failures_back_off_exponentially_and_cap() {
    let failed = CollectionOutcome::Failed {
        class: FailureClass::Network,
    };
    let (collector, calls) = FakeCollector::new(
        Provider::Claude,
        vec![failed.clone(), failed.clone(), failed.clone(), failed],
    );
    let _handle = RefreshHandle::spawn(collector, config());
    settle().await;
    for delay in [2, 4, 8] {
        time::advance(Duration::from_secs(delay - 1)).await;
        settle().await;
        let before = *calls.lock().expect("calls");
        time::advance(Duration::from_secs(1)).await;
        settle().await;
        assert_eq!(*calls.lock().expect("calls"), before + 1);
    }
}

#[tokio::test(start_paused = true)]
async fn ttl_marks_snapshot_expired_with_a_new_revision() {
    let now = OffsetDateTime::now_utc();
    let (collector, _) =
        FakeCollector::new(Provider::Claude, vec![success(Provider::Claude, now, None)]);
    let mut scheduler = config();
    scheduler.poll_interval = Duration::from_secs(3600);
    let handle = RefreshHandle::spawn(collector, scheduler);
    settle().await;
    time::advance(Duration::from_secs(1800)).await;
    settle().await;
    let state = handle.current();
    assert!(state.expired);
    assert_eq!(state.revision, 2);
}

#[tokio::test(start_paused = true)]
async fn reset_timer_marks_window_pending_and_refreshes() {
    let reset = OffsetDateTime::now_utc() + TimeDuration::seconds(10);
    let (collector, calls) = FakeCollector::new(
        Provider::Claude,
        vec![
            success(Provider::Claude, OffsetDateTime::now_utc(), Some(reset)),
            success(Provider::Claude, OffsetDateTime::now_utc(), None),
        ],
    );
    let handle = RefreshHandle::spawn(collector, config());
    settle().await;
    time::advance(Duration::from_secs(10)).await;
    settle().await;
    assert_eq!(*calls.lock().expect("calls"), 2);
    assert!(handle.current().reset_pending || handle.current().revision >= 3);
}

#[tokio::test(start_paused = true)]
async fn provider_actors_progress_independently() {
    let now = OffsetDateTime::now_utc();
    let (claude, claude_calls) =
        FakeCollector::new(Provider::Claude, vec![success(Provider::Claude, now, None)]);
    let (codex, codex_calls) =
        FakeCollector::new(Provider::Codex, vec![success(Provider::Codex, now, None)]);
    let service = RefreshService::new(
        RefreshHandle::spawn(claude, config()),
        RefreshHandle::spawn(codex, config()),
    );
    settle().await;
    assert_eq!(*claude_calls.lock().expect("calls"), 1);
    assert_eq!(*codex_calls.lock().expect("calls"), 1);
    let states = service.get_provider_states();
    assert_eq!(states.claude.provider, Provider::Claude);
    assert_eq!(states.codex.provider, Provider::Codex);
}

#[tokio::test(start_paused = true)]
async fn persistent_actor_hydrates_cache_and_persists_fresh_outcome_and_history() {
    let dir = persistence_dir();
    let snapshots = Arc::new(SnapshotRepository::new(&dir));
    let history = Arc::new(HistoryRepository::new(&dir));
    let cached = match success(Provider::Claude, OffsetDateTime::now_utc(), None) {
        CollectionOutcome::Success { mut snapshot } => {
            snapshot.revision = 5;
            snapshot
        }
        _ => unreachable!(),
    };
    snapshots
        .save_provider(ProviderRecord::success(cached))
        .expect("seed cache");
    let fresh_at = OffsetDateTime::now_utc();
    let (collector, _) = FakeCollector::new(
        Provider::Claude,
        vec![success(Provider::Claude, fresh_at, None)],
    );
    let handle = RefreshHandle::spawn_persistent(
        collector,
        config(),
        RefreshPersistence::new(Arc::clone(&snapshots), Arc::clone(&history)),
    )
    .expect("spawn");
    assert_eq!(handle.current().revision, 5);
    assert!(handle.current().snapshot.expect("cached").is_cached);
    settle().await;
    assert_eq!(
        snapshots
            .load()
            .expect("snapshots")
            .claude
            .expect("claude")
            .revision,
        6
    );
    assert_eq!(
        history
            .load_at(fresh_at)
            .expect("history")
            .claude
            .samples
            .len(),
        1
    );
    fs::remove_dir_all(dir).expect("cleanup");
}

#[tokio::test(start_paused = true)]
async fn persistence_failures_are_sanitized_and_retried_after_state_publication() {
    let now = OffsetDateTime::now_utc();
    let (collector, _) = FakeCollector::new(
        Provider::Claude,
        vec![
            success(Provider::Claude, now, None),
            success(Provider::Claude, now + TimeDuration::seconds(1), None),
        ],
    );
    let snapshots = Arc::new(FailOnceSnapshots::default());
    let history = Arc::new(FailOnceHistory::default());
    let diagnostics = Arc::new(Mutex::new(Vec::new()));
    let captured = Arc::clone(&diagnostics);
    let persistence = RefreshPersistence::with_diagnostics(
        snapshots.clone(),
        history.clone(),
        Arc::new(move |diagnostic| captured.lock().expect("diagnostics").push(diagnostic)),
    );
    let handle = RefreshHandle::spawn_persistent(collector, config(), persistence).expect("spawn");

    settle().await;
    assert_eq!(handle.current().revision, 1);
    assert_eq!(
        *diagnostics.lock().expect("diagnostics"),
        vec![
            PersistenceDiagnostic {
                provider: Provider::Claude,
                category: PersistenceCategory::Snapshot
            },
            PersistenceDiagnostic {
                provider: Provider::Claude,
                category: PersistenceCategory::History
            },
        ]
    );

    handle
        .trigger(RefreshReason::Manual)
        .await
        .expect("trigger");
    settle().await;
    time::advance(Duration::from_millis(500)).await;
    settle().await;
    assert_eq!(handle.current().revision, 2);
    assert_eq!(*snapshots.attempts.lock().expect("snapshot attempts"), 2);
    assert_eq!(*history.attempts.lock().expect("history attempts"), 3);
    assert_eq!(diagnostics.lock().expect("diagnostics").len(), 2);
}

#[test]
fn pending_history_is_capped_to_repository_limit_and_retries_retained_fifo() {
    let start = OffsetDateTime::now_utc();
    let mut pending = VecDeque::new();
    let history = Arc::new(ControlledHistory::default());
    *history.failing.lock().expect("failing") = true;
    let persistence = RefreshPersistence::with_diagnostics(
        Arc::new(NoopSnapshots),
        history.clone(),
        Arc::new(|_| {}),
    );

    for offset in 0..=MAX_PENDING_HISTORY {
        let snapshot = match success(
            Provider::Claude,
            start + TimeDuration::seconds(offset as i64),
            None,
        ) {
            CollectionOutcome::Success { snapshot } => snapshot,
            _ => unreachable!(),
        };
        enqueue_pending_history(&mut pending, snapshot);
        retry_pending_history(Provider::Claude, &persistence, &mut pending);
    }

    assert_eq!(pending.len(), MAX_PENDING_HISTORY);
    assert_eq!(
        pending.front().expect("oldest retained").captured_at,
        start + TimeDuration::seconds(1)
    );
    assert_eq!(
        pending.back().expect("newest retained").captured_at,
        start + TimeDuration::seconds(MAX_PENDING_HISTORY as i64)
    );

    *history.failing.lock().expect("failing") = false;
    while !pending.is_empty() {
        retry_pending_history(Provider::Claude, &persistence, &mut pending);
    }
    assert!(pending.is_empty());
    let captured = history.captured.lock().expect("captured");
    assert_eq!(captured.len(), MAX_PENDING_HISTORY);
    assert_eq!(captured.first(), Some(&(start + TimeDuration::seconds(1))));
    assert_eq!(
        captured.last(),
        Some(&(start + TimeDuration::seconds(MAX_PENDING_HISTORY as i64)))
    );
    assert!(captured.windows(2).all(|pair| pair[0] < pair[1]));
}

#[tokio::test(start_paused = true)]
async fn publishes_watch_state_before_attempting_persistence() {
    let ready = Arc::new(Notify::new());
    let snapshots = Arc::new(PublicationProbeSnapshots::default());
    let handle = RefreshHandle::spawn_persistent(
        Arc::new(GatedCollector {
            ready: ready.clone(),
        }),
        config(),
        RefreshPersistence::with_diagnostics(
            snapshots.clone(),
            Arc::new(ControlledHistory::default()),
            Arc::new(|_| {}),
        ),
    )
    .expect("spawn");
    *snapshots.state.lock().expect("state probe") = Some(handle.subscribe());

    ready.notify_one();
    settle().await;

    assert_eq!(
        *snapshots
            .revision_seen_during_write
            .lock()
            .expect("revision probe"),
        Some(1)
    );
}
