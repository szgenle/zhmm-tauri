//! 密码库状态与文件持久化
//!
//! 解锁后的明文驻留内存，主密码与账号缓存为字节，Drop 时 zeroize。
//! 路径不再固定，由前端通过 set_active_path / unlock_with_path 指定。
//!
//! 模块拆分：方法按域分散到子模块的 `impl VaultState` 中，
//! 共享的私有字段与 helper 方法（atomic_write / persist_with_cached
//! / backup_dir / create_rekey_backup）都留在本文件，依赖 Rust 子模块
//! 可访问父模块私有项的特性自然可见。

use parking_lot::RwLock;
use std::fs;
use std::path::{Path, PathBuf};
use zeroize::Zeroize;

use crate::crypto::{open as crypto_open, seal as crypto_seal};
use crate::errors::{AppError, AppResult};
use crate::models::{default_templates, VaultData};

mod backup;
mod passwords;
mod rekey;
mod tags;
mod templates;

/// 备份条目元信息（返回给前端）
#[derive(Debug, Clone, serde::Serialize)]
pub struct BackupInfo {
    pub name: String,
    pub size: u64,
    pub created_at: String,
}

const BACKUP_DIR_NAME: &str = ".backups";
/// 本地备份默认后缀（v2.0 起）
const BACKUP_EXT: &str = "ajot";
/// 历史备份后缀（仅读，兼容 v0.x 生成的 .zhmm 备份文件）
const BACKUP_EXT_LEGACY: &str = "zhmm";

pub struct VaultState {
    /// 当前活跃的密码库文件路径（未指定时为 None）
    path: RwLock<Option<PathBuf>>,
    /// 解锁后的明文密码库
    data: RwLock<Option<VaultData>>,
    /// 缓存当前账号名（与 master 同生命周期）
    account: RwLock<Option<String>>,
    /// 缓存主密码字节
    master: RwLock<Option<Vec<u8>>>,
}

impl VaultState {
    pub fn new() -> Self {
        Self {
            path: RwLock::new(None),
            data: RwLock::new(None),
            account: RwLock::new(None),
            master: RwLock::new(None),
        }
    }

    pub fn current_path(&self) -> Option<PathBuf> {
        self.path.read().clone()
    }

    pub fn current_account(&self) -> Option<String> {
        self.account.read().clone()
    }

    pub fn is_unlocked(&self) -> bool {
        self.data.read().is_some()
    }

    /// 创建新密码库（路径不存在时）
    pub fn create(&self, path: &Path, account: &str, master_password: &str) -> AppResult<()> {
        if account.is_empty() {
            return Err(AppError::Invalid("账号名不能为空".into()));
        }
        if master_password.is_empty() {
            return Err(AppError::Invalid("主密码不能为空".into()));
        }
        if path.exists() {
            return Err(AppError::Other(format!("文件已存在: {}", path.display())));
        }
        let mut data = VaultData::new();
        // 新建库：预装内建模板种子（账号小本本 v2.0+）
        data.templates = default_templates();
        let bytes = serde_json::to_vec(&data)?;
        let blob = crypto_seal(account, master_password, &bytes)?;
        Self::atomic_write(path, &blob)?;

        *self.path.write() = Some(path.to_path_buf());
        *self.data.write() = Some(data);
        *self.account.write() = Some(account.to_string());
        *self.master.write() = Some(master_password.as_bytes().to_vec());
        Ok(())
    }

    /// 用 (account, password) 打开指定路径的密码库并解锁
    pub fn unlock_with_path(
        &self,
        path: &Path,
        account: &str,
        master_password: &str,
    ) -> AppResult<()> {
        if !path.exists() {
            return Err(AppError::Other(format!("文件不存在: {}", path.display())));
        }
        let bytes = fs::read(path)?;
        let plain = crypto_open(account, master_password, &bytes)?;
        let mut data: VaultData = serde_json::from_slice(&plain)
            .map_err(|e| AppError::Crypto(format!("vault json: {e}")))?;
        data.upgrade();

        *self.path.write() = Some(path.to_path_buf());
        *self.data.write() = Some(data);
        *self.account.write() = Some(account.to_string());
        *self.master.write() = Some(master_password.as_bytes().to_vec());
        Ok(())
    }

    /// 锁定：清空内存中的明文与主密码
    pub fn lock(&self) {
        if let Some(mut data) = self.data.write().take() {
            for e in &mut data.entries {
                e.pwd.zeroize();
                e.desc.zeroize();
                e.totp_secret.zeroize();
                for h in &mut e.history {
                    h.pwd.zeroize();
                }
                // 扩展字段可能存证件号/口令卷等，同步 zeroize
                for (_, v) in e.custom_fields.iter_mut() {
                    v.zeroize();
                }
            }
        }
        if let Some(mut m) = self.master.write().take() {
            m.zeroize();
        }
        if let Some(mut a) = self.account.write().take() {
            a.zeroize();
        }
        // path 保持不变，下次解锁同一文件时仍可用；如需清空可调 clear_path
    }

    /// 获取当前分类列表
    pub fn roles(&self) -> AppResult<Vec<String>> {
        let guard = self.data.read();
        let data = guard.as_ref().ok_or(AppError::Locked)?;
        Ok(data.roles.clone())
    }

    /// 快照当前 VaultData（加密备份用）
    pub fn snapshot(&self) -> AppResult<VaultData> {
        let guard = self.data.read();
        let data = guard.as_ref().ok_or(AppError::Locked)?;
        Ok(data.clone())
    }

    /// 以给定 VaultData 完全替换当前（恢复备份用）
    pub fn replace(&self, mut data: VaultData) -> AppResult<()> {
        data.upgrade();
        data.utime = crate::models::now_ts();
        {
            let mut guard = self.data.write();
            *guard = Some(data);
        }
        self.persist_with_cached()
    }

    // ========== 内部方法（子模块可见） ==========

    fn backup_dir(&self) -> AppResult<PathBuf> {
        let path = self
            .path
            .read()
            .clone()
            .ok_or_else(|| AppError::Other("未指定密码库路径".into()))?;
        Ok(path
            .parent()
            .unwrap_or(Path::new("."))
            .join(BACKUP_DIR_NAME))
    }

    fn atomic_write(path: &Path, bytes: &[u8]) -> AppResult<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        // 简单写入即可（Tauri 单进程；后续如需要原子可换 NamedTempFile + rename）
        fs::write(path, bytes)?;
        Ok(())
    }

    fn persist_with_cached(&self) -> AppResult<()> {
        let account = self.account.read().clone().ok_or(AppError::Locked)?;
        let master = self.master.read().clone().ok_or(AppError::Locked)?;
        let master_str = std::str::from_utf8(&master)
            .map_err(|e| AppError::Crypto(format!("master utf8: {e}")))?;
        let path = self
            .path
            .read()
            .clone()
            .ok_or_else(|| AppError::Other("未指定密码库路径".into()))?;
        let bytes = {
            let guard = self.data.read();
            let data = guard.as_ref().ok_or(AppError::Locked)?;
            serde_json::to_vec(data)?
        };
        let blob = crypto_seal(&account, master_str, &bytes)?;
        Self::atomic_write(&path, &blob)?;
        Ok(())
    }

    fn create_rekey_backup(&self) -> AppResult<String> {
        let account = self.account.read().clone().ok_or(AppError::Locked)?;
        let master = self.master.read().clone().ok_or(AppError::Locked)?;
        let master_str = std::str::from_utf8(&master)
            .map_err(|e| AppError::Crypto(format!("master utf8: {e}")))?;
        let data = self.snapshot()?;
        let bytes = serde_json::to_vec(&data)?;
        let blob = crypto_seal(&account, master_str, &bytes)?;

        let dir = self.backup_dir()?;
        fs::create_dir_all(&dir)?;

        let ts = chrono::Local::now().format("%Y%m%d_%H%M%S");
        let name = format!("rekey_{ts}.{BACKUP_EXT}");
        let path = dir.join(&name);
        fs::write(&path, blob)?;
        Ok(name)
    }
}

impl Default for VaultState {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for VaultState {
    fn drop(&mut self) {
        self.lock();
    }
}
