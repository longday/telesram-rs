use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use tauri::{
    webview::{NewWindowResponse, PageLoadEvent},
    AppHandle, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder, WindowEvent,
};
use url::Url;

use crate::{
    config::TELEMOST_URL, macos, navigation::NavigationPolicy, settings::SettingsStore, tray,
};

#[cfg(target_os = "macos")]
use crate::config::USER_AGENT;

pub const MAIN_WINDOW_LABEL: &str = "messenger";

pub struct WindowState {
    settings: Arc<SettingsStore>,
    navigation: NavigationPolicy,
    quitting: AtomicBool,
    unread: AtomicBool,
    main_document: AtomicBool,
    native_surface_ready: AtomicBool,
    show_when_ready: AtomicBool,
    title_has_digits: AtomicBool,
}

impl WindowState {
    pub fn new(settings: Arc<SettingsStore>) -> Result<Self, String> {
        let navigation = NavigationPolicy::new(TELEMOST_URL)
            .map_err(|error| format!("invalid configured Telemost URL: {error}"))?;
        Ok(Self {
            settings,
            navigation,
            quitting: AtomicBool::new(false),
            unread: AtomicBool::new(false),
            main_document: AtomicBool::new(false),
            native_surface_ready: AtomicBool::new(false),
            show_when_ready: AtomicBool::new(false),
            title_has_digits: AtomicBool::new(false),
        })
    }

    pub fn settings(&self) -> &Arc<SettingsStore> {
        &self.settings
    }

    pub fn mark_quitting(&self) {
        self.quitting.store(true, Ordering::Release);
    }

    pub fn is_quitting(&self) -> bool {
        self.quitting.load(Ordering::Acquire)
    }

    fn request_show(&self) -> bool {
        if self.native_surface_ready.load(Ordering::Acquire) {
            true
        } else {
            self.show_when_ready.store(true, Ordering::Release);
            false
        }
    }

    fn mark_native_surface_ready(&self) -> bool {
        self.native_surface_ready.store(true, Ordering::Release);
        self.show_when_ready.swap(false, Ordering::AcqRel)
    }

    fn begin_window_configuration(&self) {
        self.native_surface_ready.store(false, Ordering::Release);
        self.main_document.store(false, Ordering::Release);
        self.title_has_digits.store(false, Ordering::Release);
    }
}

pub fn ensure_main_window(app: &AppHandle) -> Result<WebviewWindow, String> {
    if let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) {
        return Ok(window);
    }

    let state = app.state::<WindowState>();
    state.begin_window_configuration();
    let settings = state.settings().snapshot();
    let data_store_identifier = *b"telesram-webview";
    let blank_url = Url::parse("about:blank")
        .map_err(|error| format!("failed to construct blank initial URL: {error}"))?;

    let navigation = state.navigation.clone();
    let navigation_app = app.clone();
    let navigation_settings = state.settings().clone();
    let popup_navigation = state.navigation.clone();
    let popup_app = app.clone();
    let popup_settings = state.settings().clone();
    let title_app = app.clone();
    let load_app = app.clone();
    let first_document_load = AtomicBool::new(true);

    let builder =
        WebviewWindowBuilder::new(app, MAIN_WINDOW_LABEL, WebviewUrl::External(blank_url))
            .title("Telesram")
            .inner_size(settings.window_bounds.width, settings.window_bounds.height)
            .visible(false);
    #[cfg(target_os = "macos")]
    let builder = builder.user_agent(USER_AGENT);
    let window = builder
        .data_store_identifier(data_store_identifier)
        .devtools(true)
        .on_navigation(move |url| {
            if !navigation_settings.snapshot().managed_mode || navigation.is_internal(url) {
                return true;
            }
            if let Err(error) = navigation.open_external(&navigation_app, url) {
                eprintln!("[navigation] {error}");
            }
            false
        })
        .on_new_window(move |url, _features| {
            if !popup_settings.snapshot().managed_mode {
                return NewWindowResponse::Allow;
            }
            if popup_navigation.is_internal(&url) {
                if url.as_str() != "about:blank" {
                    if let Some(main) = popup_app.get_webview_window(MAIN_WINDOW_LABEL) {
                        if let Err(error) = main.navigate(url) {
                            eprintln!("[navigation] failed to load internal popup in main window: {error}");
                        }
                    }
                }
            } else if let Err(error) = popup_navigation.open_external(&popup_app, &url) {
                eprintln!("[navigation] {error}");
            }
            NewWindowResponse::Deny
        })
        .on_document_title_changed(move |_window, title| {
            title_app.state::<WindowState>().title_has_digits.store(
                title.bytes().any(|byte| byte.is_ascii_digit()),
                Ordering::Release,
            );
            if let Err(error) = refresh_unread(&title_app) {
                eprintln!("[tray] {error}");
            }
        })
        .on_page_load(move |window, payload| {
            if payload.event() != PageLoadEvent::Finished {
                return;
            }
            let state = load_app.state::<WindowState>();
            if payload.url().as_str() != "about:blank"
                && first_document_load.swap(false, Ordering::AcqRel)
                && state.settings().snapshot().show_on_startup
            {
                if let Err(error) = show_and_focus(&window) {
                    eprintln!("[window] failed to show Telemost after load: {error}");
                }
            }
            if let Err(error) = refresh_unread(&load_app) {
                eprintln!("[tray] {error}");
            }
        })
        .build()
        .map_err(|error| format!("failed to create main window: {error}"))?;

    let url_change_app = app.clone();
    macos::observe_url_changes(&window, move |url| {
        let state = url_change_app.state::<WindowState>();
        state.main_document.store(
            url.as_ref()
                .is_some_and(|url| state.navigation.is_main(url)),
            Ordering::Release,
        );
        if let Err(error) = refresh_unread(&url_change_app) {
            eprintln!("[tray] {error}");
        }
    })
    .map_err(|error| format!("failed to observe native Telemost URL changes: {error}"))?;

    let configured_window = window.clone();
    let configured_app = app.clone();
    macos::configure(&window, settings.managed_mode, move |result| match result {
        Ok(()) => {
            if let Err(error) = tray::ensure(&configured_app) {
                eprintln!("[tray] {error}");
            }
            let url = match Url::parse(TELEMOST_URL) {
                Ok(url) => url,
                Err(error) => {
                    eprintln!("[window] configured Telemost URL is invalid: {error}");
                    return;
                }
            };
            if let Err(error) = configured_window.navigate(url) {
                eprintln!("[window] failed to navigate to Telemost: {error}");
                return;
            }
            let reveal_requested = configured_app
                .state::<WindowState>()
                .mark_native_surface_ready();
            if reveal_requested {
                if let Err(error) = show_and_focus(&configured_window) {
                    eprintln!("[window] failed to reveal Telemost after configuration: {error}");
                }
            }
        }
        Err(error) => {
            eprintln!("[macos] failed to configure native webview: {error}");
            configured_app.exit(1);
        }
    })
    .map_err(|error| format!("failed to configure native webview: {error}"))?;

    Ok(window)
}

pub fn show_or_create(app: &AppHandle) -> Result<(), String> {
    let window = ensure_main_window(app)?;
    if app.state::<WindowState>().request_show() {
        show_and_focus(&window)?;
    }
    Ok(())
}

pub fn hide(app: &AppHandle) -> Result<(), String> {
    let window = app
        .get_webview_window(MAIN_WINDOW_LABEL)
        .ok_or_else(|| "main window is unavailable".to_owned())?;
    window
        .set_skip_taskbar(true)
        .map_err(|error| format!("failed to hide window from taskbar: {error}"))?;
    window
        .hide()
        .map_err(|error| format!("failed to hide main window: {error}"))
}

pub fn toggle(app: &AppHandle) -> Result<(), String> {
    match app.get_webview_window(MAIN_WINDOW_LABEL) {
        Some(window)
            if window
                .is_visible()
                .map_err(|error| format!("failed to check main window visibility: {error}"))? =>
        {
            hide(app)
        }
        _ => show_or_create(app),
    }
}

pub fn handle_window_event(app: &AppHandle, window: &tauri::Window, event: &WindowEvent) {
    if window.label() != MAIN_WINDOW_LABEL {
        return;
    }

    match event {
        WindowEvent::Resized(_) => {
            let Some(main) = app.get_webview_window(MAIN_WINDOW_LABEL) else {
                return;
            };
            let size = match main.inner_size() {
                Ok(size) => size,
                Err(error) => {
                    eprintln!("[window] failed to read resized window dimensions: {error}");
                    return;
                }
            };
            let scale = match main.scale_factor() {
                Ok(scale) => scale,
                Err(error) => {
                    eprintln!("[window] failed to read window scale factor: {error}");
                    return;
                }
            };
            let width = f64::from(size.width) / scale;
            let height = f64::from(size.height) / scale;
            if let Err(error) = app.state::<WindowState>().settings().update(|settings| {
                settings.window_bounds.width = width;
                settings.window_bounds.height = height;
            }) {
                eprintln!("[settings] failed to persist window bounds: {error}");
            }
        }
        WindowEvent::CloseRequested { api, .. } => {
            let state = app.state::<WindowState>();
            if state.settings().snapshot().close_to_tray && !state.is_quitting() {
                api.prevent_close();
                if let Err(error) = hide(app) {
                    eprintln!("[window] {error}");
                }
            }
        }
        _ => {}
    }
}

pub fn set_managed_mode(app: &AppHandle, enabled: bool) -> Result<(), String> {
    let window = ensure_main_window(app)?;
    let reload_window = window.clone();
    macos::set_managed(&window, enabled, move |result| match result {
        Ok(()) => {
            if let Err(error) = reload_window.reload() {
                eprintln!("[window] failed to reload after Managed Mode change: {error}");
            }
        }
        Err(error) => eprintln!("[macos] failed to change Managed Mode: {error}"),
    })
}

pub fn adjust_zoom(app: &AppHandle, delta: f64) -> Result<(), String> {
    macos::adjust_zoom(&ensure_main_window(app)?, delta)
}

pub fn refresh_unread(app: &AppHandle) -> Result<(), String> {
    let state = app.state::<WindowState>();
    let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) else {
        return Ok(());
    };
    let unread = state.main_document.load(Ordering::Acquire)
        && state.title_has_digits.load(Ordering::Acquire);
    if state.unread.swap(unread, Ordering::AcqRel) == unread {
        return Ok(());
    }
    tray::set_notification(app, unread)?;
    macos::set_dock_badge(&window, unread.then_some("•"))
        .map_err(|error| format!("failed to update Dock notification badge: {error}"))
}

fn show_and_focus(window: &WebviewWindow) -> Result<(), String> {
    window
        .set_skip_taskbar(false)
        .map_err(|error| format!("failed to restore window to taskbar: {error}"))?;
    window
        .unminimize()
        .map_err(|error| format!("failed to restore minimized window: {error}"))?;
    window
        .show()
        .map_err(|error| format!("failed to show main window: {error}"))?;
    window
        .set_focus()
        .map_err(|error| format!("failed to focus main window: {error}"))?;
    Ok(())
}
