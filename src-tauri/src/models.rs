//! 数据模型定义（Task 1：完整对齐 Python 版）

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use zeroize::Zeroize;

pub const DEFAULT_ROLE: &str = "个人";
pub const DEFAULT_ROLES: &[&str] = &["个人", "工作", "其它"];

pub const TAG_MAX_LEN: usize = 32;
pub const TAGS_MAX_COUNT: usize = 16;
pub const HISTORY_MAX: usize = 5;

pub const VAULT_VERSION: u32 = 2;

/// TOTP 算法限制
#[allow(dead_code)]
pub const SUPPORTED_TOTP_ALGOS: &[&str] = &["", "SHA1", "SHA256", "SHA512", "SM3"];

/// 同条目内的一条历史密码
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasswordHistoryItem {
    pub pwd: String,
    /// 旧密码被替换的时间
    pub replaced_at: DateTime<Utc>,
}

impl PasswordHistoryItem {
    pub fn new(pwd: String) -> Self {
        Self {
            pwd,
            replaced_at: Utc::now(),
        }
    }
}

/// 密码条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasswordEntry {
    pub id: String,
    pub title: String,
    #[serde(default = "default_role")]
    pub role: String,
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub password: String,
    #[serde(default)]
    pub phone: String,
    #[serde(default)]
    pub email: String,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    pub tags: Vec<String>,
    // TOTP 2FA
    #[serde(default)]
    pub totp_secret: String,
    #[serde(default)]
    pub totp_algo: String,
    #[serde(default = "default_totp_digits")]
    pub totp_digits: u8,
    #[serde(default = "default_totp_period")]
    pub totp_period: u32,
    // 历史密码
    #[serde(default)]
    pub history: Vec<PasswordHistoryItem>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
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
    pub fn new(title: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            title: title.into(),
            role: DEFAULT_ROLE.to_string(),
            username: String::new(),
            password: String::new(),
            phone: String::new(),
            email: String::new(),
            url: String::new(),
            notes: String::new(),
            tags: Vec::new(),
            totp_secret: String::new(),
            totp_algo: String::new(),
            totp_digits: 6,
            totp_period: 30,
            history: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }
}

/// 整个密码库（明文）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultData {
    pub version: u32,
    #[serde(default = "default_roles")]
    pub roles: Vec<String>,
    pub entries: Vec<PasswordEntry>,
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
            version: VAULT_VERSION,
            roles: default_roles(),
            entries: Vec::new(),
        }
    }

    /// 兼容旧版本：v1 没有 roles 字段，加载后补默认
    pub fn upgrade(&mut self) {
        if self.roles.is_empty() {
            self.roles = default_roles();
        }
        // 把 entry.role 中不在 roles 列表的补进去
        let mut existing: std::collections::HashSet<String> = self.roles.iter().cloned().collect();
        for e in &self.entries {
            if !e.role.is_empty() && !existing.contains(&e.role) {
                self.roles.push(e.role.clone());
                existing.insert(e.role.clone());
            }
        }
        self.version = VAULT_VERSION;
    }
}

/// 列表展示的轻量视图，不含密码 / TOTP secret / notes
#[derive(Debug, Clone, Serialize)]
pub struct PasswordSummary {
    pub id: String,
    pub title: String,
    pub role: String,
    pub username: String,
    pub phone: String,
    pub email: String,
    pub url: String,
    pub tags: Vec<String>,
    pub has_totp: bool,
    pub updated_at: DateTime<Utc>,
}

impl From<&PasswordEntry> for PasswordSummary {
    fn from(e: &PasswordEntry) -> Self {
        Self {
            id: e.id.clone(),
            title: e.title.clone(),
            role: e.role.clone(),
            username: e.username.clone(),
            phone: e.phone.clone(),
            email: e.email.clone(),
            url: e.url.clone(),
            tags: e.tags.clone(),
            has_totp: !e.totp_secret.is_empty(),
            updated_at: e.updated_at,
        }
    }
}

/// 前端提交的新增/编辑入参
#[derive(Debug, Clone, Deserialize)]
pub struct PasswordInput {
    pub title: String,
    #[serde(default = "default_role")]
    pub role: String,
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub password: String,
    #[serde(default)]
    pub phone: String,
    #[serde(default)]
    pub email: String,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub notes: String,
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
        self.password.zeroize();
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
