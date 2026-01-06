//! 窗口过程处理模块

use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{
    DefWindowProcW, PostQuitMessage, WM_COMMAND, WM_CREATE, WM_DESTROY, WM_TIMER,
};

use super::constants::{
    ID_MENU_ABOUT, ID_MENU_QUIT, ID_MENU_REST_NOW, ID_MENU_SETTINGS, PHASE_TIMER_ID,
    WM_TRAY_CALLBACK,
};
use super::state::Phase;
use super::timer::{schedule_phase, start_break_now, transition_on_timer};
use super::ui::dialogs::{open_settings_dialog, show_about_dialog};
use super::ui::tray::{remove_tray_icon, setup_tray_icon, show_tray_menu};

/// 主窗口过程
#[allow(unsafe_op_in_unsafe_fn)]
pub unsafe extern "system" fn main_wndproc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_CREATE => {
            // 初始化托盘图标
            setup_tray_icon(hwnd);
            // 开始工作阶段
            schedule_phase(Phase::Working);
            LRESULT(0)
        }
        WM_TIMER => {
            if wparam.0 == PHASE_TIMER_ID {
                transition_on_timer();
            }
            LRESULT(0)
        }
        WM_TRAY_CALLBACK => {
            // 托盘图标消息
            let event = lparam.0 as u32;
            match event {
                // WM_RBUTTONUP - 右键点击
                0x0205 => {
                    show_tray_menu(hwnd);
                }
                // WM_LBUTTONDBLCLK - 左键双击
                0x0203 => {
                    // 双击托盘图标显示菜单
                    show_tray_menu(hwnd);
                }
                _ => {}
            }
            LRESULT(0)
        }
        WM_COMMAND => {
            let menu_id = (wparam.0 & 0xFFFF) as u16;
            match menu_id {
                ID_MENU_REST_NOW => {
                    start_break_now();
                }
                ID_MENU_SETTINGS => {
                    open_settings_dialog(Some(hwnd));
                }
                ID_MENU_ABOUT => {
                    show_about_dialog(Some(hwnd));
                }
                ID_MENU_QUIT => {
                    // 移除托盘图标并退出
                    remove_tray_icon();
                    PostQuitMessage(0);
                }
                _ => {}
            }
            LRESULT(0)
        }
        WM_DESTROY => {
            remove_tray_icon();
            PostQuitMessage(0);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}
