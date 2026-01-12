//! 配置管理模块
//!
//! 负责应用配置的加载、保存和验证。

use std::time::Duration;

use objc2::ffi::NSInteger;
use objc2_foundation::{NSString, NSUserDefaults};

use crate::i18n::{Language, LanguagePreference};

/// 应用配置
#[derive(Clone, Debug)]
pub struct Config {
    pub interval_minutes: u64,
    pub break_seconds: u64,
    pub language: LanguagePreference,
}

impl Config {
    pub const DEFAULT_INTERVAL_MINUTES: u64 = 30;
    pub const DEFAULT_BREAK_SECONDS: u64 = 120;

    pub const MIN_INTERVAL_MINUTES: u64 = 1;
    pub const MAX_INTERVAL_MINUTES: u64 = 240;

    pub const MIN_BREAK_SECONDS: u64 = 5;
    pub const MAX_BREAK_SECONDS: u64 = 3600;

    const KEY_INTERVAL_MINUTES: &'static str = "restgap.interval_minutes";
    const KEY_BREAK_SECONDS: &'static str = "restgap.break_seconds";
    const KEY_LANGUAGE: &'static str = "restgap.language";

    const LEGACY_KEY_INTERVAL_MINUTES: &'static str = "restp.interval_minutes";
    const LEGACY_KEY_BREAK_SECONDS: &'static str = "restp.break_seconds";

    /// 从 `NSUserDefaults` 加载配置
    pub fn load() -> Self {
        let defaults = NSUserDefaults::standardUserDefaults();

        let interval_key = NSString::from_str(Self::KEY_INTERVAL_MINUTES);
        let break_key = NSString::from_str(Self::KEY_BREAK_SECONDS);
        let language_key = NSString::from_str(Self::KEY_LANGUAGE);

        let legacy_interval_key = NSString::from_str(Self::LEGACY_KEY_INTERVAL_MINUTES);
        let legacy_break_key = NSString::from_str(Self::LEGACY_KEY_BREAK_SECONDS);

        let interval_raw = defaults.integerForKey(&interval_key);
        let break_raw = defaults.integerForKey(&break_key);
        let language_raw = defaults.integerForKey(&language_key);

        let interval_raw = if interval_raw <= 0 {
            defaults.integerForKey(&legacy_interval_key)
        } else {
            interval_raw
        };
        let break_raw = if break_raw <= 0 {
            defaults.integerForKey(&legacy_break_key)
        } else {
            break_raw
        };

        let interval_minutes = if interval_raw <= 0 {
            Self::DEFAULT_INTERVAL_MINUTES
        } else {
            u64::try_from(interval_raw).unwrap_or(Self::DEFAULT_INTERVAL_MINUTES)
        };

        let break_seconds = if break_raw <= 0 {
            Self::DEFAULT_BREAK_SECONDS
        } else {
            u64::try_from(break_raw).unwrap_or(Self::DEFAULT_BREAK_SECONDS)
        };

        let language = match language_raw {
            1 => LanguagePreference::En,
            2 => LanguagePreference::Zh,
            _ => LanguagePreference::Auto,
        };

        Self {
            interval_minutes: clamp_u64(
                interval_minutes,
                Self::MIN_INTERVAL_MINUTES,
                Self::MAX_INTERVAL_MINUTES,
            ),
            break_seconds: clamp_u64(
                break_seconds,
                Self::MIN_BREAK_SECONDS,
                Self::MAX_BREAK_SECONDS,
            ),
            language,
        }
    }

    /// 保存配置到 `NSUserDefaults`
    pub fn save(&self) {
        let defaults = NSUserDefaults::standardUserDefaults();
        let interval_key = NSString::from_str(Self::KEY_INTERVAL_MINUTES);
        let break_key = NSString::from_str(Self::KEY_BREAK_SECONDS);
        let language_key = NSString::from_str(Self::KEY_LANGUAGE);

        let interval_minutes = NSInteger::try_from(self.interval_minutes).unwrap_or(NSInteger::MAX);
        let break_seconds = NSInteger::try_from(self.break_seconds).unwrap_or(NSInteger::MAX);

        defaults.setInteger_forKey(interval_minutes, &interval_key);
        defaults.setInteger_forKey(break_seconds, &break_key);

        let language_raw = match self.language {
            LanguagePreference::Auto => 0,
            LanguagePreference::En => 1,
            LanguagePreference::Zh => 2,
        };
        defaults.setInteger_forKey(language_raw, &language_key);
    }

    pub fn effective_language(&self) -> Language {
        self.language.resolve()
    }

    /// 获取工作间隔时长
    pub const fn work_interval(&self) -> Duration {
        Duration::from_secs(self.interval_minutes.saturating_mul(60))
    }

    /// 获取休息时长
    pub const fn break_duration(&self) -> Duration {
        Duration::from_secs(self.break_seconds)
    }

    /// 获取工作定时器容差（允许系统合并计时器唤醒）
    pub fn work_tolerance(&self) -> Duration {
        let secs = (self.work_interval().as_secs_f64() * 0.10).min(120.0);
        Duration::from_secs_f64(secs.max(1.0))
    }

    /// 获取休息定时器容差
    pub fn break_tolerance(&self) -> Duration {
        let secs = (self.break_duration().as_secs_f64() * 0.10).min(5.0);
        Duration::from_secs_f64(secs.max(0.5))
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            interval_minutes: Self::DEFAULT_INTERVAL_MINUTES,
            break_seconds: Self::DEFAULT_BREAK_SECONDS,
            language: LanguagePreference::Auto,
        }
    }
}

/// 将值限制在指定范围内
pub fn clamp_u64(v: u64, min: u64, max: u64) -> u64 {
    v.max(min).min(max)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clamp_u64() {
        assert_eq!(clamp_u64(5, 1, 10), 5);
        assert_eq!(clamp_u64(0, 1, 10), 1);
        assert_eq!(clamp_u64(15, 1, 10), 10);
    }

    #[test]
    fn test_config_default() {
        let config = Config::default();
        assert_eq!(config.interval_minutes, Config::DEFAULT_INTERVAL_MINUTES);
        assert_eq!(config.break_seconds, Config::DEFAULT_BREAK_SECONDS);
        assert_eq!(config.language, LanguagePreference::Auto);
    }

    #[test]
    fn test_work_interval() {
        let config = Config {
            interval_minutes: 30,
            break_seconds: 120,
            language: LanguagePreference::Auto,
        };
        assert_eq!(config.work_interval(), Duration::from_secs(1800));
    }

    #[test]
    fn test_break_duration() {
        let config = Config {
            interval_minutes: 30,
            break_seconds: 120,
            language: LanguagePreference::Auto,
        };
        assert_eq!(config.break_duration(), Duration::from_secs(120));
    }
}
