//! 加密备份与本地备份管理

use std::path::PathBuf;

use tauri::State;

use crate::errors::AppResult;
use crate::io_json;
use crate::vault::VaultState;

/// 加密 JSON 备份到指定文件。
///
/// `backup_password` 为 `None` 或空串时，默认使用当前会话的主密码加密备份（一键备份）；
/// 如需使用独立的备份密码，则显式传入。备份产物本身是密文，
/// 因此备份动作不需要额外的身份再验证。
#[tauri::command]
pub fn backup_to_file(
    path: String,
    backup_password: Option<String>,
    state: State<'_, VaultState>,
) -> AppResult<()> {
    match backup_password {
        Some(pwd) if !pwd.is_empty() => {
            let snapshot = state.snapshot()?;
            io_json::backup_to_file(&PathBuf::from(path), &snapshot, &pwd)
        }
        _ => state.backup_using_master(&PathBuf::from(path)),
    }
}

/// 从加密 JSON 文件恢复（完全覆盖当前数据）
#[tauri::command]
pub fn restore_from_file(
    path: String,
    backup_password: String,
    state: State<'_, VaultState>,
) -> AppResult<()> {
    let data = io_json::restore_from_file(&PathBuf::from(path), &backup_password)?;
    state.replace(data)
}

#[tauri::command]
pub fn create_local_backup(state: State<'_, VaultState>) -> AppResult<String> {
    state.create_local_backup()
}

#[tauri::command]
pub fn list_local_backups(
    state: State<'_, VaultState>,
) -> AppResult<Vec<crate::vault::BackupInfo>> {
    state.list_local_backups()
}

#[tauri::command]
pub fn delete_local_backup(name: String, state: State<'_, VaultState>) -> AppResult<()> {
    state.delete_local_backup(&name)
}

#[tauri::command]
pub fn restore_local_backup(name: String, state: State<'_, VaultState>) -> AppResult<()> {
    state.restore_local_backup(&name)
}

#[tauri::command]
pub fn cleanup_backups(keep: usize, state: State<'_, VaultState>) -> AppResult<u32> {
    state.cleanup_backups(keep)
}
