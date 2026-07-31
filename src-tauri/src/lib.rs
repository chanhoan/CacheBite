pub mod collectors;
pub mod domain;
pub mod refresh;
pub mod store;
pub mod window;

use std::{fs, io, path::Path};

#[cfg(test)]
mod domain_test;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    use std::{collections::BTreeMap, path::PathBuf, sync::Arc};
    use tauri::Manager;

    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    if event.state == tauri_plugin_global_shortcut::ShortcutState::Pressed {
                        toggle_overlay_visibility(app);
                    }
                })
                .build(),
        );
    #[cfg(feature = "webdriver")]
    let builder = builder.plugin(tauri_plugin_wdio_webdriver::init());

    builder
        .setup(|app| {
            #[cfg(debug_assertions)]
            {
                eprintln!(
                    "[CacheBite:native] setup:start v{}",
                    env!("CARGO_PKG_VERSION")
                );
                if std::env::var_os("CACHEBITE_OPEN_DEVTOOLS").is_some() {
                    if let Some(overlay) = app.get_webview_window("overlay") {
                        overlay.open_devtools();
                    }
                }
            }
            let app_data = app.path().app_data_dir()?;
            if let Err(error) = install_bundled_pet_packages(&app.path().resource_dir()?, &app_data)
            {
                eprintln!("failed to install bundled pet packages: {error}");
            }
            let settings_repository = store::SettingsRepository::new(&app_data);
            // Managed before the hotkey is registered: the global-shortcut
            // plugin is already live at this point (installed on the builder
            // chain), so a hotkey press dispatched to toggle_overlay_visibility
            // before this state exists would panic on app.state::<OverlayHideGate>().
            app.manage(OverlayHideGate::default());
            if let Ok(settings) = settings_repository.load() {
                restore_window_positions(app, &settings);
                if let Some(hotkey) = &settings.hide_show_hotkey {
                    register_startup_hotkey(app.handle(), &settings_repository, hotkey);
                }
            }
            let fixture_mode = std::env::var_os("CACHEBITE_E2E_FIXTURES").is_some();
            let collector_mode = refresh::ipc::CollectorModeDto::for_fixture_gate(fixture_mode);
            let mut environment = BTreeMap::new();
            if let Ok(token) = std::env::var("CLAUDE_CODE_OAUTH_TOKEN") {
                environment.insert("CLAUDE_CODE_OAUTH_TOKEN".to_owned(), token);
            }
            let broker = Arc::new(collectors::broker::CredentialBroker::new(
                environment,
                collectors::broker::CredentialLocations::documented(
                    std::env::var_os("CLAUDE_CONFIG_DIR").map(PathBuf::from),
                    platform_home_dir(),
                ),
            ));
            let (claude, codex): (
                Arc<dyn collectors::Collector>,
                Arc<dyn collectors::Collector>,
            ) = if fixture_mode {
                (
                    Arc::new(collectors::UnavailableFixtureCollector(
                        domain::Provider::Claude,
                    )),
                    Arc::new(collectors::UnavailableFixtureCollector(
                        domain::Provider::Codex,
                    )),
                )
            } else {
                let native_codex = configured_codex_search_path(
                    std::env::var_os("CACHEBITE_CODEX_PATH"),
                    std::env::var_os("PATH"),
                )
                .ok_or(collectors::CollectorError::CliMissing)
                .and_then(|path| collectors::codex::resolve_codex_executable(&path))
                .and_then(collectors::codex::CodexCollector::new);
                production_collectors(broker, native_codex)?
            };
            let snapshots = Arc::new(store::SnapshotRepository::new(&app_data));
            let history = Arc::new(store::HistoryRepository::new(&app_data));
            let persistence = refresh::RefreshPersistence::new(snapshots, history.clone());
            let service = refresh::RefreshService::new(
                refresh::RefreshHandle::spawn_persistent(
                    claude,
                    refresh::SchedulerConfig::default(),
                    persistence.clone(),
                )?,
                refresh::RefreshHandle::spawn_persistent(
                    codex,
                    refresh::SchedulerConfig::default(),
                    persistence,
                )?,
            );
            refresh::ipc::emit_provider_states(app.handle(), &service);
            app.manage(settings_repository);
            // Share the repository the refresh actors already write through
            // instead of constructing a second one over the same file.
            app.manage(history);
            app.manage(store::PetPackageRepository::new(app.path().app_data_dir()?));
            app.manage(service);
            app.manage(collector_mode);
            app.manage(refresh::ipc::PanelLayoutGate::default());
            #[cfg(debug_assertions)]
            eprintln!("[CacheBite:native] setup:ready");
            #[cfg(windows)]
            start_fullscreen_monitor(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            refresh::ipc::get_collector_mode,
            refresh::ipc::get_provider_states,
            refresh::ipc::get_settings,
            refresh::ipc::get_history,
            refresh::ipc::get_pet_package,
            refresh::ipc::list_pet_packages,
            refresh::ipc::get_platform_capabilities,
            refresh::ipc::save_position,
            refresh::ipc::refresh_provider,
            refresh::ipc::update_settings,
            refresh::ipc::show_panel,
            refresh::ipc::resize_panel,
            refresh::ipc::hide_panel,
            refresh::ipc::quit,
        ])
        .run(tauri::generate_context!())
        .expect("failed to run CacheBite");
}

type CollectorPair = (
    std::sync::Arc<dyn collectors::Collector>,
    std::sync::Arc<dyn collectors::Collector>,
);

fn production_collectors(
    broker: std::sync::Arc<dyn collectors::broker::ClaudeTokenSource>,
    native_codex: Result<collectors::codex::CodexCollector, collectors::CollectorError>,
) -> Result<CollectorPair, collectors::fallback::ProviderMismatch> {
    #[cfg(windows)]
    if let Ok(factory) = collectors::wsl::WslCommandFactory::from_system_directory() {
        let claude = collectors::fallback::FallbackCollector::new(
            Box::new(collectors::claude::ClaudeCollector::new(broker)),
            Box::new(collectors::claude::ClaudeCollector::new(
                std::sync::Arc::new(collectors::wsl::WslCredentialSource::new(factory.clone())),
            )),
            collectors::fallback::FallbackTrigger::CredentialsMissing,
        )?;
        let native_codex: Box<dyn collectors::Collector> = match native_codex {
            Ok(collector) => Box::new(collector),
            Err(_) => Box::new(MissingCodexCollector),
        };
        let codex = collectors::fallback::FallbackCollector::new(
            native_codex,
            Box::new(collectors::wsl::WslCodexCollector::new(factory)),
            collectors::fallback::FallbackTrigger::CliMissing,
        )?;
        return Ok((std::sync::Arc::new(claude), std::sync::Arc::new(codex)));
    }

    let codex: std::sync::Arc<dyn collectors::Collector> = match native_codex {
        Ok(collector) => std::sync::Arc::new(collector),
        Err(_) => std::sync::Arc::new(MissingCodexCollector),
    };
    Ok((
        std::sync::Arc::new(collectors::claude::ClaudeCollector::new(broker)),
        codex,
    ))
}

/// Whether the overlay is hidden because the user explicitly toggled it via the
/// hide/show hotkey — as opposed to the (independent, Windows-only) fullscreen
/// monitor hiding it automatically. Both hide/show call sites consult this so
/// exiting fullscreen never resurrects a pet the user explicitly hid.
#[derive(Default)]
struct OverlayHideGate {
    user_hidden: std::sync::atomic::AtomicBool,
}

fn toggle_overlay_visibility(app: &tauri::AppHandle) {
    use std::sync::atomic::Ordering;
    use tauri::Manager;

    let gate = app.state::<OverlayHideGate>();
    let hidden = {
        let next = !gate.user_hidden.load(Ordering::SeqCst);
        gate.user_hidden.store(next, Ordering::SeqCst);
        next
    };
    let Some(overlay) = app.get_webview_window("overlay") else {
        return;
    };
    if hidden {
        let _ = overlay.hide();
        if let Some(panel) = app.get_webview_window("panel") {
            let _ = panel.hide();
        }
        return;
    }
    #[cfg(windows)]
    if window::foreground_window_is_fullscreen() {
        // Leave it hidden; start_fullscreen_monitor's exit edge will show it
        // once fullscreen ends, now that user_hidden is false.
        return;
    }
    let _ = overlay.show();
}

fn register_startup_hotkey(
    app: &tauri::AppHandle,
    repository: &store::SettingsRepository,
    hotkey: &str,
) {
    use tauri_plugin_global_shortcut::GlobalShortcutExt;

    if app.global_shortcut().register(hotkey).is_err() {
        eprintln!("failed to register saved hotkey {hotkey}; clearing it");
        let _ = repository.clear_hotkey();
    }
}

#[cfg(windows)]
fn start_fullscreen_monitor(app: tauri::AppHandle) {
    use std::sync::atomic::Ordering;
    use std::time::Duration;
    use tauri::Manager;

    tauri::async_runtime::spawn(async move {
        let mut hidden_for_fullscreen = false;
        loop {
            tokio::time::sleep(Duration::from_millis(500)).await;
            let fullscreen = window::foreground_window_is_fullscreen();
            if fullscreen == hidden_for_fullscreen {
                continue;
            }
            hidden_for_fullscreen = fullscreen;
            if let Some(overlay) = app.get_webview_window("overlay") {
                let user_hidden = app
                    .state::<OverlayHideGate>()
                    .user_hidden
                    .load(Ordering::SeqCst);
                let _ = if fullscreen {
                    overlay.hide()
                } else if window::should_restore_overlay_after_fullscreen(user_hidden) {
                    overlay.show()
                } else {
                    Ok(())
                };
            }
            if fullscreen {
                if let Some(panel) = app.get_webview_window("panel") {
                    let _ = panel.hide();
                }
            }
        }
    });
}

/// Packages that shipped in an earlier release and were since renamed or
/// dropped, paired with the display name their stock build carried. Removing
/// them keeps a stale copy from appearing in the settings picker beside its
/// replacement.
const RETIRED_PETS: &[(&str, &str)] = &[("cat", "Cat")];

/// Deletes installed copies of retired packages, but only while they are still
/// stock.
///
/// A renamed package is a user customization, and `should_preserve_installed`
/// already refuses to clobber those — retirement follows the same rule rather
/// than reaching around it. Failures are logged and skipped: losing a stale
/// directory matters far less than installing the packages that replace it.
fn remove_retired_pet_packages(installed_pets: &Path, repository: &store::PetPackageRepository) {
    for (retired, stock_name) in RETIRED_PETS {
        let stale = installed_pets.join(retired);
        if !stale.exists() {
            continue;
        }
        // Unreadable means a broken stock leftover, which is worth clearing.
        let is_stock = repository
            .load(retired)
            .map(|package| package.manifest.display_name == *stock_name)
            .unwrap_or(true);
        if !is_stock {
            continue;
        }
        if let Err(error) = fs::remove_dir_all(&stale) {
            eprintln!("failed to remove retired pet package {retired}: {error}");
        }
    }
}

/// Installs every package found in the bundled resource directory.
///
/// Driven by what is on disk rather than a hardcoded list, so shipping a new
/// pet is a matter of adding its folder to `resources/pets/`. Individual
/// packages fail independently, mirroring `PetPackageRepository::list`: one bad
/// directory must not take the rest of the bundle down with it.
fn install_bundled_pet_packages(resource_dir: &Path, app_data: &Path) -> io::Result<()> {
    let bundled_pets = resource_dir.join("resources").join("pets");
    let installed_pets = app_data.join("pets");
    fs::create_dir_all(&installed_pets)?;
    let repository = store::PetPackageRepository::new(app_data);
    // Retirement runs before installation: a future id that is both retired
    // and bundled must not have its fresh copy deleted straight afterwards.
    remove_retired_pet_packages(&installed_pets, &repository);
    for entry in fs::read_dir(&bundled_pets)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let Ok(package_id) = entry.file_name().into_string() else {
            continue;
        };
        // A directory name the loader would later reject installs fine and then
        // never appears in the picker. Refuse it here, loudly, while the
        // packaging mistake can still be caught.
        if !domain::is_valid_pet_id(&package_id) {
            eprintln!("skipping bundled pet package with invalid id: {package_id}");
            continue;
        }
        let source = entry.path();
        let Some(bundled) = store::bundled_manifest_info(&source.join("manifest.json")) else {
            eprintln!("skipping bundled pet package with unreadable manifest: {package_id}");
            continue;
        };
        if repository.should_preserve_installed(&package_id, &bundled) {
            continue;
        }
        if let Err(error) = install_one_bundled_pet(&source, &installed_pets, &package_id) {
            eprintln!("failed to install bundled pet package {package_id}: {error}");
        }
    }
    Ok(())
}

fn install_one_bundled_pet(
    source: &Path,
    installed_pets: &Path,
    package_id: &str,
) -> io::Result<()> {
    let destination = installed_pets.join(package_id);
    if destination.exists() {
        fs::remove_dir_all(&destination)?;
    }
    let staging = installed_pets.join(format!(".{package_id}.installing"));
    if staging.exists() {
        fs::remove_dir_all(&staging)?;
    }
    if let Err(error) =
        copy_directory(source, &staging).and_then(|()| fs::rename(&staging, &destination))
    {
        let _ = fs::remove_dir_all(&staging);
        return Err(error);
    }
    Ok(())
}

fn copy_directory(source: &Path, destination: &Path) -> io::Result<()> {
    fs::create_dir_all(destination)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_directory(&source_path, &destination_path)?;
        } else {
            fs::copy(source_path, destination_path)?;
        }
    }
    Ok(())
}

fn configured_codex_search_path(
    override_path: Option<std::ffi::OsString>,
    process_path: Option<std::ffi::OsString>,
) -> Option<std::ffi::OsString> {
    override_path.or(process_path)
}

struct MissingCodexCollector;

impl collectors::Collector for MissingCodexCollector {
    fn provider(&self) -> domain::Provider {
        domain::Provider::Codex
    }

    fn collect(
        &self,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = domain::CollectionOutcome> + Send + '_>>
    {
        Box::pin(async { domain::CollectionOutcome::CliMissing })
    }
}

fn restore_window_positions(app: &tauri::App, settings: &store::Settings) {
    use tauri::Manager;
    let Some(overlay) = app.get_webview_window("overlay") else {
        return;
    };
    let scale = overlay
        .scale_factor()
        .ok()
        .filter(|scale| scale.is_finite() && *scale > 0.0)
        .unwrap_or(1.0);
    let target = window::Point {
        x: settings.logical_position.x * scale,
        y: settings.logical_position.y * scale,
    };
    let overlay_size = overlay.outer_size().ok();
    let displays = overlay
        .available_monitors()
        .ok()
        .into_iter()
        .flatten()
        .enumerate()
        .map(|(index, monitor)| {
            let position = monitor.position();
            let size = monitor.size();
            window::Display {
                id: monitor.name().cloned().unwrap_or_else(|| index.to_string()),
                bounds: window::Rect {
                    x: f64::from(position.x),
                    y: f64::from(position.y),
                    width: f64::from(size.width),
                    height: f64::from(size.height),
                },
                scale_factor: monitor.scale_factor(),
            }
        })
        .collect::<Vec<_>>();
    let recovered = overlay_size
        .and_then(|size| {
            window::clamp_window(
                target,
                window::Size {
                    width: f64::from(size.width),
                    height: f64::from(size.height),
                },
                &displays,
            )
        })
        .unwrap_or(target);
    let _ = overlay.set_position(tauri::PhysicalPosition::new(
        recovered.x.round() as i32,
        recovered.y.round() as i32,
    ));
    let Some(panel) = app.get_webview_window("panel") else {
        return;
    };
    // Share the placement policy with show_panel/resize_panel rather than
    // reimplementing it here — this path used the full monitor bounds and a
    // separately scaled gap, so the two disagreed on HiDPI displays.
    let _ = refresh::ipc::position_panel(&overlay, &panel);
}

fn platform_home_dir() -> Option<std::path::PathBuf> {
    #[cfg(windows)]
    {
        std::env::var_os("USERPROFILE").map(Into::into)
    }
    #[cfg(not(windows))]
    {
        std::env::var_os("HOME").map(Into::into)
    }
}

#[cfg(test)]
mod tests {
    use crate::refresh::ipc::{CollectorMode, CollectorModeDto};
    use std::ffi::OsString;
    use std::fs;
    use std::path::Path;

    #[test]
    fn application_identifier_is_stable() {
        assert_eq!(env!("CARGO_PKG_NAME"), "cachebite");
    }

    #[test]
    fn collector_mode_distinguishes_fixture_from_production_composition() {
        assert_eq!(
            CollectorModeDto::for_fixture_gate(true),
            CollectorModeDto {
                claude: CollectorMode::Fixture,
                codex: CollectorMode::Fixture
            }
        );
        assert_eq!(
            CollectorModeDto::for_fixture_gate(false),
            CollectorModeDto {
                claude: CollectorMode::Production,
                codex: CollectorMode::Production
            }
        );
    }

    #[test]
    fn explicit_codex_search_path_overrides_process_path() {
        assert_eq!(
            super::configured_codex_search_path(
                Some(OsString::from("/controlled/no-codex")),
                Some(OsString::from("/usr/bin")),
            ),
            Some(OsString::from("/controlled/no-codex"))
        );
    }

    /// Writes a bundled package whose manifest carries the fields the
    /// installer reads: `displayName` identifies stock art, `version` drives
    /// forced reinstalls.
    fn write_bundled_manifest(bundled: &Path, package_id: &str, display_name: &str, version: u32) {
        fs::create_dir_all(bundled.join(package_id).join("frames")).unwrap();
        fs::write(
            bundled.join(package_id).join("manifest.json"),
            format!(
                r#"{{"id":"{package_id}","displayName":"{display_name}","version":{version}}}"#
            ),
        )
        .unwrap();
    }

    #[test]
    fn installs_bundled_pet_packages_without_overwriting_existing_packages() {
        let temp = tempfile::tempdir().unwrap();
        let bundled = temp.path().join("resources/pets");
        let app_data = temp.path().join("app-data");
        write_bundled_manifest(&bundled, "tabby", "Tabby", 1);
        write_bundled_manifest(&bundled, "corgi", "Corgi", 1);
        fs::write(
            bundled.join("tabby/frames/tabby_idle_01.png"),
            "tabby frame",
        )
        .unwrap();
        fs::create_dir_all(app_data.join("pets/tabby/frames")).unwrap();
        fs::write(
            app_data.join("pets/tabby/manifest.json"),
            r#"{"id":"tabby","displayName":"Custom tabby","defaultSize":{"width":160,"height":160},"animations":{"idle":{"type":"image","source":"frames/custom.png"}},"states":{}}"#,
        )
        .unwrap();
        fs::write(
            app_data.join("pets/tabby/frames/custom.png"),
            "custom frame",
        )
        .unwrap();

        super::install_bundled_pet_packages(temp.path(), &app_data).unwrap();

        assert_eq!(
            fs::read_to_string(app_data.join("pets/tabby/manifest.json")).unwrap(),
            r#"{"id":"tabby","displayName":"Custom tabby","defaultSize":{"width":160,"height":160},"animations":{"idle":{"type":"image","source":"frames/custom.png"}},"states":{}}"#
        );
        assert!(app_data.join("pets/corgi/manifest.json").exists());
    }

    #[test]
    fn installs_every_package_found_in_the_bundle_directory() {
        let temp = tempfile::tempdir().unwrap();
        let bundled = temp.path().join("resources/pets");
        let app_data = temp.path().join("app-data");
        // A pet added in a later release: no code change should be required.
        for package_id in ["tabby", "corgi", "axolotl"] {
            write_bundled_manifest(&bundled, package_id, "Stock", 1);
        }

        super::install_bundled_pet_packages(temp.path(), &app_data).unwrap();

        for package_id in ["tabby", "corgi", "axolotl"] {
            assert!(
                app_data.join("pets").join(package_id).exists(),
                "expected {package_id} to be installed"
            );
        }
    }

    #[test]
    fn keeps_a_retired_package_the_user_renamed() {
        let temp = tempfile::tempdir().unwrap();
        let bundled = temp.path().join("resources/pets");
        let app_data = temp.path().join("app-data");
        write_bundled_manifest(&bundled, "tabby", "Tabby", 1);
        // Retirement must honour the same customization rule as
        // should_preserve_installed: a rename is the user's, not ours.
        fs::create_dir_all(app_data.join("pets/cat/frames")).unwrap();
        fs::write(app_data.join("pets/cat/frames/idle.svg"), "<svg/>").unwrap();
        fs::write(
            app_data.join("pets/cat/manifest.json"),
            r#"{"id":"cat","displayName":"My Cat","version":1,"defaultSize":{"width":128,"height":128},"animations":{"idle":{"type":"image","source":"frames/idle.svg"}},"states":{}}"#,
        )
        .unwrap();

        super::install_bundled_pet_packages(temp.path(), &app_data).unwrap();

        assert!(app_data.join("pets/cat/manifest.json").exists());
        assert!(app_data.join("pets/tabby").exists());
    }

    #[test]
    fn skips_bundled_packages_whose_directory_name_is_not_a_valid_pet_id() {
        let temp = tempfile::tempdir().unwrap();
        let bundled = temp.path().join("resources/pets");
        let app_data = temp.path().join("app-data");
        write_bundled_manifest(&bundled, "tabby", "Tabby", 1);
        // Installing this would succeed and then never load, because
        // is_valid_pet_id rejects underscores and uppercase.
        write_bundled_manifest(&bundled, "My_Pet", "My Pet", 1);

        super::install_bundled_pet_packages(temp.path(), &app_data).unwrap();

        assert!(!app_data.join("pets/My_Pet").exists());
        assert!(app_data.join("pets/tabby").exists());
    }

    #[test]
    fn one_unusable_bundled_package_does_not_block_the_others() {
        let temp = tempfile::tempdir().unwrap();
        let bundled = temp.path().join("resources/pets");
        let app_data = temp.path().join("app-data");
        write_bundled_manifest(&bundled, "tabby", "Tabby", 1);
        write_bundled_manifest(&bundled, "corgi", "Corgi", 1);
        // Unreadable manifest: skipped with a log, never fatal.
        fs::create_dir_all(bundled.join("broken")).unwrap();
        fs::write(bundled.join("broken/manifest.json"), "not json").unwrap();

        super::install_bundled_pet_packages(temp.path(), &app_data).unwrap();

        assert!(!app_data.join("pets/broken").exists());
        assert!(app_data.join("pets/tabby").exists());
        assert!(app_data.join("pets/corgi").exists());
    }

    #[test]
    fn removes_retired_pet_packages_from_the_install_directory() {
        let temp = tempfile::tempdir().unwrap();
        let bundled = temp.path().join("resources/pets");
        let app_data = temp.path().join("app-data");
        write_bundled_manifest(&bundled, "tabby", "Tabby", 1);
        // Left over from a release that still shipped `cat`.
        fs::create_dir_all(app_data.join("pets/cat/frames")).unwrap();
        fs::write(
            app_data.join("pets/cat/manifest.json"),
            r#"{"id":"cat","displayName":"Cat","version":1,"defaultSize":{"width":128,"height":128},"animations":{"idle":{"type":"image","source":"frames/cat_idle_01.png"}},"states":{}}"#,
        )
        .unwrap();

        super::install_bundled_pet_packages(temp.path(), &app_data).unwrap();

        assert!(!app_data.join("pets/cat").exists());
        assert!(app_data.join("pets/tabby").exists());
    }

    #[test]
    fn repairs_an_incomplete_bundled_pet_install() {
        let temp = tempfile::tempdir().unwrap();
        let bundled = temp.path().join("resources/pets");
        let app_data = temp.path().join("app-data");
        for package_id in ["tabby", "corgi"] {
            write_bundled_manifest(&bundled, package_id, "Stock", 1);
            fs::write(
                bundled
                    .join(package_id)
                    .join(format!("frames/{package_id}_idle_01.png")),
                format!("{package_id} frame"),
            )
            .unwrap();
        }
        fs::create_dir_all(app_data.join("pets/tabby")).unwrap();

        super::install_bundled_pet_packages(temp.path(), &app_data).unwrap();

        assert_eq!(
            fs::read_to_string(app_data.join("pets/tabby/frames/tabby_idle_01.png")).unwrap(),
            "tabby frame"
        );
    }

    #[test]
    fn upgrades_a_legacy_bundled_pet_package_to_current_frame_names() {
        let temp = tempfile::tempdir().unwrap();
        let bundled = temp.path().join("resources/pets");
        let app_data = temp.path().join("app-data");
        for package_id in ["tabby", "corgi"] {
            write_bundled_manifest(&bundled, package_id, "Tabby", 0);
        }
        fs::write(bundled.join("tabby/frames/tabby_idle_01.png"), "new frame").unwrap();
        fs::create_dir_all(app_data.join("pets/tabby/frames")).unwrap();
        fs::write(
            app_data.join("pets/tabby/manifest.json"),
            r#"{"id":"tabby","displayName":"Tabby","defaultSize":{"width":128,"height":128},"animations":{"idle":{"type":"image","source":"frames/idle_01.png"}},"states":{}}"#,
        )
        .unwrap();
        fs::write(app_data.join("pets/tabby/frames/idle_01.png"), "old frame").unwrap();

        super::install_bundled_pet_packages(temp.path(), &app_data).unwrap();

        assert_eq!(
            fs::read_to_string(app_data.join("pets/tabby/frames/tabby_idle_01.png")).unwrap(),
            "new frame"
        );
        assert!(!app_data.join("pets/tabby/frames/idle_01.png").exists());
    }

    #[test]
    fn reinstalls_stock_pet_when_bundled_version_is_newer() {
        let temp = tempfile::tempdir().unwrap();
        let bundled = temp.path().join("resources/pets");
        let app_data = temp.path().join("app-data");
        for package_id in ["tabby", "corgi"] {
            write_bundled_manifest(&bundled, package_id, "Tabby", 1);
        }
        fs::write(
            bundled.join("tabby/frames/tabby_idle_01.png"),
            "transparent frame",
        )
        .unwrap();

        // Installed stock tabby: current frame naming, no version field (legacy 0).
        fs::create_dir_all(app_data.join("pets/tabby/frames")).unwrap();
        fs::write(
            app_data.join("pets/tabby/manifest.json"),
            r#"{"id":"tabby","displayName":"Tabby","defaultSize":{"width":128,"height":128},"animations":{"idle":{"type":"image","source":"frames/tabby_idle_01.png"}},"states":{}}"#,
        )
        .unwrap();
        fs::write(
            app_data.join("pets/tabby/frames/tabby_idle_01.png"),
            "opaque frame",
        )
        .unwrap();

        super::install_bundled_pet_packages(temp.path(), &app_data).unwrap();

        assert_eq!(
            fs::read_to_string(app_data.join("pets/tabby/frames/tabby_idle_01.png")).unwrap(),
            "transparent frame"
        );
    }

    #[test]
    fn preserves_stock_pet_when_installed_version_matches_bundled() {
        let temp = tempfile::tempdir().unwrap();
        let bundled = temp.path().join("resources/pets");
        let app_data = temp.path().join("app-data");
        for package_id in ["tabby", "corgi"] {
            write_bundled_manifest(&bundled, package_id, "Tabby", 1);
        }
        fs::write(
            bundled.join("tabby/frames/tabby_idle_01.png"),
            "bundled frame",
        )
        .unwrap();

        // Installed stock tabby already at version 1 with current frame naming.
        fs::create_dir_all(app_data.join("pets/tabby/frames")).unwrap();
        fs::write(
            app_data.join("pets/tabby/manifest.json"),
            r#"{"id":"tabby","displayName":"Tabby","version":1,"defaultSize":{"width":128,"height":128},"animations":{"idle":{"type":"image","source":"frames/tabby_idle_01.png"}},"states":{}}"#,
        )
        .unwrap();
        fs::write(
            app_data.join("pets/tabby/frames/tabby_idle_01.png"),
            "kept frame",
        )
        .unwrap();

        super::install_bundled_pet_packages(temp.path(), &app_data).unwrap();

        assert_eq!(
            fs::read_to_string(app_data.join("pets/tabby/frames/tabby_idle_01.png")).unwrap(),
            "kept frame"
        );
    }
}
