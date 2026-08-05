use std::sync::Arc;
use std::time::{Duration, Instant};

use super::channel::{
    channel_for_version, manifest_url, should_check, Channel, AUTOMATIC_CHECK_INTERVAL,
    PANEL_OPEN_CHECK_FLOOR, STARTUP_CHECK_DELAY,
};
use super::feed::{
    FixtureFeed, FixtureScenario, InstallOutcome, InstallProgress, PendingUpdate, ProgressSink,
    ReleaseFeed,
};
use super::service::UpdateService;
use super::state::{truncate_notes, UpdateFailure, UpdateStateDto, UpdateStatus, MAX_NOTES_CHARS};

// --- channel policy -------------------------------------------------------

#[test]
fn a_pre_release_build_follows_the_beta_channel() {
    assert_eq!(channel_for_version("0.1.0-beta.4"), Channel::Beta);
}

#[test]
fn a_release_build_never_follows_the_beta_channel() {
    assert_eq!(channel_for_version("0.1.0"), Channel::Stable);
    assert_eq!(channel_for_version("1.2.3+build.7"), Channel::Stable);
}

#[test]
fn an_unparseable_version_falls_back_to_stable() {
    assert_eq!(channel_for_version("not-a-version"), Channel::Stable);
    assert_eq!(channel_for_version(""), Channel::Stable);
}

#[test]
fn each_channel_resolves_to_its_own_manifest() {
    let stable = manifest_url(Channel::Stable);
    let beta = manifest_url(Channel::Beta);

    assert_ne!(stable, beta);
    for url in [stable, beta] {
        assert!(url.starts_with("https://github.com/"), "{url}");
        assert!(url.ends_with(".json"), "{url}");
    }
}

// --- throttle -------------------------------------------------------------

#[test]
fn the_first_check_is_always_due() {
    assert!(should_check(None, Instant::now(), AUTOMATIC_CHECK_INTERVAL));
    assert!(should_check(None, Instant::now(), PANEL_OPEN_CHECK_FLOOR));
}

#[test]
fn a_recent_check_is_not_repeated() {
    let previous = Instant::now();
    let now = previous + Duration::from_secs(1);

    assert!(!should_check(Some(previous), now, AUTOMATIC_CHECK_INTERVAL));
}

#[test]
fn an_expired_check_is_due_again() {
    let previous = Instant::now();
    let now = previous + Duration::from_secs(2 * 60 * 60);

    assert!(should_check(Some(previous), now, AUTOMATIC_CHECK_INTERVAL));
}

#[test]
fn a_second_panel_reveal_inside_the_floor_does_not_recheck() {
    let previous = Instant::now();
    let now = previous + Duration::from_secs(5 * 60);

    assert!(!should_check(Some(previous), now, PANEL_OPEN_CHECK_FLOOR));
}

#[test]
fn a_panel_reveal_after_the_floor_rechecks() {
    let previous = Instant::now();
    let now = previous + Duration::from_secs(20 * 60);

    assert!(should_check(Some(previous), now, PANEL_OPEN_CHECK_FLOOR));
}

/// Guards an inverted edit: the reveal must be the responsive trigger and the
/// sweep the slow one, never the other way round.
#[test]
fn the_panel_floor_is_shorter_than_the_background_sweep() {
    assert!(PANEL_OPEN_CHECK_FLOOR < AUTOMATIC_CHECK_INTERVAL);
    assert!(STARTUP_CHECK_DELAY < PANEL_OPEN_CHECK_FLOOR);
}

// --- notes ----------------------------------------------------------------

#[test]
fn notes_are_truncated_on_a_char_boundary() {
    let body = "가".repeat(10_000);

    let truncated = truncate_notes(&body).expect("a non-empty body yields notes");

    assert!(truncated.chars().count() <= MAX_NOTES_CHARS + 1);
    assert!(truncated.ends_with('…'));
}

#[test]
fn short_notes_are_returned_verbatim_and_blank_notes_are_dropped() {
    assert_eq!(
        truncate_notes("  Fixes a crash.  "),
        Some("Fixes a crash.".to_owned())
    );
    assert_eq!(truncate_notes("   \n "), None);
}

// --- DTO privacy ----------------------------------------------------------

fn every_status() -> Vec<UpdateStatus> {
    vec![
        UpdateStatus::Idle,
        UpdateStatus::Checking,
        UpdateStatus::UpToDate,
        UpdateStatus::Available {
            version: "0.1.0-beta.5".to_owned(),
            notes: Some("notes".to_owned()),
        },
        UpdateStatus::Downloading {
            received: 1,
            total: Some(2),
        },
        UpdateStatus::Installing {
            version: "0.1.0-beta.5".to_owned(),
        },
        UpdateStatus::Failed {
            reason: UpdateFailure::VerificationFailed,
        },
    ]
}

#[test]
fn the_update_dto_never_serialises_a_download_url() {
    for status in every_status() {
        let json = serde_json::to_string(&UpdateStateDto {
            current_version: "0.1.0-beta.4".to_owned(),
            status,
        })
        .expect("the update DTO serialises");

        for forbidden in ["url", "path", "signature", "pubkey", "token"] {
            assert!(!json.contains(forbidden), "{forbidden} leaked into {json}");
        }
    }
}

#[test]
fn an_available_update_serialises_the_keys_the_renderer_narrows_on() {
    let value = serde_json::to_value(UpdateStatus::Available {
        version: "0.1.0-beta.5".to_owned(),
        notes: None,
    })
    .expect("the status serialises");
    let object = value.as_object().expect("a tagged enum is an object");

    let mut keys: Vec<_> = object.keys().map(String::as_str).collect();
    keys.sort_unstable();
    assert_eq!(keys, ["notes", "status", "version"]);
    assert_eq!(object["status"], "available");
}

#[test]
fn the_dto_reports_the_running_version_in_camel_case() {
    let value = serde_json::to_value(UpdateStateDto {
        current_version: "0.1.0-beta.4".to_owned(),
        status: UpdateStatus::Idle,
    })
    .expect("the update DTO serialises");

    assert_eq!(value["currentVersion"], "0.1.0-beta.4");
    assert_eq!(value["status"]["status"], "idle");
}

#[test]
fn every_failure_reason_serialises_as_snake_case() {
    let reasons = [
        (UpdateFailure::Offline, "offline"),
        (UpdateFailure::RateLimited, "rate_limited"),
        (UpdateFailure::MetadataInvalid, "metadata_invalid"),
        (UpdateFailure::ArtifactMissing, "artifact_missing"),
        (UpdateFailure::DownloadFailed, "download_failed"),
        (UpdateFailure::VerificationFailed, "verification_failed"),
        (UpdateFailure::InstallFailed, "install_failed"),
    ];

    for (reason, wire) in reasons {
        assert_eq!(
            serde_json::to_value(reason).expect("the reason serialises"),
            serde_json::Value::String(wire.to_owned())
        );
    }
}

#[test]
fn only_the_in_flight_install_states_block_a_background_check() {
    for status in every_status() {
        let expected = matches!(
            status,
            UpdateStatus::Downloading { .. } | UpdateStatus::Installing { .. }
        );
        assert_eq!(status.is_installing(), expected, "{status:?}");
    }
}

// --- fixture feed ---------------------------------------------------------

#[test]
fn the_fixture_scenario_falls_back_to_none_for_unknown_values() {
    assert_eq!(FixtureScenario::parse(None), FixtureScenario::None);
    assert_eq!(FixtureScenario::parse(Some("typo")), FixtureScenario::None);
    assert_eq!(
        FixtureScenario::parse(Some("available")),
        FixtureScenario::Available
    );
    assert_eq!(
        FixtureScenario::parse(Some("failed")),
        FixtureScenario::Failed
    );
}

#[tokio::test]
async fn the_fixture_feed_reports_the_selected_scenario_and_counts_probes() {
    let feed = FixtureFeed::new(FixtureScenario::Available);

    let found = feed.check().await.expect("the available fixture succeeds");

    assert_eq!(
        found,
        Some(PendingUpdate {
            version: "9.9.9".to_owned(),
            notes: Some("Fixture release notes.".to_owned()),
        })
    );
    assert_eq!(feed.probes(), 1);
}

/// The install must announce `Installing` before it finishes, or the last state
/// the renderer sees on Windows — where the plugin never returns — is
/// `Downloading`.
#[tokio::test]
async fn the_fixture_feed_installs_without_exiting_the_process() {
    let feed = FixtureFeed::new(FixtureScenario::Available);
    let seen = Arc::new(std::sync::Mutex::new(Vec::new()));
    let recorder = seen.clone();
    let sink: ProgressSink = Arc::new(move |progress| {
        recorder
            .lock()
            .expect("recorder mutex poisoned")
            .push(progress);
    });

    let outcome = feed
        .install(
            PendingUpdate {
                version: "9.9.9".to_owned(),
                notes: None,
            },
            sink,
        )
        .await
        .expect("the fixture install succeeds");

    assert_eq!(outcome, InstallOutcome::Completed);
    let progress = seen.lock().expect("recorder mutex poisoned").clone();
    assert_eq!(progress.last(), Some(&InstallProgress::Installing));
    assert!(progress
        .iter()
        .any(|value| matches!(value, InstallProgress::Downloading { .. })));
}

// --- service --------------------------------------------------------------

#[tokio::test]
async fn a_check_publishes_up_to_date_when_no_release_is_newer() {
    let service = UpdateService::new(
        Arc::new(FixtureFeed::new(FixtureScenario::None)),
        "0.1.0-beta.4".to_owned(),
    );

    service.check_now().await;

    assert_eq!(service.snapshot().status, UpdateStatus::UpToDate);
    assert_eq!(service.snapshot().current_version, "0.1.0-beta.4");
}

#[tokio::test]
async fn a_check_publishes_the_available_release() {
    let service = UpdateService::new(
        Arc::new(FixtureFeed::new(FixtureScenario::Available)),
        "0.1.0-beta.4".to_owned(),
    );

    service.check_now().await;

    assert_eq!(
        service.snapshot().status,
        UpdateStatus::Available {
            version: "9.9.9".to_owned(),
            notes: Some("Fixture release notes.".to_owned()),
        }
    );
}

/// The `Drop` guard, not a happy-path reset, is what keeps the checker alive
/// after a failure. A second check must still reach the feed.
#[tokio::test]
async fn a_failed_check_releases_the_in_flight_guard() {
    let feed = Arc::new(FixtureFeed::new(FixtureScenario::Failed));
    let service = UpdateService::new(feed.clone(), "0.1.0-beta.4".to_owned());

    service.check_now().await;
    service.check_now().await;

    assert_eq!(feed.probes(), 2);
    assert_eq!(
        service.snapshot().status,
        UpdateStatus::Failed {
            reason: UpdateFailure::Offline,
        }
    );
}

#[tokio::test]
async fn a_second_panel_reveal_inside_the_floor_spends_no_check() {
    let feed = Arc::new(FixtureFeed::new(FixtureScenario::Available));
    let service = UpdateService::new(feed.clone(), "0.1.0-beta.4".to_owned());

    // The first reveal has no prior check to throttle against, so it runs.
    service.check_now().await;
    assert_eq!(feed.probes(), 1);

    // The second reveal falls inside PANEL_OPEN_CHECK_FLOOR and is dropped
    // without spawning anything.
    service.check_on_panel_reveal();
    tokio::task::yield_now().await;

    assert_eq!(feed.probes(), 1);
}

/// A feed that yields once before answering, so the `Checking` state is
/// actually observable. The fixture feed resolves without ever suspending, and
/// a `watch` channel only keeps the latest value — so with it the subscriber
/// legitimately sees only the terminal state.
struct PausingFeed;

impl ReleaseFeed for PausingFeed {
    fn check(&self) -> super::feed::FeedFuture<'_, Option<PendingUpdate>> {
        Box::pin(async move {
            tokio::task::yield_now().await;
            Ok(Some(PendingUpdate {
                version: "9.9.9".to_owned(),
                notes: None,
            }))
        })
    }

    fn install(
        &self,
        _pending: PendingUpdate,
        _on_progress: ProgressSink,
    ) -> super::feed::FeedFuture<'_, InstallOutcome> {
        Box::pin(async move { Ok(InstallOutcome::Completed) })
    }
}

#[tokio::test]
async fn a_check_publishes_checking_before_its_result() {
    let service = UpdateService::new(Arc::new(PausingFeed), "0.1.0-beta.4".to_owned());
    let mut states = service.subscribe();

    let observer = tokio::spawn(async move {
        let mut seen = Vec::new();
        while states.changed().await.is_ok() {
            let status = states.borrow_and_update().clone();
            let last = matches!(status, UpdateStatus::Available { .. });
            seen.push(status);
            if last {
                break;
            }
        }
        seen
    });

    service.check_now().await;

    let seen = observer.await.expect("the observer task completes");
    assert_eq!(seen.first(), Some(&UpdateStatus::Checking));
    assert!(matches!(seen.last(), Some(UpdateStatus::Available { .. })));
}
