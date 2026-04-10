use std::sync::atomic::{AtomicIsize, Ordering};

use windows::Win32::Foundation::*;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Registry::*;
use windows::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState;
use windows::Win32::UI::WindowsAndMessaging::*;

static HOOK_HANDLE: AtomicIsize = AtomicIsize::new(0);

unsafe extern "system" fn keyboard_hook_proc(
    code: i32,
    w_param: WPARAM,
    l_param: LPARAM,
) -> LRESULT {
    if code >= 0 {
        let kb = unsafe { &*(l_param.0 as *const KBDLLHOOKSTRUCT) };
        let vk = kb.vkCode as u16;
        let alt_down = kb.flags.0 & 0x20 != 0;

        let block = match vk {
            0x5B | 0x5C => true,                                           // VK_LWIN / VK_RWIN
            0x09 if alt_down => true,                                      // Alt+Tab
            0x73 if alt_down => true,                                      // Alt+F4
            0x1B if unsafe { GetAsyncKeyState(0x11_i32) } < 0 => true,    // Ctrl+Esc
            _ => false,
        };

        if block {
            return LRESULT(1);
        }
    }

    // The hhk parameter is ignored for WH_KEYBOARD_LL hooks
    unsafe { CallNextHookEx(HHOOK::default(), code, w_param, l_param) }
}

pub struct WindowsLock {
    hook_installed: bool,
    taskmgr_was_disabled: bool,
}

impl WindowsLock {
    pub fn new() -> Self {
        Self {
            hook_installed: false,
            taskmgr_was_disabled: false,
        }
    }

    fn install_keyboard_hook(&mut self) -> Result<(), String> {
        unsafe {
            let h_module =
                GetModuleHandleW(None).map_err(|e| format!("GetModuleHandleW: {e}"))?;
            let hook = SetWindowsHookExW(
                WH_KEYBOARD_LL,
                Some(keyboard_hook_proc),
                HINSTANCE::from(h_module),
                0,
            )
            .map_err(|e| format!("SetWindowsHookExW: {e}"))?;
            HOOK_HANDLE.store(hook.0 as isize, Ordering::SeqCst);
            self.hook_installed = true;
        }
        log::info!("keyboard hook installed");
        Ok(())
    }

    fn uninstall_keyboard_hook(&mut self) {
        let raw = HOOK_HANDLE.swap(0, Ordering::SeqCst);
        if raw != 0 {
            unsafe {
                let _ = UnhookWindowsHookEx(HHOOK(raw as *mut _));
            }
            log::info!("keyboard hook removed");
        }
        self.hook_installed = false;
    }

    fn disable_task_manager(&mut self) -> Result<(), String> {
        let subkey = windows::core::w!(
            "Software\\Microsoft\\Windows\\CurrentVersion\\Policies\\System"
        );
        let name = windows::core::w!("DisableTaskMgr");

        unsafe {
            // Snapshot current state so we can restore it correctly
            let mut key = HKEY::default();
            if RegOpenKeyExW(HKEY_CURRENT_USER, subkey, 0, KEY_READ, &mut key) == ERROR_SUCCESS {
                let mut val: u32 = 0;
                let mut size = std::mem::size_of::<u32>() as u32;
                if RegQueryValueExW(
                    key,
                    name,
                    None,
                    None,
                    Some(&mut val as *mut u32 as *mut u8),
                    Some(&mut size),
                ) == ERROR_SUCCESS
                    && val == 1
                {
                    self.taskmgr_was_disabled = true;
                }
                let _ = RegCloseKey(key);
            }

            // Create / open the key and write the disable value
            let mut key = HKEY::default();
            let res = RegCreateKeyW(HKEY_CURRENT_USER, subkey, &mut key);
            if res != ERROR_SUCCESS {
                return Err(format!("RegCreateKeyW: error {}", res.0));
            }

            let val: u32 = 1;
            let res = RegSetValueExW(
                key,
                name,
                0,
                REG_DWORD,
                Some(std::slice::from_raw_parts(
                    &val as *const u32 as *const u8,
                    std::mem::size_of::<u32>(),
                )),
            );
            let _ = RegCloseKey(key);
            if res != ERROR_SUCCESS {
                return Err(format!("RegSetValueExW: error {}", res.0));
            }
        }
        log::info!("Task Manager disabled via registry");
        Ok(())
    }

    fn enable_task_manager(&self) {
        if self.taskmgr_was_disabled {
            return;
        }
        let subkey = windows::core::w!(
            "Software\\Microsoft\\Windows\\CurrentVersion\\Policies\\System"
        );
        let name = windows::core::w!("DisableTaskMgr");
        unsafe {
            let mut key = HKEY::default();
            if RegOpenKeyExW(HKEY_CURRENT_USER, subkey, 0, KEY_WRITE, &mut key) == ERROR_SUCCESS {
                let _ = RegDeleteValueW(key, name);
                let _ = RegCloseKey(key);
            }
        }
        log::info!("Task Manager re-enabled");
    }
}

impl super::PlatformLock for WindowsLock {
    fn engage(&mut self) -> Result<(), String> {
        self.install_keyboard_hook()?;
        self.disable_task_manager()?;
        Ok(())
    }

    fn disengage(&mut self) -> Result<(), String> {
        self.uninstall_keyboard_hook();
        self.enable_task_manager();
        Ok(())
    }
}

impl Drop for WindowsLock {
    fn drop(&mut self) {
        let _ = <Self as super::PlatformLock>::disengage(self);
    }
}
