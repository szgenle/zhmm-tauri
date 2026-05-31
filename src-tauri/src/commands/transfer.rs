//! xlsx 导入导出（含模板）

use std::path::PathBuf;

use tauri::State;

use crate::errors::{AppError, AppResult};
use crate::io_xlsx;
use crate::vault::VaultState;

/// 导出当前密码库为 xlsx（明文落盘，必须先重验主密码）
///
/// xlsx 导出是不可逆的明文脱敏，为防止"库已解锁"状态下被他人误操作，
/// 这里要求调用方传入主密码进行二次身份确认。
#[tauri::command]
pub fn export_xlsx(
    path: String,
    master_password: String,
    state: State<'_, VaultState>,
) -> AppResult<()> {
    if !state.verify_master_password(&master_password)? {
        return Err(AppError::InvalidPassword);
    }
    let snapshot = state.snapshot()?;
    io_xlsx::export_xlsx(&PathBuf::from(path), &snapshot.entries)
}

/// 从 xlsx 追加条目；返回导入条目数
#[tauri::command]
pub fn import_xlsx(path: String, state: State<'_, VaultState>) -> AppResult<usize> {
    let entries = io_xlsx::import_xlsx(&PathBuf::from(path))?;
    state.extend_entries(entries)
}

/// 导出空模板供用户填写后导入
#[tauri::command]
pub fn export_xlsx_template(path: String) -> AppResult<()> {
    io_xlsx::export_template(&PathBuf::from(path))
}
