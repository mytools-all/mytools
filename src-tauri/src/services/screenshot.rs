pub fn capture(app: &tauri::AppHandle) -> Result<Option<String>, String> {
    #[cfg(not(windows))]
    {
        let _ = app;
        Err("当前截图入口仅支持 Windows".into())
    }
    #[cfg(windows)]
    {
        use std::time::{Duration, Instant};
        use windows::core::PCWSTR;
        use windows::Win32::UI::{Shell::ShellExecuteW, WindowsAndMessaging::SW_SHOWNORMAL};
        use windows_sys::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_ESCAPE};
        let baseline = crate::infrastructure::clipboard_files::sequence();
        let uri: Vec<u16> = "ms-screenclip:".encode_utf16().chain(Some(0)).collect();
        let launched = unsafe {
            ShellExecuteW(
                None,
                PCWSTR::null(),
                PCWSTR(uri.as_ptr()),
                PCWSTR::null(),
                PCWSTR::null(),
                SW_SHOWNORMAL,
            )
        };
        if launched.0 as isize <= 32 {
            return Err("无法启动 Windows 截图工具，请检查系统截图组件".into());
        }
        let start = Instant::now();
        let mut changed_at = None;
        while start.elapsed() < Duration::from_secs(90) {
            std::thread::sleep(Duration::from_millis(20));
            let current = crate::infrastructure::clipboard_files::sequence();
            if current.is_some() && current != baseline {
                if let Ok(Some(image)) = crate::services::clipboard::read_image_content(app) {
                    return Ok(Some(image));
                }
                if changed_at.is_none() {
                    changed_at = Some(Instant::now());
                }
                if changed_at.is_some_and(|time| time.elapsed() > Duration::from_secs(3)) {
                    return Err(
                        "未读取到可用截图，请缩小选区后重试（PNG 不超过 3 MB、1600 万像素）".into(),
                    );
                }
            }
            if unsafe { GetAsyncKeyState(VK_ESCAPE as i32) } as u16 & 0x8000 != 0 {
                return Ok(None);
            }
        }
        Err("截图已超时，请重新选择区域".into())
    }
}
