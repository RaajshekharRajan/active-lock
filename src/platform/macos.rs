use objc2::MainThreadMarker;
use objc2_app_kit::{NSApplication, NSApplicationPresentationOptions};

pub struct MacosLock {
    original_options: Option<NSApplicationPresentationOptions>,
}

impl MacosLock {
    pub fn new() -> Self {
        Self {
            original_options: None,
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
        log::info!("macOS lockdown engaged");
        Ok(())
    }

    fn disengage(&mut self) -> Result<(), String> {
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
