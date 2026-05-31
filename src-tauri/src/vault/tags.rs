//! 标签管理方法：collect_tag_counts / rename_tag / delete_tag

use crate::errors::{AppError, AppResult};
use crate::models::{normalize_tags, now_ts};

use super::VaultState;

impl VaultState {
    pub fn collect_tag_counts(&self) -> AppResult<Vec<(String, usize)>> {
        let guard = self.data.read();
        let data = guard.as_ref().ok_or(AppError::Locked)?;
        let mut counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
        for entry in &data.entries {
            for tag in &entry.tags {
                if !tag.is_empty() {
                    *counts.entry(tag.clone()).or_insert(0) += 1;
                }
            }
        }
        let mut result: Vec<(String, usize)> = counts.into_iter().collect();
        result.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        Ok(result)
    }

    pub fn rename_tag(&self, old: &str, new: &str) -> AppResult<usize> {
        let old_n = old.trim();
        let new_n = new.trim();
        if old_n.is_empty() || new_n.is_empty() || old_n == new_n {
            return Ok(0);
        }
        let mut affected = 0;
        {
            let mut guard = self.data.write();
            let data = guard.as_mut().ok_or(AppError::Locked)?;
            for entry in &mut data.entries {
                if entry.tags.contains(&old_n.to_string()) {
                    entry.tags = entry
                        .tags
                        .iter()
                        .map(|t| {
                            if t == old_n {
                                new_n.to_string()
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
            if affected > 0 {
                data.utime = now_ts();
            }
        }
        if affected > 0 {
            self.persist_with_cached()?;
        }
        Ok(affected)
    }

    pub fn delete_tag(&self, tag: &str) -> AppResult<usize> {
        let target = tag.trim();
        if target.is_empty() {
            return Ok(0);
        }
        let mut affected = 0;
        {
            let mut guard = self.data.write();
            let data = guard.as_mut().ok_or(AppError::Locked)?;
            for entry in &mut data.entries {
                if entry.tags.contains(&target.to_string()) {
                    entry.tags.retain(|t| t != target);
                    entry.utime = now_ts();
                    affected += 1;
                }
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
}
