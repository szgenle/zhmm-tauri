//! 标签统计、重命名、删除与标签注册表管理

use tauri::State;

use crate::errors::AppResult;
use crate::models::{TagDefinition, TagHierarchyNode, TagStats};
use crate::site_catalog;
use crate::site_catalog::UserCatalogState;
use crate::vault::VaultState;

#[derive(serde::Serialize)]
pub struct TagCount {
    pub tag: String,
    pub count: usize,
}

#[tauri::command]
pub fn collect_tag_counts(state: State<'_, VaultState>) -> AppResult<Vec<TagCount>> {
    let counts = state.collect_tag_counts()?;
    Ok(counts
        .into_iter()
        .map(|(tag, count)| TagCount { tag, count })
        .collect())
}

#[tauri::command]
pub fn rename_tag(old: String, new: String, state: State<'_, VaultState>) -> AppResult<usize> {
    state.rename_tag(&old, &new)
}

#[tauri::command]
pub fn delete_tag(tag: String, state: State<'_, VaultState>) -> AppResult<usize> {
    state.delete_tag(&tag)
}

// ========== 标签注册表 ==========

#[tauri::command]
pub fn list_tag_registry(state: State<'_, VaultState>) -> AppResult<Vec<TagDefinition>> {
    state.list_tag_registry()
}

#[tauri::command]
pub fn save_tag_registry(
    items: Vec<TagDefinition>,
    state: State<'_, VaultState>,
) -> AppResult<()> {
    state.save_tag_registry(items)
}

#[tauri::command]
pub fn upsert_tag_def(def: TagDefinition, state: State<'_, VaultState>) -> AppResult<()> {
    state.upsert_tag_def(def)
}

#[tauri::command]
pub fn remove_tag_def(name: String, state: State<'_, VaultState>) -> AppResult<()> {
    state.remove_tag_def(&name)
}

#[tauri::command]
pub fn merge_tags(
    sources: Vec<String>,
    target: String,
    state: State<'_, VaultState>,
) -> AppResult<usize> {
    state.merge_tags(&sources, &target)
}

#[tauri::command]
pub fn get_tag_stats(
    state: State<'_, VaultState>,
    user_catalog: State<'_, UserCatalogState>,
) -> AppResult<Vec<TagStats>> {
    let catalog_tags: std::collections::HashSet<String> =
        site_catalog::all_tags(&user_catalog).into_iter().collect();
    state.get_tag_stats(&catalog_tags)
}

#[tauri::command]
pub fn get_tag_hierarchy(state: State<'_, VaultState>) -> AppResult<Vec<TagHierarchyNode>> {
    state.get_tag_hierarchy()
}
