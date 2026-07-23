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
}

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
    GetPlatformCapabilities,
    SavePosition,
    RefreshProvider,
    UpdateSettings,
    ShowPanel,
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
                | NativeCommand::ShowPanel
        ),
        "panel" => matches!(
            command,
            NativeCommand::GetCollectorMode
                | NativeCommand::GetProviderStates
                | NativeCommand::GetSettings
                | NativeCommand::GetHistory
                | NativeCommand::GetPetPackage
                | NativeCommand::GetPlatformCapabilities
                | NativeCommand::RefreshProvider
                | NativeCommand::UpdateSettings
                | NativeCommand::ShowPanel
                | NativeCommand::ResizePanel
                | NativeCommand::HidePanel
                | NativeCommand::Quit
        ),
        _ => false,
    }
}

pub trait PlatformWindowAdapter {
    fn execute(&mut self, command: WindowCommand) -> Result<CapabilityDiagnostic, PlatformError>;
    fn displays(&self) -> Result<Vec<Display>, PlatformError>;
    fn fullscreen_active(&self) -> Result<bool, PlatformError>;
}

impl PlatformCapabilities {
    pub fn linux_wayland(always_on_top: bool, fullscreen_detection: bool) -> Self {
        Self {
            os: "linux",
            always_on_top: capability(always_on_top, "compositor does not permit always-on-top"),
            fullscreen_detection: capability(
                fullscreen_detection,
                "compositor does not expose fullscreen detection",
            ),
            autostart: CapabilityDiagnostic::Available,
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

pub fn anchor_panel(pet: Rect, panel: Size, display: Rect, gap: f64) -> Point {
    let right = pet.x + pet.width + gap;
    let x = if right + panel.width <= display.x + display.width {
        right
    } else {
        pet.x - gap - panel.width
    };
    clamp_to_rect(Point { x, y: pet.y }, panel, display)
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
