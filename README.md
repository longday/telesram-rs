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

## Controls and storage

- **control menu:** Managed Mode, Close to Tray, Show on Startup, zoom, and Developer Tools. The Web Inspector can open docked inside the main window.
- **Tray:** Show Window, Hide Window, Quit; a left click toggles the main window.
- Managed Mode reloads the page, blocks a pinned WebKit-compatible subset of EasyPrivacy and uBlock Privacy network rules, and hides `div.yamb-global-bar`. It keeps exact Telemost/authentication HTTPS hosts inside the window, opens other links in the default browser, and redirects internal popups into the main window. Document navigations and essential Telemost/authentication subresources are exempt.
- `.runtime/settings.json` stores geometry and toggles. Cookies and website data belong to WebKit's persistent OS-managed data store, not that JSON file.
- Developer Tools is enabled in the local release build through Tauri's `devtools` feature. WebKit uses private macOS inspector APIs; this ad-hoc-signed bundle is neither Developer ID signed nor notarized and is not suitable for public/App Store distribution. The inspector can expose authenticated page data.

[✓ `src/menu.rs`, `src/tray.rs`, `src/window.rs`, `src/macos.rs`, `src/settings.rs`; isolated native menu and separate-window smoke]

## Managed filter sources

`assets/filter-sources/easyprivacy-202609241222.txt` is the EasyPrivacy 2026-09-24 12:22 UTC snapshot (upstream commit `71969edb834254e8172d3a2c7710805cbfabe3ce`). The two uBlock Privacy snapshots under `assets/filter-sources/` come from uAssets commit `038b6edb958a9684ccde4ccc5b7b28788d03c9a6`, including its `resource-abuse.txt` include. `python3 scripts/generate_managed_rules.py` regenerates the checked-in `assets/managed-rules.json`; `--check` compares it without writing.

The generated WebKit list contains 55,180 network blocks, 75 exceptions (including five protected Telemost/authentication hosts), and one CSS rule. The converter counts 2,780 skipped entries, chiefly domain-scoped filters, unsupported patterns, cosmetics, and negated third-party modifiers. Host-anchored third-party blocks outside Yandex use WebKit's `load-type`; WebKit tests origins while uBlock tests sites, so this is not equivalent to the uBlock engine. EasyPrivacy is [GPLv3-or-later or CC BY-SA 3.0-or-later](https://easylist.to/pages/licence.html); uAssets is [GPLv3](https://github.com/uBlockOrigin/uAssets/blob/master/LICENSE). Check redistribution obligations before publishing a bundle with these snapshots.

[✓ source headers in `assets/filter-sources/`; `python3 scripts/generate_managed_rules.py --check`]

## Networking

WebKit uses its normal networking, which may inherit system proxy/PAC settings. The application does not configure proxies, force Direct, run a loopback adapter, or read `proxy.yml`. System settings are not changed. An existing user-owned `proxy.yml` is left untouched and remains ignored by Git; obsolete `proxyProfileId` settings are removed on the next settings write.

## Verification boundary

Before proxy removal, native controlled fixtures exercised menus/tray, window and restart behavior, unread indication, zoom including window recreation, managed filtering and document exemptions, popup/external navigation, and persistent cookies. The proxy implementation and its tests have been removed with the feature.

The current Telemost landing page and Yandex ID login form opened inside an isolated native window with Managed Mode enabled. The updated working bundle then opened an authenticated Telemost session using the existing WebKit profile; no credentials were entered during this check. The user later reported successful screen sharing in a live call after switching the macOS WebKit user agent to Safari; remote participant receipt was not independently checked. Comparable release-workload RAM measurements remain open; no savings are claimed.

Telesram enables WebKit's element Fullscreen API for pages in the main webview. A local `WKWebView` fixture reported `document.fullscreenEnabled=true` after configuration; in a real call, check the button, entering/exiting fullscreen, and hiding/restoring the window from the tray after restarting the rebuilt app.

The user confirmed that Web Inspector opening docked inside the working window is acceptable. An earlier isolated build also demonstrated WebKit's separate-window Detach control. The app no longer overrides WebKit's inspector placement.

The enlarged list compiled in an isolated native `WKContentRuleListStore`. A nonpersistent `WKWebView` fixture returned `display: block` for the bar with no rules and `display: none` with rules; an unrelated element stayed visible. Synthetic URL checks covered Yandex service exemptions, analytics hosts, lookalike hosts, and document navigations. No working-profile traffic was inspected.

`build.sh` produced the Tauri release bundle under `.runtime/target/release/bundle/macos` and completed an incremental second build. Its strict code signature is ad-hoc with Hardened Runtime and camera/microphone entitlements; Tauri reported notarization skipped even with synthetic Apple signing variables. Isolated fixtures confirmed `start.sh` builds before launch and never launches after build failure, while `dev.sh` still launches its own signed bundle without replacing the release bundle. Neither launcher was run against the working WebKit profile; camera/microphone capture, screen sharing, and Web Inspector remain unverified under the new Hardened Runtime signature.

[✓ `./build.sh`, synthetic-Apple-env `./build.sh`, `codesign --verify --deep --strict`, `codesign -dv --verbose=4`, `codesign -d --entitlements - --xml`, isolated start/debug fixtures; prior `cargo test --locked --features custom-protocol` (2 passed), reset fixture and [screen-sharing probe](_bmad-output/planning-artifacts/research/screen-sharing-probe.md)]
