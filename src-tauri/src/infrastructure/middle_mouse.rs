use std::sync::{atomic::{AtomicBool, Ordering}, OnceLock};

static ENABLED: AtomicBool = AtomicBool::new(false);
type Trigger = Box<dyn Fn(i32, i32) + Send + Sync>;
static TRIGGER: OnceLock<Trigger> = OnceLock::new();

pub fn set_enabled(enabled: bool) { ENABLED.store(enabled, Ordering::Relaxed); }

pub fn install(trigger: Trigger) -> Result<(), String> {
    TRIGGER.set(trigger).map_err(|_| "中键监听已经初始化".to_string())?;
    #[cfg(windows)]
    unsafe {
        use windows_sys::Win32::{System::LibraryLoader::GetModuleHandleW, UI::WindowsAndMessaging::{SetWindowsHookExW, WH_MOUSE_LL}};
        let hook = SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_hook), GetModuleHandleW(std::ptr::null()), 0);
        if hook.is_null() { return Err(std::io::Error::last_os_error().to_string()); }
    }
    Ok(())
}

#[cfg(windows)]
unsafe extern "system" fn mouse_hook(code: i32, message: usize, data: isize) -> isize {
    use windows_sys::Win32::UI::WindowsAndMessaging::{CallNextHookEx, MSLLHOOKSTRUCT, WM_MBUTTONDOWN, WM_MBUTTONUP, LLMHF_INJECTED};
    if code >= 0 && ENABLED.load(Ordering::Relaxed) && (message == WM_MBUTTONDOWN as usize || message == WM_MBUTTONUP as usize) {
        let event = &*(data as *const MSLLHOOKSTRUCT);
        if event.flags & LLMHF_INJECTED == 0 {
            if message == WM_MBUTTONUP as usize {
                if let Some(trigger) = TRIGGER.get() { trigger(event.pt.x, event.pt.y); }
            }
            return 1;
        }
    }
    CallNextHookEx(std::ptr::null_mut(), code, message, data)
}
