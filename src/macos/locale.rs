//! Locale / language detection for macOS.

use objc2::msg_send;
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2_foundation::{NSString, NSUserDefaults};

use crate::i18n::{Language, detect_system_language_env, first_quoted, language_from_tag};

pub fn detect_system_language() -> Language {
    let defaults = NSUserDefaults::standardUserDefaults();
    let key = NSString::from_str("AppleLanguages");

    let obj: Option<Retained<AnyObject>> = unsafe { msg_send![&*defaults, objectForKey: &*key] };
    if let Some(obj) = obj {
        // The description string is stable enough for extracting language tags such as "en-US",
        // "zh-Hans", etc.
        let desc: Retained<NSString> = unsafe { msg_send![&*obj, description] };
        if let Some(first) = first_quoted(&desc.to_string()) {
            if let Some(lang) = language_from_tag(first) {
                return lang;
            }
        }
    }

    detect_system_language_env()
}
