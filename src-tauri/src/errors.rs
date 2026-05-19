//! 错误类型定义

use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("密码库未解锁")]
    Locked,

    #[error("主密码错误")]
    InvalidPassword,

    #[error("加密错误: {0}")]
    Crypto(String),

    #[error("完整性校验失败")]
    IntegrityCheck,

    #[error("序列化错误: {0}")]
    Serde(#[from] serde_json::Error),

    #[error("IO 错误: {0}")]
    Io(#[from] std::io::Error),

    #[error("记录不存在")]
    NotFound,

    #[error("参数无效: {0}")]
    Invalid(String),

    #[error("其他错误: {0}")]
    Other(String),
}

pub type AppResult<T> = Result<T, AppError>;

// 序列化为字符串方便 Tauri 返回给前端
impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}
