use super::*;

fn display(id: &str, x: f64, y: f64, width: f64, height: f64, scale: f64) -> Display {
    Display {
        id: id.into(),
        bounds: Rect {
            x,
            y,
            width,
            height,
        },
        scale_factor: scale,
    }
}

#[test]
fn converts_logical_and_physical_coordinates_for_dpi() {
    let logical = Point { x: -120.0, y: 45.0 };
    let physical = logical_to_physical(logical, 1.5).unwrap();
    assert_eq!(physical, Point { x: -180.0, y: 67.5 });
    assert_eq!(physical_to_logical(physical, 1.5).unwrap(), logical);
    assert!(logical_to_physical(logical, 0.0).is_err());
}

#[test]
fn clamps_negative_coordinates_and_recovers_from_removed_display() {
    let displays = vec![
        display("left", -1920.0, 0.0, 1920.0, 1080.0, 1.0),
        display("main", 0.0, 0.0, 1920.0, 1080.0, 1.0),
    ];
    assert_eq!(
        clamp_window(
            Point {
                x: -2000.0,
                y: -20.0
            },
            Size {
                width: 200.0,
                height: 100.0
            },
            &displays
        )
        .unwrap(),
        Point { x: -1920.0, y: 0.0 }
    );
    let remaining = vec![displays[1].clone()];
    assert_eq!(
        clamp_window(
            Point {
                x: -1600.0,
                y: 300.0
            },
            Size {
                width: 200.0,
                height: 100.0
            },
            &remaining
        )
        .unwrap(),
        Point { x: 0.0, y: 300.0 }
    );
}

#[test]
fn chooses_nearest_display_and_handles_oversized_windows() {
    let displays = vec![
        display("a", 0.0, 0.0, 100.0, 100.0, 1.0),
        display("b", 300.0, 0.0, 100.0, 100.0, 2.0),
    ];
    assert_eq!(
        nearest_display(Point { x: 260.0, y: 50.0 }, &displays)
            .unwrap()
            .id,
        "b"
    );
    assert_eq!(
        clamp_window(
            Point { x: 350.0, y: 20.0 },
            Size {
                width: 200.0,
                height: 150.0
            },
            &displays
        )
        .unwrap(),
        Point { x: 300.0, y: 0.0 }
    );
}

#[test]
fn panel_anchor_flips_then_clamps_inside_display() {
    let bounds = Rect {
        x: 0.0,
        y: 0.0,
        width: 800.0,
        height: 600.0,
    };
    let pet = Rect {
        x: 700.0,
        y: 550.0,
        width: 80.0,
        height: 60.0,
    };
    assert_eq!(
        anchor_panel(
            pet,
            Size {
                width: 300.0,
                height: 240.0
            },
            bounds,
            8.0
        ),
        Point { x: 392.0, y: 360.0 }
    );
}

#[test]
fn fullscreen_changes_visibility_without_touching_collection_revision() {
    let state = RuntimeState {
        collection_revision: 42,
        fullscreen: false,
        overlay_visible: true,
        panel_visible: true,
    };
    let hidden = apply_fullscreen(&state, true);
    assert_eq!(
        hidden,
        RuntimeState {
            collection_revision: 42,
            fullscreen: true,
            overlay_visible: false,
            panel_visible: false
        }
    );
    assert_eq!(state.collection_revision, 42);
}

#[test]
fn fullscreen_rect_must_cover_the_monitor_in_both_dimensions() {
    let monitor = Rect {
        x: 0.0,
        y: 0.0,
        width: 1920.0,
        height: 1080.0,
    };
    assert!(super::rect_covers_monitor(monitor, monitor));
    assert!(!super::rect_covers_monitor(
        Rect {
            width: 1920.0,
            height: 1040.0,
            ..monitor
        },
        monitor,
    ));
}

#[test]
fn autostart_is_idempotent_and_reports_visible_degradation() {
    let mut adapter = FakeAutostart {
        supported: true,
        ..Default::default()
    };
    assert_eq!(
        set_autostart(&mut adapter, true).unwrap(),
        CapabilityDiagnostic::Available
    );
    assert_eq!(
        set_autostart(&mut adapter, true).unwrap(),
        CapabilityDiagnostic::Available
    );
    assert_eq!(adapter.enable_calls, 1);
    assert_eq!(
        set_autostart(&mut adapter, false).unwrap(),
        CapabilityDiagnostic::Available
    );
    assert_eq!(adapter.enable_calls, 2);
    let mut unsupported = FakeAutostart::default();
    assert!(matches!(
        set_autostart(&mut unsupported, true).unwrap(),
        CapabilityDiagnostic::Unavailable { .. }
    ));
    assert_eq!(unsupported.enable_calls, 0);
    let capabilities = PlatformCapabilities::linux_wayland(false, false);
    assert!(matches!(
        capabilities.always_on_top,
        CapabilityDiagnostic::Unavailable { .. }
    ));
    assert!(matches!(
        capabilities.fullscreen_detection,
        CapabilityDiagnostic::Unavailable { .. }
    ));
}

#[test]
fn native_commands_are_authorized_by_window_label() {
    assert!(command_allowed("overlay", NativeCommand::GetCollectorMode));
    assert!(command_allowed("overlay", NativeCommand::ShowPanel));
    assert!(!command_allowed("overlay", NativeCommand::ResizePanel));
    assert!(command_allowed("overlay", NativeCommand::GetProviderStates));
    assert!(command_allowed("overlay", NativeCommand::GetSettings));
    assert!(!command_allowed("overlay", NativeCommand::GetHistory));
    assert!(command_allowed("overlay", NativeCommand::GetPetPackage));
    assert!(command_allowed(
        "overlay",
        NativeCommand::GetPlatformCapabilities
    ));
    assert!(command_allowed("overlay", NativeCommand::SavePosition));
    assert!(!command_allowed("overlay", NativeCommand::RefreshProvider));
    assert!(!command_allowed("overlay", NativeCommand::UpdateSettings));
    assert!(!command_allowed("overlay", NativeCommand::HidePanel));
    assert!(!command_allowed("overlay", NativeCommand::Quit));
    assert!(command_allowed("panel", NativeCommand::UpdateSettings));
    assert!(command_allowed("panel", NativeCommand::GetCollectorMode));
    assert!(command_allowed("panel", NativeCommand::Quit));
    assert!(command_allowed("panel", NativeCommand::HidePanel));
    assert!(command_allowed("panel", NativeCommand::ResizePanel));
    assert!(!command_allowed("panel", NativeCommand::SavePosition));
    assert!(!command_allowed("unknown", NativeCommand::ShowPanel));
}

#[test]
fn normalizes_supported_platform_names_and_falls_back_to_linux() {
    assert_eq!(super::platform_os("macos"), "macos");
    assert_eq!(super::platform_os("windows"), "windows");
    assert_eq!(super::platform_os("linux"), "linux");
    assert_eq!(super::platform_os("freebsd"), "linux");
}

#[test]
fn controller_recovers_position_anchors_panel_and_hides_only_presentation() {
    let mut adapter = FakePlatform {
        displays: vec![display("main", 0.0, 0.0, 800.0, 600.0, 1.0)],
        fullscreen: true,
        ..Default::default()
    };
    let recovered = recover_position(
        &adapter,
        Point {
            x: -500.0,
            y: 700.0,
        },
        Size {
            width: 100.0,
            height: 100.0,
        },
    )
    .unwrap();
    assert_eq!(recovered, Point { x: 0.0, y: 500.0 });
    assert_eq!(
        panel_position(
            &adapter,
            Rect {
                x: 700.0,
                y: 550.0,
                width: 80.0,
                height: 60.0
            },
            Size {
                width: 300.0,
                height: 240.0
            },
            8.0,
        )
        .unwrap(),
        Point { x: 392.0, y: 360.0 }
    );
    let state = RuntimeState {
        collection_revision: 9,
        fullscreen: false,
        overlay_visible: true,
        panel_visible: true,
    };
    let hidden = synchronize_fullscreen(&mut adapter, &state).unwrap();
    assert_eq!(hidden.collection_revision, 9);
    assert_eq!(
        adapter.commands,
        vec![WindowCommand::HidePet, WindowCommand::HidePanel]
    );
    assert!(!hidden.overlay_visible);
    assert!(!hidden.panel_visible);
}

#[derive(Default)]
struct FakePlatform {
    displays: Vec<Display>,
    fullscreen: bool,
    commands: Vec<WindowCommand>,
}

impl PlatformWindowAdapter for FakePlatform {
    fn execute(&mut self, command: WindowCommand) -> Result<CapabilityDiagnostic, PlatformError> {
        self.commands.push(command);
        Ok(CapabilityDiagnostic::Available)
    }
    fn displays(&self) -> Result<Vec<Display>, PlatformError> {
        Ok(self.displays.clone())
    }
    fn fullscreen_active(&self) -> Result<bool, PlatformError> {
        Ok(self.fullscreen)
    }
}

#[derive(Default)]
struct FakeAutostart {
    enabled: bool,
    enable_calls: usize,
    supported: bool,
}

impl AutostartAdapter for FakeAutostart {
    fn capability(&self) -> CapabilityDiagnostic {
        if self.supported {
            CapabilityDiagnostic::Available
        } else {
            CapabilityDiagnostic::Unavailable {
                reason: "autostart unavailable",
            }
        }
    }
    fn is_enabled(&self) -> Result<bool, PlatformError> {
        Ok(self.enabled)
    }
    fn set_enabled(&mut self, enabled: bool) -> Result<(), PlatformError> {
        self.enabled = enabled;
        self.enable_calls += 1;
        Ok(())
    }
}
