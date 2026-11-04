use std::sync::OnceLock;

pub fn apply(window: &tauri::WebviewWindow) -> Result<(), String> {
    window
        .set_icon(icon()?)
        .map_err(|error| error.to_string())?;
    #[cfg(windows)]
    {
        use windows_sys::Win32::{
            System::LibraryLoader::GetModuleHandleW,
            UI::WindowsAndMessaging::{
                LoadImageW, SendMessageW, ICON_BIG, IMAGE_ICON, LR_DEFAULTSIZE, LR_SHARED,
                WM_SETICON,
            },
        };
        let hwnd = window.hwnd().map_err(|error| error.to_string())?.0 as isize;
        use tauri::Manager;
        static TASKBAR_ICON: OnceLock<Result<std::path::PathBuf, String>> = OnceLock::new();
        let path = TASKBAR_ICON
            .get_or_init(|| {
                super::taskbar_identity::materialize_icon(
                    &super::paths::data_root(window.app_handle()).join("branding"),
                    include_bytes!("../../icons/icon.ico"),
                )
            })
            .as_ref()
            .map_err(Clone::clone)?;
        super::taskbar_identity::apply(hwnd, &window.app_handle().config().identifier, path)?;
        let handle = unsafe {
            LoadImageW(
                GetModuleHandleW(std::ptr::null()),
                32512usize as *const u16,
                IMAGE_ICON,
                0,
                0,
                LR_SHARED | LR_DEFAULTSIZE,
            )
        };
        if handle.is_null() {
            return Err(format!(
                "无法加载任务栏图标：{}",
                std::io::Error::last_os_error()
            ));
        }
        let handle = handle as isize;
        window
            .run_on_main_thread(move || unsafe {
                SendMessageW(hwnd as _, WM_SETICON, ICON_BIG as usize, handle);
            })
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

pub fn icon() -> Result<tauri::image::Image<'static>, String> {
    static ICON: OnceLock<Result<tauri::image::Image<'static>, String>> = OnceLock::new();
    ICON.get_or_init(|| {
        let image = image::load_from_memory(include_bytes!("../../icons/128x128.png"))
            .map_err(|error| format!("无法加载 MyTools 图标：{error}"))?
            .into_rgba8();
        let (width, height) = image.dimensions();
        Ok(tauri::image::Image::new_owned(
            image.into_raw(),
            width,
            height,
        ))
    })
    .clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_icon_uses_the_current_brand_asset() {
        let expected = image::load_from_memory(include_bytes!("../../icons/128x128.png"))
            .unwrap()
            .into_rgba8();
        let actual = icon().unwrap();
        assert_eq!((actual.width(), actual.height()), (128, 128));
        assert_eq!(actual.rgba(), expected.as_raw());
        assert_eq!(icon().unwrap().rgba(), actual.rgba());
    }
}
