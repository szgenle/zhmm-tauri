//! 标签注册表管理方法：list / save / upsert / remove / merge / stats

use crate::errors::{AppError, AppResult};
use crate::models::{normalize_tags, now_ts, TagDefinition, TagHierarchyNode, TagStats};

use super::VaultState;

impl VaultState {
    /// 返回完整标签注册表
    pub fn list_tag_registry(&self) -> AppResult<Vec<TagDefinition>> {
        let guard = self.data.read();
        let data = guard.as_ref().ok_or(AppError::Locked)?;
        Ok(data.tag_registry.clone())
    }

    /// 整体覆盖保存标签注册表（前端拖拽排序后一次性提交）
    pub fn save_tag_registry(&self, items: Vec<TagDefinition>) -> AppResult<()> {
        {
            let mut guard = self.data.write();
            let data = guard.as_mut().ok_or(AppError::Locked)?;
            data.tag_registry = items;
            data.utime = now_ts();
        }
        self.persist_with_cached()
    }

    /// 新增或更新单条标签定义（按 name 匹配）
    pub fn upsert_tag_def(&self, def: TagDefinition) -> AppResult<()> {
        let name = def.name.trim().to_string();
        if name.is_empty() {
            return Err(AppError::Invalid("标签名不能为空".into()));
        }
        {
            let mut guard = self.data.write();
            let data = guard.as_mut().ok_or(AppError::Locked)?;
            if let Some(existing) = data.tag_registry.iter_mut().find(|t| t.name == name) {
                existing.color = def.color;
                existing.icon = def.icon;
                existing.order = def.order;
                // source 只在非空时更新（不覆盖已有来源）
                if !def.source.is_empty() {
                    existing.source = def.source;
                }
            } else {
                data.tag_registry.push(TagDefinition {
                    name,
                    color: def.color,
                    icon: def.icon,
                    order: def.order,
                    source: def.source,
                });
            }
            data.utime = now_ts();
        }
        self.persist_with_cached()
    }

    /// 从注册表移除（不影响已有条目的 tags 字段）
    pub fn remove_tag_def(&self, name: &str) -> AppResult<()> {
        let target = name.trim();
        if target.is_empty() {
            return Ok(());
        }
        {
            let mut guard = self.data.write();
            let data = guard.as_mut().ok_or(AppError::Locked)?;
            let before = data.tag_registry.len();
            data.tag_registry.retain(|t| t.name != target);
            if data.tag_registry.len() < before {
                data.utime = now_ts();
            }
        }
        self.persist_with_cached()
    }

    /// 将多个标签合并为 target（遍历所有条目替换 + 注册表合并）
    pub fn merge_tags(&self, sources: &[String], target: &str) -> AppResult<usize> {
        let target = target.trim().to_string();
        if target.is_empty() {
            return Err(AppError::Invalid("目标标签不能为空".into()));
        }
        let sources: Vec<String> = sources
            .iter()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty() && s != &target)
            .collect();
        if sources.is_empty() {
            return Ok(0);
        }

        let mut affected = 0;
        {
            let mut guard = self.data.write();
            let data = guard.as_mut().ok_or(AppError::Locked)?;

            // 遍历所有条目，将 sources 中的标签替换为 target
            for entry in &mut data.entries {
                let has_source = entry.tags.iter().any(|t| sources.contains(t));
                if has_source {
                    entry.tags = entry
                        .tags
                        .iter()
                        .map(|t| {
                            if sources.contains(t) {
                                target.clone()
                            } else {
                                t.clone()
                            }
                        })
                        .collect();
                    entry.tags = normalize_tags(&entry.tags);
                    entry.utime = now_ts();
                    affected += 1;
                }
            }

            // 从注册表中移除 sources，确保 target 存在
            data.tag_registry.retain(|t| !sources.contains(&t.name));
            if !data.tag_registry.iter().any(|t| t.name == target) {
                data.tag_registry.push(TagDefinition::new(&target));
            }

            if affected > 0 {
                data.utime = now_ts();
            }
        }
        if affected > 0 {
            self.persist_with_cached()?;
        }
        Ok(affected)
    }

    /// 返回每个标签的使用统计
    pub fn get_tag_stats(
        &self,
        catalog_tags: &std::collections::HashSet<String>,
    ) -> AppResult<Vec<TagStats>> {
        let guard = self.data.read();
        let data = guard.as_ref().ok_or(AppError::Locked)?;

        let mut stats_map: std::collections::HashMap<String, TagStats> =
            std::collections::HashMap::new();

        for entry in &data.entries {
            for (i, tag) in entry.tags.iter().enumerate() {
                if tag.is_empty() {
                    continue;
                }
                let stat = stats_map.entry(tag.clone()).or_insert_with(|| TagStats {
                    name: tag.clone(),
                    count: 0,
                    primary_count: 0,
                    in_catalog: catalog_tags.contains(tag),
                });
                stat.count += 1;
                if i == 0 {
                    stat.primary_count += 1;
                }
            }
        }

        // 也加入注册表中定义但未被使用的标签
        for def in &data.tag_registry {
            stats_map
                .entry(def.name.clone())
                .or_insert_with(|| TagStats {
                    name: def.name.clone(),
                    count: 0,
                    primary_count: 0,
                    in_catalog: catalog_tags.contains(&def.name),
                });
        }

        let mut result: Vec<TagStats> = stats_map.into_values().collect();
        result.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.name.cmp(&b.name)));
        Ok(result)
    }

    /// 返回标签逻辑层级：以 tags[0] 为一级标签，聚合其下所有子标签（tags[1..]）
    pub fn get_tag_hierarchy(&self) -> AppResult<Vec<TagHierarchyNode>> {
        let guard = self.data.read();
        let data = guard.as_ref().ok_or(AppError::Locked)?;

        let mut hierarchy: std::collections::HashMap<String, std::collections::HashSet<String>> =
            std::collections::HashMap::new();

        for entry in &data.entries {
            if entry.tags.is_empty() {
                continue;
            }
            let primary = &entry.tags[0];
            if primary.is_empty() {
                continue;
            }
            let children_set = hierarchy.entry(primary.clone()).or_default();
            for tag in entry.tags.iter().skip(1) {
                if !tag.is_empty() {
                    children_set.insert(tag.clone());
                }
            }
        }

        // 确保所有一级标签都有条目（包括只有单标签的情况）
        // 按注册表 order 排序一级标签
        let reg_order: std::collections::HashMap<String, i32> = data
            .tag_registry
            .iter()
            .map(|d| (d.name.clone(), d.order))
            .collect();

        let mut result: Vec<TagHierarchyNode> = hierarchy
            .into_iter()
            .map(|(name, children_set)| {
                let mut children: Vec<String> = children_set.into_iter().collect();
                children.sort();
                TagHierarchyNode { name, children }
            })
            .collect();

        result.sort_by(|a, b| {
            let oa = reg_order.get(&a.name).copied().unwrap_or(99999);
            let ob = reg_order.get(&b.name).copied().unwrap_or(99999);
            oa.cmp(&ob).then_with(|| a.name.cmp(&b.name))
        });

        Ok(result)
    }
}
