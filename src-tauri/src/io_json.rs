//! 加密 JSON 备份与恢复（用户自定义备份密码）
//!
//! 与 vault.zhmm 共用同一加密栈（Argon2id + SM4-GCM v6）。
//! 由于备份场景不需要账号名因素，使用固定常量 "zhmm-backup" 作为 account
//! 输入；用户输入的备份密码作为唯一可变因子。

use std::fs;
use std::path::Path;

use crate::crypto::{open as crypto_open, seal as crypto_seal};
use crate::errors::{AppError, AppResult};
use crate::models::VaultData;

const BACKUP_ACCOUNT: &str = "zhmm-backup";

pub fn backup_to_file(path: &Path, data: &VaultData, password: &str) -> AppResult<()> {
    if password.is_empty() {
        return Err(AppError::Invalid("备份密码不能为空".into()));
    }
    let bytes = serde_json::to_vec(data)?;
    let blob = crypto_seal(BACKUP_ACCOUNT, password, &bytes)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, blob)?;
    Ok(())
}

pub fn restore_from_file(path: &Path, password: &str) -> AppResult<VaultData> {
    if !path.exists() {
        return Err(AppError::Other(format!("文件不存在: {}", path.display())));
    }
    let bytes = fs::read(path)?;
    let plain = crypto_open(BACKUP_ACCOUNT, password, &bytes)?;
    let mut data: VaultData =
        serde_json::from_slice(&plain).map_err(|e| AppError::Crypto(format!("json: {e}")))?;
    data.upgrade();
    Ok(data)
}
