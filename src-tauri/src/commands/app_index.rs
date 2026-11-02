use crate::services::apps::index::{IndexedApp, ListedApp, ResolvedApp};

#[tauri::command]
pub async fn app_index_list() -> Vec<ListedApp> {
    crate::services::apps::index::list_async().await
}

#[tauri::command]
pub async fn app_index_refresh_if_stale() -> bool {
    crate::services::apps::index::refresh_if_stale_async().await
}

#[tauri::command]
pub async fn app_index_refresh() -> Vec<IndexedApp> {
    crate::services::apps::index::refresh_async().await
}

#[tauri::command]
pub async fn app_resolve(target: String) -> Option<ResolvedApp> {
    crate::services::apps::index::resolve_async(target).await
}
