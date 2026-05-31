//! 本地备份方法：create_local_backup / list_local_backups / delete_local_backup
//! / restore_local_backup / cleanup_backups / backup_using_master

use std::fs;
use std::path::Path;

use crate::crypto::{open as crypto_open, seal as crypto_seal};
use crate::errors::{AppError, AppResult};
use crate::models::VaultData;

use super::{BackupInfo, VaultState, BACKUP_EXT, BACKUP_EXT_LEGACY};

impl VaultState {
    pub fn create_local_backup(&self) -> AppResult<String> {
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
        let name = format!("backup_{ts}.{BACKUP_EXT}");
        let path = dir.join(&name);
        fs::write(&path, blob)?;
        Ok(name)
    }

    pub fn list_local_backups(&self) -> AppResult<Vec<BackupInfo>> {
        let dir = match self.backup_dir() {
            Ok(d) => d,
            Err(_) => return Ok(Vec::new()),
        };
        if !dir.exists() {
            return Ok(Vec::new());
        }
        let mut items: Vec<BackupInfo> = Vec::new();
        for entry in fs::read_dir(&dir)? {
            let entry = entry?;
            let path = entry.path();
            // 同时接受新后缀 .ajot 与历史后缀 .zhmm
            let ext = path.extension().and_then(|e| e.to_str());
            if ext != Some(BACKUP_EXT) && ext != Some(BACKUP_EXT_LEGACY) {
                continue;
            }
            let meta = entry.metadata()?;
            let name = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            let created_at = meta
                .modified()
                .or_else(|_| meta.created())
                .map(|t| {
                    let dt: chrono::DateTime<chrono::Local> = t.into();
                    dt.format("%Y-%m-%d %H:%M:%S").to_string()
                })
                .unwrap_or_default();
            items.push(BackupInfo {
                name,
                size: meta.len(),
                created_at,
            });
        }
        items.sort_by(|a, b| b.name.cmp(&a.name));
        Ok(items)
    }

    pub fn delete_local_backup(&self, name: &str) -> AppResult<()> {
        let path = self.backup_dir()?.join(name);
        if !path.exists() {
            return Err(AppError::NotFound);
        }
        fs::remove_file(&path)?;
        Ok(())
    }

    pub fn restore_local_backup(&self, name: &str) -> AppResult<()> {
        let account = self.account.read().clone().ok_or(AppError::Locked)?;
        let master = self.master.read().clone().ok_or(AppError::Locked)?;
        let master_str = std::str::from_utf8(&master)
            .map_err(|e| AppError::Crypto(format!("master utf8: {e}")))?;
        let path = self.backup_dir()?.join(name);
        if !path.exists() {
            return Err(AppError::Other(format!("备份不存在: {name}")));
        }
        let bytes = fs::read(&path)?;
        let plain = crypto_open(&account, master_str, &bytes)?;
        let mut data: VaultData = serde_json::from_slice(&plain)
            .map_err(|e| AppError::Crypto(format!("备份 json: {e}")))?;
        data.upgrade();
        {
            let mut guard = self.data.write();
            *guard = Some(data);
        }
        self.persist_with_cached()
    }

    pub fn cleanup_backups(&self, keep: usize) -> AppResult<u32> {
        let items = self.list_local_backups()?;
        if items.len() <= keep {
            return Ok(0);
        }
        let dir = self.backup_dir()?;
        let mut removed = 0u32;
        for item in items.iter().skip(keep) {
            let path = dir.join(&item.name);
            if fs::remove_file(&path).is_ok() {
                removed += 1;
            }
        }
        Ok(removed)
    }

    /// 用当前缓存的主密码加密导出备份（备份场景的便捷调用）
    ///
    /// 与 `io_json::backup_to_file` 共用同一加密格式；调用方无需显式传入备份密码。
    /// 备份后的文件可通过 `restore_from_file(path, <主密码>)` 解锁恢复。
    pub fn backup_using_master(&self, path: &Path) -> AppResult<()> {
        let master = self.master.read().clone().ok_or(AppError::Locked)?;
        let master_str = std::str::from_utf8(&master)
            .map_err(|e| AppError::Crypto(format!("master utf8: {e}")))?;
        let snapshot = self.snapshot()?;
        crate::io_json::backup_to_file(path, &snapshot, master_str)
    }
}
