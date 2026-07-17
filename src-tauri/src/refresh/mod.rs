mod actor;
mod service;

#[cfg(feature = "tauri-ipc")]
pub mod ipc;

#[cfg(test)]
pub(crate) use actor::{enqueue_pending_history, retry_pending_history, MAX_PENDING_HISTORY};
pub use actor::{
    HistoryPersistence, PersistenceCategory, PersistenceDiagnostic, ProviderState, RefreshHandle,
    RefreshPersistence, RefreshReason, SchedulerConfig, SnapshotPersistence,
};
pub use service::{ProviderStates, RefreshService};

#[cfg(test)]
mod tests;
