//! 倒计时窗口模块

use std::time::{Duration, Instant};

use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    BeginPaint, CreateFontW, DeleteObject, EndPaint, FillRect, GetStockObject, HBRUSH,
    InvalidateRect, PAINTSTRUCT, SelectObject, SetBkMode, SetTextColor, TRANSPARENT, TextOutW,
    WHITE_BRUSH,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, GetClientRect, GetSystemMetrics, HWND_TOPMOST,
    KillTimer, RegisterClassW, SM_CXSCREEN, SM_CYSCREEN, SW_HIDE, SW_SHOW, SWP_NOMOVE, SWP_NOSIZE,
    SetTimer, SetWindowPos, ShowWindow, WM_CLOSE, WM_PAINT, WM_TIMER, WNDCLASSW, WS_EX_TOOLWINDOW,
    WS_EX_TOPMOST, WS_POPUP, WS_VISIBLE,
};
use windows::core::PCWSTR;

use super::super::constants::{APP_NAME_ZH, COUNTDOWN_TIMER_ID, COUNTDOWN_WINDOW_CLASS};
use super::super::state::with_state;
use super::super::utils::{SoundType, format_countdown, play_sound, to_wide_string};

/// 倒计时窗口过程
#[allow(unsafe_op_in_unsafe_fn)]
pub unsafe extern "system" fn countdown_wndproc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_PAINT => {
            let mut ps = PAINTSTRUCT::default();
            let hdc = BeginPaint(hwnd, &mut ps);

            let mut rect = RECT::default();
            let _ = GetClientRect(hwnd, &mut rect);

            // 填充背景
            let brush = GetStockObject(WHITE_BRUSH);
            FillRect(hdc, &rect, HBRUSH(brush.0));

            // 设置文本属性
            let _ = SetBkMode(hdc, TRANSPARENT);
            let _ = SetTextColor(hdc, COLORREF(0x0000_0000)); // 黑色

            // 获取倒计时文本
            let countdown_text = with_state(|state| {
                state
                    .countdown_end_time
                    .map(|end_time| {
                        let now = Instant::now();
                        if now >= end_time {
                            "00:00".to_string()
                        } else {
                            let remaining = end_time.duration_since(now);
                            format_countdown(remaining.as_secs())
                        }
                    })
                    .unwrap_or_else(|| "00:00".to_string())
            });

            // 绘制标题
            let title = format!("{APP_NAME_ZH} · 休息倒计时");
            let title_wide = to_wide_string(&title);
            let title_font = CreateFontW(
                48,
                0,
                0,
                0,
                400,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                windows::core::w!("Microsoft YaHei"),
            );
            let old_font = SelectObject(hdc, title_font);
            let title_y = rect.bottom / 2 - 100;
            let title_x = (rect.right - (title.chars().count() as i32 * 24)) / 2;
            let _ = TextOutW(hdc, title_x, title_y, &title_wide[..title_wide.len() - 1]);

            // 绘制倒计时数字
            let countdown_wide = to_wide_string(&countdown_text);
            let countdown_font = CreateFontW(
                96,
                0,
                0,
                0,
                700,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                windows::core::w!("Microsoft YaHei"),
            );
            let _ = SelectObject(hdc, countdown_font);
            let countdown_y = rect.bottom / 2 - 30;
            let countdown_x = (rect.right - (countdown_text.chars().count() as i32 * 48)) / 2;
            let _ = TextOutW(
                hdc,
                countdown_x,
                countdown_y,
                &countdown_wide[..countdown_wide.len() - 1],
            );

            // 绘制提示文本
            let hint = "放松眼睛，伸展身体";
            let hint_wide = to_wide_string(hint);
            let hint_font = CreateFontW(
                32,
                0,
                0,
                0,
                400,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                windows::core::w!("Microsoft YaHei"),
            );
            let _ = SelectObject(hdc, hint_font);
            let _ = SetTextColor(hdc, COLORREF(0x0080_8080)); // 灰色
            let hint_y = rect.bottom / 2 + 80;
            let hint_x = (rect.right - (hint.chars().count() as i32 * 16)) / 2;
            let _ = TextOutW(hdc, hint_x, hint_y, &hint_wide[..hint_wide.len() - 1]);

            // 绘制跳过按钮提示
            let skip_hint = "按 ESC 或点击任意位置跳过";
            let skip_wide = to_wide_string(skip_hint);
            let skip_font = CreateFontW(
                24,
                0,
                0,
                0,
                400,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                windows::core::w!("Microsoft YaHei"),
            );
            let _ = SelectObject(hdc, skip_font);
            let skip_y = rect.bottom / 2 + 140;
            let skip_x = (rect.right - (skip_hint.chars().count() as i32 * 12)) / 2;
            let _ = TextOutW(hdc, skip_x, skip_y, &skip_wide[..skip_wide.len() - 1]);

            // 清理
            let _ = SelectObject(hdc, old_font);
            let _ = DeleteObject(title_font);
            let _ = DeleteObject(countdown_font);
            let _ = DeleteObject(hint_font);
            let _ = DeleteObject(skip_font);

            let _ = EndPaint(hwnd, &ps);
            LRESULT(0)
        }
        WM_TIMER => {
            if wparam.0 == COUNTDOWN_TIMER_ID {
                if !update_countdown() {
                    // 倒计时结束
                    finish_countdown();
                } else {
                    // 刷新窗口
                    let _ = InvalidateRect(hwnd, None, true);
                }
            }
            LRESULT(0)
        }
        // 处理鼠标点击和键盘事件 - 跳过休息
        0x0201 | 0x0100 => {
            // WM_LBUTTONDOWN | WM_KEYDOWN
            // ESC 键或鼠标点击跳过
            if msg == 0x0100 && wparam.0 != 0x1B {
                // 不是 ESC 键
                return DefWindowProcW(hwnd, msg, wparam, lparam);
            }
            close_countdown_window();
            LRESULT(0)
        }
        WM_CLOSE => {
            close_countdown_window();
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

/// 注册倒计时窗口类
pub fn register_countdown_class() -> bool {
    let class_name = to_wide_string(COUNTDOWN_WINDOW_CLASS);

    let wc = WNDCLASSW {
        lpfnWndProc: Some(countdown_wndproc),
        lpszClassName: PCWSTR(class_name.as_ptr()),
        hbrBackground: unsafe { HBRUSH(GetStockObject(WHITE_BRUSH).0) },
        ..Default::default()
    };

    unsafe { RegisterClassW(&wc) != 0 }
}

/// 显示倒计时窗口
pub fn show_countdown_window(seconds: u64, play_start_sound: bool) {
    // 关闭已存在的倒计时窗口
    close_countdown_window();

    // 播放开始声音
    if play_start_sound {
        play_sound(SoundType::BreakStart);
    }

    // 获取屏幕尺寸
    let screen_width = unsafe { GetSystemMetrics(SM_CXSCREEN) };
    let screen_height = unsafe { GetSystemMetrics(SM_CYSCREEN) };

    let class_name = to_wide_string(COUNTDOWN_WINDOW_CLASS);

    // 创建全屏窗口
    let hwnd = unsafe {
        CreateWindowExW(
            WS_EX_TOPMOST | WS_EX_TOOLWINDOW,
            PCWSTR(class_name.as_ptr()),
            PCWSTR::null(),
            WS_POPUP | WS_VISIBLE,
            0,
            0,
            screen_width,
            screen_height,
            None,
            None,
            None,
            None,
        )
    };

    let Ok(hwnd) = hwnd else { return };

    // 确保窗口在最前
    unsafe {
        let _ = SetWindowPos(hwnd, HWND_TOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE);
        let _ = ShowWindow(hwnd, SW_SHOW);
    }

    // 设置结束时间
    let end_time = Instant::now() + Duration::from_secs(seconds);

    // 创建定时器每秒更新倒计时
    unsafe {
        let _ = SetTimer(hwnd, COUNTDOWN_TIMER_ID, 1000, None);
    }

    // 保存状态
    with_state(|state| {
        state.countdown_hwnd = Some(hwnd);
        state.countdown_timer_id = Some(COUNTDOWN_TIMER_ID);
        state.countdown_end_time = Some(end_time);
    });
}

/// 更新倒计时显示
pub fn update_countdown() -> bool {
    with_state(|state| {
        let Some(end_time) = state.countdown_end_time else {
            return false;
        };

        let now = Instant::now();
        if now >= end_time {
            // 倒计时结束
            return false;
        }

        true
    })
}

/// 关闭倒计时窗口
pub fn close_countdown_window() {
    with_state(|state| {
        // 先停止定时器
        if let (Some(hwnd), Some(timer_id)) = (state.countdown_hwnd, state.countdown_timer_id) {
            unsafe {
                let _ = KillTimer(hwnd, timer_id);
            }
        }

        // 销毁窗口
        if let Some(hwnd) = state.countdown_hwnd.take() {
            unsafe {
                let _ = ShowWindow(hwnd, SW_HIDE);
                let _ = DestroyWindow(hwnd);
            }
        }

        state.countdown_timer_id = None;
        state.countdown_end_time = None;
    });
}

/// 完成倒计时
pub fn finish_countdown() {
    close_countdown_window();
    play_sound(SoundType::BreakEnd);
}
