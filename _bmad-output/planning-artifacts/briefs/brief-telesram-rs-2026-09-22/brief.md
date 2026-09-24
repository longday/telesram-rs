---
status: draft
project: telesram-rs
date: 2026-09-22
---

# Product Brief: telesram-rs

## Purpose

Build a personal-use replacement for [Yangertron](https://github.com/longday/yangertron), the Electron-based Yandex 360 Messenger client, using Rust and Tauri 2. Preserve its user-visible functionality while reducing RAM consumption and application size. Resource savings are a goal, not a measured result.

The user selected macOS 26.0+ on Apple Silicon and local builds for personal use. Intel Macs, Linux, Windows, installer distribution, signing, and notarization are outside this release's delivery scope. Platform-required local build signing is not a distribution feature.

Development is container-free: install project dependencies, build, test, and run directly on macOS. Do not add Docker or container-based development workflows for this project. Global toolchain changes and system privacy permissions require separate approval.

## User and Experience

The user wants the existing Yangertron experience, not a redesigned messenger. Functional parity remains mandatory except for application-managed proxies, which the user explicitly removed. Other limitations must not silently become scope reductions.

The source inventory pins Yangertron revision [`c6df711b3acff9b4e6208197de93a34c9fc9da13`](https://github.com/longday/yangertron/commit/c6df711b3acff9b4e6208197de93a34c9fc9da13) and assigns feature IDs YFI-01–YFI-12. Applicable parity includes login and session persistence, navigation, native menus, tray and unread indication, window behavior, managed mode, media calls, and screen sharing. Proxy profiles (YFI-10) are excluded by explicit user approval.

## Success Criteria

1. Every applicable behavior in the source inventory works on an Apple Silicon Mac. Linux-specific installation integration is outside the agreed platform scope.
2. Both total application RAM consumption and built application size are lower than Yangertron under equivalent conditions. Numeric thresholds will be agreed after baseline measurements; none have been set.
3. Comparisons account for all application-attributable processes, not only the Rust or Electron parent process. The final measurement protocol must specify memory metric, workload, build mode, sampling interval, and repetitions before results are accepted.
4. Size comparisons use equivalent application artifacts and clearly separate application size from build caches, dependencies, user profiles, and any downloaded distribution archive. The exact artifact definition remains open until a comparable baseline is available.
5. Lower resource use does not excuse missing functionality.

## Boundaries

**In scope:** a Rust/Tauri 2 desktop replacement with complete applicable Yangertron behavior, macOS Apple Silicon, local personal builds, and a reproducible comparison with the Electron original.

**Out of scope:** application-managed proxies, new product features, a new messenger service, UI redesign, other platforms or CPU architectures, public distribution, and migration mechanisms not required by the agreed parity baseline.

The user authorized the production application, then explicitly approved removal of its proxy subsystem. Retain Rust/Tauri/WKWebView and normal WebKit networking, including inherited system proxy/PAC behavior; do not change system settings. The [implementation specification](../../../implementation-artifacts/spec-telesram-desktop-app.md#approved-scope-amendment-remove-proxy-support) records the amendment. Login and macOS privacy actions remain the user's responsibility.

## Open Gates

- Prove screen sharing into the messenger's WebRTC call. Local window and monitor capture already succeeded in WKWebView on macOS 26.6.2; remote call delivery and other OS versions remain untested.
- Validate Yandex authentication, session persistence, calls, and remote-page customization on the target runtime.
- Resolve any incompatibility with the user before changing scope or the chosen stack.
- Define and run the baseline measurement protocol in a separately authorized validation stage.

## Evidence and Next Step

Product scope and goals above were selected by the user. Local window and monitor capture succeeded; prior native checks are recorded in the implementation specification. Authenticated Messenger behavior and comparative resource savings remain unverified.

Supporting reports:

- [Yangertron feature inventory](../../research/yangertron-feature-inventory.md)
- [Tauri macOS feasibility](../../research/tauri-macos-feasibility.md)
- [Native screen-sharing experiment](../../research/screen-sharing-probe.md)

Proxy compatibility is no longer an acceptance gate. Private-PAC/CEF experiments are cancelled; remaining validation concerns authenticated sessions, calls/sharing, and resource measurements.
