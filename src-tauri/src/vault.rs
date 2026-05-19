//! 密码库状态与文件持久化
//!
//! 解锁后的明文驻留内存，主密码缓存为字节，Drop 时 zeroize。

use parking_lot::RwLock;
use std::fs;
use std::path::PathBuf;
use zeroize::Zeroize;

use crate::crypto::{decrypt, encrypt, Envelope};
use crate::errors::{AppError, AppResult};
use crate::models::{normalize_tags, PasswordEntry, PasswordHistoryItem, PasswordInput, VaultData, HISTORY_MAX};

pub struct VaultState {
    /// 密码库文件路径
    path: RwLock<PathBuf>,
    /// 解锁后的明文密码库
    data: RwLock<Option<VaultData>>,
    /// 缓存主密码字节，Drop 时 zeroize
    master: RwLock<Option<Vec<u8>>>,
}

impl VaultState {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path: RwLock::new(path),
            data: RwLock::new(None),
            master: RwLock::new(None),
        }
    }

    pub fn path(&self) -> PathBuf {
        self.path.read().clone()
    }

    pub fn is_unlocked(&self) -> bool {
        self.data.read().is_some()
    }

    pub fn vault_exists(&self) -> bool {
        self.path.read().exists()
    }

    /// 创建新密码库（路径不存在时）
    pub fn create(&self, master_password: &str) -> AppResult<()> {
        if self.vault_exists() {
            return Err(AppError::Other("密码库已存在".into()));
        }
        let data = VaultData::new();
        let bytes = serde_json::to_vec(&data)?;
        let env = encrypt(&bytes, master_password.as_bytes())?;
        self.write_envelope(&env)?;

        *self.data.write() = Some(data);
        *self.master.write() = Some(master_password.as_bytes().to_vec());
        Ok(())
    }

    /// 解锁现有密码库
    pub fn unlock(&self, master_password: &str) -> AppResult<()> {
        let bytes = fs::read(&*self.path.read())?;
        let env = Envelope::from_bytes(&bytes)?;
        let plain = decrypt(&env, master_password.as_bytes())?;
        let mut data: VaultData = serde_json::from_slice(&plain)
            .map_err(|e| AppError::Crypto(format!("vault json: {e}")))?;
        data.upgrade();

        *self.data.write() = Some(data);
        *self.master.write() = Some(master_password.as_bytes().to_vec());
        Ok(())
    }

    /// 锁定：清空内存中的明文与主密码
    pub fn lock(&self) {
        if let Some(mut data) = self.data.write().take() {
            for e in &mut data.entries {
                e.password.zeroize();
                e.notes.zeroize();
                e.totp_secret.zeroize();
                for h in &mut e.history {
                    h.pwd.zeroize();
                }
            }
        }
        if let Some(mut m) = self.master.write().take() {
            m.zeroize();
        }
    }

    /// 列出所有条目（轻量视图）
    pub fn list(&self) -> AppResult<Vec<crate::models::PasswordSummary>> {
        let guard = self.data.read();
        let data = guard.as_ref().ok_or(AppError::Locked)?;
        Ok(data.entries.iter().map(Into::into).collect())
    }

    /// 取完整条目
    pub fn get(&self, id: &str) -> AppResult<PasswordEntry> {
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
        let mut entry = PasswordEntry::new(std::mem::take(&mut input.title));
        if !input.role.is_empty() {
            entry.role = std::mem::take(&mut input.role);
        }
        entry.username = std::mem::take(&mut input.username);
        entry.password = std::mem::take(&mut input.password);
        entry.phone = std::mem::take(&mut input.phone);
        entry.email = std::mem::take(&mut input.email);
        entry.url = std::mem::take(&mut input.url);
        entry.notes = std::mem::take(&mut input.notes);
        entry.tags = normalize_tags(&std::mem::take(&mut input.tags));
        entry.totp_secret = std::mem::take(&mut input.totp_secret);
        entry.totp_algo = std::mem::take(&mut input.totp_algo);
        entry.totp_digits = input.totp_digits;
        entry.totp_period = input.totp_period;

        {
            let mut guard = self.data.write();
            let data = guard.as_mut().ok_or(AppError::Locked)?;
            // 同步登记 role
            if !entry.role.is_empty() && !data.roles.iter().any(|r| r == &entry.role) {
                data.roles.push(entry.role.clone());
            }
            data.entries.push(entry.clone());
        }
        self.persist_with_cached_master()?;
        Ok(entry)
    }

    /// 更新条目；若 password 变动，旧密码压入 history
    pub fn update(&self, id: &str, mut input: PasswordInput) -> AppResult<PasswordEntry> {
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
            let new_pwd = std::mem::take(&mut input.password);
            if !entry.password.is_empty() && new_pwd != entry.password {
                let old = std::mem::take(&mut entry.password);
                entry.history.insert(0, PasswordHistoryItem::new(old));
                if entry.history.len() > HISTORY_MAX {
                    // 叠后尾位都 zeroize 一下
                    for h in entry.history.drain(HISTORY_MAX..) {
                        let mut p = h.pwd;
                        zeroize::Zeroize::zeroize(&mut p);
                    }
                }
            }
            entry.password = new_pwd;

            entry.title = std::mem::take(&mut input.title);
            if !input.role.is_empty() {
                entry.role = std::mem::take(&mut input.role);
            }
            entry.username = std::mem::take(&mut input.username);
            entry.phone = std::mem::take(&mut input.phone);
            entry.email = std::mem::take(&mut input.email);
            entry.url = std::mem::take(&mut input.url);
            entry.notes = std::mem::take(&mut input.notes);
            entry.tags = normalize_tags(&std::mem::take(&mut input.tags));
            entry.totp_secret = std::mem::take(&mut input.totp_secret);
            entry.totp_algo = std::mem::take(&mut input.totp_algo);
            entry.totp_digits = input.totp_digits;
            entry.totp_period = input.totp_period;
            entry.updated_at = chrono::Utc::now();

            // 同步登记 role
            if !entry.role.is_empty() && !data.roles.iter().any(|r| r == &entry.role) {
                data.roles.push(entry.role.clone());
            }
            updated = entry.clone();
        }
        self.persist_with_cached_master()?;
        Ok(updated)
    }

    /// 取历史密码列表
    pub fn history(&self, id: &str) -> AppResult<Vec<PasswordHistoryItem>> {
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
    pub fn remove(&self, id: &str) -> AppResult<()> {
        {
            let mut guard = self.data.write();
            let data = guard.as_mut().ok_or(AppError::Locked)?;
            let before = data.entries.len();
            data.entries.retain(|e| e.id != id);
            if data.entries.len() == before {
                return Err(AppError::NotFound);
            }
        }
        self.persist_with_cached_master()
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
        // 重生成 id 这里不做——备份本身保留 id
        {
            let mut guard = self.data.write();
            *guard = Some(data);
        }
        self.persist_with_cached_master()
    }

    /// 追加一批条目（导入 xlsx 用，id 重生成避免冲突）
    pub fn extend_entries(&self, mut entries: Vec<PasswordEntry>) -> AppResult<usize> {
        let count = entries.len();
        {
            let mut guard = self.data.write();
            let data = guard.as_mut().ok_or(AppError::Locked)?;
            for e in entries.iter_mut() {
                // 重生 UUID、保证唯一
                e.id = uuid::Uuid::new_v4().to_string();
                if e.role.is_empty() {
                    e.role = crate::models::DEFAULT_ROLE.to_string();
                }
                if !data.roles.iter().any(|r| r == &e.role) {
                    data.roles.push(e.role.clone());
                }
            }
            data.entries.extend(entries);
        }
        self.persist_with_cached_master()?;
        Ok(count)
    }

    fn write_envelope(&self, env: &Envelope) -> AppResult<()> {
        let path = self.path.read().clone();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&path, env.to_bytes())?;
        Ok(())
    }

    fn persist_with_cached_master(&self) -> AppResult<()> {
        let master = self.master.read().clone().ok_or(AppError::Locked)?;
        let bytes = {
            let guard = self.data.read();
            let data = guard.as_ref().ok_or(AppError::Locked)?;
            serde_json::to_vec(data)?
        };
        let env = encrypt(&bytes, &master)?;
        self.write_envelope(&env)?;
        Ok(())
    }
}

impl Drop for VaultState {
    fn drop(&mut self) {
        self.lock();
    }
}
