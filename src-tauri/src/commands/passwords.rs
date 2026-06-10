//! 密码条目 CRUD 与历史回滚

use tauri::State;

use crate::errors::AppResult;
use crate::models::{PasswordEntry, PasswordHistoryItem, PasswordInput, PasswordSummary};
use crate::vault::VaultState;

#[tauri::command]
pub fn list_passwords(state: State<'_, VaultState>) -> AppResult<Vec<PasswordSummary>> {
    state.list()
}

#[tauri::command]
pub fn get_password(id: i64, state: State<'_, VaultState>) -> AppResult<PasswordEntry> {
    state.get(id)
}

#[tauri::command]
pub fn add_password(
    input: PasswordInput,
    state: State<'_, VaultState>,
) -> AppResult<PasswordEntry> {
    state.add(input)
}

#[tauri::command]
pub fn delete_password(id: i64, state: State<'_, VaultState>) -> AppResult<()> {
    state.remove(id)
}

#[tauri::command]
pub fn update_password(
    id: i64,
    input: PasswordInput,
    state: State<'_, VaultState>,
) -> AppResult<PasswordEntry> {
    state.update(id, input)
}

#[tauri::command]
pub fn get_password_history(
    id: i64,
    state: State<'_, VaultState>,
) -> AppResult<Vec<PasswordHistoryItem>> {
    state.history(id)
}

#[tauri::command]
pub fn rollback_password(
    id: i64,
    history_index: usize,
    state: State<'_, VaultState>,
) -> AppResult<PasswordEntry> {
    state.rollback_password(id, history_index)
}

/// 批量给指定条目追加标签（不重复），返回实际修改的条目数
#[tauri::command]
pub fn batch_add_tag(
    ids: Vec<i64>,
    tag: String,
    state: State<'_, VaultState>,
) -> AppResult<usize> {
    state.batch_add_tag(&ids, &tag)
}
