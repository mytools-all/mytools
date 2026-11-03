use crate::infrastructure::everything::{self, Query};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
static RESULTS: OnceLock<Mutex<HashMap<String, HashSet<PathBuf>>>> = OnceLock::new();

pub fn icons(scope: &str, paths: &[String]) -> Result<Value, String> {
    if paths.len() > 16 {
        return Err("每次最多读取16个搜索结果图标".into());
    }
    let allowed = RESULTS
        .get_or_init(Default::default)
        .lock()
        .map_err(|_| "搜索会话不可用")?
        .get(scope)
        .is_some_and(|results| {
            paths
                .iter()
                .all(|path| results.contains(&PathBuf::from(path)))
        });
    if !allowed {
        return Err("只能读取当前搜索结果中的图标，请重新搜索".into());
    }
    Ok(Value::Array(paths.iter().map(|path| {
        json!({"path":path,"icon":crate::infrastructure::app_icons::resolve_icon(path)})
    }).collect()))
}
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
        if metadata.len() > 8 * 1024 * 1024 {
            return Err("文件大于 8 MB，请使用默认程序打开".into());
        }
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

#[cfg(test)]
mod icon_tests {
    use super::*;

    #[test]
    fn icon_requests_require_current_results_and_bounded_batches() {
        assert!(icons("missing-icon-scope", &["unlisted".into()]).is_err());
        assert!(icons("missing-icon-scope", &vec!["unlisted".into(); 17]).is_err());
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().to_string_lossy().into_owned();
        let scope = "icon-test-current-results";
        RESULTS
            .get_or_init(Default::default)
            .lock()
            .unwrap()
            .insert(scope.into(), HashSet::from([PathBuf::from(&path)]));
        let response = icons(scope, std::slice::from_ref(&path)).unwrap();
        assert_eq!(response.as_array().unwrap().len(), 1);
        assert_eq!(response[0]["path"], path);
        assert!(icons(scope, &[path.clone(), "unlisted".into()]).is_err());
        RESULTS
            .get()
            .unwrap()
            .lock()
            .unwrap()
            .insert(scope.into(), HashSet::new());
        assert!(icons(scope, &[path]).is_err());
        RESULTS.get().unwrap().lock().unwrap().remove(scope);
    }
}
