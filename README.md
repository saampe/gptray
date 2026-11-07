# GPTray 0.2

A lightweight Windows tray client for the real [ChatGPT website](https://chatgpt.com), using Tauri v2 / Rust / WebView2. Your account, Plus entitlement, history and memory come from the website; there is no separate AI service or API chatbot.

## Use

- `Ctrl+Shift+Space`, tray left-click, or **Toggle Quick Panel** opens/hides the quick panel: 400 × 600 logical pixels, topmost, no taskbar entry, hides on focus loss.
- Tray → **Open Full Window**: standard titlebar, taskbar entry, resizing, maximize/minimize. Focus loss leaves it open. Default content size 1100 × 750; normal bounds and maximized state are saved. Native close hides to tray; **Quit** exits.
- Tray → **Return to Quick Panel** or the hotkey changes the same window back to the panel. Mode switching does not navigate or recreate ChatGPT.
- **Launch at Login** is optional and managed through the existing Windows startup registration.
- **Free RAM — discards page state** → **Discard page state and Free RAM now** destroys the WebView. **This stops replies and loses unsent text.** Next open reloads ChatGPT; profile data is retained.
- **Auto-discard after 2 min hidden (this run)** is optional, explicitly destructive and **off at every startup**. Only use it when losing a hidden draft or stopping a hidden reply is acceptable. Visible windows, including minimized full windows, are excluded. Reopening cancels the timer. Default hiding never automatically discards a page.

One ChatGPT WebView is created on first use; none is created at startup. Hidden by default means warm, fast reopen. Destroying the WebView can release resources, but actual WebView2 process lifetime and RAM savings must be measured on Windows. No performance percentage is promised.

## Test installation and rollback

Use the completed Windows Actions run linked from the Draft PR. Download artifact **gptray-0.2-windows-x64-unsigned-test** (available for seven days); unzip the installer and `SHA256SUMS.txt` together. A GitHub login may be required. Before running:

```powershell
Get-FileHash .\chatgpt-overlay-bar_0.2.0_x64-setup.exe -Algorithm SHA256
Get-Content .\SHA256SUMS.txt
```

Compare the entire hash and confirm the run's source commit. The installer is unsigned: only proceed if the source and checksum are trusted; signing/SmartScreen reputation is not established by a successful build. Quit the old GPTray, then run the per-user NSIS installer. Keep the same app identifier and do not delete `%LOCALAPPDATA%\com.globalvox.chatgpt-overlay-bar` when comparing versions.

Keep a copy of the previous 0.1 installer plus hash before upgrading. PR [#1](https://github.com/saampe/gptray/pull/1), branch `ci/windows-build-test`, commit `f2c5903e2c1d4bfcb5505e41b39adcc1a8267159` remain the old source baseline. For rollback, Quit 0.2, uninstall it **without deleting user data**, and install the saved 0.1 build (the installer may reject an in-place downgrade). Rebuild the old commit if its short-lived artifact has expired. Profile compatibility across WebView2 runtime changes is not guaranteed; back up the profile with GPTray closed before testing. Do not run both versions concurrently.

## Verification

See [Windows acceptance and RAM measurement](docs/WINDOWS-VALIDATION.md) and [architecture / security](ARCHITECTURE.md). A passing CI build verifies compilation, automated policy tests and packaging, **not** live Windows focus, login, draft preservation or measured RAM use.

The earlier observations (~139.9 MB open, ~125 MB hidden) include WebView2 processes identified at the time, but are not validated process-group baselines. Measure both versions with the same script and conditions before comparing. Earlier README claims of 12–27 MB or other fixed memory costs are not acceptance evidence for this version.

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
