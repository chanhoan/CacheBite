use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

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

/// Gap between the pet and the panel, in logical pixels. Scaled by the
/// monitor's factor before it reaches `anchor_panel`, which works in physical
/// pixels throughout.
const PANEL_ANCHOR_GAP_LOGICAL: f64 = 12.0;
/// Fixed panel width in logical pixels. Only the height tracks content.
const PANEL_WIDTH_LOGICAL: f64 = 312.0;
/// How long `show_panel` waits for the renderer to report its measured height
/// before revealing the panel anyway. A single misplaced frame beats a panel
/// that never appears because the renderer failed to measure.
const PANEL_LAYOUT_GRACE: Duration = Duration::from_millis(150);

/// Tracks a `show_panel` request that is still waiting for the renderer to
/// report its content height, so the panel is revealed only once it has been
/// placed at its final size.
#[derive(Default)]
pub struct PanelLayoutGate {
    awaiting_layout: AtomicBool,
}

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
    InvalidPanelSize,
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
    repository: State<'_, Arc<HistoryRepository>>,
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
    let fullscreen_detection = if cfg!(windows) {
        CapabilityDiagnostic::Available
    } else {
        CapabilityDiagnostic::Unavailable {
            reason: "fullscreen detection is unavailable on this build",
        }
    };
    Ok(PlatformCapabilities {
        os: crate::window::platform_os(std::env::consts::OS),
        always_on_top: CapabilityDiagnostic::Unavailable {
            reason: "always-on-top support is unverified on this platform build",
        },
        fullscreen_detection,
        autostart: CapabilityDiagnostic::Available,
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
    app: AppHandle,
    repository: State<'_, SettingsRepository>,
    settings: Settings,
) -> Result<Settings, IpcError> {
    authorize(&window, NativeCommand::UpdateSettings)?;
    let previous = repository
        .load()
        .map_err(|_| IpcError::PersistenceUnavailable)?;
    repository.save(&settings).map_err(|error| {
        if error.kind() == std::io::ErrorKind::InvalidData {
            IpcError::InvalidSettings
        } else {
            IpcError::PersistenceUnavailable
        }
    })?;
    if previous.start_at_login != settings.start_at_login {
        use tauri_plugin_autostart::ManagerExt;

        let manager = app.autolaunch();
        let result = if settings.start_at_login {
            manager.enable()
        } else {
            manager.disable()
        };
        if result.is_err() {
            let _ = repository.save(&previous);
            return Err(IpcError::ServiceUnavailable);
        }
    }
    app.emit("settings-updated", &settings)
        .map_err(|_| IpcError::ServiceUnavailable)?;
    Ok(settings)
}

#[tauri::command]
pub fn show_panel(
    window: tauri::WebviewWindow,
    app: AppHandle,
    gate: State<'_, PanelLayoutGate>,
) -> Result<(), IpcError> {
    authorize(&window, NativeCommand::ShowPanel)?;
    let panel = app
        .get_webview_window("panel")
        .ok_or(IpcError::PanelUnavailable)?;
    // Place it now even though the renderer may resize it in a moment: the
    // grace timer below can reveal the panel without a second placement pass.
    position_panel(&window, &panel)?;

    if panel.is_visible().unwrap_or(false) {
        return Ok(());
    }
    gate.awaiting_layout.store(true, Ordering::SeqCst);

    let deadline_app = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(PANEL_LAYOUT_GRACE).await;
        if deadline_app
            .state::<PanelLayoutGate>()
            .awaiting_layout
            .swap(false, Ordering::SeqCst)
        {
            if let Some(panel) = deadline_app.get_webview_window("panel") {
                let _ = panel.show();
            }
        }
    });
    Ok(())
}

pub(crate) fn position_panel(
    anchor: &tauri::WebviewWindow,
    panel: &tauri::WebviewWindow,
) -> Result<(), IpcError> {
    // Headless runners and some Wayland compositors report no monitor. Skipping
    // placement there is deliberate — promoting it to an error would break the
    // fixture jobs in native-smoke.yml.
    let (Ok(Some(monitor)), Ok(position), Ok(pet_size), Ok(panel_size)) = (
        anchor.current_monitor(),
        anchor.outer_position(),
        anchor.outer_size(),
        panel.outer_size(),
    ) else {
        return Ok(());
    };
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
        usable_area(&monitor),
        PANEL_ANCHOR_GAP_LOGICAL * usable_scale(monitor.scale_factor()),
    );
    panel
        .set_position(tauri::PhysicalPosition::new(
            anchored.x.round() as i32,
            anchored.y.round() as i32,
        ))
        .map_err(|_| IpcError::PanelUnavailable)
}

/// The monitor area left over once the taskbar, dock and menu bar are excluded.
///
/// Backends that cannot report a work area hand back `PhysicalRect::default()`,
/// which is all zeroes. Taking that literally would clamp the panel into the
/// top-left corner, so fall back to the full monitor instead.
fn usable_area(monitor: &tauri::Monitor) -> crate::window::Rect {
    let work_area = monitor.work_area();
    if work_area.size.width > 0 && work_area.size.height > 0 {
        return crate::window::Rect {
            x: f64::from(work_area.position.x),
            y: f64::from(work_area.position.y),
            width: f64::from(work_area.size.width),
            height: f64::from(work_area.size.height),
        };
    }
    let position = monitor.position();
    let size = monitor.size();
    crate::window::Rect {
        x: f64::from(position.x),
        y: f64::from(position.y),
        width: f64::from(size.width),
        height: f64::from(size.height),
    }
}

fn usable_scale(scale_factor: f64) -> f64 {
    if scale_factor.is_finite() && scale_factor > 0.0 {
        scale_factor
    } else {
        1.0
    }
}

#[tauri::command]
pub fn resize_panel(
    window: tauri::WebviewWindow,
    app: AppHandle,
    gate: State<'_, PanelLayoutGate>,
    height: f64,
) -> Result<(), IpcError> {
    authorize(&window, NativeCommand::ResizePanel)?;
    if !height.is_finite() || height <= 0.0 {
        return Err(IpcError::InvalidPanelSize);
    }
    let panel = app
        .get_webview_window("panel")
        .ok_or(IpcError::PanelUnavailable)?;
    panel
        .set_size(tauri::LogicalSize::new(PANEL_WIDTH_LOGICAL, height.ceil()))
        .map_err(|_| IpcError::PanelUnavailable)?;
    let overlay = app
        .get_webview_window("overlay")
        .ok_or(IpcError::PanelUnavailable)?;
    position_panel(&overlay, &panel)?;
    // The measurement this resize carries is what show_panel was waiting for.
    if gate.awaiting_layout.swap(false, Ordering::SeqCst) {
        panel.show().map_err(|_| IpcError::PanelUnavailable)?;
    }
    Ok(())
}

#[tauri::command]
pub fn hide_panel(
    window: tauri::WebviewWindow,
    gate: State<'_, PanelLayoutGate>,
) -> Result<(), IpcError> {
    authorize(&window, NativeCommand::HidePanel)?;
    // Stop a pending grace timer from resurrecting the panel just closed.
    gate.awaiting_layout.store(false, Ordering::SeqCst);
    window.hide().map_err(|_| IpcError::PanelUnavailable)
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
                #[cfg(debug_assertions)]
                {
                    let session = state.snapshot.as_ref().and_then(|snapshot| {
                        snapshot
                            .session
                            .as_ref()
                            .map(|window| (window.used_percent, window.window_minutes))
                    });
                    let weekly = state.snapshot.as_ref().and_then(|snapshot| {
                        snapshot
                            .weekly
                            .as_ref()
                            .map(|window| (window.used_percent, window.window_minutes))
                    });
                    eprintln!(
                        "[CacheBite:{:?}] emit has_snapshot={} session={session:?} weekly={weekly:?} fail={:?} unavailable={:?} expired={} reset_pending={} rev={}",
                        state.provider,
                        state.snapshot.is_some(),
                        state.failure_class,
                        state.unavailable_reason,
                        state.expired,
                        state.reset_pending,
                        state.revision,
                    );
                }
                if app.emit(PROVIDER_STATE_EVENT, state).is_err() {
                    break;
                }
            }
        });
    }
}
