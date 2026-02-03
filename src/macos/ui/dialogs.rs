//! 对话框模块

use std::process::Command;

use objc2::rc::Retained;
use objc2::{MainThreadOnly, msg_send};
use objc2_app_kit::{
    NSAlert, NSAlertFirstButtonReturn, NSAlertSecondButtonReturn, NSAlertThirdButtonReturn,
    NSTextField, NSView,
};
use objc2_foundation::{NSPoint, NSRect, NSSize, NSString};

use super::super::config::{Config, clamp_u64};
use super::super::constants::APP_NAME_DISPLAY;
use super::super::delegate::RestGapDelegate;
use super::super::state::{Phase, with_state, with_state_ref};
use super::super::timer::schedule_phase;
use super::countdown::show_countdown_window;
use super::status_bar::{refresh_header_title, refresh_menu_info, refresh_static_menu_titles};
use crate::i18n::{LanguagePreference, Texts};

/// 显示无效配置警告
pub fn show_invalid_settings_alert(delegate: &RestGapDelegate) {
    let mtm = delegate.mtm();
    let texts = Texts::new(with_state_ref(|s| s.config.effective_language()));
    let alert: Retained<NSAlert> = unsafe { msg_send![NSAlert::alloc(mtm), init] };
    alert.setMessageText(&NSString::from_str(texts.invalid_settings_title()));
    alert.setInformativeText(&NSString::from_str(texts.invalid_settings_message()));
    let _ = alert.addButtonWithTitle(&NSString::from_str(texts.ok_button()));
    let _ = alert.runModal();
}

fn open_language_dialog(delegate: &RestGapDelegate) {
    let mtm = delegate.mtm();
    let texts = Texts::new(with_state_ref(|s| s.config.effective_language()));

    let alert: Retained<NSAlert> = unsafe { msg_send![NSAlert::alloc(mtm), init] };
    alert.setMessageText(&NSString::from_str(texts.menu_language_header()));
    alert.setInformativeText(&NSString::from_str(&format!(
        "{}\n{}",
        texts.choose_language_message(),
        texts.choose_language_note()
    )));

    let _ = alert.addButtonWithTitle(&NSString::from_str(texts.language_auto()));
    let _ = alert.addButtonWithTitle(&NSString::from_str(texts.language_en()));
    let _ = alert.addButtonWithTitle(&NSString::from_str(texts.language_zh()));
    let _ = alert.addButtonWithTitle(&NSString::from_str(texts.settings_cancel_button()));

    let resp = alert.runModal();
    let pref = if resp == NSAlertFirstButtonReturn {
        LanguagePreference::Auto
    } else if resp == NSAlertSecondButtonReturn {
        LanguagePreference::En
    } else if resp == NSAlertThirdButtonReturn {
        LanguagePreference::Zh
    } else {
        return;
    };

    with_state(|state| {
        state.config.language = pref;
        state.config.save();
    });

    refresh_header_title();
    refresh_static_menu_titles();
    refresh_menu_info();
}

/// 打开配置对话框
pub fn open_settings_dialog(delegate: &RestGapDelegate) {
    let mtm = delegate.mtm();

    let current = with_state_ref(|s| s.config.clone());
    let texts = Texts::new(current.effective_language());

    let alert: Retained<NSAlert> = unsafe { msg_send![NSAlert::alloc(mtm), init] };
    alert.setMessageText(&NSString::from_str(texts.settings_title()));
    alert.setInformativeText(&NSString::from_str(texts.settings_informative_text()));
    let _ = alert.addButtonWithTitle(&NSString::from_str(texts.settings_save_button()));
    let _ = alert.addButtonWithTitle(&NSString::from_str(texts.settings_language_button()));
    let _ = alert.addButtonWithTitle(&NSString::from_str(texts.settings_cancel_button()));

    let accessory_frame = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(320.0, 78.0));
    let accessory = NSView::initWithFrame(NSView::alloc(mtm), accessory_frame);

    let label_style = |label: &NSTextField| {
        label.setBezeled(false);
        label.setDrawsBackground(false);
        label.setEditable(false);
        label.setSelectable(false);
    };

    let minutes_label_frame = NSRect::new(NSPoint::new(0.0, 44.0), NSSize::new(160.0, 20.0));
    let minutes_label = NSTextField::initWithFrame(NSTextField::alloc(mtm), minutes_label_frame);
    minutes_label.setStringValue(&NSString::from_str(texts.settings_interval_label()));
    label_style(&minutes_label);

    let minutes_input_frame = NSRect::new(NSPoint::new(170.0, 40.0), NSSize::new(130.0, 24.0));
    let minutes_input = NSTextField::initWithFrame(NSTextField::alloc(mtm), minutes_input_frame);
    minutes_input.setStringValue(&NSString::from_str(&current.interval_minutes.to_string()));

    let seconds_label_frame = NSRect::new(NSPoint::new(0.0, 10.0), NSSize::new(160.0, 20.0));
    let seconds_label = NSTextField::initWithFrame(NSTextField::alloc(mtm), seconds_label_frame);
    seconds_label.setStringValue(&NSString::from_str(texts.settings_break_label()));
    label_style(&seconds_label);

    let seconds_input_frame = NSRect::new(NSPoint::new(170.0, 6.0), NSSize::new(130.0, 24.0));
    let seconds_input = NSTextField::initWithFrame(NSTextField::alloc(mtm), seconds_input_frame);
    seconds_input.setStringValue(&NSString::from_str(&current.break_seconds.to_string()));

    accessory.addSubview(&minutes_label);
    accessory.addSubview(&minutes_input);
    accessory.addSubview(&seconds_label);
    accessory.addSubview(&seconds_input);

    alert.setAccessoryView(Some(&accessory));

    let resp = alert.runModal();
    if resp == NSAlertSecondButtonReturn {
        open_language_dialog(delegate);
        return;
    }
    if resp != NSAlertFirstButtonReturn {
        return;
    }

    let interval_minutes = minutes_input
        .stringValue()
        .to_string()
        .trim()
        .parse::<u64>()
        .ok();
    let break_seconds = seconds_input
        .stringValue()
        .to_string()
        .trim()
        .parse::<u64>()
        .ok();

    let (Some(interval_minutes), Some(break_seconds)) = (interval_minutes, break_seconds) else {
        show_invalid_settings_alert(delegate);
        return;
    };

    if interval_minutes == 0 || break_seconds == 0 {
        show_invalid_settings_alert(delegate);
        return;
    }

    let new_config = Config {
        interval_minutes: clamp_u64(
            interval_minutes,
            Config::MIN_INTERVAL_MINUTES,
            Config::MAX_INTERVAL_MINUTES,
        ),
        break_seconds: clamp_u64(
            break_seconds,
            Config::MIN_BREAK_SECONDS,
            Config::MAX_BREAK_SECONDS,
        ),
        language: current.language,
    };
    new_config.save();

    let phase = with_state(|state| {
        state.config = new_config.clone();
        state.phase
    });

    // 从现在开始重新计时（避免显示与实际触发不一致）。
    schedule_phase(delegate, phase);

    // 若当前正在休息，则同步更新倒计时窗口（不播放开始声音）。
    if phase == Phase::Breaking {
        show_countdown_window(delegate, new_config.break_seconds, false);
    }
}

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
