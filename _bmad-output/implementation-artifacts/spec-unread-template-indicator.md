---
title: 'Make unread notifications visible in the template tray icon'
type: 'bugfix'
created: '2026-09-29'
status: 'done'
route: 'oneshot'
review_loop_iteration: 0
baseline_commit: '63389a4f9adb28cd6e8563d7563e0e43cc770606'
context: []
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

The user reported that the icon does not change when messages are unread, then requested autonomous completion, commit, and push. Make unread and cleared states visibly distinct while preserving macOS template rendering, tooltip changes, Dock badges, and the existing title-based unread signal. Keep unread tracking active on either supported Telemost messenger root after a deep link. Do not read message contents, mutate chat read status, introduce another unread data source, or change authentication and system permissions.

</frozen-after-approval>

## Implementation Notes

- `src/tray.rs` uses template icons at creation and updates. `assets/tray-blue-22.png` and the original `assets/tray-red-22.png` had identical alpha masks; their only difference was color, which template rendering ignores.
- A disposable native AppKit probe rendered the actual images through NSStatusBarButton: normal and unread produced zero differing pixels before the change. Added a separate notification dot to the unread asset's alpha silhouette, preserving template behavior. The final probe reports 284 differing rendered pixels; clearing unread returns exactly to the normal render. Visual comparison confirmed the dot. Screen capture of the probe's status item was unavailable because macOS placed it offscreen; view-cache rendering was used without changing menu-bar layout.
- `src/navigation.rs::is_main` previously rejected the consumer Telemost host, disabling unread tracking there. A regression test failed on the consumer root before the fix. The predicate now admits both exact HTTPS roots, query strings, and hash routes, while rejecting foreign hosts, authentication pages, credentials, nonstandard ports, and non-root documents.
- No image-only unit test was added: the native render comparison directly exercises the reported visual failure. Keep the navigation regression because it verifies consumer-visible eligibility rather than asset bytes.
- Review and final release verification are shared with the deep-link change; the application and WebKit profile remain the same.
- Final locked tests passed (8/8); release build and signature verification passed. The final live-app observation was limited by the locked desktop, not by changed permissions. No live message receipt was simulated or chat read state changed.

## Review Triage Log

- low, patched: the first draft asset re-rasterized unrelated glyph pixels and clipped the dot edge. Rebuilt only the badge/knockout alpha masks with BOX sampling; original glyph pixels outside (13,1)-(21,10) remain unchanged and the dot is inset.
- low, patched: the first host predicate ignored the configured main host. Restored self.main_url.host() as the primary host and added the consumer-host alias explicitly. A shared host-registry refactor was not needed for this correction.
- The reviewer found no other functional errors. No deferred changes remain.
