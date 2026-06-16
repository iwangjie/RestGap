//! Simple internationalization (i18n) helpers.
//!
//! Supports English and Simplified Chinese with an `Auto` mode that follows the OS language.

use serde::{Deserialize, Serialize};

/// Supported UI languages.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Language {
    /// English
    En,
    /// Chinese (Simplified)
    Zh,
}

/// User-configurable language preference.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LanguagePreference {
    /// Follow the OS language.
    #[default]
    Auto,
    /// Force English.
    En,
    /// Force Chinese.
    Zh,
}

impl LanguagePreference {
    pub fn resolve(self) -> Language {
        match self {
            Self::Auto => detect_system_language(),
            Self::En => Language::En,
            Self::Zh => Language::Zh,
        }
    }
}

#[cfg(target_os = "macos")]
pub struct Texts {
    lang: Language,
}

#[cfg(target_os = "macos")]
impl Texts {
    pub const fn new(lang: Language) -> Self {
        Self { lang }
    }

    pub const fn language(&self) -> Language {
        self.lang
    }

    pub const fn app_name_short(&self) -> &'static str {
        match self.lang {
            Language::En => "RestGap",
            Language::Zh => "息间",
        }
    }

    pub const fn menu_next_break_placeholder(&self) -> &'static str {
        match self.lang {
            Language::En => "Next break: --:--",
            Language::Zh => "下次休息：--:--",
        }
    }

    pub const fn menu_remaining_placeholder(&self) -> &'static str {
        match self.lang {
            Language::En => "Break remaining: —",
            Language::Zh => "休息剩余：—",
        }
    }

    pub const fn menu_rest_now(&self) -> &'static str {
        match self.lang {
            Language::En => "Rest now",
            Language::Zh => "现在休息",
        }
    }

    pub const fn menu_settings(&self) -> &'static str {
        match self.lang {
            Language::En => "Settings",
            Language::Zh => "配置",
        }
    }

    pub fn menu_about(&self) -> String {
        match self.lang {
            Language::En => format!("About {}", self.app_name_short()),
            Language::Zh => format!("关于 {}", self.app_name_short()),
        }
    }

    pub const fn menu_quit(&self) -> &'static str {
        match self.lang {
            Language::En => "Quit",
            Language::Zh => "退出",
        }
    }

    pub const fn menu_language_header(&self) -> &'static str {
        match self.lang {
            Language::En => "Language",
            Language::Zh => "语言",
        }
    }

    pub const fn language_auto(&self) -> &'static str {
        match self.lang {
            Language::En => "Auto",
            Language::Zh => "自动",
        }
    }

    pub const fn language_en(&self) -> &'static str {
        match self.lang {
            Language::En | Language::Zh => "English",
        }
    }

    pub const fn language_zh(&self) -> &'static str {
        match self.lang {
            Language::En | Language::Zh => "中文",
        }
    }

    pub fn header_title(&self, interval_minutes: u64, break_seconds: u64) -> String {
        match self.lang {
            Language::En => format!(
                "{} · Break every {} min for {} sec",
                self.app_name_short(),
                interval_minutes,
                break_seconds
            ),
            Language::Zh => format!(
                "{} · 每 {} 分钟休息 {} 秒",
                self.app_name_short(),
                interval_minutes,
                break_seconds
            ),
        }
    }

    pub fn next_break_title(&self, hm: &str, approx: &str) -> String {
        match self.lang {
            Language::En => format!("Next break: {hm} ({approx})"),
            Language::Zh => format!("下次休息：{hm}（{approx}）"),
        }
    }

    pub fn remaining_title_breaking(&self, approx: &str, end_hm: &str) -> String {
        match self.lang {
            Language::En => format!("Break remaining: {approx} (until {end_hm})"),
            Language::Zh => format!("休息剩余：{approx}（至 {end_hm}）"),
        }
    }

    pub const fn settings_title(&self) -> &'static str {
        match self.lang {
            Language::En => "Settings",
            Language::Zh => "配置",
        }
    }

    #[cfg(target_os = "macos")]
    pub const fn settings_save_button(&self) -> &'static str {
        match self.lang {
            Language::En => "Save",
            Language::Zh => "保存",
        }
    }

    #[cfg(target_os = "macos")]
    pub const fn settings_cancel_button(&self) -> &'static str {
        match self.lang {
            Language::En => "Cancel",
            Language::Zh => "取消",
        }
    }

    #[cfg(target_os = "macos")]
    pub const fn settings_interval_label(&self) -> &'static str {
        match self.lang {
            Language::En => "Break every N minutes:",
            Language::Zh => "每 N 分钟休息：",
        }
    }

    #[cfg(target_os = "macos")]
    pub const fn settings_break_label(&self) -> &'static str {
        match self.lang {
            Language::En => "Rest for N seconds:",
            Language::Zh => "休息 N 秒：",
        }
    }

    #[cfg(target_os = "macos")]
    pub const fn settings_skip_break_label(&self) -> &'static str {
        match self.lang {
            Language::En => "Allow skipping a break:",
            Language::Zh => "允许跳过休息：",
        }
    }

    #[cfg(target_os = "macos")]
    pub const fn settings_skip_break_hint(&self) -> &'static str {
        match self.lang {
            Language::En => "Show a discreet skip button on the break screen",
            Language::Zh => "在休息界面右上角显示隐蔽的跳过按钮",
        }
    }

    #[cfg(target_os = "macos")]
    pub const fn ok_button(&self) -> &'static str {
        match self.lang {
            Language::En => "OK",
            Language::Zh => "好",
        }
    }

    #[cfg(target_os = "macos")]
    pub const fn visit_homepage_button(&self) -> &'static str {
        match self.lang {
            Language::En => "Visit homepage",
            Language::Zh => "访问主页",
        }
    }

    #[cfg(target_os = "macos")]
    pub fn about_message_macos(&self) -> String {
        match self.lang {
            Language::En => format!(
                "Version: {}\nmacOS menu bar break reminder (event-driven / no polling).",
                env!("CARGO_PKG_VERSION")
            ),
            Language::Zh => format!(
                "版本：{}\nmacOS 菜单栏休息提醒（事件驱动 / 非轮询）。",
                env!("CARGO_PKG_VERSION")
            ),
        }
    }

    pub fn countdown_title(&self) -> String {
        match self.lang {
            Language::En => "Rest".to_string(),
            Language::Zh => "休息".to_string(),
        }
    }

    pub const fn countdown_hint(&self) -> &'static str {
        match self.lang {
            Language::En | Language::Zh => "",
        }
    }
}

pub fn detect_system_language() -> Language {
    #[cfg(target_os = "windows")]
    {
        detect_system_language_windows()
    }
    #[cfg(target_os = "macos")]
    {
        detect_system_language_macos()
    }
    #[cfg(not(target_os = "macos"))]
    {
        detect_system_language_env()
    }
}

#[cfg(not(target_os = "windows"))]
pub fn detect_system_language_env() -> Language {
    for key in ["LC_ALL", "LC_MESSAGES", "LANG"] {
        if let Some(v) = std::env::var_os(key) {
            if let Some(lang) = language_from_tag(&v.to_string_lossy()) {
                return lang;
            }
        }
    }
    Language::En
}

#[cfg(not(target_os = "windows"))]
pub fn language_from_tag(tag: &str) -> Option<Language> {
    let s = tag.trim();
    if s.is_empty() {
        return None;
    }
    let s = s.to_ascii_lowercase();
    if s.contains("zh") {
        return Some(Language::Zh);
    }
    if s.contains("en") {
        return Some(Language::En);
    }
    None
}

#[cfg(target_os = "macos")]
fn detect_system_language_macos() -> Language {
    crate::macos::locale::detect_system_language()
}

#[cfg(target_os = "macos")]
pub fn first_quoted(s: &str) -> Option<&str> {
    let mut in_quote = false;
    let mut start = 0usize;
    for (idx, ch) in s.char_indices() {
        if ch != '"' {
            continue;
        }
        if in_quote {
            return s.get(start..idx);
        }
        in_quote = true;
        start = idx + 1;
    }
    None
}
