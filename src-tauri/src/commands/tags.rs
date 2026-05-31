//! 标签统计、重命名与删除

use tauri::State;

use crate::errors::AppResult;
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
