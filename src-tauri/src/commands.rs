//! Tauri 命令：前端调用入口
//!
//! 多账号库改造后：路径不再固定，前端通过 create_vault_at / unlock_with_path 指定。
//! 最近访问列表 (RecentStore) 与 bcrypt 命令支撑前端的"账号文件列表"页面。

use std::path::PathBuf;

use tauri::State;

use crate::accounts::{RecentEntry, RecentStore};
use crate::errors::{AppError, AppResult};
use crate::io_json;
use crate::io_xlsx;
use crate::models::{PasswordEntry, PasswordHistoryItem, PasswordInput, PasswordSummary};
use crate::settings::{AppSettings, SettingsState};
use crate::site_catalog;
use crate::totp::{self, OtpAuthParams};
use crate::vault::VaultState;

#[derive(serde::Serialize)]
pub struct VaultStatus {
    pub unlocked: bool,
    pub current_path: Option<String>,
    pub current_account: Option<String>,
}

#[tauri::command]
pub fn vault_status(state: State<'_, VaultState>) -> VaultStatus {
    VaultStatus {
        unlocked: state.is_unlocked(),
        current_path: state.current_path().map(|p| p.to_string_lossy().to_string()),
        current_account: state.current_account(),
    }
}

#[tauri::command]
pub fn create_vault_at(
    path: String,
    account: String,
    master_password: String,
    state: State<'_, VaultState>,
) -> AppResult<()> {
    state.create(&PathBuf::from(path), &account, &master_password)
}

#[tauri::command]
pub fn unlock_with_path(
    path: String,
    account: String,
    master_password: String,
    state: State<'_, VaultState>,
) -> AppResult<()> {
    state.unlock_with_path(&PathBuf::from(path), &account, &master_password)
}

#[tauri::command]
pub fn lock_vault(state: State<'_, VaultState>) {
    state.lock();
}

#[tauri::command]
pub fn list_passwords(state: State<'_, VaultState>) -> AppResult<Vec<PasswordSummary>> {
    state.list()
}

#[tauri::command]
pub fn get_password(id: i64, state: State<'_, VaultState>) -> AppResult<PasswordEntry> {
    state.get(id)
}

#[tauri::command]
pub fn add_password(
    input: PasswordInput,
    state: State<'_, VaultState>,
) -> AppResult<PasswordEntry> {
    state.add(input)
}

#[tauri::command]
pub fn delete_password(id: i64, state: State<'_, VaultState>) -> AppResult<()> {
    state.remove(id)
}

#[tauri::command]
pub fn update_password(
    id: i64,
    input: PasswordInput,
    state: State<'_, VaultState>,
) -> AppResult<PasswordEntry> {
    state.update(id, input)
}

#[tauri::command]
pub fn get_password_history(
    id: i64,
    state: State<'_, VaultState>,
) -> AppResult<Vec<PasswordHistoryItem>> {
    state.history(id)
}

#[derive(serde::Serialize)]
pub struct TotpCode {
    pub code: String,
    pub remaining_seconds: u32,
}

#[tauri::command]
pub fn generate_totp(id: i64, state: State<'_, VaultState>) -> AppResult<TotpCode> {
    let entry = state.get(id)?;
    if entry.totp_secret.is_empty() {
        return Err(AppError::Invalid("未启用 TOTP".into()));
    }
    let algo = if entry.totp_algo.is_empty() {
        "SHA1"
    } else {
        entry.totp_algo.as_str()
    };
    let digits = if entry.totp_digits == 0 { 6 } else { entry.totp_digits };
    let period = if entry.totp_period == 0 { 30 } else { entry.totp_period };
    let code = totp::generate(&entry.totp_secret, algo, digits, period, None)?;
    let remaining = totp::remaining_seconds(period, None)?;
    Ok(TotpCode {
        code,
        remaining_seconds: remaining,
    })
}

#[tauri::command]
pub fn parse_otpauth(uri: String) -> AppResult<OtpAuthParams> {
    totp::parse_otpauth_uri(&uri)
}

/// 导出当前密码库为 xlsx（明文落盘）
#[tauri::command]
pub fn export_xlsx(path: String, state: State<'_, VaultState>) -> AppResult<()> {
    let snapshot = state.snapshot()?;
    io_xlsx::export_xlsx(&PathBuf::from(path), &snapshot.entries)
}

/// 从 xlsx 追加条目；返回导入条目数
#[tauri::command]
pub fn import_xlsx(path: String, state: State<'_, VaultState>) -> AppResult<usize> {
    let entries = io_xlsx::import_xlsx(&PathBuf::from(path))?;
    state.extend_entries(entries)
}

/// 加密 JSON 备份到指定文件；可使用与主密码不同的备份密码
#[tauri::command]
pub fn backup_to_file(
    path: String,
    backup_password: String,
    state: State<'_, VaultState>,
) -> AppResult<()> {
    if backup_password.is_empty() {
        return Err(AppError::Invalid("备份密码不能为空".into()));
    }
    let snapshot = state.snapshot()?;
    io_json::backup_to_file(&PathBuf::from(path), &snapshot, &backup_password)
}

/// 从加密 JSON 文件恢复（完全覆盖当前数据）
#[tauri::command]
pub fn restore_from_file(
    path: String,
    backup_password: String,
    state: State<'_, VaultState>,
) -> AppResult<()> {
    let data = io_json::restore_from_file(&PathBuf::from(path), &backup_password)?;
    state.replace(data)
}

#[tauri::command]
pub fn get_settings(state: State<'_, SettingsState>) -> AppSettings {
    state.get()
}

#[tauri::command]
pub fn update_settings(
    new_settings: AppSettings,
    state: State<'_, SettingsState>,
) -> AppResult<AppSettings> {
    state.update(new_settings)
}

#[tauri::command]
pub fn list_roles(state: State<'_, VaultState>) -> AppResult<Vec<String>> {
    state.roles()
}

// ========== 本地备份管理 ==========

#[tauri::command]
pub fn create_local_backup(state: State<'_, VaultState>) -> AppResult<String> {
    state.create_local_backup()
}

#[tauri::command]
pub fn list_local_backups(state: State<'_, VaultState>) -> AppResult<Vec<crate::vault::BackupInfo>> {
    state.list_local_backups()
}

#[tauri::command]
pub fn delete_local_backup(name: String, state: State<'_, VaultState>) -> AppResult<()> {
    state.delete_local_backup(&name)
}

#[tauri::command]
pub fn restore_local_backup(name: String, state: State<'_, VaultState>) -> AppResult<()> {
    state.restore_local_backup(&name)
}

#[tauri::command]
pub fn cleanup_backups(keep: usize, state: State<'_, VaultState>) -> AppResult<u32> {
    state.cleanup_backups(keep)
}

// ========== 标签管理 ==========

#[derive(serde::Serialize)]
pub struct TagCount {
    pub tag: String,
    pub count: usize,
}

#[tauri::command]
pub fn collect_tag_counts(state: State<'_, VaultState>) -> AppResult<Vec<TagCount>> {
    let counts = state.collect_tag_counts()?;
    Ok(counts.into_iter().map(|(tag, count)| TagCount { tag, count }).collect())
}

#[tauri::command]
pub fn rename_tag(old: String, new: String, state: State<'_, VaultState>) -> AppResult<usize> {
    state.rename_tag(&old, &new)
}

#[tauri::command]
pub fn delete_tag(tag: String, state: State<'_, VaultState>) -> AppResult<usize> {
    state.delete_tag(&tag)
}

// ========== 密码历史回滚 ==========

#[tauri::command]
pub fn rollback_password(
    id: i64,
    history_index: usize,
    state: State<'_, VaultState>,
) -> AppResult<PasswordEntry> {
    state.rollback_password(id, history_index)
}

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
pub fn verify_master_password(
    password: String,
    state: State<'_, VaultState>,
) -> AppResult<bool> {
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

// ========== xlsx 模板 ==========

/// 导出空模板供用户填写后导入
#[tauri::command]
pub fn export_xlsx_template(path: String) -> AppResult<()> {
    io_xlsx::export_template(&PathBuf::from(path))
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

/// 检查 `app_data_dir/vault.zhmm` 旧版固定路径文件是否存在。
///
/// 旧版（v1）使用 AES-GCM + 单口令派生，新版改为 SM4-GCM + 双因子（账号+口令），
/// 加密格式与 KDF 输入均不兼容，无法直接读取。FileListView 检测到该文件存在
/// 时会显示一条不可关闭的提示，引导用户用旧版导出 xlsx 后再在新版导入。
#[tauri::command]
pub fn legacy_vault_exists(app: tauri::AppHandle) -> bool {
    use tauri::Manager;
    match app.path().app_data_dir() {
        Ok(dir) => dir.join("vault.zhmm").exists(),
        Err(_) => false,
    }
}
