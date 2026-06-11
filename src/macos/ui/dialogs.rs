//! 对话框模块

use std::process::Command;

use objc2::rc::Retained;
use objc2::{MainThreadOnly, msg_send};
use objc2_app_kit::{NSAlert, NSAlertSecondButtonReturn};
use objc2_foundation::NSString;

use super::super::constants::APP_NAME_DISPLAY;
use super::super::delegate::RestGapDelegate;
use super::super::state::with_state_ref;
use crate::i18n::Texts;

/// 显示关于对话框
pub fn show_about_dialog(delegate: &RestGapDelegate) {
    let mtm = delegate.mtm();
    let texts = Texts::new(with_state_ref(|s| s.config.effective_language()));
    let alert: Retained<NSAlert> = unsafe { msg_send![NSAlert::alloc(mtm), init] };
    alert.setMessageText(&NSString::from_str(APP_NAME_DISPLAY));
    alert.setInformativeText(&NSString::from_str(&texts.about_message_macos()));

    let _ = alert.addButtonWithTitle(&NSString::from_str(texts.ok_button()));
    let _ = alert.addButtonWithTitle(&NSString::from_str(texts.visit_homepage_button()));

    let resp = alert.runModal();
    if resp == NSAlertSecondButtonReturn {
        let _ = Command::new("open")
            .arg("https://github.com/iwangjie/RestGap")
            .spawn();
    }
}
