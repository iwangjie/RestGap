//! 对话框模块

use std::process::Command;

use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{
    IDYES, MB_ICONINFORMATION, MB_OK, MB_YESNO, MessageBoxW,
};
use windows::core::PCWSTR;

use super::super::constants::APP_NAME_DISPLAY;
use super::super::state::{Phase, with_state, with_state_ref};
use super::super::timer::schedule_phase;
use super::super::utils::to_wide_string;
use super::countdown::show_countdown_window;
use crate::common::Config;

/// 显示无效配置警告
pub fn show_invalid_settings_alert(hwnd: Option<HWND>) {
    let title = to_wide_string("配置无效");
    let message = to_wide_string("请输入有效的数字：每 N 分钟休息 N 秒。");

    unsafe {
        let _ = MessageBoxW(
            hwnd.unwrap_or(HWND::default()),
            PCWSTR(message.as_ptr()),
            PCWSTR(title.as_ptr()),
            MB_OK | MB_ICONINFORMATION,
        );
    }
}

/// 打开配置对话框
///
/// 由于 Windows API 创建自定义对话框比较复杂，这里使用简单的 InputBox 方式
/// 通过两次 MessageBox 输入来获取配置
pub fn open_settings_dialog(hwnd: Option<HWND>) {
    let current = with_state_ref(|s| s.config.clone());

    // 使用简单的方式：显示当前配置，让用户确认是否修改
    // 实际生产环境中应该使用 DialogBoxIndirectParam 创建自定义对话框

    let title = to_wide_string("配置");
    let message = to_wide_string(&format!(
        "当前配置：\n\n每 {} 分钟休息 {} 秒\n\n保存后将从现在开始重新计时。\n\n是否使用默认配置（30分钟/120秒）？",
        current.interval_minutes, current.break_seconds
    ));

    let result = unsafe {
        MessageBoxW(
            hwnd.unwrap_or(HWND::default()),
            PCWSTR(message.as_ptr()),
            PCWSTR(title.as_ptr()),
            MB_YESNO | MB_ICONINFORMATION,
        )
    };

    if result == IDYES {
        // 使用默认配置
        let new_config = Config {
            interval_minutes: 30,
            break_seconds: 120,
        };
        new_config.save();

        let phase = with_state(|state| {
            state.config = new_config.clone();
            state.phase
        });

        // 从现在开始重新计时
        schedule_phase(phase);

        // 若当前正在休息，则同步更新倒计时窗口
        if phase == Phase::Breaking {
            show_countdown_window(new_config.break_seconds, false);
        }
    }
    // 如果选择"否"，保持当前配置不变
}

/// 显示关于对话框
pub fn show_about_dialog(hwnd: Option<HWND>) {
    let title = to_wide_string(APP_NAME_DISPLAY);
    let message = to_wide_string(&format!(
        "版本：{}\n\nWindows 系统托盘休息提醒（事件驱动 / 非轮询）。\n\n是否访问主页？",
        env!("CARGO_PKG_VERSION")
    ));

    let result = unsafe {
        MessageBoxW(
            hwnd.unwrap_or(HWND::default()),
            PCWSTR(message.as_ptr()),
            PCWSTR(title.as_ptr()),
            MB_YESNO | MB_ICONINFORMATION,
        )
    };

    if result == IDYES {
        let _ = Command::new("cmd")
            .args(["/C", "start", "https://github.com/iwangjie"])
            .spawn();
    }
}
