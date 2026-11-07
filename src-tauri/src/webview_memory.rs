//! UI-thread-only typed WebView2 adapter. No COM object escapes this call.
use crate::memory::{Outcome, Target};
use tauri::webview::PlatformWebview;
use webview2_com::Microsoft::Web::WebView2::Win32::{
    ICoreWebView2_19, COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_LOW,
    COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_NORMAL,
};
use windows::core::Interface;

pub fn apply(view: &PlatformWebview, target: Target) -> Outcome {
    let result = (|| -> windows::core::Result<()> {
        // SAFETY: Tauri invokes with_webview on the owning UI/COM thread. The
        // controller owns a live CoreWebView2; typed QueryInterface checks _19.
        let core = unsafe { view.controller().CoreWebView2()? };
        let memory = core.cast::<ICoreWebView2_19>()?;
        let level = match target {
            Target::Normal => COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_NORMAL,
            Target::Low => COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_LOW,
        };
        // SAFETY: live typed COM interface, valid enum and stack output pointer.
        unsafe {
            memory.SetMemoryUsageTargetLevel(level)?;
            let mut actual = COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_NORMAL;
            memory.MemoryUsageTargetLevel(&mut actual)?;
            if actual != level {
                return Err(windows::core::Error::from_hresult(windows::core::HRESULT(
                    0x80004005u32 as i32,
                )));
            }
        }
        Ok(())
    })();
    match result {
        Ok(()) => Outcome::Applied,
        Err(e) if e.code().0 == 0x80004002u32 as i32 => Outcome::Unsupported, // E_NOINTERFACE: old Runtime
        Err(e) => Outcome::Failed(e.code().0),
    }
}
