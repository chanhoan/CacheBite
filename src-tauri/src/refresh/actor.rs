use std::{collections::VecDeque, future::pending, io, sync::Arc};

use ::time::OffsetDateTime;
use serde::{Deserialize, Serialize};
use tokio::{
    sync::{mpsc, watch},
    task::JoinHandle,
    time::{self, Duration, Instant},
};

use crate::{
    collectors::Collector,
    domain::{CollectionOutcome, FailureClass, Provider, ProviderUsageSnapshot, UnavailableReason},
    store::{
        HistoryRepository, OutcomeMetadata, ProviderRecord, SnapshotRepository, SnapshotStore,
    },
};

pub(crate) const MAX_PENDING_HISTORY: usize = 3_000;

pub trait SnapshotPersistence: Send + Sync {
    fn load(&self) -> io::Result<SnapshotStore>;
    fn save_provider(&self, record: ProviderRecord) -> io::Result<()>;
}

impl SnapshotPersistence for SnapshotRepository {
    fn load(&self) -> io::Result<SnapshotStore> {
        SnapshotRepository::load(self)
    }
    fn save_provider(&self, record: ProviderRecord) -> io::Result<()> {
        SnapshotRepository::save_provider(self, record)
    }
}

pub trait HistoryPersistence: Send + Sync {
    fn append_success(
        &self,
        snapshot: &ProviderUsageSnapshot,
        now: OffsetDateTime,
    ) -> io::Result<bool>;

    fn append_success_batch(
        &self,
        snapshots: &[ProviderUsageSnapshot],
        now: OffsetDateTime,
    ) -> io::Result<usize> {
        let mut appended = 0;
        for snapshot in snapshots {
            appended += usize::from(self.append_success(snapshot, now)?);
        }
        Ok(appended)
    }
}

impl HistoryPersistence for HistoryRepository {
    fn append_success(
        &self,
        snapshot: &ProviderUsageSnapshot,
        now: OffsetDateTime,
    ) -> io::Result<bool> {
        HistoryRepository::append_success(self, snapshot, now)
    }

    fn append_success_batch(
        &self,
        snapshots: &[ProviderUsageSnapshot],
        now: OffsetDateTime,
    ) -> io::Result<usize> {
        HistoryRepository::append_success_batch(self, snapshots, now)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PersistenceCategory {
    Snapshot,
    History,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PersistenceDiagnostic {
    pub provider: Provider,
    pub category: PersistenceCategory,
}

#[derive(Clone)]
pub struct RefreshPersistence {
    snapshots: Arc<dyn SnapshotPersistence>,
    history: Arc<dyn HistoryPersistence>,
    diagnostic: Arc<dyn Fn(PersistenceDiagnostic) + Send + Sync>,
}
impl RefreshPersistence {
    pub fn new(snapshots: Arc<SnapshotRepository>, history: Arc<HistoryRepository>) -> Self {
        Self::with_diagnostics(
            snapshots,
            history,
            Arc::new(|diagnostic| {
                tracing::warn!(
                    provider = ?diagnostic.provider,
                    category = ?diagnostic.category,
                    "persistence write failed; retained for retry"
                );
            }),
        )
    }

    pub fn with_diagnostics<S, H>(
        snapshots: Arc<S>,
        history: Arc<H>,
        diagnostic: Arc<dyn Fn(PersistenceDiagnostic) + Send + Sync>,
    ) -> Self
    where
        S: SnapshotPersistence + 'static,
        H: HistoryPersistence + 'static,
    {
        Self {
            snapshots,
            history,
            diagnostic,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RefreshReason {
    Manual,
    Focus,
    Resume,
}

#[derive(Clone, Copy, Debug)]
pub struct SchedulerConfig {
    pub poll_interval: Duration,
    pub debounce: Duration,
    pub ttl: Duration,
    pub backoff_base: Duration,
    pub backoff_cap: Duration,
    pub jitter: Duration,
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            poll_interval: Duration::from_secs(15 * 60),
            debounce: Duration::from_millis(500),
            ttl: Duration::from_secs(30 * 60),
            backoff_base: Duration::from_secs(2),
            backoff_cap: Duration::from_secs(5 * 60),
            jitter: Duration::from_millis(250),
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ProviderState {
    pub provider: Provider,
    pub snapshot: Option<ProviderUsageSnapshot>,
    pub failure_class: Option<FailureClass>,
    pub unavailable_reason: Option<UnavailableReason>,
    pub expired: bool,
    pub reset_pending: bool,
    pub revision: u64,
}

impl ProviderState {
    fn initial(provider: Provider) -> Self {
        Self {
            provider,
            snapshot: None,
            failure_class: None,
            unavailable_reason: None,
            expired: false,
            reset_pending: false,
            revision: 0,
        }
    }
}

enum Command {
    Trigger(RefreshReason),
}

#[derive(Clone)]
pub struct RefreshHandle {
    provider: Provider,
    commands: mpsc::Sender<Command>,
    state: watch::Receiver<ProviderState>,
}

impl RefreshHandle {
    pub fn spawn(collector: Arc<dyn Collector>, config: SchedulerConfig) -> Self {
        let initial = ProviderState::initial(collector.provider());
        Self::spawn_inner(collector, config, None, initial)
    }

    pub fn spawn_persistent(
        collector: Arc<dyn Collector>,
        config: SchedulerConfig,
        persistence: RefreshPersistence,
    ) -> std::io::Result<Self> {
        let provider = collector.provider();
        let store = persistence.snapshots.load()?;
        let record = match provider {
            Provider::Claude => store.claude,
            Provider::Codex => store.codex,
        };
        let initial = record
            .map(provider_state_from_record)
            .unwrap_or_else(|| ProviderState::initial(provider));
        Ok(Self::spawn_inner(
            collector,
            config,
            Some(persistence),
            initial,
        ))
    }

    fn spawn_inner(
        collector: Arc<dyn Collector>,
        config: SchedulerConfig,
        persistence: Option<RefreshPersistence>,
        initial: ProviderState,
    ) -> Self {
        let provider = collector.provider();
        let (commands, receiver) = mpsc::channel(16);
        let (state_tx, state) = watch::channel(initial.clone());
        let actor = run(
            provider,
            collector,
            config,
            receiver,
            state_tx,
            initial,
            persistence,
        );
        if let Ok(runtime) = tokio::runtime::Handle::try_current() {
            runtime.spawn(actor);
        } else {
            #[cfg(feature = "tauri-ipc")]
            tauri::async_runtime::spawn(actor);
            #[cfg(not(feature = "tauri-ipc"))]
            panic!("refresh actor requires an async runtime");
        }
        Self {
            provider,
            commands,
            state,
        }
    }

    pub fn provider(&self) -> Provider {
        self.provider
    }
    pub fn current(&self) -> ProviderState {
        self.state.borrow().clone()
    }
    pub fn subscribe(&self) -> watch::Receiver<ProviderState> {
        self.state.clone()
    }
    pub async fn trigger(&self, reason: RefreshReason) -> Result<(), &'static str> {
        self.commands
            .send(Command::Trigger(reason))
            .await
            .map_err(|_| "refresh actor stopped")
    }
}

async fn run(
    provider: Provider,
    collector: Arc<dyn Collector>,
    config: SchedulerConfig,
    mut commands: mpsc::Receiver<Command>,
    state_tx: watch::Sender<ProviderState>,
    mut state: ProviderState,
    persistence: Option<RefreshPersistence>,
) {
    let mut in_flight = Some(start_collect(Arc::clone(&collector)));
    let mut poll_deadline = Instant::now() + config.poll_interval;
    let mut debounce_deadline = None;
    let mut backoff_deadline = None;
    let mut ttl_deadline = initial_ttl_deadline(&state, config.ttl);
    let mut reset_deadline = state.snapshot.as_ref().and_then(next_reset_deadline);
    let mut consecutive_failures = 0_u32;
    let mut refresh_pending = false;
    let mut pending_snapshot = None;
    let mut pending_history = VecDeque::new();

    loop {
        tokio::select! {
            result = await_collect(&mut in_flight) => {
                in_flight = None;
                state.revision = state.revision.saturating_add(1);
                let mut fresh_sample = None;
                match result {
                    Some(CollectionOutcome::Success { mut snapshot }) if snapshot.provider == provider => {
                        snapshot.revision = state.revision;
                        fresh_sample = Some(snapshot.clone());
                        ttl_deadline = Some(Instant::now() + config.ttl);
                        reset_deadline = next_reset_deadline(&snapshot);
                        state.snapshot = Some(snapshot);
                        state.failure_class = None;
                        state.unavailable_reason = None;
                        state.expired = false;
                        state.reset_pending = false;
                        consecutive_failures = 0;
                        backoff_deadline = None;
                    }
                    Some(CollectionOutcome::CredentialsMissing) => {
                        state.failure_class = None;
                        state.unavailable_reason = Some(UnavailableReason::NotSignedIn);
                        schedule_backoff(&mut consecutive_failures, &mut backoff_deadline, config);
                    }
                    Some(CollectionOutcome::CliMissing) => {
                        state.failure_class = None;
                        state.unavailable_reason = Some(UnavailableReason::NotInstalled);
                        schedule_backoff(&mut consecutive_failures, &mut backoff_deadline, config);
                    }
                    Some(CollectionOutcome::Failed { class }) => {
                        state.failure_class = Some(class);
                        state.unavailable_reason = None;
                        schedule_backoff(&mut consecutive_failures, &mut backoff_deadline, config);
                    }
                    Some(CollectionOutcome::Success { .. }) | None => {
                        state.failure_class = Some(FailureClass::Internal);
                        state.unavailable_reason = None;
                        schedule_backoff(&mut consecutive_failures, &mut backoff_deadline, config);
                    }
                }
                poll_deadline = Instant::now() + config.poll_interval;
                if persistence.is_some() {
                    let outcome = if state.unavailable_reason == Some(UnavailableReason::NotSignedIn) { OutcomeMetadata::CredentialsMissing }
                        else if state.unavailable_reason == Some(UnavailableReason::NotInstalled) { OutcomeMetadata::CliMissing }
                        else if let Some(class) = state.failure_class { OutcomeMetadata::Failed { class } }
                        else { OutcomeMetadata::Success };
                    let record = ProviderRecord { provider, latest: state.snapshot.clone(), last_outcome: outcome, revision: state.revision };
                    pending_snapshot = Some(record);
                    if let Some(snapshot) = fresh_sample {
                        enqueue_pending_history(&mut pending_history, snapshot);
                    }
                }
                state_tx.send_replace(state.clone());
                if let Some(persistence) = &persistence {
                    retry_pending_persistence(
                        provider,
                        persistence,
                        &mut pending_snapshot,
                        &mut pending_history,
                    );
                }
                if refresh_pending {
                    refresh_pending = false;
                    start_if_idle(&mut in_flight, &collector);
                }
            }
            command = commands.recv() => match command {
                Some(Command::Trigger(_reason)) => debounce_deadline = Some(Instant::now() + config.debounce),
                None => break,
            },
            () = sleep_optional(debounce_deadline), if debounce_deadline.is_some() => {
                debounce_deadline = None;
                refresh_pending = !start_if_idle(&mut in_flight, &collector);
            }
            () = time::sleep_until(poll_deadline) => {
                poll_deadline = Instant::now() + config.poll_interval;
                refresh_pending |= !start_if_idle(&mut in_flight, &collector);
            }
            () = sleep_optional(backoff_deadline), if backoff_deadline.is_some() => {
                backoff_deadline = None;
                refresh_pending |= !start_if_idle(&mut in_flight, &collector);
            }
            () = sleep_optional(ttl_deadline), if ttl_deadline.is_some() => {
                ttl_deadline = None;
                if !state.expired {
                    state.revision = state.revision.saturating_add(1);
                    state.expired = true;
                    state_tx.send_replace(state.clone());
                }
            }
            () = sleep_optional(reset_deadline), if reset_deadline.is_some() => {
                reset_deadline = None;
                state.revision = state.revision.saturating_add(1);
                state.reset_pending = true;
                state_tx.send_replace(state.clone());
                refresh_pending |= !start_if_idle(&mut in_flight, &collector);
            }
        }
    }
}

fn retry_pending_persistence(
    provider: Provider,
    persistence: &RefreshPersistence,
    pending_snapshot: &mut Option<ProviderRecord>,
    pending_history: &mut VecDeque<ProviderUsageSnapshot>,
) {
    if let Some(record) = pending_snapshot.take() {
        if persistence.snapshots.save_provider(record.clone()).is_err() {
            (persistence.diagnostic)(PersistenceDiagnostic {
                provider,
                category: PersistenceCategory::Snapshot,
            });
            *pending_snapshot = Some(record);
        }
    }
    retry_pending_history(provider, persistence, pending_history);
}

pub(crate) fn enqueue_pending_history(
    pending_history: &mut VecDeque<ProviderUsageSnapshot>,
    snapshot: ProviderUsageSnapshot,
) {
    // Match the product repository cap: compact the oldest pending sample first,
    // while preserving FIFO order among the retained 3,000 samples.
    if pending_history.len() == MAX_PENDING_HISTORY {
        pending_history.pop_front();
    }
    pending_history.push_back(snapshot);
}

pub(crate) fn retry_pending_history(
    provider: Provider,
    persistence: &RefreshPersistence,
    pending_history: &mut VecDeque<ProviderUsageSnapshot>,
) {
    const MAX_HISTORY_BATCH: usize = 128;
    let batch: Vec<_> = pending_history
        .iter()
        .take(MAX_HISTORY_BATCH)
        .cloned()
        .collect();
    if batch.is_empty() {
        return;
    }
    if persistence
        .history
        .append_success_batch(&batch, OffsetDateTime::now_utc())
        .is_err()
    {
        (persistence.diagnostic)(PersistenceDiagnostic {
            provider,
            category: PersistenceCategory::History,
        });
        return;
    }
    for _ in 0..batch.len() {
        pending_history.pop_front();
    }
}

fn provider_state_from_record(record: ProviderRecord) -> ProviderState {
    let (failure_class, unavailable_reason) = match record.last_outcome {
        OutcomeMetadata::Success => (None, None),
        OutcomeMetadata::Failed { class } => (Some(class), None),
        OutcomeMetadata::CredentialsMissing => (None, Some(UnavailableReason::NotSignedIn)),
        OutcomeMetadata::CliMissing => (None, Some(UnavailableReason::NotInstalled)),
    };
    let snapshot = record.latest.map(|mut snapshot| {
        snapshot.is_cached = true;
        snapshot
    });
    let expired = snapshot.as_ref().is_some_and(|snapshot| {
        OffsetDateTime::now_utc() - snapshot.captured_at > ::time::Duration::minutes(30)
    });
    ProviderState {
        provider: record.provider,
        snapshot,
        failure_class,
        unavailable_reason,
        expired,
        reset_pending: false,
        revision: record.revision,
    }
}

fn initial_ttl_deadline(state: &ProviderState, ttl: Duration) -> Option<Instant> {
    if state.expired {
        return None;
    }
    let captured = state.snapshot.as_ref()?.captured_at;
    let age = OffsetDateTime::now_utc() - captured;
    let elapsed = Duration::try_from(age).unwrap_or(Duration::ZERO);
    Some(Instant::now() + ttl.saturating_sub(elapsed))
}

fn start_collect(collector: Arc<dyn Collector>) -> JoinHandle<CollectionOutcome> {
    tokio::spawn(async move { collector.collect().await })
}

fn start_if_idle(
    in_flight: &mut Option<JoinHandle<CollectionOutcome>>,
    collector: &Arc<dyn Collector>,
) -> bool {
    if in_flight.is_none() {
        *in_flight = Some(start_collect(Arc::clone(collector)));
        true
    } else {
        false
    }
}

async fn await_collect(
    in_flight: &mut Option<JoinHandle<CollectionOutcome>>,
) -> Option<CollectionOutcome> {
    match in_flight {
        Some(task) => task.await.ok(),
        None => pending().await,
    }
}

async fn sleep_optional(deadline: Option<Instant>) {
    match deadline {
        Some(deadline) => time::sleep_until(deadline).await,
        None => pending().await,
    }
}

fn schedule_backoff(failures: &mut u32, deadline: &mut Option<Instant>, config: SchedulerConfig) {
    *failures = failures.saturating_add(1);
    let multiplier = 1_u32
        .checked_shl(failures.saturating_sub(1).min(31))
        .unwrap_or(u32::MAX);
    let base_delay = config
        .backoff_base
        .saturating_mul(multiplier)
        .min(config.backoff_cap);
    let delay = base_delay
        .saturating_add(deterministic_jitter(*failures, config.jitter))
        .min(config.backoff_cap);
    *deadline = Some(Instant::now() + delay);
}

fn deterministic_jitter(attempt: u32, maximum: Duration) -> Duration {
    if maximum.is_zero() {
        return Duration::ZERO;
    }
    let fraction = (u64::from(attempt)
        .wrapping_mul(1_103_515_245)
        .wrapping_add(12_345)
        % 1_000) as u32;
    maximum.saturating_mul(fraction) / 999
}

fn next_reset_deadline(snapshot: &ProviderUsageSnapshot) -> Option<Instant> {
    let reset = snapshot
        .session
        .as_ref()
        .and_then(|window| window.resets_at)
        .into_iter()
        .chain(snapshot.weekly.as_ref().and_then(|window| window.resets_at))
        .min()?;
    let duration = reset - OffsetDateTime::now_utc();
    if duration.is_negative() {
        return Some(Instant::now());
    }
    Some(Instant::now() + Duration::try_from(duration).ok()?)
}
