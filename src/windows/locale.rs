//! Locale / language detection for Windows.

use windows::Win32::Globalization::GetUserDefaultUILanguage;

use crate::i18n::Language;

pub fn detect_system_language() -> Language {
    let langid = unsafe { GetUserDefaultUILanguage() };
    let primary = langid & 0x03ff;
    if primary == 0x04 {
        Language::Zh
    } else {
        Language::En
    }
}
