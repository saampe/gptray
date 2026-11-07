# GPTray 0.3

A lightweight Windows tray client for the real [ChatGPT website](https://chatgpt.com), using Tauri v2 / Rust / WebView2. Your account, Plus entitlement, history and memory come from the website; there is no separate AI service or API chatbot.

## Use

- `Ctrl+Shift+Enter` opens/restores/focuses Full Window. Both shortcuts register independently; a conflict is reported in the tray menu and never prevents startup.
- `Ctrl+Shift+Space`, tray left-click, or **Toggle Quick Panel** opens/hides the quick panel: 400 × 600 logical pixels, topmost, no taskbar entry, hides on focus loss.
- Tray → **Open Full Window**: standard titlebar, taskbar entry, resizing, maximize/minimize. Focus loss leaves it open. Default content size 1100 × 750; normal bounds and maximized state are saved. Native close hides to tray; **Quit** exits.
- Tray → **Return to Quick Panel** or the hotkey changes the same window back to the panel. Mode switching does not navigate or recreate ChatGPT.
- **Launch at Login** is optional and managed through the existing Windows startup registration.
- **Free RAM — discards page state** → **Discard page state and Free RAM now** destroys the WebView. **This stops replies and loses unsent text.** Next open reloads ChatGPT; profile data is retained.
- **Auto-discard after 2 min hidden (this run)** is optional, explicitly destructive and **off at every startup**. Only use it when losing a hidden draft or stopping a hidden reply is acceptable. Visible windows, including minimized full windows, are excluded. Reopening cancels the timer. Default hiding never automatically discards a page.

**Automatic non-destructive memory hints:** inactive/hidden/minimized WebViews request WebView2 Low, then Normal before active use. Scripts/network continue; no default suspend or page destruction. Older runtimes without the API continue normally. [API support and tradeoffs](docs/WEBVIEW2-MEMORY.md).

Full is constructed at the requested size/chrome before its WebView; controller bounds synchronize before showing. First network/website paint can still take time.

One ChatGPT WebView is created on first use; none is created at startup. Hiding keeps the live page but permits Low-memory eviction; reopening may be slower. Destroying the WebView can release resources, but actual WebView2 process lifetime and RAM savings must be measured on Windows. No performance percentage is promised.

## Test installation and rollback

Use the completed Windows Actions run linked from the Draft PR. Download artifact **gptray-0.3-windows-x64-unsigned-test** (available for fourteen days); unzip the installer and `SHA256SUMS.txt` together. A GitHub login may be required. Before running:

```powershell
$installer = Get-ChildItem . -Recurse -Filter chatgpt-overlay-bar_0.3.0_x64-setup.exe | Select-Object -First 1
Get-FileHash $installer.FullName -Algorithm SHA256
Get-Content .\SHA256SUMS.txt
```

Compare the entire hash and confirm the run's source commit. The installer is unsigned: only proceed if the source and checksum are trusted; signing/SmartScreen reputation is not established by a successful build. Quit the old GPTray, then run the per-user NSIS installer. Keep the same app identifier and do not delete `%LOCALAPPDATA%\com.globalvox.chatgpt-overlay-bar` when comparing versions.

Keep a copy of the previous 0.2 installer/hash before upgrading. The exact baseline is main commit `6cf978e1efdb55d6195571c54fc8a6c437a52a1f` (merged PR #2). For rollback, Quit 0.3, uninstall **without deleting user data**, then install the saved 0.2 build; NSIS may reject an in-place downgrade. Rebuild the old commit if its artifact expired. Back up the profile with GPTray closed before comparison and never run both versions concurrently. This update never clears or migrates the profile.

## Verification

See [dependency/security results](docs/SECURITY-REVIEW.md), [Windows acceptance and RAM measurement](docs/WINDOWS-VALIDATION.md) and [architecture / security](ARCHITECTURE.md). CI verifies policy tests, Windows compilation/packaging and separately attempts a native API/viewport probe with isolated local content. Its actual outcome is recorded in a separate artifact. A build or that probe does **not** verify live ChatGPT login, drafts, rendering quality or measured RAM savings.

The earlier observations (~139.9 MB open, ~125 MB hidden) include WebView2 processes identified at the time, but are not validated process-group baselines. Measure both versions with the same script and conditions before comparing. The observed 450–470 MB activity peaks are also uncontrolled. Earlier README claims of 12–27 MB or other fixed memory costs are not acceptance evidence for this version.

## One-session memory comparison

Start one fresh GPTray without opening it, then run the same tooling for each version:

```powershell
.\scripts\Invoke-GPTrayBenchmark.ps1 -VersionLabel 0.3 -AuthState LoggedOut
```

One guided run captures seven phases, metadata, raw process/total CSV and a version-separated summary. Use `-VersionLabel 0.2` for the saved baseline executable. Full stays visible while using the console; follow the countdown to refocus it. Inspect the opening-transition timeline separately from settled samples. See [Windows validation](docs/WINDOWS-VALIDATION.md) for matched conditions and remaining checks.

## Build

Windows 10/11 x64, WebView2 Runtime, Node.js 22, stable Rust MSVC and Visual Studio C++ Build Tools are required.

```powershell
npm ci
cargo test --locked --manifest-path src-tauri/Cargo.toml --target x86_64-pc-windows-msvc
npm run tauri build -- --target x86_64-pc-windows-msvc --bundles nsis -- --locked
```

Installer output: `src-tauri/target/x86_64-pc-windows-msvc/release/bundle/nsis/`.

Pure logic tests can also run on Linux without a GUI or Tauri system libraries:

```sh
mkdir -p work
rustc --edition=2021 --test src-tauri/src/policy_tests.rs -o work/policy-tests
./work/policy-tests
```

Inspired by [ik190/macos-chatgpt-overlay-bar](https://github.com/ik190/macos-chatgpt-overlay-bar).
