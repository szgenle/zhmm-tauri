//! 数据模型：与 Python 版 JSON Schema 完全互通
//!
//! 顶层 `{ data, roles, utime }`；条目字段沿用 Python 版命名（`userID`/`pwd`/`desc`/`utime`）。
//! Rust 内部 snake_case，serde 在序列化层映射到 Python 风格字段名。

use serde::{Deserialize, Serialize};
use zeroize::Zeroize;

pub const DEFAULT_ROLE: &str = "个人";
pub const DEFAULT_ROLES: &[&str] = &["个人", "工作", "其它"];

pub const TAG_MAX_LEN: usize = 32;
pub const TAGS_MAX_COUNT: usize = 16;
pub const HISTORY_MAX: usize = 5;

/// TOTP 算法限制
#[allow(dead_code)]
pub const SUPPORTED_TOTP_ALGOS: &[&str] = &["", "SHA1", "SHA256", "SHA512", "SM3"];

/// 当前秒级 UNIX 时间戳
pub fn now_ts() -> i64 {
    chrono::Utc::now().timestamp()
}

/// 同条目内的一条历史密码（pwd/utime 与 Python 版字段名一致）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasswordHistoryItem {
    #[serde(default)]
    pub pwd: String,
    #[serde(default)]
    pub utime: i64,
}

impl PasswordHistoryItem {
    pub fn new(pwd: String) -> Self {
        Self {
            pwd,
            utime: now_ts(),
        }
    }
}

/// 密码条目；Rust 内部 snake_case，落盘字段对齐 Python（`userID`/`pwd`/`desc`/`utime`）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasswordEntry {
    /// Python 版用秒级时间戳作为 id
    #[serde(default)]
    pub id: i64,
    #[serde(default = "default_role")]
    pub role: String,
    #[serde(rename = "userID", default)]
    pub user_id: String,
    #[serde(default)]
    pub pwd: String,
    #[serde(default)]
    pub phone: String,
    #[serde(default)]
    pub email: String,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub desc: String,
    #[serde(default)]
    pub utime: i64,
    // TOTP 2FA
    #[serde(default)]
    pub totp_secret: String,
    #[serde(default)]
    pub totp_algo: String,
    #[serde(default = "default_totp_digits")]
    pub totp_digits: u8,
    #[serde(default = "default_totp_period")]
    pub totp_period: u32,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub history: Vec<PasswordHistoryItem>,
}

fn default_role() -> String {
    DEFAULT_ROLE.to_string()
}
fn default_totp_digits() -> u8 {
    6
}
fn default_totp_period() -> u32 {
    30
}

impl PasswordEntry {
    pub fn new() -> Self {
        let ts = now_ts();
        Self {
            id: ts,
            role: DEFAULT_ROLE.to_string(),
            user_id: String::new(),
            pwd: String::new(),
            phone: String::new(),
            email: String::new(),
            url: String::new(),
            desc: String::new(),
            utime: ts,
            totp_secret: String::new(),
            totp_algo: String::new(),
            totp_digits: 6,
            totp_period: 30,
            tags: Vec::new(),
            history: Vec::new(),
        }
    }
}

impl Default for PasswordEntry {
    fn default() -> Self {
        Self::new()
    }
}

/// 整个密码库（明文）；顶层 `{ data, roles, utime }`，对齐 Python `Vault.to_dict()`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultData {
    #[serde(rename = "data", default)]
    pub entries: Vec<PasswordEntry>,
    #[serde(default = "default_roles")]
    pub roles: Vec<String>,
    #[serde(default)]
    pub utime: i64,
}

fn default_roles() -> Vec<String> {
    DEFAULT_ROLES.iter().map(|s| s.to_string()).collect()
}

impl Default for VaultData {
    fn default() -> Self {
        Self::new()
    }
}

impl VaultData {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            roles: default_roles(),
            utime: now_ts(),
        }
    }

    /// 兼容旧版/Python 旧库：补默认 roles，把 entry.role 中不存在的 role 加入 roles 列表
    pub fn upgrade(&mut self) {
        if self.roles.is_empty() {
            self.roles = default_roles();
        }
        let mut existing: std::collections::HashSet<String> = self.roles.iter().cloned().collect();
        for e in &self.entries {
            if !e.role.is_empty() && !existing.contains(&e.role) {
                self.roles.push(e.role.clone());
                existing.insert(e.role.clone());
            }
        }
    }

    /// 生成一个不与现有条目冲突的 id（秒级时间戳，撞了就 +1）
    pub fn next_id(&self) -> i64 {
        let mut id = now_ts();
        let existing: std::collections::HashSet<i64> = self.entries.iter().map(|e| e.id).collect();
        while existing.contains(&id) {
            id = id.wrapping_add(1);
        }
        id
    }
}

/// 列表展示的轻量视图；不含密码 / TOTP secret / desc / history
#[derive(Debug, Clone, Serialize)]
pub struct PasswordSummary {
    pub id: i64,
    pub role: String,
    #[serde(rename = "userID")]
    pub user_id: String,
    pub phone: String,
    pub email: String,
    pub url: String,
    pub tags: Vec<String>,
    pub has_totp: bool,
    pub utime: i64,
}

impl From<&PasswordEntry> for PasswordSummary {
    fn from(e: &PasswordEntry) -> Self {
        Self {
            id: e.id,
            role: e.role.clone(),
            user_id: e.user_id.clone(),
            phone: e.phone.clone(),
            email: e.email.clone(),
            url: e.url.clone(),
            tags: e.tags.clone(),
            has_totp: !e.totp_secret.is_empty(),
            utime: e.utime,
        }
    }
}

/// 前端提交的新增/编辑入参
#[derive(Debug, Clone, Deserialize)]
pub struct PasswordInput {
    #[serde(default = "default_role")]
    pub role: String,
    #[serde(rename = "userID", default)]
    pub user_id: String,
    #[serde(default)]
    pub pwd: String,
    #[serde(default)]
    pub phone: String,
    #[serde(default)]
    pub email: String,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub desc: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub totp_secret: String,
    #[serde(default)]
    pub totp_algo: String,
    #[serde(default = "default_totp_digits")]
    pub totp_digits: u8,
    #[serde(default = "default_totp_period")]
    pub totp_period: u32,
}

impl Drop for PasswordInput {
    fn drop(&mut self) {
        self.pwd.zeroize();
        self.totp_secret.zeroize();
    }
}

/// 标签归一化：去空、去重、单个截 32、总数截 16
pub fn normalize_tags(raw: &[String]) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for t in raw {
        let t = t.trim();
        if t.is_empty() {
            continue;
        }
        let s: String = t.chars().take(TAG_MAX_LEN).collect();
        if seen.insert(s.clone()) {
            out.push(s);
            if out.len() >= TAGS_MAX_COUNT {
                break;
            }
        }
    }
    out
}
