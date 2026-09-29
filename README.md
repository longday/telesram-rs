# Telesram

Rust/Tauri 2 desktop shell for Yandex Telemost (`https://telemost.360.yandex.ru`) on macOS 26+ Apple Silicon. The historical parity target is [Yangertron `c6df711`](https://github.com/longday/yangertron/commit/c6df711b3acff9b4e6208197de93a34c9fc9da13), except the retired Messenger destination and application-managed proxies.

The [approved scope amendments](_bmad-output/implementation-artifacts/spec-telesram-desktop-app.md#approved-scope-amendment-current-telemost-service) define the current destination and exclude application-managed proxies. Comparable resource measurements, independent verification of remote screen-share delivery, and fullscreen behavior in a real call remain open.

## Run

Prerequisites: native Apple Silicon macOS 26+, Apple Command Line Tools, `rustup` on `PATH`, and network access for the first build to install pinned Tauri CLI 2.11.5 into `.runtime/`.

```sh
./build.sh
./start.sh
./dev.sh
./reset.sh
```

`build.sh` runs a locked, optimized Tauri release build and creates an ad-hoc-signed `.runtime/target/release/bundle/macos/Telesram.app` without launching it. Cargo's compiled binary is under `.runtime/target/release/`; the first build installs the pinned CLI locally. `start.sh` rebuilds and launches the release bundle; a failed build does not launch an older copy.

`dev.sh` builds and launches a separate debug bundle at `.runtime/Telesram.app`. Both bundles use the same identifier, `.runtime/settings.json`, and WebKit login data. Close any running Telesram process before building; switching bundles or rebuilding changed code may require re-granting camera, microphone, or screen-recording permission. Unlike the release bundle, the debug bundle is signed without Hardened Runtime or media entitlements, so it cannot verify release-mode permissions. An older `.runtime/Telesram.app` remains the previous release build until `dev.sh` replaces it; remove it and the unused `.runtime/Telesram.app.build` stamp manually if you no longer need that copy.

`reset.sh` requires an interactive terminal, a stopped application, and typing `RESET dev.longday.telesram`. It deletes only `~/Library/WebKit/dev.longday.telesram/`, including cookies, website storage, and compiled content rules; both modes will be signed out. It does not delete `.runtime/` or macOS privacy permissions. No reset runs automatically.

Tauri may regenerate ignored ACL metadata under its fixed `gen/schemas/` path and update `Cargo.toml` when configuration-driven features change. `capabilities/native-shell.json` grants no native IPC permissions and keeps Tauri's watched capability directory present. Configuration paths are tied to this checkout at compile time; rebuild after moving it. [✓ `build.sh`, `start.sh`, `dev.sh`, `reset.sh`, `src/config.rs`, `tauri.conf.json`]

Sign in and grant camera, microphone, or screen-recording access yourself when macOS requests it. The application does not import another browser's session.

## Telemost links

Telesram handles the Telemost website's `telemost://` links:

- `telemost://https://telemost.yandex.ru/j/<meeting-id>` and the `telemost.360.yandex.ru` equivalent.
- `telemost://ychat/<telemost-host>/<path, query and fragment>` for those same two hosts.

Links reuse and reveal the main window, including when the app starts or its window must be recreated. Navigation waits for native WebKit configuration. Incoming destinations are validated independently of Managed Mode; foreign hosts, credentials, nonstandard ports, and malformed meeting IDs are rejected. The destination receives `skip_app=1` to prevent automatic relaunch through the website.

After building, register the release bundle with macOS LaunchServices:

```sh
/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister -f "$PWD/.runtime/target/release/bundle/macos/Telesram.app"
```

This changes macOS URL-handler registration, not browser login data. The original Telemost client should not compete for the scheme. The debug bundle inherits the same scheme declaration, so register the intended release bundle again after switching builds.

Verification: the release bundle was selected by macOS for `telemost://`; cold launch opened the messenger threads route, and a link reopened the same window after Close to Tray. The user observed the expected nonexistent-meeting error from synthetic meeting links. Joining a live meeting through a deep link was not tested. [✓ `NSWorkspace.urlForApplication(toOpen:)`, `open`, Orca window observation, user confirmation]

## Controls and storage

- **control menu:** Managed Mode, Close to Tray, Show on Startup, Reload Page, Home (loads the configured Telemost URL), zoom, and Developer Tools. The Web Inspector can open docked inside the main window.
- **Tray:** Show Window, Hide Window, Quit; a left click toggles the main window.
- **Unread indicator:** a dot changes the tray icon's silhouette; template rendering follows the menu-bar theme. The existing title-digit signal is tracked across routes on both Telemost HTTPS origins, including `/threads` and `/chats/...`, with tooltip and Dock badge updates. Muted messages follow the website's title-counter behavior.
- Managed Mode reloads the page, blocks the Yandex-only tracking/advertising profile, and hides `div.yamb-global-bar` on Telemost. It keeps exact Telemost/authentication HTTPS hosts inside the window, opens other links in the default browser, and redirects internal popups into the main window. Document navigations and essential Telemost/authentication/CDN subresources remain allowed.
- `.runtime/settings.json` stores geometry and toggles. Cookies and website data belong to WebKit's persistent OS-managed data store, not that JSON file.
- Developer Tools is enabled in the local release build through Tauri's `devtools` feature. WebKit uses private macOS inspector APIs; this ad-hoc-signed bundle is neither Developer ID signed nor notarized and is not suitable for public/App Store distribution. The inspector can expose authenticated page data.

[✓ `src/menu.rs`, `src/tray.rs`, `src/window.rs`, `src/macos.rs`, `src/settings.rs`; isolated native menu and separate-window smoke]

The unread template image was checked with a native `NSStatusBarButton` render: normal and unread differed after the fix, and clearing unread restored the normal image. This isolates icon rendering without changing chat read state. [✓ native AppKit render probe]

An isolated native WebKit fixture kept the tray tooltip and Dock badge set across `history.pushState('/threads')` and `history.replaceState('/chats/example')` without changing the unread title, then cleared both when the title counter disappeared. The navigation regression failed on the captured `/threads` URL before the fix; all eight tests passed afterward. [✓ native unread-route smoke; `cargo test --locked --features custom-protocol`]

## Managed filter sources

`assets/managed-rules.json` is a manually maintained profile: 33 network rules and one Telemost-scoped CSS rule. Dedicated tracking hosts include Metrica, Webvisor, AppMetrica and Yandex advertising; mixed-use hosts are blocked only on explicit tracking paths. Non-Yandex trackers are intentionally outside scope. Documents are excluded; Telemost, Passport, ID, cookie-helper and general CDN hosts are not blocked.

Sources: selected Yandex rules from [EasyPrivacy at `71969edb`](https://github.com/easylist/easylist/tree/71969edb834254e8172d3a2c7710805cbfabe3ce) (2026-09-24 snapshot), `yandexmetrica.com` from [uAssets Privacy at `038b6edb`](https://github.com/uBlockOrigin/uAssets/blob/038b6edb958a9684ccde4ccc5b7b28788d03c9a6/filters/privacy.txt), and the application's existing explicit tracker rules. EasyPrivacy offers [GPLv3-or-later or CC BY-SA 3.0-or-later](https://easylist.to/pages/licence.html); uAssets uses [GPLv3](https://github.com/uBlockOrigin/uAssets/blob/master/LICENSE). Broad snapshots and their converter are no longer bundled.

Edit the JSON directly; preserve hostname boundaries, exact-host path rules and document exclusion. No automatic list updates or uBlock scriptlets run. From the repository root on macOS, check native WebKit behavior:

```sh
clang -fobjc-arc -framework AppKit -framework WebKit scripts/check_managed_rules.m -o .runtime/check-managed-rules
./.runtime/check-managed-rules
```

The checker compiles the exact JSON in an isolated store, intercepts HTTP/HTTPS locally and uses nonpersistent WebKit data. It exercises image/fetch blocking, service/CDN and lookalike allowances, document navigation, CSS scoping and off → on → off transitions. It deletes its temporary store on completion.

## Networking

WebKit uses its normal networking, which may inherit system proxy/PAC settings. The application does not configure proxies, force Direct, run a loopback adapter, or read `proxy.yml`. System settings are not changed. An existing user-owned `proxy.yml` is left untouched and remains ignored by Git; obsolete `proxyProfileId` settings are removed on the next settings write.

## Verification boundary

Before proxy removal, native controlled fixtures exercised menus/tray, window and restart behavior, unread indication, zoom including window recreation, managed filtering and document exemptions, popup/external navigation, and persistent cookies. The proxy implementation and its tests have been removed with the feature.

The current Telemost landing page and Yandex ID login form opened inside an isolated native window with Managed Mode enabled. The updated working bundle then opened an authenticated Telemost session using the existing WebKit profile; no credentials were entered during this check. The user later reported successful screen sharing in a live call after switching the macOS WebKit user agent to Safari; remote participant receipt was not independently checked. Comparable release-workload RAM measurements remain open; no savings are claimed.

Telesram enables WebKit's element Fullscreen API for pages in the main webview. A local `WKWebView` fixture reported `document.fullscreenEnabled=true` after configuration; in a real call, check the button, entering/exiting fullscreen, and hiding/restoring the window from the tray after restarting the rebuilt app.

The user confirmed that Web Inspector opening docked inside the working window is acceptable. An earlier isolated build also demonstrated WebKit's separate-window Detach control. The app no longer overrides WebKit's inspector placement.

The Yandex-only profile passed the native checker: 26 image and 26 fetch cases in each of three modes, plus document exemptions and CSS scope checks. This proves selected rules, not complete analytics coverage or traffic savings. Fresh login, message delivery and live-call media were not exercised; no messages were sent or calls started.

`build.sh` produced the Tauri release bundle under `.runtime/target/release/bundle/macos` and completed an incremental second build. Its strict code signature is ad-hoc with Hardened Runtime and camera/microphone entitlements; Tauri reported notarization skipped even with synthetic Apple signing variables. Isolated fixtures confirmed `start.sh` builds before launch and never launches after build failure, while `dev.sh` still launches its own signed bundle without replacing the release bundle. Neither launcher was run against the working WebKit profile; camera/microphone capture, screen sharing, and Web Inspector remain unverified under the new Hardened Runtime signature.

[✓ `./build.sh`, synthetic-Apple-env `./build.sh`, `codesign --verify --deep --strict`, `codesign -dv --verbose=4`, `codesign -d --entitlements - --xml`, isolated start/debug fixtures; prior `cargo test --locked --features custom-protocol` (2 passed), reset fixture and [screen-sharing probe](_bmad-output/planning-artifacts/research/screen-sharing-probe.md)]
