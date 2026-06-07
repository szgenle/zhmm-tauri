//! 离线站点词典匹配器：根据 URL 建议中文名与标签。
//!
//! 数据分两层：
//! - 内置词典：编译进二进制（`include_str!`），只读，提供通用基础；
//! - 用户词典：`app_data_dir/site_catalog_user.json`，可导入/导出/重置。
//!
//! 合并策略：用户词典同 host 条目覆盖内置词典。

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::LazyLock;

use crate::errors::{AppError, AppResult};

/// 单个词典条目（返回给前端）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SiteCatalogEntry {
    pub host: String,
    pub name: String,
    pub tags: Vec<String>,
}

/// 站点建议结果
#[derive(Debug, Clone, Serialize)]
pub struct SiteSuggestion {
    pub name: String,
    pub tags: Vec<String>,
    /// 命中来源: "host" | "domain" | "rule" | ""
    pub matched: String,
}

impl SiteSuggestion {
    fn empty() -> Self {
        Self {
            name: String::new(),
            tags: Vec::new(),
            matched: String::new(),
        }
    }
}

/// 内嵌的 JSON 数据
const CATALOG_JSON: &str = include_str!("../../resources/site_catalog.json");

/// 缓存的内置词典（只读）
struct CatalogData {
    sites: HashMap<String, CatalogItem>,
}

#[derive(Clone)]
struct CatalogItem {
    name: String,
    tags: Vec<String>,
}

static BUILTIN_CATALOG: LazyLock<CatalogData> = LazyLock::new(load_builtin_catalog);

fn load_builtin_catalog() -> CatalogData {
    parse_catalog_json(CATALOG_JSON)
}

fn parse_catalog_json(json: &str) -> CatalogData {
    let mut sites = HashMap::new();
    if let Ok(raw) = serde_json::from_str::<serde_json::Value>(json) {
        if let Some(sites_obj) = raw.get("sites").and_then(|s| s.as_object()) {
            for (host, info) in sites_obj {
                let name = info
                    .get("name")
                    .and_then(|n| n.as_str())
                    .unwrap_or("")
                    .to_string();
                let tags: Vec<String> = info
                    .get("tags")
                    .and_then(|t| t.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(|s| s.to_string()))
                            .collect()
                    })
                    .unwrap_or_default();
                sites.insert(host.trim().to_lowercase(), CatalogItem { name, tags });
            }
        }
    }
    CatalogData { sites }
}

// ========== 用户词典状态 ==========

/// 用户词典持久化状态（Tauri managed state）
pub struct UserCatalogState {
    path: PathBuf,
    /// 用户自定义条目（host -> CatalogItem）
    data: RwLock<HashMap<String, CatalogItem>>,
}

impl UserCatalogState {
    pub fn new(path: PathBuf) -> Self {
        let data = Self::load_from_file(&path);
        Self {
            path,
            data: RwLock::new(data),
        }
    }

    fn load_from_file(path: &PathBuf) -> HashMap<String, CatalogItem> {
        if !path.exists() {
            return HashMap::new();
        }
        let Ok(bytes) = fs::read(path) else {
            return HashMap::new();
        };
        let Ok(raw) = serde_json::from_slice::<serde_json::Value>(&bytes) else {
            return HashMap::new();
        };
        let catalog = parse_catalog_json(&raw.to_string());
        catalog.sites
    }

    fn persist(&self) -> AppResult<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        let data = self.data.read();
        let json = Self::to_catalog_json(&data)?;
        fs::write(&self.path, json)?;
        Ok(())
    }

    fn to_catalog_json(data: &HashMap<String, CatalogItem>) -> AppResult<Vec<u8>> {
        let mut sites = serde_json::Map::new();
        let mut hosts: Vec<&String> = data.keys().collect();
        hosts.sort();
        for host in hosts {
            if let Some(item) = data.get(host) {
                let mut entry = serde_json::Map::new();
                entry.insert("name".into(), serde_json::Value::String(item.name.clone()));
                entry.insert(
                    "tags".into(),
                    serde_json::Value::Array(
                        item.tags.iter().map(|t| serde_json::Value::String(t.clone())).collect(),
                    ),
                );
                sites.insert(host.clone(), serde_json::Value::Object(entry));
            }
        }
        let mut root = serde_json::Map::new();
        let mut meta = serde_json::Map::new();
        meta.insert("version".into(), serde_json::Value::Number(2.into()));
        meta.insert(
            "description".into(),
            serde_json::Value::String("用户自定义网站词典".into()),
        );
        let now = chrono::Local::now().format("%Y-%m").to_string();
        meta.insert("updated".into(), serde_json::Value::String(now));
        root.insert("_meta".into(), serde_json::Value::Object(meta));
        root.insert("sites".into(), serde_json::Value::Object(sites));
        let bytes = serde_json::to_vec_pretty(&serde_json::Value::Object(root))?;
        Ok(bytes)
    }

    /// 是否有用户自定义数据
    pub fn has_user_data(&self) -> bool {
        !self.data.read().is_empty()
    }
}

// ========== 合并后查询接口 ==========

/// 返回合并后的全部条目（用户覆盖内置，按 host 升序）
pub fn all_entries_merged(user_state: &UserCatalogState) -> Vec<SiteCatalogEntry> {
    let builtin = &BUILTIN_CATALOG.sites;
    let user_data = user_state.data.read();

    let mut merged: HashMap<&str, (&CatalogItem, bool)> = HashMap::new();
    // 先加内置
    for (host, item) in builtin.iter() {
        merged.insert(host.as_str(), (item, false));
    }
    // 用户覆盖
    for (host, item) in user_data.iter() {
        merged.insert(host.as_str(), (item, true));
    }

    let mut result: Vec<SiteCatalogEntry> = merged
        .into_iter()
        .map(|(host, (item, _))| SiteCatalogEntry {
            host: host.to_string(),
            name: item.name.clone(),
            tags: item.tags.clone(),
        })
        .collect();
    result.sort_by(|a, b| a.host.cmp(&b.host));
    result
}

/// 根据 URL 或 host 给出中文名 + 建议标签（合并用户词典）
pub fn suggest_merged(url_or_host: &str, user_state: &UserCatalogState) -> SiteSuggestion {
    let host = extract_host(url_or_host);
    if host.is_empty() {
        return SiteSuggestion::empty();
    }

    let user_data = user_state.data.read();

    // 优先查用户词典
    if let Some(item) = user_data.get(&host) {
        return SiteSuggestion {
            name: item.name.clone(),
            tags: item.tags.clone(),
            matched: "host".into(),
        };
    }
    let domain = registrable_domain(&host);
    if !domain.is_empty() && domain != host {
        if let Some(item) = user_data.get(&domain) {
            return SiteSuggestion {
                name: item.name.clone(),
                tags: item.tags.clone(),
                matched: "domain".into(),
            };
        }
    }

    // 再查内置词典
    let builtin = &BUILTIN_CATALOG.sites;

    if let Some(item) = builtin.get(&host) {
        return SiteSuggestion {
            name: item.name.clone(),
            tags: item.tags.clone(),
            matched: "host".into(),
        };
    }

    if !domain.is_empty() && domain != host {
        if let Some(item) = builtin.get(&domain) {
            return SiteSuggestion {
                name: item.name.clone(),
                tags: item.tags.clone(),
                matched: "domain".into(),
            };
        }
    }

    // 兜底规则
    let rule_tags = apply_rules(&host);
    if !rule_tags.is_empty() {
        return SiteSuggestion {
            name: String::new(),
            tags: rule_tags,
            matched: "rule".into(),
        };
    }

    SiteSuggestion::empty()
}

// ========== 导入/导出/重置 ==========

/// 收集词典中所有出现过的标签（去重、排序），用于前端筛选
pub fn all_tags(user_state: &UserCatalogState) -> Vec<String> {
    let entries = all_entries_merged(user_state);
    let mut set: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    for entry in &entries {
        for tag in &entry.tags {
            let t = tag.trim().to_string();
            if !t.is_empty() {
                set.insert(t);
            }
        }
    }
    set.into_iter().collect()
}

/// 导出合并后的完整词典到指定路径，同时补充密码库中尚未收录的站点
/// `filter_tags` 为空时导出全部；非空时只导出含有任一指定标签的条目
/// `exclude_tags` 非空时排除含有任一指定标签的条目（对词典条目和密码库补充条目均生效）
pub fn export_catalog(
    user_state: &UserCatalogState,
    dest: &str,
    vault_hosts: &[(String, Vec<String>)],
    filter_tags: &[String],
    exclude_tags: &[String],
) -> AppResult<usize> {
    let entries = all_entries_merged(user_state);

    // 如果指定了 include 标签筛选，只保留含有任一标签的条目
    let entries: Vec<SiteCatalogEntry> = if filter_tags.is_empty() {
        entries
    } else {
        let filter_set: std::collections::HashSet<&str> =
            filter_tags.iter().map(|t| t.as_str()).collect();
        entries
            .into_iter()
            .filter(|e| e.tags.iter().any(|t| filter_set.contains(t.as_str())))
            .collect()
    };

    // 已有的 host 集合（用 owned String 避免借用冲突）
    let existing: std::collections::HashSet<String> =
        entries.iter().map(|e| e.host.clone()).collect();

    // 从密码库补充尚未收录的 host（带标签，仅在未指定 include 筛选时补充）
    let mut all = entries;
    if filter_tags.is_empty() {
        for (host, tags) in vault_hosts {
            let h = host.trim().to_lowercase();
            if !h.is_empty() && !existing.contains(&h) {
                all.push(SiteCatalogEntry {
                    host: h,
                    name: String::new(),
                    tags: tags.clone(),
                });
            }
        }
    }

    // 排除含有指定标签的条目（对词典和密码库补充条目均生效）
    let all: Vec<SiteCatalogEntry> = if exclude_tags.is_empty() {
        all
    } else {
        let exclude_set: std::collections::HashSet<&str> =
            exclude_tags.iter().map(|t| t.as_str()).collect();
        all.into_iter()
            .filter(|e| !e.tags.iter().any(|t| exclude_set.contains(t.as_str())))
            .collect()
    };

    let mut all = all;
    all.sort_by(|a, b| a.host.cmp(&b.host));
    let count = all.len();

    let mut sites = serde_json::Map::new();
    for entry in &all {
        let mut obj = serde_json::Map::new();
        obj.insert("name".into(), serde_json::Value::String(entry.name.clone()));
        obj.insert(
            "tags".into(),
            serde_json::Value::Array(
                entry.tags.iter().map(|t| serde_json::Value::String(t.clone())).collect(),
            ),
        );
        sites.insert(entry.host.clone(), serde_json::Value::Object(obj));
    }

    let mut root = serde_json::Map::new();
    let mut meta = serde_json::Map::new();
    meta.insert("version".into(), serde_json::Value::Number(2.into()));
    meta.insert(
        "description".into(),
        serde_json::Value::String("网站词典导出（可外部编辑后重新导入）".into()),
    );
    let now = chrono::Local::now().format("%Y-%m-%d").to_string();
    meta.insert("updated".into(), serde_json::Value::String(now));
    root.insert("_meta".into(), serde_json::Value::Object(meta));
    root.insert("sites".into(), serde_json::Value::Object(sites));

    let json = serde_json::to_vec_pretty(&serde_json::Value::Object(root))?;
    fs::write(dest, json)?;
    Ok(count)
}

/// 从 JSON 文件导入为用户词典（完全替换用户层）
pub fn import_catalog(user_state: &UserCatalogState, src: &str) -> AppResult<usize> {
    let bytes = fs::read(src).map_err(|e| AppError::Other(format!("读取文件失败: {e}")))?;
    let raw: serde_json::Value =
        serde_json::from_slice(&bytes).map_err(|e| AppError::Other(format!("JSON 解析失败: {e}")))?;

    let sites_obj = raw
        .get("sites")
        .and_then(|s| s.as_object())
        .ok_or_else(|| AppError::Other("JSON 格式错误：缺少 sites 字段".into()))?;

    let mut new_data: HashMap<String, CatalogItem> = HashMap::new();
    for (host, info) in sites_obj {
        let name = info
            .get("name")
            .and_then(|n| n.as_str())
            .unwrap_or("")
            .to_string();
        let tags: Vec<String> = info
            .get("tags")
            .and_then(|t| t.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
            .unwrap_or_default();
        new_data.insert(host.trim().to_lowercase(), CatalogItem { name, tags });
    }

    let count = new_data.len();
    *user_state.data.write() = new_data;
    user_state.persist()?;
    Ok(count)
}

/// 重置用户词典（清空用户层，恢复为纯内置）
pub fn reset_user_catalog(user_state: &UserCatalogState) -> AppResult<()> {
    user_state.data.write().clear();
    // 删除用户词典文件
    if user_state.path.exists() {
        fs::remove_file(&user_state.path)?;
    }
    Ok(())
}

// ========== 兼容旧接口（不依赖 state 的只读内置查询） ==========

/// 返回内置词典全部条目（按 host 升序）—— 仅在无 state 时使用
#[allow(dead_code)]
pub fn all_entries() -> Vec<SiteCatalogEntry> {
    let catalog = &*BUILTIN_CATALOG;
    let mut result: Vec<SiteCatalogEntry> = catalog
        .sites
        .iter()
        .map(|(host, item)| SiteCatalogEntry {
            host: host.clone(),
            name: item.name.clone(),
            tags: item.tags.clone(),
        })
        .collect();
    result.sort_by(|a, b| a.host.cmp(&b.host));
    result
}

/// 根据 URL 或 host 给出中文名 + 建议标签（仅内置词典）
#[allow(dead_code)]
pub fn suggest(url_or_host: &str) -> SiteSuggestion {
    let host = extract_host(url_or_host);
    if host.is_empty() {
        return SiteSuggestion::empty();
    }

    let catalog = &*BUILTIN_CATALOG;

    // 1. 完整 host 精确匹配
    if let Some(item) = catalog.sites.get(&host) {
        return SiteSuggestion {
            name: item.name.clone(),
            tags: item.tags.clone(),
            matched: "host".into(),
        };
    }

    // 2. 根域名精确匹配
    let domain = registrable_domain(&host);
    if !domain.is_empty() && domain != host {
        if let Some(item) = catalog.sites.get(&domain) {
            return SiteSuggestion {
                name: item.name.clone(),
                tags: item.tags.clone(),
                matched: "domain".into(),
            };
        }
    }

    // 3. 兜底规则（后缀 + 关键字）
    let rule_tags = apply_rules(&host);
    if !rule_tags.is_empty() {
        return SiteSuggestion {
            name: String::new(),
            tags: rule_tags,
            matched: "rule".into(),
        };
    }

    SiteSuggestion::empty()
}

// ========== 辅助函数 ==========

/// 从 URL 或 host 中提取纯 host（小写）
fn extract_host(input: &str) -> String {
    let s = input.trim().to_lowercase();
    if s.is_empty() {
        return String::new();
    }
    // 尝试解析为 URL
    if let Ok(url) = url::Url::parse(&s) {
        return url.host_str().unwrap_or("").to_string();
    }
    // 尝试加 scheme 后解析
    if let Ok(url) = url::Url::parse(&format!("https://{s}")) {
        return url.host_str().unwrap_or("").to_string();
    }
    // 移除端口和路径
    s.split('/')
        .next()
        .unwrap_or("")
        .split(':')
        .next()
        .unwrap_or("")
        .to_string()
}

/// 提取可注册域名（简化版：取最后两段，特殊处理 .com.cn 等）
fn registrable_domain(host: &str) -> String {
    let parts: Vec<&str> = host.split('.').collect();
    if parts.len() <= 2 {
        return host.to_string();
    }
    // 特殊二级后缀
    let special_suffixes = [
        "com.cn", "net.cn", "org.cn", "gov.cn", "edu.cn", "co.uk", "co.jp", "co.kr", "com.au",
        "com.br",
    ];
    let last_two = format!("{}.{}", parts[parts.len() - 2], parts[parts.len() - 1]);
    for suffix in &special_suffixes {
        if last_two == *suffix && parts.len() > 2 {
            // 取最后三段
            if parts.len() >= 3 {
                return format!("{}.{}", parts[parts.len() - 3], last_two);
            }
        }
    }
    last_two
}

/// 兜底规则：按后缀和关键字推断标签
fn apply_rules(host: &str) -> Vec<String> {
    let mut tags = Vec::new();

    // 后缀规则
    let suffix_rules: &[(&str, &str)] = &[
        (".gov.cn", "政务"),
        (".edu.cn", "教育"),
        (".edu", "教育"),
        (".gov", "政务"),
        (".mil", "军事"),
        (".ac.cn", "学术"),
    ];
    for (suffix, tag) in suffix_rules {
        if host.ends_with(suffix) {
            tags.push(tag.to_string());
        }
    }

    // 关键字规则
    let keyword_rules: &[(&str, &[&str])] = &[
        ("bank", &["金融", "银行"]),
        ("pay", &["金融", "支付"]),
        ("mail", &["邮箱"]),
        ("cloud", &["云服务"]),
        ("shop", &["购物"]),
        ("game", &["游戏"]),
    ];
    for (keyword, kw_tags) in keyword_rules {
        if host.contains(keyword) {
            for t in *kw_tags {
                if !tags.contains(&t.to_string()) {
                    tags.push(t.to_string());
                }
            }
        }
    }

    tags
}
