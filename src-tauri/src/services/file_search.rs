use crate::infrastructure::everything::{self, Query};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
static RESULTS: OnceLock<Mutex<HashMap<String, HashSet<PathBuf>>>> = OnceLock::new();
pub fn search(scope: &str, options: Query) -> Result<Value, String> {
    let result = everything::query(options)?;
    let paths = result
        .entries
        .iter()
        .map(|entry| PathBuf::from(&entry.path))
        .collect();
    let mut cache = RESULTS
        .get_or_init(Default::default)
        .lock()
        .map_err(|_| "搜索会话不可用")?;
    if cache.len() > 100 {
        cache.clear();
    }
    cache.insert(scope.into(), paths);
    serde_json::to_value(result).map_err(|e| e.to_string())
}
pub fn preview(scope: &str, path: &str) -> Result<Value, String> {
    let path = PathBuf::from(path);
    let allowed = RESULTS
        .get_or_init(Default::default)
        .lock()
        .map_err(|_| "搜索会话不可用")?
        .get(scope)
        .is_some_and(|paths| paths.contains(&path));
    if !allowed {
        return Err("只能预览当前搜索结果中的文件，请重新搜索并选择文件".into());
    }
    let metadata = std::fs::metadata(&path).map_err(|e| e.to_string())?;
    if !metadata.is_file() {
        return Ok(json!({"kind":"directory","text":path.to_string_lossy()}));
    }
    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase();
    let mime = match ext.as_str() {
        "png" => Some("image/png"),
        "jpg" | "jpeg" => Some("image/jpeg"),
        "gif" => Some("image/gif"),
        "webp" => Some("image/webp"),
        "bmp" => Some("image/bmp"),
        _ => None,
    };
    if let Some(mime) = mime {
        if metadata.len() > 4 * 1024 * 1024 {
            return Err("图片大于 4 MB，请使用默认程序打开".into());
        }
        let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
        return Ok(json!({"kind":"image","bytes":bytes,"mime":mime}));
    }
    let media = match ext.as_str() {
        "pdf" => Some(("pdf", "application/pdf")),
        "mp3" => Some(("audio", "audio/mpeg")),
        "wav" => Some(("audio", "audio/wav")),
        "ogg" => Some(("audio", "audio/ogg")),
        "mp4" => Some(("video", "video/mp4")),
        "webm" => Some(("video", "video/webm")),
        _ => None,
    };
    if let Some((kind, mime)) = media {
        if metadata.len() > 8 * 1024 * 1024 { return Err("文件大于 8 MB，请使用默认程序打开".into()); }
        let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
        return Ok(json!({"kind":kind,"bytes":bytes,"mime":mime}));
    }
    use std::io::Read;
    let mut bytes = Vec::new();
    std::fs::File::open(path)
        .map_err(|e| e.to_string())?
        .take(20000)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.contains(&0) {
        return Err("此文件格式暂不支持内嵌预览，请使用默认程序打开".into());
    }
    Ok(
        json!({"kind":"text","text":String::from_utf8_lossy(&bytes),"truncated":metadata.len()>20000}),
    )
}
