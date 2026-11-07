//! Automated native API/viewport probe with isolated a local blank document. Not a
//! ChatGPT login, rendering-quality, or RAM benchmark; never uses the user profile.
#[allow(dead_code)]
#[path = "../src/geometry.rs"]
mod geometry;
#[allow(dead_code)]
#[path = "../src/lifecycle.rs"]
mod lifecycle;
#[allow(dead_code)]
#[path = "../src/memory.rs"]
mod memory;
#[path = "../src/native_window.rs"]
mod native_window;
#[cfg(windows)]
#[path = "../src/webview_memory.rs"]
mod webview_memory;
#[allow(dead_code)]
#[path = "../src/window_mode.rs"]
mod window_mode;

#[cfg(windows)]
fn main() {
    use memory::{Outcome, Target};
    use tauri::{RunEvent, Url, WebviewUrl};
    let profile = std::env::temp_dir().join(format!(
        "gptray-native-smoke-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    let profile_for_setup = profile.clone();
    let result = tauri::Builder::default().setup(move |app| {
        let version = tauri::webview_version()?;
        println!("Native smoke WebView2 Runtime: {version}");
        let window = native_window::build(app.handle(), lifecycle::Mode::Full,
            geometry::Bounds { width: 980., height: 640., ..geometry::Bounds::default() }, profile_for_setup,
            WebviewUrl::External(Url::parse("about:blank")?))?;
        if !window.is_decorated()? || !window.is_resizable()? {
            return Err(std::io::Error::other("Initial Full native chrome/resizability missing").into());
        }
        let expected = window.inner_size()?;
        native_window::with_ui_webview(&window, move |view| {
            let mut bounds = windows::Win32::Foundation::RECT::default();
            // SAFETY: owning UI thread, live controller, valid stack output.
            unsafe { view.controller().Bounds(&mut bounds) }.map_err(|e| std::io::Error::other(e.to_string()))?;
            if (bounds.right - bounds.left, bounds.bottom - bounds.top) != (expected.width as i32, expected.height as i32) {
                return Err(std::io::Error::other(format!("Initial Full viewport mismatch: {bounds:?} vs {expected:?}")).into());
            }
            for target in [Target::Normal, Target::Low, Target::Normal] {
                match webview_memory::apply(&view, target) {
                    Outcome::Applied => println!("Native memory target accepted/read back: {target:?}"),
                    other => return Err(std::io::Error::other(format!("Memory target failed: {other:?}")).into()),
                }
            }
            Ok(())
        })?;
        println!("Native initial Full viewport and chrome verified");
        // Native window operations run after with_webview releases its mutex.
        window.set_size(tauri::LogicalSize::new(850., 620.))?;
        let resized = window.inner_size()?;
        native_window::with_ui_webview(&window, move |view| {
            native_window::sync_viewport(&view, resized).map_err(|e| std::io::Error::other(e.to_string()))?;
            let mut bounds = windows::Win32::Foundation::RECT::default();
            // SAFETY: same valid controller/output contract as above.
            unsafe { view.controller().Bounds(&mut bounds) }.map_err(|e| std::io::Error::other(e.to_string()))?;
            if (bounds.right, bounds.bottom) != (resized.width as i32, resized.height as i32) {
                return Err(std::io::Error::other("Synchronous resized viewport mismatch").into());
            }
            Ok(())
        })?;
        println!("Native resized viewport verified");
        native_window::show(&window)?;
        if !window.is_visible()? { return Err(std::io::Error::other("Show did not make window visible").into()); }
        window.hide()?;
        if window.is_visible()? { return Err(std::io::Error::other("Hide did not hide window").into()); }
        println!("NATIVE_SMOKE_PASS: initial Full viewport, chrome, Low/Normal readback, resized viewport, show/hide");
        app.handle().exit(0);
        Ok(())
    }).build(tauri::generate_context!());
    let exit_code = match result {
        Ok(app) => {
            let handle = app.handle().clone();
            let (cancel, wait) = std::sync::mpsc::channel();
            let watchdog = std::thread::spawn(move || {
                if matches!(
                    wait.recv_timeout(std::time::Duration::from_secs(30)),
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout)
                ) {
                    eprintln!("NATIVE_SMOKE_FAIL: watchdog");
                    handle.exit(2);
                }
            });
            let code = app.run_return(|_, event| {
                if let RunEvent::ExitRequested {
                    code: None, api, ..
                } = event
                {
                    api.prevent_exit();
                }
            });
            let _ = cancel.send(());
            let _ = watchdog.join();
            code
        }
        Err(error) => {
            eprintln!("NATIVE_SMOKE_FAIL: initialization: {error}");
            1
        }
    };
    // Only our unique disposable test directory; never app_local_data_dir.
    // WebView2 helpers can briefly retain files after native teardown.
    for _ in 0..20 {
        if !profile.exists() || std::fs::remove_dir_all(&profile).is_ok() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    if profile.exists() {
        eprintln!("Native smoke temporary profile cleanup deferred to OS temp cleanup");
    }
    std::process::exit(exit_code);
}
#[cfg(not(windows))]
fn main() {
    eprintln!("Native probe requires Windows");
}
