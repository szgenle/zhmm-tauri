//! 应用设置与角色列表

use tauri::State;

use crate::errors::AppResult;
use crate::settings::{AppSettings, SettingsState};
use crate::vault::VaultState;

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
