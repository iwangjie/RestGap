//! 系统托盘模块

use std::time::{Duration, Instant};

use windows::Win32::Foundation::HWND;
use windows::Win32::UI::Shell::{
    NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NIM_MODIFY, NOTIFYICONDATAW,
    Shell_NotifyIconW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreatePopupMenu, DestroyMenu, GetCursorPos, HMENU, IMAGE_ICON, LR_SHARED,
    LoadImageW, MF_DISABLED, MF_GRAYED, MF_SEPARATOR, MF_STRING, SetForegroundWindow,
    TPM_BOTTOMALIGN, TPM_LEFTALIGN, TrackPopupMenu,
};
use windows::core::PCWSTR;

use super::super::constants::{
    APP_NAME_ZH, ID_MENU_ABOUT, ID_MENU_HEADER, ID_MENU_NEXT_BREAK, ID_MENU_QUIT,
    ID_MENU_REMAINING, ID_MENU_REST_NOW, ID_MENU_SETTINGS, TRAY_ICON_ID, WM_TRAY_CALLBACK,
};
use super::super::state::{Phase, with_state, with_state_ref};
use super::super::utils::{approx_duration, format_hhmm, to_wide_array, to_wide_string};

/// 设置系统托盘图标
pub fn setup_tray_icon(hwnd: HWND) {
    let config = with_state_ref(|s| s.config.clone());

    // 创建托盘菜单
    let menu = unsafe { CreatePopupMenu() }.expect("Failed to create popup menu");

    // 添加菜单项
    let header = format!(
        "{APP_NAME_ZH} · 每 {} 分钟休息 {} 秒",
        config.interval_minutes, config.break_seconds
    );
    let header_wide = to_wide_string(&header);
    unsafe {
        let _ = AppendMenuW(
            menu,
            MF_STRING | MF_DISABLED | MF_GRAYED,
            ID_MENU_HEADER as usize,
            PCWSTR(header_wide.as_ptr()),
        );
    }

    let next_break_wide = to_wide_string("下次休息：--:--");
    unsafe {
        let _ = AppendMenuW(
            menu,
            MF_STRING | MF_DISABLED | MF_GRAYED,
            ID_MENU_NEXT_BREAK as usize,
            PCWSTR(next_break_wide.as_ptr()),
        );
    }

    let remaining_wide = to_wide_string("休息剩余：—");
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

    let rest_now_wide = to_wide_string("现在休息");
    unsafe {
        let _ = AppendMenuW(
            menu,
            MF_STRING,
            ID_MENU_REST_NOW as usize,
            PCWSTR(rest_now_wide.as_ptr()),
        );
    }

    let settings_wide = to_wide_string("配置");
    unsafe {
        let _ = AppendMenuW(
            menu,
            MF_STRING,
            ID_MENU_SETTINGS as usize,
            PCWSTR(settings_wide.as_ptr()),
        );
    }

    let about_wide = to_wide_string(&format!("关于 {APP_NAME_ZH}"));
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

    let quit_wide = to_wide_string("退出");
    unsafe {
        let _ = AppendMenuW(
            menu,
            MF_STRING,
            ID_MENU_QUIT as usize,
            PCWSTR(quit_wide.as_ptr()),
        );
    }

    // 创建托盘图标数据
    let mut nid = NOTIFYICONDATAW {
        cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
        hWnd: hwnd,
        uID: TRAY_ICON_ID,
        uFlags: NIF_ICON | NIF_MESSAGE | NIF_TIP,
        uCallbackMessage: WM_TRAY_CALLBACK,
        hIcon: load_tray_icon(),
        szTip: to_wide_array::<128>(&format!("{APP_NAME_ZH} - 休息提醒")),
        ..Default::default()
    };

    // 添加托盘图标
    unsafe {
        let _ = Shell_NotifyIconW(NIM_ADD, &nid);
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
    // 尝试加载自定义图标，失败则使用系统默认图标
    // 这里使用系统默认图标作为后备
    unsafe {
        let icon = LoadImageW(
            None,
            windows::core::w!("shell32.dll,13"), // 使用系统图标
            IMAGE_ICON,
            16,
            16,
            LR_SHARED,
        );
        match icon {
            Ok(h) => windows::Win32::UI::WindowsAndMessaging::HICON(h.0),
            Err(_) => windows::Win32::UI::WindowsAndMessaging::HICON::default(),
        }
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
        let _ = GetCursorPos(&mut pt);
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
        let Some(ref mut nid) = state.tray_icon_data else {
            return;
        };

        let tip = match state.phase {
            Phase::Working => {
                let hm = state
                    .phase_deadline_wall
                    .map_or_else(|| "--:--".to_string(), format_hhmm);
                format!("⏰ 下次休息：{hm}")
            }
            Phase::Breaking => {
                let remaining = state
                    .phase_deadline_mono
                    .and_then(|t| t.checked_duration_since(Instant::now()))
                    .unwrap_or(Duration::from_secs(0));
                format!("☕ 休息中：{}", approx_duration(remaining))
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
            let _ = EnableMenuItem(menu, ID_MENU_REST_NOW as u32, flags);
        }
    });
}

/// 刷新菜单信息
pub fn refresh_menu_info() {
    let now = Instant::now();
    with_state(|state| {
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
        let next_title = format!(
            "下次休息：{}（{}）",
            next_hm,
            approx_duration(next_break_in)
        );

        let remaining_title = match state.phase {
            Phase::Working => "休息剩余：—".to_string(),
            Phase::Breaking => {
                let remaining = phase_deadline_mono
                    .and_then(|t| t.checked_duration_since(now))
                    .unwrap_or(Duration::from_secs(0));
                let end_hm = phase_deadline_wall.map_or_else(|| "--:--".to_string(), format_hhmm);
                format!("休息剩余：{}（至 {}）", approx_duration(remaining), end_hm)
            }
        };

        // 更新菜单项文本
        unsafe {
            use windows::Win32::UI::WindowsAndMessaging::{MF_BYCOMMAND, ModifyMenuW};

            let next_wide = to_wide_string(&next_title);
            let _ = ModifyMenuW(
                menu,
                ID_MENU_NEXT_BREAK as u32,
                MF_BYCOMMAND | MF_STRING | MF_DISABLED | MF_GRAYED,
                ID_MENU_NEXT_BREAK as usize,
                PCWSTR(next_wide.as_ptr()),
            );

            let remaining_wide = to_wide_string(&remaining_title);
            let _ = ModifyMenuW(
                menu,
                ID_MENU_REMAINING as u32,
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

        let title = format!(
            "{APP_NAME_ZH} · 每 {} 分钟休息 {} 秒",
            state.config.interval_minutes, state.config.break_seconds
        );

        unsafe {
            use windows::Win32::UI::WindowsAndMessaging::{MF_BYCOMMAND, ModifyMenuW};

            let title_wide = to_wide_string(&title);
            let _ = ModifyMenuW(
                menu,
                ID_MENU_HEADER as u32,
                MF_BYCOMMAND | MF_STRING | MF_DISABLED | MF_GRAYED,
                ID_MENU_HEADER as usize,
                PCWSTR(title_wide.as_ptr()),
            );
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
