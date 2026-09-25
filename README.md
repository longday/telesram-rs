# Telesram

Rust/Tauri 2 desktop shell for Yandex Telemost (`https://telemost.360.yandex.ru`) on macOS 26+ Apple Silicon. The historical parity target is [Yangertron `c6df711`](https://github.com/longday/yangertron/commit/c6df711b3acff9b4e6208197de93a34c9fc9da13), except the retired Messenger destination and application-managed proxies.

The [approved scope amendments](_bmad-output/implementation-artifacts/spec-telesram-desktop-app.md#approved-scope-amendment-current-telemost-service) define the current destination and exclude application-managed proxies. Comparable resource measurements, independent verification of remote screen-share delivery, and fullscreen behavior in a real call remain open.

## Run

Prerequisites: native Apple Silicon macOS 26+, Apple Command Line Tools, and `rustup` on `PATH`.

```sh
./build.sh
./start.sh
./dev.sh
./reset.sh
```

`build.sh` checks prerequisites, runs an incremental locked release build, and assembles/signs `.runtime/Telesram.app` without launching it. `start.sh` runs `build.sh` and then launches that app, so source changes are picked up on the next launch. Every launch needs a successful Cargo build; if it fails, the app does not start, even when an older bundle exists.

`dev.sh` builds the debug binary on every invocation. Build and launch scripts use `.runtime/Telesram.app`, the same bundle ID, `.runtime/settings.json`, and WebKit login data. Switching modes replaces and locally signs that bundle; close any running Telesram process before building. Debug/release signing changes may require re-granting macOS media permissions.

`reset.sh` requires an interactive terminal, a stopped application, and typing `RESET dev.longday.telesram`. It deletes only `~/Library/WebKit/dev.longday.telesram/`, including cookies, website storage, and compiled content rules; both modes will be signed out. It does not delete `.runtime/` or macOS privacy permissions. No reset runs automatically.

Tauri may regenerate ignored ACL metadata under its fixed `gen/schemas/` path. `capabilities/native-shell.json` grants no native IPC permissions and keeps Tauri's watched capability directory present. Configuration paths are tied to this checkout at compile time; rebuild after moving it. [✓ `build.sh`, `start.sh`, `dev.sh`, `reset.sh`, `src/config.rs`, `tauri.conf.json`]

Sign in and grant camera, microphone, or screen-recording access yourself when macOS requests it. The application does not import another browser's session.

## Controls and storage

- **control menu:** Managed Mode, Close to Tray, Show on Startup, zoom, and Developer Tools. The Web Inspector can open docked inside the main window.
- **Tray:** Show Window, Hide Window, Quit; a left click toggles the main window.
- Managed Mode reloads the page, blocks a pinned WebKit-compatible subset of EasyPrivacy and uBlock Privacy network rules, and hides `div.yamb-global-bar`. It keeps exact Telemost/authentication HTTPS hosts inside the window, opens other links in the default browser, and redirects internal popups into the main window. Document navigations and essential Telemost/authentication subresources are exempt.
- `.runtime/settings.json` stores geometry and toggles. Cookies and website data belong to WebKit's persistent OS-managed data store, not that JSON file.
- Developer Tools is enabled in the local release build through Tauri's `devtools` feature. WebKit uses private macOS inspector APIs; this build is not suitable for App Store distribution. The inspector can expose authenticated page data.

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

`build.sh` completed a release build and a second incremental build; the resulting `.app` passed strict signature verification. An isolated `start.sh` fixture ran build before app and did not launch app when build failed. A fake-HOME reset fixture refused noninteractive, cancelled, and symlink-swapped requests, then removed only its WebKit directory on confirmation. The real debug binary built; the new `start.sh` has not been launched against the working WebKit profile.

[✓ `./build.sh` twice, isolated `start.sh` fixture, `codesign --verify --deep --strict`; prior `cargo test --locked --features custom-protocol` (2 passed), `cargo build --locked --features custom-protocol`, reset fixture and [screen-sharing probe](_bmad-output/planning-artifacts/research/screen-sharing-probe.md)]
