use std::{
    ffi::c_void,
    sync::{Arc, LazyLock, Mutex},
};

use block2::RcBlock;
use objc2::{
    define_class,
    ffi::{objc_getAssociatedObject, objc_setAssociatedObject, OBJC_ASSOCIATION_RETAIN_NONATOMIC},
    msg_send,
    rc::Retained,
    runtime::{AnyObject, NSObject},
    AnyThread, DefinedClass,
};
use objc2_app_kit::{NSApplication, NSImage};
use objc2_foundation::{
    MainThreadMarker, NSData, NSDictionary, NSError, NSKeyValueChangeKey,
    NSKeyValueObservingOptions, NSString,
};
use objc2_web_kit::{
    WKContentRuleList, WKContentRuleListStore, WKUserContentController, WKWebView,
};
use tauri::WebviewWindow;
use url::Url;

const MANAGED_RULE_LIST_ID: &str = "telesram-managed-content-rules-v5";

const MANAGED_RULES: &str = include_str!("../assets/managed-rules.json");

/// Configures the native WKWebView before the caller navigates to Telemost.
///
/// Element fullscreen is enabled before `ready` is called, even without managed
/// rules. `ready` is invoked only after the managed rule list has either been
/// added or an actionable error is known. The caller must not navigate before
/// success, otherwise early requests can escape the blocker.
pub fn configure(
    webview: &WebviewWindow,
    managed: bool,
    ready: impl FnOnce(Result<(), String>) + Send + 'static,
) -> Result<(), String> {
    let ready = completion(ready);
    let scheduled = webview.with_webview({
        let ready = Arc::clone(&ready);
        move |platform| {
            unsafe {
                let native_webview = &*platform.inner().cast::<WKWebView>();
                native_webview
                    .configuration()
                    .preferences()
                    .setElementFullscreenEnabled(true);
            }
            let controller = platform.controller().cast::<WKUserContentController>();

            if let Err(error) = ensure_dock_icon() {
                complete(&ready, Err(error));
                return;
            }

            if managed {
                apply_managed_rules(controller, true, Arc::clone(&ready));
            } else {
                complete(&ready, Ok(()));
            }
        }
    });

    if let Err(error) = scheduled {
        let message = format!("failed to access the native webview: {error}");
        complete(&ready, Err(message.clone()));
        return Err(message);
    }

    Ok(())
}

/// Applies or removes the native managed-mode content rule list.
///
/// The caller reloads after this callback succeeds. Superseded requests
/// complete with an error, so an older callback cannot reload the page while a
/// newer native rule transition is still pending.
///
/// The remote page receives no Tauri IPC or native-command capability.
pub fn set_managed(
    webview: &WebviewWindow,
    enabled: bool,
    ready: impl FnOnce(Result<(), String>) + Send + 'static,
) -> Result<(), String> {
    let ready = completion(ready);
    let scheduled = webview.with_webview({
        let ready = Arc::clone(&ready);
        move |platform| {
            let controller = platform.controller().cast::<WKUserContentController>();
            apply_managed_rules(controller, enabled, ready);
        }
    });

    if let Err(error) = scheduled {
        let message = format!("failed to access the native webview: {error}");
        complete(&ready, Err(message.clone()));
        return Err(message);
    }

    Ok(())
}

type Ready = Box<dyn FnOnce(Result<(), String>) + Send>;
type Completion = Arc<Mutex<Option<Ready>>>;

struct ManagedRulesState {
    generation: u64,
    desired: bool,
    installed: bool,
}

struct ManagedRulesIvars {
    state: Mutex<ManagedRulesState>,
}

define_class!(
    #[unsafe(super(NSObject))]
    #[ivars = ManagedRulesIvars]
    struct ManagedRulesAssociation;
);

impl ManagedRulesAssociation {
    fn new() -> Retained<Self> {
        let association = Self::alloc().set_ivars(ManagedRulesIvars {
            state: Mutex::new(ManagedRulesState {
                generation: 0,
                desired: false,
                installed: false,
            }),
        });
        unsafe { msg_send![super(association), init] }
    }

    fn request(&self, desired: bool) -> u64 {
        let mut state = self
            .ivars()
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        state.generation = state.generation.wrapping_add(1);
        state.desired = desired;
        state.generation
    }

    fn needs_install(&self, generation: u64) -> Option<bool> {
        let state = self
            .ivars()
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if state.generation != generation || !state.desired {
            None
        } else {
            Some(!state.installed)
        }
    }

    fn mark_installed_if_current(&self, generation: u64) -> bool {
        let mut state = self
            .ivars()
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if state.generation != generation || !state.desired {
            return false;
        }

        state.installed = true;
        true
    }

    fn remove_if_current(&self, generation: u64) -> bool {
        let mut state = self
            .ivars()
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if state.generation != generation || state.desired {
            return false;
        }

        state.installed = false;
        true
    }

    fn is_current(&self, generation: u64, desired: bool) -> bool {
        let state = self
            .ivars()
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        state.generation == generation && state.desired == desired
    }
}

static MANAGED_RULES_ASSOCIATION_KEY: u8 = 0;

fn managed_rules_association(
    controller: *mut WKUserContentController,
) -> Result<Retained<ManagedRulesAssociation>, String> {
    unsafe {
        let key = &MANAGED_RULES_ASSOCIATION_KEY as *const u8 as *const c_void;
        let association =
            objc_getAssociatedObject(controller.cast(), key).cast::<ManagedRulesAssociation>();
        if let Some(association) = Retained::retain(association.cast_mut()) {
            return Ok(association);
        }

        let association = ManagedRulesAssociation::new();
        objc_setAssociatedObject(
            controller.cast(),
            key,
            Retained::as_ptr(&association).cast_mut().cast(),
            OBJC_ASSOCIATION_RETAIN_NONATOMIC,
        );
        Ok(association)
    }
}

fn apply_managed_rules(controller: *mut WKUserContentController, enabled: bool, ready: Completion) {
    let association = match managed_rules_association(controller) {
        Ok(association) => association,
        Err(error) => {
            complete(&ready, Err(error));
            return;
        }
    };
    let generation = association.request(enabled);

    if enabled {
        install_managed_rules(controller, association, generation, ready);
    } else {
        remove_managed_rules(controller, association, generation, ready);
    }
}

fn superseded_managed_request() -> String {
    "managed-mode request was superseded by a newer selection".to_owned()
}

fn completion(ready: impl FnOnce(Result<(), String>) + Send + 'static) -> Completion {
    Arc::new(Mutex::new(Some(Box::new(ready))))
}

fn complete(ready: &Completion, result: Result<(), String>) {
    let callback = ready
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .take();
    if let Some(callback) = callback {
        // with_webview holds Wry's window-id mutex; Tauri calls must wait for its return.
        dispatch2::DispatchQueue::main().exec_async(move || callback(result));
    }
}

fn install_managed_rules(
    controller: *mut WKUserContentController,
    association: Retained<ManagedRulesAssociation>,
    generation: u64,
    ready: Completion,
) {
    match association.needs_install(generation) {
        Some(false) => {
            complete(&ready, Ok(()));
            return;
        }
        Some(true) => {}
        None => {
            complete(&ready, Err(superseded_managed_request()));
            return;
        }
    }

    let Some(marker) = MainThreadMarker::new() else {
        complete(
            &ready,
            Err("WKContentRuleListStore requires the main thread".to_owned()),
        );
        return;
    };
    let Some(store) = (unsafe { WKContentRuleListStore::defaultStore(marker) }) else {
        complete(
            &ready,
            Err("WKContentRuleListStore.defaultStore returned nil".to_owned()),
        );
        return;
    };
    let Some(controller) = (unsafe { Retained::retain(controller) }) else {
        complete(
            &ready,
            Err("native WKUserContentController was nil".to_owned()),
        );
        return;
    };

    let identifier = NSString::from_str(MANAGED_RULE_LIST_ID);
    let rules = NSString::from_str(MANAGED_RULES);
    let completion = RcBlock::new(
        move |rule_list: *mut WKContentRuleList, error: *mut NSError| {
            if !association.is_current(generation, true) {
                complete(&ready, Err(superseded_managed_request()));
                return;
            }
            if rule_list.is_null() {
                complete(&ready, Err(content_rule_error("compile", error)));
                return;
            }

            // The controller retains the list. Keeping the controller retained in
            // this callback protects the asynchronous compile against window teardown.
            if !association.mark_installed_if_current(generation) {
                complete(&ready, Err(superseded_managed_request()));
                return;
            }
            unsafe { controller.addContentRuleList(&*rule_list) };
            complete(&ready, Ok(()));
        },
    );

    unsafe {
        store.compileContentRuleListForIdentifier_encodedContentRuleList_completionHandler(
            Some(&identifier),
            Some(&rules),
            Some(&completion),
        );
    }
}

fn remove_managed_rules(
    controller: *mut WKUserContentController,
    association: Retained<ManagedRulesAssociation>,
    generation: u64,
    ready: Completion,
) {
    if !association.is_current(generation, false) {
        complete(&ready, Err(superseded_managed_request()));
        return;
    }

    let Some(marker) = MainThreadMarker::new() else {
        complete(
            &ready,
            Err("WKContentRuleListStore requires the main thread".to_owned()),
        );
        return;
    };
    let Some(store) = (unsafe { WKContentRuleListStore::defaultStore(marker) }) else {
        complete(
            &ready,
            Err("WKContentRuleListStore.defaultStore returned nil".to_owned()),
        );
        return;
    };
    let Some(controller) = (unsafe { Retained::retain(controller) }) else {
        complete(
            &ready,
            Err("native WKUserContentController was nil".to_owned()),
        );
        return;
    };

    let identifier = NSString::from_str(MANAGED_RULE_LIST_ID);
    let completion = RcBlock::new(
        move |rule_list: *mut WKContentRuleList, error: *mut NSError| {
            if !association.is_current(generation, false) {
                complete(&ready, Err(superseded_managed_request()));
                return;
            }
            if rule_list.is_null() {
                if error.is_null() {
                    if !association.remove_if_current(generation) {
                        complete(&ready, Err(superseded_managed_request()));
                        return;
                    }
                    complete(&ready, Ok(()));
                } else {
                    complete(&ready, Err(content_rule_error("look up", error)));
                }
                return;
            }

            if !association.remove_if_current(generation) {
                complete(&ready, Err(superseded_managed_request()));
                return;
            }
            unsafe { controller.removeContentRuleList(&*rule_list) };
            complete(&ready, Ok(()));
        },
    );

    unsafe {
        store.lookUpContentRuleListForIdentifier_completionHandler(
            Some(&identifier),
            Some(&completion),
        );
    }
}

fn content_rule_error(operation: &str, error: *mut NSError) -> String {
    if error.is_null() {
        return format!("failed to {operation} the managed WKContentRuleList");
    }

    let description = unsafe { (&*error).localizedDescription() };
    format!("failed to {operation} the managed WKContentRuleList: {description}")
}

struct UrlObserverIvars {
    callback: Arc<dyn Fn(Option<Url>) + Send + Sync>,
}

define_class!(
    #[unsafe(super(NSObject))]
    #[ivars = UrlObserverIvars]
    struct UrlObserver;

    impl UrlObserver {
        #[unsafe(method(observeValueForKeyPath:ofObject:change:context:))]
        unsafe fn observe_value_for_key_path(
            &self,
            _key_path: Option<&NSString>,
            object: Option<&AnyObject>,
            _change: Option<&NSDictionary<NSKeyValueChangeKey, AnyObject>>,
            _context: *mut c_void,
        ) {
            let url = object.and_then(|object| {
                let webview = &*(object as *const AnyObject).cast::<WKWebView>();
                webview.URL()?.absoluteString()
                    .and_then(|value| Url::parse(&value.to_string()).ok())
            });
            let callback = Arc::clone(&self.ivars().callback);
            dispatch2::DispatchQueue::main().exec_async(move || callback(url));
        }
    }
);

impl UrlObserver {
    fn new(callback: Arc<dyn Fn(Option<Url>) + Send + Sync>) -> Retained<Self> {
        let observer = Self::alloc().set_ivars(UrlObserverIvars { callback });
        unsafe { msg_send![super(observer), init] }
    }
}

static URL_OBSERVER_ASSOCIATION_KEY: u8 = 0;
static DOCK_ICON_INSTALLED: LazyLock<Result<(), String>> = LazyLock::new(|| {
    let marker = MainThreadMarker::new()
        .ok_or_else(|| "NSApplication requires the AppKit main thread".to_owned())?;
    let image_data = NSData::with_bytes(include_bytes!("../assets/tray-blue.png"));
    let image = NSImage::initWithData(NSImage::alloc(), &image_data)
        .ok_or_else(|| "could not decode assets/tray-blue.png for the Dock".to_owned())?;
    let application = NSApplication::sharedApplication(marker);
    unsafe { application.setApplicationIconImage(Some(&image)) };
    Ok(())
});

/// Invokes `on_change` for every native WKWebView URL mutation, including
/// history and hash-only navigation, without exposing any remote-page IPC.
pub fn observe_url_changes(
    webview: &WebviewWindow,
    on_change: impl Fn(Option<Url>) + Send + Sync + 'static,
) -> Result<(), String> {
    let callback: Arc<dyn Fn(Option<Url>) + Send + Sync> = Arc::new(on_change);
    webview
        .with_webview(move |platform| unsafe {
            let native_webview = &*platform.inner().cast::<WKWebView>();
            let native_pointer = native_webview as *const WKWebView as *mut AnyObject;
            let key = &URL_OBSERVER_ASSOCIATION_KEY as *const u8 as *const c_void;

            if !objc_getAssociatedObject(native_pointer, key).is_null() {
                return;
            }

            let observer = UrlObserver::new(callback);
            let url_key_path = NSString::from_str("URL");
            let observer_pointer = Retained::as_ptr(&observer).cast_mut().cast::<NSObject>();
            let (): () = msg_send![
                native_webview,
                addObserver: observer_pointer,
                forKeyPath: &*url_key_path,
                options: NSKeyValueObservingOptions::Initial | NSKeyValueObservingOptions::New,
                context: std::ptr::null_mut::<c_void>()
            ];
            objc_setAssociatedObject(
                native_pointer,
                key,
                Retained::as_ptr(&observer).cast_mut().cast(),
                OBJC_ASSOCIATION_RETAIN_NONATOMIC,
            );
        })
        .map_err(|error| format!("failed to observe native WKWebView URL changes: {error}"))
}

pub fn adjust_zoom(webview: &WebviewWindow, delta: f64) -> Result<(), String> {
    webview
        .with_webview(move |platform| unsafe {
            let native_webview = &*platform.inner().cast::<WKWebView>();
            let zoom = crate::menu::adjusted_zoom(native_webview.pageZoom(), delta);
            native_webview.setPageZoom(zoom);
        })
        .map_err(|error| format!("failed to adjust the native page zoom: {error}"))
}

/// Sets the Dock tile badge on AppKit's main thread. The upstream indicator is
/// a bullet (`•`), not an unread-count string.
pub fn set_dock_badge(webview: &WebviewWindow, label: Option<&str>) -> Result<(), String> {
    let label = label.map(ToOwned::to_owned);
    webview
        .run_on_main_thread(move || {
            let marker = MainThreadMarker::new()
                .expect("Tauri run_on_main_thread must execute on the AppKit main thread");
            let application = NSApplication::sharedApplication(marker);
            let dock_tile = application.dockTile();
            let label = label.as_deref().map(NSString::from_str);
            dock_tile.setBadgeLabel(label.as_deref());
        })
        .map_err(|error| format!("failed to set the Dock badge: {error}"))
}

fn ensure_dock_icon() -> Result<(), String> {
    DOCK_ICON_INSTALLED.clone()
}
