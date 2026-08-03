use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct Size {
    pub width: f64,
    pub height: f64,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Display {
    pub id: String,
    pub bounds: Rect,
    pub scale_factor: f64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum CapabilityDiagnostic {
    Available,
    Unavailable { reason: &'static str },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct PlatformCapabilities {
    pub os: &'static str,
    pub always_on_top: CapabilityDiagnostic,
    pub fullscreen_detection: CapabilityDiagnostic,
    pub autostart: CapabilityDiagnostic,
    pub hide_show_hotkey: CapabilityDiagnostic,
}

/// The startup registration result for [`DEFAULT_HIDE_SHOW_HOTKEY`], managed so
/// `get_platform_capabilities` can report a conflict without the settings file
/// ever recording one.
pub struct HideShowHotkeyCapability(pub CapabilityDiagnostic);

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(tag = "command", content = "enabled", rename_all = "snake_case")]
pub enum WindowCommand {
    ShowPet,
    HidePet,
    ShowPanel,
    HidePanel,
    SetAlwaysOnTop(bool),
    SetTaskbarVisibility(bool),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NativeCommand {
    GetCollectorMode,
    GetProviderStates,
    GetSettings,
    GetHistory,
    GetPetPackage,
    ListPetPackages,
    GetPlatformCapabilities,
    SavePosition,
    RefreshProvider,
    UpdateSettings,
    TogglePanel,
    ResizePanel,
    HidePanel,
    Quit,
}

pub fn command_allowed(window_label: &str, command: NativeCommand) -> bool {
    match window_label {
        "overlay" => matches!(
            command,
            NativeCommand::GetCollectorMode
                | NativeCommand::GetProviderStates
                | NativeCommand::GetSettings
                | NativeCommand::GetPetPackage
                | NativeCommand::GetPlatformCapabilities
                | NativeCommand::SavePosition
                | NativeCommand::TogglePanel
        ),
        "panel" => matches!(
            command,
            NativeCommand::GetCollectorMode
                | NativeCommand::GetProviderStates
                | NativeCommand::GetSettings
                | NativeCommand::GetHistory
                | NativeCommand::GetPetPackage
                | NativeCommand::ListPetPackages
                | NativeCommand::GetPlatformCapabilities
                | NativeCommand::RefreshProvider
                | NativeCommand::UpdateSettings
                | NativeCommand::ResizePanel
                | NativeCommand::HidePanel
                | NativeCommand::Quit
        ),
        _ => false,
    }
}

/// What a pet double-click must do for a panel in the given state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PanelToggle {
    /// On screen, or on its way there: hide it. CacheBite keeps running and
    /// keeps polling.
    Hide,
    /// Off screen: anchor it and arm the layout gate, so the reveal waits for
    /// the renderer to measure its content height.
    Show,
}

/// A reveal that is still waiting for the renderer counts as visible.
///
/// `show()` is deferred until the renderer reports its height, or until the
/// grace timer fires, so for a moment the panel reports `is_visible() == false`
/// while already being on its way to the screen. Reading visibility alone would
/// make a second double-click inside that window arm the reveal a second time
/// instead of cancelling it — the panel would appear right after the gesture
/// meant to dismiss it.
pub fn panel_toggle(visible: bool, reveal_pending: bool) -> PanelToggle {
    if visible || reveal_pending {
        PanelToggle::Hide
    } else {
        PanelToggle::Show
    }
}

/// The one hide/show combination CacheBite claims. `CommandOrControl` is Tauri's
/// cross-platform token: Cmd on macOS, Ctrl on Windows and Linux.
///
/// It is deliberately not user-configurable. A stored binding could record
/// itself as disabled, which is how a single failed registration used to become
/// permanent — the settings file kept a `null` no migration would ever undo.
pub const DEFAULT_HIDE_SHOW_HOTKEY: &str = "CommandOrControl+Shift+H";

/// Maps a global-shortcut registration outcome to the capability the settings
/// panel reports.
///
/// Split from the Tauri call so both branches are covered by tests. The call
/// site is left as a single expression with no conditional of its own, which is
/// what keeps an inverted or missing mapping from being introduced there.
pub fn hide_show_hotkey_capability<E>(registration: Result<(), E>) -> CapabilityDiagnostic {
    if registration.is_err() {
        eprintln!("failed to register the hide/show shortcut; another application may own it");
    }
    capability(
        registration.is_ok(),
        "another application already owns this shortcut",
    )
}

/// Whether the overlay should be shown again when fullscreen ends. `false` when
/// the user explicitly hid it via the hide/show hotkey — fullscreen exiting must
/// not silently reverse that.
pub fn should_restore_overlay_after_fullscreen(user_hidden: bool) -> bool {
    !user_hidden
}

pub trait PlatformWindowAdapter {
    fn execute(&mut self, command: WindowCommand) -> Result<CapabilityDiagnostic, PlatformError>;
    fn displays(&self) -> Result<Vec<Display>, PlatformError>;
    fn fullscreen_active(&self) -> Result<bool, PlatformError>;
}

impl PlatformCapabilities {
    pub fn linux_wayland(
        always_on_top: bool,
        fullscreen_detection: bool,
        hide_show_hotkey: bool,
    ) -> Self {
        Self {
            os: "linux",
            always_on_top: capability(always_on_top, "compositor does not permit always-on-top"),
            fullscreen_detection: capability(
                fullscreen_detection,
                "compositor does not expose fullscreen detection",
            ),
            autostart: CapabilityDiagnostic::Available,
            hide_show_hotkey: capability(
                hide_show_hotkey,
                "compositor does not permit a global shortcut",
            ),
        }
    }
}

pub(crate) fn platform_os(os: &str) -> &'static str {
    match os {
        "macos" => "macos",
        "windows" => "windows",
        "linux" => "linux",
        _ => "linux",
    }
}

fn capability(available: bool, reason: &'static str) -> CapabilityDiagnostic {
    if available {
        CapabilityDiagnostic::Available
    } else {
        CapabilityDiagnostic::Unavailable { reason }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeState {
    pub collection_revision: u64,
    pub fullscreen: bool,
    pub overlay_visible: bool,
    pub panel_visible: bool,
}

pub fn apply_fullscreen(state: &RuntimeState, fullscreen: bool) -> RuntimeState {
    RuntimeState {
        fullscreen,
        overlay_visible: !fullscreen,
        panel_visible: if fullscreen {
            false
        } else {
            state.panel_visible
        },
        ..state.clone()
    }
}

pub fn rect_covers_monitor(window: Rect, monitor: Rect) -> bool {
    const TOLERANCE: f64 = 2.0;
    (window.x - monitor.x).abs() <= TOLERANCE
        && (window.y - monitor.y).abs() <= TOLERANCE
        && (window.width - monitor.width).abs() <= TOLERANCE
        && (window.height - monitor.height).abs() <= TOLERANCE
}

#[cfg(windows)]
pub fn foreground_window_is_fullscreen() -> bool {
    use core::ffi::c_void;

    #[repr(C)]
    #[derive(Clone, Copy, Default)]
    struct WinRect {
        left: i32,
        top: i32,
        right: i32,
        bottom: i32,
    }
    #[repr(C)]
    #[derive(Default)]
    struct MonitorInfo {
        size: u32,
        monitor: WinRect,
        work: WinRect,
        flags: u32,
    }
    #[link(name = "user32")]
    extern "system" {
        fn GetForegroundWindow() -> *mut c_void;
        fn GetShellWindow() -> *mut c_void;
        fn IsWindowVisible(window: *mut c_void) -> i32;
        fn GetWindowRect(window: *mut c_void, rect: *mut WinRect) -> i32;
        fn MonitorFromWindow(window: *mut c_void, flags: u32) -> *mut c_void;
        fn GetMonitorInfoW(monitor: *mut c_void, info: *mut MonitorInfo) -> i32;
    }

    const MONITOR_DEFAULT_TO_NEAREST: u32 = 2;
    let (window, monitor) = unsafe {
        let handle = GetForegroundWindow();
        if handle.is_null() || handle == GetShellWindow() || IsWindowVisible(handle) == 0 {
            return false;
        }
        let mut window = WinRect::default();
        if GetWindowRect(handle, &mut window) == 0 {
            return false;
        }
        let monitor_handle = MonitorFromWindow(handle, MONITOR_DEFAULT_TO_NEAREST);
        let mut monitor = MonitorInfo {
            size: std::mem::size_of::<MonitorInfo>() as u32,
            ..MonitorInfo::default()
        };
        if monitor_handle.is_null() || GetMonitorInfoW(monitor_handle, &mut monitor) == 0 {
            return false;
        }
        (window, monitor.monitor)
    };
    let rect = |value: WinRect| Rect {
        x: f64::from(value.left),
        y: f64::from(value.top),
        width: f64::from(value.right - value.left),
        height: f64::from(value.bottom - value.top),
    };
    rect_covers_monitor(rect(window), rect(monitor))
}

pub fn synchronize_fullscreen(
    adapter: &mut impl PlatformWindowAdapter,
    state: &RuntimeState,
) -> Result<RuntimeState, PlatformError> {
    let fullscreen = adapter.fullscreen_active()?;
    let next = apply_fullscreen(state, fullscreen);
    if fullscreen {
        adapter.execute(WindowCommand::HidePet)?;
        adapter.execute(WindowCommand::HidePanel)?;
    } else if state.fullscreen {
        adapter.execute(WindowCommand::ShowPet)?;
    }
    Ok(next)
}

pub fn recover_position(
    adapter: &impl PlatformWindowAdapter,
    saved: Point,
    size: Size,
) -> Result<Point, PlatformError> {
    clamp_window(saved, size, &adapter.displays()?).ok_or(PlatformError::OperationFailed)
}

pub fn panel_position(
    adapter: &impl PlatformWindowAdapter,
    pet: Rect,
    panel: Size,
    gap: f64,
) -> Result<Point, PlatformError> {
    let displays = adapter.displays()?;
    let display = nearest_display(Point { x: pet.x, y: pet.y }, &displays)
        .ok_or(PlatformError::OperationFailed)?;
    Ok(anchor_panel(pet, panel, display.bounds, gap))
}

#[derive(Debug, thiserror::Error)]
pub enum PlatformError {
    #[error("platform operation failed")]
    OperationFailed,
}

pub trait AutostartAdapter {
    fn capability(&self) -> CapabilityDiagnostic;
    fn is_enabled(&self) -> Result<bool, PlatformError>;
    fn set_enabled(&mut self, enabled: bool) -> Result<(), PlatformError>;
}

pub struct UnsupportedAutostart;

impl AutostartAdapter for UnsupportedAutostart {
    fn capability(&self) -> CapabilityDiagnostic {
        CapabilityDiagnostic::Unavailable {
            reason: "autostart integration is unavailable on this build",
        }
    }
    fn is_enabled(&self) -> Result<bool, PlatformError> {
        Ok(false)
    }
    fn set_enabled(&mut self, _enabled: bool) -> Result<(), PlatformError> {
        Err(PlatformError::OperationFailed)
    }
}

pub fn set_autostart(
    adapter: &mut impl AutostartAdapter,
    enabled: bool,
) -> Result<CapabilityDiagnostic, PlatformError> {
    let capability = adapter.capability();
    if !matches!(capability, CapabilityDiagnostic::Available) {
        return Ok(capability);
    }
    if adapter.is_enabled()? != enabled {
        adapter.set_enabled(enabled)?;
    }
    Ok(CapabilityDiagnostic::Available)
}

pub fn logical_to_physical(point: Point, scale_factor: f64) -> Result<Point, PlatformError> {
    valid_scale(scale_factor)?;
    valid_point(point)?;
    Ok(Point {
        x: point.x * scale_factor,
        y: point.y * scale_factor,
    })
}

pub fn physical_to_logical(point: Point, scale_factor: f64) -> Result<Point, PlatformError> {
    valid_scale(scale_factor)?;
    valid_point(point)?;
    Ok(Point {
        x: point.x / scale_factor,
        y: point.y / scale_factor,
    })
}

fn valid_scale(scale_factor: f64) -> Result<(), PlatformError> {
    if scale_factor.is_finite() && scale_factor > 0.0 {
        Ok(())
    } else {
        Err(PlatformError::OperationFailed)
    }
}

fn valid_point(point: Point) -> Result<(), PlatformError> {
    if point.x.is_finite() && point.y.is_finite() {
        Ok(())
    } else {
        Err(PlatformError::OperationFailed)
    }
}

pub fn nearest_display(point: Point, displays: &[Display]) -> Option<&Display> {
    displays
        .iter()
        .filter(|display| valid_rect(display.bounds))
        .min_by(|a, b| {
            distance_squared(point, a.bounds).total_cmp(&distance_squared(point, b.bounds))
        })
}

pub fn clamp_window(position: Point, size: Size, displays: &[Display]) -> Option<Point> {
    let display = nearest_display(position, displays)?;
    Some(clamp_to_rect(position, size, display.bounds))
}

/// Anchors the panel beside the pet.
///
/// Horizontally the panel sits to the right of the pet, flipping to the left
/// when it would overflow. Vertically it centres on the pet: top-aligning
/// pushes the panel past the bottom edge for a pet parked low on the screen,
/// which clamping can only resolve by pinning it flush against that edge.
///
/// `work_area` must be the usable region — the monitor minus taskbar, dock and
/// menu bar — not the full resolution. The result is clamped inside it.
pub fn anchor_panel(pet: Rect, panel: Size, work_area: Rect, gap: f64) -> Point {
    let right = pet.x + pet.width + gap;
    let x = if right + panel.width <= work_area.x + work_area.width {
        right
    } else {
        pet.x - gap - panel.width
    };
    let y = pet.y + (pet.height - panel.height) / 2.0;
    clamp_to_rect(Point { x, y }, panel, work_area)
}

fn clamp_to_rect(position: Point, size: Size, bounds: Rect) -> Point {
    let maximum_x = (bounds.x + bounds.width - size.width).max(bounds.x);
    let maximum_y = (bounds.y + bounds.height - size.height).max(bounds.y);
    Point {
        x: position.x.clamp(bounds.x, maximum_x),
        y: position.y.clamp(bounds.y, maximum_y),
    }
}

fn distance_squared(point: Point, rect: Rect) -> f64 {
    let dx = if point.x < rect.x {
        rect.x - point.x
    } else if point.x > rect.x + rect.width {
        point.x - rect.x - rect.width
    } else {
        0.0
    };
    let dy = if point.y < rect.y {
        rect.y - point.y
    } else if point.y > rect.y + rect.height {
        point.y - rect.y - rect.height
    } else {
        0.0
    };
    dx * dx + dy * dy
}

fn valid_rect(rect: Rect) -> bool {
    rect.x.is_finite()
        && rect.y.is_finite()
        && rect.width.is_finite()
        && rect.height.is_finite()
        && rect.width > 0.0
        && rect.height > 0.0
}

#[cfg(test)]
mod tests;
