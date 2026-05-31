//! 查询解析与解锁辅助

use std::path::Path;

use zhmm_tauri_lib::{
    errors::{AppError, AppResult},
    models::{PasswordEntry, PasswordSummary},
    vault::VaultState,
};

use super::io_helpers::{ellipsize, prompt_password};

/// 解析用户输入为具体条目（要求唯一命中，用于 totp/del/get -p）。
/// 1. 纯数字且能记录 id 精确命中 → 返回该条目
/// 2. 否则按 user_id / url / desc 子串不区分大小写匹配
///    - 0 条 → 报错
///    - 1 条 → 返回
///    - >1 条 → 列出候选让用户细化
pub fn resolve_entry(state: &VaultState, query: &str) -> AppResult<PasswordEntry> {
    let q = query.trim();
    if q.is_empty() {
        return Err(AppError::Invalid("查询字串为空".into()));
    }
    // 1) 先试 id
    if let Ok(id) = q.parse::<i64>() {
        if let Ok(entry) = state.get(id) {
            return Ok(entry);
        }
    }
    // 2) 子串搜索
    let q_low = q.to_lowercase();
    let items = state.list()?;
    let hits: Vec<&PasswordSummary> = items
        .iter()
        .filter(|i| {
            i.user_id.to_lowercase().contains(&q_low)
                || i.url.to_lowercase().contains(&q_low)
                || i.desc.to_lowercase().contains(&q_low)
        })
        .collect();
    match hits.len() {
        0 => Err(AppError::Other(format!(
            "找不到与 “{q}” 匹配的条目（请试 zhmm-cli list 查看现有条目）"
        ))),
        1 => state.get(hits[0].id),
        n => {
            eprintln!("⚠ 匹配到 {n} 条，请进一步细化关键字或直接传 id：");
            for i in &hits {
                eprintln!(
                    "  id={:<12} role={:<4} user={:<24} url={}",
                    i.id,
                    ellipsize(&i.role, 4),
                    ellipsize(&i.user_id, 24),
                    i.url
                );
            }
            Err(AppError::Invalid(format!("“{q}” 不唯一")))
        }
    }
}

/// 解析用户输入为一或多条完整条目（用于 get 多命中全部展示）。
/// 规则同 resolve_entry，但多条命中不报错、全部返回。
pub fn resolve_entries(state: &VaultState, query: &str) -> AppResult<Vec<PasswordEntry>> {
    let q = query.trim();
    if q.is_empty() {
        return Err(AppError::Invalid("查询字串为空".into()));
    }
    // 1) 先试 id。纯数字命中 → 只返该条，不再做子串搜
    if let Ok(id) = q.parse::<i64>() {
        if let Ok(entry) = state.get(id) {
            return Ok(vec![entry]);
        }
    }
    // 2) 子串搜索
    let q_low = q.to_lowercase();
    let items = state.list()?;
    let hit_ids: Vec<i64> = items
        .iter()
        .filter(|i| {
            i.user_id.to_lowercase().contains(&q_low)
                || i.url.to_lowercase().contains(&q_low)
                || i.desc.to_lowercase().contains(&q_low)
        })
        .map(|i| i.id)
        .collect();
    if hit_ids.is_empty() {
        return Err(AppError::Other(format!(
            "找不到与 “{q}” 匹配的条目（请试 zhmm-cli list 查看现有条目）"
        )));
    }
    let mut entries = Vec::with_capacity(hit_ids.len());
    for id in hit_ids {
        entries.push(state.get(id)?);
    }
    Ok(entries)
}

pub fn unlock(password: Option<&str>, file: &Path, account: &str) -> AppResult<VaultState> {
    let pwd = match password {
        Some(p) => p.to_string(),
        None => prompt_password("主密码: ")?,
    };
    let state = VaultState::new();
    state.unlock_with_path(file, account, &pwd)?;
    Ok(state)
}
