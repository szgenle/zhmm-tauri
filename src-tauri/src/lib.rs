mod commands;
mod crypto;
mod errors;
mod io_json;
mod io_xlsx;
mod models;
mod settings;
mod totp;
mod vault;

use tauri::Manager;
use settings::SettingsState;
use vault::VaultState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .setup(|app| {
            let data_dir = app
                .path()
                .app_data_dir()
                .expect("无法获取应用数据目录");
            let vault_path = data_dir.join("vault.zhmm");
            let settings_path = data_dir.join("settings.json");
            app.manage(VaultState::new(vault_path));
            app.manage(SettingsState::new(settings_path));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::vault_status,
            commands::create_vault,
            commands::unlock_vault,
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
