//! Shared production/probe factory: selected mode is applied before WebView creation.
use crate::{
    geometry::{Bounds, WorkArea},
    lifecycle::Mode,
    window_mode::WindowMode,
};
use std::path::PathBuf;
use tauri::{
    webview::NewWindowResponse, window::Color, AppHandle, PhysicalPosition, WebviewUrl,
    WebviewWindow, WebviewWindowBuilder,
};

pub fn fitted(app: &AppHandle, bounds: Bounds) -> Bounds {
    let areas = app
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
pub fn build(
    app: &AppHandle,
    mode: Mode,
    bounds: Bounds,
    profile: PathBuf,
    url: WebviewUrl,
) -> tauri::Result<WebviewWindow> {
    let bounds = fitted(app, bounds);
    let spec = WindowMode::new(mode, bounds);
    let window = WebviewWindowBuilder::new(app, "panel", url)
        .title("GPTray — ChatGPT")
        .inner_size(spec.width, spec.height)
        .decorations(spec.full)
        .always_on_top(!spec.full)
        .skip_taskbar(!spec.full)
        .resizable(spec.full)
        .maximizable(spec.full)
        .minimizable(spec.full)
        .maximized(spec.maximized)
        .shadow(true)
        .visible(false)
        .focused(false)
        .background_color(Color(13, 13, 13, 255))
        .data_directory(profile)
        .on_new_window(|_, _| NewWindowResponse::Deny)
        .build()?;
    if spec.full {
        window.set_position(PhysicalPosition::new(bounds.x, bounds.y))?;
    }
    Ok(window)
}

/// Synchronize WebView2's physical viewport before show, rather than waiting for
/// Wry's asynchronous WM_SIZE notification. It is the sole child in this app.
#[cfg(windows)]
pub fn sync_viewport(
    view: &tauri::webview::PlatformWebview,
    size: tauri::PhysicalSize<u32>,
) -> windows::core::Result<()> {
    let rect = windows::Win32::Foundation::RECT {
        left: 0,
        top: 0,
        right: size.width as i32,
        bottom: size.height as i32,
    };
    // SAFETY: called by with_webview on its owning UI thread with a live
    // controller and a valid client-area rectangle, never a website DOM change.
    unsafe { view.controller().SetBounds(rect) }
}

pub fn show(window: &WebviewWindow) -> tauri::Result<()> {
    #[cfg(windows)]
    {
        let window_clone = window.clone();
        // Showing inside the callback orders viewport synchronization before
        // visibility even if Tauri dispatches with_webview asynchronously.
        window.with_webview(move |view| {
            if let Ok(size) = window_clone.inner_size() {
                if let Err(e) = sync_viewport(&view, size) {
                    eprintln!("GPTray viewport sync failed: {}", e.code());
                }
            }
            if let Err(e) = window_clone.show().and_then(|_| window_clone.set_focus()) {
                eprintln!("GPTray show failed: {e}");
            }
        })
    }
    #[cfg(not(windows))]
    {
        window.show()?;
        window.set_focus()
    }
}
