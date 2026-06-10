//! 杂项命令：站点词典、主密码、防截屏、最近访问、bcrypt、文件存在性

use tauri::State;

use crate::accounts::{RecentEntry, RecentStore};
use crate::errors::{AppError, AppResult};
use crate::site_catalog;
use crate::vault::VaultState;

// ========== 站点词典 ==========

#[tauri::command]
pub fn list_site_catalog(
    user_catalog: State<'_, site_catalog::UserCatalogState>,
) -> Vec<site_catalog::SiteCatalogEntry> {
    site_catalog::all_entries_merged(&user_catalog)
}

#[tauri::command]
pub fn suggest_site(
    url_or_host: String,
    user_catalog: State<'_, site_catalog::UserCatalogState>,
) -> site_catalog::SiteSuggestion {
    site_catalog::suggest_merged(&url_or_host, &user_catalog)
}

/// 导出合并后的完整词典到指定路径，返回条目数
/// 同时会自动补充密码库中已录入但词典未收录的站点（空名/空标签，供外部分类）
/// `filter_tags` 为空时导出全部；非空时只导出含有指定标签的条目
/// `exclude_tags` 非空时排除含有指定标签的条目
/// `vault_override` 为 true 时：对密库中已存在的 host，用密库当前 tags 覆盖词典 tags
/// `scope` 取值："all" / "used" / "unused"，分别表示全部、仅密库已用、仅密库未用
#[tauri::command]
pub fn export_site_catalog(
    path: String,
    filter_tags: Vec<String>,
    exclude_tags: Vec<String>,
    vault_override: bool,
    scope: String,
    user_catalog: State<'_, site_catalog::UserCatalogState>,
    vault: State<'_, VaultState>,
) -> AppResult<usize> {
    // 尝试从密码库收集 hosts（未解锁时跳过，不影响导出）
    let vault_hosts = vault.collect_url_hosts().unwrap_or_default();
    site_catalog::export_catalog(
        &user_catalog,
        &path,
        &vault_hosts,
        &filter_tags,
        &exclude_tags,
        vault_override,
        &scope,
    )
}

/// 列出所有可用标签（词典标签 + 密码库中用户自定义标签，去重、排序）
#[tauri::command]
pub fn list_catalog_tags(
    user_catalog: State<'_, site_catalog::UserCatalogState>,
    vault: State<'_, VaultState>,
) -> Vec<String> {
    let mut tags: std::collections::BTreeSet<String> =
        site_catalog::all_tags(&user_catalog).into_iter().collect();
    // 合并密码库中用户自己添加的标签
    if let Ok(counts) = vault.collect_tag_counts() {
        for (tag, _) in counts {
            tags.insert(tag);
        }
    }
    tags.into_iter().collect()
}

/// 从 JSON 文件导入为用户词典（完全替换用户层），返回导入条目数
#[tauri::command]
pub fn import_site_catalog(
    path: String,
    user_catalog: State<'_, site_catalog::UserCatalogState>,
) -> AppResult<usize> {
    site_catalog::import_catalog(&user_catalog, &path)
}

/// 重置用户词典，恢复为纯内置
#[tauri::command]
pub fn reset_site_catalog(
    user_catalog: State<'_, site_catalog::UserCatalogState>,
) -> AppResult<bool> {
    site_catalog::reset_user_catalog(&user_catalog)?;
    Ok(true)
}

/// 用户是否有自定义词典数据
#[tauri::command]
pub fn has_user_catalog(
    user_catalog: State<'_, site_catalog::UserCatalogState>,
) -> bool {
    user_catalog.has_user_data()
}

/// 列出所有预制身份词典（base / developer / game-dev / cross-border / creator / small-biz / crypto）
#[tauri::command]
pub fn list_preset_personas() -> Vec<site_catalog::PresetPersonaInfo> {
    site_catalog::list_preset_personas()
}

/// 把指定预制身份的「站点全集 + primary_tags」合并到用户词典层。
///
/// - `persona_id`：预制身份标识（base / developer / game-dev / creator / small-biz / cross-border）
/// - `override_tags`：同 host 已存在时是否用预制 name+tags 覆盖（默认 false 保留用户数据）
///
/// 行为：
/// - 把 sites.json 全集合并进用户词典（按 override_tags 三态归类）
/// - 把 persona 的 primary_tags 追加到用户 primary_tags（去重，保留用户已有顺序）
///
/// 返回 added / overwritten / kept 三态计数。
#[tauri::command]
pub fn import_preset_persona(
    persona_id: String,
    override_tags: bool,
    user_catalog: State<'_, site_catalog::UserCatalogState>,
) -> AppResult<site_catalog::PresetImportResult> {
    site_catalog::import_preset_persona(&user_catalog, &persona_id, override_tags)
}

/// 兼容旧接口：行为等价于 `import_preset_persona`，保留一段时间以兼容旧前端。
#[tauri::command]
pub fn import_preset_catalog(
    persona: String,
    override_tags: bool,
    user_catalog: State<'_, site_catalog::UserCatalogState>,
) -> AppResult<site_catalog::PresetImportResult> {
    #[allow(deprecated)]
    site_catalog::import_preset_catalog(&user_catalog, &persona, override_tags)
}

/// 列出合并视图中全部标签 + 频次 + 是否为用户选定的一级标签
#[tauri::command]
pub fn list_catalog_tag_stats(
    user_catalog: State<'_, site_catalog::UserCatalogState>,
) -> Vec<site_catalog::CatalogTagStat> {
    site_catalog::list_tag_stats(&user_catalog)
}

/// 获取用户选择的一级标签集合
#[tauri::command]
pub fn get_user_primary_tags(
    user_catalog: State<'_, site_catalog::UserCatalogState>,
) -> Vec<String> {
    user_catalog.get_primary_tags()
}

/// 设置用户选择的一级标签集合（去重、过滤空字符串）
#[tauri::command]
pub fn set_user_primary_tags(
    tags: Vec<String>,
    user_catalog: State<'_, site_catalog::UserCatalogState>,
) -> AppResult<()> {
    user_catalog.set_primary_tags(tags)
}

// ========== 主密码管理 ==========

#[tauri::command]
pub fn verify_master_password(password: String, state: State<'_, VaultState>) -> AppResult<bool> {
    state.verify_master_password(&password)
}

#[tauri::command]
pub fn rekey_vault(
    old_password: String,
    new_password: String,
    state: State<'_, VaultState>,
) -> AppResult<String> {
    state.rekey(&old_password, &new_password)
}

// ========== 防截屏 ==========

/// 对主窗口应用 / 撤销防截屏保护
#[tauri::command]
pub fn apply_anti_capture(window: tauri::WebviewWindow, enabled: bool) -> bool {
    #[cfg(target_os = "macos")]
    {
        if let Ok(ns) = window.ns_window() {
            return crate::anti_capture::apply_macos(ns as *mut _, enabled);
        }
    }
    #[cfg(target_os = "windows")]
    {
        if let Ok(hwnd) = window.hwnd() {
            return crate::anti_capture::apply_windows(hwnd.0 as *mut _, enabled);
        }
    }
    let _ = (window, enabled);
    false
}

// ========== 最近访问列表 ==========

#[tauri::command]
pub fn list_recent(store: State<'_, RecentStore>) -> Vec<RecentEntry> {
    store.list()
}

#[tauri::command]
pub fn upsert_recent(entry: RecentEntry, store: State<'_, RecentStore>) -> AppResult<()> {
    store.upsert(entry)
}

#[tauri::command]
pub fn remove_recent(path: String, store: State<'_, RecentStore>) -> AppResult<()> {
    store.remove(&path)
}

#[tauri::command]
pub fn clear_recent(store: State<'_, RecentStore>) -> AppResult<()> {
    store.clear()
}

// ========== bcrypt（最近访问列表的 UI 层快速密码预校验） ==========

#[tauri::command]
pub fn bcrypt_hash(password: String) -> AppResult<String> {
    bcrypt::hash(password, bcrypt::DEFAULT_COST)
        .map_err(|e| AppError::Crypto(format!("bcrypt hash: {e}")))
}

#[tauri::command]
pub fn bcrypt_verify(password: String, hash: String) -> AppResult<bool> {
    bcrypt::verify(password, &hash).map_err(|e| AppError::Crypto(format!("bcrypt verify: {e}")))
}

// ========== Favicon 缓存 ==========

/// 获取指定域名的 favicon（base64 data-url）。
/// 优先从本地缓存读取；未命中时直接从网站抓取 /favicon.ico 并缓存。
/// 下载失败会写入 ".failed" 标记，后续不再重试。
#[tauri::command]
pub fn cache_favicon(app: tauri::AppHandle, domain: String) -> AppResult<String> {
    use std::io::Read;
    use tauri::Manager;

    let cache_dir = app
        .path()
        .app_cache_dir()
        .map_err(|e| AppError::Other(format!("获取缓存目录失败: {e}")))?;
    let favicon_dir = cache_dir.join("favicons");
    std::fs::create_dir_all(&favicon_dir)?;

    // 域名安全化为文件名
    let safe_name: String = domain
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '.' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let file_path = favicon_dir.join(format!("{safe_name}.png"));
    let failed_marker = favicon_dir.join(format!("{safe_name}.failed"));

    // 命中缓存
    if file_path.exists() {
        let data = std::fs::read(&file_path)?;
        return Ok(format!(
            "data:image/png;base64,{}",
            base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &data)
        ));
    }

    // 已知失败，不再重试
    if failed_marker.exists() {
        return Err(AppError::Other("favicon 已标记为不可用".into()));
    }

    // 直接从网站抓取 favicon.ico（3 秒超时）
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(std::time::Duration::from_secs(3))
        .timeout_read(std::time::Duration::from_secs(3))
        .redirects(3)
        .build();

    let url = format!("https://{domain}/favicon.ico");
    let result = agent.get(&url).call();

    let data = match result {
        Ok(resp) => {
            let mut buf = Vec::new();
            resp.into_reader().take(64 * 1024).read_to_end(&mut buf)?;
            if buf.is_empty() {
                let _ = std::fs::write(&failed_marker, b"");
                return Err(AppError::Other("favicon 为空".into()));
            }
            buf
        }
        Err(_) => {
            // HTTPS 失败则尝试 HTTP
            let url_http = format!("http://{domain}/favicon.ico");
            match agent.get(&url_http).call() {
                Ok(resp) => {
                    let mut buf = Vec::new();
                    resp.into_reader().take(64 * 1024).read_to_end(&mut buf)?;
                    if buf.is_empty() {
                        let _ = std::fs::write(&failed_marker, b"");
                        return Err(AppError::Other("favicon 为空".into()));
                    }
                    buf
                }
                Err(e) => {
                    // 标记失败，后续不再重试
                    let _ = std::fs::write(&failed_marker, b"");
                    return Err(AppError::Other(format!("下载 favicon 失败: {e}")));
                }
            }
        }
    };

    // 写入缓存
    let _ = std::fs::write(&file_path, &data);

    Ok(format!(
        "data:image/png;base64,{}",
        base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &data)
    ))
}

// ========== 文件存在性 ==========

/// 探测路径是否存在（用于前端校验"创建新库"时不能覆盖已有文件）
#[tauri::command]
pub fn path_exists(path: String) -> bool {
    std::path::Path::new(&path).exists()
}

/// 检查 `app_data_dir/vault.zmb` 旧版固定路径文件是否存在。
///
/// 旧版（v1）使用 AES-GCM + 单口令派生，新版改为 SM4-GCM + 双因子（账号+口令），
/// 加密格式与 KDF 输入均不兼容，无法直接读取。FileListView 检测到该文件存在
/// 时会显示一条不可关闭的提示，引导用户用旧版导出 xlsx 后再在新版导入。
#[tauri::command]
pub fn legacy_vault_exists(app: tauri::AppHandle) -> bool {
    use tauri::Manager;
    match app.path().app_data_dir() {
        Ok(dir) => dir.join("vault.zmb").exists(),
        Err(_) => false,
    }
}
