#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;

pub trait PlatformLock: Send {
    fn engage(&mut self) -> Result<(), String>;
    fn disengage(&mut self) -> Result<(), String>;
}

pub fn create_platform_lock() -> Box<dyn PlatformLock> {
    #[cfg(target_os = "macos")]
    {
        Box::new(macos::MacosLock::new())
    }

    #[cfg(target_os = "windows")]
    {
        Box::new(windows::WindowsLock::new())
    }
}
