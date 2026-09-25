use tauri::{
    menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu},
    AppHandle, Wry,
};

use crate::settings::Settings;

pub const MANAGED_MODE_ID: &str = "managed-mode";
pub const CLOSE_TO_TRAY_ID: &str = "close-to-tray";
pub const SHOW_ON_STARTUP_ID: &str = "show-on-startup";
pub const RELOAD_ID: &str = "reload-page";
pub const HOME_ID: &str = "home";
pub const ZOOM_IN_ID: &str = "zoom-in";
pub const ZOOM_OUT_ID: &str = "zoom-out";
pub const DEVTOOLS_ID: &str = "developer-tools";

const MIN_ZOOM: f64 = 0.25;
const MAX_ZOOM: f64 = 3.0;
const ZOOM_STEP: f64 = 0.1;

pub struct MenuControls {
    managed_mode: CheckMenuItem<Wry>,
    close_to_tray: CheckMenuItem<Wry>,
    show_on_startup: CheckMenuItem<Wry>,
}

#[derive(Debug, PartialEq)]
pub enum MenuAction {
    SetManagedMode(bool),
    SetCloseToTray(bool),
    SetShowOnStartup(bool),
    Reload,
    Home,
    AdjustZoom(f64),
    OpenDevtools,
}

impl MenuControls {
    pub fn install(app: &AppHandle, settings: &Settings) -> Result<Self, String> {
        let menu = Menu::default(app)
            .map_err(|error| format!("failed to create default menu: {error}"))?;
        let managed_mode = check(app, MANAGED_MODE_ID, "Managed Mode", settings.managed_mode)?;
        let close_to_tray = check(
            app,
            CLOSE_TO_TRAY_ID,
            "Close to Tray",
            settings.close_to_tray,
        )?;
        let show_on_startup = check(
            app,
            SHOW_ON_STARTUP_ID,
            "Show on Startup",
            settings.show_on_startup,
        )?;
        let separator = PredefinedMenuItem::separator(app)
            .map_err(|error| format!("failed to create menu separator: {error}"))?;
        let reload = MenuItem::with_id(app, RELOAD_ID, "Reload Page", true, None::<&str>)
            .map_err(|error| format!("failed to create reload menu item: {error}"))?;
        let home = MenuItem::with_id(app, HOME_ID, "Home", true, None::<&str>)
            .map_err(|error| format!("failed to create home menu item: {error}"))?;
        let zoom_in = MenuItem::with_id(app, ZOOM_IN_ID, "Zoom +10%", true, None::<&str>)
            .map_err(|error| format!("failed to create zoom-in menu item: {error}"))?;
        let zoom_out = MenuItem::with_id(app, ZOOM_OUT_ID, "Zoom -10%", true, None::<&str>)
            .map_err(|error| format!("failed to create zoom-out menu item: {error}"))?;
        let devtools =
            MenuItem::with_id(app, DEVTOOLS_ID, "Developer Tools", true, None::<&str>)
                .map_err(|error| format!("failed to create developer tools menu item: {error}"))?;
        let control = Submenu::with_items(
            app,
            "Control",
            true,
            &[
                &managed_mode,
                &close_to_tray,
                &show_on_startup,
                &separator,
                &reload,
                &home,
                &separator,
                &zoom_in,
                &zoom_out,
                &separator,
                &devtools,
            ],
        )
        .map_err(|error| format!("failed to create control menu: {error}"))?;
        menu.append(&control)
            .map_err(|error| format!("failed to add control menu: {error}"))?;
        app.set_menu(menu)
            .map_err(|error| format!("failed to install application menu: {error}"))?;

        Ok(Self {
            managed_mode,
            close_to_tray,
            show_on_startup,
        })
    }

    pub fn action_for(&self, id: &str, settings: &Settings) -> Option<MenuAction> {
        match id {
            MANAGED_MODE_ID => Some(MenuAction::SetManagedMode(!settings.managed_mode)),
            CLOSE_TO_TRAY_ID => Some(MenuAction::SetCloseToTray(!settings.close_to_tray)),
            SHOW_ON_STARTUP_ID => Some(MenuAction::SetShowOnStartup(!settings.show_on_startup)),
            RELOAD_ID => Some(MenuAction::Reload),
            HOME_ID => Some(MenuAction::Home),
            ZOOM_IN_ID => Some(MenuAction::AdjustZoom(ZOOM_STEP)),
            ZOOM_OUT_ID => Some(MenuAction::AdjustZoom(-ZOOM_STEP)),
            DEVTOOLS_ID => Some(MenuAction::OpenDevtools),
            _ => None,
        }
    }

    pub fn synchronize(&self, settings: &Settings) -> Result<(), String> {
        self.managed_mode
            .set_checked(settings.managed_mode)
            .map_err(|error| format!("failed to update Managed Mode menu state: {error}"))?;
        self.close_to_tray
            .set_checked(settings.close_to_tray)
            .map_err(|error| format!("failed to update Close to Tray menu state: {error}"))?;
        self.show_on_startup
            .set_checked(settings.show_on_startup)
            .map_err(|error| format!("failed to update Show on Startup menu state: {error}"))?;
        Ok(())
    }
}

pub fn adjusted_zoom(current: f64, delta: f64) -> f64 {
    ((current + delta).clamp(MIN_ZOOM, MAX_ZOOM) * 100.0).round() / 100.0
}

fn check(
    app: &AppHandle,
    id: &str,
    label: &str,
    checked: bool,
) -> Result<CheckMenuItem<Wry>, String> {
    CheckMenuItem::with_id(app, id, label, true, checked, None::<&str>)
        .map_err(|error| format!("failed to create {label} menu item: {error}"))
}
