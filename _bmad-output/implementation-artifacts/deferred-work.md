- source_spec: `_bmad-output/implementation-artifacts/spec-conventional-production-bundle.md`
  summary: Verify camera, microphone, screen sharing, and Web Inspector in the ad-hoc-signed Hardened Runtime release bundle.
  evidence: The signed entitlements and runtime flag are present, but an actual call/capture and private inspector interaction with the new bundle were not performed; those interactions would settle the risk without modifying the working profile.
- source_spec: `_bmad-output/implementation-artifacts/spec-conventional-production-bundle.md`
  summary: Determine which application receives macOS media permissions when `start.sh` executes the bundle binary directly.
  evidence: The launcher executes `Contents/MacOS/telesram-rs` rather than using LaunchServices; a real permission request and inspection of macOS privacy attribution would settle whether prompts are attributed to the app or its terminal.
