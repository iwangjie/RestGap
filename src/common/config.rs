//! Cross-platform configuration module
//!
//! Handles loading and saving configuration across all platforms.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

use crate::i18n::{Language, LanguagePreference};

/// Application configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
    pub interval_minutes: u64,
    pub break_seconds: u64,
    #[serde(default)]
    pub language: LanguagePreference,
}

impl Config {
    pub const DEFAULT_INTERVAL_MINUTES: u64 = 30;
    pub const DEFAULT_BREAK_SECONDS: u64 = 120;

    pub const MIN_INTERVAL_MINUTES: u64 = 1;
    pub const MAX_INTERVAL_MINUTES: u64 = 240;

    pub const MIN_BREAK_SECONDS: u64 = 5;
    pub const MAX_BREAK_SECONDS: u64 = 3600;

    /// Create a new config with default values
    pub const fn default() -> Self {
        Self {
            interval_minutes: Self::DEFAULT_INTERVAL_MINUTES,
            break_seconds: Self::DEFAULT_BREAK_SECONDS,
            language: LanguagePreference::Auto,
        }
    }

    /// Get the config file path
    fn config_path() -> Option<PathBuf> {
        dirs::config_dir().map(|mut path| {
            path.push("restgap");
            path.push("config.json");
            path
        })
    }

    /// Load configuration from disk (or use defaults)
    pub fn load() -> Self {
        if let Some(path) = Self::config_path() {
            if let Ok(contents) = fs::read_to_string(&path) {
                if let Ok(config) = serde_json::from_str::<Self>(&contents) {
                    return Self::validate(config);
                }
            }
        }
        Self::default()
    }

    /// Save configuration to disk
    #[allow(dead_code)]
    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(path) = Self::config_path() {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            let contents = serde_json::to_string_pretty(self)?;
            fs::write(&path, contents)?;
        }
        Ok(())
    }

    /// Validate and clamp config values to valid ranges
    fn validate(mut config: Self) -> Self {
        config.interval_minutes = config
            .interval_minutes
            .clamp(Self::MIN_INTERVAL_MINUTES, Self::MAX_INTERVAL_MINUTES);
        config.break_seconds = config
            .break_seconds
            .clamp(Self::MIN_BREAK_SECONDS, Self::MAX_BREAK_SECONDS);
        config
    }

    pub fn effective_language(&self) -> Language {
        self.language.resolve()
    }

    /// Get work interval as Duration
    #[allow(dead_code)]
    pub const fn work_interval(&self) -> std::time::Duration {
        std::time::Duration::from_secs(self.interval_minutes * 60)
    }

    /// Get break duration as Duration
    #[allow(dead_code)]
    pub const fn break_duration(&self) -> std::time::Duration {
        std::time::Duration::from_secs(self.break_seconds)
    }
}
