---
title: 'Separate release build from launch'
type: 'refactor'
created: '2026-09-26'
status: 'done'
route: 'oneshot'
review_loop_iteration: 0
context: []
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** `start.sh` currently builds a release binary only when absent, so subsequent launches can run stale code. There is no build-only command that produces the signed application bundle without launching it.

**Approach:** Extract the release build and bundle assembly into executable `build.sh`. Make `start.sh` invoke that script and then execute the built app. Preserve the existing platform/toolchain checks, shared app bundle and profile, running-process guard, and debug/reset workflows. Update run instructions to describe the new commands.

</frozen-after-approval>

## Implementation Notes

`build.sh` now owns the platform/toolchain checks, running-process guard, unconditional incremental `cargo build --locked --release --features custom-protocol`, and signed bundle assembly. `start.sh` checks its arguments, invokes `build.sh`, then execs the app. `dev.sh` and `reset.sh` are unchanged. Updated the README launch instructions; verification evidence follows.

`zsh -n build.sh start.sh` and `git diff --check` passed. `./build.sh` built the release bundle; a second invocation completed incrementally, and `codesign --verify --deep --strict .runtime/Telesram.app` passed. An isolated `start.sh` fixture observed `build` then `app`, and exit 7 from `build.sh` prevented launch. The working WebKit profile was not opened.

The bundle stamp now includes the release binary, `Info.plist`, and `tauri.conf.json`, so build-only packaging reflects metadata edits. A post-review `./build.sh` replaced the existing bundle signature once; a second incremental invocation did not. Strict signature verification passed.

## Review Triage Log

- False: moving the running-process guard after Cargo would allow a running app to trigger a build and would not prevent a process from starting during compilation; the existing early guard is retained. The possible race already existed during the initial build.
- Low: the running-app error incorrectly mentioned a mode switch during a release rebuild; reworded.
- Medium: the bundle stamp ignored metadata inputs, so a build-only command could leave `Info.plist` stale; added metadata hashes to the stamp.
- Low: README did not state the new successful-build prerequisite for each launch; clarified failure behavior.
- Low: README's unverified-launch wording conflicted with a previously authenticated app launch; scoped it to the new `start.sh`.
- Low: README dropped prior test and debug-build evidence; restored it as prior evidence.
