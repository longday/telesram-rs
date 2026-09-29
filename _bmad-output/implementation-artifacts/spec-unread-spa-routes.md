---
title: 'Keep unread indicators active across Telemost SPA routes'
type: 'bugfix'
created: '2026-09-29'
status: 'done'
route: 'oneshot'
review_loop_iteration: 0
context: []
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

The user repeatedly observed missing unread indicators while Telemost still displayed unread messages. Keep the tray indicator and Dock badge synchronized with the existing title-digit signal throughout navigation within either supported Telemost HTTPS origin. Preserve rejection of foreign hosts, credentials, insecure schemes, and nonstandard ports. Do not read or mutate messages, change authentication or system permissions, or introduce polling or another unread source.

</frozen-after-approval>

## Implementation Notes

- A live recurrence in release PID 35934 exposed `https://telemost.360.yandex.ru/threads` through native Accessibility. The page showed three unread messages while the tray tooltip was normal and Dock AXStatusLabel absent. Restarting returned to `/` and restored both indicators.
- `src/navigation.rs::is_main` incorrectly requires the root pathname. `src/window.rs` observes native URL changes and clears its main-document flag on `/threads`. Remove pathname gating: SPA paths do not change the trusted Telemost origin. Remove the now-unused pathname normalizer.
- Extend the existing navigation-policy regression to real path routes and retain origin-boundary rejections. The previous `/j/123` rejection encoded the obsolete root-only restriction; meetings on a supported origin use the same title signal.
- Native smoke must exercise root -> history.pushState('/threads') without changing the unread title, then clearing the title; both native indicators must remain set across the route transition and clear afterward. Use an isolated nonpersistent fixture, not real chat mutations.
- Filtering work remains a separate pending task. Do not mix its changes into this fix.
- The captured-route regression failed on `/threads` before the fix. Afterward all eight tests passed. A disposable native fixture using production modules and nonpersistent WebKit proved root -> `/threads` -> `/chats/example` retains the unread tooltip and Dock badge, then clears both when the title loses its counter. Native button renders showed distinct normal/unread silhouettes.
- This fix deliberately supersedes the root-path restriction recorded in earlier desktop-app and template-indicator specifications. Historical frozen intent is not rewritten; README describes the new origin-based behavior.

## Review Triage Log

- low, patched: README described roots rather than origins/routes; updated it. Earlier frozen specifications remain historical, with the supersession recorded here.
- maybe-false, medium, deferred: a meeting title containing non-unread digits could trigger the existing title-digit heuristic now that route gating is removed. The public `/j/123?skip_app=1` response has title `Яндекс Телемост`, without numeric ID; titles during real calls have not been inspected. Do not invent a new title parser or route exception without evidence.
- low, patched: origin-boundary negatives now use `/threads` and exercise both supported hosts rather than only root paths.
- low, rejected: replacing stored `Url` with an owned host or renaming `is_main` would expand this fix without changing behavior; parsing/storage occurs only at startup and the caller remains unchanged.
