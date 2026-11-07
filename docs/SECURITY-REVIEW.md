# GPTray 0.3 targeted dependency/configuration review

Date: 2026-11-07. This is a scoped source/dependency review, not a full security audit.

## Checks and security update

- `npm audit`: zero reported vulnerabilities for the locked frontend development tooling.
- `cargo-audit 0.22.2`, RustSec database commit `309392b18f6b17022f82c701f300433126901068` (1422 advisories, last updated 2026-11-06): the baseline lock reported **two vulnerabilities** in quick-xml 0.39.4, through plist/Tauri utils. RUSTSEC-2026-0194 is quadratic duplicate-attribute checking; RUSTSEC-2026-0195 is unbounded namespace-declaration allocation.
- A compatible, narrow Cargo lock update changes **plist 1.9.0 → 1.10.3** and **quick-xml 0.39.4 → 0.42.0** (patched versions >=0.41.0). The updated plist requires **base64 0.23.1**, added alongside the already used base64 0.22.1. Other package versions remain unchanged, including Tauri/Wry/WebView2/Windows.
- Base64 is a local encoding/decoding library, MIT OR Apache-2.0, no build script; the declared features/dependency origin and lock checksum were inspected. It adds no service, permissions, credential handling or telemetry. It is required by the security update, not a speculative performance dependency.
- Re-audit of the resulting lock: **zero reported vulnerabilities**, exit 0, with **eight informational warnings retained** (below). This does not mean every dependency is maintained or the application is proven safe.

## Retained informational warnings

Six unmaintained-crate warnings remain in the baseline dependency family: proc-macro-error 1.0.4 (RUSTSEC-2024-0370), unic-char-property 0.9.0 (2025-0081), unic-char-range 0.9.0 (2025-0075), unic-common 0.9.0 (2025-0080), unic-ucd-ident 0.9.0 (2025-0100), and unic-ucd-version 0.9.0 (2025-0098). They are disclosed rather than suppressed; replacing that framework dependency family is outside this focused change.

Two unsoundness warnings also remain:

- glib 0.18.5 / RUSTSEC-2024-0429: VariantStrIter. `cargo tree --target x86_64-pc-windows-msvc -i glib` reports nothing; this is the Linux GTK dependency side, not the Windows distribution graph.
- tokio 1.52.3 / RUSTSEC-2026-0386: `tokio::io::unix::AsyncFd` constructor I/O safety. The reported functions are Unix-only and are not compiled into the Windows target. No claim is made that a Linux build is audited/remediated by this Windows-focused PR.

The RustSec report/target dependency paths were inspected, not dismissed based on a successful compile. Future framework/runtime updates and fresh advisory checks remain appropriate.

## Changed native/security boundaries

Two pinned Windows-only direct declarations expose **existing** locked webview2-com 0.38.2 and windows 0.61.3 for typed native APIs. QueryInterface checks `_19` before use; unsupported runtime uses normal behavior. No handcrafted COM vtables, raw layout casts, browser-security switches, process-memory trimming or page-data APIs are introduced. Native references are scoped to Tauri's UI-thread callback; stale callbacks are invalidated on reopen/destruction.

Identifier/profile/CSP/empty web-content IPC grants remain unchanged. No custom native invoke commands, website script injection, conversation/session/password collection or telemetry were added. Popup WebViews remain denied: improving popup compatibility without a tracked extra browser or unsafe navigation replacement was not established, so the existing explicit compatibility boundary is retained. Top-level HTTPS login redirects remain available; the app is not a domain-isolated browser. OAuth popup paths and official-app feature parity still need real account testing.

Manual Free RAM and the opt-in destructive timer retain explicit page-state-loss warnings. Low/Normal does not inspect or destroy page state; real authenticated streaming, drafts, runtime behavior and memory savings still require the scoped Windows acceptance session. The NSIS installer is unsigned, and its SHA-256 is byte-integrity evidence rather than publisher identity assurance.
