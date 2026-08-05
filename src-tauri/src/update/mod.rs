//! In-app updates from signed GitHub Release manifests.
//!
//! The renderer never talks to `tauri-plugin-updater`; it only reads the state
//! this module publishes and asks it to act. Signature verification is owned by
//! the plugin and is fail-closed — there is no bypass.
//!
//! Layout mirrors `refresh/`: pure policy in [`channel`] and [`state`], the
//! side-effecting source behind the [`feed::ReleaseFeed`] trait, the actor in
//! [`service`], and the Tauri command surface in [`ipc`].

pub mod channel;
pub mod feed;
pub mod ipc;
pub mod service;
pub mod state;

#[cfg(test)]
mod tests;

pub use channel::{
    channel_for_version, manifest_url, should_check, Channel, AUTOMATIC_CHECK_INTERVAL,
    PANEL_OPEN_CHECK_FLOOR, STARTUP_CHECK_DELAY,
};
pub use feed::{
    FixtureFeed, FixtureScenario, InstallOutcome, InstallProgress, PendingUpdate, ReleaseFeed,
    TauriUpdaterFeed,
};
pub use service::UpdateService;
pub use state::{truncate_notes, UpdateFailure, UpdateStateDto, UpdateStatus, MAX_NOTES_CHARS};
