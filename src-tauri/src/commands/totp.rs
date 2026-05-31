//! TOTP 生成与 otpauth URI 解析

use tauri::State;

use crate::errors::{AppError, AppResult};
use crate::totp::{self, OtpAuthParams};
use crate::vault::VaultState;

#[derive(serde::Serialize)]
pub struct TotpCode {
    pub code: String,
    pub remaining_seconds: u32,
}

#[tauri::command]
pub fn generate_totp(id: i64, state: State<'_, VaultState>) -> AppResult<TotpCode> {
    let entry = state.get(id)?;
    if entry.totp_secret.is_empty() {
        return Err(AppError::Invalid("未启用 TOTP".into()));
    }
    let algo = if entry.totp_algo.is_empty() {
        "SHA1"
    } else {
        entry.totp_algo.as_str()
    };
    let digits = if entry.totp_digits == 0 {
        6
    } else {
        entry.totp_digits
    };
    let period = if entry.totp_period == 0 {
        30
    } else {
        entry.totp_period
    };
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
