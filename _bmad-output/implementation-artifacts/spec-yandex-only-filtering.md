---
title: 'Replace broad tracking lists with a Yandex-only Telemost profile'
type: 'feature'
created: '2026-09-29'
status: 'done'
route: 'oneshot'
review_loop_iteration: 0
context: []
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

Replace the broad EasyPrivacy/uBlock subset with a small Yandex-only tracking and advertising profile for Telemost, as approved by the user. Use selected existing Yandex rules compatible with native WebKit, not regional or global lists. Preserve Managed Mode toggling, essential Telemost/authentication/CDN traffic, document navigation, and hiding the Telemost global bar. Do not import unrelated search/market cosmetics, block whole Yandex/CDN/service domains, change authentication/system settings, send messages, or initiate calls. Verify network-rule behavior with native WebKit and check the working application without mutating chat read state; report the limits of live-call and fresh-login verification.

</frozen-after-approval>

## Implementation Notes

- Keep `assets/managed-rules.json` as the direct, human-readable WebKit profile. Select explicit dedicated tracker hosts and exact-root-host tracking paths from the existing pinned EasyPrivacy snapshot plus the four existing application tracker rules. No runtime conversion, new dependency, or auto-update service is needed.
- Remove obsolete broad source snapshots under `assets/filter-sources/` and `scripts/generate_managed_rules.py`. Their provenance remains available through pinned upstream links and repository history, not through bundled unused lists.
- Exclude global substring rules, unknown-domain wildcards, browser-updater/extmaps resources, and uBlock scriptlets. Dedicated tracker domains allow subdomains; path rules on mixed-use Yandex domains match only their exact hosts. Keep the existing network resource categories, excluding documents.
- Broad service-host exceptions are unnecessary once generic blockers are removed: the small profile has no blocks on Telemost, Passport, ID, cookie-helper, or general CDN hosts. The CSS selector is restricted to the two Telemost HTTPS origins.
- Retain the native content-list identifier in `src/macos.rs`: recompilation replaces the existing stored list without leaving an obsolete broad-list cache. Reuse compile-before-navigation, toggle, removal and stale-completion behavior unchanged.
- Native smoke will compile the exact JSON and exercise blocked Yandex trackers, allowed non-Yandex trackers, lookalike hosts, service/CDN/auth paths, document exemptions, CSS scoping, and removal/reinstallation in an isolated nonpersistent WKWebView. A local fixture must not contact trackers or read the working profile.
- README must name the scope, source revisions, manual maintenance procedure and verification limits; no claim of comprehensive analytics coverage or measured traffic savings.
- Review corrections: include exact-host `ya.ru/clck/` and the pinned uAssets `yandexmetrica.com` rule. Final profile: 33 network rules and one scoped CSS rule.
- Keep `scripts/check_managed_rules.m` as the native boundary regression checker. Image and fetch requests exercise resource types separately, using distinct local request tokens; temporary compiled stores are removed on success or failure.

## Verification

- Native checker passed: 26 image and 26 fetch cases in each of off/on/off, plus document exemptions and scoped CSS. All responses were local; no real tracker requests or persistent profile data were used.
- `cargo test --locked --features custom-protocol`: eight passed.
- `./build.sh`: signed release bundle built successfully. The user authorized restarting the application; the rebuilt bundle was launched.
- The desktop session was locked, preventing final window inspection. Fresh login, message delivery and live-call media remain unverified; no chats were opened, messages sent or calls initiated.

## Review

- Added omitted `ya.ru/clck/` and `yandexmetrica.com` rules.
- Retained the existing content-list identifier to avoid leaving the old compiled list behind.
- No broad-list converter, bundled source snapshots or runtime API changes remain.
