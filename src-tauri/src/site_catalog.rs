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

/// 内嵌的 JSON 数据（复用 catalogs.next/sites.json 站点全集，唯一事实源）
const CATALOG_JSON: &str = include_str!("../../resources/catalogs.next/sites.json");

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
    /// 用户选择的一级标签集合（来自 _meta.primary_tags）
    primary_tags: RwLock<Vec<String>>,
}

impl UserCatalogState {
    pub fn new(path: PathBuf) -> Self {
        let (data, primary_tags) = Self::load_from_file(&path);
        Self {
            path,
            data: RwLock::new(data),
            primary_tags: RwLock::new(primary_tags),
        }
    }

    fn load_from_file(path: &PathBuf) -> (HashMap<String, CatalogItem>, Vec<String>) {
        if !path.exists() {
            return (HashMap::new(), Vec::new());
        }
        let Ok(bytes) = fs::read(path) else {
            return (HashMap::new(), Vec::new());
        };
        let Ok(raw) = serde_json::from_slice::<serde_json::Value>(&bytes) else {
            return (HashMap::new(), Vec::new());
        };
        let primary_tags: Vec<String> = raw
            .get("_meta")
            .and_then(|m| m.get("primary_tags"))
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();
        let catalog = parse_catalog_json(&raw.to_string());
        (catalog.sites, primary_tags)
    }

    fn persist(&self) -> AppResult<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        let data = self.data.read();
        let primary_tags = self.primary_tags.read();
        let json = Self::to_catalog_json(&data, &primary_tags)?;
        fs::write(&self.path, json)?;
        Ok(())
    }

    fn to_catalog_json(
        data: &HashMap<String, CatalogItem>,
        primary_tags: &[String],
    ) -> AppResult<Vec<u8>> {
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
        meta.insert("version".into(), serde_json::Value::Number(3.into()));
        meta.insert(
            "description".into(),
            serde_json::Value::String("用户自定义网站词典".into()),
        );
        let now = chrono::Local::now().format("%Y-%m").to_string();
        meta.insert("updated".into(), serde_json::Value::String(now));
        meta.insert(
            "primary_tags".into(),
            serde_json::Value::Array(
                primary_tags
                    .iter()
                    .map(|t| serde_json::Value::String(t.clone()))
                    .collect(),
            ),
        );
        root.insert("_meta".into(), serde_json::Value::Object(meta));
        root.insert("sites".into(), serde_json::Value::Object(sites));
        let bytes = serde_json::to_vec_pretty(&serde_json::Value::Object(root))?;
        Ok(bytes)
    }

    /// 是否有用户自定义数据
    pub fn has_user_data(&self) -> bool {
        !self.data.read().is_empty() || !self.primary_tags.read().is_empty()
    }

    /// 获取用户选择的一级标签集合
    pub fn get_primary_tags(&self) -> Vec<String> {
        self.primary_tags.read().clone()
    }

    /// 设置用户选择的一级标签集合（去重、过滤空字符串、保持调用方传入顺序）
    pub fn set_primary_tags(&self, tags: Vec<String>) -> AppResult<()> {
        let mut seen = std::collections::HashSet::new();
        let cleaned: Vec<String> = tags
            .into_iter()
            .map(|t| t.trim().to_string())
            .filter(|t| !t.is_empty() && seen.insert(t.clone()))
            .collect();
        *self.primary_tags.write() = cleaned;
        self.persist()
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
/// `vault_override` 为 true 时：对密库中已存在的 host，用密库 tags 覆盖词典原 tags
///                  （name 保留词典原值；密库 tags 为空则不覆盖，避免清空已有标签）
/// `scope` 取值：
///   - "all"    ：导出全部（默认行为，包含密库未收录补充）
///   - "used"   ：仅导出密库中已使用的 host（词典中存在 + 密库补充均算"已用"）
///   - "unused" ：仅导出密库中未使用的 host（词典里有但密库未录入；不补充密库 host）
pub fn export_catalog(
    user_state: &UserCatalogState,
    dest: &str,
    vault_hosts: &[(String, Vec<String>)],
    filter_tags: &[String],
    exclude_tags: &[String],
    vault_override: bool,
    scope: &str,
) -> AppResult<usize> {
    let entries = all_entries_merged(user_state);

    // 构建密库 host → tags 索引（host 已小写）
    let vault_map: std::collections::HashMap<String, Vec<String>> = vault_hosts
        .iter()
        .map(|(h, tags)| (h.trim().to_lowercase(), tags.clone()))
        .filter(|(h, _)| !h.is_empty())
        .collect();

    // 当开启 vault_override 时，先用密库 tags 覆盖同 host 词典条目的 tags
    let entries: Vec<SiteCatalogEntry> = if vault_override {
        entries
            .into_iter()
            .map(|mut e| {
                if let Some(vt) = vault_map.get(&e.host) {
                    if !vt.is_empty() {
                        e.tags = vt.clone();
                    }
                }
                e
            })
            .collect()
    } else {
        entries
    };

    // 范围筛选（在 include/exclude 之前作用于词典条目）
    let scope_norm = scope.trim().to_lowercase();
    let entries: Vec<SiteCatalogEntry> = match scope_norm.as_str() {
        "used" => entries
            .into_iter()
            .filter(|e| vault_map.contains_key(&e.host))
            .collect(),
        "unused" => entries
            .into_iter()
            .filter(|e| !vault_map.contains_key(&e.host))
            .collect(),
        _ => entries, // "all" 或未识别值 → 不过滤
    };

    // 如果指定了 include 标签筛选，只保留含有任一标签的条目（在覆盖之后生效）
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

    // 从密码库补充尚未收录的 host（带标签）
    // 规则：仅在 "all" 或 "used" 范围、且未指定 include 筛选时补充；
    //      "unused" 范围下补充的都是已用 host，与语义冲突，故不补充。
    let mut all = entries;
    let allow_supplement = filter_tags.is_empty() && scope_norm.as_str() != "unused";
    if allow_supplement {
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
    user_state.primary_tags.write().clear();
    // 删除用户词典文件
    if user_state.path.exists() {
        fs::remove_file(&user_state.path)?;
    }
    Ok(())
}

// ========== 预制身份词典（resources/catalogs.next/，编译进二进制） ==========
//
// 数据布局（与旧 catalogs/<persona>.json 不同）：
// - sites.json：站点全集，唯一事实源（与身份解耦，每个 site 的 tags 中已注入身份标签）
// - personas/<id>.json：每个身份只保留 _meta（含 primary_tags），不再包含 sites 子集
//
// 用户选定 persona 时，导入行为为：
// - 把 sites.json 全集合并进用户词典（按 override_tags 三态归类：added/overwritten/kept）
// - 把对应 persona 的 primary_tags 追加到用户 primary_tags（去重，保留用户已有顺序）

const PRESET_SITES_JSON: &str = include_str!("../../resources/catalogs.next/sites.json");

const PRESET_PERSONA_BASE_JSON: &str =
    include_str!("../../resources/catalogs.next/personas/base.json");
const PRESET_PERSONA_DEVELOPER_JSON: &str =
    include_str!("../../resources/catalogs.next/personas/developer.json");
const PRESET_PERSONA_GAME_DEV_JSON: &str =
    include_str!("../../resources/catalogs.next/personas/game-dev.json");
const PRESET_PERSONA_CREATOR_JSON: &str =
    include_str!("../../resources/catalogs.next/personas/creator.json");
const PRESET_PERSONA_SMALL_BIZ_JSON: &str =
    include_str!("../../resources/catalogs.next/personas/small-biz.json");
const PRESET_PERSONA_CROSS_BORDER_JSON: &str =
    include_str!("../../resources/catalogs.next/personas/cross-border.json");

/// 单份 persona 的元信息（不含 sites 子集——sites 是全集，与身份解耦）
struct PresetPersona {
    id: String,
    description: String,
    primary_tags: Vec<String>,
    community: bool,
    default_enabled: bool,
}

/// 给前端的预制身份元信息
///
/// 字段命名沿用旧版 `persona`（而非 `id`）保持前端兼容；
/// `site_count` 对所有 persona 取同一值——sites.json 全集大小。
#[derive(Debug, Clone, Serialize)]
pub struct PresetPersonaInfo {
    pub persona: String,
    pub description: String,
    pub site_count: usize,
    pub primary_tags: Vec<String>,
    pub community: bool,
    pub default_enabled: bool,
}

/// 导入预制词典后的统计结果
#[derive(Debug, Clone, Serialize)]
pub struct PresetImportResult {
    /// host 不存在 → 新增
    pub added: usize,
    /// host 已存在且 override_tags=true → 用预制覆盖
    pub overwritten: usize,
    /// host 已存在且 override_tags=false → 保留用户当前数据
    pub kept: usize,
}

/// 站点全集（解析自 sites.json）
static PRESET_SITES_NEXT: LazyLock<HashMap<String, CatalogItem>> =
    LazyLock::new(|| parse_catalog_json(PRESET_SITES_JSON).sites);

/// 全部 persona 元信息（解析自 personas/*.json），顺序与 PERSONA_FILES 对应
static PRESET_PERSONAS_NEXT: LazyLock<Vec<PresetPersona>> = LazyLock::new(|| {
    let raws = [
        PRESET_PERSONA_BASE_JSON,
        PRESET_PERSONA_DEVELOPER_JSON,
        PRESET_PERSONA_GAME_DEV_JSON,
        PRESET_PERSONA_CREATOR_JSON,
        PRESET_PERSONA_SMALL_BIZ_JSON,
        PRESET_PERSONA_CROSS_BORDER_JSON,
    ];
    raws.iter().filter_map(|json| parse_preset_persona(json)).collect()
});

fn parse_preset_persona(json: &str) -> Option<PresetPersona> {
    let raw: serde_json::Value = serde_json::from_str(json).ok()?;
    let meta = raw.get("_meta")?.as_object()?;
    let id = meta.get("id")?.as_str()?.to_string();
    let description = meta
        .get("description")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let primary_tags: Vec<String> = meta
        .get("primary_tags")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();
    let community = meta
        .get("community")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    // 社区词典默认 default_enabled=false；其它默认 true
    let default_enabled = meta
        .get("default_enabled")
        .and_then(|v| v.as_bool())
        .unwrap_or(!community);

    Some(PresetPersona {
        id,
        description,
        primary_tags,
        community,
        default_enabled,
    })
}

/// 列出全部预制身份的元信息
pub fn list_preset_personas() -> Vec<PresetPersonaInfo> {
    let total_sites = PRESET_SITES_NEXT.len();
    PRESET_PERSONAS_NEXT
        .iter()
        .map(|p| PresetPersonaInfo {
            persona: p.id.clone(),
            description: p.description.clone(),
            site_count: total_sites,
            primary_tags: p.primary_tags.clone(),
            community: p.community,
            default_enabled: p.default_enabled,
        })
        .collect()
}

/// 把指定 persona 对应的「站点全集 + primary_tags」并入用户词典层。
///
/// 站点合并三态（基于「内置 + 用户层」合并视图判断 host 是否已存在）：
/// - host 不存在 → 写入用户层（added）
/// - host 已存在 && override_tags=true → 用预制 name+tags 覆盖（overwritten）
/// - host 已存在 && override_tags=false → 保留用户当前数据不动（kept）
///
/// 一级标签：persona 的 primary_tags 追加到用户 primary_tags（去重，保留用户已有顺序）
pub fn import_preset_persona(
    user_state: &UserCatalogState,
    persona_id: &str,
    override_tags: bool,
) -> AppResult<PresetImportResult> {
    let persona = PRESET_PERSONAS_NEXT
        .iter()
        .find(|p| p.id == persona_id)
        .ok_or_else(|| AppError::Other(format!("未找到预制身份: {persona_id}")))?;

    let builtin_hosts: std::collections::HashSet<&str> =
        BUILTIN_CATALOG.sites.keys().map(|s| s.as_str()).collect();

    let mut added = 0usize;
    let mut overwritten = 0usize;
    let mut kept = 0usize;

    {
        let mut user_data = user_state.data.write();
        for (host, item) in PRESET_SITES_NEXT.iter() {
            let exists_in_user = user_data.contains_key(host);
            let exists_in_builtin = builtin_hosts.contains(host.as_str());
            let exists = exists_in_user || exists_in_builtin;

            if !exists {
                user_data.insert(host.clone(), item.clone());
                added += 1;
            } else if override_tags {
                user_data.insert(host.clone(), item.clone());
                overwritten += 1;
            } else {
                kept += 1;
            }
        }
    }

    // 合并 persona 的 primary_tags（追加去重，保留用户已有顺序）
    {
        let mut pt = user_state.primary_tags.write();
        let mut seen: std::collections::HashSet<String> = pt.iter().cloned().collect();
        for t in &persona.primary_tags {
            if seen.insert(t.clone()) {
                pt.push(t.clone());
            }
        }
    }

    user_state.persist()?;

    Ok(PresetImportResult {
        added,
        overwritten,
        kept,
    })
}

/// 兼容旧接口：转发到 `import_preset_persona`。
///
/// 旧签名 `(persona, override_tags)`，行为现等价于新版「全集导入 + persona primary_tags 注入」。
/// 保留一段时间以兼容尚未升级的前端调用方，新代码请使用 `import_preset_persona`。
#[deprecated(note = "请使用 import_preset_persona")]
pub fn import_preset_catalog(
    user_state: &UserCatalogState,
    persona: &str,
    override_tags: bool,
) -> AppResult<PresetImportResult> {
    import_preset_persona(user_state, persona, override_tags)
}

// ========== 标签统计与一级标签管理 ==========

/// 单个标签的统计信息（合并 builtin + 用户词典）
#[derive(Debug, Clone, Serialize)]
pub struct CatalogTagStat {
    pub tag: String,
    pub count: usize,
    pub is_primary: bool,
}

/// 列出合并视图（builtin + user）中所有出现过的标签 + 频次 + 是否一级
pub fn list_tag_stats(user_state: &UserCatalogState) -> Vec<CatalogTagStat> {
    let entries = all_entries_merged(user_state);
    let primary: std::collections::HashSet<String> =
        user_state.get_primary_tags().into_iter().collect();

    let mut counts: HashMap<String, usize> = HashMap::new();
    for e in &entries {
        for t in &e.tags {
            let trimmed = t.trim().to_string();
            if trimmed.is_empty() {
                continue;
            }
            *counts.entry(trimmed).or_insert(0) += 1;
        }
    }

    let mut result: Vec<CatalogTagStat> = counts
        .into_iter()
        .map(|(tag, count)| CatalogTagStat {
            is_primary: primary.contains(&tag),
            tag,
            count,
        })
        .collect();
    // 排序：一级在前，频次降序，再按标签字典序
    result.sort_by(|a, b| {
        b.is_primary
            .cmp(&a.is_primary)
            .then(b.count.cmp(&a.count))
            .then(a.tag.cmp(&b.tag))
    });
    result
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
