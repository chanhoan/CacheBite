use tauri::{AppHandle, Emitter, Manager, State};

use super::{ProviderState, RefreshService};
use crate::{
    domain::{FailureClass, Provider, ProviderUsageSnapshot, UnavailableReason},
    store::{
        HistoryRepository, HistoryStore, LogicalPosition, PetPackage, PetPackageRepository,
        Settings, SettingsRepository,
    },
    window::{command_allowed, CapabilityDiagnostic, NativeCommand, PlatformCapabilities},
};
use serde::Serialize;

pub const PROVIDER_STATE_EVENT: &str = "provider-state";

#[derive(Clone, Debug, Serialize)]
pub struct ProviderStateDto {
    pub provider: Provider,
    pub snapshot: Option<ProviderUsageSnapshot>,
    pub failure_class: Option<FailureClass>,
    pub unavailable_reason: Option<UnavailableReason>,
    pub expired: bool,
    pub reset_pending: bool,
    pub revision: u64,
}

impl From<ProviderState> for ProviderStateDto {
    fn from(state: ProviderState) -> Self {
        Self {
            provider: state.provider,
            snapshot: state.snapshot,
            failure_class: state.failure_class,
            unavailable_reason: state.unavailable_reason,
            expired: state.expired,
            reset_pending: state.reset_pending,
            revision: state.revision,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct ProviderStatesDto {
    pub claude: ProviderStateDto,
    pub codex: ProviderStateDto,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CollectorMode {
    Fixture,
    Production,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub struct CollectorModeDto {
    pub claude: CollectorMode,
    pub codex: CollectorMode,
}

impl CollectorModeDto {
    pub fn for_fixture_gate(enabled: bool) -> Self {
        let mode = if enabled {
            CollectorMode::Fixture
        } else {
            CollectorMode::Production
        };
        Self {
            claude: mode,
            codex: mode,
        }
    }
}

#[tauri::command]
pub fn get_collector_mode(
    window: tauri::WebviewWindow,
    mode: State<'_, CollectorModeDto>,
) -> Result<CollectorModeDto, IpcError> {
    authorize(&window, NativeCommand::GetCollectorMode)?;
    Ok(*mode)
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum IpcError {
    Forbidden,
    ServiceUnavailable,
    InvalidSettings,
    PersistenceUnavailable,
    PanelUnavailable,
}

fn authorize(window: &tauri::WebviewWindow, command: NativeCommand) -> Result<(), IpcError> {
    command_allowed(window.label(), command)
        .then_some(())
        .ok_or(IpcError::Forbidden)
}

#[tauri::command]
pub fn get_provider_states(
    window: tauri::WebviewWindow,
    service: State<'_, RefreshService>,
) -> Result<ProviderStatesDto, IpcError> {
    authorize(&window, NativeCommand::GetProviderStates)?;
    let states = service.get_provider_states();
    Ok(ProviderStatesDto {
        claude: states.claude.into(),
        codex: states.codex.into(),
    })
}

#[tauri::command]
pub async fn refresh_provider(
    window: tauri::WebviewWindow,
    service: State<'_, RefreshService>,
    provider: Provider,
) -> Result<(), IpcError> {
    authorize(&window, NativeCommand::RefreshProvider)?;
    service
        .refresh_provider(provider)
        .await
        .map_err(|_| IpcError::ServiceUnavailable)
}

#[tauri::command]
pub fn get_settings(
    window: tauri::WebviewWindow,
    repository: State<'_, SettingsRepository>,
) -> Result<Settings, IpcError> {
    authorize(&window, NativeCommand::GetSettings)?;
    repository
        .load()
        .map_err(|_| IpcError::PersistenceUnavailable)
}

#[tauri::command]
pub fn get_history(
    window: tauri::WebviewWindow,
    repository: State<'_, HistoryRepository>,
) -> Result<HistoryStore, IpcError> {
    authorize(&window, NativeCommand::GetHistory)?;
    repository
        .load()
        .map_err(|_| IpcError::PersistenceUnavailable)
}

#[tauri::command]
pub fn get_pet_package(
    window: tauri::WebviewWindow,
    settings: State<'_, SettingsRepository>,
    pets: State<'_, PetPackageRepository>,
) -> Result<PetPackage, IpcError> {
    authorize(&window, NativeCommand::GetPetPackage)?;
    let id = settings
        .load()
        .map_err(|_| IpcError::PersistenceUnavailable)?
        .selected_pet_id;
    pets.load(&id).map_err(|_| IpcError::PersistenceUnavailable)
}

#[tauri::command]
pub fn get_platform_capabilities(
    window: tauri::WebviewWindow,
) -> Result<PlatformCapabilities, IpcError> {
    authorize(&window, NativeCommand::GetPlatformCapabilities)?;
    Ok(PlatformCapabilities {
        always_on_top: CapabilityDiagnostic::Unavailable {
            reason: "always-on-top support is unverified on this platform build",
        },
        fullscreen_detection: CapabilityDiagnostic::Unavailable {
            reason: "fullscreen detection is unavailable on this build",
        },
        autostart: CapabilityDiagnostic::Unavailable {
            reason: "autostart integration is unavailable on this build",
        },
    })
}

#[tauri::command]
pub fn save_position(
    window: tauri::WebviewWindow,
    repository: State<'_, SettingsRepository>,
    position: LogicalPosition,
) -> Result<(), IpcError> {
    authorize(&window, NativeCommand::SavePosition)?;
    repository.save_position(position).map_err(|error| {
        if error.kind() == std::io::ErrorKind::InvalidData {
            IpcError::InvalidSettings
        } else {
            IpcError::PersistenceUnavailable
        }
    })
}

#[tauri::command]
pub fn update_settings(
    window: tauri::WebviewWindow,
    repository: State<'_, SettingsRepository>,
    settings: Settings,
) -> Result<Settings, IpcError> {
    authorize(&window, NativeCommand::UpdateSettings)?;
    repository.save(&settings).map_err(|error| {
        if error.kind() == std::io::ErrorKind::InvalidData {
            IpcError::InvalidSettings
        } else {
            IpcError::PersistenceUnavailable
        }
    })?;
    Ok(settings)
}

#[tauri::command]
pub fn show_panel(window: tauri::WebviewWindow, app: AppHandle) -> Result<(), IpcError> {
    authorize(&window, NativeCommand::ShowPanel)?;
    let panel = app
        .get_webview_window("panel")
        .ok_or(IpcError::PanelUnavailable)?;
    if let (Ok(Some(monitor)), Ok(position), Ok(pet_size), Ok(panel_size)) = (
        window.current_monitor(),
        window.outer_position(),
        window.outer_size(),
        panel.outer_size(),
    ) {
        let monitor_position = monitor.position();
        let monitor_size = monitor.size();
        let anchored = crate::window::anchor_panel(
            crate::window::Rect {
                x: f64::from(position.x),
                y: f64::from(position.y),
                width: f64::from(pet_size.width),
                height: f64::from(pet_size.height),
            },
            crate::window::Size {
                width: f64::from(panel_size.width),
                height: f64::from(panel_size.height),
            },
            crate::window::Rect {
                x: f64::from(monitor_position.x),
                y: f64::from(monitor_position.y),
                width: f64::from(monitor_size.width),
                height: f64::from(monitor_size.height),
            },
            12.0,
        );
        panel
            .set_position(tauri::PhysicalPosition::new(
                anchored.x.round() as i32,
                anchored.y.round() as i32,
            ))
            .map_err(|_| IpcError::PanelUnavailable)?;
    }
    panel.show().map_err(|_| IpcError::PanelUnavailable)
}

#[tauri::command]
pub fn quit(window: tauri::WebviewWindow, app: AppHandle) -> Result<(), IpcError> {
    authorize(&window, NativeCommand::Quit)?;
    app.exit(0);
    Ok(())
}

pub fn emit_provider_states(app: &AppHandle, service: &RefreshService) {
    for provider in [Provider::Claude, Provider::Codex] {
        let app = app.clone();
        let mut states = service.subscribe(provider);
        tauri::async_runtime::spawn(async move {
            while states.changed().await.is_ok() {
                let state: ProviderStateDto = states.borrow_and_update().clone().into();
                if app.emit(PROVIDER_STATE_EVENT, state).is_err() {
                    break;
                }
            }
        });
    }
}
