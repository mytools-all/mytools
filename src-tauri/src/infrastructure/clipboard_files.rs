use std::path::PathBuf;

pub fn sequence() -> Option<u32> {
    #[cfg(windows)]
    {
        let value =
            unsafe { windows_sys::Win32::System::DataExchange::GetClipboardSequenceNumber() };
        (value != 0).then_some(value)
    }
    #[cfg(not(windows))]
    {
        None
    }
}

pub fn single_copied_file() -> Result<Option<PathBuf>, String> {
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStringExt;
        use windows_sys::Win32::System::DataExchange::{
            CloseClipboard, GetClipboardData, IsClipboardFormatAvailable, OpenClipboard,
        };
        use windows_sys::Win32::UI::Shell::DragQueryFileW;

        struct ClipboardGuard;
        impl Drop for ClipboardGuard {
            fn drop(&mut self) {
                unsafe { CloseClipboard() };
            }
        }

        let format = u32::from(windows::Win32::System::Ole::CF_HDROP.0);
        unsafe {
            if IsClipboardFormatAvailable(format) == 0 {
                return Ok(None);
            }
            if OpenClipboard(std::ptr::null_mut()) == 0 {
                return Err("剪贴板暂时被占用".into());
            }
            let _guard = ClipboardGuard;
            let drop = GetClipboardData(format);
            if drop.is_null() || DragQueryFileW(drop, u32::MAX, std::ptr::null_mut(), 0) != 1 {
                return Ok(None);
            }
            let length = DragQueryFileW(drop, 0, std::ptr::null_mut(), 0);
            if length == 0 || length > 32767 {
                return Ok(None);
            }
            let mut text = vec![0u16; length as usize + 1];
            if DragQueryFileW(drop, 0, text.as_mut_ptr(), text.len() as u32) != length {
                return Err("读取复制文件路径失败".into());
            }
            Ok(Some(PathBuf::from(std::ffi::OsString::from_wide(
                &text[..length as usize],
            ))))
        }
    }
    #[cfg(not(windows))]
    {
        Ok(None)
    }
}
