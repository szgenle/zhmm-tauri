//! Vault 顶层明文数据模型 `VaultData`（v7 schema）

use serde::{Deserialize, Serialize};

use super::{default_templates, now_ts, AccountTemplate, PasswordEntry, DEFAULT_ROLES};

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
