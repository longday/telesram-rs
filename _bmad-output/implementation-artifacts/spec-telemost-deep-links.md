---
title: 'Open Telemost deep links in Telesram'
type: 'feature'
created: '2026-09-29'
status: 'done'
route: 'dispatch'
review_loop_iteration: 0
context: []
baseline_commit: '63389a4f9adb28cd6e8563d7563e0e43cc770606'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** The Telemost website offers to open its native application, but Telesram neither declares the native URL scheme nor navigates to the requested destination. The original Telemost client will not be installed.

**Approach:** Register Telesram for `telemost`, decode the two verified website link formats, and open their destination in the existing main webview. Support application launch, an already-running application, and window recreation. Registration without destination delivery does not satisfy this feature.

## Boundaries & Constraints

**Always:** Accept `telemost://https://telemost.yandex.ru/j/<digits>` and the equivalent `telemost.360.yandex.ru` meeting URL. Accept `telemost://ychat/<host>/<path, query and fragment>` for those two exact Telemost hosts, reconstructing HTTPS. Validate external input independently of Managed Mode. Reject credentials, nonstandard ports, unexpected hosts, malformed payloads, and unsupported schemes. Preserve valid chat paths, queries, and fragments. Set a single `skip_app=1` query parameter on decoded destinations to prevent automatic native-app relaunch. A received valid link reveals and focuses the main window regardless of Show on Startup. Pending navigation must wait until native WebKit configuration and Managed Mode rules are ready. If several valid links arrive before readiness, the latest destination wins; invalid links do not clear it.

Use the existing WebKit login profile without importing credentials. Registration and smoke checks run as the current user without sudo, Apple account authentication, or new camera/microphone permissions. Only the release bundle is deliberately registered for end-to-end verification. Debug metadata must inherit the same declaration through the existing Info.plist copy.

**Never:** Register HTTPS globally, claim unrelated `ychat` or Messenger schemes, add a custom Telesram scheme, install the original client, change login data, delete other bundles, or terminate a user-owned process. Do not automatically join a live meeting for verification. Native launch navigation must not bypass Managed Mode setup. Do not log complete incoming URLs or sensitive query values.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|---|---|---|---|
| Meeting | Verified nested HTTPS format | Same host and meeting ID, with skip_app=1 | Reject nonnumeric/missing meeting ID |
| Messenger | Verified ychat wrapper on allowed host | Preserve route, query, fragment over HTTPS | Reject other authorities |
| Cold launch | Valid link while stopped | One configured window at destination | Surface navigation errors without logging the URL |
| Warm launch | Valid link while running or hidden | Reuse and reveal existing window | Do not spawn a second persistent instance |
| Configuration pending | Multiple links before native readiness | Navigate to latest valid destination after configuration | Invalid later input does not overwrite pending URL |
| Host spoofing | Subdomain/lookalike, credentials, foreign host, port | No navigation and no external opener fallback | Concise rejection diagnostic |
| Ordinary launch | No incoming link | Existing home/startup behavior | Existing handling |

</frozen-after-approval>

## Code Map

- `Info.plist`: shared extra bundle metadata; Tauri release bundling merges it and `dev.sh:48` copies it. No launcher edit is needed for scheme declaration.
- `src/main.rs`: single-instance callback currently ignores arguments; setup installs WindowState before creating the webview; run callback handles Reopen but not Opened.
- `.runtime/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tauri-2.11.5/src/app.rs`: RunEvent::Opened provides Vec<url::Url> on macOS. Use existing Tauri support without a new plugin dependency.
- `src/window.rs`: WindowState tracks native_surface_ready; ensure_main_window initially creates about:blank; macos::configure asynchronously installs native settings and rules before navigating home. Integrate pending destination with this boundary rather than navigating early.
- `src/navigation.rs`: existing Managed Mode host policy includes authentication hosts; do not reuse that broader policy as deep-link validation.
- `build.sh`: locked host release build, strict signature verification, refuses to rebuild while Telesram runs. Output: `.runtime/target/release/bundle/macos/Telesram.app`.
- `README.md`: run and verification documentation; preserve existing unverified-feature caveats.

## Tasks & Acceptance

**Execution:**
- [x] `src/deep_link.rs` — add pure decoding and consumer-visible boundary tests for the matrix, using the existing url dependency.
- [x] `src/window.rs` — add pending navigation and a destination-opening entry point integrated with configuration readiness, focus, and recreation.
- [x] `src/main.rs` — handle native Opened events and relevant launch arguments through the same validated path; preserve ordinary activation.
- [x] `Info.plist` — declare telemost under CFBundleURLTypes.
- [x] `README.md` — document supported formats, local registration, startup behavior, and observed verification limits.
- [x] Release bundle — rebuild, register with macOS LaunchServices after explicit system-change approval, and smoke both cold and warm delivery. Do not manipulate LaunchServices database files directly.

**Acceptance Criteria:**
- Given the registered release bundle, when a telemost URL is dispatched by macOS, then Telesram receives and opens the decoded destination without a second application instance.
- Given a ready or configuring main webview, when a valid URL arrives, then the final visible destination is the requested route and native configuration is not bypassed.
- Given either supported wrapper, when a malicious authority is supplied, then no web navigation occurs even with Managed Mode disabled.
- Given normal startup without links, when launched, then existing home and Show on Startup behavior remains unchanged.

## Implementation Notes

- Added the already-locked parking_lot 0.12.5 as a direct dependency to comply with the injected mutex rule. No dependency version changed.
- A compiled probe confirmed Tauri's URL normalization removes the empty-port colon in the nested HTTPS wrapper; decoding supports both raw website and native normalized representations.
- Five unit tests passed; release build and strict signature verification passed. LaunchServices selected the release bundle. Cold ychat delivery opened Threads; warm delivery reused the same process and restored a window closed to the tray. The user confirmed nonexistent-meeting errors from synthetic call links. No live meeting was joined.
- User approved implementation, release build, and OS registration through the interactive checkpoint. The existing user-owned process was closed by the user before building.
- Final verification: eight locked tests passed, including pending navigation/consume-once, raw query preservation, password-only credential rejection, and both-host unread eligibility. Final release build and strict signature verification passed; its CFBundleURLTypes and LaunchServices registration were checked.
- Final CLI-queue smoke launched the rebuilt bundle and rejected the invalid trailing destination without exposing its URL. Visual re-inspection was blocked after the desktop session locked (CGSSessionScreenIsLocked=1); the process main thread was running its normal AppKit event loop. Earlier native URL cold/warm and Close to Tray checks remain the UI evidence; no claim is made for a live-call join.

## Spec Change Log

## Review Triage Log

| Finding | Verdict and disposition |
|---|---|
| Blind: rejected argv suppresses ordinary restore | low; patched received to become true only after validated scheduling, preserving second-instance activation on rejected arguments. Native rejected events still deliberately do not navigate. |
| Blind: skip_app rewrites other query bytes | medium; reproduced with bare flags, slash and percent-encoded space; fixed raw-segment preservation and added failing-before/passing-after regression. |
| Blind: consumer host disables unread tracking | medium; reproduced by NavigationPolicy regression and fixed under the user's added unread-indicator request. |
| Blind: active call replaced by a new link | false as a contract violation; the approved behavior intentionally opens a supplied destination in the same webview, like Home. No active-call preservation or confirmation was requested; a path alone would not establish call state. |
| Blind: lifecycle and argv verification gap | low; added pending/ready/recreation state regression and exercised startup argv. Native UI smoke covers cold/warm URLs and tray restoration; final argv visual observation was blocked by screen lock. |
| Blind: inner wrapper case handling | low, rejected; observed first-party wrapper literals are lowercase. Adding alternate payload spelling handling is not needed for these formats; the outer URI scheme remains case-insensitive. |
| Blind: failed navigation does not reveal window | low; moved reveal before warm navigation. Retained a URL-free error message to honor the input-privacy boundary. |
| Blind: URL type name not reverse-DNS | low; corrected to dev.longday.telesram.telemost. |
| Blind: use std Mutex rather than parking_lot | false; an injected mandatory rule requires parking_lot for this lock use. The dependency was already locked transitively. |
| Blind: extra README blank line | low; removed. |
| Edge: rejected argv suppresses restore | low; same accepted-only received patch, independently recorded. |
| Edge: non-Unicode first-launch arguments panic | low; startup now reads args_os and ignores non-UTF-8 non-URL input rather than panicking. |
| Verification: pending latest-wins/consume-once gap | low; added a real WindowState transition test covering queued, ready and recreated states. Invalid URLs are rejected before the scheduling closure can touch the pending state. |
| Verification: password-only credentials gap | low; added both wrapper forms with an empty username and nonempty password to rejection tests. |
| Verification: startup argument handling gap | low; exercised the rebuilt bundle with ordered valid/invalid URL arguments; screen lock prevented final visual inspection. Decoder and queued state decisions have permanent regression coverage. |
| Verification: query re-encoding | medium; same raw-query regression and patch, independently recorded. |

- Focused follow-up review found no new defects in decoding and pending navigation. No deferred code changes remain.

## Design Notes

Website evidence: https://yastatic.net/s3/chat-static/telemessenger/_/212.3.0/web/app.js. Module 51637 prefixes an HTTPS meeting URL with telemost://; module 91311 constructs /j/<meetingId> on the consumer or business host. The messenger route builder replaces https:// with telemost://ychat/. The meeting auto-launch registration checks skip_app=1.

The change is cohesive but spans parsing, asynchronous startup, packaging, and persistent OS registration. Use the full approval path rather than treating it as a metadata-only edit. There are no unresolved user-intent questions; system registration is an explicit approval gate. No data migration or deletion is planned.

## Verification

- Run cargo test --locked --features custom-protocol with RUSTUP_HOME, CARGO_HOME, and CARGO_TARGET_DIR from build.sh; boundary tests must reject hostile URLs and preserve valid routing.
- Run ./build.sh on the host; inspect the built CFBundleURLTypes and strict code signature.
- After approval, register the release bundle and dispatch safe test URLs through macOS. Observe cold launch, warm hidden-window activation, destination URL, and process count in the actual application. A nonexistent numeric meeting may show an unavailable-meeting screen; do not claim a live-call join was tested.
- Exercise the ychat route and invalid-link handling, then leave the intended release registration in place. Remove only temporary verification artifacts and processes created by this task. If a running user-owned instance prevents rebuild, request that the user quit it rather than killing it.
