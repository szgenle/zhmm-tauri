//! Tauri 命令：前端调用入口
//!
//! 多账号库改造后：路径不再固定，前端通过 create_vault_at / unlock_with_path 指定。
//! 最近访问列表 (RecentStore) 与 bcrypt 命令支撑前端的"账号文件列表"页面。
//!
//! 本模块按业务域拆为多个子文件，对外仍以 `commands::xxx` 路径暴露所有命令，
//! 因此 [crate::lib] 的 `invoke_handler!` 注册无需调整。

pub mod backups;
pub mod lifecycle;
pub mod misc;
pub mod passwords;
pub mod settings;
pub mod tags;
pub mod templates;
pub mod totp;
pub mod transfer;

pub use backups::*;
pub use lifecycle::*;
pub use misc::*;
pub use passwords::*;
pub use settings::*;
pub use tags::*;
pub use templates::*;
pub use totp::*;
pub use transfer::*;
