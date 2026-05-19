//! 加密 JSON 备份与恢复
//!
//! 备份格式与 vault.zhmm 完全相同（Argon2id + SM4-CBC + HMAC-SM3 整段加密 JSON）
//! 区别只是文件路径由用户指定。可以使用与主密码不同的备份密码。

use std::fs;
use std::path::Path;

use crate::crypto::{decrypt, encrypt, Envelope};
use crate::errors::{AppError, AppResult};
use crate::models::VaultData;

/// 把整个 VaultData 加密写到指定文件
pub fn backup_to_file(path: &Path, data: &VaultData, password: &[u8]) -> AppResult<()> {
    let bytes = serde_json::to_vec(data)?;
    let env = encrypt(&bytes, password)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, env.to_bytes())?;
    Ok(())
}

/// 从文件解密恢复 VaultData
pub fn restore_from_file(path: &Path, password: &[u8]) -> AppResult<VaultData> {
    if !path.exists() {
        return Err(AppError::Other(format!("文件不存在: {}", path.display())));
    }
    let bytes = fs::read(path)?;
    let env = Envelope::from_bytes(&bytes)?;
    let plain = decrypt(&env, password)?;
    let mut data: VaultData =
        serde_json::from_slice(&plain).map_err(|e| AppError::Crypto(format!("json: {e}")))?;
    data.upgrade();
    Ok(data)
}
