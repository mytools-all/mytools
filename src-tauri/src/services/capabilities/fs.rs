use mytools_core::capability::{
    Capability, CapabilityCall, CapabilityError, CapabilityRegistry, Permission,
};
use mytools_core::dir_grants::DirGrantStore;
use serde_json::Value;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;
use tauri::{AppHandle, Manager, Runtime};

pub(super) fn register<R: Runtime>(
    app: &AppHandle<R>,
    registry: &mut CapabilityRegistry,
    dir_grants: Arc<DirGrantStore>,
) -> Result<(), CapabilityError> {
    let drop_app = app.clone();
    let drop_grants = Arc::clone(&dir_grants);
    registry.register(Capability::new("fs.importdropped", Permission::Write, "Import native dropped files and authorize their containing directories", Box::new(move |call| {
        let window = call.ctx.window_label.as_deref().ok_or_else(|| CapabilityError::invalid_args("missing window"))?;
        let paths = drop_app.state::<crate::runtime::DroppedFiles>().take(window)
            .map_err(|e| CapabilityError::handler_failed("fs.importdropped", e))?;
        let mut entries = Vec::new();
        for path in paths {
            let metadata = std::fs::symlink_metadata(&path).map_err(|e| CapabilityError::invalid_args(e.to_string()))?;
            if metadata.file_type().is_symlink() || !(metadata.is_file() || metadata.is_dir()) {
                return Err(CapabilityError::invalid_args("不支持符号链接或特殊文件"));
            }
            let parent = path.parent().ok_or_else(|| CapabilityError::invalid_args("无法导入根目录"))?;
            drop_grants.grant(call.ctx.scope(), parent).map_err(|e| CapabilityError::handler_failed("fs.importdropped", e))?;
            entries.push(serde_json::json!({"path":path,"name":path.file_name().unwrap_or_default().to_string_lossy(),"directory":metadata.is_dir(),"size":metadata.len(),"modified":metadata.modified().ok().and_then(|t|t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d|d.as_secs()).unwrap_or(0)}));
        }
        Ok(serde_json::json!({"entries":entries}))
    })))?;
    let export_app = app.clone();
    registry.register(Capability::new(
        "fs.exporttext",
        Permission::Write,
        "Save text to a file selected by the user",
        Box::new(move |call| {
            use tauri_plugin_dialog::DialogExt;
            let content = call
                .arg_str("content")
                .ok_or_else(|| CapabilityError::invalid_args("expected content"))?;
            if content.len() > 8 * 1024 * 1024 {
                return Err(CapabilityError::invalid_args("导出内容超过 8 MB"));
            }
            let name = call
                .arg_str("name")
                .unwrap_or("文档.md")
                .replace(['\\', '/'], "_");
            let picked = export_app
                .dialog()
                .file()
                .set_title("导出文档")
                .set_file_name(name)
                .blocking_save_file();
            let Some(picked) = picked else {
                return Ok(serde_json::json!({"cancelled":true}));
            };
            let path = picked
                .into_path()
                .map_err(|e| CapabilityError::handler_failed("fs.exporttext", e.to_string()))?;
            std::fs::write(&path, content)
                .map_err(|e| CapabilityError::handler_failed("fs.exporttext", e.to_string()))?;
            Ok(serde_json::json!({"path":path}))
        }),
    ))?;
    registry.register(Capability::new(
        "fs.searchstatus",
        Permission::Read,
        "Check the local Everything connection",
        Box::new(|_| Ok(crate::infrastructure::everything::status())),
    ))?;
    registry.register(Capability::new(
        "fs.globalsearch",
        Permission::Exec,
        "Query the Everything file index",
        Box::new(|call| {
            let query = serde_json::from_value(call.args.clone())
                .map_err(|e| CapabilityError::invalid_args(format!("{e}")))?;
            crate::services::file_search::search(call.ctx.scope(), query)
                .map_err(|e| CapabilityError::handler_failed("fs.globalsearch", e))
        }),
    ))?;
    registry.register(Capability::new(
        "fs.searchicons",
        Permission::Read,
        "Read Windows Shell icons for current search results",
        Box::new(|call| {
            let paths: Vec<String> = serde_json::from_value(call.args.get("paths").cloned().unwrap_or(Value::Null))
                .map_err(|_| CapabilityError::invalid_args("expected { paths: string[] }"))?;
            crate::services::file_search::icons(call.ctx.scope(), &paths)
                .map_err(|e| CapabilityError::handler_failed("fs.searchicons", e))
        }),
    ))?;
    registry.register(Capability::new(
        "fs.searchpreview",
        Permission::Exec,
        "Preview a file from the current search results",
        Box::new(|call| {
            let path = call
                .arg_str("path")
                .ok_or_else(|| CapabilityError::invalid_args("expected path"))?;
            crate::services::file_search::preview(call.ctx.scope(), path)
                .map_err(|e| CapabilityError::handler_failed("fs.searchpreview", e))
        }),
    ))?;
    let picker_app = app.clone();
    let picker_grants = Arc::clone(&dir_grants);
    registry.register(Capability::new("fs.pickfiles", Permission::Write, "Select files and grant access to their containing directories", Box::new(move |call| {
        use tauri_plugin_dialog::DialogExt;
        let directories=call.args.get("directories").and_then(Value::as_bool).unwrap_or(false);
        let picker=picker_app.dialog().file().set_title("选择条目并授权所在目录，用于文件操作");
        let picked=if directories {picker.blocking_pick_folder().map(|p|vec![p])}else{picker.blocking_pick_files()}.ok_or_else(||CapabilityError::handler_failed("fs.pickfiles","cancelled"))?;
        let mut entries=Vec::new();
        for picked in picked.into_iter().take(1000) {
            let path=picked.into_path().map_err(|e|CapabilityError::handler_failed("fs.pickfiles",e.to_string()))?;
            let parent=path.parent().ok_or_else(||CapabilityError::invalid_args("无法选择根目录"))?;
            picker_grants.grant(call.ctx.scope(),parent).map_err(|e|CapabilityError::handler_failed("fs.pickfiles",e))?;
            let metadata=std::fs::metadata(&path).map_err(|e|CapabilityError::handler_failed("fs.pickfiles",e.to_string()))?;
            entries.push(serde_json::json!({"path":path,"name":path.file_name().unwrap_or_default().to_string_lossy(),"directory":metadata.is_dir(),"size":metadata.len(),"modified":metadata.modified().ok().and_then(|t|t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d|d.as_secs()).unwrap_or(0)}));
        }
        Ok(serde_json::json!({"entries":entries}))
    })))?;
    let fs_read_app = app.clone();
    let fs_read_grants = Arc::clone(&dir_grants);
    registry.register(Capability::new(
        "fs.read",
        Permission::Exec,
        "Read a UTF-8 file from the plugin file directory or a granted directory",
        Box::new(move |call: &CapabilityCall| {
            let path = resolve_fs_path(&fs_read_app, call, &fs_read_grants)?;
            let content = std::fs::read_to_string(&path)
                .map_err(|e| CapabilityError::handler_failed("fs.read", e.to_string()))?;
            Ok(serde_json::json!({ "content": content }))
        }),
    ))?;
    let fs_write_app = app.clone();
    let fs_write_grants = Arc::clone(&dir_grants);
    registry.register(Capability::new(
        "fs.write",
        Permission::Write,
        "Write a UTF-8 file into the plugin file directory or a granted directory",
        Box::new(move |call: &CapabilityCall| {
            let path = resolve_fs_path(&fs_write_app, call, &fs_write_grants)?;
            let content = call.arg_str("content").ok_or_else(|| {
                CapabilityError::invalid_args("fs.write expects { path, content }")
            })?;
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| CapabilityError::handler_failed("fs.write", e.to_string()))?;
            }
            std::fs::write(&path, content)
                .map_err(|e| CapabilityError::handler_failed("fs.write", e.to_string()))?;
            Ok(Value::Null)
        }),
    ))?;
    let pick_app = app.clone();
    let pick_grants = Arc::clone(&dir_grants);
    registry.register(Capability::new(
        "fs.pickdirectory",
        Permission::Write,
        "Ask the user for a directory and remember it as a grant",
        Box::new(move |call: &CapabilityCall| {
            use tauri_plugin_dialog::DialogExt;
            let picked = pick_app
                .dialog()
                .file()
                .blocking_pick_folder()
                .ok_or_else(|| CapabilityError::handler_failed("fs.pickdirectory", "cancelled"))?;
            let path = picked
                .into_path()
                .map_err(|e| CapabilityError::handler_failed("fs.pickdirectory", e.to_string()))?;
            pick_grants
                .grant(call.ctx.scope(), &path)
                .map_err(|e| CapabilityError::handler_failed("fs.pickdirectory", e))?;
            Ok(serde_json::json!({ "path": path.to_string_lossy() }))
        }),
    ))?;
    let list_grants = Arc::clone(&dir_grants);
    registry.register(Capability::new(
        "fs.grants",
        Permission::Read,
        "List the directories the caller was granted",
        Box::new(move |call: &CapabilityCall| {
            let scope = call.ctx.scope();
            let grants: Vec<serde_json::Value> = list_grants
                .list()
                .into_iter()
                .filter(|g| g.plugin == scope)
                .map(|g| serde_json::json!({ "plugin": g.plugin, "path": g.path }))
                .collect();
            Ok(serde_json::json!({ "grants": grants }))
        }),
    ))?;
    let revoke_grants = Arc::clone(&dir_grants);
    registry.register(Capability::new(
        "fs.revokegrant",
        Permission::Write,
        "Revoke directory grants of the caller",
        Box::new(move |call: &CapabilityCall| {
            let scope = call.ctx.scope();
            let path = call.arg_str("path");
            let removed = revoke_grants
                .revoke(scope, path)
                .map_err(|e| CapabilityError::handler_failed("fs.revokegrant", e))?;
            Ok(serde_json::json!({ "removed": removed }))
        }),
    ))?;
    let search_grants = Arc::clone(&dir_grants);
    registry.register(Capability::new(
        "fs.search",
        Permission::Read,
        "Search files in a granted directory",
        Box::new(move |call| {
            let options = serde_json::from_value(call.args.clone())
                .map_err(|e| CapabilityError::invalid_args(format!("{e}")))?;
            crate::services::file_tools::search(&search_grants, call.ctx.scope(), options)
        }),
    ))?;
    registry.register(
        Capability::new(
            "fs.renamemany",
            Permission::Dangerous,
            "Rename files in granted directories without overwriting",
            Box::new(move |call| {
                let entries = serde_json::from_value(
                    call.args.get("entries").cloned().unwrap_or(Value::Null),
                )
                .map_err(|e| CapabilityError::invalid_args(format!("{e}")))?;
                crate::services::file_tools::rename_many(&dir_grants, call.ctx.scope(), entries)
            }),
        )
        .with_custom_consent(),
    )?;
    Ok(())
}

fn safe_scope_component(scope: &str) -> String {
    let mut out: String = scope
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '_'
            }
        })
        .collect();
    if out.is_empty() {
        out.push_str("app");
    }
    out
}

fn resolve_relative_path(root: &Path, requested: &str) -> Result<PathBuf, CapabilityError> {
    let path = Path::new(requested);
    if requested.trim().is_empty()
        || path.is_absolute()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(CapabilityError::Denied {
            capability: "fs".to_string(),
            reason: "only relative paths inside the plugin file directory are allowed".to_string(),
        });
    }
    Ok(root.join(path))
}

fn resolve_fs_path<R: Runtime>(
    app: &AppHandle<R>,
    call: &CapabilityCall,
    dir_grants: &DirGrantStore,
) -> Result<PathBuf, CapabilityError> {
    let requested = call
        .arg_str("path")
        .ok_or_else(|| CapabilityError::invalid_args("expected { path }"))?;
    let path = Path::new(requested);
    if path.is_absolute() {
        if dir_grants.is_granted(call.ctx.scope(), path) {
            return Ok(path.to_path_buf());
        }
        return Err(CapabilityError::Denied {
            capability: "fs".to_string(),
            reason: format!(
                "`{requested}` is outside the granted directories; ask the user for access first"
            ),
        });
    }
    let root = crate::infrastructure::paths::data_root(app)
        .join("plugin-files")
        .join(safe_scope_component(call.ctx.scope()));
    resolve_relative_path(&root, requested)
}

#[cfg(test)]
mod tests {
    use super::*;
    use mytools_core::capability::{CallContext, Origin};
    #[test]
    fn plugin_file_scope_is_sanitized() {
        assert_eq!(safe_scope_component("demo"), "demo");
        assert_eq!(safe_scope_component("@scope/pkg"), "_scope_pkg");
        assert_eq!(safe_scope_component("../escape"), "___escape");
        assert_eq!(safe_scope_component(""), "app");
    }

    #[test]
    fn plugin_file_paths_stay_inside_the_plugin_directory() {
        let root = Path::new("C:/data/plugin-files/demo");
        assert_eq!(
            resolve_relative_path(root, "notes/a.txt").unwrap(),
            root.join("notes/a.txt")
        );
        assert!(resolve_relative_path(root, "../../secret").is_err());
        assert!(resolve_relative_path(root, "C:/Windows/win.ini").is_err());
        assert!(resolve_relative_path(root, "/etc/passwd").is_err());
        assert!(resolve_relative_path(root, "").is_err());
    }

    #[test]
    fn fs_write_is_allowed_for_self_installed_plugins_after_consent() {
        use mytools_core::capability::{Decision, Policy};
        let handler: mytools_core::capability::Handler =
            Box::new(|_call: &CapabilityCall| Ok(Value::Null));
        let write = Capability::new("fs.write", Permission::Write, "d", handler);
        let policy = mytools_core::capability::DefaultPolicy;
        assert_eq!(
            policy.decide(&write, &CallContext::new(Origin::Custom)),
            Decision::Prompt
        );
        assert_eq!(
            policy.decide(&write, &CallContext::new(Origin::Official)),
            Decision::Allow
        );
    }
}
