mod events;
mod model;
mod native;
mod upload;

use model::{EffectBatch, Model, Operation, UiState};
use std::sync::Mutex;
use tauri::{Manager, State};

struct Runtime {
    model: Mutex<Model>,
    started: u64,
}
impl Runtime {
    fn now(&self) -> u64 {
        native::monotonic_millis().saturating_sub(self.started)
    }
    fn snapshot(&self) -> Result<UiState, String> {
        let mut model = self
            .model
            .lock()
            .map_err(|_| "Estado local no disponible.".to_string())?;
        Ok(model.snapshot(self.now()))
    }
}

// All native/main-thread work executes AFTER releasing Rust state/queue locks.
fn execute(app: &tauri::AppHandle, batch: EffectBatch) {
    // The main-thread destination versions this snapshot before applying notices.
    native::sync_ids(&batch.live_ids, batch.revision);
    for operation in batch.operations {
        match operation {
            Operation::Configure {
                enabled,
                paused,
                generation,
                user_change,
            } => {
                native::configure(enabled, paused, generation, user_change, batch.revision);
            }
            Operation::Notify { id, result } => native::notify(&id, result, batch.revision),
            Operation::SyncIds(_) => {}
            Operation::Result { id, action } => native::result_operation(&id, action),
            Operation::ShowWindow { candidate_id } => {
                let handle = app.clone();
                let _ = app.run_on_main_thread(move || {
                    if let Some(id) = candidate_id {
                        let state = handle.state::<Runtime>();
                        // Recheck after scheduling: pause/OFF/wake/expiry or a
                        // newer candidate may have invalidated this preview.
                        let allowed = state
                            .model
                            .lock()
                            .map(|mut model| model.can_foreground_candidate(&id, state.now()))
                            .unwrap_or(false);
                        if !allowed {
                            return;
                        }
                    }
                    if let Some(window) = handle.get_webview_window("main") {
                        // Application hiding is separate from window visibility.
                        // A menu-bar app can be launched hidden by macOS.
                        #[cfg(target_os = "macos")]
                        let _ = handle.show();
                        let _ = window.unminimize();
                        let _ = window.show();
                        // Tauri's macOS runtime also activates NSApplication.
                        let _ = window.set_focus();
                    }
                });
            }
            Operation::Quit => app.exit(0),
            Operation::Upload { id, job } => {
                let handle = app.clone();
                std::thread::spawn(move || {
                    let outcome = upload::run(&mut native::HttpTransport, job, upload::wall_millis);
                    let state = handle.state::<Runtime>();
                    let batch = {
                        let Ok(mut model) = state.model.lock() else {
                            return;
                        };
                        let operations = model.finish_upload(&id, outcome, state.now());
                        model.effect_batch(operations)
                    };
                    execute(&handle, batch);
                });
            }
        }
    }
}
fn change(
    app: &tauri::AppHandle,
    state: &Runtime,
    apply: impl FnOnce(&mut Model, u64) -> Result<Vec<Operation>, String>,
) -> Result<UiState, String> {
    let operations = {
        let mut model = state
            .model
            .lock()
            .map_err(|_| "Estado local no disponible.".to_string())?;
        let operations = apply(&mut model, state.now())?;
        model.effect_batch(operations)
    };
    execute(app, operations);
    state.snapshot()
}

#[tauri::command]
fn get_state(state: State<'_, Runtime>) -> Result<UiState, String> {
    state.snapshot()
}
#[tauri::command]
fn generate_synthetic(state: State<'_, Runtime>) -> Result<UiState, String> {
    state
        .model
        .lock()
        .map_err(|_| "Estado local no disponible.".to_string())?
        .require_native()?;
    native::generate();
    state.snapshot()
}
#[tauri::command]
fn paste_image(state: State<'_, Runtime>) -> Result<UiState, String> {
    state
        .model
        .lock()
        .map_err(|_| "Estado local no disponible.".to_string())?
        .require_native()?;
    native::paste();
    state.snapshot()
}
#[tauri::command]
fn set_clipboard_enabled(
    app: tauri::AppHandle,
    state: State<'_, Runtime>,
    enabled: bool,
) -> Result<UiState, String> {
    change(&app, &state, |model, now| model.set_clipboard(enabled, now))
}
#[tauri::command]
fn set_paused(
    app: tauri::AppHandle,
    state: State<'_, Runtime>,
    paused: bool,
) -> Result<UiState, String> {
    change(&app, &state, |model, now| Ok(model.set_paused(paused, now)))
}
#[tauri::command]
fn request_notifications(state: State<'_, Runtime>) -> Result<UiState, String> {
    {
        state
            .model
            .lock()
            .map_err(|_| "Estado local no disponible.".to_string())?
            .request_notifications()?;
    }
    native::request_notifications();
    state.snapshot()
}
#[tauri::command]
fn candidate_action(
    app: tauri::AppHandle,
    state: State<'_, Runtime>,
    id: String,
    action: String,
) -> Result<UiState, String> {
    change(&app, &state, |model, now| {
        model.candidate_action(&id, &action, now)
    })
}
#[tauri::command]
fn result_action(
    app: tauri::AppHandle,
    state: State<'_, Runtime>,
    id: String,
    action: String,
) -> Result<UiState, String> {
    if !matches!(action.as_str(), "copy" | "open") {
        return Err("Acción desconocida.".into());
    }
    change(&app, &state, |model, now| {
        model.result_action(&id, &action, now)
    })
}

fn main() {
    let mut context = tauri::generate_context!();
    // Only macOS has the native tray. Other platforms keep an accessible window.
    #[cfg(target_os = "macos")]
    if let Some(window) = context
        .config_mut()
        .app
        .windows
        .iter_mut()
        .find(|w| w.label == "main")
    {
        window.visible = false;
        window.focus = false;
    }
    tauri::Builder::default()
        .manage(Runtime {
            model: Mutex::new(Model::new(native::available())),
            started: native::monotonic_millis(),
        })
        .invoke_handler(tauri::generate_handler![
            get_state,
            generate_synthetic,
            set_clipboard_enabled,
            paste_image,
            set_paused,
            request_notifications,
            candidate_action,
            result_action
        ])
        .setup(|app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);
            native::start();
            let handle = app.handle().clone();
            // This engine runs independently of hidden/closed/throttled WebView JS.
            std::thread::spawn(move || loop {
                let events = native::drain();
                let state = handle.state::<Runtime>();
                let operations = {
                    let Ok(mut model) = state.model.lock() else {
                        break;
                    };
                    let now = state.now();
                    model.tick(now);
                    let mut operations = Vec::new();
                    for event in events {
                        operations.extend(model.apply_event(event, now));
                    }
                    model.effect_batch(operations)
                };
                execute(&handle, operations);
                std::thread::sleep(std::time::Duration::from_millis(500));
            });
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if native::available() {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .run(context)
        .expect("local shell startup");
}
