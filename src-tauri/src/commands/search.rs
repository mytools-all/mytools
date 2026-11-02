use crate::services::plugins::SharedPluginRegistry;
use crate::services::search::SearchResult;

#[tauri::command]
pub async fn search(
    query: String,
    registry: tauri::State<'_, SharedPluginRegistry>,
) -> Result<Vec<SearchResult>, String> {
    crate::services::search::search(query, registry.inner().clone()).await
}
