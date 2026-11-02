use mytools_core::dir_grants::{DirGrant, DirGrantStore};

#[tauri::command]
pub fn dir_grants_list(grants: tauri::State<'_, std::sync::Arc<DirGrantStore>>) -> Vec<DirGrant> {
    grants.list()
}

/// Revoke one grant, or every grant of a plugin when `path` is absent.
#[tauri::command]
pub fn dir_grants_revoke(
    grants: tauri::State<'_, std::sync::Arc<DirGrantStore>>,
    plugin: String,
    path: Option<String>,
) -> Result<usize, String> {
    grants.revoke(&plugin, path.as_deref())
}
