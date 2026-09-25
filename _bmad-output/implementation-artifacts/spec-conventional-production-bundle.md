---
title: 'Build a conventional macOS release bundle'
type: 'feature'
created: '2026-09-26'
status: 'done'
route: 'dispatch'
review_loop_iteration: 1
baseline_commit: '64b1ab5f5ee164588ea0d32bb232d0034aef006f'
context: []
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** `build.sh` compiles an optimized Rust binary into `.runtime/target/release` but assembles a minimal ad-hoc-signed `.app` by hand in `.runtime/Telesram.app`. The user wants a real production build in a standard Rust/Tauri output location rather than this manually assembled runtime bundle.

**Approach:** Produce a macOS application bundle through the conventional Tauri release build path, under `.runtime/target/release/bundle/macos`, and make `start.sh` run the resulting bundle. Keep build-only and launch commands separate and document the artifact and its distribution limits.

**Decision:** Build a local optimized Tauri `.app` with ad-hoc signing. Keep the previously approved Web Inspector; do not add Developer ID signing, notarization, or public distribution.

## Boundaries & Constraints

**Always:** Use locked Rust dependencies and a pinned project-local Tauri CLI, Apple Silicon macOS, the existing bundle identifier and user-controlled WebKit session; leave `reset.sh` and profile data untouched. Ad-hoc sign the bundle, retain camera/microphone capability, and keep the documented distinction from Developer ID signing/notarization.

**Never:** Claim an ad-hoc signed bundle is notarized or suitable for public distribution. Do not publish or request credentials silently.
</frozen-after-approval>

## Code Map

- `build.sh` -- baseline manually packages `.runtime/Telesram.app`. Use pinned `tauri-cli` 2.11.5 (matching `tauri` 2.11.5) with a project-local locked install and separate temporary target directory. Reinstall if the local binary version differs; invoke that exact binary directly. Remove its generated compile cache after successful install. Forward `--locked` to Cargo, clear inherited Apple signing/notarization variables, and verify the signed bundle contains both media entitlements.
- `start.sh` -- calls `build.sh`, then executes `.runtime/Telesram.app/Contents/MacOS/telesram-rs`; must follow the new release bundle location.
- `dev.sh` -- maintains a debug bundle at `.runtime/Telesram.app`; its stamp is only consumed by the current `build.sh` and becomes obsolete at cutover. Keep debug launch and identifier; remove dead stamp writes.
- `Cargo.toml`, `tauri.conf.json`, `Info.plist` -- release profile, bundle target/icon/usage strings and enabled `devtools`. Tauri's `hardenedRuntime` defaults to true; configure ad-hoc `signingIdentity` explicitly and attach camera/microphone entitlements.
- `README.md` -- document release/debug artifact paths, local signing, Web Inspector limitations, differing Hardened Runtime flags and permission reapproval risk.
- `.gitignore` -- `.runtime/` is ignored, so the standard Cargo/Tauri target structure can stay repository-local.

## Tasks & Acceptance

**Execution:**
- [x] `build.sh` -- use pinned local Tauri CLI 2.11.5 for a locked release `.app` build; auto-reinstall on version mismatch, remove successful-install cache, invoke the checked executable directly, clear inherited Apple release credentials and validate signed media entitlements.
- [x] `tauri.conf.json`, `Entitlements.plist` -- ad-hoc sign through Tauri with Hardened Runtime and camera/microphone entitlements, without changing the bundle identifier.
- [x] `start.sh` -- execute the release bundle only after successful `build.sh`.
- [x] `dev.sh` -- remove obsolete stamp writes; keep existing debug bundle workflow.
- [x] `README.md` -- document both artifact paths, different signing/runtime modes, shared profile, permission reapproval risk, and no public distribution guarantee.

**Acceptance Criteria:**
- Given native prerequisites and no app running, when `./build.sh` succeeds, then an optimized `.app` exists in `.runtime/target/release/bundle/macos` without launching it.
- Given the produced bundle, when its signature is inspected, then it is ad-hoc and strictly valid with camera/microphone entitlements; Tauri reports that notarization was skipped.
- Given a successful release build, when `./start.sh` runs, then it builds first and launches that bundle; on build failure it does not launch.
- Given a debug workflow, when `./dev.sh` runs, then it keeps its existing bundle location and profile and does not overwrite the release artifact.

## Implementation Notes

`build.sh` now installs pinned `tauri-cli` 2.9.6 under `.runtime/` on first use and calls `cargo tauri build --ci --bundles app --features custom-protocol -- --locked`. The app is in `.runtime/target/release/bundle/macos/Telesram.app`. Tauri rewrote the pinned `tauri-build` dependency in `Cargo.toml` to its equivalent table form with `features = []`; a second build left it stable. `start.sh` executes that release bundle, while `dev.sh` retains the debug bundle and no longer writes the unused stamp.

Both builds completed; the second compiled incrementally. `codesign --verify --deep --strict` passed. `codesign -dv --verbose=4` showed `Signature=adhoc` and `flags=0x10002(adhoc,runtime)`; `codesign -d --entitlements :-` included camera and audio-input. Tauri reported notarization skipped. The build-only command did not start the app. Isolated start and debug fixtures passed, including failure-before-launch and preservation of the release bundle. Actual media permissions in a call and the working WebKit profile were not exercised.

Review loop re-derived the build with `tauri-cli` 2.11.5. The existing 2.9.6 project-local CLI was replaced automatically, and its 1.3 GB compiler cache was removed after installation. `build.sh` now invokes the checked binary directly and validates the bundle signature and camera/audio-input keys. An incremental rebuild with synthetic Apple signing/notarization variables remained ad-hoc and reported notarization skipped. The final isolated start/debug fixtures passed; the release bundle was unchanged by the debug fixture. The actual call/capture and private inspector under Hardened Runtime remain unverified.

Final review patch made Hardened Runtime explicit, checked the ad-hoc identity and runtime flag after strict signature verification, consolidated the CLI pin, and removed the redundant release feature argument. Both a normal build and a build with synthetic Apple signing variables succeeded after the patch; notarization remained skipped. Documentation now distinguishes stale pre-cutover bundles, possible manifest rewrites, and permission reapproval after code-changing rebuilds.

## Verification

**Commands:**
- `./build.sh` -- Tauri release bundle at the documented path; strict ad-hoc signature, Hardened Runtime and media entitlements verified after packaging. Repeat with synthetic Apple env vars to confirm local signing.
- Isolated `start.sh` fixture -- build-before-exec and no launch on build failure without opening the working profile.
- Isolated `dev.sh` fixture -- signed debug app launches at its previous path without replacing the release bundle.

## Design Notes

Prefer Tauri's own ad-hoc signing (`signingIdentity: "-"`) and an entitlements plist for camera and microphone rather than disabling the default Hardened Runtime. Explicitly `unset` `APPLE_CERTIFICATE`, `APPLE_CERTIFICATE_PASSWORD`, `APPLE_SIGNING_IDENTITY`, `APPLE_ID`, `APPLE_PASSWORD`, `APPLE_TEAM_ID`, `APPLE_API_KEY`, `APPLE_API_ISSUER`, and `APPLE_API_KEY_PATH` before the CLI to prevent inherited release credentials from changing the local build.

## Review Triage Log

- Medium: CLI 2.9.6 is older than the pinned Tauri runtime 2.11.5; the local registry has `tauri-cli` 2.11.5. Re-plan the CLI pin to avoid divergent Tauri config models.
- Medium: the current version gate fails after a pin bump because the old CLI binary remains installed; reinstallation must be automatic when the version differs.
- Low: the CLI compilation cache occupies 1.3 GB after installation; remove the generated target directory after a successful install.
- Low: indirect `cargo tauri` may select a different external subcommand than the checked local executable if one exists under Cargo home; invoke the checked executable directly.
- Maybe-false: Hardened Runtime may change camera, microphone, screen share, or private inspector behavior; an actual call and inspector interaction on the new bundle would settle this. Keep as a documented unverified integration risk.
- Low: debug and release bundles now have different signing/runtime flags; document that debug cannot validate release media permissions.
- Low rejected: stale `.runtime/Telesram.app.build` has no reader; its old `.app` remains the debug path and will be replaced on the next debug launch. Auto-removal is not needed for ordinary use.
- False: the spec's Verification section omitted debug, but the isolated `dev.sh` fixture ran and preserved the release bundle; add its evidence to the spec.
- Low: “no Apple service was contacted” exceeds the evidence of a skipped notarization; narrow the criterion to the observed outcome.
- Medium: the build can succeed with absent camera/audio entitlements; validate the signed bundle's keys after packaging, while real-call behavior remains an explicit manual limit.
- Maybe-false: inherited Apple env overrides were not exercised in the first builds; a build with synthetic Apple variables and an ad-hoc result would settle it.
- Low rejected: the first Implementation Notes paragraph says "now" about CLI 2.9.6, but it records the initial implementation; the later review-loop paragraph explicitly supersedes it with CLI 2.11.5. Editing the spec alone does not fix build behavior.
- Medium patched: `build.sh` previously verified signature validity and media keys but not the ad-hoc identity or Hardened Runtime flag. Explicitly configure Hardened Runtime and reject a signed bundle missing either property.
- Low patched: separate CLI-version literals in `build.sh` could diverge during a future pin update and trigger repeated reinstalls; derive install and version checks from one variable.
- Low patched: the release CLI adds the Tauri custom-protocol feature already; remove the redundant explicit feature argument without changing the debug workflow.
- Low patched: Tauri rewrote `Cargo.toml` to its equivalent dependency form on the first build and may update it again for configuration-driven features; document that possible tracked-file mutation.
- Maybe-false carried: Hardened Runtime may affect media capture, screen sharing, or the private inspector; a real call and inspector check with the new bundle would settle this. Defer the unverified integration risk without touching the working profile.
- Maybe-false deferred: executing the bundle binary directly might attribute TCC prompts to the launching terminal rather than the application; observe a permission request and its macOS privacy attribution before changing `start.sh` semantics.
- Maybe-false rejected as low: ad-hoc signature changes may require fresh media permission after a code-changing rebuild, but this was not observed. The README now mentions rebuilds as well as bundle switching without promising a particular TCC outcome.
- Low patched: an older `.runtime/Telesram.app` can remain a release bundle until `dev.sh` replaces it; document that transition and manual removal of the unused old stamp rather than deleting user artifacts automatically.

## Spec Change Log

- Review found that `tauri-cli` 2.11.5 is available and matches the pinned `tauri` runtime, while the initial 2.9.6 pin could drift in configuration behavior. Updated the Code Map and tasks to pin 2.11.5, automatically replace an older local CLI, invoke the checked binary, remove its 1.3 GB compile cache, and verify signed media entitlements. This avoids a stale CLI or a green build with missing media entitlements. KEEP: Tauri-generated app path, ad-hoc signing with Hardened Runtime, the shared profile, distinct debug bundle, locked Cargo builds, isolated launcher checks and zero working-profile interactions.
