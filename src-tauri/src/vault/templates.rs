//! 账号模板相关方法：templates / upsert_template / delete_template
//! / seed_default_templates / export_templates_json / import_templates_json

use std::fs;
use std::path::Path;

use crate::errors::{AppError, AppResult};
use crate::models::{
    default_templates, normalize_match_rules, now_ts, sanitize_imported_template, AccountTemplate,
    TemplateImportResult, TemplatePack, TEMPLATE_PACK_KIND, TEMPLATE_PACK_MAX_TEMPLATES,
};

use super::VaultState;

impl VaultState {
    /// 列出当前 vault 的所有模板
    pub fn templates(&self) -> AppResult<Vec<AccountTemplate>> {
        let guard = self.data.read();
        let data = guard.as_ref().ok_or(AppError::Locked)?;
        Ok(data.templates.clone())
    }

    /// 插入或更新模板（按 id）；返回赋值后的模板
    pub fn upsert_template(&self, mut template: AccountTemplate) -> AppResult<AccountTemplate> {
        let id = template.id.trim().to_string();
        if id.is_empty() {
            return Err(AppError::Invalid("模板 id 不能为空".into()));
        }
        if template.name.trim().is_empty() {
            return Err(AppError::Invalid("模板名称不能为空".into()));
        }
        template.id = id;
        template.match_rules = normalize_match_rules(&template.match_rules);
        template.utime = now_ts();

        {
            let mut guard = self.data.write();
            let data = guard.as_mut().ok_or(AppError::Locked)?;
            if let Some(pos) = data.templates.iter().position(|t| t.id == template.id) {
                data.templates[pos] = template.clone();
            } else {
                data.templates.push(template.clone());
            }
            data.utime = now_ts();
        }
        self.persist_with_cached()?;
        Ok(template)
    }

    /// 按 id 删除模板；不会主动清理条目上的 template_id 引用（允许 dangling）
    pub fn delete_template(&self, id: &str) -> AppResult<()> {
        {
            let mut guard = self.data.write();
            let data = guard.as_mut().ok_or(AppError::Locked)?;
            let before = data.templates.len();
            data.templates.retain(|t| t.id != id);
            if data.templates.len() == before {
                return Err(AppError::NotFound);
            }
            data.utime = now_ts();
        }
        self.persist_with_cached()
    }

    /// 将内建种子模板幂等合并到当前 vault；返回新增数量
    /// （已存在的 id 不会覆盖，保护用户修改）
    pub fn seed_default_templates(&self) -> AppResult<usize> {
        let added = {
            let mut guard = self.data.write();
            let data = guard.as_mut().ok_or(AppError::Locked)?;
            let existing: std::collections::HashSet<String> =
                data.templates.iter().map(|t| t.id.clone()).collect();
            let mut added = 0usize;
            for t in default_templates() {
                if !existing.contains(&t.id) {
                    data.templates.push(t);
                    added += 1;
                }
            }
            if added > 0 {
                data.utime = now_ts();
            }
            added
        };
        if added > 0 {
            self.persist_with_cached()?;
        }
        Ok(added)
    }

    /// 将当前 vault 的模板（可按 ids 过滤）以明文 JSON 模板包格式写入 path
    ///
    /// `ids` 为 None 或空数组：导出全部模板。
    /// 返回实际写入的模板数。
    pub fn export_templates_json(&self, path: &Path, ids: Option<&[String]>) -> AppResult<usize> {
        let templates_to_export = {
            let guard = self.data.read();
            let data = guard.as_ref().ok_or(AppError::Locked)?;
            match ids {
                Some(ids) if !ids.is_empty() => {
                    let want: std::collections::HashSet<&str> =
                        ids.iter().map(|s| s.as_str()).collect();
                    data.templates
                        .iter()
                        .filter(|t| want.contains(t.id.as_str()))
                        .cloned()
                        .collect::<Vec<_>>()
                }
                _ => data.templates.clone(),
            }
        };
        if templates_to_export.is_empty() {
            return Err(AppError::Invalid("没有可导出的模板".into()));
        }
        let count = templates_to_export.len();
        let pack = TemplatePack::new(templates_to_export);
        // pretty-print 便于 git diff / 人工审阅
        let json = serde_json::to_vec_pretty(&pack)?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, json)?;
        Ok(count)
    }

    /// 从明文 JSON 模板包读取模板并合并到当前 vault
    ///
    /// `overwrite=false` (merge模式)：id 冲突时保留当前 vault 版本，跳过导入者
    /// `overwrite=true`  (overwrite模式)：id 冲突时采用导入者覆盖
    pub fn import_templates_json(
        &self,
        path: &Path,
        overwrite: bool,
    ) -> AppResult<TemplateImportResult> {
        if !path.exists() {
            return Err(AppError::Other(format!("文件不存在: {}", path.display())));
        }
        let bytes = fs::read(path)?;
        let pack: TemplatePack = serde_json::from_slice(&bytes)
            .map_err(|e| AppError::Invalid(format!("模板包 JSON 解析失败: {e}")))?;
        if pack.kind != TEMPLATE_PACK_KIND {
            return Err(AppError::Invalid(format!(
                "文件格式不是模板包（kind={}）",
                pack.kind
            )));
        }
        if pack.templates.len() > TEMPLATE_PACK_MAX_TEMPLATES {
            return Err(AppError::Invalid(format!(
                "模板包包含 {} 个模板，超过上限 {}",
                pack.templates.len(),
                TEMPLATE_PACK_MAX_TEMPLATES
            )));
        }

        let mut result = TemplateImportResult::default();
        // 包内同 id 去重：只保留后出现的一份
        let mut sanitized_by_id: std::collections::BTreeMap<String, AccountTemplate> =
            std::collections::BTreeMap::new();
        for raw in pack.templates.into_iter() {
            match sanitize_imported_template(raw) {
                Some(t) => {
                    sanitized_by_id.insert(t.id.clone(), t);
                }
                None => result.invalid += 1,
            }
        }

        let mut changed = false;
        {
            let mut guard = self.data.write();
            let data = guard.as_mut().ok_or(AppError::Locked)?;
            for (_id, t) in sanitized_by_id.into_iter() {
                let pos = data.templates.iter().position(|x| x.id == t.id);
                match (pos, overwrite) {
                    (Some(idx), true) => {
                        data.templates[idx] = t;
                        result.updated += 1;
                        changed = true;
                    }
                    (Some(_), false) => {
                        result.skipped += 1;
                    }
                    (None, _) => {
                        data.templates.push(t);
                        result.added += 1;
                        changed = true;
                    }
                }
            }
            if changed {
                data.utime = now_ts();
            }
        }
        if changed {
            self.persist_with_cached()?;
        }
        Ok(result)
    }
}
