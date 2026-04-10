use objc2::rc::Retained;
use objc2::runtime::{AnyClass, AnyObject};
use objc2::{msg_send, MainThreadMarker};
use objc2_app_kit::{NSApplication, NSApplicationPresentationOptions};

/// Wrapper so the monitor handle can cross thread boundaries (we only
/// ever access it on the main thread in practice).
struct SendMonitor(Retained<AnyObject>);
unsafe impl Send for SendMonitor {}

pub struct MacosLock {
    original_options: Option<NSApplicationPresentationOptions>,
    event_monitor: Option<SendMonitor>,
}

impl MacosLock {
    pub fn new() -> Self {
        Self {
            original_options: None,
            event_monitor: None,
        }
    }
}

/// Installs an `NSEvent` local monitor that intercepts key-down events
/// with the Cmd modifier and swallows Cmd+Q, Cmd+W, Cmd+H, and Cmd+M
/// before AppKit can act on them.
fn install_cmd_key_blocker() -> Option<SendMonitor> {
    unsafe {
        let block = block2::RcBlock::new(|event: *mut AnyObject| -> *mut AnyObject {
            if event.is_null() {
                return event;
            }
            let dominated = {
                let flags: usize = msg_send![&*event, modifierFlags];
                // NSEventModifierFlagCommand = 1 << 20
                if flags & (1 << 20) != 0 {
                    let chars: *mut AnyObject =
                        msg_send![&*event, charactersIgnoringModifiers];
                    if !chars.is_null() {
                        let utf8: *const std::ffi::c_char = msg_send![&*chars, UTF8String];
                        if !utf8.is_null() {
                            let ch = *utf8 as u8;
                            matches!(ch, b'q' | b'w' | b'h' | b'm')
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                } else {
                    false
                }
            };
            if dominated {
                std::ptr::null_mut()
            } else {
                event
            }
        });

        let cls = AnyClass::get(c"NSEvent")?;
        // NSEventMaskKeyDown = 1 << 10
        let mask: u64 = 1 << 10;
        let monitor: Option<Retained<AnyObject>> = msg_send![
            cls,
            addLocalMonitorForEventsMatchingMask: mask,
            handler: &*block,
        ];
        monitor.map(SendMonitor)
    }
}

fn remove_cmd_key_blocker(monitor: &SendMonitor) {
    unsafe {
        if let Some(cls) = AnyClass::get(c"NSEvent") {
            let _: () = msg_send![cls, removeMonitor: &*monitor.0];
        }
    }
}

impl super::PlatformLock for MacosLock {
    fn engage(&mut self) -> Result<(), String> {
        let mtm =
            MainThreadMarker::new().ok_or_else(|| "not on main thread".to_string())?;
        let app = NSApplication::sharedApplication(mtm);

        self.original_options = Some(app.presentationOptions());

        let options = NSApplicationPresentationOptions::HideDock
            | NSApplicationPresentationOptions::HideMenuBar
            | NSApplicationPresentationOptions::DisableProcessSwitching
            | NSApplicationPresentationOptions::DisableForceQuit
            | NSApplicationPresentationOptions::DisableAppleMenu
            | NSApplicationPresentationOptions::DisableSessionTermination
            | NSApplicationPresentationOptions::DisableHideApplication;

        app.setPresentationOptions(options);
        self.event_monitor = install_cmd_key_blocker();

        log::info!("macOS lockdown engaged");
        Ok(())
    }

    fn disengage(&mut self) -> Result<(), String> {
        if let Some(monitor) = self.event_monitor.take() {
            remove_cmd_key_blocker(&monitor);
        }

        let Some(mtm) = MainThreadMarker::new() else {
            log::warn!("cannot disengage macOS lock off main thread; use --reset");
            return Ok(());
        };
        let app = NSApplication::sharedApplication(mtm);

        let restore = self
            .original_options
            .take()
            .unwrap_or(NSApplicationPresentationOptions::empty());
        app.setPresentationOptions(restore);
        log::info!("macOS lockdown disengaged");
        Ok(())
    }
}

impl Drop for MacosLock {
    fn drop(&mut self) {
        let _ = <Self as super::PlatformLock>::disengage(self);
    }
}
