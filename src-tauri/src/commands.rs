//! Tauri 命令：前端调用入口

use tauri::State;

use crate::errors::AppResult;
use crate::models::{PasswordEntry, PasswordHistoryItem, PasswordInput, PasswordSummary};
use crate::totp::{self, OtpAuthParams};
use crate::vault::VaultState;

#[derive(serde::Serialize)]
pub struct VaultStatus {
    pub exists: bool,
    pub unlocked: bool,
}

#[tauri::command]
pub fn vault_status(state: State<'_, VaultState>) -> VaultStatus {
    VaultStatus {
        exists: state.vault_exists(),
        unlocked: state.is_unlocked(),
    }
}

#[tauri::command]
pub fn create_vault(master_password: String, state: State<'_, VaultState>) -> AppResult<()> {
    state.create(&master_password)
}

#[tauri::command]
pub fn unlock_vault(master_password: String, state: State<'_, VaultState>) -> AppResult<()> {
    state.unlock(&master_password)
}

#[tauri::command]
pub fn lock_vault(state: State<'_, VaultState>) {
    state.lock();
}

#[tauri::command]
pub fn list_passwords(state: State<'_, VaultState>) -> AppResult<Vec<PasswordSummary>> {
    state.list()
}

#[tauri::command]
pub fn get_password(id: String, state: State<'_, VaultState>) -> AppResult<PasswordEntry> {
    state.get(&id)
}

#[tauri::command]
pub fn add_password(
    input: PasswordInput,
    state: State<'_, VaultState>,
) -> AppResult<PasswordEntry> {
    state.add(input)
}

#[tauri::command]
pub fn delete_password(id: String, state: State<'_, VaultState>) -> AppResult<()> {
    state.remove(&id)
}

#[tauri::command]
pub fn update_password(
    id: String,
    input: PasswordInput,
    state: State<'_, VaultState>,
) -> AppResult<PasswordEntry> {
    state.update(&id, input)
}

#[tauri::command]
pub fn get_password_history(
    id: String,
    state: State<'_, VaultState>,
) -> AppResult<Vec<PasswordHistoryItem>> {
    state.history(&id)
}

#[derive(serde::Serialize)]
pub struct TotpCode {
    pub code: String,
    pub remaining_seconds: u32,
}

#[tauri::command]
pub fn generate_totp(id: String, state: State<'_, VaultState>) -> AppResult<TotpCode> {
    let entry = state.get(&id)?;
    if entry.totp_secret.is_empty() {
        return Err(crate::errors::AppError::Invalid("未启用 TOTP".into()));
    }
    let algo = if entry.totp_algo.is_empty() {
        "SHA1"
    } else {
        entry.totp_algo.as_str()
    };
    let digits = if entry.totp_digits == 0 { 6 } else { entry.totp_digits };
    let period = if entry.totp_period == 0 { 30 } else { entry.totp_period };
    let code = totp::generate(&entry.totp_secret, algo, digits, period, None)?;
    let remaining = totp::remaining_seconds(period, None)?;
    Ok(TotpCode {
        code,
        remaining_seconds: remaining,
    })
}

#[tauri::command]
pub fn parse_otpauth(uri: String) -> AppResult<OtpAuthParams> {
    totp::parse_otpauth_uri(&uri)
}
