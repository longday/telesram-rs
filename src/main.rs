mod config;
mod macos;
mod menu;
mod navigation;
mod settings;
mod tray;
mod window;

use std::{error::Error, sync::Arc};

use tauri::{Manager, RunEvent};

use crate::{
    config::runtime_dir,
    menu::{MenuAction, MenuControls},
    settings::SettingsStore,
    window::WindowState,
};

fn main() {
    if let Err(error) = run() {
        eprintln!("telesram failed to start: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    tauri::Builder::default()
        // Managed navigation owns `_blank` behavior. Do not inject the opener's broad remote-page script.
        .plugin(
            tauri_plugin_opener::Builder::new()
                .open_js_links_on_click(false)
                .build(),
        )
        .plugin(tauri_plugin_single_instance::init(
            |app, _arguments, _working_directory| {
                if let Err(error) = window::show_or_create(app) {
                    eprintln!("[instance] failed to restore existing Telemost window: {error}");
                }
            },
        ))
        .setup(|app| {
            let runtime_dir = runtime_dir();
            std::fs::create_dir_all(&runtime_dir)?;
            // The single-instance plugin can fail open if its IPC socket fails.
            let instance_lock = std::fs::File::options()
                .read(true)
                .write(true)
                .create(true)
                .open(runtime_dir.join("instance.lock"))?;
            match instance_lock.try_lock() {
                Ok(()) => {}
                Err(std::fs::TryLockError::WouldBlock) => {
                    return Err(boxed_error("Telesram is already running".to_owned()));
                }
                Err(error) => {
                    return Err(boxed_error(format!(
                        "failed to lock Telesram instance: {error}"
                    )));
                }
            }
            app.manage(instance_lock);
            let settings = Arc::new(SettingsStore::load(runtime_dir.join("settings.json"))?);
            let state = WindowState::new(settings).map_err(boxed_error)?;
            app.manage(state);
            let state = app.state::<WindowState>();
            let controls = MenuControls::install(app.handle(), &state.settings().snapshot())
                .map_err(boxed_error)?;
            app.manage(controls);
            window::ensure_main_window(app.handle()).map_err(boxed_error)?;
            Ok(())
        })
        .on_menu_event(|app, event| {
            if let Err(error) = handle_menu_event(app, event.id().as_ref()) {
                eprintln!("[menu] {error}");
            }
        })
        .on_tray_icon_event(|tray, event| {
            if let tauri::tray::TrayIconEvent::Click {
                button: tauri::tray::MouseButton::Left,
                button_state: tauri::tray::MouseButtonState::Up,
                ..
            } = event
            {
                if let Err(error) = window::toggle(tray.app_handle()) {
                    eprintln!("[tray] {error}");
                }
            }
        })
        .on_window_event(|window, event| {
            window::handle_window_event(window.app_handle(), window, event);
        })
        .build(tauri::generate_context!())?
        .run(|app, event| match event {
            #[cfg(target_os = "macos")]
            RunEvent::Reopen { .. } => {
                if let Err(error) = window::show_or_create(app) {
                    eprintln!("[activation] failed to restore Telemost window: {error}");
                }
            }
            RunEvent::ExitRequested { code, api, .. } => {
                let state = app.state::<WindowState>();
                if code.is_none() && !state.is_quitting() {
                    api.prevent_exit();
                } else {
                    state.mark_quitting();
                }
            }
            _ => {}
        });

    Ok(())
}

fn handle_menu_event(app: &tauri::AppHandle, id: &str) -> Result<(), String> {
    match id {
        tray::SHOW_WINDOW_ID => return window::show_or_create(app),
        tray::HIDE_WINDOW_ID => return window::hide(app),
        tray::QUIT_ID => {
            app.state::<WindowState>().mark_quitting();
            app.remove_tray_by_id(tray::TRAY_ID);
            app.exit(0);
            return Ok(());
        }
        _ => {}
    }

    let state = app.state::<WindowState>();
    let current = state.settings().snapshot();
    let controls = app.state::<MenuControls>();
    let Some(action) = controls.action_for(id, &current) else {
        return Ok(());
    };

    match action {
        MenuAction::SetManagedMode(enabled) => {
            let next = state
                .settings()
                .update(|settings| settings.managed_mode = enabled)
                .map_err(|error| format!("failed to persist Managed Mode: {error}"))?;
            controls.synchronize(&next)?;
            window::set_managed_mode(app, enabled)?;
        }
        MenuAction::SetCloseToTray(enabled) => {
            let next = state
                .settings()
                .update(|settings| settings.close_to_tray = enabled)
                .map_err(|error| format!("failed to persist Close to Tray: {error}"))?;
            controls.synchronize(&next)?;
        }
        MenuAction::SetShowOnStartup(enabled) => {
            let next = state
                .settings()
                .update(|settings| settings.show_on_startup = enabled)
                .map_err(|error| format!("failed to persist Show on Startup: {error}"))?;
            controls.synchronize(&next)?;
        }
        MenuAction::Reload => window::reload(app)?,
        MenuAction::Home => window::go_home(app)?,
        MenuAction::AdjustZoom(delta) => window::adjust_zoom(app, delta)?,
        MenuAction::OpenDevtools => window::ensure_main_window(app)?.open_devtools(),
    }
    Ok(())
}

fn boxed_error(message: String) -> Box<dyn Error> {
    Box::new(std::io::Error::other(message))
}
