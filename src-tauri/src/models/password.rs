//! 密码条目相关模型：`PasswordHistoryItem` / `PasswordEntry` / `PasswordSummary` / `PasswordInput`
//!
//! 还包含与密码条目密切相关的字段归一化函数（tags / custom_fields）。

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use zeroize::Zeroize;

use super::{
    now_ts, CUSTOM_FIELDS_MAX_COUNT, CUSTOM_FIELD_KEY_MAX, CUSTOM_FIELD_VALUE_MAX, DEFAULT_ROLE,
    TAGS_MAX_COUNT, TAG_MAX_LEN,
};

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
    /// 名称：网站名/App 名等通用主标识（与 Python 版兼容，旧库无此字段时为空）
    #[serde(default)]
    pub name: String,
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
    /// 当前条目应用的模板 id（指向 `VaultData.templates`），空串表示无模板
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub template_id: String,
    /// 扩展字段（key -> value），承载模板定义之外或之内的差异化信息
    /// - BTreeMap 保证序列化顺序稳定（避免无意义 diff）
    /// - 不区分加密/明文：v2.0 P1 不引入字段级加密，整库整体由 v7 加密
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub custom_fields: BTreeMap<String, String>,
}

pub(super) fn default_role() -> String {
    DEFAULT_ROLE.to_string()
}
pub(super) fn default_totp_digits() -> u8 {
    6
}
pub(super) fn default_totp_period() -> u32 {
    30
}

impl PasswordEntry {
    pub fn new() -> Self {
        let ts = now_ts();
        Self {
            id: ts,
            role: DEFAULT_ROLE.to_string(),
            name: String::new(),
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
            template_id: String::new(),
            custom_fields: BTreeMap::new(),
        }
    }
}

impl Default for PasswordEntry {
    fn default() -> Self {
        Self::new()
    }
}

/// 列表展示的轻量视图；不含密码 / TOTP secret / history
#[derive(Debug, Clone, Serialize)]
pub struct PasswordSummary {
    pub id: i64,
    pub role: String,
    pub name: String,
    #[serde(rename = "userID")]
    pub user_id: String,
    pub phone: String,
    pub email: String,
    pub url: String,
    pub desc: String,
    pub tags: Vec<String>,
    pub has_totp: bool,
    pub utime: i64,
    /// 当前密码生效时间：history[0].utime 表示上次密码替换的时刻；
    /// 若从未改过密码，则回退到条目创建时间 (id)。
    pub pwd_utime: i64,
    /// 关联的模板 id（v2.0+），空串=无模板
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub template_id: String,
    /// 是否含扩展字段（轻量提示，列表层无需返回完整 map）
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub has_custom_fields: bool,
}

impl From<&PasswordEntry> for PasswordSummary {
    fn from(e: &PasswordEntry) -> Self {
        let pwd_utime = e.history.first().map(|h| h.utime).unwrap_or(e.id);
        Self {
            id: e.id,
            role: e.role.clone(),
            name: e.name.clone(),
            user_id: e.user_id.clone(),
            phone: e.phone.clone(),
            email: e.email.clone(),
            url: e.url.clone(),
            desc: e.desc.clone(),
            tags: e.tags.clone(),
            has_totp: !e.totp_secret.is_empty(),
            utime: e.utime,
            pwd_utime,
            template_id: e.template_id.clone(),
            has_custom_fields: !e.custom_fields.is_empty(),
        }
    }
}

/// 前端提交的新增/编辑入参
#[derive(Debug, Clone, Deserialize)]
pub struct PasswordInput {
    #[serde(default = "default_role")]
    pub role: String,
    #[serde(default)]
    pub name: String,
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
    /// 可选模板 id，空串表示未应用模板
    #[serde(default)]
    pub template_id: String,
    /// 扩展字段（与 PasswordEntry.custom_fields 同构）
    #[serde(default)]
    pub custom_fields: BTreeMap<String, String>,
}

impl Drop for PasswordInput {
    fn drop(&mut self) {
        self.pwd.zeroize();
        self.totp_secret.zeroize();
        // custom_fields 可能包含敏感信息（身份证/口令卷等），逐项清到字节零值
        for (_, v) in self.custom_fields.iter_mut() {
            v.zeroize();
        }
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

/// 扩展字段归一化：去空 key、截断超长、差异量限数量
pub fn normalize_custom_fields(raw: BTreeMap<String, String>) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    for (k, v) in raw {
        let k = k.trim();
        if k.is_empty() {
            continue;
        }
        let key: String = k.chars().take(CUSTOM_FIELD_KEY_MAX).collect();
        let value: String = v.chars().take(CUSTOM_FIELD_VALUE_MAX).collect();
        out.insert(key, value);
        if out.len() >= CUSTOM_FIELDS_MAX_COUNT {
            break;
        }
    }
    out
}
