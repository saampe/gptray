# GPTray 0.3: memory and rendering evidence

## Baseline and Graphify

Development starts from merged main `6cf978e1efdb55d6195571c54fc8a6c437a52a1f`. Graphify graph_stats returned that exact SHA, **141 code nodes / 225 relationships**, indexed 2026-11-07. `query_graph` mapped get_or_create_panel → open/native_mode, hide/release and pending reopen; `graphify_rank_files` ranked lifecycle.rs and lib.rs; `impact_and_risk` identified lib.rs, lifecycle.rs and geometry.rs as change hotspots. `graphify_tests_for` returned no links for lib.rs or lifecycle.rs. This is incomplete graph coverage: 15 baseline tests exist and passed on direct execution. The graph is analysis evidence, not a test run or proof of safety. All changed code and locked dependency implementations were read directly.

## Official APIs and actual access

| Surface | Verified in locked sources | Runtime support |
|---|---|---|
| Tauri 2.11.5 | `WebviewWindow::with_webview` → UI-thread `PlatformWebview::controller()` | Existing native controller |
| Wry 0.55.1 | `WebViewExtWindows::set_memory_usage_level` uses typed `_19` and `SetMemoryUsageTargetLevel`; Tauri's PlatformWebview does not expose this Wry setter | Runtime 114.0.1823.32+ |
| webview2-com 0.38.2 | `_19`, Low/Normal constants, setters/getters; `_3` TrySuspend/Resume | QueryInterface checks installed runtime |
| Windows 0.61.3 | typed COM `Interface::cast`, RECT | Same versions already in Cargo.lock |

Two **Windows-only direct dependency declarations** expose already locked webview2-com and windows packages. Separately, the RustSec review required a compatible plist/quick-xml security update and its base64 dependency; see [security results](SECURITY-REVIEW.md) for exact versions and retained warnings. Typed QueryInterface is used, not handwritten COM layouts or speculative vtables. Missing `_19` is handled as unsupported for that WebView; normal operation continues. Transient errors leave future requests retryable. Native COM references are local to one UI callback and are not retained in state.

Official references reviewed:

- [Performance best practices](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/performance)
- [MemoryUsageTargetLevel remarks](https://learn.microsoft.com/en-us/dotnet/api/microsoft.web.webview2.core.corewebview2.memoryusagetargetlevel)
- [ICoreWebView2_19](https://learn.microsoft.com/en-us/microsoft-edge/webview2/reference/win32/icorewebview2_19) (SDK introduction 1.0.1823.32)
- [ICoreWebView2_3](https://learn.microsoft.com/en-us/microsoft-edge/webview2/reference/win32/icorewebview2_3) (TrySuspend/Resume SDK introduction 1.0.774.44)

## Selected strategy

Automatically request **Low** after actual hide, minimization, or confirmed loss of foreground while Full stays visible. Request **Normal** before opening/refocusing/restoring. Revalidate foreground ownership for delayed focus notifications. Unknown foreground on a visible, restored window chooses Normal. Set/get target acceptance is checked; acceptance is not measured RAM reduction. A generation/revision ticket prevents a queued Low callback from overriding Normal or touching a recreated page. Successful repeated hints are deduplicated. There is no new timer, polling task, setting or WebView prewarm for this feature.

Microsoft states that Low does not stop scripts or network connections. It may evict caches/page memory to disk, trading responsiveness for a lower private working set. It is **best effort**, not a memory cap; busy ChatGPT scripts can fault memory back in. Normal must be restored explicitly. Keep Evergreen updated for security and runtime improvements.

TrySuspend was evaluated and **not enabled**: it requires an invisible controller, pauses scripts/timers/animations and could disrupt an unknown streaming response or synchronization. Microsoft advises choosing either Low/Normal or suspend/resume; they are not mixed here. No safe default can infer ChatGPT generation/draft state from the native boundary. Destructive Free RAM remains manual or explicitly warned session-only opt-in, off at every startup.

Active pages keep Normal as Microsoft recommends. No unmeasured active Low policy, JavaScript heap cap, injected garbage collection, automatic refresh, cache clearing or disabled browser security/GPU is introduced. ChatGPT owns its JS/DOM/network workload; conversation growth may keep renderers alive, and process presence alone is not a leak. The benchmark records renderer/GPU/utility/browser roles so growth after matched questions/conversation switches can be investigated without recording their contents. DevTools heap snapshots require deliberate local inspection and may contain conversation data; GPTray does not collect them.

## Full Window rendering

The baseline always built a 400×600 borderless Quick WebView, then changed the native parent to Full. Wry handles later resize notifications asynchronously, so the native client could grow before its WebView viewport. This explains the reported shape from source, but a live ChatGPT frame trace was not available during development.

The shared factory now sets requested content size, decorations, taskbar/topmost/resizability and saved maximization **before constructing WebView2**. Existing mode changes temporarily hide the native window without arming an inactivity timer, apply native properties and synchronize controller bounds to the current physical client size before showing. Opening an already Full window restores/focuses it without an unnecessary unmaximize/resize sequence. No second WebView, navigation, artificial one-second delay or injected resize JavaScript is used. Actual website/network/compositor first paint can still take time; no instant-content claim is made.

## Evidence boundaries

Pure tests validate policy decisions/races and initial mode specs. Locked Windows-target compilation verifies typed API integration. The separate native smoke example uses **an isolated `about:blank` document and a unique temporary profile**, exercises the production factory and adapter, reads initial/resized controller bounds and Low/Normal targets, then shows/hides/exits. Its CI log/result explicitly records pass/fail. It is not a ChatGPT login, rendering-quality or account-state test, and never opens the personal profile. Actual Windows 11 ChatGPT drafts/streaming/layout, runtime-dependent savings and process cleanup need the matched benchmark/acceptance session.

The user's ~450–470 MB during activity, and older ~139.9/125 MB figures, are uncontrolled observations with incompletely specified process grouping/metrics. They are not verified comparison baselines. No numerical RAM improvement is promised or fabricated.


The first native CI attempts failed at Windows loader status `0xC0000139`. MSVC import inspection identified `comctl32.dll!TaskDialogIndirect`. Locked tauri-winres/embed-resource emits the Common Controls v6 manifest resource for **binaries**, not Cargo **examples**. The probe's build now links that same already generated `resource.lib` into examples; the shipping app already receives it. This is a test executable packaging correction, not a disabled security feature or a speculative COM workaround. Only the final successful probe result counts as execution evidence.


The native regression also exposed nested dispatcher deadlock: the locked runtime holds its WebView-id mutex during `with_webview`, so calling another `with_webview` (or indirectly dispatching reentrant events) inside that callback can hang. Production show and the probe now keep callbacks **COM-only**, return through a nonblocking result channel on the verified owning UI thread, then perform native show/focus/resize operations outside the callback. Off-thread/deferred use fails explicitly rather than blocking the UI. The same actual smoke regression checks this path through show/hide, not just successful compilation.
