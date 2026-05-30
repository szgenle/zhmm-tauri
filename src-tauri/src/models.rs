//! 数据模型：账号小本本 v7 schema（兼容读取 Python 版 v6/v5 .zmb）
//!
//! 顶层 `{ data, roles, utime, templates? }`；条目沿用 Python 版命名
//! （`userID`/`pwd`/`desc`/`utime`）以保持单向导入兼容。v2.0 新增字段：
//! - `PasswordEntry.custom_fields`：扩展字段 map，承载"千差万别的账号信息"
//! - `PasswordEntry.template_id`：指向当前 vault 里的 AccountTemplate
//! - `VaultData.templates`：vault 级账号模板注册表
//!
//! 所有新增字段都带 `#[serde(default)]`，旧库（含 Python 单向导入的 .zmb）
//! 不需要任何迁移即可加载；写入时按需省略空值，避免给 v6/v5 老文件
//! 引入额外字节（虽然 v2.0 起一律写 v7，但保持紧凑仍是好习惯）。

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use zeroize::Zeroize;

pub const DEFAULT_ROLE: &str = "个人";
pub const DEFAULT_ROLES: &[&str] = &["个人", "工作", "其它"];

pub const TAG_MAX_LEN: usize = 32;
pub const TAGS_MAX_COUNT: usize = 16;
pub const HISTORY_MAX: usize = 5;

/// 单条扩展字段最大长度（key/value 各自上限，防退化为附件）
pub const CUSTOM_FIELD_KEY_MAX: usize = 64;
pub const CUSTOM_FIELD_VALUE_MAX: usize = 4096;
pub const CUSTOM_FIELDS_MAX_COUNT: usize = 64;

/// 模板 match_rules 总条数上限：超出会被裁剪（防失控配置拖慢推荐扫描）
pub const MATCH_RULES_MAX_COUNT: usize = 32;
/// match_rules 单条 value 长度上限
pub const MATCH_RULE_VALUE_MAX: usize = 128;

/// 模板包导入单批最大模板数（防爆）
pub const TEMPLATE_PACK_MAX_TEMPLATES: usize = 200;
/// 当前模板包格式标识与 schema 版本
pub const TEMPLATE_PACK_KIND: &str = "ajot.template-pack.v1";
pub const TEMPLATE_PACK_SCHEMA_VERSION: u32 = 1;

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

/// 账号库明文模型（v7 schema）；顶层 `{ data, roles, utime, templates? }`
///
/// 与 Python 版 v6 .zmb 在 `data/roles/utime` 三个字段上保持兼容（导入用），
/// `templates` 为 v2.0 新增字段，旧库反序列化时为空。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultData {
    #[serde(rename = "data", default)]
    pub entries: Vec<PasswordEntry>,
    #[serde(default = "default_roles")]
    pub roles: Vec<String>,
    #[serde(default)]
    pub utime: i64,
    /// vault 级账号模板注册表（v2.0+），旧库无此字段，反序列化得到空 Vec
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub templates: Vec<AccountTemplate>,
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
            templates: Vec::new(),
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

        // v2.0.0-rc.2：内建模板的 match_rules 在 alpha.2 才引入；
        // 老 vault（v0.x / v2.0.0-alpha.0/1）解锁后内建模板 match_rules 仍为空，
        // 导致编辑账号时模板自动推荐永远不触发。这里幂等补齐：
        //   - 仅当某内建 id 模板的 match_rules 为空时，从最新 default_templates 复制
        //   - 用户已自定义过 match_rules 的不动
        //   - 仅修改内存中的 data，不强制写盘（下次任意 mutate 时自然落盘）
        let builtins = default_templates();
        for d in builtins {
            if let Some(t) = self.templates.iter_mut().find(|t| t.id == d.id) {
                if t.match_rules.is_empty() && !d.match_rules.is_empty() {
                    t.match_rules = d.match_rules;
                }
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

/// 账号模板字段类型（v2.0 P1 最小集）
///
/// 字段表现为字符串 + UI 提示，不在序列化层区分加密。
/// `Url` / `Email` / `Phone` 仅为 UI 输入提示与点击动作提示，
/// 底层仍以 `String` 存以便跨项目复用。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TemplateFieldType {
    Text,
    Secret,
    Url,
    Email,
    Phone,
    Multiline,
    Date,
}

impl Default for TemplateFieldType {
    fn default() -> Self {
        Self::Text
    }
}

/// 账号模板中的单个字段定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateField {
    /// 字段 key，在同一模板内唯一；也是 `PasswordEntry.custom_fields` 的 key
    pub key: String,
    /// 字段展示名（UI 标题）
    pub label: String,
    #[serde(default)]
    pub field_type: TemplateFieldType,
    /// 是否必填（仅 UI 提示，不在后端强校，以不阻断导入场景）
    #[serde(default)]
    pub required: bool,
    /// 占位符提示文本
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub placeholder: String,
}

/// 模板自动匹配规则（预留）：后续用于根据 url/keyword/role 推荐模板
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind", content = "value")]
pub enum TemplateMatchRule {
    /// 域名/子串匹配条目的 url
    UrlContains(String),
    /// 名称/描述中包含关键词
    Keyword(String),
    /// role 完全匹配
    Role(String),
}

/// 账号模板：vault 级资源，序列化于 `VaultData.templates`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountTemplate {
    /// 模板唯一 id（如 `bank_card` / `social` / `work_internal`），推荐 snake_case
    pub id: String,
    /// 模板展示名
    pub name: String,
    /// emoji 或图标名（UI 可选渲染）
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub icon: String,
    /// 业务字段（顶层内置字段之外的扩展字段定义）
    #[serde(default)]
    pub fields: Vec<TemplateField>,
    /// 匹配规则（可选，预留给后续自动推荐）
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub match_rules: Vec<TemplateMatchRule>,
    /// 创建/修改时间
    #[serde(default)]
    pub utime: i64,
}

impl AccountTemplate {
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            icon: String::new(),
            fields: Vec::new(),
            match_rules: Vec::new(),
            utime: now_ts(),
        }
    }
}

/// 内建账号模板种子（v2.0 P1 起步集）
///
/// 设计原则：
/// - 不重复内置字段（不再为模板加 url/email/phone 这种已经有的）
/// - 字段尽可能少，先把"形"立起来，让用户用起来再扩
/// - icon 用 emoji，UI 不需要图标资产
pub fn default_templates() -> Vec<AccountTemplate> {
    let now = now_ts();
    vec![
        AccountTemplate {
            id: "bank_card".into(),
            name: "银行卡".into(),
            icon: "💳".into(),
            fields: vec![
                TemplateField {
                    key: "card_no".into(),
                    label: "卡号".into(),
                    field_type: TemplateFieldType::Secret,
                    required: true,
                    placeholder: "16~19 位卡号".into(),
                },
                TemplateField {
                    key: "expiry".into(),
                    label: "有效期".into(),
                    field_type: TemplateFieldType::Text,
                    required: false,
                    placeholder: "MM/YY".into(),
                },
                TemplateField {
                    key: "cvv".into(),
                    label: "CVV".into(),
                    field_type: TemplateFieldType::Secret,
                    required: false,
                    placeholder: "卡背 3 位".into(),
                },
                TemplateField {
                    key: "bank".into(),
                    label: "开户行".into(),
                    field_type: TemplateFieldType::Text,
                    required: false,
                    placeholder: String::new(),
                },
            ],
            // 国内主流银行域名 + 通用关键词；命中即推荐应用「银行卡」模板
            match_rules: vec![
                TemplateMatchRule::UrlContains("icbc.com.cn".into()),
                TemplateMatchRule::UrlContains("ccb.com".into()),
                TemplateMatchRule::UrlContains("abchina.com".into()),
                TemplateMatchRule::UrlContains("boc.cn".into()),
                TemplateMatchRule::UrlContains("bankcomm.com".into()),
                TemplateMatchRule::UrlContains("cmbchina.com".into()),
                TemplateMatchRule::UrlContains("spdb.com.cn".into()),
                TemplateMatchRule::UrlContains("cmbc.com.cn".into()),
                TemplateMatchRule::UrlContains("psbc.com".into()),
                TemplateMatchRule::Keyword("银行".into()),
                TemplateMatchRule::Keyword("信用卡".into()),
                TemplateMatchRule::Keyword("储蓄卡".into()),
            ],
            utime: now,
        },
        AccountTemplate {
            id: "id_card".into(),
            name: "证件".into(),
            icon: "🪪".into(),
            fields: vec![
                TemplateField {
                    key: "id_type".into(),
                    label: "证件类型".into(),
                    field_type: TemplateFieldType::Text,
                    required: false,
                    placeholder: "身份证 / 护照 / 驾照…".into(),
                },
                TemplateField {
                    key: "id_number".into(),
                    label: "证件号".into(),
                    field_type: TemplateFieldType::Secret,
                    required: true,
                    placeholder: String::new(),
                },
                TemplateField {
                    key: "valid_until".into(),
                    label: "有效期至".into(),
                    field_type: TemplateFieldType::Date,
                    required: false,
                    placeholder: "YYYY-MM-DD".into(),
                },
            ],
            match_rules: vec![
                TemplateMatchRule::Keyword("身份证".into()),
                TemplateMatchRule::Keyword("护照".into()),
                TemplateMatchRule::Keyword("驾照".into()),
                TemplateMatchRule::Keyword("驾驶证".into()),
                TemplateMatchRule::Keyword("证件".into()),
                TemplateMatchRule::Keyword("户口".into()),
            ],
            utime: now,
        },
        AccountTemplate {
            id: "work_internal".into(),
            name: "工作内网".into(),
            icon: "💼".into(),
            fields: vec![
                TemplateField {
                    key: "employee_id".into(),
                    label: "工号".into(),
                    field_type: TemplateFieldType::Text,
                    required: false,
                    placeholder: String::new(),
                },
                TemplateField {
                    key: "vpn".into(),
                    label: "VPN 地址".into(),
                    field_type: TemplateFieldType::Url,
                    required: false,
                    placeholder: String::new(),
                },
                TemplateField {
                    key: "department".into(),
                    label: "部门".into(),
                    field_type: TemplateFieldType::Text,
                    required: false,
                    placeholder: String::new(),
                },
            ],
            match_rules: vec![
                TemplateMatchRule::Role("工作".into()),
                TemplateMatchRule::Keyword("内网".into()),
                TemplateMatchRule::Keyword("VPN".into()),
                TemplateMatchRule::Keyword("OA".into()),
                TemplateMatchRule::Keyword("工号".into()),
            ],
            utime: now,
        },
        AccountTemplate {
            id: "game".into(),
            name: "游戏".into(),
            icon: "🎮".into(),
            fields: vec![
                TemplateField {
                    key: "server".into(),
                    label: "服务器/区服".into(),
                    field_type: TemplateFieldType::Text,
                    required: false,
                    placeholder: String::new(),
                },
                TemplateField {
                    key: "char_name".into(),
                    label: "角色名".into(),
                    field_type: TemplateFieldType::Text,
                    required: false,
                    placeholder: String::new(),
                },
                TemplateField {
                    key: "uid".into(),
                    label: "UID".into(),
                    field_type: TemplateFieldType::Text,
                    required: false,
                    placeholder: String::new(),
                },
            ],
            match_rules: vec![
                TemplateMatchRule::UrlContains("steampowered.com".into()),
                TemplateMatchRule::UrlContains("steamcommunity.com".into()),
                TemplateMatchRule::UrlContains("epicgames.com".into()),
                TemplateMatchRule::UrlContains("battle.net".into()),
                TemplateMatchRule::UrlContains("mihoyo.com".into()),
                TemplateMatchRule::UrlContains("hoyoverse.com".into()),
                TemplateMatchRule::UrlContains("ea.com".into()),
                TemplateMatchRule::UrlContains("ubisoft.com".into()),
                TemplateMatchRule::Keyword("游戏".into()),
                TemplateMatchRule::Keyword("steam".into()),
            ],
            utime: now,
        },
    ]
}

/// 模板 match_rules 归一化：trim、去空、按 (kind, value) 去重、限制总数与单条长度
pub fn normalize_match_rules(raw: &[TemplateMatchRule]) -> Vec<TemplateMatchRule> {
    let mut seen: std::collections::HashSet<(u8, String)> = std::collections::HashSet::new();
    let mut out: Vec<TemplateMatchRule> = Vec::new();
    for r in raw {
        let (kind_id, value): (u8, String) = match r {
            TemplateMatchRule::UrlContains(v) => (0, v.trim().to_string()),
            TemplateMatchRule::Keyword(v) => (1, v.trim().to_string()),
            TemplateMatchRule::Role(v) => (2, v.trim().to_string()),
        };
        if value.is_empty() {
            continue;
        }
        let value: String = value.chars().take(MATCH_RULE_VALUE_MAX).collect();
        if !seen.insert((kind_id, value.clone())) {
            continue;
        }
        let normalized = match kind_id {
            0 => TemplateMatchRule::UrlContains(value),
            1 => TemplateMatchRule::Keyword(value),
            _ => TemplateMatchRule::Role(value),
        };
        out.push(normalized);
        if out.len() >= MATCH_RULES_MAX_COUNT {
            break;
        }
    }
    out
}

/// 模板包：明文 JSON 数据交换格式（v2.0+）
///
/// 用于「模板 .json 导入导出」与社区预设包分享。模板属于结构化定义，
/// 不含敏感信息，故采用明文 JSON 便于 git 版本管理与人工审阅。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplatePack {
    /// schema 版本号；当前为 1。后续若有破坏性结构变化将递增并保持兼容读
    pub schema_version: u32,
    /// 固定标识符，用于识别此 JSON 是模板包而非其他数据交换格式
    pub kind: String,
    /// 导出时的 unix 秒级时间戳
    #[serde(default)]
    pub exported_at: i64,
    /// 模板列表
    #[serde(default)]
    pub templates: Vec<AccountTemplate>,
}

impl TemplatePack {
    /// 构造一个新的模板包（自动填 schema_version / kind / exported_at）
    pub fn new(templates: Vec<AccountTemplate>) -> Self {
        Self {
            schema_version: TEMPLATE_PACK_SCHEMA_VERSION,
            kind: TEMPLATE_PACK_KIND.to_string(),
            exported_at: now_ts(),
            templates,
        }
    }
}

/// 模板包导入结果统计
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TemplateImportResult {
    /// 新增模板数（id 在当前 vault 不存在）
    pub added: usize,
    /// 覆盖更新数（仅 overwrite 模式下有意义）
    pub updated: usize,
    /// 跳过数（merge 模式下因 id 冲突跳过）
    pub skipped: usize,
    /// 因结构问题被丢弃的模板数（id/name 非法、字段重复等）
    pub invalid: usize,
}

/// 校验并归一化单个待导入模板；返回 None 表示该模板被丢弃
///
/// 校验规则与 [crate::vault::VaultState::upsert_template] 对齐，但更宽松：
/// 不会因为单个模板 invalid 就让整批失败。
pub fn sanitize_imported_template(mut t: AccountTemplate) -> Option<AccountTemplate> {
    let id = t.id.trim().to_string();
    let name = t.name.trim().to_string();
    if id.is_empty() || name.is_empty() {
        return None;
    }
    // id 仅支持字母、数字、下划线、短横线（与前端校验一致）
    if !id
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        return None;
    }
    // 字段 key 去重 + 合法性
    let mut seen_keys = std::collections::HashSet::<String>::new();
    let mut fields = Vec::with_capacity(t.fields.len());
    for mut f in t.fields.into_iter() {
        let k = f.key.trim().to_string();
        let lb = f.label.trim().to_string();
        if k.is_empty() || lb.is_empty() {
            return None;
        }
        if !k
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        {
            return None;
        }
        if !seen_keys.insert(k.clone()) {
            return None;
        }
        f.key = k;
        f.label = lb;
        f.placeholder = f.placeholder.trim().to_string();
        fields.push(f);
    }
    t.id = id;
    t.name = name;
    t.icon = t.icon.trim().to_string();
    t.fields = fields;
    t.match_rules = normalize_match_rules(&t.match_rules);
    t.utime = now_ts();
    Some(t)
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

#[cfg(test)]
mod tests {
    use super::*;

    /// 旧库 JSON（无 templates / template_id / custom_fields）仍可反序列化；
    /// 这保证从P ython 单向导入的 v6/.zmb 数据不需迁移即能加载。
    #[test]
    fn legacy_vault_json_loads_without_new_fields() {
        let legacy = r#"{
            "data": [
                { "id": 1700000000, "role": "个人", "name": "某网站", "userID": "alice",
                  "pwd": "hunter2", "phone": "", "email": "", "url": "", "desc": "",
                  "utime": 1700000000, "tags": ["社交"] }
            ],
            "roles": ["个人", "工作"],
            "utime": 1700000000
        }"#;
        let data: VaultData = serde_json::from_str(legacy).expect("旧库反序列化应成功");
        assert_eq!(data.entries.len(), 1);
        assert!(data.templates.is_empty());
        let e = &data.entries[0];
        assert_eq!(e.template_id, "");
        assert!(e.custom_fields.is_empty());
    }

    /// 含 custom_fields + template_id 的条目应 round-trip 保留
    #[test]
    fn entry_with_custom_fields_roundtrip() {
        let mut e = PasswordEntry::new();
        e.name = "某银行".into();
        e.template_id = "bank_card".into();
        e.custom_fields.insert("card_no".into(), "6217 0000 0000 0000".into());
        e.custom_fields.insert("exp_date".into(), "12/29".into());
        let mut data = VaultData::new();
        data.entries.push(e);
        data.templates
            .push(AccountTemplate::new("bank_card", "银行卡"));

        let json = serde_json::to_string(&data).unwrap();
        let parsed: VaultData = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.entries[0].template_id, "bank_card");
        assert_eq!(
            parsed.entries[0].custom_fields.get("card_no").map(|s| s.as_str()),
            Some("6217 0000 0000 0000")
        );
        assert_eq!(parsed.templates.len(), 1);
        assert_eq!(parsed.templates[0].id, "bank_card");
    }

    /// 空 custom_fields / templates 应 skip_serializing_if 生效（输出不含该 key）
    #[test]
    fn empty_extensions_are_omitted_in_json() {
        let data = VaultData::new();
        let json = serde_json::to_string(&data).unwrap();
        assert!(!json.contains("templates"), "templates 为空时不该出现在 JSON 里");
        let e = PasswordEntry::new();
        let json = serde_json::to_string(&e).unwrap();
        assert!(!json.contains("custom_fields"));
        assert!(!json.contains("template_id"));
    }

    /// 内置模板的 match_rules 应配齐：能匹配到典型 url / 关键词 / role
    #[test]
    fn default_templates_have_match_rules() {
        let templates = default_templates();
        let bank = templates.iter().find(|t| t.id == "bank_card").unwrap();
        assert!(
            bank.match_rules
                .iter()
                .any(|r| matches!(r, TemplateMatchRule::UrlContains(s) if s.contains("icbc"))),
            "银行卡模板应包含 icbc 域名匹配"
        );
        let work = templates.iter().find(|t| t.id == "work_internal").unwrap();
        assert!(
            work.match_rules
                .iter()
                .any(|r| matches!(r, TemplateMatchRule::Role(s) if s == "工作")),
            "工作内网模板应匹配 role=工作"
        );
        let id_card = templates.iter().find(|t| t.id == "id_card").unwrap();
        assert!(
            id_card
                .match_rules
                .iter()
                .any(|r| matches!(r, TemplateMatchRule::Keyword(s) if s == "身份证")),
            "证件模板应匹配关键词「身份证」"
        );
    }

    /// normalize_match_rules 去重、去空、限制总数
    #[test]
    fn normalize_match_rules_basic() {
        let raw = vec![
            TemplateMatchRule::UrlContains("  icbc.com.cn  ".into()),
            TemplateMatchRule::UrlContains("icbc.com.cn".into()), // 与上条 trim 后相同，应去重
            TemplateMatchRule::Keyword("".into()),                // 空，应丢弃
            TemplateMatchRule::Keyword("   ".into()),             // 全空白，应丢弃
            TemplateMatchRule::Role("工作".into()),
            TemplateMatchRule::Keyword("工作".into()), // 与 Role 字面一样但 kind 不同，应保留
        ];
        let out = normalize_match_rules(&raw);
        assert_eq!(out.len(), 3, "应保留 3 条：1 条 url + 1 条 role + 1 条 keyword");
        assert!(matches!(
            out[0],
            TemplateMatchRule::UrlContains(ref s) if s == "icbc.com.cn"
        ));
    }

    /// sanitize_imported_template 拒绝非法 id / 重复字段 / 空字段
    #[test]
    fn sanitize_imported_template_validation() {
        // 空 id
        let mut t = AccountTemplate::new("", "name");
        assert!(sanitize_imported_template(t.clone()).is_none());
        // 非法字符
        t = AccountTemplate::new("bank card", "name"); // 含空格
        assert!(sanitize_imported_template(t).is_none());
        // 重复字段 key
        let dup = AccountTemplate {
            id: "ok_id".into(),
            name: "name".into(),
            icon: String::new(),
            fields: vec![
                TemplateField {
                    key: "k".into(),
                    label: "a".into(),
                    field_type: TemplateFieldType::Text,
                    required: false,
                    placeholder: String::new(),
                },
                TemplateField {
                    key: "k".into(),
                    label: "b".into(),
                    field_type: TemplateFieldType::Text,
                    required: false,
                    placeholder: String::new(),
                },
            ],
            match_rules: vec![],
            utime: 0,
        };
        assert!(sanitize_imported_template(dup).is_none());
    }

    /// sanitize_imported_template trim 并归一化 match_rules
    #[test]
    fn sanitize_imported_template_trim_normalize() {
        let t = AccountTemplate {
            id: "  good_id  ".into(),
            name: "  Good Name  ".into(),
            icon: "  🔑  ".into(),
            fields: vec![TemplateField {
                key: "  user  ".into(),
                label: "  用户名  ".into(),
                field_type: TemplateFieldType::Text,
                required: false,
                placeholder: "  ph  ".into(),
            }],
            match_rules: vec![
                TemplateMatchRule::UrlContains("  example.com  ".into()),
                TemplateMatchRule::UrlContains("example.com".into()), // 应被去重
            ],
            utime: 0,
        };
        let out = sanitize_imported_template(t).expect("应该保留");
        assert_eq!(out.id, "good_id");
        assert_eq!(out.name, "Good Name");
        assert_eq!(out.icon, "🔑");
        assert_eq!(out.fields[0].key, "user");
        assert_eq!(out.fields[0].label, "用户名");
        assert_eq!(out.fields[0].placeholder, "ph");
        assert_eq!(out.match_rules.len(), 1, "重复 url 应被去重");
    }

    /// TemplatePack 序列化与反序列化循环
    #[test]
    fn template_pack_serde_roundtrip() {
        let mut t = AccountTemplate::new("my_id", "My Name");
        t.fields.push(TemplateField {
            key: "k".into(),
            label: "中文名".into(),
            field_type: TemplateFieldType::Secret,
            required: true,
            placeholder: String::new(),
        });
        t.match_rules.push(TemplateMatchRule::UrlContains("foo.com".into()));
        let pack = TemplatePack::new(vec![t]);
        let json = serde_json::to_string(&pack).unwrap();
        let parsed: TemplatePack = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.kind, TEMPLATE_PACK_KIND);
        assert_eq!(parsed.schema_version, TEMPLATE_PACK_SCHEMA_VERSION);
        assert_eq!(parsed.templates.len(), 1);
        assert_eq!(parsed.templates[0].id, "my_id");
        assert_eq!(parsed.templates[0].fields[0].label, "中文名");
    }

    /// normalize_custom_fields 应去空 key、截断超长、限制总数
    #[test]
    fn normalize_custom_fields_basic() {
        let mut raw = BTreeMap::new();
        raw.insert("  ".into(), "v".into()); // 空 key 跳过
        raw.insert("k".into(), "v".into());
        let long_key: String = "x".repeat(CUSTOM_FIELD_KEY_MAX + 50);
        let long_val: String = "y".repeat(CUSTOM_FIELD_VALUE_MAX + 50);
        raw.insert(long_key, long_val);

        let out = normalize_custom_fields(raw);
        assert!(out.contains_key("k"));
        assert!(out.keys().all(|k| !k.is_empty()));
        let truncated_key_len = out
            .keys()
            .map(|k| k.chars().count())
            .max()
            .unwrap_or(0);
        assert!(truncated_key_len <= CUSTOM_FIELD_KEY_MAX);
        for v in out.values() {
            assert!(v.chars().count() <= CUSTOM_FIELD_VALUE_MAX);
        }
    }

    /// rc.2 hotfix：老 vault 中内建模板 match_rules 为空时，upgrade() 应从 default_templates 补齐
    #[test]
    fn upgrade_fills_missing_builtin_match_rules() {
        let mut data = VaultData::new();
        // 模拟老版本创建的 vault：有内建 id 但 match_rules 为空
        data.templates.push(AccountTemplate {
            id: "bank_card".into(),
            name: "银行卡".into(),
            icon: "💳".into(),
            fields: vec![],
            match_rules: vec![],
            utime: 0,
        });
        data.upgrade();
        let bank = data.templates.iter().find(|t| t.id == "bank_card").unwrap();
        assert!(
            !bank.match_rules.is_empty(),
            "老 vault 的内建银行卡模板 match_rules 应被 upgrade 补齐"
        );
        assert!(
            bank.match_rules
                .iter()
                .any(|r| matches!(r, TemplateMatchRule::UrlContains(s) if s.contains("icbc"))),
            "补齐后应包含 icbc 域名规则"
        );
    }

    /// upgrade() 不应覆盖用户已自定义过的 match_rules
    #[test]
    fn upgrade_keeps_user_customized_match_rules() {
        let mut data = VaultData::new();
        let custom_rule = TemplateMatchRule::UrlContains("my-private-bank.local".into());
        data.templates.push(AccountTemplate {
            id: "bank_card".into(),
            name: "银行卡".into(),
            icon: "💳".into(),
            fields: vec![],
            match_rules: vec![custom_rule.clone()],
            utime: 0,
        });
        data.upgrade();
        let bank = data.templates.iter().find(|t| t.id == "bank_card").unwrap();
        assert_eq!(bank.match_rules.len(), 1, "用户已自定义 match_rules 不应被 upgrade 覆盖或追加");
        assert!(matches!(
            bank.match_rules[0],
            TemplateMatchRule::UrlContains(ref s) if s == "my-private-bank.local"
        ));
    }
}
