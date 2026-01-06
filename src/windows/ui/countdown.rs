//! 倒计时窗口模块

use std::time::{Duration, Instant};

use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    BeginPaint, CreateFontW, DT_CENTER, DT_SINGLELINE, DT_VCENTER, DeleteObject, DrawTextW,
    EndPaint, FillRect, GetStockObject, HBRUSH, InvalidateRect, PAINTSTRUCT, SelectObject,
    SetBkMode, SetTextColor, TRANSPARENT, WHITE_BRUSH,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, GetClientRect, GetSystemMetrics, HWND_TOPMOST,
    KillTimer, RegisterClassW, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN,
    SM_YVIRTUALSCREEN, SW_HIDE, SW_SHOW, SWP_NOMOVE, SWP_NOSIZE, SetTimer, SetWindowPos,
    ShowWindow, WM_CHAR, WM_CLOSE, WM_KEYDOWN, WM_LBUTTONDOWN, WM_PAINT, WM_TIMER, WNDCLASSW,
    WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP, WS_VISIBLE,
};
use windows::core::PCWSTR;

use super::super::constants::{APP_NAME_ZH, COUNTDOWN_TIMER_ID, COUNTDOWN_WINDOW_CLASS};
use super::super::state::with_state;
use super::super::utils::{SoundType, format_countdown, play_sound, to_wide_string};

const SKIP_PHRASE_SMART: &str = "i don’t care about my health.";
const SKIP_PHRASE_ASCII: &str = "i don't care about my health.";

fn advance_phrase_idx(idx: &mut usize, phrase: &str, ch: char) -> bool {
    let mut buf = [0u8; 4];
    let ch_bytes = ch.encode_utf8(&mut buf).as_bytes();
    let phrase_bytes = phrase.as_bytes();

    if *idx + ch_bytes.len() <= phrase_bytes.len()
        && &phrase_bytes[*idx..(*idx + ch_bytes.len())] == ch_bytes
    {
        *idx += ch_bytes.len();
        return *idx == phrase_bytes.len();
    }

    *idx = 0;
    if ch_bytes.len() <= phrase_bytes.len() && &phrase_bytes[..ch_bytes.len()] == ch_bytes {
        *idx = ch_bytes.len();
        return *idx == phrase_bytes.len();
    }
    false
}

fn on_countdown_char(ch: char) -> bool {
    let ch = if ch.is_ascii() {
        ch.to_ascii_lowercase()
    } else {
        ch
    };
    with_state(|state| {
        if state.countdown_hwnd.is_none() {
            return false;
        }

        let smart_done =
            advance_phrase_idx(&mut state.countdown_skip_smart_idx, SKIP_PHRASE_SMART, ch);
        let ascii_done =
            advance_phrase_idx(&mut state.countdown_skip_ascii_idx, SKIP_PHRASE_ASCII, ch);
        if smart_done || ascii_done {
            state.countdown_skip_smart_idx = 0;
            state.countdown_skip_ascii_idx = 0;
            return true;
        }
        false
    })
}

fn clamp_i32(v: i32, min: i32, max: i32) -> i32 {
    if v < min {
        min
    } else if v > max {
        max
    } else {
        v
    }
}

fn line_rect(full: &RECT, center_y: i32, line_height: i32) -> RECT {
    let top = center_y - (line_height / 2);
    let bottom = center_y + (line_height / 2);
    RECT {
        left: full.left,
        top,
        right: full.right,
        bottom,
    }
}

/// 倒计时窗口过程
#[allow(unsafe_op_in_unsafe_fn, clippy::too_many_lines)]
pub unsafe extern "system" fn countdown_wndproc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_PAINT => {
            let mut ps = PAINTSTRUCT::default();
            let hdc = BeginPaint(hwnd, &raw mut ps);

            let mut rect = RECT::default();
            let _ = GetClientRect(hwnd, &raw mut rect);

            // 填充背景
            let brush = GetStockObject(WHITE_BRUSH);
            FillRect(hdc, &raw const rect, HBRUSH(brush.0));

            // 设置文本属性
            let _ = SetBkMode(hdc, TRANSPARENT);
            let _ = SetTextColor(hdc, COLORREF(0x0000_0000)); // 黑色

            // 获取倒计时文本
            let countdown_text = with_state(|state| {
                state.countdown_end_time.map_or_else(
                    || "00:00".to_string(),
                    |end_time| {
                        let now = Instant::now();
                        if now >= end_time {
                            "00:00".to_string()
                        } else {
                            let remaining = end_time.duration_since(now);
                            format_countdown(remaining.as_secs())
                        }
                    },
                )
            });

            let width = rect.right - rect.left;
            let height = rect.bottom - rect.top;
            let base = width.min(height).max(1);

            let title_px = clamp_i32(base / 18, 28, 56);
            let countdown_px = clamp_i32(base / 9, 64, 144);
            let hint_px = clamp_i32(base / 28, 18, 36);

            let title_center_y = rect.top + height * 32 / 100;
            let countdown_center_y = rect.top + height * 50 / 100;
            let hint_center_y = rect.top + height * 68 / 100;

            // 绘制标题（自动居中，避免 DPI/字体导致的错位）
            let title = format!("{APP_NAME_ZH} · 休息倒计时");
            let title_wide = to_wide_string(&title);
            let title_font = CreateFontW(
                -title_px,
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
                windows::core::w!("Segoe UI"),
            );
            let old_font = SelectObject(hdc, title_font);
            let mut title_rect = line_rect(&rect, title_center_y, title_px * 2);
            let _ = DrawTextW(
                hdc,
                PCWSTR(title_wide.as_ptr()),
                -1,
                &raw mut title_rect,
                DT_CENTER | DT_VCENTER | DT_SINGLELINE,
            );

            // 绘制倒计时数字
            let countdown_wide = to_wide_string(&countdown_text);
            let countdown_font = CreateFontW(
                -countdown_px,
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
                windows::core::w!("Segoe UI"),
            );
            let _ = SelectObject(hdc, countdown_font);
            let mut countdown_rect = line_rect(&rect, countdown_center_y, countdown_px * 2);
            let _ = DrawTextW(
                hdc,
                PCWSTR(countdown_wide.as_ptr()),
                -1,
                &raw mut countdown_rect,
                DT_CENTER | DT_VCENTER | DT_SINGLELINE,
            );

            // 绘制提示文本
            let hint = "放松眼睛，伸展身体";
            let hint_wide = to_wide_string(hint);
            let hint_font = CreateFontW(
                -hint_px,
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
                windows::core::w!("Segoe UI"),
            );
            let _ = SelectObject(hdc, hint_font);
            let _ = SetTextColor(hdc, COLORREF(0x0080_8080)); // 灰色
            let mut hint_rect = line_rect(&rect, hint_center_y, hint_px * 2);
            let _ = DrawTextW(
                hdc,
                PCWSTR(hint_wide.as_ptr()),
                -1,
                &raw mut hint_rect,
                DT_CENTER | DT_VCENTER | DT_SINGLELINE,
            );

            // 清理
            let _ = SelectObject(hdc, old_font);
            let _ = DeleteObject(title_font);
            let _ = DeleteObject(countdown_font);
            let _ = DeleteObject(hint_font);

            let _ = EndPaint(hwnd, &raw const ps);
            LRESULT(0)
        }
        WM_TIMER => {
            if wparam.0 == COUNTDOWN_TIMER_ID {
                if update_countdown() {
                    // 刷新窗口
                    let _ = InvalidateRect(hwnd, None, true);
                } else {
                    // 倒计时结束
                    finish_countdown();
                }
            }
            LRESULT(0)
        }
        WM_CHAR => {
            // 仅在休息窗口激活时处理隐藏短语；不增加非休息时开销（无全局钩子/轮询）。
            if let Ok(code) = u32::try_from(wparam.0) {
                if let Some(ch) = char::from_u32(code) {
                    // 过滤控制字符（保留空格）
                    if (ch >= ' ' && ch != '\u{7f}') && on_countdown_char(ch) {
                        super::super::timer::skip_break();
                    }
                }
            }
            LRESULT(0)
        }
        // 禁止跳过休息：吞掉鼠标点击 / 键盘事件
        WM_LBUTTONDOWN | WM_KEYDOWN => LRESULT(0),
        WM_CLOSE => {
            // 禁止用户关闭窗口；程序结束/计时结束时会主动 DestroyWindow
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

    unsafe { RegisterClassW(&raw const wc) != 0 }
}

/// 显示倒计时窗口
pub fn show_countdown_window(seconds: u64, play_start_sound: bool) {
    // 关闭已存在的倒计时窗口
    close_countdown_window();

    // 播放开始声音
    if play_start_sound {
        play_sound(SoundType::BreakStart);
    }

    // 覆盖所有显示器（虚拟屏幕），避免多屏幕下窗口尺寸/位置错乱
    let screen_x = unsafe { GetSystemMetrics(SM_XVIRTUALSCREEN) };
    let screen_y = unsafe { GetSystemMetrics(SM_YVIRTUALSCREEN) };
    let screen_width = unsafe { GetSystemMetrics(SM_CXVIRTUALSCREEN) };
    let screen_height = unsafe { GetSystemMetrics(SM_CYVIRTUALSCREEN) };

    let class_name = to_wide_string(COUNTDOWN_WINDOW_CLASS);

    // 创建全屏窗口
    let hwnd = unsafe {
        CreateWindowExW(
            WS_EX_TOPMOST | WS_EX_TOOLWINDOW,
            PCWSTR(class_name.as_ptr()),
            PCWSTR::null(),
            WS_POPUP | WS_VISIBLE,
            screen_x,
            screen_y,
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
        state.countdown_skip_smart_idx = 0;
        state.countdown_skip_ascii_idx = 0;
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
        state.countdown_skip_smart_idx = 0;
        state.countdown_skip_ascii_idx = 0;
    });
}

/// 完成倒计时
pub fn finish_countdown() {
    close_countdown_window();
    play_sound(SoundType::BreakEnd);
}
