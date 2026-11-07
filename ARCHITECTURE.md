# GPTray 0.3 architecture and scoped security review

This describes implementation and source inspection. It does **not** assert that Windows GUI, account login or RAM consumption have been manually verified for 0.2.

## One window, one page

All entry points serialize native actions on Tauri's UI thread. `get_or_create_panel` is the only WebView builder and uses label `panel`. Startup declares `app.windows: []`. Full and Quick share the existing WebView: `set_decorations`, `set_always_on_top`, `set_skip_taskbar`, `set_resizable`, `set_maximizable`, `set_minimizable`, size/position and maximize/minimize APIs change the native window. No URL reload, JS injection, DOM selectors or reading of conversations is used. These setters and Windows support were checked against locked Tauri **2.11.5** sources and type-checked for `x86_64-pc-windows-msvc`. Interactive behavior still needs Windows verification.

The factory chooses mode size/chrome before creating the WebView; native controller bounds synchronize before show. Existing mode changes temporarily hide the parent to conceal intermediate viewports, with no policy-hide timer. Reopening Full avoids redundant unmaximize/resize.

Quick is borderless/topmost/taskbarless and hides on blur. Full has standard chrome and ignores blur. The shortcut always selects Quick (hiding it if already visible), so it also provides a way back from Full. Ctrl+Shift+Enter opens/restores Full; registration is independent of Ctrl+Shift+Space. Conflicts leave the app running and are visible in disabled tray status rows. Native close hides; Quit exits. Window-open popups are denied through `NewWindowResponse::Deny` to prevent untracked extra WebViews. This can limit OAuth popups, external links and other site features; validate login methods on Windows before relying on them. No browser popup workaround is claimed.

`geometry.rs` stores a versioned text record of physical top-left position, logical normal content size and maximized state. Bounds are cached during normal window events and written on hide, switch, RAM release and Quit, using a temporary sibling then rename. It never stores page contents or URLs. Corrupt/nonfinite/unbounded sizes fall back to defaults. Restore fits bounds to connected monitor work areas, reserving room for titlebar/borders. A crash or power loss can lose the most recent unsaved geometry. Extreme DPI/native event ordering needs manual testing.

See [native memory/rendering evidence](docs/WEBVIEW2-MEMORY.md) for the verified Graphify baseline, API version matrix, typed COM access, Low/Normal policy and reasons for rejecting suspend/cache/JS interventions.

## Hide, destroy, exit

| Operation | Live page | Persistent profile | Host/tray |
|---|---|---|---|
| Hide / native close | Retained; Low memory target, scripts/network continue | Retained | Running |
| Mode switch | Reused, no intentional reload | Same | Running |
| Free RAM | Destroyed; drafts/replies may be lost/stopped | Retained, not cleared | Running |
| Quit | Closed with application | Retained | Exits |

Default: **no automatic destructive unloading**. There is no supported reliable ChatGPT busy/draft signal at the native WebView boundary. Detecting DOM internals would be brittle, so it is not attempted. The optional auto-discard setting warns in the same submenu as manual discard and resets off every app start. Its safety boundary is explicit user acceptance of losing hidden page state, not a claim to detect whether ChatGPT is idle.

For this opt-in only, inactivity means continuously **hidden** since the last successful hide, not keyboard inactivity, focus loss in Full, or minimizing. Enabling while hidden starts a fresh 120-second period. A monotonic clock and epoch-tagged deadline reject stale queued timeouts after show, opt-out, release or a new hidden period. The final UI-thread check also requires the native window to be invisible; errors fail closed. No hidden page is inspected. A single cancellable receiver thread starts only on first opt-in, blocks when unarmed and stops on Quit. There are no polling loops, periodic page tasks or per-hide sleeping threads. An asynchronous destruction guard prevents recreating the same label before cleanup; pending mode opens are queued after destruction. Automatic last-window exit is prevented so manual RAM release leaves tray/hotkey alive.

## Profile and session

The app identifier remains `com.globalvox.chatgpt-overlay-bar`. Tauri's locked Windows implementation defaults the WebView data directory to LocalData + identifier; 0.3 explicitly passes that same `app_local_data_dir()`. WebView2 normally stores browser profile files under its `EBWebView` subdirectory there. Neither normal hide, destroy nor Quit calls a clear-data/cookie API or changes profile location. No incognito/in-private mode or unsafe Chromium flags are used.

Source inspection verifies path continuity and absence of explicit clearing. Retention of a real authenticated session still requires Windows testing without sharing credentials. Website expiry, account policy, logout, WebView2 behavior and provider restrictions can require a fresh login. Disk cookies do not preserve arbitrary unsent DOM state after destroy.

## Performance review

- Lazy construction remains: no WebView before first use. No new background service, network client or telemetry.
- One label and one shared mode-aware factory; mode switches reuse it. Popup creation is denied. Window listeners are attached once per created window and disposed through Tauri destruction; no global frontend listeners are added.
- Single opt-in destructive timer worker; no timer is armed in the default configuration. Non-destructive Low/Normal hints use native state changes, with no polling or extra worker.
- Full-window geometry writes are tied to user lifecycle actions; resize/move events update memory only.
- Destroy uses Tauri's native close path rather than process killing, working-set trimming or disabling browser sandbox/security. WebView2 may retain/reparent helper processes temporarily. No claim of leak-free behavior or measured savings is made; the repeated-cycle process-group procedure must establish it on Windows.
- Two Windows-only direct dependencies expose existing locked webview2-com/windows versions for typed native API access. A scoped security update also advances plist/quick-xml and adds the required base64 version. Root metadata advances to 0.3.0. npm/RustSec checks and retained informational warnings are recorded in [SECURITY-REVIEW.md](docs/SECURITY-REVIEW.md); this is not a complete dependency audit.

## Security review of changed surfaces

Previously `csp: null` applied to bundled assets. The new restrictive local CSP blocks scripts, connections, frames/forms by default, permitting only local styles/images. This does not rewrite or replace `chatgpt.com`'s HTTP CSP; remote content and its login flow retain their own policies. Bundled scaffold assets are unused. The capability grants no permissions, is not enabled for local content, and has no remote origins. All native controls (tray, global shortcut, autostart, geometry files) run in Rust; no custom invoke commands are registered. No web content needs native IPC. The unchanged plugin crates still run natively; a browser/runtime compromise remains a risk.

External HTTPS content is rendered by WebView2. GPTray does not read, export, log or forward conversation content, collect passwords, or introduce telemetry. Website/WebView2 network behavior remains controlled by their providers. Native navigation remains available for login redirects; this is not a domain-isolated browser. Popups are blocked, which trades compatibility for a bounded window lifecycle. Downloads and authentication must be tested according to website/WebView2 behavior. Local profile files contain sensitive session data and should be protected/backed up as browser data. The unsigned installer has no publisher identity assurance. A checksum checks downloaded bytes against the CI artifact, not publisher trust or full application safety.

This is a targeted source/configuration review, **not a full security audit**. GitHub Actions uses hosted Windows 2022, read-only token permissions, checkout without stored credentials, locked installs and artifact retention fourteen days; no deployment, release publishing or auto-merge is configured.

## Feature availability

GPTray exposes what `chatgpt.com` offers the account in WebView2. Plus/history/memory are website features, not reimplemented. ChatGPT Work, Codex, Computer Use, voice, uploads, OAuth popups, desktop integrations and permissions are **not verified to match** the official Windows app. No official-app feature parity is claimed. See the explicit manual matrix in `docs/WINDOWS-VALIDATION.md`.
