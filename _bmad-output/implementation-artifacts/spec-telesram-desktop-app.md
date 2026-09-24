---
title: 'Implement the personal macOS Yangertron replacement'
type: 'feature'
created: '2026-09-22'
status: 'in-progress'
route: 'dispatch'
baseline_commit: '806080dc6fb68c128f9fbabc8bd83f7d6e84ca2e'
---

<frozen-after-approval reason="user-owned product scope">

## Intent

Build the actual telesram-rs application, not another prototype: a Rust/Tauri 2 replacement for Yangertron on macOS Apple Silicon, built locally for personal use. Preserve all applicable user-visible behavior from upstream commit `c6df711b3acff9b4e6208197de93a34c9fc9da13`, documented by YFI-01–YFI-12 in the source inventory. Reduce application size and total attributable RAM without dropping functionality; numeric targets follow equivalent measurements.

## Boundaries & Constraints

**Always:** native host development without containers; exact direct dependency versions and Cargo lockfile; local release build and one documented launch entrypoint; persistent settings and web session; narrowly scoped native access for remote content; explicit reporting of unverified compatibility.

**Never:** substitute a partial wrapper for full parity, invent a messenger backend or new frontend design, expose unrestricted native commands to Yandex content, add other platforms, publish installers, change global toolchains, commit or push without approval, enter credentials or grant privacy permissions on the user's behalf.

The user performs login and macOS permission actions during assisted verification. Runtime session data belongs to this application, not the existing Yangertron profile. Automatic import of another browser's session is not required.

Minimum supported macOS: 26.0, Apple Silicon only (approved by the user).

## I/O & Edge-Case Matrix

| Scenario | Required behavior |
| --- | --- |
| Startup and second instance | Load the configured Yandex Messenger endpoint; a second launch reveals/focuses the existing application rather than opening a duplicate. |
| Window state | Restore saved positive width/height (initial 1280×800); Close to Tray and Show on Startup initially enabled; persist the three upstream toggles and geometry. |
| Close, reopen, quit | Respect Close to Tray; native activation restores/recreates the window; explicit Quit terminates rather than hiding. |
| Menus and tray | Messenger menu includes Managed Mode, Close to Tray, Show on Startup, Direct/profile proxy choices, and zoom ±0.1 clamped to 0.25–3.0. Tray supports show/hide/quit and click toggling. |
| Unread signal | Preserve the exact-main-URL plus title-digit heuristic, template tray imagery, tooltip change, and macOS Dock badge. |
| Managed Mode off | Preserve ordinary login/navigation and popup behavior; do not apply the analytics filter or custom stylesheet. |
| Managed Mode on | Reload on toggle; internal popup targets load in the main window, external navigation opens in the default browser; apply upstream CSS and host/path request rules while exempting document frames. |
| Proxy selection | Read upstream-compatible YAML profiles; preserve optional credentials and bypass rules; Direct is explicit; selection persists and applies at next startup, not silently midway through a session. |
| Proxy failure | Preserve source-defined profile parsing behavior and diagnostics; never claim an authenticated/bypass profile works merely because an endpoint can be selected. |
| Media | Enable camera/microphone usage declarations and normal OS consent. Retain browser screen-sharing functionality; verify the actual messenger call with user assistance. |
| Session restart | An authenticated application session remains available after restart unless the user/service expires it. No plaintext application-owned copy of Yandex credentials. |

</frozen-after-approval>

## Approved scope amendment: remove proxy support

The user explicitly removed proxy support and approved the deletion plan. This amendment supersedes the frozen historical proxy requirements above and the proxy tasks, acceptance criteria, and architecture blockers below. All other requirements remain.

- Retain Rust/Tauri/WKWebView. Cancel private-PAC and CEF research/experiments.
- Remove `src/proxy.rs`, `src/proxy/`, `proxy.example.yml`, proxy controls, profile selection, native proxy configuration, and adapter-only dependencies.
- Use WebKit's normal networking, which may inherit system proxies/PAC. Do not change system settings or force Direct.
- Stop reading `proxy.yml`. Preserve any user-owned file and its ignore rule. Discard the obsolete `proxyProfileId` field on the next settings write; preserve unrelated settings.
- Verify the locked release build, actual startup, remote page loading, and remaining native menu controls. Login, OS consent, real calls/sharing, and comparative resource measurements remain open.

## Approved scope amendment: current Telemost service

On 2026-09-23 the user chose to replace the retired Messenger destination with Yandex Telemost. This supersedes the frozen Messenger URL, visible Messenger identity, and Messenger-specific navigation target above; it does not restore proxy support or change the remaining window, settings, and media requirements.

- Start at `https://telemost.360.yandex.ru`, matching the existing Telemost desktop refactor in the sibling Yangertron project. Preserve the current Telesram bundle identifier and WebKit data store so existing application state is not discarded.
- In Managed Mode, keep Telemost's two exact HTTPS hosts and the authentication hosts `passport.yandex.ru`, `id.yandex.ru`, and `cookier.360.yandex.ru` inside the native window, following the sibling desktop refactor. External links still open in the default browser. Do not admit all `*.yandex.ru` hosts or insecure HTTP variants.
- Update native menu, window, tray, and privacy-purpose text to Telemost. Keep the application name Telesram and upstream media/settings controls.
- Keep the existing analytics request blocker, but remove the obsolete Messenger-only `.yamb-global-bar` stylesheet rule and unused asset; do not apply a guessed Telemost-specific CSS change.
- Given a fresh anonymous launch, when the remote page loads, then the native window displays the current Telemost landing page rather than the Messenger migration page. Given an allowed authentication or Telemost URL in Managed Mode, when navigating, then it remains inside the app; an unrelated URL opens externally. Login, calls, and remote screen-share delivery require user-assisted verification before completion.

## Approved scope amendment: developer tools

On 2026-09-24 the user requested a Telemost menu item that opens Web Inspector in a separate window when possible, then confirmed that opening docked inside the main window is acceptable. This personal local release may enable Tauri's `devtools` feature; it uses private WebKit inspector APIs and is not for App Store distribution. Leave inspector placement to WebKit; do not expose native IPC or inspect the user's authenticated profile for this smoke check.

- Given the local release build, selecting Developer Tools from the Telemost menu opens Web Inspector for the main webview. Docked presentation is acceptable; WebKit's Detach control can move it to a separate window.

## Approved scope amendment: Managed Mode filter

On 2026-09-24 the user asked to hide `div.yamb-global-bar` in Managed Mode and expand analytics blocking with uBlock lists, without indiscriminately blocking Yandex. The selected implementation pins EasyPrivacy and uBlock Privacy source snapshots and converts supported network block/exception rules to native WebKit content rules. Unsupported uBlock syntax is skipped and counted, not silently approximated; document navigations and Telemost/authentication service hosts stay exempt. No proxy, uBlock engine, or custom request interception is added.

- With Managed Mode enabled, the bar is hidden and supported trackers are blocked; with it disabled, the bar is visible and content rules are absent. The user-owned working WebKit profile must not be inspected as part of this check.

## Approved scope amendment: launch modes and WebKit reset

On 2026-09-24 the user requested separate `start.sh`, `dev.sh`, and `reset.sh` scripts. `start.sh` builds release only when its cached binary is absent; subsequent invocations launch that binary without invoking Cargo, even if sources changed. `dev.sh` builds debug on each invocation. The user selected a shared bundle identifier, WebKit profile, and application settings for both modes. Neither script replaces a running application. `reset.sh` must require a stopped app and explicit terminal confirmation before deleting only `~/Library/WebKit/dev.longday.telesram/`; it does not run implicitly, remove `.runtime`, or alter macOS privacy permissions. A reset signs out both modes. The implementation must not invoke reset on the working profile during verification.

- Given a cached release build and no running app, when `start.sh` runs twice, then Cargo is not invoked; after `dev.sh`, `start.sh` restores the cached release binary without recompilation. Given edited sources, production changes only after an explicit release build.
- Given an app-specific WebKit data directory, when `reset.sh` is run interactively with exact confirmation and Telesram stopped, then only that directory is removed. A cancelled, noninteractive, symlinked, or running-app attempt leaves it intact.

## Code Map

- `../planning-artifacts/briefs/brief-telesram-rs-2026-09-22/brief.md`: agreed personal-use scope and container-free constraint.
- `../planning-artifacts/research/yangertron-feature-inventory.md`: pinned behavior/defaults and original source permalinks; authoritative parity checklist.
- `../planning-artifacts/research/tauri-macos-feasibility.md`: native API gaps and research-version caveats.
- `../planning-artifacts/research/screen-sharing-probe.md`: successful local capture evidence, not Yandex-call validation.
- `src/`, `assets/`, `capabilities/`, `Cargo.toml`, `tauri.conf.json`, `Info.plist`: native application implementation. No container setup is used.
- `start.sh`, `README.md`: project-local release build, runtime paths, and current verification limits. Keep BMAD installation and prior planning artifacts intact.

## Tasks & Acceptance

**Execution:**
- `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`, `build.rs`, `tauri.conf.json`, `Info.plist`, `assets/`, `capabilities/`: create the native package, pinned dependencies, privacy declarations, upstream imagery/CSS, and an empty native IPC capability. No frontend framework is needed.
- `src/main.rs`, `src/config.rs`, `src/settings.rs`: lifecycle, configuration, and persistent settings. Keep Rust builds/caches/settings under ignored `.runtime/`; ignore Tauri's fixed generated ACL metadata path `gen/schemas/`. Disclose OS-managed WebKit session storage.
- `src/window.rs`, `src/menu.rs`, `src/tray.rs`: integrate remote content, native controls, title/URL observation, badge, zoom, geometry, and close/quit/activation transitions.
- `src/navigation.rs`, `src/macos.rs`: URL policy, native WebKit content rules, media configuration, and documented native access; do not use Tauri's custom-protocol request hook as a fake external HTTPS blocker.
- `src/proxy.rs`, `src/proxy/`, `proxy.example.yml`: upstream-compatible profile parsing/selection and a local Rust forwarding adapter. Bind only an ephemeral loopback endpoint, configure WebKit to use it before navigation, and retain it for the application session. Rust makes Direct/bypass/upstream decisions without inheriting system proxy settings; remote TLS remains end-to-end. Credentials never reach origin servers or logs.
- `start.sh`, `.gitignore`, `README.md`: one native startup path, local runtime paths, required tooling, profile setup, permission instructions, and verification limits. No global installations or shell-profile changes.

**Acceptance Criteria:**
- Given the documented native prerequisites, when the user launches the application, then the real Yandex Messenger page opens in the native application and the menu/tray controls work.
- Given changed settings, when the application restarts, then geometry, toggles, and next-launch proxy selection are preserved.
- Given Managed Mode, when representative internal/external links and analytics resource requests occur, then navigation and blocking match the upstream rules without breaking document navigation.
- Given a reachable configured proxy, when authentication/bypass scenarios are exercised, then credentials and exclusions affect actual requests, not only stored configuration.
- Given a user-authenticated messenger session and granted privacy access, when the user makes a call and shares a source, then the remote participant receives the intended stream; mark unexercised scenarios explicitly until user-assisted verification is available.
- Given comparable release builds and workloads, when size/RAM are measured, then report both full artifact size and the application-attributable process set; do not assert savings without measurements.

## Implementation Notes

The application request supersedes the earlier prototype-only restriction. The pinned Rust toolchain, dependency cache, and build outputs are project-local under `.runtime/`. Existing planning changes are preserved and no commit is planned. Native WebKit behavior is checked against the actual Cargo lockfile and native runtime, not a development-branch manifest.

The user approved a local Rust proxy adapter after native API inspection established that `WKWebsiteDataStore.proxyConfigurations` falls back to system/PAC settings when cleared and Network.framework exclusions cannot represent Chromium `<local>`. The native endpoint-only implementation is replaced, not retained as a second path.

## Verification

Run a locked native build; targeted tests only for genuinely uncertain URL/filter/proxy semantics and persistent-state transitions. Exercise the real native menu, tray, windows, and restart path. Use local controlled request fixtures for filter and proxy observations, then user-assisted Yandex login/calls. Keep UI evidence and observed outcomes distinct from source-only claims. Check release metrics separately from debug behavior. No test suite substitutes for native interaction or authenticated integration evidence.

### Observed checks — 2026-09-22

- `cargo fmt --all --check` and `cargo test --locked --features custom-protocol` passed; 12 tests. The optimized locked release build completed.
- Native menu/tray callbacks, startup visibility, saved geometry, close/hide/recreate/quit, second-instance activation, and native application reopen were exercised. A second launch with an invalid newly selected profile now reaches the existing instance before profile initialization.
- Native unread title/URL transitions and zoom clamps passed. Recreated windows adjust from their actual WebKit zoom rather than stale application state.
- A native regression probe reproduced hidden-window reappearance after a later document load, then passed after startup visibility was limited to the first nonblank document. The probe was disposable because it requires an isolated native application instance and startup settings.
- Managed-mode rapid toggles, host/path filtering, allowed query/path lookalikes, document-frame exemptions, native popup behavior, and external default-browser navigation were exercised with controlled pages.
- Native A → authenticated B → explicit Direct restarts verified actual routing, bypasses, next-launch selection, preservation of unknown settings, and a controlled persistent cookie. This is not authenticated Yandex-session evidence.
- Proxy checks cover challenge-only Basic/Digest behavior, Digest request-body replay, fragmented CONNECT responses, SOCKS4/5 handshakes, address ordering, and routing/bypass edges.
- Cargo fingerprint diagnostics found a missing watched `capabilities` directory causing a roughly 49-second rebuild on every launch. An empty capability manifest fixed it; an unchanged release build then completed in 0.14 seconds without recompilation. No native IPC permissions were added.
- The documented launcher was exercised, including repeated launch while the previous executable remained running. Binary replacement is atomic rather than an in-place overwrite.
- The final native release window loaded the configured Yandex endpoint, which currently shows the anonymous “Your Messenger chats are now in Telemost” page. No login was performed.
- Local bundle signature verification passed. `CFBundleVersion` is `0.1.0`; `du -sk .runtime/Telesram.app` reported 7184 KiB allocated for the bundle alone. This excludes OS WebKit and build caches and is not an Electron comparison.

[✓ locked Cargo checks; native smoke and launch logs; Orca release-window inspection; `codesign --verify --deep --strict`; `plutil`; `du -sk`]

### Blocking decision: full proxy compatibility

A native wire probe configured a fresh `WKWebsiteDataStore` with an HTTP CONNECT proxy and issued one HTTPS fetch and one WSS connection. Both exposed only the target authority plus `Host`, `Proxy-Connection: keep-alive`, and `Connection: keep-alive`; no original URL scheme distinguished WSS from HTTPS.

The application-level reproduction used:

```yaml
server: 'https=http://127.0.0.1:51213;socks=http://127.0.0.1:51214'
```

Both HTTPS and WSS reached the first fixture; the second received neither probe. [Chromium's documented WebSocket policy](https://raw.githubusercontent.com/chromium/chromium/main/net/docs/proxy.md) instead chooses the nonempty `socks=`/“other proxies” list for WSS before considering the HTTPS list. This is a reproduced mismatch, not merely an untested case. [✓ `proxy-wire-probe`, `native-wire-smoke`, and `native-proxy-A/B` logs]

The Rust adapter also currently supports only Basic/Digest HTTP proxy authentication, not NTLM/Negotiate. Authenticated plain-HTTP forwarding buffers bodies for replay and does not preserve request trailers. [✓ `src/proxy/transport.rs`, `src/proxy/server.rs`]

The user selected **full parity** after this reproduction and rejected restricted proxy profiles. Implementation remains in progress. Reassess WebKit integration and a Rust-hosted Chromium path against the complete source inventory; obtain separate approval before changing the engine. Do not mark this requirement passed or the application complete.

User-assisted Yandex login/session restart, real calls, and remote screen-share delivery remain open. The baseline size/RAM protocol and comparative measurements also remain open; no resource savings are claimed.

Disposable native fixtures, test proxy/settings files, and diagnostic samples were removed. All supervised test processes are stopped; the final application exited normally through native Cmd+Q. The bundle, toolchain, build cache, and normal runtime state are retained. [✓ final artifact glob, supervised process state, and native Quit exit code 0]

### Observed checks — 2026-09-23

- A locked release build and the settings regression test passed. The test exercises removal of `proxyProfileId` on the next write while preserving unrelated JSON and toggle values after reload.
- An isolated, separately identified native bundle launched and loaded the configured endpoint. Its menu exposed Managed Mode, Close to Tray, Show on Startup, and zoom without proxy controls; toggles persisted. The existing user-owned application was not replaced or stopped.
- The endpoint displayed “Your Messenger chats are now in Telemost” instead of a chat view. The user subsequently approved the Telemost cutover above; authenticated calls and remote screen-share delivery remain untested.
- A local package build of pinned Yangertron commit `c6df711b3acff9b4e6208197de93a34c9fc9da13` with publishing disabled produced a 280204 KiB `.app`; the existing Telesram bundle occupied 6424 KiB (`du -sk`). These are bundle sizes, not a comparison of shared OS frameworks or caches.
- One snapshot of the former anonymous Messenger migration page measured 597202560 bytes for Electron and its three child processes, versus 312331120 bytes for Telesram and its two WebKit processes (`footprint --noCategories --format bytes`). The WebKit processes had open files under `~/Library/WebKit/dev.longday.telesram/`. This is **not** a release-equivalent RAM comparison: the pinned Electron application was run from its built `dist/main.js` with Electron 42.4.0 because its packaged app did not expose a usable window, while the existing Telesram process had been running for over 13 hours. No RAM savings are claimed.
- The temporary pinned checkout, measurement runtime, and supervised processes were removed or stopped. The existing Telesram process was left untouched.
- The frozen media matrix row has no passing authenticated end-to-end test. Native control checks were manual smoke runs, not a replacement for the workflow's automated matrix audit. Keep the spec `in-progress` until user-assisted call and screen-share verification resolve those gates.

### Observed checks — Telemost cutover

- `cargo test --locked --features custom-protocol` passed both settings and exact-host navigation tests; the isolated locked release build and ad-hoc bundle signature verification passed.
- A distinct-ID native process loaded the current Telemost landing page and its Yandex ID login form inside the same window with Managed Mode enabled. No credentials were entered in this isolated profile; the working-profile authentication check is recorded below. Calls and remote screen-share delivery remain untested.
- At the time of the isolated smoke, the pre-existing Telesram process still ran the earlier Messenger build. The launcher could update its bundle, but not replace that live process.
- The temporary checkout and supervised native process were removed or stopped. With explicit user approval, the anonymous WebKit data store for the temporary identifier `dev.longday.telesram.telemostsmoke` was deleted; the existing `dev.longday.telesram` profile was untouched. The project's own release binary was rebuilt after the isolated smoke build.
- On 2026-09-24, the earlier PID 36460 had already exited before the approved termination attempt, so no signal was sent. `./start.sh` launched the updated working bundle as PID 41380; its native window loaded Telemost and the existing WebKit profile yielded an authenticated session. No credentials were entered and no chats were opened during the check.
- The user deferred an empty-call test. Real calls, camera/microphone permissions, and remote screen-share delivery remain unverified. The supervised working process subsequently exited with code 0.
- On 2026-09-24, an isolated release build with Tauri `devtools` enabled exposed Developer Tools in the Telemost menu. Native menu activation opened Web Inspector docked by default; its Detach button created a second window. Registering `WebKit2InspectorStartsAttached = false` as a process-local fallback before WebKit initialization made the first menu activation open a separate Web Inspector window. The working `dev.longday.telesram` WebKit profile was not used in this check. The main project release build passed after porting the setting.
- The isolated test process was stopped and its temporary checkout, WebKit store, Preferences plist, and Cache directory were removed with user approval. At that point the main-project locked release build, two existing tests, and `cargo fmt --all --check` had passed; the working application bundle had not yet been relaunched.
- The user subsequently confirmed that the working Inspector opened docked and accepted that placement; the detached-placement override was removed. This latest behavior is user-reported, not a repeat inspection of the authenticated profile.
- The pinned EasyPrivacy/uBlock Privacy conversion produced 55,180 network blocks, 75 exceptions, and one CSS rule, skipping 2,780 unsupported or inactive entries. `WKContentRuleListStore` compiled the full JSON in a temporary project-local store; a nonpersistent `WKWebView` fixture reported `OFF=block,block` and `ON=none,block` for the bar and an unrelated element. Synthetic URL checks allowed Telemost/authentication hosts and documents, blocked analytics hosts including `google-analytics.com`, and did not block a lookalike `mc.yandex.ru.evil.com` host. The working WebKit profile was not inspected for this filter check.
- An isolated launcher fixture used stubbed Cargo/Rustup and real macOS packaging/signing to exercise `start.sh`, cached `start.sh`, `dev.sh`, and `start.sh` again. Cargo ran only for the initial release and debug builds; the restored app passed strict code-signature verification. A real locked debug build completed without launching the app.
- A fake-HOME reset fixture refused noninteractive and wrong confirmations, refused a symlink substituted during the confirmation prompt, and on correct confirmation removed only the app-specific WebKit directory while preserving a sibling. The temporary fixtures were removed. The working WebKit directory names were examined to scope deletion, but no file contents were opened; neither that directory nor `.runtime/settings.json` was modified.
