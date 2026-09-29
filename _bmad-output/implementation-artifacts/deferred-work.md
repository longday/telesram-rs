- source_spec: `_bmad-output/implementation-artifacts/spec-conventional-production-bundle.md`
  summary: Verify camera, microphone, screen sharing, and Web Inspector in the ad-hoc-signed Hardened Runtime release bundle.
  evidence: The signed entitlements and runtime flag are present, but an actual call/capture and private inspector interaction with the new bundle were not performed; those interactions would settle the risk without modifying the working profile.
- source_spec: `_bmad-output/implementation-artifacts/spec-conventional-production-bundle.md`
  summary: Determine which application receives macOS media permissions when `start.sh` executes the bundle binary directly.
  evidence: The launcher executes `Contents/MacOS/telesram-rs` rather than using LaunchServices; a real permission request and inspection of macOS privacy attribution would settle whether prompts are attributed to the app or its terminal.
- source_spec: `_bmad-output/implementation-artifacts/spec-menu-reload-and-home.md`
  summary: Add a Cmd+R accelerator for Reload Page.
  evidence: Both new menu items are created with no accelerator (`src/menu.rs`), so the standard refresh shortcut does nothing.
- source_spec: `_bmad-output/implementation-artifacts/spec-unread-spa-routes.md`
  summary: Verify that live meeting titles do not contain digits unrelated to unread messages (medium, unverified).
  evidence: Unread eligibility now follows the Telemost origin across paths. The existing digit-based title heuristic could count meeting IDs or participant numbers if present; the public `/j/123?skip_app=1` response title is `Яндекс Телемост`, but real-call titles were not inspected. A real-call title observation would settle this without changing message read state.
