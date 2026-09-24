---
title: 'Probe screen sharing in a native macOS Tauri window'
type: 'chore'
created: '2026-09-22'
status: 'done'
route: 'oneshot'
---

<frozen-after-approval reason="user-approved temporary experiment">

## Intent

Determine whether a minimal Tauri 2 WKWebView on this Apple Silicon Mac exposes and can invoke `navigator.mediaDevices.getDisplayMedia()` from a real user gesture. Show API availability, secure-context state, the exact result/error, and a live local preview if a stream is returned. Stop all tracks when requested or before closing.

Use a temporary project under `.runtime/screen-share-probe/`, pinned Rust/Tauri dependencies, and project-local toolchain/cache directories. Build and run natively without containers. Do not visit Yandex, authenticate, transmit or record captured frames, approve OS permissions automatically, change global toolchain settings, or commit files. Pause for the user if macOS authorization is required. A negative API result is a valid experiment outcome, not a request to add native capture workarounds.

Record the environment, exercised scenario, evidence, limitations, and cleanup in `_bmad-output/planning-artifacts/research/screen-sharing-probe.md`. Distinguish a local WKWebView probe from end-to-end Yandex/WebRTC compatibility. Remove the temporary project and resources after the experiment.

</frozen-after-approval>

## Implementation Notes

- Small, reversible, single-question experiment; no production API, app implementation, or remote session.
- Continue on the existing working tree without committing BMAD/bootstrap/planning changes, as requested.
- Rustup is provided by Home Manager; no toolchains are installed. Use `RUSTUP_HOME` and `CARGO_HOME` under `.runtime/` and explicit Rust 1.98.1, leaving the global profile unchanged.
- Use Tauri 2.11.5 (the researched release), static HTML/JavaScript, and Cargo directly; no npm installation or frontend framework is needed.
- No persistent tests or additional product features. Runtime observation is the proof.
- Installed Rust 1.98.1 only under `.runtime/`; the existing global rustup settings remained unchanged.
- Built Tauri 2.11.5 with tauri-build 2.6.3; Cargo resolved Wry 0.55.1 and Tao 0.35.3. Added the neutral icon required by `generate_context!()` after the first build identified that prerequisite.
- Runtime evidence on macOS 26.6.2: secure loopback context, active user gestures, live window and monitor video tracks, loaded preview frames. The user confirmed successful capture.
- The native process exited normally. The page server was stopped; temporary source, build output, toolchain, cache, and screenshot were removed.
- Result and limitations: `../planning-artifacts/research/screen-sharing-probe.md`. No Yandex login, production changes, or commit.

## Review Triage Log

- Inline experiment review: localhost-only content, incognito state, no upload or recording path, no native capture bridge, and track-stop handling. No permanent tests or production review suite: this was a disposable runtime experiment.
- Evidence limitation: local capture does not prove Yandex call delivery, older macOS support, or measured sustained frame rate; retained explicitly in the result report.
- The attempted automated Stop click returned `app_not_found` after the process had already exited. It is not presented as a successful action; the log independently contains a stop event between the two captures.
