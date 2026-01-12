//! 状态栏 UI 模块

use std::time::{Duration, Instant};

use objc2::runtime::{AnyObject, ProtocolObject};
use objc2::{ClassType, MainThreadOnly, sel};
use objc2_app_kit::{NSMenu, NSMenuDelegate, NSMenuItem, NSStatusBar, NSVariableStatusItemLength};
use objc2_foundation::NSString;

use super::super::delegate::RestGapDelegate;
use super::super::state::{Phase, with_state, with_state_ref};
use super::super::utils::{approx_duration, format_hhmm};
use crate::i18n::Texts;

/// 获取 delegate 的 `AnyObject` 引用
pub fn target_anyobject(delegate: &RestGapDelegate) -> &AnyObject {
    delegate.as_super().as_super()
}

/// 刷新状态栏标题
pub fn refresh_status_title() {
    with_state(|state| {
        let Some(status_item) = state.status_item.as_ref() else {
            return;
        };

        let title = match state.phase {
            Phase::Working => {
                let hm = state
                    .phase_deadline_wall
                    .map_or_else(|| "--:--".to_string(), format_hhmm);
                format!("⏰ {hm}")
            }
            Phase::Breaking => {
                let remaining = state
                    .phase_deadline_mono
                    .and_then(|t| t.checked_duration_since(Instant::now()))
                    .unwrap_or(Duration::from_secs(0));
                format!("☕ {}", approx_duration(remaining))
            }
        };
        let ns_title = NSString::from_str(&title);
        status_item.setTitle(Some(&ns_title));
    });
}

/// 设置"现在休息"菜单项的启用状态
pub fn set_rest_now_enabled(enabled: bool) {
    with_state(|state| {
        let Some(item) = state.rest_now_item.as_ref() else {
            return;
        };
        item.setEnabled(enabled);
    });
}

/// 刷新菜单信息
pub fn refresh_menu_info() {
    let now = Instant::now();
    with_state(|state| {
        let texts = Texts::new(state.config.effective_language());
        let Some(next_item) = state.next_break_item.as_ref() else {
            return;
        };
        let Some(remaining_item) = state.remaining_break_item.as_ref() else {
            return;
        };

        let phase_deadline_mono = state.phase_deadline_mono;
        let phase_deadline_wall = state.phase_deadline_wall;

        let (next_break_in, next_break_wall) = match state.phase {
            Phase::Working => {
                let in_dur = phase_deadline_mono
                    .and_then(|t| t.checked_duration_since(now))
                    .unwrap_or(Duration::from_secs(0));
                (in_dur, phase_deadline_wall)
            }
            Phase::Breaking => {
                let remaining_break = phase_deadline_mono
                    .and_then(|t| t.checked_duration_since(now))
                    .unwrap_or(Duration::from_secs(0));
                let in_dur = remaining_break + state.config.work_interval();
                let wall = phase_deadline_wall
                    .and_then(|t| t.checked_add(state.config.work_interval()))
                    .or(phase_deadline_wall);
                (in_dur, wall)
            }
        };

        let next_hm = next_break_wall.map_or_else(|| "--:--".to_string(), format_hhmm);
        let next_title = texts.next_break_title(&next_hm, &approx_duration(next_break_in));
        next_item.setTitle(&NSString::from_str(&next_title));

        match state.phase {
            Phase::Working => {
                remaining_item.setTitle(&NSString::from_str(texts.remaining_title_working()));
            }
            Phase::Breaking => {
                let remaining = phase_deadline_mono
                    .and_then(|t| t.checked_duration_since(now))
                    .unwrap_or(Duration::from_secs(0));
                let end_hm = phase_deadline_wall.map_or_else(|| "--:--".to_string(), format_hhmm);
                let title = texts.remaining_title_breaking(&approx_duration(remaining), &end_hm);
                remaining_item.setTitle(&NSString::from_str(&title));
            }
        }
    });
}

/// 刷新头部标题
pub fn refresh_header_title() {
    with_state(|state| {
        let texts = Texts::new(state.config.effective_language());
        let Some(item) = state.header_item.as_ref() else {
            return;
        };
        let title = texts.header_title(state.config.interval_minutes, state.config.break_seconds);
        item.setTitle(&NSString::from_str(&title));
    });
}

pub fn refresh_static_menu_titles() {
    with_state(|state| {
        let texts = Texts::new(state.config.effective_language());

        if let Some(item) = state.rest_now_item.as_ref() {
            item.setTitle(&NSString::from_str(texts.menu_rest_now()));
        }
        if let Some(item) = state.settings_item.as_ref() {
            item.setTitle(&NSString::from_str(texts.menu_settings()));
        }
        if let Some(item) = state.about_item.as_ref() {
            item.setTitle(&NSString::from_str(&texts.menu_about()));
        }
        if let Some(item) = state.quit_item.as_ref() {
            item.setTitle(&NSString::from_str(texts.menu_quit()));
        }

        if let Some(item) = state.language_auto_item.as_ref() {
            item.setTitle(&NSString::from_str(texts.language_auto()));
        }
        if let Some(item) = state.language_en_item.as_ref() {
            item.setTitle(&NSString::from_str(texts.language_en()));
        }
        if let Some(item) = state.language_zh_item.as_ref() {
            item.setTitle(&NSString::from_str(texts.language_zh()));
        }
    });
}

/// 设置状态栏菜单
#[allow(clippy::too_many_lines)]
pub fn setup_status_item(delegate: &RestGapDelegate) {
    let mtm = delegate.mtm();
    let status_item =
        NSStatusBar::systemStatusBar().statusItemWithLength(NSVariableStatusItemLength);

    let texts = Texts::new(with_state_ref(|s| s.config.effective_language()));

    let menu = NSMenu::new(mtm);
    menu.setAutoenablesItems(false);
    menu.setDelegate(Some(ProtocolObject::<dyn NSMenuDelegate>::from_ref(
        delegate,
    )));

    let header = texts.header_title(
        with_state_ref(|s| s.config.interval_minutes),
        with_state_ref(|s| s.config.break_seconds),
    );
    let header_item = NSMenuItem::sectionHeaderWithTitle(&NSString::from_str(&header), mtm);
    menu.addItem(&header_item);

    let next_break_item = unsafe {
        menu.addItemWithTitle_action_keyEquivalent(
            &NSString::from_str(texts.menu_next_break_placeholder()),
            None,
            &NSString::from_str(""),
        )
    };
    next_break_item.setEnabled(false);

    let remaining_break_item = unsafe {
        menu.addItemWithTitle_action_keyEquivalent(
            &NSString::from_str(texts.menu_remaining_placeholder()),
            None,
            &NSString::from_str(""),
        )
    };
    remaining_break_item.setEnabled(false);

    menu.addItem(&NSMenuItem::separatorItem(mtm));

    let rest_now_item = unsafe {
        menu.addItemWithTitle_action_keyEquivalent(
            &NSString::from_str(texts.menu_rest_now()),
            Some(sel!(restNow:)),
            &NSString::from_str(""),
        )
    };
    unsafe { rest_now_item.setTarget(Some(target_anyobject(delegate))) };

    let settings_item = unsafe {
        menu.addItemWithTitle_action_keyEquivalent(
            &NSString::from_str(texts.menu_settings()),
            Some(sel!(openSettings:)),
            &NSString::from_str(""),
        )
    };
    unsafe { settings_item.setTarget(Some(target_anyobject(delegate))) };

    menu.addItem(&NSMenuItem::separatorItem(mtm));

    let language_header_item =
        NSMenuItem::sectionHeaderWithTitle(&NSString::from_str(texts.menu_language_header()), mtm);
    menu.addItem(&language_header_item);

    let language_auto_item = unsafe {
        menu.addItemWithTitle_action_keyEquivalent(
            &NSString::from_str(texts.language_auto()),
            Some(sel!(languageAuto:)),
            &NSString::from_str(""),
        )
    };
    unsafe { language_auto_item.setTarget(Some(target_anyobject(delegate))) };

    let language_en_item = unsafe {
        menu.addItemWithTitle_action_keyEquivalent(
            &NSString::from_str(texts.language_en()),
            Some(sel!(languageEnglish:)),
            &NSString::from_str(""),
        )
    };
    unsafe { language_en_item.setTarget(Some(target_anyobject(delegate))) };

    let language_zh_item = unsafe {
        menu.addItemWithTitle_action_keyEquivalent(
            &NSString::from_str(texts.language_zh()),
            Some(sel!(languageChinese:)),
            &NSString::from_str(""),
        )
    };
    unsafe { language_zh_item.setTarget(Some(target_anyobject(delegate))) };

    menu.addItem(&NSMenuItem::separatorItem(mtm));

    let about_item = unsafe {
        menu.addItemWithTitle_action_keyEquivalent(
            &NSString::from_str(&texts.menu_about()),
            Some(sel!(about:)),
            &NSString::from_str(""),
        )
    };
    unsafe { about_item.setTarget(Some(target_anyobject(delegate))) };

    menu.addItem(&NSMenuItem::separatorItem(mtm));

    let quit_item = unsafe {
        menu.addItemWithTitle_action_keyEquivalent(
            &NSString::from_str(texts.menu_quit()),
            Some(sel!(quit:)),
            &NSString::from_str(""),
        )
    };
    unsafe { quit_item.setTarget(Some(target_anyobject(delegate))) };

    status_item.setMenu(Some(&menu));

    with_state(|state| {
        state.status_item = Some(status_item);
        state.header_item = Some(header_item);
        state.rest_now_item = Some(rest_now_item);
        state.settings_item = Some(settings_item);
        state.language_auto_item = Some(language_auto_item);
        state.language_en_item = Some(language_en_item);
        state.language_zh_item = Some(language_zh_item);
        state.about_item = Some(about_item);
        state.quit_item = Some(quit_item);
        state.next_break_item = Some(next_break_item);
        state.remaining_break_item = Some(remaining_break_item);
    });
}
