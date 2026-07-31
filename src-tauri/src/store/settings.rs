use std::{
    fs, io,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use serde::{Deserialize, Serialize};

use super::{path_lock, quarantine, write_json_atomically};
use crate::domain::{is_valid_pet_id, Provider};

const SETTINGS_SCHEMA_VERSION: u32 = 4;
pub const DEFAULT_HIDE_SHOW_HOTKEY: &str = "CommandOrControl+Shift+H";

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LogicalPosition {
    pub x: f64,
    pub y: f64,
}

impl Default for LogicalPosition {
    fn default() -> Self {
        Self { x: 0.0, y: 0.0 }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Settings {
    pub schema_version: u32,
    pub primary_provider: Provider,
    pub selected_pet_id: String,
    pub bubble_enabled: bool,
    pub start_at_login: bool,
    pub notification_enabled: bool,
    pub secondary_notification_enabled: bool,
    pub logical_position: LogicalPosition,
    pub hide_show_hotkey: Option<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            schema_version: SETTINGS_SCHEMA_VERSION,
            primary_provider: Provider::Claude,
            selected_pet_id: "tabby".into(),
            bubble_enabled: true,
            start_at_login: false,
            notification_enabled: false,
            secondary_notification_enabled: false,
            logical_position: LogicalPosition::default(),
            hide_show_hotkey: Some(DEFAULT_HIDE_SHOW_HOTKEY.into()),
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LegacySettings {
    primary_provider: Provider,
    pet_id: String,
    show_bubbles: bool,
    start_at_login: bool,
    position: LogicalPosition,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SettingsV1 {
    schema_version: u32,
    primary_provider: Provider,
    selected_pet_id: String,
    bubble_enabled: bool,
    start_at_login: bool,
    logical_position: LogicalPosition,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SettingsV2 {
    schema_version: u32,
    primary_provider: Provider,
    selected_pet_id: String,
    bubble_enabled: bool,
    start_at_login: bool,
    notification_enabled: bool,
    logical_position: LogicalPosition,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SettingsV3 {
    schema_version: u32,
    primary_provider: Provider,
    selected_pet_id: String,
    bubble_enabled: bool,
    start_at_login: bool,
    notification_enabled: bool,
    secondary_notification_enabled: bool,
    logical_position: LogicalPosition,
}

pub struct SettingsRepository {
    path: PathBuf,
    lock: Arc<Mutex<()>>,
}

impl SettingsRepository {
    pub fn new(app_data_dir: impl AsRef<Path>) -> Self {
        let path = app_data_dir.as_ref().join("settings.json");
        let lock = path_lock(&path);
        Self { path, lock }
    }

    pub fn load(&self) -> io::Result<Settings> {
        let _guard = self
            .lock
            .lock()
            .map_err(|_| io::Error::other("settings lock poisoned"))?;
        self.load_locked()
    }

    pub fn save(&self, settings: &Settings) -> io::Result<()> {
        let _guard = self
            .lock
            .lock()
            .map_err(|_| io::Error::other("settings lock poisoned"))?;
        validate(settings)?;
        write_json_atomically(&self.path, settings)
    }

    pub fn save_position(&self, logical_position: LogicalPosition) -> io::Result<()> {
        let _guard = self
            .lock
            .lock()
            .map_err(|_| io::Error::other("settings lock poisoned"))?;
        let current = self.load_locked()?;
        let updated = Settings {
            logical_position,
            ..current
        };
        validate(&updated)?;
        write_json_atomically(&self.path, &updated)
    }

    pub fn clear_hotkey(&self) -> io::Result<()> {
        let _guard = self
            .lock
            .lock()
            .map_err(|_| io::Error::other("settings lock poisoned"))?;
        let current = self.load_locked()?;
        let updated = Settings {
            hide_show_hotkey: None,
            ..current
        };
        validate(&updated)?;
        write_json_atomically(&self.path, &updated)
    }

    fn load_locked(&self) -> io::Result<Settings> {
        let bytes = match fs::read(&self.path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Settings::default()),
            Err(error) => return Err(error),
        };
        if let Ok(settings) = serde_json::from_slice::<Settings>(&bytes) {
            if validate(&settings).is_ok() {
                // `idle` is an animation key an early build wrote as a package
                // id; `cat` was renamed to `tabby`. Either value points at a
                // package that no longer exists, so repair it in place rather
                // than letting get_pet_package fail.
                let repaired = match settings.selected_pet_id.as_str() {
                    "idle" => Some(match settings.primary_provider {
                        Provider::Claude => "tabby",
                        Provider::Codex => "corgi",
                    }),
                    "cat" => Some("tabby"),
                    _ => None,
                };
                if let Some(pet_id) = repaired {
                    let migrated = Settings {
                        selected_pet_id: pet_id.into(),
                        ..settings
                    };
                    write_json_atomically(&self.path, &migrated)?;
                    return Ok(migrated);
                }
                return Ok(settings);
            }
        }
        if let Ok(previous) = serde_json::from_slice::<SettingsV1>(&bytes) {
            if previous.schema_version == 1 {
                let migrated = Settings {
                    primary_provider: previous.primary_provider,
                    selected_pet_id: previous.selected_pet_id,
                    bubble_enabled: previous.bubble_enabled,
                    start_at_login: previous.start_at_login,
                    logical_position: previous.logical_position,
                    ..Settings::default()
                };
                // Validate before writing, as the legacy path does. An old file
                // carrying an unusable value would otherwise be written out and
                // then quarantined on re-read, discarding position and
                // notification settings that migrated across just fine.
                validate(&migrated)?;
                write_json_atomically(&self.path, &migrated)?;
                // Re-read so the current-schema pass above can repair a
                // retired `selected_pet_id` carried over from the old file.
                return self.load_locked();
            }
        }
        if let Ok(previous) = serde_json::from_slice::<SettingsV2>(&bytes) {
            if previous.schema_version == 2 {
                let migrated = Settings {
                    primary_provider: previous.primary_provider,
                    selected_pet_id: previous.selected_pet_id,
                    bubble_enabled: previous.bubble_enabled,
                    start_at_login: previous.start_at_login,
                    notification_enabled: previous.notification_enabled,
                    logical_position: previous.logical_position,
                    ..Settings::default()
                };
                validate(&migrated)?;
                write_json_atomically(&self.path, &migrated)?;
                return self.load_locked();
            }
        }
        if let Ok(previous) = serde_json::from_slice::<SettingsV3>(&bytes) {
            if previous.schema_version == 3 {
                let migrated = Settings {
                    primary_provider: previous.primary_provider,
                    selected_pet_id: previous.selected_pet_id,
                    bubble_enabled: previous.bubble_enabled,
                    start_at_login: previous.start_at_login,
                    notification_enabled: previous.notification_enabled,
                    secondary_notification_enabled: previous.secondary_notification_enabled,
                    logical_position: previous.logical_position,
                    ..Settings::default()
                };
                validate(&migrated)?;
                write_json_atomically(&self.path, &migrated)?;
                return self.load_locked();
            }
        }
        if let Ok(legacy) = serde_json::from_slice::<LegacySettings>(&bytes) {
            let migrated = Settings {
                primary_provider: legacy.primary_provider,
                selected_pet_id: legacy.pet_id,
                bubble_enabled: legacy.show_bubbles,
                start_at_login: legacy.start_at_login,
                logical_position: legacy.position,
                ..Settings::default()
            };
            validate(&migrated)?;
            write_json_atomically(&self.path, &migrated)?;
            return self.load_locked();
        }
        quarantine(&self.path)?;
        Ok(Settings::default())
    }
}

fn validate(settings: &Settings) -> io::Result<()> {
    if settings.schema_version != SETTINGS_SCHEMA_VERSION {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "unsupported settings schema",
        ));
    }
    if !is_valid_pet_id(&settings.selected_pet_id) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "pet id is invalid",
        ));
    }
    if !settings.logical_position.x.is_finite() || !settings.logical_position.y.is_finite() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "position must be finite",
        ));
    }
    if let Some(hotkey) = &settings.hide_show_hotkey {
        use std::str::FromStr;
        if tauri_plugin_global_shortcut::Shortcut::from_str(hotkey).is_err() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "hotkey is invalid",
            ));
        }
    }
    Ok(())
}
