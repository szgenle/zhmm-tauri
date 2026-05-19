//! 应用设置：明文 JSON 落 app_data_dir/settings.json
//!
//! 不含敏感数据，故意不加密。

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

use crate::errors::AppResult;

/// 应用设置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    /// "auto" | "light" | "dark"
    #[serde(default = "default_theme")]
    pub theme: String,
    /// 0 = 不自动锁定，单位分钟
    #[serde(default = "default_auto_lock")]
    pub auto_lock_minutes: u32,
    /// 复制密码后剪贴板清空秒数；0 = 不清空
    #[serde(default = "default_clipboard_clear")]
    pub clipboard_clear_seconds: u32,
}

fn default_theme() -> String {
    "auto".into()
}
fn default_auto_lock() -> u32 {
    5
}
fn default_clipboard_clear() -> u32 {
    30
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            theme: default_theme(),
            auto_lock_minutes: default_auto_lock(),
            clipboard_clear_seconds: default_clipboard_clear(),
        }
    }
}

pub struct SettingsState {
    path: RwLock<PathBuf>,
    cached: RwLock<AppSettings>,
}

impl SettingsState {
    pub fn new(path: PathBuf) -> Self {
        let cached = Self::load_or_default(&path);
        Self {
            path: RwLock::new(path),
            cached: RwLock::new(cached),
        }
    }

    fn load_or_default(path: &PathBuf) -> AppSettings {
        if let Ok(bytes) = fs::read(path) {
            if let Ok(s) = serde_json::from_slice::<AppSettings>(&bytes) {
                return s;
            }
        }
        AppSettings::default()
    }

    pub fn get(&self) -> AppSettings {
        self.cached.read().clone()
    }

    pub fn update(&self, new_settings: AppSettings) -> AppResult<AppSettings> {
        // 写文件，再更新缓存
        let bytes = serde_json::to_vec_pretty(&new_settings)?;
        let path = self.path.read().clone();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&path, bytes)?;
        *self.cached.write() = new_settings.clone();
        Ok(new_settings)
    }
}
