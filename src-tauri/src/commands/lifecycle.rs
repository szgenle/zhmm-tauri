//! 账号库生命周期：状态查询、创建、解锁、上锁

use std::path::PathBuf;

use tauri::State;

use crate::errors::AppResult;
use crate::vault::VaultState;

#[derive(serde::Serialize)]
pub struct VaultStatus {
    pub unlocked: bool,
    pub current_path: Option<String>,
    pub current_account: Option<String>,
}

#[tauri::command]
pub fn vault_status(state: State<'_, VaultState>) -> VaultStatus {
    VaultStatus {
        unlocked: state.is_unlocked(),
        current_path: state
            .current_path()
            .map(|p| p.to_string_lossy().to_string()),
        current_account: state.current_account(),
    }
}

#[tauri::command]
pub fn create_vault_at(
    path: String,
    account: String,
    master_password: String,
    state: State<'_, VaultState>,
) -> AppResult<()> {
    state.create(&PathBuf::from(path), &account, &master_password)
}

#[tauri::command]
pub fn unlock_with_path(
    path: String,
    account: String,
    master_password: String,
    state: State<'_, VaultState>,
) -> AppResult<()> {
    state.unlock_with_path(&PathBuf::from(path), &account, &master_password)
}

#[tauri::command]
pub fn lock_vault(state: State<'_, VaultState>) {
    state.lock();
}
