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

    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .setup(|app| {
            #[cfg(debug_assertions)]
            {
                eprintln!(
                    "[CacheBite:native] setup:start v{}",
                    env!("CARGO_PKG_VERSION")
                );
                if let Some(overlay) = app.get_webview_window("overlay") {
                    overlay.open_devtools();
                }
            }
            let app_data = app.path().app_data_dir()?;
            if let Err(error) = install_bundled_pet_packages(&app.path().resource_dir()?, &app_data)
            {
                eprintln!("failed to install bundled pet packages: {error}");
            }
            let settings_repository = store::SettingsRepository::new(&app_data);
            if let Ok(settings) = settings_repository.load() {
                restore_window_positions(app, &settings);
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
            let persistence = refresh::RefreshPersistence::new(snapshots, history);
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
            app.manage(store::HistoryRepository::new(app.path().app_data_dir()?));
            app.manage(store::PetPackageRepository::new(app.path().app_data_dir()?));
            app.manage(service);
            app.manage(collector_mode);
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
            refresh::ipc::get_platform_capabilities,
            refresh::ipc::save_position,
            refresh::ipc::refresh_provider,
            refresh::ipc::update_settings,
            refresh::ipc::show_panel,
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

#[cfg(windows)]
fn start_fullscreen_monitor(app: tauri::AppHandle) {
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
                let _ = if fullscreen {
                    overlay.hide()
                } else {
                    overlay.show()
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

fn install_bundled_pet_packages(resource_dir: &Path, app_data: &Path) -> io::Result<()> {
    let bundled_pets = resource_dir.join("resources").join("pets");
    let installed_pets = app_data.join("pets");
    fs::create_dir_all(&installed_pets)?;
    for package_id in ["cat", "corgi"] {
        let destination = installed_pets.join(package_id);
        if store::PetPackageRepository::new(app_data).should_preserve_installed(package_id) {
            continue;
        }
        if destination.exists() {
            fs::remove_dir_all(&destination)?;
        }
        let staging = installed_pets.join(format!(".{package_id}.installing"));
        if staging.exists() {
            fs::remove_dir_all(&staging)?;
        }
        if let Err(error) = copy_directory(&bundled_pets.join(package_id), &staging)
            .and_then(|()| fs::rename(&staging, &destination))
        {
            let _ = fs::remove_dir_all(&staging);
            return Err(error);
        }
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
    let (Ok(position), Ok(pet_size), Ok(panel_size), Ok(Some(monitor))) = (
        overlay.outer_position(),
        overlay.outer_size(),
        panel.outer_size(),
        overlay.current_monitor(),
    ) else {
        return;
    };
    let monitor_position = monitor.position();
    let monitor_size = monitor.size();
    let anchored = window::anchor_panel(
        window::Rect {
            x: f64::from(position.x),
            y: f64::from(position.y),
            width: f64::from(pet_size.width),
            height: f64::from(pet_size.height),
        },
        window::Size {
            width: f64::from(panel_size.width),
            height: f64::from(panel_size.height),
        },
        window::Rect {
            x: f64::from(monitor_position.x),
            y: f64::from(monitor_position.y),
            width: f64::from(monitor_size.width),
            height: f64::from(monitor_size.height),
        },
        12.0 * scale,
    );
    let _ = panel.set_position(tauri::PhysicalPosition::new(
        anchored.x.round() as i32,
        anchored.y.round() as i32,
    ));
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

    #[test]
    fn installs_bundled_pet_packages_without_overwriting_existing_packages() {
        let temp = tempfile::tempdir().unwrap();
        let bundled = temp.path().join("resources/pets");
        let app_data = temp.path().join("app-data");
        fs::create_dir_all(bundled.join("cat/frames")).unwrap();
        fs::create_dir_all(bundled.join("corgi/frames")).unwrap();
        fs::write(bundled.join("cat/manifest.json"), "bundled cat").unwrap();
        fs::write(bundled.join("cat/frames/cat_idle_01.png"), "cat frame").unwrap();
        fs::write(bundled.join("corgi/manifest.json"), "bundled corgi").unwrap();
        fs::create_dir_all(app_data.join("pets/cat/frames")).unwrap();
        fs::write(
            app_data.join("pets/cat/manifest.json"),
            r#"{"id":"cat","displayName":"Custom cat","defaultSize":{"width":160,"height":160},"animations":{"idle":{"type":"image","source":"frames/custom.png"}},"states":{}}"#,
        )
        .unwrap();
        fs::write(app_data.join("pets/cat/frames/custom.png"), "custom frame").unwrap();

        super::install_bundled_pet_packages(temp.path(), &app_data).unwrap();

        assert_eq!(
            fs::read_to_string(app_data.join("pets/cat/manifest.json")).unwrap(),
            r#"{"id":"cat","displayName":"Custom cat","defaultSize":{"width":160,"height":160},"animations":{"idle":{"type":"image","source":"frames/custom.png"}},"states":{}}"#
        );
        assert_eq!(
            fs::read_to_string(app_data.join("pets/corgi/manifest.json")).unwrap(),
            "bundled corgi"
        );
    }

    #[test]
    fn repairs_an_incomplete_bundled_pet_install() {
        let temp = tempfile::tempdir().unwrap();
        let bundled = temp.path().join("resources/pets");
        let app_data = temp.path().join("app-data");
        for package_id in ["cat", "corgi"] {
            fs::create_dir_all(bundled.join(package_id).join("frames")).unwrap();
            fs::write(
                bundled.join(package_id).join("manifest.json"),
                format!("bundled {package_id}"),
            )
            .unwrap();
            fs::write(
                bundled
                    .join(package_id)
                    .join(format!("frames/{package_id}_idle_01.png")),
                format!("{package_id} frame"),
            )
            .unwrap();
        }
        fs::create_dir_all(app_data.join("pets/cat")).unwrap();

        super::install_bundled_pet_packages(temp.path(), &app_data).unwrap();

        assert_eq!(
            fs::read_to_string(app_data.join("pets/cat/manifest.json")).unwrap(),
            "bundled cat"
        );
        assert_eq!(
            fs::read_to_string(app_data.join("pets/cat/frames/cat_idle_01.png")).unwrap(),
            "cat frame"
        );
    }

    #[test]
    fn upgrades_a_legacy_bundled_pet_package_to_current_frame_names() {
        let temp = tempfile::tempdir().unwrap();
        let bundled = temp.path().join("resources/pets");
        let app_data = temp.path().join("app-data");
        for package_id in ["cat", "corgi"] {
            fs::create_dir_all(bundled.join(package_id).join("frames")).unwrap();
            fs::write(
                bundled.join(package_id).join("manifest.json"),
                format!("bundled {package_id}"),
            )
            .unwrap();
        }
        fs::write(bundled.join("cat/frames/cat_idle_01.png"), "new frame").unwrap();
        fs::create_dir_all(app_data.join("pets/cat/frames")).unwrap();
        fs::write(
            app_data.join("pets/cat/manifest.json"),
            r#"{"id":"cat","displayName":"Cat","defaultSize":{"width":128,"height":128},"animations":{"idle":{"type":"image","source":"frames/idle_01.png"}},"states":{}}"#,
        )
        .unwrap();
        fs::write(app_data.join("pets/cat/frames/idle_01.png"), "old frame").unwrap();

        super::install_bundled_pet_packages(temp.path(), &app_data).unwrap();

        assert_eq!(
            fs::read_to_string(app_data.join("pets/cat/frames/cat_idle_01.png")).unwrap(),
            "new frame"
        );
        assert!(!app_data.join("pets/cat/frames/idle_01.png").exists());
    }
}
