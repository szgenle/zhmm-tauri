mod accounts;
mod anti_capture;
mod commands;
mod crypto;
mod errors;
mod io_json;
mod io_xlsx;
mod models;
mod settings;
mod site_catalog;
mod totp;
mod vault;

use accounts::RecentStore;
use settings::SettingsState;
use tauri::Manager;
use vault::VaultState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_window_state::Builder::new().build())
        .setup(|app| {
            let data_dir = app.path().app_data_dir().expect("无法获取应用数据目录");
            let settings_path = data_dir.join("settings.json");
            let recent_path = data_dir.join("recent_files.json");
            app.manage(VaultState::new());
            app.manage(SettingsState::new(settings_path));
            app.manage(RecentStore::new(recent_path));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::vault_status,
            commands::create_vault_at,
            commands::unlock_with_path,
            commands::lock_vault,
            commands::list_passwords,
            commands::get_password,
            commands::add_password,
            commands::delete_password,
            commands::update_password,
            commands::get_password_history,
            commands::generate_totp,
            commands::parse_otpauth,
            commands::export_xlsx,
            commands::import_xlsx,
            commands::backup_to_file,
            commands::restore_from_file,
            commands::get_settings,
            commands::update_settings,
            commands::list_roles,
            commands::create_local_backup,
            commands::list_local_backups,
            commands::delete_local_backup,
            commands::restore_local_backup,
            commands::cleanup_backups,
            commands::collect_tag_counts,
            commands::rename_tag,
            commands::delete_tag,
            commands::rollback_password,
            commands::list_site_catalog,
            commands::suggest_site,
            commands::verify_master_password,
            commands::rekey_vault,
            commands::apply_anti_capture,
            commands::export_xlsx_template,
            commands::list_recent,
            commands::upsert_recent,
            commands::remove_recent,
            commands::clear_recent,
            commands::bcrypt_hash,
            commands::bcrypt_verify,
            commands::path_exists,
            commands::legacy_vault_exists,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
