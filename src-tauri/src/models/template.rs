//! 账号模板相关模型：`TemplateFieldType` / `TemplateField` / `TemplateMatchRule`
//! / `AccountTemplate` / `TemplatePack` / `TemplateImportResult`，以及内建种子
//! 与导入归一化辅助。

use serde::{Deserialize, Serialize};

use super::{
    now_ts, MATCH_RULES_MAX_COUNT, MATCH_RULE_VALUE_MAX, TEMPLATE_PACK_KIND,
    TEMPLATE_PACK_SCHEMA_VERSION,
};

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
    /// 模板唯一 id（如 `bank_card` / `social` / `game`），推荐 snake_case
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
