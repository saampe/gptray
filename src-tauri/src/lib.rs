mod geometry;
mod lifecycle;

use geometry::{Bounds, WorkArea};
use lifecycle::{Deadline, Lifecycle, Mode};
use std::{
    path::PathBuf,
    sync::{mpsc, Mutex},
    time::{Duration, Instant},
};
use tauri::{
    image::Image,
    menu::{
        CheckMenuItemBuilder, MenuBuilder, MenuItemBuilder, PredefinedMenuItem, SubmenuBuilder,
    },
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    webview::NewWindowResponse,
    window::Color,
    AppHandle, LogicalSize, Manager, PhysicalPosition, RunEvent, Url, WebviewUrl, WebviewWindow,
    WebviewWindowBuilder, WindowEvent,
};
use tauri_plugin_autostart::{MacosLauncher, ManagerExt};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

const PANEL_LABEL: &str = "panel";
const TRAY_ICON_BYTES: &[u8] = include_bytes!("../icons/tray.png");
const CHATGPT_URL: &str = "https://chatgpt.com";

struct Inner {
    policy: Lifecycle,
    bounds: Bounds,
    pending_show: Option<Mode>,
    timer: Option<mpsc::Sender<TimerMessage>>,
}
struct Controller {
    inner: Mutex<Inner>,
    started: Instant,
    settings: PathBuf,
}
enum TimerMessage {
    Update(Option<Deadline>),
    Stop,
}
#[derive(Clone, Copy)]
enum Action {
    QuickToggle,
    Open(Mode),
    FreeRam,
    AutoFree(bool),
    TimedFree(Deadline),
    Quit,
}

fn now(app: &AppHandle) -> u64 {
    app.state::<Controller>().started.elapsed().as_millis() as u64
}

// All creation, switching and release requests converge on the event-loop thread.
fn queue(app: &AppHandle, action: Action) {
    let handle = app.clone();
    if let Err(error) = app.run_on_main_thread(move || {
        if let Err(error) = perform(&handle, action) {
            eprintln!("GPTray action failed: {error}");
        }
    }) {
        eprintln!("GPTray dispatch failed: {error}");
    }
}

// No timer/thread before opt-in. One cancellable worker, no polling, no page access.
fn start_timer(app: &AppHandle) -> mpsc::Sender<TimerMessage> {
    let (tx, rx) = mpsc::channel();
    let handle = app.clone();
    std::thread::spawn(move || {
        let mut deadline: Option<Deadline> = None;
        loop {
            let message = match deadline {
                None => rx.recv().map_err(|_| mpsc::RecvTimeoutError::Disconnected),
                Some(d) => {
                    rx.recv_timeout(Duration::from_millis(d.at_ms.saturating_sub(now(&handle))))
                }
            };
            match message {
                Ok(TimerMessage::Update(d)) => deadline = d,
                Ok(TimerMessage::Stop) | Err(mpsc::RecvTimeoutError::Disconnected) => break,
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    if let Some(d) = deadline.take() {
                        queue(&handle, Action::TimedFree(d));
                    }
                }
            }
        }
    });
    tx
}
fn sync_timer(app: &AppHandle) {
    let controller = app.state::<Controller>();
    let state = controller.inner.lock().unwrap();
    if let Some(tx) = &state.timer {
        let _ = tx.send(TimerMessage::Update(state.policy.deadline()));
    }
}
fn save_bounds(app: &AppHandle) {
    let controller = app.state::<Controller>();
    let bounds = controller.inner.lock().unwrap().bounds;
    let result = (|| -> std::io::Result<()> {
        if let Some(parent) = controller.settings.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let temp = controller.settings.with_extension("tmp");
        std::fs::write(&temp, bounds.encode())?;
        std::fs::rename(temp, &controller.settings)
    })();
    if let Err(error) = result {
        eprintln!("GPTray could not save window bounds: {error}");
    }
}
fn capture_bounds(app: &AppHandle, window: &WebviewWindow) {
    let controller = app.state::<Controller>();
    let mut state = controller.inner.lock().unwrap();
    if state.policy.mode != Mode::Full
        || state.policy.transitioning
        || state.policy.releasing
        || window.is_minimized().unwrap_or(true)
    {
        return;
    }
    let Ok(maximized) = window.is_maximized() else {
        return;
    };
    state.bounds.maximized = maximized;
    if maximized {
        return;
    } // Keep the last normal restore rectangle.
    if let (Ok(position), Ok(size), Ok(scale)) = (
        window.outer_position(),
        window.inner_size(),
        window.scale_factor(),
    ) {
        let logical = size.to_logical::<f64>(scale);
        if logical.width >= 200. && logical.height >= 200. {
            state.bounds = Bounds {
                x: position.x,
                y: position.y,
                width: logical.width,
                height: logical.height,
                maximized: false,
            };
        }
    }
}
fn hide(app: &AppHandle, window: &WebviewWindow) -> tauri::Result<()> {
    capture_bounds(app, window);
    window.hide()?;
    app.state::<Controller>()
        .inner
        .lock()
        .unwrap()
        .policy
        .hide(now(app));
    save_bounds(app);
    sync_timer(app);
    Ok(())
}
fn fit_bounds(window: &WebviewWindow, bounds: Bounds) -> Bounds {
    let areas = window
        .available_monitors()
        .unwrap_or_default()
        .iter()
        .map(|m| {
            let a = m.work_area();
            WorkArea {
                x: a.position.x,
                y: a.position.y,
                width: a.size.width.saturating_sub((16. * m.scale_factor()) as u32),
                height: a
                    .size
                    .height
                    .saturating_sub((48. * m.scale_factor()) as u32),
                scale: m.scale_factor(),
            }
        })
        .collect::<Vec<_>>();
    bounds.fit(&areas)
}
fn native_mode(window: &WebviewWindow, mode: Mode, bounds: Bounds) -> tauri::Result<()> {
    let full = mode == Mode::Full;
    window.unminimize()?;
    window.unmaximize()?;
    window.set_decorations(full)?;
    window.set_always_on_top(!full)?;
    window.set_skip_taskbar(!full)?;
    window.set_resizable(full)?;
    window.set_maximizable(full)?;
    window.set_minimizable(full)?;
    if full {
        let bounds = fit_bounds(window, bounds);
        window.set_size(LogicalSize::new(bounds.width, bounds.height))?;
        window.set_position(PhysicalPosition::new(bounds.x, bounds.y))?;
        if bounds.maximized {
            window.maximize()?;
        }
    } else {
        window.set_size(LogicalSize::new(400., 600.))?;
        if let (Ok(Some(monitor)), Ok(size)) = (window.primary_monitor(), window.outer_size()) {
            let a = monitor.work_area();
            window.set_position(PhysicalPosition::new(
                a.position.x + (a.size.width as i32 - size.width as i32 - 12).max(0),
                a.position.y + (a.size.height as i32 - size.height as i32 - 12).max(0),
            ))?;
        }
    }
    Ok(())
}
// WebView2 may post LostFocus after a mode change has already refocused the page.
// Check the foreground top-level HWND, including focus inside WebView2 child HWNDs.
#[cfg(target_os = "windows")]
fn native_foreground(window: &WebviewWindow) -> Option<bool> {
    use std::ffi::c_void;
    #[link(name = "user32")]
    unsafe extern "system" {
        fn GetForegroundWindow() -> *mut c_void;
        fn GetAncestor(window: *mut c_void, flags: u32) -> *mut c_void;
    }
    let own = window.hwnd().ok()?.0;
    // SAFETY: these read-only User32 APIs accept HWND values; null/failed queries
    // are handled by keeping the panel open. No raw pointer is dereferenced.
    unsafe {
        let foreground = GetForegroundWindow();
        if foreground.is_null() {
            return None;
        }
        let root = GetAncestor(foreground, 2); // GA_ROOT
        let own_root = GetAncestor(own, 2);
        if root.is_null() || own_root.is_null() {
            None
        } else {
            Some(root == own_root)
        }
    }
}
#[cfg(not(target_os = "windows"))]
fn native_foreground(window: &WebviewWindow) -> Option<bool> {
    window.is_focused().ok()
}

fn get_or_create_panel(app: &AppHandle) -> tauri::Result<WebviewWindow> {
    if let Some(window) = app.get_webview_window(PANEL_LABEL) {
        return Ok(window);
    }
    // This is Tauri's existing default directory. Keep the identifier and profile unchanged.
    let profile = app.path().app_local_data_dir()?;
    let window = WebviewWindowBuilder::new(
        app,
        PANEL_LABEL,
        WebviewUrl::External(Url::parse(CHATGPT_URL).expect("constant HTTPS URL")),
    )
    .title("GPTray — ChatGPT")
    .inner_size(400., 600.)
    .decorations(false)
    .always_on_top(true)
    .skip_taskbar(true)
    .resizable(false)
    .shadow(true)
    .visible(false)
    .focused(false)
    .background_color(Color(13, 13, 13, 255))
    .data_directory(profile)
    // Do not let window.open create an untracked second ChatGPT/browser instance.
    .on_new_window(|_, _| NewWindowResponse::Deny)
    .build()?;
    let handle = app.clone();
    window.on_window_event(move |event| match event {
        WindowEvent::CloseRequested { api, .. } => {
            api.prevent_close();
            if let Some(w) = handle.get_webview_window(PANEL_LABEL) {
                let _ = hide(&handle, &w);
            }
        }
        WindowEvent::Focused(false) => {
            if let Some(w) = handle.get_webview_window(PANEL_LABEL) {
                let should_hide = handle
                    .state::<Controller>()
                    .inner
                    .lock()
                    .unwrap()
                    .policy
                    .blur_should_hide(native_foreground(&w));
                if should_hide {
                    let _ = hide(&handle, &w);
                }
            }
        }
        WindowEvent::Resized(_) | WindowEvent::Moved(_) => {
            if let Some(w) = handle.get_webview_window(PANEL_LABEL) {
                capture_bounds(&handle, &w);
            }
        }
        WindowEvent::Destroyed => {
            let pending = {
                let controller = handle.state::<Controller>();
                let mut state = controller.inner.lock().unwrap();
                state.policy.releasing = false;
                state.pending_show.take()
            };
            // Tauri has removed its label; defer through the event queue so Wry's
            // native destruction callback also returns before constructing a new WebView.
            if let Some(mode) = pending {
                let app = handle.clone();
                std::thread::spawn(move || queue(&app, Action::Open(mode)));
            }
        }
        _ => {}
    });
    Ok(window)
}
fn open(app: &AppHandle, mode: Mode) -> tauri::Result<()> {
    {
        let controller = app.state::<Controller>();
        let mut state = controller.inner.lock().unwrap();
        if state.policy.releasing {
            state.pending_show = Some(mode);
            return Ok(());
        }
    }
    let window = get_or_create_panel(app)?;
    capture_bounds(app, &window);
    let (old_mode, bounds) = {
        let controller = app.state::<Controller>();
        let mut state = controller.inner.lock().unwrap();
        let old = state.policy.mode;
        state.policy.transitioning = true;
        state.policy.show(mode); // Invalidates a queued timeout before any native actions.
        (old, state.bounds)
    };
    sync_timer(app);
    let result = native_mode(&window, mode, bounds).and_then(|_| {
        window.show()?;
        window.set_focus()
    });
    if result.is_err() {
        // Best-effort restore; never navigate/recreate to recover a style failure.
        let _ = native_mode(&window, old_mode, bounds);
    }
    {
        let controller = app.state::<Controller>();
        let mut state = controller.inner.lock().unwrap();
        if result.is_err() {
            state.policy.mode = old_mode;
        }
        state.policy.transitioning = false;
    }
    save_bounds(app);
    result
}
fn release(app: &AppHandle) -> tauri::Result<()> {
    let Some(window) = app.get_webview_window(PANEL_LABEL) else {
        return Ok(());
    };
    capture_bounds(app, &window);
    {
        let controller = app.state::<Controller>();
        let mut state = controller.inner.lock().unwrap();
        if state.policy.releasing {
            return Ok(());
        }
        let mode = state.policy.mode;
        state.policy.show(mode); // Invalidate any old idle ticket.
        state.policy.releasing = true;
    }
    save_bounds(app);
    sync_timer(app);
    if let Err(error) = window.destroy() {
        app.state::<Controller>()
            .inner
            .lock()
            .unwrap()
            .policy
            .releasing = false;
        return Err(error);
    }
    Ok(())
}
fn perform(app: &AppHandle, action: Action) -> tauri::Result<()> {
    match action {
        Action::QuickToggle => {
            if let Some(w) = app.get_webview_window(PANEL_LABEL) {
                let should_hide = app
                    .state::<Controller>()
                    .inner
                    .lock()
                    .unwrap()
                    .policy
                    .toggle_should_hide(w.is_visible()?);
                if should_hide {
                    return hide(app, &w);
                }
            }
            open(app, Mode::Quick)
        }
        Action::Open(mode) => open(app, mode),
        Action::FreeRam => release(app),
        Action::AutoFree(enabled) => {
            {
                let controller = app.state::<Controller>();
                let mut state = controller.inner.lock().unwrap();
                if enabled && state.timer.is_none() {
                    state.timer = Some(start_timer(app));
                }
                state.policy.set_auto_free(enabled, now(app));
            }
            sync_timer(app);
            Ok(())
        }
        Action::TimedFree(ticket) => {
            let permitted = app
                .state::<Controller>()
                .inner
                .lock()
                .unwrap()
                .policy
                .may_auto_release(ticket, now(app));
            if permitted {
                if let Some(w) = app.get_webview_window(PANEL_LABEL) {
                    // Fail closed: never auto-destroy a visible/minimized window, even if policy is stale.
                    if !w.is_visible()? && !w.is_minimized()? {
                        release(app)?;
                    }
                }
            }
            Ok(())
        }
        Action::Quit => {
            if let Some(w) = app.get_webview_window(PANEL_LABEL) {
                capture_bounds(app, &w);
            }
            save_bounds(app);
            if let Some(tx) = &app.state::<Controller>().inner.lock().unwrap().timer {
                let _ = tx.send(TimerMessage::Stop);
            }
            app.exit(0);
            Ok(())
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            None,
        ))
        .setup(|app| {
            let settings = app
                .path()
                .app_local_data_dir()?
                .join("window-bounds-v1.txt");
            let bounds = std::fs::read_to_string(&settings)
                .ok()
                .and_then(|s| Bounds::decode(&s))
                .unwrap_or_default();
            app.manage(Controller {
                inner: Mutex::new(Inner {
                    policy: Lifecycle::default(),
                    bounds,
                    pending_show: None,
                    timer: None,
                }),
                started: Instant::now(),
                settings,
            });
            let shortcut = Shortcut::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::Space);
            app.handle().plugin(
                tauri_plugin_global_shortcut::Builder::new()
                    .with_handler(move |app, hit, event| {
                        if hit.id() == shortcut.id() && event.state() == ShortcutState::Pressed {
                            queue(app, Action::QuickToggle);
                        }
                    })
                    .build(),
            )?;
            app.global_shortcut().register(shortcut)?;

            let toggle = MenuItemBuilder::with_id("toggle", "Toggle Quick Panel").build(app)?;
            let full = MenuItemBuilder::with_id("full", "Open Full Window").build(app)?;
            let quick = MenuItemBuilder::with_id("quick", "Return to Quick Panel").build(app)?;
            let warning = MenuItemBuilder::new("Stops replies and loses unsent text")
                .enabled(false)
                .build(app)?;
            let release =
                MenuItemBuilder::with_id("release", "Discard page state and Free RAM now")
                    .build(app)?;
            let auto =
                CheckMenuItemBuilder::with_id("auto", "Auto-discard after 2 min hidden (this run)")
                    .checked(false)
                    .build(app)?;
            let ram = SubmenuBuilder::new(app, "Free RAM — discards page state")
                .items(&[&warning, &release, &auto])
                .build()?;
            let autostart = CheckMenuItemBuilder::with_id("autostart", "Launch at Login")
                .checked(app.autolaunch().is_enabled().unwrap_or(false))
                .build(app)?;
            let quit = MenuItemBuilder::with_id("quit", "Quit").build(app)?;
            let menu = MenuBuilder::new(app)
                .items(&[
                    &toggle,
                    &full,
                    &quick,
                    &ram,
                    &autostart,
                    &PredefinedMenuItem::separator(app)?,
                    &quit,
                ])
                .build()?;
            TrayIconBuilder::new()
                .icon(Image::from_bytes(TRAY_ICON_BYTES).expect("valid tray icon"))
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(move |app, event| match event.id().as_ref() {
                    "toggle" => queue(app, Action::QuickToggle),
                    "full" => queue(app, Action::Open(Mode::Full)),
                    "quick" => queue(app, Action::Open(Mode::Quick)),
                    "release" => queue(app, Action::FreeRam),
                    "auto" => {
                        if let Ok(checked) = auto.is_checked() {
                            queue(app, Action::AutoFree(checked));
                        }
                    }
                    "autostart" => {
                        let mgr = app.autolaunch();
                        let enabled = mgr.is_enabled().unwrap_or(false);
                        let result = if enabled { mgr.disable() } else { mgr.enable() };
                        if result.is_ok() {
                            let _ = autostart.set_checked(!enabled);
                        }
                    }
                    "quit" => queue(app, Action::Quit),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        queue(tray.app_handle(), Action::QuickToggle);
                    }
                })
                .build(app)?;
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building GPTray")
        .run(|_, event| {
            // Destroying the last WebView frees RAM; the tray/shortcut must stay alive.
            if let RunEvent::ExitRequested {
                code: None, api, ..
            } = event
            {
                api.prevent_exit();
            }
        });
}
