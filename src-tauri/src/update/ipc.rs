use tauri::{AppHandle, Emitter, State};

use crate::refresh::ipc::{authorize, IpcError};
use crate::window::NativeCommand;

use super::{service::UpdateService, state::UpdateStateDto};

pub const UPDATE_STATE_EVENT: &str = "update-state";

/// Streams every state transition to the panel, mirroring
/// `refresh::ipc::emit_provider_states`.
///
/// The loop ends when the app is gone, which is what keeps a closed window from
/// leaking a task for the rest of the process lifetime.
pub fn emit_update_state(app: &AppHandle, service: &UpdateService) {
    let app = app.clone();
    let current_version = service.current_version().to_owned();
    let mut status = service.subscribe();
    tauri::async_runtime::spawn(async move {
        while status.changed().await.is_ok() {
            let state = UpdateStateDto {
                current_version: current_version.clone(),
                status: status.borrow_and_update().clone(),
            };
            if app.emit(UPDATE_STATE_EVENT, state).is_err() {
                break;
            }
        }
    });
}

#[tauri::command]
pub fn get_update_state(
    window: tauri::WebviewWindow,
    service: State<'_, UpdateService>,
) -> Result<UpdateStateDto, IpcError> {
    authorize(&window, NativeCommand::GetUpdateState)?;
    Ok(service.snapshot())
}

/// A failed check is a *state*, not an IPC error: the command returns `Ok` and
/// the reason travels in `UpdateStatus::Failed` so the panel can render a
/// recoverable sentence instead of a rejected promise.
#[tauri::command]
pub async fn check_for_update(
    window: tauri::WebviewWindow,
    service: State<'_, UpdateService>,
) -> Result<(), IpcError> {
    authorize(&window, NativeCommand::CheckForUpdate)?;
    let service = service.inner().clone();
    service.check_now().await;
    Ok(())
}

#[tauri::command]
pub async fn install_update(
    window: tauri::WebviewWindow,
    app: AppHandle,
    service: State<'_, UpdateService>,
) -> Result<(), IpcError> {
    authorize(&window, NativeCommand::InstallUpdate)?;
    let service = service.inner().clone();
    service.install(&app).await;
    Ok(())
}

/// The fixture feed behind the webdriver probe below.
///
/// Managed only when the app is composed with fixtures, so the probe reports 0
/// in a production composition rather than pretending to have counted.
#[cfg(feature = "webdriver")]
pub struct UpdateProbe(pub std::sync::Arc<super::feed::FixtureFeed>);

/// Webdriver-only, exactly like `refresh::ipc::get_window_states`: no
/// `NativeCommand` variant and no `command_allowed` entry, because adding a
/// test affordance to the allowlist would leak it into the shipped surface.
#[cfg(feature = "webdriver")]
#[tauri::command]
pub fn get_update_probe_count(app: AppHandle) -> Result<usize, IpcError> {
    use tauri::Manager;

    Ok(app
        .try_state::<UpdateProbe>()
        .map(|probe| probe.0.probes())
        .unwrap_or(0))
}
