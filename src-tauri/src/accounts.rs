//! 最近访问的密码库列表（明文 JSON 配置）
//!
//! 与 Python 版 `saved_files` 等价：记录每个曾访问过的 .zmb 文件的
//! 路径、账号名、bcrypt 哈希后的密码（仅用于 UI 层快速预校验避免
//! 触发昂贵的 Argon2id），以及最近访问时间。

use std::fs;
use std::path::{Path, PathBuf};

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

use crate::errors::{AppError, AppResult};

/// 最近访问条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecentEntry {
    pub path: String,
    pub account: String,
    /// bcrypt 哈希；UI 层快速校验密码（命中后再触发解密）
    pub hashpw: String,
    pub last_access_time: String,
}

/// 最近访问列表存储
pub struct RecentStore {
    file: RwLock<PathBuf>,
}

impl RecentStore {
    pub fn new(file: PathBuf) -> Self {
        Self {
            file: RwLock::new(file),
        }
    }

    fn read_all(&self) -> Vec<RecentEntry> {
        let path = self.file.read().clone();
        if !path.exists() {
            return Vec::new();
        }
        match fs::read_to_string(&path) {
            Ok(text) => serde_json::from_str::<Vec<RecentEntry>>(&text).unwrap_or_default(),
            Err(_) => Vec::new(),
        }
    }

    fn write_all(&self, items: &[RecentEntry]) -> AppResult<()> {
        let path = self.file.read().clone();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let text =
            serde_json::to_string_pretty(items).map_err(|e| AppError::Crypto(format!("{e}")))?;
        fs::write(&path, text)?;
        Ok(())
    }

    /// 列出全部条目，按最近访问时间倒序。期间会过滤掉文件已不存在的条目。
    pub fn list(&self) -> Vec<RecentEntry> {
        let mut items = self.read_all();
        items.retain(|it| Path::new(&it.path).exists());
        items.sort_by(|a, b| b.last_access_time.cmp(&a.last_access_time));
        items
    }

    /// 添加或更新一条记录（按 path 去重）
    pub fn upsert(&self, entry: RecentEntry) -> AppResult<()> {
        let mut items = self.read_all();
        items.retain(|it| it.path != entry.path);
        items.insert(0, entry);
        self.write_all(&items)
    }

    pub fn remove(&self, path: &str) -> AppResult<()> {
        let mut items = self.read_all();
        items.retain(|it| it.path != path);
        self.write_all(&items)
    }

    pub fn clear(&self) -> AppResult<()> {
        self.write_all(&[])
    }
}
