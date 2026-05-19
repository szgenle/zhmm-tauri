//! 离线站点词典匹配器：根据 URL 建议中文名与标签。
//!
//! 数据内嵌在二进制中（`include_str!`），完全离线。

use serde::Serialize;
use std::collections::HashMap;
use std::sync::LazyLock;

/// 单个词典条目（返回给前端）
#[derive(Debug, Clone, Serialize)]
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

/// 缓存的词典
struct CatalogData {
    sites: HashMap<String, CatalogItem>,
}

struct CatalogItem {
    name: String,
    tags: Vec<String>,
}

static CATALOG: LazyLock<CatalogData> = LazyLock::new(|| load_catalog());

fn load_catalog() -> CatalogData {
    let mut sites = HashMap::new();
    if let Ok(raw) = serde_json::from_str::<serde_json::Value>(CATALOG_JSON) {
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

/// 返回词典全部条目（按 host 升序）
pub fn all_entries() -> Vec<SiteCatalogEntry> {
    let catalog = &*CATALOG;
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

/// 根据 URL 或 host 给出中文名 + 建议标签
pub fn suggest(url_or_host: &str) -> SiteSuggestion {
    let host = extract_host(url_or_host);
    if host.is_empty() {
        return SiteSuggestion::empty();
    }

    let catalog = &*CATALOG;

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
    s.split('/').next().unwrap_or("").split(':').next().unwrap_or("").to_string()
}

/// 提取可注册域名（简化版：取最后两段，特殊处理 .com.cn 等）
fn registrable_domain(host: &str) -> String {
    let parts: Vec<&str> = host.split('.').collect();
    if parts.len() <= 2 {
        return host.to_string();
    }
    // 特殊二级后缀
    let special_suffixes = [
        "com.cn", "net.cn", "org.cn", "gov.cn", "edu.cn",
        "co.uk", "co.jp", "co.kr", "com.au", "com.br",
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
