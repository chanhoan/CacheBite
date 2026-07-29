use std::{
    fs, io,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use serde::{Deserialize, Serialize};

use super::{path_lock, quarantine, write_json_atomically};
use crate::domain::{FailureClass, Provider, ProviderUsageSnapshot};

const SNAPSHOT_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum OutcomeMetadata {
    Success,
    Failed { class: FailureClass },
    CredentialsMissing,
    CliMissing,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderRecord {
    pub provider: Provider,
    pub latest: Option<ProviderUsageSnapshot>,
    pub last_outcome: OutcomeMetadata,
    pub revision: u64,
}

impl ProviderRecord {
    pub fn success(snapshot: ProviderUsageSnapshot) -> Self {
        Self {
            provider: snapshot.provider,
            revision: snapshot.revision,
            latest: Some(snapshot),
            last_outcome: OutcomeMetadata::Success,
        }
    }

    pub fn failed(provider: Provider, revision: u64, class: FailureClass) -> Self {
        Self {
            provider,
            latest: None,
            last_outcome: OutcomeMetadata::Failed { class },
            revision,
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SnapshotStore {
    pub schema_version: u32,
    pub claude: Option<ProviderRecord>,
    pub codex: Option<ProviderRecord>,
}

impl Default for SnapshotStore {
    fn default() -> Self {
        Self {
            schema_version: SNAPSHOT_SCHEMA_VERSION,
            claude: None,
            codex: None,
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LegacySnapshotStore {
    claude: Option<ProviderRecord>,
    codex: Option<ProviderRecord>,
}

pub struct SnapshotRepository {
    path: PathBuf,
    lock: Arc<Mutex<()>>,
}

impl SnapshotRepository {
    pub fn new(app_data_dir: impl AsRef<Path>) -> Self {
        let path = app_data_dir.as_ref().join("snapshots.json");
        let lock = path_lock(&path);
        Self { path, lock }
    }

    pub fn load(&self) -> io::Result<SnapshotStore> {
        let _guard = self
            .lock
            .lock()
            .map_err(|_| io::Error::other("snapshot lock poisoned"))?;
        self.load_locked()
    }

    pub fn save_provider(&self, record: ProviderRecord) -> io::Result<()> {
        validate_record(&record)?;
        let _guard = self
            .lock
            .lock()
            .map_err(|_| io::Error::other("snapshot lock poisoned"))?;
        let current = self.load_locked()?;
        let previous = match record.provider {
            Provider::Claude => current.claude.as_ref(),
            Provider::Codex => current.codex.as_ref(),
        };
        if previous.is_some_and(|previous| record.revision <= previous.revision) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "provider revision must increase",
            ));
        }
        let record = preserve_latest(record, previous);
        validate_record(&record)?;
        let updated = match record.provider {
            Provider::Claude => SnapshotStore {
                claude: Some(record),
                ..current
            },
            Provider::Codex => SnapshotStore {
                codex: Some(record),
                ..current
            },
        };
        write_json_atomically(&self.path, &updated)
    }

    fn load_locked(&self) -> io::Result<SnapshotStore> {
        let bytes = match fs::read(&self.path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                return Ok(SnapshotStore::default())
            }
            Err(error) => return Err(error),
        };
        let value: serde_json::Value = match serde_json::from_slice(&bytes) {
            Ok(value) => value,
            Err(_) => {
                quarantine(&self.path)?;
                return Ok(SnapshotStore::default());
            }
        };
        if validate_json_schema(&value, false).is_ok() {
            if let Ok(store) = serde_json::from_value::<SnapshotStore>(value.clone()) {
                if validate_store(&store).is_ok() {
                    return Ok(store);
                }
            }
        }
        if validate_json_schema(&value, true).is_ok() {
            if let Ok(legacy) = serde_json::from_value::<LegacySnapshotStore>(value) {
                let migrated = SnapshotStore {
                    claude: legacy.claude,
                    codex: legacy.codex,
                    ..SnapshotStore::default()
                };
                if validate_store(&migrated).is_ok() {
                    write_json_atomically(&self.path, &migrated)?;
                    return Ok(migrated);
                }
            }
        }
        quarantine(&self.path)?;
        Ok(SnapshotStore::default())
    }
}

fn validate_store(store: &SnapshotStore) -> io::Result<()> {
    if store.schema_version != SNAPSHOT_SCHEMA_VERSION {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "unsupported snapshot schema",
        ));
    }
    if let Some(record) = &store.claude {
        validate_record(record)?;
        if record.provider != Provider::Claude {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "claude slot mismatch",
            ));
        }
    }
    if let Some(record) = &store.codex {
        validate_record(record)?;
        if record.provider != Provider::Codex {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "codex slot mismatch",
            ));
        }
    }
    Ok(())
}

fn validate_record(record: &ProviderRecord) -> io::Result<()> {
    match (&record.last_outcome, &record.latest) {
        (OutcomeMetadata::Success, Some(snapshot)) => {
            if snapshot.provider != record.provider || snapshot.revision != record.revision {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "successful snapshot mismatch",
                ));
            }
        }
        (OutcomeMetadata::Success, None) => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "success requires snapshot",
            ))
        }
        (_, Some(snapshot)) => {
            if snapshot.provider != record.provider || snapshot.revision >= record.revision {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "last-good snapshot mismatch",
                ));
            }
        }
        (_, None) => {}
    }
    Ok(())
}

fn preserve_latest(
    mut record: ProviderRecord,
    previous: Option<&ProviderRecord>,
) -> ProviderRecord {
    if !matches!(record.last_outcome, OutcomeMetadata::Success) && record.latest.is_none() {
        record.latest = previous.and_then(|previous| previous.latest.clone());
    }
    record
}

fn validate_json_schema(value: &serde_json::Value, legacy: bool) -> io::Result<()> {
    validate_keys(
        value,
        if legacy {
            &["claude", "codex"]
        } else {
            &["schema_version", "claude", "codex"]
        },
    )?;
    for key in ["claude", "codex"] {
        if let Some(record) = value.get(key).filter(|value| !value.is_null()) {
            validate_keys(record, &["provider", "latest", "last_outcome", "revision"])?;
            let outcome = &record["last_outcome"];
            match outcome.get("kind").and_then(serde_json::Value::as_str) {
                Some("failed") => validate_keys(outcome, &["kind", "class"])?,
                Some("success" | "credentials_missing" | "cli_missing") => {
                    validate_keys(outcome, &["kind"])?;
                }
                _ => {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "invalid outcome kind",
                    ))
                }
            }
            if let Some(snapshot) = record.get("latest").filter(|value| !value.is_null()) {
                validate_keys(
                    snapshot,
                    &[
                        "provider",
                        "plan_type",
                        "session",
                        "weekly",
                        "captured_at",
                        "source",
                        "is_cached",
                        "revision",
                    ],
                )?;
                for window in ["session", "weekly"] {
                    if let Some(window) = snapshot.get(window).filter(|value| !value.is_null()) {
                        validate_keys(window, &["used_percent", "window_minutes", "resets_at"])?;
                    }
                }
            }
        }
    }
    Ok(())
}

fn validate_keys(value: &serde_json::Value, allowed: &[&str]) -> io::Result<()> {
    let object = value
        .as_object()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "store object expected"))?;
    if let Some(key) = object.keys().find(|key| !allowed.contains(&key.as_str())) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("unknown persisted field: {key}"),
        ));
    }
    Ok(())
}
