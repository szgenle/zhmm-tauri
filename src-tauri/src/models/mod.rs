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
//!
//! 模块拆分：
//! - [`password`]：`PasswordHistoryItem` / `PasswordEntry` / `PasswordSummary` / `PasswordInput` 与 tags / custom_fields 归一化
//! - [`template`]：`AccountTemplate` 及配套字段、匹配规则、模板包与导入归一化
//! - [`vault`]：`VaultData` 顶层容器与 upgrade 兼容逻辑
//!
//! 通过 `pub use` 重导出全部公共项，外部 `crate::models::Xxx` 路径保持不变。

mod password;
mod template;
mod vault;

pub use password::*;
pub use template::*;
pub use vault::*;

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

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
        e.custom_fields
            .insert("card_no".into(), "6217 0000 0000 0000".into());
        e.custom_fields.insert("exp_date".into(), "12/29".into());
        let mut data = VaultData::new();
        data.entries.push(e);
        data.templates
            .push(AccountTemplate::new("bank_card", "银行卡"));

        let json = serde_json::to_string(&data).unwrap();
        let parsed: VaultData = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.entries[0].template_id, "bank_card");
        assert_eq!(
            parsed.entries[0]
                .custom_fields
                .get("card_no")
                .map(|s| s.as_str()),
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
        assert!(
            !json.contains("templates"),
            "templates 为空时不该出现在 JSON 里"
        );
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
        assert_eq!(
            out.len(),
            3,
            "应保留 3 条：1 条 url + 1 条 role + 1 条 keyword"
        );
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
        t.match_rules
            .push(TemplateMatchRule::UrlContains("foo.com".into()));
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
        let truncated_key_len = out.keys().map(|k| k.chars().count()).max().unwrap_or(0);
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
        assert_eq!(
            bank.match_rules.len(),
            1,
            "用户已自定义 match_rules 不应被 upgrade 覆盖或追加"
        );
        assert!(matches!(
            bank.match_rules[0],
            TemplateMatchRule::UrlContains(ref s) if s == "my-private-bank.local"
        ));
    }
}
