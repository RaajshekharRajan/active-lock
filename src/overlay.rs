use winit::window::{Fullscreen, Window, WindowAttributes, WindowLevel};

use crate::monitors::MonitorInfo;

pub fn window_attributes(monitor: &MonitorInfo) -> WindowAttributes {
    WindowAttributes::default()
        .with_title("")
        .with_decorations(false)
        .with_fullscreen(Some(Fullscreen::Borderless(Some(monitor.handle.clone()))))
        .with_window_level(WindowLevel::AlwaysOnTop)
        .with_resizable(false)
}

#[cfg(target_os = "macos")]
pub fn augment_window(window: &Window) {
    use objc2_app_kit::{NSView, NSWindowCollectionBehavior};
    use winit::raw_window_handle::HasWindowHandle;

    let Ok(handle) = window.window_handle() else {
        return;
    };
    let raw = handle.as_raw();
    let winit::raw_window_handle::RawWindowHandle::AppKit(appkit) = raw else {
        return;
    };

    unsafe {
        let ns_view = appkit.ns_view.as_ptr() as *const NSView;
        let ns_view = &*ns_view;
        if let Some(ns_window) = ns_view.window() {
            // NSScreenSaverWindowLevel (1000) + 1
            ns_window.setLevel(1001);
            ns_window.setCollectionBehavior(
                NSWindowCollectionBehavior::CanJoinAllSpaces
                    | NSWindowCollectionBehavior::FullScreenAuxiliary,
            );
        }
    }
}

#[cfg(target_os = "windows")]
pub fn augment_window(window: &Window) {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::*;
    use winit::raw_window_handle::HasWindowHandle;

    let Ok(handle) = window.window_handle() else {
        return;
    };
    let raw = handle.as_raw();
    let winit::raw_window_handle::RawWindowHandle::Win32(win32) = raw else {
        return;
    };

    unsafe {
        let hwnd = HWND(win32.hwnd.get() as *mut _);
        let _ = SetWindowPos(hwnd, HWND_TOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE);
        let ex = GetWindowLongW(hwnd, GWL_EXSTYLE);
        SetWindowLongW(hwnd, GWL_EXSTYLE, ex | WS_EX_TOOLWINDOW.0 as i32);
    }
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
pub fn augment_window(_window: &Window) {}
