//! 主密码管理：verify_master_password / rekey

use std::fs;
use zeroize::Zeroize;

use crate::crypto::{open as crypto_open, seal as crypto_seal};
use crate::errors::{AppError, AppResult};

use super::VaultState;

impl VaultState {
    /// 校验主密码：尝试用给定密码解密当前文件，成功即正确
    pub fn verify_master_password(&self, password: &str) -> AppResult<bool> {
        let account = self.account.read().clone().ok_or(AppError::Locked)?;
        let path = self
            .path
            .read()
            .clone()
            .ok_or_else(|| AppError::Other("未指定密码库路径".into()))?;
        let bytes = fs::read(&path)?;
        match crypto_open(&account, password, &bytes) {
            Ok(_) => Ok(true),
            Err(AppError::InvalidPassword)
            | Err(AppError::Crypto(_))
            | Err(AppError::IntegrityCheck) => Ok(false),
            Err(e) => Err(e),
        }
    }

    /// 更换主密码：必须已解锁，先用旧密码核对缓存 → 创建保险备份 → 用新密码重写
    pub fn rekey(&self, old_password: &str, new_password: &str) -> AppResult<String> {
        if new_password.is_empty() {
            return Err(AppError::Invalid("新主密码不能为空".into()));
        }
        if new_password == old_password {
            return Err(AppError::Invalid("新主密码不能与旧主密码相同".into()));
        }
        let cached = self.master.read().clone().ok_or(AppError::Locked)?;
        if cached.as_slice() != old_password.as_bytes() {
            return Err(AppError::InvalidPassword);
        }
        let account = self.account.read().clone().ok_or(AppError::Locked)?;
        let path = self
            .path
            .read()
            .clone()
            .ok_or_else(|| AppError::Other("未指定密码库路径".into()))?;

        // 1) 保险备份（旧密码加密）
        let backup_name = self.create_rekey_backup()?;

        // 2) 用新密码重新加密当前数据并原子落盘
        let bytes = {
            let guard = self.data.read();
            let data = guard.as_ref().ok_or(AppError::Locked)?;
            serde_json::to_vec(data)?
        };
        let new_blob = crypto_seal(&account, new_password, &bytes)?;
        Self::atomic_write(&path, &new_blob)?;

        // 3) 更新缓存的主密码
        {
            let mut master_guard = self.master.write();
            if let Some(mut old) = master_guard.take() {
                old.zeroize();
            }
            *master_guard = Some(new_password.as_bytes().to_vec());
        }
        Ok(backup_name)
    }
}
