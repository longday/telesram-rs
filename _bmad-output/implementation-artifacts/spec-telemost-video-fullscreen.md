---
title: 'Enable fullscreen video in Telemost calls'
type: 'bugfix'
created: '2026-09-25'
status: 'done'
route: 'oneshot'
review_loop_iteration: 0
context: []
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** The fullscreen button is absent during a Telemost video call in Telesram but available in a normal browser. The native WKWebView defaults to disabling its Fullscreen API.

**Approach:** Enable element fullscreen in the existing native WebKit configuration before loading Telemost, without changing the user agent, popup routing, or permissions. Keep the shared login profile intact. Validate native fullscreen capability locally; a real call still needs user-assisted verification.

</frozen-after-approval>

## Implementation Notes

`src/macos.rs::configure` runs before the initial navigation from `src/window.rs::ensure_main_window`. `WKWebViewConfiguration::preferences().setElementFullscreenEnabled(true)` requires the `WKPreferences` feature in `Cargo.toml`'s `objc2-web-kit` dependency. `swift -e` measured the native default as false. Do not restart the already-running authenticated app for a fixture check. Existing `README.md` documents media verification limits.


Enabled `WKPreferences` in `Cargo.toml` and element fullscreen in `src/macos.rs::configure` before navigation. `cargo test --locked --features custom-protocol` passed (2 tests). A fresh native `WKWebView` with the preference enabled reported `document.fullscreenEnabled=true` after loading a video element; the default configuration reported `false`. The already-running authenticated app was not replaced or restarted, so its call UI remains unverified. Updated `README.md` with this limit.

The follow-up fixture sets the preference **after** creating the view, like `configure`:

```sh
swift -e 'import Cocoa; import WebKit; let app = NSApplication.shared; let view = WKWebView(frame: NSRect(x: 0, y: 0, width: 320, height: 240), configuration: WKWebViewConfiguration()); print("before=\(view.configuration.preferences.isElementFullscreenEnabled)"); view.configuration.preferences.isElementFullscreenEnabled = true; view.loadHTMLString("<video controls></video>", baseURL: nil); DispatchQueue.main.asyncAfter(deadline: .now() + 1) { view.evaluateJavaScript("document.fullscreenEnabled") { result, error in print("after=\(String(describing: result)), error=\(String(describing: error))"); exit(error == nil && (result as? Bool) == true ? 0 : 1) } }; RunLoop.main.run()'
```

Observed `before=false`, `after=Optional(1), error=nil`. Release build and formatting check passed. The new binary is only in `.runtime/target/release/`; the running signed `.app` remains unchanged until it is quit and `./start.sh` is run.

## Review Triage Log

- Medium: the initial fixture configured fullscreen before creating the view; the post-creation fixture now covers the live WebKit API path, though not a real call.
- Low: missing README paragraph break; added.
- Low: README did not list the pending fullscreen check or its scope; clarified open item, all-page behavior, and the manual checks. A duplicate item in Controls adds no information.
- Low: the native configuration comment omitted the fullscreen dependency; updated.
- Low: the callback's unsafe scope included unrelated operations; narrowed.
- False: spec status was in-progress because review had not yet finalized; marked done at workflow completion. No staging or commit without user approval.
- Low: the original fixture command was not preserved; recorded the repeatable post-creation command above.
- Maybe-false: hiding/closing the Tauri window during WebKit element fullscreen might affect the video or saved geometry. No call was run; the README now asks for the tray interaction check.