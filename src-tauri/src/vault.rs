//! 密码库状态与文件持久化
//!
//! 解锁后的明文驻留内存，主密码与账号缓存为字节，Drop 时 zeroize。
//! 路径不再固定，由前端通过 set_active_path / unlock_with_path 指定。

use parking_lot::RwLock;
use std::fs;
use std::path::{Path, PathBuf};
use zeroize::Zeroize;

use crate::crypto::{open as crypto_open, seal as crypto_seal};
use crate::errors::{AppError, AppResult};
use crate::models::{
    normalize_tags, now_ts, PasswordEntry, PasswordHistoryItem, PasswordInput, VaultData,
    HISTORY_MAX,
};

/// 备份条目元信息（返回给前端）
#[derive(Debug, Clone, serde::Serialize)]
pub struct BackupInfo {
    pub name: String,
    pub size: u64,
    pub created_at: String,
}

const BACKUP_DIR_NAME: &str = ".backups";
const BACKUP_EXT: &str = "zhmm";

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
        let data = VaultData::new();
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

    /// 列出所有条目（轻量视图）
    pub fn list(&self) -> AppResult<Vec<crate::models::PasswordSummary>> {
        let guard = self.data.read();
        let data = guard.as_ref().ok_or(AppError::Locked)?;
        Ok(data.entries.iter().map(Into::into).collect())
    }

    /// 获取当前分类列表
    pub fn roles(&self) -> AppResult<Vec<String>> {
        let guard = self.data.read();
        let data = guard.as_ref().ok_or(AppError::Locked)?;
        Ok(data.roles.clone())
    }

    /// 取完整条目
    pub fn get(&self, id: i64) -> AppResult<PasswordEntry> {
        let guard = self.data.read();
        let data = guard.as_ref().ok_or(AppError::Locked)?;
        data.entries
            .iter()
            .find(|e| e.id == id)
            .cloned()
            .ok_or(AppError::NotFound)
    }

    /// 添加条目，返回入库后的完整条目
    pub fn add(&self, mut input: PasswordInput) -> AppResult<PasswordEntry> {
        let mut entry = PasswordEntry::new();
        if !input.role.is_empty() {
            entry.role = std::mem::take(&mut input.role);
        }
        entry.user_id = std::mem::take(&mut input.user_id);
        entry.pwd = std::mem::take(&mut input.pwd);
        entry.phone = std::mem::take(&mut input.phone);
        entry.email = std::mem::take(&mut input.email);
        entry.url = std::mem::take(&mut input.url);
        entry.desc = std::mem::take(&mut input.desc);
        entry.tags = normalize_tags(&std::mem::take(&mut input.tags));
        entry.totp_secret = std::mem::take(&mut input.totp_secret);
        entry.totp_algo = std::mem::take(&mut input.totp_algo);
        entry.totp_digits = input.totp_digits;
        entry.totp_period = input.totp_period;

        {
            let mut guard = self.data.write();
            let data = guard.as_mut().ok_or(AppError::Locked)?;
            entry.id = data.next_id();
            entry.utime = now_ts();
            // 同步登记 role
            if !entry.role.is_empty() && !data.roles.iter().any(|r| r == &entry.role) {
                data.roles.push(entry.role.clone());
            }
            data.entries.push(entry.clone());
            data.utime = now_ts();
        }
        self.persist_with_cached()?;
        Ok(entry)
    }

    /// 更新条目；若 pwd 变动，旧密码压入 history
    pub fn update(&self, id: i64, mut input: PasswordInput) -> AppResult<PasswordEntry> {
        let updated;
        {
            let mut guard = self.data.write();
            let data = guard.as_mut().ok_or(AppError::Locked)?;
            let entry = data
                .entries
                .iter_mut()
                .find(|e| e.id == id)
                .ok_or(AppError::NotFound)?;

            // 检测密码变化 -> 旧密码压入历史
            let new_pwd = std::mem::take(&mut input.pwd);
            if !entry.pwd.is_empty() && new_pwd != entry.pwd {
                let old = std::mem::take(&mut entry.pwd);
                entry.history.insert(0, PasswordHistoryItem::new(old));
                if entry.history.len() > HISTORY_MAX {
                    for h in entry.history.drain(HISTORY_MAX..) {
                        let mut p = h.pwd;
                        zeroize::Zeroize::zeroize(&mut p);
                    }
                }
            }
            entry.pwd = new_pwd;

            if !input.role.is_empty() {
                entry.role = std::mem::take(&mut input.role);
            }
            entry.user_id = std::mem::take(&mut input.user_id);
            entry.phone = std::mem::take(&mut input.phone);
            entry.email = std::mem::take(&mut input.email);
            entry.url = std::mem::take(&mut input.url);
            entry.desc = std::mem::take(&mut input.desc);
            entry.tags = normalize_tags(&std::mem::take(&mut input.tags));
            entry.totp_secret = std::mem::take(&mut input.totp_secret);
            entry.totp_algo = std::mem::take(&mut input.totp_algo);
            entry.totp_digits = input.totp_digits;
            entry.totp_period = input.totp_period;
            entry.utime = now_ts();

            if !entry.role.is_empty() && !data.roles.iter().any(|r| r == &entry.role) {
                data.roles.push(entry.role.clone());
            }
            updated = entry.clone();
            data.utime = now_ts();
        }
        self.persist_with_cached()?;
        Ok(updated)
    }

    /// 取历史密码列表
    pub fn history(&self, id: i64) -> AppResult<Vec<PasswordHistoryItem>> {
        let guard = self.data.read();
        let data = guard.as_ref().ok_or(AppError::Locked)?;
        let entry = data
            .entries
            .iter()
            .find(|e| e.id == id)
            .ok_or(AppError::NotFound)?;
        Ok(entry.history.clone())
    }

    /// 删除条目
    pub fn remove(&self, id: i64) -> AppResult<()> {
        {
            let mut guard = self.data.write();
            let data = guard.as_mut().ok_or(AppError::Locked)?;
            let before = data.entries.len();
            data.entries.retain(|e| e.id != id);
            if data.entries.len() == before {
                return Err(AppError::NotFound);
            }
            data.utime = now_ts();
        }
        self.persist_with_cached()
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
        data.utime = now_ts();
        {
            let mut guard = self.data.write();
            *guard = Some(data);
        }
        self.persist_with_cached()
    }

    /// 追加一批条目（导入 xlsx 用，id 重生成避免冲突）
    pub fn extend_entries(&self, mut entries: Vec<PasswordEntry>) -> AppResult<usize> {
        let count = entries.len();
        {
            let mut guard = self.data.write();
            let data = guard.as_mut().ok_or(AppError::Locked)?;
            for e in entries.iter_mut() {
                // 重新分配 id 保证唯一
                e.id = data.next_id();
                e.utime = now_ts();
                if e.role.is_empty() {
                    e.role = crate::models::DEFAULT_ROLE.to_string();
                }
                if !data.roles.iter().any(|r| r == &e.role) {
                    data.roles.push(e.role.clone());
                }
                data.entries.push(e.clone());
            }
            data.utime = now_ts();
        }
        self.persist_with_cached()?;
        Ok(count)
    }

    // ========== 标签管理 ==========

    pub fn collect_tag_counts(&self) -> AppResult<Vec<(String, usize)>> {
        let guard = self.data.read();
        let data = guard.as_ref().ok_or(AppError::Locked)?;
        let mut counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
        for entry in &data.entries {
            for tag in &entry.tags {
                if !tag.is_empty() {
                    *counts.entry(tag.clone()).or_insert(0) += 1;
                }
            }
        }
        let mut result: Vec<(String, usize)> = counts.into_iter().collect();
        result.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        Ok(result)
    }

    pub fn rename_tag(&self, old: &str, new: &str) -> AppResult<usize> {
        let old_n = old.trim();
        let new_n = new.trim();
        if old_n.is_empty() || new_n.is_empty() || old_n == new_n {
            return Ok(0);
        }
        let mut affected = 0;
        {
            let mut guard = self.data.write();
            let data = guard.as_mut().ok_or(AppError::Locked)?;
            for entry in &mut data.entries {
                if entry.tags.contains(&old_n.to_string()) {
                    entry.tags = entry
                        .tags
                        .iter()
                        .map(|t| {
                            if t == old_n {
                                new_n.to_string()
                            } else {
                                t.clone()
                            }
                        })
                        .collect();
                    entry.tags = normalize_tags(&entry.tags);
                    entry.utime = now_ts();
                    affected += 1;
                }
            }
            if affected > 0 {
                data.utime = now_ts();
            }
        }
        if affected > 0 {
            self.persist_with_cached()?;
        }
        Ok(affected)
    }

    pub fn delete_tag(&self, tag: &str) -> AppResult<usize> {
        let target = tag.trim();
        if target.is_empty() {
            return Ok(0);
        }
        let mut affected = 0;
        {
            let mut guard = self.data.write();
            let data = guard.as_mut().ok_or(AppError::Locked)?;
            for entry in &mut data.entries {
                if entry.tags.contains(&target.to_string()) {
                    entry.tags.retain(|t| t != target);
                    entry.utime = now_ts();
                    affected += 1;
                }
            }
            if affected > 0 {
                data.utime = now_ts();
            }
        }
        if affected > 0 {
            self.persist_with_cached()?;
        }
        Ok(affected)
    }

    // ========== 密码历史回滚 ==========

    pub fn rollback_password(&self, id: i64, history_index: usize) -> AppResult<PasswordEntry> {
        let updated;
        {
            let mut guard = self.data.write();
            let data = guard.as_mut().ok_or(AppError::Locked)?;
            let entry = data
                .entries
                .iter_mut()
                .find(|e| e.id == id)
                .ok_or(AppError::NotFound)?;
            if history_index >= entry.history.len() {
                return Err(AppError::Invalid("历史索引超出范围".into()));
            }
            let target = entry.history.remove(history_index);
            let current_pwd = std::mem::take(&mut entry.pwd);
            if !current_pwd.is_empty() {
                entry
                    .history
                    .insert(0, PasswordHistoryItem::new(current_pwd));
            }
            if entry.history.len() > HISTORY_MAX {
                for h in entry.history.drain(HISTORY_MAX..) {
                    let mut p = h.pwd;
                    zeroize::Zeroize::zeroize(&mut p);
                }
            }
            entry.pwd = target.pwd;
            entry.utime = now_ts();
            updated = entry.clone();
            data.utime = now_ts();
        }
        self.persist_with_cached()?;
        Ok(updated)
    }

    // ========== 本地备份管理 ==========

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
            if path.extension().and_then(|e| e.to_str()) != Some(BACKUP_EXT) {
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

    // ========== 主密码管理 ==========

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

    // ========== 内部方法 ==========

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
