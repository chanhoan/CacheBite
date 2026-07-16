use std::{
    fs, io,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use serde::{Deserialize, Serialize};
use time::{Duration, OffsetDateTime};

use super::{path_lock, quarantine, write_json_atomically};
use crate::domain::{Provider, ProviderUsageSnapshot, UsageWindow};

const HISTORY_SCHEMA_VERSION: u32 = 1;
const MAX_SAMPLES_PER_PROVIDER: usize = 3_000;
const RETENTION: Duration = Duration::days(30);

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct HistoryPoint {
    pub used_percent: f64,
    #[serde(with = "time::serde::rfc3339::option")]
    pub resets_at: Option<OffsetDateTime>,
    pub starts_new_segment: bool,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct HistorySample {
    #[serde(with = "time::serde::rfc3339")]
    pub captured_at: OffsetDateTime,
    pub session: Option<HistoryPoint>,
    pub weekly: Option<HistoryPoint>,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct HistoryProvider {
    pub samples: Vec<HistorySample>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct HistoryStore {
    pub schema_version: u32,
    pub claude: HistoryProvider,
    pub codex: HistoryProvider,
}

impl Default for HistoryStore {
    fn default() -> Self {
        Self {
            schema_version: HISTORY_SCHEMA_VERSION,
            claude: HistoryProvider::default(),
            codex: HistoryProvider::default(),
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LegacyStore {
    claude: Vec<LegacySample>,
    codex: Vec<LegacySample>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LegacySample {
    #[serde(with = "time::serde::rfc3339")]
    captured_at: OffsetDateTime,
    session: Option<LegacyPoint>,
    weekly: Option<LegacyPoint>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LegacyPoint {
    used_percent: f64,
    #[serde(with = "time::serde::rfc3339::option")]
    resets_at: Option<OffsetDateTime>,
}

pub struct HistoryRepository {
    path: PathBuf,
    lock: Arc<Mutex<()>>,
}

impl HistoryRepository {
    pub fn new(app_data_dir: impl AsRef<Path>) -> Self {
        let path = app_data_dir.as_ref().join("history.json");
        Self {
            lock: path_lock(&path),
            path,
        }
    }

    pub fn load(&self) -> io::Result<HistoryStore> {
        self.load_at(OffsetDateTime::now_utc())
    }

    pub fn load_at(&self, now: OffsetDateTime) -> io::Result<HistoryStore> {
        let _guard = self
            .lock
            .lock()
            .map_err(|_| io::Error::other("history lock poisoned"))?;
        self.load_locked(now)
    }

    pub fn append_success(
        &self,
        snapshot: &ProviderUsageSnapshot,
        now: OffsetDateTime,
    ) -> io::Result<bool> {
        if snapshot.is_cached {
            return Ok(false);
        }
        let _guard = self
            .lock
            .lock()
            .map_err(|_| io::Error::other("history lock poisoned"))?;
        let mut store = self.load_locked(now)?;
        let provider = match snapshot.provider {
            Provider::Claude => &mut store.claude,
            Provider::Codex => &mut store.codex,
        };
        if provider
            .samples
            .last()
            .is_some_and(|sample| snapshot.captured_at <= sample.captured_at)
        {
            return Ok(false);
        }
        let previous = provider.samples.last();
        let sample = HistorySample {
            captured_at: snapshot.captured_at,
            session: point(
                snapshot.session.as_ref(),
                previous.and_then(|sample| sample.session.as_ref()),
                snapshot.captured_at,
            ),
            weekly: point(
                snapshot.weekly.as_ref(),
                previous.and_then(|sample| sample.weekly.as_ref()),
                snapshot.captured_at,
            ),
        };
        if sample.session.is_none() && sample.weekly.is_none() {
            return Ok(false);
        }
        provider.samples.push(sample);
        prune(provider, now);
        write_json_atomically(&self.path, &store)?;
        Ok(true)
    }

    fn load_locked(&self, now: OffsetDateTime) -> io::Result<HistoryStore> {
        let bytes = match fs::read(&self.path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                return Ok(HistoryStore::default())
            }
            Err(error) => return Err(error),
        };
        if let Ok(mut store) = serde_json::from_slice::<HistoryStore>(&bytes) {
            if store.schema_version == HISTORY_SCHEMA_VERSION {
                let before = store.clone();
                prune(&mut store.claude, now);
                prune(&mut store.codex, now);
                if validate(&store).is_ok() {
                    if store != before {
                        write_json_atomically(&self.path, &store)?;
                    }
                    return Ok(store);
                }
            }
        }
        if let Ok(legacy) = serde_json::from_slice::<LegacyStore>(&bytes) {
            let mut store = HistoryStore {
                schema_version: HISTORY_SCHEMA_VERSION,
                claude: migrate(legacy.claude),
                codex: migrate(legacy.codex),
            };
            prune(&mut store.claude, now);
            prune(&mut store.codex, now);
            validate(&store)?;
            write_json_atomically(&self.path, &store)?;
            return Ok(store);
        }
        quarantine(&self.path)?;
        Ok(HistoryStore::default())
    }
}

fn point(
    window: Option<&UsageWindow>,
    previous: Option<&HistoryPoint>,
    captured_at: OffsetDateTime,
) -> Option<HistoryPoint> {
    window.map(|window| HistoryPoint {
        used_percent: window.used_percent,
        resets_at: window.resets_at,
        starts_new_segment: previous.is_some_and(|previous| {
            previous.resets_at != window.resets_at
                || previous.resets_at.is_some_and(|reset| reset <= captured_at)
        }),
    })
}

fn migrate(samples: Vec<LegacySample>) -> HistoryProvider {
    let mut provider = HistoryProvider::default();
    for sample in samples {
        if provider
            .samples
            .last()
            .is_some_and(|last| sample.captured_at <= last.captured_at)
        {
            continue;
        }
        let previous = provider.samples.last();
        let convert = |point: Option<LegacyPoint>, previous: Option<&HistoryPoint>| {
            point.map(|point| HistoryPoint {
                used_percent: point.used_percent,
                resets_at: point.resets_at,
                starts_new_segment: previous.is_some_and(|previous| {
                    previous.resets_at != point.resets_at
                        || previous
                            .resets_at
                            .is_some_and(|reset| reset <= sample.captured_at)
                }),
            })
        };
        provider.samples.push(HistorySample {
            captured_at: sample.captured_at,
            session: convert(sample.session, previous.and_then(|s| s.session.as_ref())),
            weekly: convert(sample.weekly, previous.and_then(|s| s.weekly.as_ref())),
        });
    }
    provider
}

fn prune(provider: &mut HistoryProvider, now: OffsetDateTime) {
    let cutoff = now - RETENTION;
    provider
        .samples
        .retain(|sample| sample.captured_at >= cutoff && sample.captured_at <= now);
    if provider.samples.len() > MAX_SAMPLES_PER_PROVIDER {
        provider
            .samples
            .drain(..provider.samples.len() - MAX_SAMPLES_PER_PROVIDER);
    }
}

fn validate(store: &HistoryStore) -> io::Result<()> {
    for provider in [&store.claude, &store.codex] {
        if provider.samples.len() > MAX_SAMPLES_PER_PROVIDER
            || provider
                .samples
                .windows(2)
                .any(|pair| pair[0].captured_at >= pair[1].captured_at)
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "history samples must be bounded and ordered",
            ));
        }
        for point in provider
            .samples
            .iter()
            .flat_map(|sample| [sample.session.as_ref(), sample.weekly.as_ref()])
            .flatten()
        {
            if !point.used_percent.is_finite() || !(0.0..=100.0).contains(&point.used_percent) {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "invalid history percent",
                ));
            }
        }
    }
    Ok(())
}
