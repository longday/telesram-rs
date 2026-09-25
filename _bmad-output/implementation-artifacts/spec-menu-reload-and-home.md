---
title: 'Add Reload and Home controls to the application menu'
type: 'feature'
created: '2026-09-26'
status: 'done'
route: 'oneshot'
review_loop_iteration: 0
context: []
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** The Control menu has no direct way to refresh the current page or return to Telemost's starting page.

**Approach:** Add Reload and Home items to the existing Control menu. Reload refreshes the main webview's current document; Home navigates that webview to the same configured Telemost URL used at startup, without changing settings or the WebKit profile.

</frozen-after-approval>

## Implementation Notes

- `src/menu.rs`: Reload Page and Home items after the toggles, separated.
- `src/window.rs`: `reload`/`go_home` act only after native configuration (`native_surface_ready`); earlier, configuration loads Telemost itself and loading sooner would bypass Managed Mode rules. With no window, both show a new window, which loads Telemost.
- `src/main.rs`: dispatches the new actions. `README.md`: control menu list.
- Smoke (dev build, managed profile): Home from Threads returned to «Главная»; Reload Page reset the selected chat tab. No-window path not exercised: Close to Tray is on, so closing only hides.

## Review Triage Log

- Reload/Home with no window created a hidden window and did nothing visible — medium, patched: route through `show_or_create`.
- README control menu list omitted new items — low, patched.
- `go_home` re-parses `TELEMOST_URL` with an unreachable error — low, rejected: `WindowState::new` already validates it; fix needs a new accessor.
- No Cmd+R shortcut for Reload — deferred: outside intent.
- "Home" label unclear — rejected: label specified by intent.
