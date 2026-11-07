# Windows acceptance and memory measurement

## Evidence boundary

Automated policy tests cover mode-dependent blur decisions, safe defaults, deadlines and stale cancellation, repeated mode transitions, geometry serialization/validation, removed monitors and DPI fitting. Windows CI additionally runs the Rust application test suite, formats, compiles/packages with MSVC, tests PowerShell grouping fixtures and uploads an NSIS installer/hash. These are not interactive GUI tests.

The development environment is Linux. No Windows desktop or user credentials are available here. Until completed on Windows, mark every item below **not manually verified**. Do not fill measurement cells from guesses or earlier README claims.

## Acceptance checklist (Windows 11 reference: i3-8130U, 4 GB RAM, UHD 620)

Record Windows build, WebView2 version (`msedgewebview2.exe` Details), GPTray commit, display resolution/DPI, power mode and other open applications. Keep only one GPTray host running. First test without login; enter credentials only locally if you choose to test your account.

1. Start/quit five times before ever opening: tray available, no window/taskbar flash and no GPTray-associated WebView2 instance before first open. Quit removes the host; observe child processes for at least 30 seconds.
2. Hotkey, tray left-click and Toggle Quick Panel: open and hide a ~400 × 600 logical-pixel topmost panel; click another app to hide it; no taskbar entry. Check tray-click close behavior separately (blur and mouse event ordering can differ).
3. Open Full Window: native titlebar/system controls, taskbar/Alt-Tab entry, resize/minimize/maximize. Clicking another app must not hide Full. Native close hides to tray; Quit exits. Opening a minimized Full restores it.
4. Enter distinctive **unsent** text; switch Quick ↔ Full at least 20 times and verify text, page URL, scroll and active response are retained without a reload. While a harmless response generates, switch and hide/reopen in default mode; verify no intentional interruption. Inspect process tree for one application-created WebView, not two page instances (Chromium helper processes are expected).
5. Move/resize Full, maximize, return Quick, return Full. Quit/restart and verify normal restore bounds plus maximized state. Repeat unmaximized, minimized then Quit, secondary monitor at negative coordinates, 100/150/200% DPI, and unplugged monitor. Titlebar must remain reachable. Corrupt `window-bounds-v1.txt` with GPTray closed; defaults must recover.
6. Default auto-discard is unchecked. Hide for >2 minutes with unsent text: reopen must retain it. Repeat with a response. A visible, unfocused Full and a minimized Full must never unload, even when the risky auto option is enabled.
7. Use a disposable unsent draft for manual Free RAM: confirm the menu warns, choose discard, host/tray stays running, WebView disappears, next hotkey reloads it. Repeat 20 times and rapidly press hotkey while releasing; no label-conflict failures or extra windows. Child processes may exit asynchronously; record their lifetimes, do not assume immediate freeing.
8. Explicitly opt into auto-discard using disposable state. Hide 110 seconds → reopen → hide: it must not unload until 120 seconds after the second hide. Reopen around the deadline; stale queued callbacks must not destroy a visible page. Disable around deadline and ensure no later discard. Restart and confirm option is unchecked again. This mode intentionally can lose hidden drafts/stop hidden replies.
9. Login via `https://chatgpt.com`: test the actual login method, including provider redirects/popups. Verify cookies after hide, Free RAM, Quit/restart and Windows restart. Check account/Plus, history and personalization through the website. **Do not send credentials or profile files for this test.** Popups are blocked; record login failures as compatibility issues.
10. Launch at Login on/off; restart Windows for each state. Check no second app host, no stuck child processes after Quit and no unwanted reload/duplicates over repeated open/close/switch cycles.

| Website/official-app capability | Windows manual result |
|---|---|
| Login (record specific method), Plus, history, memory | Not verified |
| Work / Codex / Computer Use | Not verified; no app parity claimed |
| Voice, uploads, downloads, external links and popup-required flows | Not verified; popups intentionally blocked |

If login or essential functionality fails, keep the PR Draft; a build alone is insufficient for acceptance.

## Reproducible process-group measurement

Use Windows PowerShell 5.1 or PowerShell 7 as the same user running GPTray. No administrator rights should be needed; access/counter errors produce incomplete samples instead of zero-filled results. Close any second GPTray sharing the profile. Do not include unrelated Edge/WebView2 hosts from other applications.

```powershell
Get-Process chatgpt-overlay-bar | Select-Object Id, StartTime
# Use the host Id shown above, not a WebView2 PID:
.\scripts\Measure-GPTrayMemory.ps1 -RootProcessId 1234 -Scenario '02-quick-open' -Samples 30 -OutputDirectory .\memory-results\v02
```

`RootProcessId` is mandatory. The script groups the host and all descendants from `Win32_Process`, recursively; remembers PID **and creation time** to retain reparented survivors across samples without accepting recycled PIDs; and recovers detached `msedgewebview2.exe` processes whose exact `--user-data-dir` matches the app profile or its `EBWebView` subdirectory. The default profile is `%LOCALAPPDATA%\com.globalvox.chatgpt-overlay-bar`; override `-ProfileDirectory` for a custom location. It does not sum all `msedgewebview2.exe` processes on the machine. Command lines are inspected locally for profile attribution and are not written to CSV. Grouping fixtures test recursion, PID reuse, orphan retention and exclusion of another profile.

**Primary metric:** sum of **private working set**, bytes from `Win32_PerfRawData_PerfProc_Process.WorkingSetPrivate`, converted to MiB (1,048,576 bytes). This is private resident memory. Also record sum of `Get-Process.PrivateMemorySize64` (private committed bytes, which may include nonresident pages) and `WorkingSet64` (total resident bytes; shared pages may be counted multiple times). Do not call private commit physical RAM, or compare it to Task Manager's different columns. Shared browser pages mean even this primary metric does not capture every system-wide marginal memory cost.

Each sample emits per-process IDs/creation times/name and totals. Samples raced by process exit or unavailable counters are marked incomplete and excluded from summaries. Inspect totals and per-process CSV, report mean/min/max of complete samples plus process count, and retain raw CSV. CIM/process/counter queries are sequential, not an atomic OS snapshot; allow the system to settle and repeat suspicious results. After Quit, totals include surviving known children but `HostPresent=false` and samples are excluded from complete steady-state summaries; zero members therefore do not count as a successful memory baseline. A replacement process at the old host PID is excluded by creation identity. missing attribution cannot prove no orphans exist. Independently compare Task Manager Process Explorer's tree/command lines, especially after unload/Quit, and record any orphan not attributed by the script. Do not share profile contents.

Use the same output directory across invocations (timestamped CSV avoids overwrites). Use a longer continuous run spanning manual Free RAM or Quit, so remembered identities capture descendants that later reparent:

```powershell
.\scripts\Measure-GPTrayMemory.ps1 -RootProcessId 1234 -Scenario '04-discard-transition' -Samples 90
# During capture, use Free RAM; inspect the per-sample fall/rise and surviving PIDs.
```

For each version, start a fresh GPTray process. Record timings relative to each action: capture 30 one-second samples after a 30-second settling period; separately capture transition latency if desired. For hidden-idle default tests, add a >120-second capture. Test logged-out and logged-in conditions separately. Keep network, page/conversation, DPI, runtime version, power mode and background workload consistent. Run at least three comparable repetitions; do not combine first-load warmup with steady state.

| Scenario | 0.1 measured MiB | 0.2 measured MiB | Processes / conditions |
|---|---|---|---|
| Started, never opened | Pending | Pending | Fresh host |
| Quick open, settled | Pending | Pending | Same page/auth state |
| Quick hidden, warm | Pending | Pending | Record seconds hidden |
| After manual discard | N/A | Pending | Also record surviving helpers and reload delay |
| Full open | N/A | Pending | Same page/content |
| After 20 mode switches | N/A | Pending | Check process count/trend, then 30 settled samples |
| After 20 discard/reopen cycles, then Quit | N/A | Pending | Include 30s post-Quit observation |

Earlier ~139.9 MB open / ~125 MB hidden observations used incompletely validated grouping and an unspecified memory column. Retain as historical observations, not directly comparable benchmarks. Report observed tradeoffs (warm RAM vs reopen latency; discarded draft vs smaller footprint), with no invented improvement target.
