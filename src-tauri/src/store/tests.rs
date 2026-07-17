use std::{
    fs,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    thread,
};
use time::OffsetDateTime;

use super::*;
use crate::domain::{FailureClass, Provider, ProviderUsageSnapshot, Source, UsageWindow};

static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

struct TempDir(PathBuf);

impl TempDir {
    fn new() -> std::io::Result<Self> {
        let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "cachebite-store-test-{}-{sequence}",
            std::process::id()
        ));
        fs::create_dir(&path)?;
        Ok(Self(path))
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn snapshot(provider: Provider, revision: u64) -> ProviderUsageSnapshot {
    ProviderUsageSnapshot {
        provider,
        plan_type: Some("pro".into()),
        session: Some(UsageWindow::new(42.0, 300, None).expect("valid window")),
        weekly: None,
        captured_at: OffsetDateTime::UNIX_EPOCH,
        source: match provider {
            Provider::Claude => Source::OauthApi,
            Provider::Codex => Source::CliRpc,
        },
        is_cached: false,
        revision,
    }
}

#[test]
fn settings_round_trip_is_versioned_and_atomic() {
    let dir = TempDir::new().expect("temp dir");
    let repository = SettingsRepository::new(dir.path());
    let settings = Settings {
        primary_provider: Provider::Codex,
        selected_pet_id: "generated-idle".into(),
        bubble_enabled: false,
        start_at_login: true,
        logical_position: LogicalPosition { x: 12.5, y: -4.0 },
        ..Settings::default()
    };

    repository.save(&settings).expect("save settings");

    assert_eq!(repository.load().expect("load settings"), settings);
    assert_eq!(fs::read_dir(dir.path()).expect("read dir").count(), 1);
}

#[test]
fn settings_can_replace_an_existing_file_atomically() {
    let dir = TempDir::new().expect("temp dir");
    let repository = SettingsRepository::new(dir.path());
    repository.save(&Settings::default()).expect("first save");
    let updated = Settings {
        bubble_enabled: false,
        ..Settings::default()
    };
    repository.save(&updated).expect("second save");
    assert_eq!(repository.load().expect("load replacement"), updated);
    assert_eq!(fs::read_dir(dir.path()).expect("read dir").count(), 1);
}

#[test]
fn history_batch_appends_ordered_samples_in_one_repository_operation() {
    let dir = TempDir::new().expect("temp dir");
    let repository = HistoryRepository::new(dir.path());
    let first = snapshot(Provider::Claude, 1);
    let mut second = snapshot(Provider::Claude, 2);
    second.captured_at = OffsetDateTime::UNIX_EPOCH + time::Duration::seconds(1);

    assert_eq!(
        repository
            .append_success_batch(
                &[first, second],
                OffsetDateTime::UNIX_EPOCH + time::Duration::seconds(2)
            )
            .expect("append batch"),
        2
    );
    let history = repository
        .load_at(OffsetDateTime::UNIX_EPOCH + time::Duration::seconds(2))
        .expect("load history");
    assert_eq!(history.claude.samples.len(), 2);
}

#[cfg(unix)]
#[test]
fn persisted_files_are_owner_only() {
    use std::os::unix::fs::PermissionsExt;
    let dir = TempDir::new().expect("temp dir");
    SettingsRepository::new(dir.path())
        .save(&Settings::default())
        .expect("save");
    let mode = fs::metadata(dir.path().join("settings.json"))
        .expect("metadata")
        .permissions()
        .mode();
    assert_eq!(mode & 0o777, 0o600);
}

#[test]
fn legacy_settings_are_migrated_and_rewritten() {
    let dir = TempDir::new().expect("temp dir");
    fs::write(
        dir.path().join("settings.json"),
        r#"{"primary_provider":"codex","pet_id":"old-pet","show_bubbles":false,"start_at_login":true,"position":{"x":3.0,"y":4.0}}"#,
    )
    .expect("write legacy settings");

    let loaded = SettingsRepository::new(dir.path()).load().expect("migrate");

    assert_eq!(loaded.schema_version, 3);
    assert_eq!(loaded.selected_pet_id, "old-pet");
    assert!(!loaded.bubble_enabled);
    let rewritten = fs::read_to_string(dir.path().join("settings.json")).expect("rewritten");
    assert!(rewritten.contains("\"schema_version\": 3"));
}

#[test]
fn version_one_settings_migrate_with_notifications_off() {
    let dir = TempDir::new().expect("temp dir");
    fs::write(
        dir.path().join("settings.json"),
        r#"{"schema_version":1,"primary_provider":"claude","selected_pet_id":"idle","bubble_enabled":true,"start_at_login":false,"logical_position":{"x":0.0,"y":0.0}}"#,
    )
    .expect("write v1 settings");
    let loaded = SettingsRepository::new(dir.path())
        .load()
        .expect("migrate v1");
    assert_eq!(loaded.schema_version, 3);
    assert!(!loaded.notification_enabled);
}

#[test]
fn version_two_settings_migrate_with_secondary_notifications_off() {
    let dir = TempDir::new().expect("temp dir");
    fs::write(
        dir.path().join("settings.json"),
        r#"{"schema_version":2,"primary_provider":"claude","selected_pet_id":"idle","bubble_enabled":true,"start_at_login":false,"notification_enabled":true,"logical_position":{"x":0.0,"y":0.0}}"#,
    )
    .expect("write v2 settings");
    let loaded = SettingsRepository::new(dir.path())
        .load()
        .expect("migrate v2");
    assert_eq!(loaded.schema_version, 3);
    assert!(loaded.notification_enabled);
    assert!(!loaded.secondary_notification_enabled);
}

#[test]
fn corrupt_settings_are_quarantined_and_defaults_returned() {
    let dir = TempDir::new().expect("temp dir");
    fs::write(dir.path().join("settings.json"), b"not-json").expect("write corrupt file");

    let loaded = SettingsRepository::new(dir.path()).load().expect("recover");

    assert_eq!(loaded, Settings::default());
    assert!(!dir.path().join("settings.json").exists());
    assert!(fs::read_dir(dir.path())
        .expect("read dir")
        .filter_map(Result::ok)
        .any(|entry| entry
            .file_name()
            .to_string_lossy()
            .starts_with("settings.json.corrupt-")));
}

#[test]
fn repeated_quarantines_never_overwrite_an_existing_quarantine() {
    let dir = TempDir::new().expect("temp dir");
    let repository = SettingsRepository::new(dir.path());
    fs::write(dir.path().join("settings.json"), "bad-one").expect("first corrupt");
    repository.load().expect("first quarantine");
    fs::write(dir.path().join("settings.json"), "bad-two").expect("second corrupt");
    repository.load().expect("second quarantine");
    let quarantines: Vec<_> = fs::read_dir(dir.path())
        .expect("read dir")
        .filter_map(Result::ok)
        .collect();
    assert_eq!(quarantines.len(), 2);
    let contents: Vec<_> = quarantines
        .into_iter()
        .map(|entry| fs::read_to_string(entry.path()).expect("read quarantine"))
        .collect();
    assert!(contents.contains(&"bad-one".to_string()));
    assert!(contents.contains(&"bad-two".to_string()));
}

#[test]
fn concurrent_provider_updates_merge_without_clobbering() {
    let dir = TempDir::new().expect("temp dir");
    let repository = Arc::new(SnapshotRepository::new(dir.path()));
    let claude_repo = Arc::clone(&repository);
    let codex_repo = Arc::clone(&repository);

    let claude = thread::spawn(move || {
        claude_repo
            .save_provider(ProviderRecord::success(snapshot(Provider::Claude, 7)))
            .expect("save claude")
    });
    let codex = thread::spawn(move || {
        codex_repo
            .save_provider(ProviderRecord::failed(
                Provider::Codex,
                8,
                FailureClass::Network,
            ))
            .expect("save codex")
    });
    claude.join().expect("claude thread");
    codex.join().expect("codex thread");

    let snapshots = repository.load().expect("load snapshots");
    assert_eq!(snapshots.claude.expect("claude").revision, 7);
    assert_eq!(snapshots.codex.expect("codex").revision, 8);
}

#[test]
fn distinct_repository_instances_share_a_path_lock() {
    let dir = TempDir::new().expect("temp dir");
    let claude_repo = SnapshotRepository::new(dir.path());
    let codex_repo = SnapshotRepository::new(dir.path());
    let claude = thread::spawn(move || {
        claude_repo.save_provider(ProviderRecord::success(snapshot(Provider::Claude, 1)))
    });
    let codex = thread::spawn(move || {
        codex_repo.save_provider(ProviderRecord::success(snapshot(Provider::Codex, 1)))
    });
    claude.join().expect("claude thread").expect("save claude");
    codex.join().expect("codex thread").expect("save codex");
    let loaded = SnapshotRepository::new(dir.path()).load().expect("load");
    assert!(loaded.claude.is_some());
    assert!(loaded.codex.is_some());
}

#[test]
fn stale_provider_revision_is_rejected() {
    let dir = TempDir::new().expect("temp dir");
    let repository = SnapshotRepository::new(dir.path());
    repository
        .save_provider(ProviderRecord::success(snapshot(Provider::Claude, 5)))
        .expect("save current");
    let result = repository.save_provider(ProviderRecord::success(snapshot(Provider::Claude, 4)));
    assert!(result.is_err());
    assert_eq!(
        repository
            .load()
            .expect("load")
            .claude
            .expect("claude")
            .revision,
        5
    );
}

#[test]
fn failure_retains_last_successful_snapshot() {
    let dir = TempDir::new().expect("temp dir");
    let repository = SnapshotRepository::new(dir.path());
    repository
        .save_provider(ProviderRecord::success(snapshot(Provider::Claude, 5)))
        .expect("save success");
    repository
        .save_provider(ProviderRecord::failed(
            Provider::Claude,
            6,
            FailureClass::Network,
        ))
        .expect("save failure");
    let record = repository.load().expect("load").claude.expect("claude");
    assert_eq!(record.revision, 6);
    assert_eq!(record.latest.expect("last good snapshot").revision, 5);
    assert!(matches!(
        record.last_outcome,
        OutcomeMetadata::Failed {
            class: FailureClass::Network
        }
    ));
}

#[test]
fn legacy_snapshot_store_is_migrated_without_cross_provider_loss() {
    let dir = TempDir::new().expect("temp dir");
    let legacy = serde_json::json!({
        "claude": ProviderRecord::success(snapshot(Provider::Claude, 2)),
        "codex": ProviderRecord::success(snapshot(Provider::Codex, 3))
    });
    fs::write(
        dir.path().join("snapshots.json"),
        serde_json::to_vec(&legacy).expect("serialize legacy"),
    )
    .expect("write legacy snapshots");

    let loaded = SnapshotRepository::new(dir.path())
        .load()
        .expect("migrate snapshots");

    assert_eq!(loaded.schema_version, 1);
    assert_eq!(loaded.claude.expect("claude").revision, 2);
    assert_eq!(loaded.codex.expect("codex").revision, 3);
    let rewritten = fs::read_to_string(dir.path().join("snapshots.json")).expect("rewritten");
    assert!(rewritten.contains("\"schema_version\": 1"));
}

#[test]
fn serialized_store_excludes_secrets_bodies_accounts_and_paths() {
    let dir = TempDir::new().expect("temp dir");
    let repository = SnapshotRepository::new(dir.path());
    repository
        .save_provider(ProviderRecord::success(snapshot(Provider::Claude, 1)))
        .expect("save snapshot");

    let serialized = fs::read_to_string(dir.path().join("snapshots.json")).expect("read store");
    for forbidden in [
        "SECRET_MARKER",
        "access_token",
        "refresh_token",
        "authorization",
        "raw_body",
        "account_id",
        "/home/",
        "C:\\\\Users\\\\",
    ] {
        assert!(!serialized
            .to_ascii_lowercase()
            .contains(&forbidden.to_ascii_lowercase()));
    }
}

#[test]
fn unknown_forbidden_fields_are_quarantined_instead_of_silently_loaded() {
    let dir = TempDir::new().expect("temp dir");
    let mut value = serde_json::to_value(SnapshotStore {
        schema_version: 1,
        claude: Some(ProviderRecord::success(snapshot(Provider::Claude, 1))),
        codex: None,
    })
    .expect("serialize");
    value["claude"]["latest"]["raw_body"] = serde_json::json!("SECRET_MARKER");
    fs::write(
        dir.path().join("snapshots.json"),
        serde_json::to_vec(&value).expect("serialize poisoned store"),
    )
    .expect("write poisoned store");
    let loaded = SnapshotRepository::new(dir.path()).load().expect("recover");
    assert_eq!(loaded, SnapshotStore::default());
    assert!(!dir.path().join("snapshots.json").exists());
}

#[cfg(unix)]
#[test]
fn permission_failure_is_reported_without_destroying_existing_settings() {
    use std::os::unix::fs::PermissionsExt;

    let dir = TempDir::new().expect("temp dir");
    let repository = SettingsRepository::new(dir.path());
    repository.save(&Settings::default()).expect("initial save");
    let original = fs::read(dir.path().join("settings.json")).expect("read original");
    fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o500)).expect("make read only");

    let result = repository.save(&Settings {
        bubble_enabled: false,
        ..Settings::default()
    });

    fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o700))
        .expect("restore permissions");
    if unsafe { libc::geteuid() } != 0 {
        assert!(result.is_err());
        assert_eq!(
            fs::read(dir.path().join("settings.json")).expect("read current"),
            original
        );
    }
}

#[test]
fn history_dedupes_out_of_order_and_marks_reset_gaps_independently() {
    let dir = TempDir::new().expect("temp dir");
    let repository = HistoryRepository::new(dir.path());
    let at = OffsetDateTime::UNIX_EPOCH;
    let mut first = snapshot(Provider::Claude, 1);
    first.captured_at = at;
    first.session.as_mut().unwrap().resets_at = Some(at + time::Duration::hours(5));
    repository.append_success(&first, at).expect("append first");
    repository.append_success(&first, at).expect("dedupe equal");
    let mut older = first.clone();
    older.captured_at = at - time::Duration::minutes(1);
    repository.append_success(&older, at).expect("ignore older");
    let mut reset = first.clone();
    reset.revision = 2;
    reset.captured_at = at + time::Duration::minutes(15);
    reset.session.as_mut().unwrap().resets_at = Some(at + time::Duration::hours(10));
    reset.weekly = Some(UsageWindow::new(20.0, 10_080, None).unwrap());
    repository
        .append_success(&reset, reset.captured_at)
        .expect("append reset");

    let store = repository.load_at(reset.captured_at).expect("load");
    let samples = &store.claude.samples;
    assert_eq!(samples.len(), 2);
    assert!(!samples[0].session.as_ref().unwrap().starts_new_segment);
    assert!(samples[1].session.as_ref().unwrap().starts_new_segment);
    assert!(!samples[1].weekly.as_ref().unwrap().starts_new_segment);
}

#[test]
fn history_keeps_only_fresh_successes_and_applies_retention_and_cap() {
    let dir = TempDir::new().expect("temp dir");
    let repository = HistoryRepository::new(dir.path());
    let now = OffsetDateTime::UNIX_EPOCH + time::Duration::days(40);
    let mut cached = snapshot(Provider::Codex, 1);
    cached.is_cached = true;
    assert!(!repository
        .append_success(&cached, now)
        .expect("ignore cached"));
    let samples = (0..3_005_i64)
        .map(|index| HistorySample {
            captured_at: now - time::Duration::days(29) + time::Duration::minutes(index),
            session: Some(HistoryPoint {
                used_percent: 42.0,
                resets_at: None,
                starts_new_segment: false,
            }),
            weekly: None,
        })
        .collect();
    let seeded = HistoryStore {
        schema_version: 1,
        claude: HistoryProvider::default(),
        codex: HistoryProvider { samples },
    };
    fs::write(
        dir.path().join("history.json"),
        serde_json::to_vec(&seeded).unwrap(),
    )
    .unwrap();
    let store = repository.load_at(now).expect("load");
    assert_eq!(store.codex.samples.len(), 3_000);
    assert!(store
        .codex
        .samples
        .iter()
        .all(|sample| sample.captured_at >= now - time::Duration::days(30)));
}

#[test]
fn legacy_history_migrates_and_prunes_expired_samples() {
    let dir = TempDir::new().expect("temp dir");
    let now = OffsetDateTime::UNIX_EPOCH + time::Duration::days(40);
    let legacy = serde_json::json!({
        "claude": [{"captured_at":"1970-01-01T00:00:00Z","session":{"used_percent":1.0,"resets_at":null},"weekly":null},
                   {"captured_at": now.format(&time::format_description::well_known::Rfc3339).unwrap(),"session":{"used_percent":2.0,"resets_at":null},"weekly":null}],
        "codex": []
    });
    fs::write(
        dir.path().join("history.json"),
        serde_json::to_vec(&legacy).unwrap(),
    )
    .unwrap();
    let store = HistoryRepository::new(dir.path())
        .load_at(now)
        .expect("migrate");
    assert_eq!(store.schema_version, 1);
    assert_eq!(store.claude.samples.len(), 1);
    assert!(fs::read_to_string(dir.path().join("history.json"))
        .unwrap()
        .contains("schema_version"));
}

#[test]
fn current_history_load_rewrites_pruned_samples_to_disk() {
    let dir = TempDir::new().expect("temp dir");
    let now = OffsetDateTime::UNIX_EPOCH + time::Duration::days(40);
    let store = serde_json::json!({"schema_version":1,"claude":{"samples":[{"captured_at":"1970-01-01T00:00:00Z","session":{"used_percent":1.0,"resets_at":null,"starts_new_segment":false},"weekly":null}]},"codex":{"samples":[]}});
    fs::write(
        dir.path().join("history.json"),
        serde_json::to_vec(&store).unwrap(),
    )
    .unwrap();
    assert!(HistoryRepository::new(dir.path())
        .load_at(now)
        .unwrap()
        .claude
        .samples
        .is_empty());
    let disk = fs::read_to_string(dir.path().join("history.json")).unwrap();
    assert!(!disk.contains("1970-01-01"));
}

#[test]
fn selected_pet_id_rejects_traversal_uppercase_and_excessive_length() {
    let dir = TempDir::new().expect("temp dir");
    let repository = SettingsRepository::new(dir.path());
    for selected_pet_id in [
        "../secret".to_owned(),
        "Uppercase".to_owned(),
        "a".repeat(65),
        "a_underscore".to_owned(),
    ] {
        assert!(repository
            .save(&Settings {
                selected_pet_id,
                ..Settings::default()
            })
            .is_err());
    }
    repository
        .save(&Settings {
            selected_pet_id: "pet-2".to_owned(),
            ..Settings::default()
        })
        .expect("valid id");
}

#[test]
fn default_settings_select_the_bundled_claude_pet() {
    assert_eq!(Settings::default().selected_pet_id, "cat");
}

#[test]
fn schema_three_idle_pet_is_reconciled_to_the_primary_provider() {
    let dir = TempDir::new().expect("temp dir");
    fs::write(
        dir.path().join("settings.json"),
        r#"{"schema_version":3,"primary_provider":"codex","selected_pet_id":"idle","bubble_enabled":true,"start_at_login":false,"notification_enabled":false,"secondary_notification_enabled":false,"logical_position":{"x":0.0,"y":0.0}}"#,
    )
    .unwrap();

    let loaded = SettingsRepository::new(dir.path()).load().unwrap();

    assert_eq!(loaded.selected_pet_id, "corgi");
    assert!(fs::read_to_string(dir.path().join("settings.json"))
        .unwrap()
        .contains("\"selected_pet_id\": \"corgi\""));
}

#[test]
fn save_position_is_atomic_validated_and_preserves_other_settings() {
    let dir = TempDir::new().expect("temp dir");
    let repository = SettingsRepository::new(dir.path());
    repository
        .save(&Settings {
            primary_provider: Provider::Codex,
            bubble_enabled: false,
            ..Settings::default()
        })
        .unwrap();
    repository
        .save_position(LogicalPosition { x: -12.5, y: 20.0 })
        .unwrap();
    let loaded = repository.load().unwrap();
    assert_eq!(loaded.primary_provider, Provider::Codex);
    assert!(!loaded.bubble_enabled);
    assert_eq!(
        loaded.logical_position,
        LogicalPosition { x: -12.5, y: 20.0 }
    );
    assert!(repository
        .save_position(LogicalPosition {
            x: f64::NAN,
            y: 0.0
        })
        .is_err());
}

#[test]
fn pet_package_loader_returns_safe_asset_url_and_rejects_escaping_assets() {
    let dir = TempDir::new().expect("temp dir");
    let root = dir.path().join("pets/pet-2");
    fs::create_dir_all(root.join("frames")).unwrap();
    fs::write(root.join("frames/idle.svg"), "<svg/>").unwrap();
    let manifest = serde_json::json!({"id":"pet-2","displayName":"Pet","defaultSize":{"width":128,"height":128},"animations":{"idle":{"type":"image","source":"frames/idle.svg"}},"states":{}});
    fs::write(
        root.join("manifest.json"),
        serde_json::to_vec(&manifest).unwrap(),
    )
    .unwrap();
    let package = PetPackageRepository::new(dir.path())
        .load("pet-2")
        .expect("load package");
    assert!(
        Path::new(package.asset_base_url.trim_end_matches(['/', '\\']))
            .ends_with(Path::new("pets").join("pet-2"))
    );
    let poisoned = serde_json::json!({"id":"pet-2","displayName":"Pet","defaultSize":{"width":128,"height":128},"animations":{"idle":{"type":"image","source":"../secret"}},"states":{}});
    fs::write(
        root.join("manifest.json"),
        serde_json::to_vec(&poisoned).unwrap(),
    )
    .unwrap();
    assert!(PetPackageRepository::new(dir.path()).load("pet-2").is_err());
}
