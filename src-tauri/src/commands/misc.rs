//! 杂项命令：站点词典、主密码、防截屏、最近访问、bcrypt、文件存在性

use tauri::State;

use crate::accounts::{RecentEntry, RecentStore};
use crate::errors::{AppError, AppResult};
use crate::site_catalog;
use crate::vault::VaultState;

// ========== 站点词典 ==========

#[tauri::command]
pub fn list_site_catalog() -> Vec<site_catalog::SiteCatalogEntry> {
    site_catalog::all_entries()
}

#[tauri::command]
pub fn suggest_site(url_or_host: String) -> site_catalog::SiteSuggestion {
    site_catalog::suggest(&url_or_host)
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
