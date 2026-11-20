//! Clipboard commands — store + retrieve text and image history. The polling
//! task that fills the store is launched from `lib.rs::run`.

use mytools_core::clipboard::ClipboardStore;
use std::path::PathBuf;
use std::sync::Arc;

pub type SharedClipboardStore = Arc<ClipboardStore>;

pub fn open_store(path: PathBuf) -> Result<SharedClipboardStore, String> {
    ClipboardStore::open(&path)
        .map(Arc::new)
        .map_err(|e| e.to_string())
}

pub fn open_in_memory_store() -> Result<SharedClipboardStore, String> {
    ClipboardStore::open_in_memory()
        .map(Arc::new)
        .map_err(|e| e.to_string())
}

pub fn read_text<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> Result<String, String> {
    use tauri_plugin_clipboard_manager::ClipboardExt;
    app.clipboard().read_text().map_err(|e| e.to_string())
}

pub struct HistoryContent {
    pub content: String,
    pub image: bool,
}

pub fn read_history_content(app: &tauri::AppHandle) -> Result<Option<HistoryContent>, String> {
    if let Some(content) = read_image_content(app)? {
        return Ok(Some(HistoryContent {
            content,
            image: true,
        }));
    }
    let text = read_text(app)?;
    Ok((!text.is_empty()).then_some(HistoryContent {
        content: text,
        image: false,
    }))
}

pub fn read_image_content<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> Result<Option<String>, String> {
    use tauri_plugin_clipboard_manager::ClipboardExt;
    if let Ok(image) = app.clipboard().read_image() {
        return encode_history_image(&image);
    }
    if let Some(path) = crate::infrastructure::clipboard_files::single_copied_file()? {
        if let Some(content) = read_png_file(&path)? {
            return Ok(Some(content));
        }
    }
    Ok(None)
}

fn read_png_file(path: &std::path::Path) -> Result<Option<String>, String> {
    use std::io::Read;

    if !path.is_absolute()
        || !path
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("png"))
    {
        return Ok(None);
    }
    #[cfg(windows)]
    if !matches!(path.components().next(), Some(std::path::Component::Prefix(prefix))
        if matches!(prefix.kind(), std::path::Prefix::Disk(_) | std::path::Prefix::VerbatimDisk(_)))
    {
        return Ok(None);
    }
    let file = std::fs::File::open(path).map_err(|error| error.to_string())?;
    let metadata = file.metadata().map_err(|error| error.to_string())?;
    let max_bytes = mytools_core::clipboard::MAX_IMAGE_CONTENT_BYTES;
    if !metadata.is_file() || metadata.len() > max_bytes as u64 {
        return Ok(None);
    }
    let mut bytes = Vec::new();
    file.take(max_bytes as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| error.to_string())?;
    if bytes.len() > max_bytes {
        return Ok(None);
    }
    let reader =
        || image::ImageReader::with_format(std::io::Cursor::new(&bytes), image::ImageFormat::Png);
    let (width, height) = reader()
        .into_dimensions()
        .map_err(|error| error.to_string())?;
    if width.saturating_mul(height) > 16_000_000 {
        return Ok(None);
    }
    let mut decoder = reader();
    let mut limits = image::Limits::default();
    limits.max_alloc = Some(128 * 1024 * 1024);
    decoder.limits(limits);
    let rgba = decoder
        .decode()
        .map_err(|error| error.to_string())?
        .to_rgba8();
    encode_history_image(&tauri::image::Image::new_owned(
        rgba.into_raw(),
        width,
        height,
    ))
}

fn encode_history_image(image: &tauri::image::Image<'_>) -> Result<Option<String>, String> {
    if image.width().saturating_mul(image.height()) > 16_000_000 {
        return Ok(None);
    }
    let pixels = image::RgbaImage::from_raw(image.width(), image.height(), image.rgba().to_vec())
        .ok_or_else(|| "无效的剪贴板图片尺寸".to_string())?;
    let mut cursor = std::io::Cursor::new(Vec::new());
    pixels
        .write_to(&mut cursor, image::ImageFormat::Png)
        .map_err(|error| error.to_string())?;
    if cursor.get_ref().len() > 3 * 1024 * 1024 {
        return Ok(None);
    }
    let content = format!(
        "data:image/png;base64,{}",
        crate::infrastructure::app_icons::base64_encode(cursor.get_ref())
    );
    Ok((content.len() <= mytools_core::clipboard::MAX_IMAGE_CONTENT_BYTES).then_some(content))
}

pub fn write_text<R: tauri::Runtime>(app: &tauri::AppHandle<R>, text: &str) -> Result<(), String> {
    use tauri_plugin_clipboard_manager::ClipboardExt;
    app.clipboard()
        .write_text(text.to_string())
        .map_err(|e| e.to_string())
}

pub fn write_image<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    image: &tauri::image::Image<'_>,
) -> Result<(), String> {
    use tauri_plugin_clipboard_manager::ClipboardExt;
    app.clipboard()
        .write_image(image)
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn history_image_is_encoded_as_a_bounded_png_data_url() {
        let image = tauri::image::Image::new_owned(vec![255, 0, 0, 255], 1, 1);
        let content = encode_history_image(&image).unwrap().unwrap();
        assert!(content.starts_with("data:image/png;base64,iVBOR"));
        assert!(content.len() <= mytools_core::clipboard::MAX_IMAGE_CONTENT_BYTES);
    }

    #[test]
    fn invalid_pixels_and_oversized_images_are_not_recorded() {
        let invalid = tauri::image::Image::new_owned(Vec::new(), 1, 1);
        assert!(encode_history_image(&invalid).is_err());
        let oversized = tauri::image::Image::new_owned(Vec::new(), 16_000_001, 1);
        assert!(encode_history_image(&oversized).unwrap().is_none());
    }

    #[test]
    fn copied_png_file_uses_the_same_bounded_image_history_format() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("复制图片.PNG");
        image::RgbaImage::from_pixel(2, 2, image::Rgba([30, 80, 140, 255]))
            .save(&path)
            .unwrap();
        let content = read_png_file(&path).unwrap().unwrap();
        assert!(content.starts_with("data:image/png;base64,iVBOR"));
        assert!(read_png_file(std::path::Path::new("relative.png"))
            .unwrap()
            .is_none());
        assert!(read_png_file(&directory.path().join("ignored.txt"))
            .unwrap()
            .is_none());
    }

    #[test]
    fn invalid_and_large_png_files_do_not_enter_history() {
        let directory = tempfile::tempdir().unwrap();
        let invalid = directory.path().join("invalid.png");
        std::fs::write(&invalid, b"not a png").unwrap();
        assert!(read_png_file(&invalid).is_err());
        let large = directory.path().join("large.png");
        std::fs::File::create(&large)
            .unwrap()
            .set_len(mytools_core::clipboard::MAX_IMAGE_CONTENT_BYTES as u64 + 1)
            .unwrap();
        assert!(read_png_file(&large).unwrap().is_none());
        let oversized = directory.path().join("oversized.png");
        image::RgbaImage::new(4001, 4000).save(&oversized).unwrap();
        assert!(read_png_file(&oversized).unwrap().is_none());
    }
}
