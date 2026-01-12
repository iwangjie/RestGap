//! 系统托盘模块

use std::time::{Duration, Instant};

use windows::Win32::Foundation::HWND;
use windows::Win32::UI::Shell::{
    NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NIM_MODIFY, NOTIFYICONDATAW,
    Shell_NotifyIconW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreatePopupMenu, DestroyMenu, GetCursorPos, IDI_APPLICATION, LoadIconW,
    MF_CHECKED, MF_DISABLED, MF_GRAYED, MF_SEPARATOR, MF_STRING, SetForegroundWindow,
    TPM_BOTTOMALIGN, TPM_LEFTALIGN, TrackPopupMenu,
};
use windows::core::PCWSTR;

use super::super::constants::{
    ID_MENU_ABOUT, ID_MENU_HEADER, ID_MENU_LANGUAGE_AUTO, ID_MENU_LANGUAGE_EN,
    ID_MENU_LANGUAGE_HEADER, ID_MENU_LANGUAGE_ZH, ID_MENU_NEXT_BREAK, ID_MENU_QUIT,
    ID_MENU_REMAINING, ID_MENU_REST_NOW, ID_MENU_SETTINGS, TRAY_ICON_ID, WM_TRAY_CALLBACK,
};
use super::super::state::{Phase, with_state, with_state_ref};
use super::super::utils::{approx_duration, format_hhmm, to_wide_array, to_wide_string};
use crate::i18n::{LanguagePreference, Texts};

/// 设置系统托盘图标
pub fn setup_tray_icon(hwnd: HWND) {
    let config = with_state_ref(|s| s.config.clone());
    let texts = Texts::new(config.effective_language());

    // 创建托盘菜单
    let menu = unsafe { CreatePopupMenu() }.expect("Failed to create popup menu");

    // 添加菜单项
    let header = texts.header_title(config.interval_minutes, config.break_seconds);
    let header_wide = to_wide_string(&header);
    unsafe {
        let _ = AppendMenuW(
            menu,
            MF_STRING | MF_DISABLED | MF_GRAYED,
            ID_MENU_HEADER as usize,
            PCWSTR(header_wide.as_ptr()),
        );
    }

    let next_break_wide = to_wide_string(texts.menu_next_break_placeholder());
    unsafe {
        let _ = AppendMenuW(
            menu,
            MF_STRING | MF_DISABLED | MF_GRAYED,
            ID_MENU_NEXT_BREAK as usize,
            PCWSTR(next_break_wide.as_ptr()),
        );
    }

    let remaining_wide = to_wide_string(texts.menu_remaining_placeholder());
    unsafe {
        let _ = AppendMenuW(
            menu,
            MF_STRING | MF_DISABLED | MF_GRAYED,
            ID_MENU_REMAINING as usize,
            PCWSTR(remaining_wide.as_ptr()),
        );
    }

    unsafe {
        let _ = AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null());
    }

    let rest_now_wide = to_wide_string(texts.menu_rest_now());
    unsafe {
        let _ = AppendMenuW(
            menu,
            MF_STRING,
            ID_MENU_REST_NOW as usize,
            PCWSTR(rest_now_wide.as_ptr()),
        );
    }

    let settings_wide = to_wide_string(texts.menu_settings());
    unsafe {
        let _ = AppendMenuW(
            menu,
            MF_STRING,
            ID_MENU_SETTINGS as usize,
            PCWSTR(settings_wide.as_ptr()),
        );
    }

    let about_wide = to_wide_string(&texts.menu_about());
    unsafe {
        let _ = AppendMenuW(
            menu,
            MF_STRING,
            ID_MENU_ABOUT as usize,
            PCWSTR(about_wide.as_ptr()),
        );
    }

    unsafe {
        let _ = AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null());
    }

    // Language options (stored in config; Auto follows OS language)
    let language_header_wide = to_wide_string(texts.menu_language_header());
    unsafe {
        let _ = AppendMenuW(
            menu,
            MF_STRING | MF_DISABLED | MF_GRAYED,
            ID_MENU_LANGUAGE_HEADER as usize,
            PCWSTR(language_header_wide.as_ptr()),
        );
    }

    let auto_flags = if config.language == LanguagePreference::Auto {
        MF_STRING | MF_CHECKED
    } else {
        MF_STRING
    };
    let en_flags = if config.language == LanguagePreference::En {
        MF_STRING | MF_CHECKED
    } else {
        MF_STRING
    };
    let zh_flags = if config.language == LanguagePreference::Zh {
        MF_STRING | MF_CHECKED
    } else {
        MF_STRING
    };

    let language_auto_wide = to_wide_string(texts.language_auto());
    unsafe {
        let _ = AppendMenuW(
            menu,
            auto_flags,
            ID_MENU_LANGUAGE_AUTO as usize,
            PCWSTR(language_auto_wide.as_ptr()),
        );
    }

    let language_en_wide = to_wide_string(texts.language_en());
    unsafe {
        let _ = AppendMenuW(
            menu,
            en_flags,
            ID_MENU_LANGUAGE_EN as usize,
            PCWSTR(language_en_wide.as_ptr()),
        );
    }

    let language_zh_wide = to_wide_string(texts.language_zh());
    unsafe {
        let _ = AppendMenuW(
            menu,
            zh_flags,
            ID_MENU_LANGUAGE_ZH as usize,
            PCWSTR(language_zh_wide.as_ptr()),
        );
    }

    unsafe {
        let _ = AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null());
    }

    let quit_wide = to_wide_string(texts.menu_quit());
    unsafe {
        let _ = AppendMenuW(
            menu,
            MF_STRING,
            ID_MENU_QUIT as usize,
            PCWSTR(quit_wide.as_ptr()),
        );
    }

    // 创建托盘图标数据
    #[allow(clippy::cast_possible_truncation)]
    let nid = NOTIFYICONDATAW {
        cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
        hWnd: hwnd,
        uID: TRAY_ICON_ID,
        uFlags: NIF_ICON | NIF_MESSAGE | NIF_TIP,
        uCallbackMessage: WM_TRAY_CALLBACK,
        hIcon: load_tray_icon(),
        szTip: to_wide_array::<128>(&texts.tray_tip_app()),
        ..Default::default()
    };

    // 添加托盘图标
    unsafe {
        let _ = Shell_NotifyIconW(NIM_ADD, &raw const nid);
    }

    // 保存状态
    with_state(|state| {
        state.main_hwnd = Some(hwnd);
        state.tray_menu = Some(menu);
        state.tray_icon_data = Some(nid);
    });
}

/// 加载托盘图标
fn load_tray_icon() -> windows::Win32::UI::WindowsAndMessaging::HICON {
    // 使用稳定的系统默认图标，避免因资源加载失败导致托盘图标缺失。
    // 需要更换为应用图标时，建议通过资源文件（.rc）或在安装包中提供 .ico。
    unsafe {
        LoadIconW(None, IDI_APPLICATION)
            .unwrap_or_else(|_| windows::Win32::UI::WindowsAndMessaging::HICON::default())
    }
}

/// 显示托盘菜单
pub fn show_tray_menu(hwnd: HWND) {
    let menu = with_state_ref(|s| s.tray_menu);
    let Some(menu) = menu else { return };

    // 刷新菜单信息
    refresh_menu_info();

    let mut pt = windows::Win32::Foundation::POINT::default();
    unsafe {
        let _ = GetCursorPos(&raw mut pt);
        // 必须先调用 SetForegroundWindow，否则菜单不会在点击其他地方时消失
        let _ = SetForegroundWindow(hwnd);
        let _ = TrackPopupMenu(
            menu,
            TPM_LEFTALIGN | TPM_BOTTOMALIGN,
            pt.x,
            pt.y,
            0,
            hwnd,
            None,
        );
    }
}

/// 刷新状态栏标题（托盘图标提示）
pub fn refresh_status_title() {
    with_state(|state| {
        let texts = Texts::new(state.config.effective_language());
        let Some(ref mut nid) = state.tray_icon_data else {
            return;
        };

        let tip = match state.phase {
            Phase::Working => {
                let hm = state
                    .phase_deadline_wall
                    .map_or_else(|| "--:--".to_string(), format_hhmm);
                texts.status_tip_working(&hm)
            }
            Phase::Breaking => {
                let remaining = state
                    .phase_deadline_mono
                    .and_then(|t| t.checked_duration_since(Instant::now()))
                    .unwrap_or(Duration::from_secs(0));
                texts.status_tip_breaking(&approx_duration(remaining))
            }
        };

        nid.szTip = to_wide_array::<128>(&tip);
        nid.uFlags = NIF_TIP;

        unsafe {
            let _ = Shell_NotifyIconW(NIM_MODIFY, nid);
        }
    });
}

/// 设置"现在休息"菜单项的启用状态
pub fn set_rest_now_enabled(enabled: bool) {
    with_state(|state| {
        let Some(menu) = state.tray_menu else { return };

        unsafe {
            use windows::Win32::UI::WindowsAndMessaging::{
                EnableMenuItem, MF_BYCOMMAND, MF_ENABLED,
            };
            let flags = if enabled {
                MF_BYCOMMAND | MF_ENABLED
            } else {
                MF_BYCOMMAND | MF_DISABLED | MF_GRAYED
            };
            let _ = EnableMenuItem(menu, u32::from(ID_MENU_REST_NOW), flags);
        }
    });
}

/// 刷新菜单信息
pub fn refresh_menu_info() {
    let now = Instant::now();
    with_state(|state| {
        let texts = Texts::new(state.config.effective_language());
        let Some(menu) = state.tray_menu else { return };

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

        let remaining_title = match state.phase {
            Phase::Working => texts.remaining_title_working().to_string(),
            Phase::Breaking => {
                let remaining = phase_deadline_mono
                    .and_then(|t| t.checked_duration_since(now))
                    .unwrap_or(Duration::from_secs(0));
                let end_hm = phase_deadline_wall.map_or_else(|| "--:--".to_string(), format_hhmm);
                texts.remaining_title_breaking(&approx_duration(remaining), &end_hm)
            }
        };

        // 更新菜单项文本
        unsafe {
            use windows::Win32::UI::WindowsAndMessaging::{MF_BYCOMMAND, ModifyMenuW};

            let next_wide = to_wide_string(&next_title);
            let _ = ModifyMenuW(
                menu,
                u32::from(ID_MENU_NEXT_BREAK),
                MF_BYCOMMAND | MF_STRING | MF_DISABLED | MF_GRAYED,
                ID_MENU_NEXT_BREAK as usize,
                PCWSTR(next_wide.as_ptr()),
            );

            let remaining_wide = to_wide_string(&remaining_title);
            let _ = ModifyMenuW(
                menu,
                u32::from(ID_MENU_REMAINING),
                MF_BYCOMMAND | MF_STRING | MF_DISABLED | MF_GRAYED,
                ID_MENU_REMAINING as usize,
                PCWSTR(remaining_wide.as_ptr()),
            );
        }
    });
}

/// 刷新头部标题
pub fn refresh_header_title() {
    with_state(|state| {
        let Some(menu) = state.tray_menu else { return };

        let texts = Texts::new(state.config.effective_language());
        let title = texts.header_title(state.config.interval_minutes, state.config.break_seconds);

        unsafe {
            use windows::Win32::UI::WindowsAndMessaging::{MF_BYCOMMAND, ModifyMenuW};

            let title_wide = to_wide_string(&title);
            let _ = ModifyMenuW(
                menu,
                u32::from(ID_MENU_HEADER),
                MF_BYCOMMAND | MF_STRING | MF_DISABLED | MF_GRAYED,
                ID_MENU_HEADER as usize,
                PCWSTR(title_wide.as_ptr()),
            );
        }
    });
}

pub fn refresh_static_menu_titles() {
    with_state(|state| {
        let texts = Texts::new(state.config.effective_language());
        let Some(menu) = state.tray_menu else { return };

        unsafe {
            use windows::Win32::UI::WindowsAndMessaging::{MF_BYCOMMAND, ModifyMenuW};

            let rest_now_wide = to_wide_string(texts.menu_rest_now());
            let _ = ModifyMenuW(
                menu,
                u32::from(ID_MENU_REST_NOW),
                MF_BYCOMMAND | MF_STRING,
                ID_MENU_REST_NOW as usize,
                PCWSTR(rest_now_wide.as_ptr()),
            );

            let settings_wide = to_wide_string(texts.menu_settings());
            let _ = ModifyMenuW(
                menu,
                u32::from(ID_MENU_SETTINGS),
                MF_BYCOMMAND | MF_STRING,
                ID_MENU_SETTINGS as usize,
                PCWSTR(settings_wide.as_ptr()),
            );

            let about_wide = to_wide_string(&texts.menu_about());
            let _ = ModifyMenuW(
                menu,
                u32::from(ID_MENU_ABOUT),
                MF_BYCOMMAND | MF_STRING,
                ID_MENU_ABOUT as usize,
                PCWSTR(about_wide.as_ptr()),
            );

            let quit_wide = to_wide_string(texts.menu_quit());
            let _ = ModifyMenuW(
                menu,
                u32::from(ID_MENU_QUIT),
                MF_BYCOMMAND | MF_STRING,
                ID_MENU_QUIT as usize,
                PCWSTR(quit_wide.as_ptr()),
            );

            let language_header_wide = to_wide_string(texts.menu_language_header());
            let _ = ModifyMenuW(
                menu,
                u32::from(ID_MENU_LANGUAGE_HEADER),
                MF_BYCOMMAND | MF_STRING | MF_DISABLED | MF_GRAYED,
                ID_MENU_LANGUAGE_HEADER as usize,
                PCWSTR(language_header_wide.as_ptr()),
            );

            let language_auto_wide = to_wide_string(texts.language_auto());
            let _ = ModifyMenuW(
                menu,
                u32::from(ID_MENU_LANGUAGE_AUTO),
                MF_BYCOMMAND | MF_STRING,
                ID_MENU_LANGUAGE_AUTO as usize,
                PCWSTR(language_auto_wide.as_ptr()),
            );

            let language_en_wide = to_wide_string(texts.language_en());
            let _ = ModifyMenuW(
                menu,
                u32::from(ID_MENU_LANGUAGE_EN),
                MF_BYCOMMAND | MF_STRING,
                ID_MENU_LANGUAGE_EN as usize,
                PCWSTR(language_en_wide.as_ptr()),
            );

            let language_zh_wide = to_wide_string(texts.language_zh());
            let _ = ModifyMenuW(
                menu,
                u32::from(ID_MENU_LANGUAGE_ZH),
                MF_BYCOMMAND | MF_STRING,
                ID_MENU_LANGUAGE_ZH as usize,
                PCWSTR(language_zh_wide.as_ptr()),
            );
        }

        // Update checked state (best-effort; menu must exist).
        unsafe {
            use windows::Win32::UI::WindowsAndMessaging::{
                CheckMenuItem, MF_BYCOMMAND, MF_CHECKED, MF_UNCHECKED,
            };
            let auto_flag = if state.config.language == LanguagePreference::Auto {
                MF_BYCOMMAND | MF_CHECKED
            } else {
                MF_BYCOMMAND | MF_UNCHECKED
            };
            let en_flag = if state.config.language == LanguagePreference::En {
                MF_BYCOMMAND | MF_CHECKED
            } else {
                MF_BYCOMMAND | MF_UNCHECKED
            };
            let zh_flag = if state.config.language == LanguagePreference::Zh {
                MF_BYCOMMAND | MF_CHECKED
            } else {
                MF_BYCOMMAND | MF_UNCHECKED
            };

            let _ = CheckMenuItem(menu, u32::from(ID_MENU_LANGUAGE_AUTO), auto_flag);
            let _ = CheckMenuItem(menu, u32::from(ID_MENU_LANGUAGE_EN), en_flag);
            let _ = CheckMenuItem(menu, u32::from(ID_MENU_LANGUAGE_ZH), zh_flag);
        }
    });
}

/// 移除托盘图标
pub fn remove_tray_icon() {
    with_state(|state| {
        if let Some(ref nid) = state.tray_icon_data {
            unsafe {
                let _ = Shell_NotifyIconW(NIM_DELETE, nid);
            }
        }
        if let Some(menu) = state.tray_menu.take() {
            unsafe {
                let _ = DestroyMenu(menu);
            }
        }
        state.tray_icon_data = None;
    });
}
