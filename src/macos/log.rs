//! 日志模块
//!
//! 提供简单的日志功能，用于调试和问题排查。

#![allow(dead_code)]

use std::fmt::Arguments;

/// 日志级别
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

impl LogLevel {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Debug => "DEBUG",
            Self::Info => "INFO",
            Self::Warn => "WARN",
            Self::Error => "ERROR",
        }
    }
}

/// 当前日志级别（编译时配置）
#[cfg(debug_assertions)]
const CURRENT_LEVEL: LogLevel = LogLevel::Debug;

#[cfg(not(debug_assertions))]
const CURRENT_LEVEL: LogLevel = LogLevel::Info;

/// 记录日志
pub fn log(level: LogLevel, args: Arguments<'_>) {
    if level >= CURRENT_LEVEL {
        eprintln!("[RestGap] [{}] {}", level.as_str(), args);
    }
}

/// 调试日志宏
#[macro_export]
macro_rules! log_debug {
    ($($arg:tt)*) => {
        $crate::macos::log::log($crate::macos::log::LogLevel::Debug, format_args!($($arg)*))
    };
}

/// 信息日志宏
#[macro_export]
macro_rules! log_info {
    ($($arg:tt)*) => {
        $crate::macos::log::log($crate::macos::log::LogLevel::Info, format_args!($($arg)*))
    };
}

/// 警告日志宏
#[macro_export]
macro_rules! log_warn {
    ($($arg:tt)*) => {
        $crate::macos::log::log($crate::macos::log::LogLevel::Warn, format_args!($($arg)*))
    };
}

/// 错误日志宏
#[macro_export]
macro_rules! log_error {
    ($($arg:tt)*) => {
        $crate::macos::log::log($crate::macos::log::LogLevel::Error, format_args!($($arg)*))
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_level_ordering() {
        assert!(LogLevel::Debug < LogLevel::Info);
        assert!(LogLevel::Info < LogLevel::Warn);
        assert!(LogLevel::Warn < LogLevel::Error);
    }

    #[test]
    fn test_log_level_as_str() {
        assert_eq!(LogLevel::Debug.as_str(), "DEBUG");
        assert_eq!(LogLevel::Info.as_str(), "INFO");
        assert_eq!(LogLevel::Warn.as_str(), "WARN");
        assert_eq!(LogLevel::Error.as_str(), "ERROR");
    }
}
