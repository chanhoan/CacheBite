use std::time::{Duration, Instant};

/// Which manifest a build is allowed to read.
///
/// Derived from the running version rather than stored: a persisted channel
/// could record a beta opt-in that no migration would ever undo, which is the
/// same failure mode that removed the persisted hide/show hotkey in schema v5.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Channel {
    Stable,
    Beta,
}

const STABLE_MANIFEST: &str =
    "https://github.com/chanhoan/CacheBite/releases/download/updater/stable.json";
const BETA_MANIFEST: &str =
    "https://github.com/chanhoan/CacheBite/releases/download/updater/beta.json";

/// A pre-release build follows beta; a release build never sees one.
///
/// An unparseable version is treated as stable — the conservative direction.
/// Getting this backwards would silently move every stable user onto betas.
pub fn channel_for_version(version: &str) -> Channel {
    match semver::Version::parse(version) {
        Ok(parsed) if !parsed.pre.is_empty() => Channel::Beta,
        _ => Channel::Stable,
    }
}

pub fn manifest_url(channel: Channel) -> &'static str {
    match channel {
        Channel::Stable => STABLE_MANIFEST,
        Channel::Beta => BETA_MANIFEST,
    }
}

/// How long after launch the first automatic check waits. Long enough that it
/// never competes with the first provider collection for startup bandwidth.
pub const STARTUP_CHECK_DELAY: Duration = Duration::from_secs(30);

/// Background sweep. Deliberately slow: the notice is only ever *seen* when
/// the panel is open, and [`PANEL_OPEN_CHECK_FLOOR`] already covers that moment.
/// This exists so a session that leaves the panel open all day still notices,
/// and so the state is warm before the panel is revealed — a check that only
/// started on reveal would pop the banner in a moment later and resize the
/// panel under the user's cursor.
pub const AUTOMATIC_CHECK_INTERVAL: Duration = Duration::from_secs(60 * 60);

/// Minimum gap between checks triggered by revealing the panel. Matches the
/// provider poll interval (`SchedulerConfig::default().poll_interval`), so
/// opening the panel never costs more network traffic than the usage
/// collection the panel exists to show.
pub const PANEL_OPEN_CHECK_FLOOR: Duration = Duration::from_secs(15 * 60);

/// Whether an automatic check is due. A manual check from Settings does not
/// consult this — the user asking is the signal.
///
/// One function serves all three cadences; only `interval` differs.
pub fn should_check(last_checked: Option<Instant>, now: Instant, interval: Duration) -> bool {
    match last_checked {
        None => true,
        Some(previous) => now.saturating_duration_since(previous) >= interval,
    }
}
