use tauri::{
    image::Image,
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::TrayIconBuilder,
    AppHandle,
};

pub const TRAY_ID: &str = "messenger-tray";
pub const SHOW_WINDOW_ID: &str = "tray-show-window";
pub const HIDE_WINDOW_ID: &str = "tray-hide-window";
pub const QUIT_ID: &str = "tray-quit";

const NORMAL_TOOLTIP: &str = "Yandex Telemost";
const NOTIFICATION_TOOLTIP: &str = "Yandex Telemost — new notifications";

pub fn ensure(app: &AppHandle) -> Result<(), String> {
    if app.tray_by_id(TRAY_ID).is_some() {
        return Ok(());
    }

    let show = MenuItem::with_id(app, SHOW_WINDOW_ID, "Show Window", true, None::<&str>)
        .map_err(|error| format!("failed to create tray Show Window action: {error}"))?;
    let hide = MenuItem::with_id(app, HIDE_WINDOW_ID, "Hide Window", true, None::<&str>)
        .map_err(|error| format!("failed to create tray Hide Window action: {error}"))?;
    let separator = PredefinedMenuItem::separator(app)
        .map_err(|error| format!("failed to create tray separator: {error}"))?;
    let quit = MenuItem::with_id(app, QUIT_ID, "Quit", true, None::<&str>)
        .map_err(|error| format!("failed to create tray Quit action: {error}"))?;
    let menu = Menu::with_items(app, &[&show, &hide, &separator, &quit])
        .map_err(|error| format!("failed to create tray menu: {error}"))?;

    TrayIconBuilder::with_id(TRAY_ID)
        .menu(&menu)
        .icon(normal_icon()?)
        .icon_as_template(true)
        .show_menu_on_left_click(false)
        .tooltip(NORMAL_TOOLTIP)
        .build(app)
        .map_err(|error| format!("failed to create tray icon: {error}"))?;
    Ok(())
}

pub fn set_notification(app: &AppHandle, unread: bool) -> Result<(), String> {
    let tray = app
        .tray_by_id(TRAY_ID)
        .ok_or_else(|| "tray icon is unavailable".to_owned())?;
    let icon = if unread {
        alert_icon()?
    } else {
        normal_icon()?
    };
    tray.set_icon_with_as_template(Some(icon), true)
        .map_err(|error| format!("failed to update tray icon: {error}"))?;
    tray.set_tooltip(Some(if unread {
        NOTIFICATION_TOOLTIP
    } else {
        NORMAL_TOOLTIP
    }))
    .map_err(|error| format!("failed to update tray tooltip: {error}"))?;
    Ok(())
}

fn normal_icon() -> Result<Image<'static>, String> {
    Image::from_bytes(include_bytes!("../assets/tray-blue-22.png"))
        .map_err(|error| format!("failed to decode blue tray icon: {error}"))
}

fn alert_icon() -> Result<Image<'static>, String> {
    Image::from_bytes(include_bytes!("../assets/tray-red-22.png"))
        .map_err(|error| format!("failed to decode red tray icon: {error}"))
}
