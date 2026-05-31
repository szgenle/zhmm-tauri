//! 账号模板（v2.0+）CRUD 与导入导出

use std::path::PathBuf;

use tauri::State;

use crate::errors::AppResult;
use crate::models::{AccountTemplate, TemplateImportResult};
use crate::vault::VaultState;

#[tauri::command]
pub fn list_templates(state: State<'_, VaultState>) -> AppResult<Vec<AccountTemplate>> {
    state.templates()
}

#[tauri::command]
pub fn upsert_template(
    template: AccountTemplate,
    state: State<'_, VaultState>,
) -> AppResult<AccountTemplate> {
    state.upsert_template(template)
}

#[tauri::command]
pub fn delete_template(id: String, state: State<'_, VaultState>) -> AppResult<()> {
    state.delete_template(&id)
}

#[tauri::command]
pub fn seed_default_templates(state: State<'_, VaultState>) -> AppResult<usize> {
    state.seed_default_templates()
}

#[tauri::command]
pub fn export_templates_json(
    path: String,
    ids: Option<Vec<String>>,
    state: State<'_, VaultState>,
) -> AppResult<usize> {
    state.export_templates_json(&PathBuf::from(path), ids.as_deref())
}

#[tauri::command]
pub fn import_templates_json(
    path: String,
    overwrite: bool,
    state: State<'_, VaultState>,
) -> AppResult<TemplateImportResult> {
    state.import_templates_json(&PathBuf::from(path), overwrite)
}
