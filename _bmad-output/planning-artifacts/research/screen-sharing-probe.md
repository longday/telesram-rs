# Native Tauri screen-sharing probe

Date: 2026-09-22. Result: **local window and monitor capture succeeded**. This is not an end-to-end Yandex Messenger or WebRTC-call result.

## Observed environment

| Item | Value | Evidence |
| --- | --- | --- |
| macOS | 26.6.2, build 25G83 | `sw_vers` |
| Target | Apple Silicon, `aarch64-apple-darwin` | Installed native Rust toolchain |
| Apple SDK | 27.0, Command Line Tools | `xcode-select --print-path`, `xcrun --show-sdk-version` |
| Rust | 1.98.1 | Project-local `rustup toolchain install` output |
| Tauri / tauri-build | 2.11.5 / 2.6.3 | Exact Cargo dependency pins and lockfile |
| tauri-runtime-wry / Wry / Tao | 2.11.4 / 0.55.1 / 0.35.3 | Resolved Cargo lockfile |
| Build | Native debug executable; deployment target 14.0 | Successful `cargo +1.98.1 build --locked` |
| Page origin | `http://127.0.0.1:18765/`, `isSecureContext === true` | WKWebView probe output |
| Webview state | Incognito; default user agent; no private capture API or native media bridge | Temporary Rust entrypoint |

The source snapshots in the earlier feasibility report used development-branch manifests. They are **not** identical to the published dependency graph exercised here. The browser user agent reported `Intel Mac OS X 10_15_7` and `AppleWebKit/605.1.15`; those strings are not measurements of hardware, installed macOS, or the precise WebKit build.

## Procedure

1. Installed Rust only into project-local `RUSTUP_HOME` and `CARGO_HOME` under `.runtime/` through the existing Home Manager-provided rustup.
2. Created a disposable Tauri window with a static page served only on loopback. No frontend framework, npm package, Yandex page, remote account, or Tauri command bridge was used.
3. Bound the following call directly to a button's click handler, without an asynchronous step before requesting capture:

   ```javascript
   stream = await navigator.mediaDevices.getDisplayMedia({ video: true, audio: false });
   preview.srcObject = stream;
   ```

4. Displayed API availability, secure-context state, transient user activation, returned video-track settings, and the video element's loaded-frame dimensions. The native title-change callback mirrored this diagnostic JSON to stdout.
5. The user operated the prototype and subsequently confirmed: “Я понажимал, захват работал.” The agent did not approve a screen-capture permission dialog.

## Runtime evidence

Both `getDisplayMedia` and `getUserMedia` were exposed as functions. Only `getDisplayMedia` was invoked; this experiment says nothing about camera/microphone access.

Selected diagnostic fields from the native process log, with opaque device IDs omitted:

```json
{"status":"ready","secureContext":true,"getDisplayMedia":"function","getUserMedia":"function"}
{"status":"requesting","activeAtClick":true}
{"status":"stream-returned","activeAtClick":true,"tracks":[{"kind":"video","readyState":"live","settings":{"displaySurface":"window","frameRate":30,"height":720,"width":900}}]}
{"status":"preview-frame-loaded","width":900,"height":720}
{"status":"stopped"}
{"status":"requesting","activeAtClick":true}
{"status":"stream-returned","activeAtClick":true,"tracks":[{"kind":"video","readyState":"live","settings":{"displaySurface":"monitor","frameRate":30,"height":1117,"width":1728}}]}
{"status":"preview-frame-loaded","width":1728,"height":1117}
```

A native-window screenshot independently showed the window-capture preview displaying the prototype itself, with the macOS sharing indicator active. The monitor scenario is supported by the returned track, loaded-frame event, and user confirmation; no monitor screenshot was retained. The reported 30 fps is a track setting, not measured sustained throughput.

The process later exited normally with code 0. An agent attempt to click Stop after that returned `app_not_found`; it is not counted as a successful automated action. The earlier `stopped` event was already in the log between the two captures.

## Interpretation and remaining limits

- **Established:** an ordinary Tauri 2 WKWebView on this tested macOS version can return usable window and monitor capture streams through the browser API. No custom ScreenCaptureKit-to-JavaScript bridge was needed for these local scenarios.
- **Not established:** Yandex authentication, site compatibility, screen sharing to another call participant, WebRTC transport/codecs, system audio, permission denial/cancellation, persistent permission behavior, older macOS versions, packaged-app behavior, or performance under load.
- The source-only absence of a Tauri/Wry display-capture callback is insufficient evidence that WKWebView screen capture is unavailable. Do not plan a custom native capture bridge solely on that basis.
- The prototype used a secure loopback origin and the default WebKit user agent, not the upstream Linux/Chrome user-agent override. Those differences remain integration checks.
- No application-size or RAM comparison was performed. A debug prototype is not a release-size baseline.

## Experiment audit

The first build failed because Tauri's context generator expected `icons/icon.png`; adding a neutral 32×32 RGBA icon resolved the failure. The subsequent locked build succeeded.

Original temporary-source SHA-256 values, recorded before cleanup:

- `Cargo.lock`: `3e3d7473b912f140a1885a9a6627eb5834cea0432a62a6b8379f1072ef7957b5`
- `Cargo.toml`: `a92e16d41a5a2b5307b71e9be013f2250ee499fa94909e079c2a7c613ec0bc48`
- `src/main.rs`: `e6a00c79f87ab55e81cae65e36d481566786705c25481afac8f7a0659918dcf9`
- `web/index.html`: `3c7d87d24d80283b7944c8cd506ae5b81becbb0e151bf30c8068722e230b7117`

## Cleanup

The native process exited and the supervised loopback server was stopped. Temporary source, build output, local Rust toolchain, Cargo cache, and the agent-generated prototype screenshot are removed at experiment completion. No screen recording was saved; only diagnostic text remains in this report. Global rustup settings remain unchanged. No OS permission was granted or reset by the agent, and no commit was created.
