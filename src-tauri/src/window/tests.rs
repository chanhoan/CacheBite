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

/// 1920x1080 with a 48px taskbar leaves a 1032px work area. A 312x520 panel
/// anchored to a pet parked at the bottom must not slide under the taskbar.
#[test]
fn panel_anchor_keeps_panel_above_taskbar_for_bottom_pet() {
    let work_area = Rect {
        x: 0.0,
        y: 0.0,
        width: 1920.0,
        height: 1032.0,
    };
    let pet = Rect {
        x: 100.0,
        y: 800.0,
        width: 240.0,
        height: 240.0,
    };
    let panel = Size {
        width: 312.0,
        height: 520.0,
    };

    let anchored = anchor_panel(pet, panel, work_area, 12.0);

    assert_eq!(anchored, Point { x: 352.0, y: 512.0 });
    assert!(anchored.y + panel.height <= work_area.y + work_area.height);
}

/// With room to spare the panel centres on the pet and is never clamped.
/// Top-aligning would yield y=400 here, so this pins down which policy is live.
#[test]
fn panel_anchor_centers_vertically_on_pet_when_space_allows() {
    let work_area = Rect {
        x: 0.0,
        y: 0.0,
        width: 1920.0,
        height: 1032.0,
    };
    let pet = Rect {
        x: 800.0,
        y: 400.0,
        width: 240.0,
        height: 240.0,
    };
    assert_eq!(
        anchor_panel(
            pet,
            Size {
                width: 312.0,
                height: 520.0
            },
            work_area,
            12.0
        ),
        Point {
            x: 1052.0,
            y: 260.0
        }
    );
}

/// A pet near the top centres above the work area, so the clamp pulls it down
/// to the first usable row — here a macOS menu bar starting the area at y=25.
#[test]
fn panel_anchor_clamps_to_work_area_top_for_top_pet() {
    let work_area = Rect {
        x: 0.0,
        y: 25.0,
        width: 1440.0,
        height: 875.0,
    };
    let pet = Rect {
        x: 40.0,
        y: 30.0,
        width: 240.0,
        height: 240.0,
    };
    assert_eq!(
        anchor_panel(
            pet,
            Size {
                width: 312.0,
                height: 520.0
            },
            work_area,
            12.0
        ),
        Point { x: 292.0, y: 25.0 }
    );
}

/// A panel taller than the work area pins to the top instead of overflowing
/// past the bottom, keeping its header reachable.
#[test]
fn panel_anchor_pins_oversized_panel_to_work_area_top() {
    let work_area = Rect {
        x: 0.0,
        y: 0.0,
        width: 1920.0,
        height: 1032.0,
    };
    let pet = Rect {
        x: 100.0,
        y: 800.0,
        width: 240.0,
        height: 240.0,
    };
    assert_eq!(
        anchor_panel(
            pet,
            Size {
                width: 312.0,
                height: 1200.0
            },
            work_area,
            12.0
        ),
        Point { x: 352.0, y: 0.0 }
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
    let capabilities = PlatformCapabilities::linux_wayland(false, false, false);
    assert!(matches!(
        capabilities.always_on_top,
        CapabilityDiagnostic::Unavailable { .. }
    ));
    assert!(matches!(
        capabilities.fullscreen_detection,
        CapabilityDiagnostic::Unavailable { .. }
    ));
    assert!(matches!(
        capabilities.hide_show_hotkey,
        CapabilityDiagnostic::Unavailable { .. }
    ));
    assert_eq!(
        PlatformCapabilities::linux_wayland(true, true, true).hide_show_hotkey,
        CapabilityDiagnostic::Available
    );
}

/// The startup registration path reports through this mapping, so an inverted
/// or dropped branch here would tell the panel a claimed shortcut is live.
#[test]
fn hide_show_hotkey_capability_reports_both_registration_outcomes() {
    assert_eq!(
        hide_show_hotkey_capability(Ok::<(), ()>(())),
        CapabilityDiagnostic::Available
    );
    assert!(matches!(
        hide_show_hotkey_capability(Err::<(), ()>(())),
        CapabilityDiagnostic::Unavailable { .. }
    ));
}

#[test]
fn native_commands_are_authorized_by_window_label() {
    assert!(command_allowed("overlay", NativeCommand::GetCollectorMode));
    assert!(command_allowed("overlay", NativeCommand::TogglePanel));
    assert!(!command_allowed("overlay", NativeCommand::ResizePanel));
    assert!(command_allowed("overlay", NativeCommand::GetProviderStates));
    assert!(command_allowed("overlay", NativeCommand::GetSettings));
    assert!(!command_allowed("overlay", NativeCommand::GetHistory));
    assert!(command_allowed("overlay", NativeCommand::GetPetPackage));
    // The pet picker lives in the panel; the overlay has no use for the list.
    assert!(!command_allowed("overlay", NativeCommand::ListPetPackages));
    assert!(command_allowed("panel", NativeCommand::ListPetPackages));
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
    // The pet gesture is the overlay's; the panel dismisses itself with `hide_panel`.
    assert!(!command_allowed("panel", NativeCommand::TogglePanel));
    assert!(!command_allowed("unknown", NativeCommand::TogglePanel));
    // The context menu is the pet's gesture; item actions run in Rust, so the
    // overlay still cannot call `quit` or the other panel-only commands.
    assert!(command_allowed("overlay", NativeCommand::ShowPetMenu));
    assert!(!command_allowed("panel", NativeCommand::ShowPetMenu));
    assert!(!command_allowed("unknown", NativeCommand::ShowPetMenu));
}

#[test]
fn pet_menu_ids_map_to_their_actions_and_unknown_ids_to_none() {
    assert_eq!(
        pet_menu_action(PET_MENU_TOGGLE_PANEL_ID),
        Some(PetMenuAction::TogglePanel)
    );
    assert_eq!(
        pet_menu_action(PET_MENU_HIDE_PET_ID),
        Some(PetMenuAction::HidePet)
    );
    assert_eq!(pet_menu_action(PET_MENU_QUIT_ID), Some(PetMenuAction::Quit));
    // Ids from other menus (or future items) must fall through untouched.
    assert_eq!(pet_menu_action("unrelated-menu-item"), None);
}

#[test]
fn pet_menu_panel_label_matches_the_toggle_the_click_performs() {
    assert_eq!(pet_menu_panel_label(PanelToggle::Hide), "Hide usage panel");
    assert_eq!(pet_menu_panel_label(PanelToggle::Show), "Show usage panel");
}

#[test]
fn a_visible_or_pending_panel_is_hidden_and_a_hidden_one_is_shown() {
    assert_eq!(panel_toggle(false, false), PanelToggle::Show);
    assert_eq!(panel_toggle(true, false), PanelToggle::Hide);
    // Rapid double-clicks: the second one cancels the reveal the first armed
    // rather than arming a second one on a panel that is already on its way.
    assert_eq!(panel_toggle(false, true), PanelToggle::Hide);
    assert_eq!(panel_toggle(true, true), PanelToggle::Hide);
}

#[test]
fn fullscreen_exit_does_not_restore_a_hotkey_hidden_overlay() {
    assert!(should_restore_overlay_after_fullscreen(false));
    assert!(!should_restore_overlay_after_fullscreen(true));
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
