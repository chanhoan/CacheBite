use std::io;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use tauri::{AppHandle, Emitter, Manager, State};

use super::{ProviderState, RefreshService};
use crate::{
    domain::{FailureClass, Provider, ProviderUsageSnapshot, UnavailableReason},
    store::{
        HistoryRepository, HistoryStore, LogicalPosition, PetPackage, PetPackageRepository,
        PetSummary, Settings, SettingsRepository,
    },
    window::{
        command_allowed, panel_toggle, CapabilityDiagnostic, HideShowHotkeyCapability,
        NativeCommand, PanelToggle, PlatformCapabilities,
    },
};
use serde::Serialize;

pub const PROVIDER_STATE_EVENT: &str = "provider-state";

/// Gap between the pet and the panel, in logical pixels. Scaled by the
/// monitor's factor before it reaches `anchor_panel`, which works in physical
/// pixels throughout.
const PANEL_ANCHOR_GAP_LOGICAL: f64 = 12.0;
/// Fixed panel width in logical pixels. Only the height tracks content.
const PANEL_WIDTH_LOGICAL: f64 = 312.0;
/// How long `toggle_panel` waits for the renderer to report its measured height
/// before revealing the panel anyway. A single misplaced frame beats a panel
/// that never appears because the renderer failed to measure.
const PANEL_LAYOUT_GRACE: Duration = Duration::from_millis(150);

/// Tracks a `toggle_panel` request that is still waiting for the renderer to
/// report its content height, so the panel is revealed only once it has been
/// placed at its final size.
#[derive(Default)]
pub struct PanelLayoutGate {
    awaiting_layout: AtomicBool,
}

impl PanelLayoutGate {
    /// Cancels a pending reveal.
    ///
    /// Exposed because `lib.rs` hides the panel too — the hide/show hotkey and
    /// the fullscreen monitor — and an armed grace timer would put the panel
    /// back on screen milliseconds after it was hidden.
    pub fn disarm(&self) {
        self.awaiting_layout.store(false, Ordering::SeqCst);
    }
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

/// The state the panel will be in once this toggle settles — and the state the
/// next toggle will read.
///
/// Returned so the native boundary can be exercised end to end without a
/// visibility query command. The renderer must not cache it: the `✕`, the
/// hide/show hotkey and the fullscreen monitor all change panel visibility
/// without going through the renderer at all.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PanelVisibility {
    Shown,
    Hidden,
}

#[cfg(feature = "webdriver")]
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct WindowStateDto {
    pub label: String,
    pub is_visible: bool,
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

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum IpcError {
    Forbidden,
    ServiceUnavailable,
    InvalidSettings,
    InvalidPanelSize,
    PersistenceUnavailable,
    PanelUnavailable,
    SettingsRollbackFailed,
}

/// Saves first, then applies the one OS-facing setting that can fail. A failed
/// side effect rolls the persisted file back, so the saved state never claims
/// something the OS refused.
fn persist_and_apply_settings<SaveSettings, SetAutostart>(
    previous: &Settings,
    settings: &Settings,
    mut save_settings: SaveSettings,
    mut set_autostart: SetAutostart,
) -> Result<(), IpcError>
where
    SaveSettings: FnMut(&Settings) -> io::Result<()>,
    SetAutostart: FnMut(bool) -> Result<(), ()>,
{
    save_settings(settings).map_err(|error| {
        if error.kind() == io::ErrorKind::InvalidData {
            IpcError::InvalidSettings
        } else {
            IpcError::PersistenceUnavailable
        }
    })?;

    if previous.start_at_login != settings.start_at_login
        && set_autostart(settings.start_at_login).is_err()
    {
        if save_settings(previous).is_err() {
            eprintln!("failed to restore persisted settings after settings update failure");
            return Err(IpcError::SettingsRollbackFailed);
        }
        return Err(IpcError::ServiceUnavailable);
    }

    Ok(())
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

#[cfg(feature = "webdriver")]
#[tauri::command]
pub fn get_window_states(app: AppHandle) -> Result<Vec<WindowStateDto>, IpcError> {
    Ok(["overlay", "panel"]
        .into_iter()
        .filter_map(|label| {
            app.get_webview_window(label).map(|window| WindowStateDto {
                label: label.to_string(),
                is_visible: window.is_visible().unwrap_or(false),
            })
        })
        .collect())
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
pub fn list_pet_packages(
    window: tauri::WebviewWindow,
    pets: State<'_, PetPackageRepository>,
) -> Result<Vec<PetSummary>, IpcError> {
    authorize(&window, NativeCommand::ListPetPackages)?;
    pets.list().map_err(|_| IpcError::PersistenceUnavailable)
}

#[tauri::command]
pub fn get_platform_capabilities(
    window: tauri::WebviewWindow,
    hotkey: State<'_, HideShowHotkeyCapability>,
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
        hide_show_hotkey: hotkey.0.clone(),
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
    use tauri_plugin_autostart::ManagerExt;

    let autostart = app.autolaunch();
    persist_and_apply_settings(
        &previous,
        &settings,
        |value| repository.save(value),
        |enabled| {
            let result = if enabled {
                autostart.enable()
            } else {
                autostart.disable()
            };
            result.map_err(|_| ())
        },
    )?;
    // Persistence and OS integrations are committed at this point. Reporting
    // an event-delivery failure as a save failure would invite the caller to
    // retry a transaction that already succeeded.
    if app.emit("settings-updated", &settings).is_err() {
        eprintln!("failed to emit settings-updated after settings were committed");
    }
    Ok(settings)
}

/// Brings the panel to the front of the window stack.
///
/// Every path that reveals the panel goes through here so a double-click on the
/// pet always has a visible effect.
fn reveal_panel(panel: &tauri::WebviewWindow) -> Result<(), IpcError> {
    // A minimised panel still reports `is_visible() == true`, so restoring it has
    // to happen before the raise or the focus lands on nothing.
    if panel.is_minimized().unwrap_or(false) {
        let _ = panel.unminimize();
    }
    panel.show().map_err(|_| IpcError::PanelUnavailable)?;
    // Best-effort, mirroring `position_panel`: headless runners and restrictive
    // Wayland compositors refuse focus changes, and promoting that to an error
    // would fail the fixture jobs in native-smoke.yml for a panel that is in fact
    // on screen.
    let _ = panel.set_focus();
    Ok(())
}

/// Cancels any pending reveal, then hides the panel.
///
/// Every in-process path that hides the panel goes through here, so the
/// double-click and the `✕` cannot leave the gate in different states.
fn conceal_panel(panel: &tauri::WebviewWindow, gate: &PanelLayoutGate) -> Result<(), IpcError> {
    gate.disarm();
    panel.hide().map_err(|_| IpcError::PanelUnavailable)
}

/// Anchors the panel beside the pet and arms the layout gate.
///
/// The panel is placed before it is revealed even though the renderer may
/// resize it in a moment: the grace timer below can reveal the panel without a
/// second placement pass.
fn begin_reveal(
    anchor: &tauri::WebviewWindow,
    panel: &tauri::WebviewWindow,
    app: &AppHandle,
    gate: &PanelLayoutGate,
) -> Result<(), IpcError> {
    position_panel(anchor, panel)?;
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
                let _ = reveal_panel(&panel);
            }
        }
    });
    Ok(())
}

/// Toggles the usage panel from the pet's double-click.
///
/// The decision is made here rather than in the renderer because the panel's
/// visibility is changed by paths the renderer never sees: the `✕`, the
/// hide/show hotkey, and the fullscreen monitor.
#[tauri::command]
pub fn toggle_panel(
    window: tauri::WebviewWindow,
    app: AppHandle,
    gate: State<'_, PanelLayoutGate>,
) -> Result<PanelVisibility, IpcError> {
    authorize(&window, NativeCommand::TogglePanel)?;
    let panel = app
        .get_webview_window("panel")
        .ok_or(IpcError::PanelUnavailable)?;
    match panel_toggle(
        panel.is_visible().unwrap_or(false),
        gate.awaiting_layout.load(Ordering::SeqCst),
    ) {
        PanelToggle::Hide => {
            conceal_panel(&panel, gate.inner())?;
            Ok(PanelVisibility::Hidden)
        }
        PanelToggle::Show => {
            begin_reveal(&window, &panel, &app, gate.inner())?;
            Ok(PanelVisibility::Shown)
        }
    }
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
    // The measurement this resize carries is what toggle_panel was waiting for.
    if gate.awaiting_layout.swap(false, Ordering::SeqCst) {
        reveal_panel(&panel)?;
    }
    Ok(())
}

#[tauri::command]
pub fn hide_panel(
    window: tauri::WebviewWindow,
    gate: State<'_, PanelLayoutGate>,
) -> Result<(), IpcError> {
    authorize(&window, NativeCommand::HidePanel)?;
    conceal_panel(&window, gate.inner())
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

#[cfg(test)]
mod settings_effect_tests {
    use std::cell::RefCell;
    use std::io;

    use super::{persist_and_apply_settings, IpcError, Settings};

    fn settings(start_at_login: bool) -> Settings {
        Settings {
            start_at_login,
            ..Settings::default()
        }
    }

    #[test]
    fn an_unchanged_autostart_value_is_never_reapplied() {
        let previous = settings(true);
        let next = Settings {
            bubble_enabled: false,
            ..settings(true)
        };
        let autostart_calls = RefCell::new(0);

        let result = persist_and_apply_settings(
            &previous,
            &next,
            |_| Ok(()),
            |_| {
                *autostart_calls.borrow_mut() += 1;
                Ok(())
            },
        );

        assert_eq!(result, Ok(()));
        assert_eq!(*autostart_calls.borrow(), 0);
    }

    #[test]
    fn failed_side_effect_restores_previous_persisted_settings() {
        let previous = settings(false);
        let next = settings(true);
        let saved = RefCell::new(Vec::new());

        let result = persist_and_apply_settings(
            &previous,
            &next,
            |value| {
                saved.borrow_mut().push(value.clone());
                Ok(())
            },
            |_| Err(()),
        );

        assert_eq!(result, Err(IpcError::ServiceUnavailable));
        assert_eq!(&*saved.borrow(), &[next, previous]);
    }

    #[test]
    fn persistence_compensation_failure_is_reported_as_rollback_failure() {
        let previous = settings(false);
        let next = settings(true);
        let save_calls = RefCell::new(0);

        let result = persist_and_apply_settings(
            &previous,
            &next,
            |_| {
                let mut calls = save_calls.borrow_mut();
                *calls += 1;
                if *calls == 1 {
                    Ok(())
                } else {
                    Err(io::Error::other("synthetic rollback failure"))
                }
            },
            |_| Err(()),
        );

        assert_eq!(result, Err(IpcError::SettingsRollbackFailed));
        assert_eq!(*save_calls.borrow(), 2);
    }
}
