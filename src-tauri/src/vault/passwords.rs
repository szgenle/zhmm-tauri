//! 密码条目相关方法：list / get / add / update / history / remove
//! / extend_entries / rollback_password

use crate::errors::{AppError, AppResult};
use crate::models::{
    normalize_custom_fields, normalize_tags, now_ts, PasswordEntry, PasswordHistoryItem,
    PasswordInput, HISTORY_MAX,
};

use super::VaultState;

impl VaultState {
    /// 列出所有条目（轻量视图）
    pub fn list(&self) -> AppResult<Vec<crate::models::PasswordSummary>> {
        let guard = self.data.read();
        let data = guard.as_ref().ok_or(AppError::Locked)?;
        Ok(data.entries.iter().map(Into::into).collect())
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
        entry.name = std::mem::take(&mut input.name);
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
        entry.template_id = std::mem::take(&mut input.template_id);
        entry.custom_fields = normalize_custom_fields(std::mem::take(&mut input.custom_fields));

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
            entry.name = std::mem::take(&mut input.name);
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
            entry.template_id = std::mem::take(&mut input.template_id);
            entry.custom_fields = normalize_custom_fields(std::mem::take(&mut input.custom_fields));
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

    /// 历史密码回滚：把 history[history_index] 提为当前密码，当前密码下沉到 history[0]
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

    /// 批量给指定条目追加标签（已有则跳过），返回实际修改的条目数
    pub fn batch_add_tag(&self, ids: &[i64], tag: &str) -> AppResult<usize> {
        let tag = tag.trim().to_string();
        if tag.is_empty() {
            return Ok(0);
        }
        let mut count = 0usize;
        {
            let mut guard = self.data.write();
            let data = guard.as_mut().ok_or(AppError::Locked)?;
            for entry in data.entries.iter_mut() {
                if ids.contains(&entry.id) && !entry.tags.contains(&tag) {
                    entry.tags.push(tag.clone());
                    entry.utime = now_ts();
                    count += 1;
                }
            }
            if count > 0 {
                data.utime = now_ts();
            }
        }
        if count > 0 {
            self.persist_with_cached()?;
        }
        Ok(count)
    }

    /// 收集密码库中所有条目的 URL host 及其标签（同 host 多条目标签合并），用于导出词典时补充
    pub fn collect_url_hosts(&self) -> AppResult<Vec<(String, Vec<String>)>> {
        let guard = self.data.read();
        let data = guard.as_ref().ok_or(AppError::Locked)?;
        let mut host_tags: std::collections::HashMap<String, std::collections::BTreeSet<String>> =
            std::collections::HashMap::new();
        for entry in &data.entries {
            let url_str = entry.url.trim();
            if url_str.is_empty() {
                continue;
            }
            let host = extract_host_from_url(url_str);
            if !host.is_empty() {
                let tag_set = host_tags.entry(host).or_default();
                for tag in &entry.tags {
                    let t = tag.trim().to_string();
                    if !t.is_empty() {
                        tag_set.insert(t);
                    }
                }
            }
        }
        let mut result: Vec<(String, Vec<String>)> = host_tags
            .into_iter()
            .map(|(h, ts)| (h, ts.into_iter().collect()))
            .collect();
        result.sort_by(|a, b| a.0.cmp(&b.0));
        Ok(result)
    }
}

/// 从 URL 字符串提取纯 host（小写）
fn extract_host_from_url(input: &str) -> String {
    let s = input.trim().to_lowercase();
    if s.is_empty() {
        return String::new();
    }
    if let Ok(url) = url::Url::parse(&s) {
        return url.host_str().unwrap_or("").to_string();
    }
    if let Ok(url) = url::Url::parse(&format!("https://{s}")) {
        return url.host_str().unwrap_or("").to_string();
    }
    s.split('/')
        .next()
        .unwrap_or("")
        .split(':')
        .next()
        .unwrap_or("")
        .to_string()
}
