//! 错误处理模块
//!
//! 定义应用级别的错误类型。

use std::fmt;

/// 应用错误类型
#[derive(Debug)]
#[allow(dead_code)]
pub enum AppError {
    /// 配置错误
    Config(String),
    /// UI 错误
    Ui(String),
    /// 系统错误
    System(String),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Config(msg) => write!(f, "配置错误: {msg}"),
            Self::Ui(msg) => write!(f, "UI 错误: {msg}"),
            Self::System(msg) => write!(f, "系统错误: {msg}"),
        }
    }
}

impl std::error::Error for AppError {}

/// 应用结果类型
#[allow(dead_code)]
pub type AppResult<T> = Result<T, AppError>;
