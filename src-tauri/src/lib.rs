pub mod collectors;
pub mod domain;
pub mod refresh;
pub mod store;
pub mod window;

#[cfg(test)]
mod domain_test;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    use std::{collections::BTreeMap, path::PathBuf, sync::Arc};
    use tauri::Manager;

    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            let app_data = app.path().app_data_dir()?;
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
                (
                    Arc::new(collectors::claude::ClaudeCollector::new(broker)),
                    match configured_codex_search_path(
                        std::env::var_os("CACHEBITE_CODEX_PATH"),
                        std::env::var_os("PATH"),
                    )
                    .ok_or(collectors::CollectorError::CliMissing)
                    .and_then(|path| collectors::codex::resolve_codex_executable(&path))
                    .and_then(collectors::codex::CodexCollector::new)
                    {
                        Ok(collector) => Arc::new(collector),
                        Err(_) => Arc::new(MissingCodexCollector),
                    },
                )
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
}
