//! 倒计时窗口模块

use std::time::{Duration, Instant};

use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    BeginPaint, CreateFontW, DT_CENTER, DT_LEFT, DT_SINGLELINE, DT_TOP, DT_VCENTER, DeleteObject,
    DrawTextW, EndPaint, FillRect, GetStockObject, HBRUSH, InvalidateRect, PAINTSTRUCT,
    SelectObject, SetBkMode, SetTextColor, TRANSPARENT, WHITE_BRUSH,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, GetClientRect, GetSystemMetrics, HWND_TOPMOST,
    KillTimer, RegisterClassW, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN,
    SM_YVIRTUALSCREEN, SW_HIDE, SW_SHOW, SWP_NOMOVE, SWP_NOSIZE, SetTimer, SetWindowPos,
    ShowWindow, WM_CHAR, WM_CLOSE, WM_KEYDOWN, WM_LBUTTONDOWN, WM_PAINT, WM_TIMER, WNDCLASSW,
    WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP, WS_VISIBLE,
};
use windows::core::PCWSTR;

use super::super::constants::{
    COUNTDOWN_FEEDBACK_TIMER_ID, COUNTDOWN_TIMER_ID, COUNTDOWN_WINDOW_CLASS,
};
use super::super::state::{with_state, with_state_ref};
use super::super::utils::{SoundType, format_countdown, play_sound, to_wide_string};
use crate::i18n::Texts;
use crate::skip_challenge::{Feedback, SkipChallenge, Snapshot};

fn on_countdown_char(ch: char) -> bool {
    let mut completed = false;
    let mut hwnd = None;
    let mut should_start_feedback_timer = false;

    with_state(|state| {
        if state.countdown_hwnd.is_none() {
            return;
        }
        let Some(challenge) = state.countdown_skip_challenge.as_mut() else {
            return;
        };

        let result = challenge.register_char(ch, Instant::now());
        completed = result.completed;
        hwnd = state.countdown_hwnd;

        if matches!(
            result.snapshot.feedback,
            Feedback::Mismatch | Feedback::Timeout
        ) {
            state.countdown_feedback_flash_until =
                Some(Instant::now() + Duration::from_millis(420));
            should_start_feedback_timer = state.countdown_feedback_timer_id.is_none();
        }
    });

    if should_start_feedback_timer {
        if let Some(hwnd) = hwnd {
            unsafe {
                let _ = SetTimer(hwnd, COUNTDOWN_FEEDBACK_TIMER_ID, 60, None);
            }
            with_state(|state| {
                state.countdown_feedback_timer_id = Some(COUNTDOWN_FEEDBACK_TIMER_ID);
            });
        }
    }

    if let Some(hwnd) = hwnd {
        unsafe {
            let _ = InvalidateRect(hwnd, None, true);
        }
    }

    completed
}

const fn clamp_i32(v: i32, min: i32, max: i32) -> i32 {
    if v < min {
        min
    } else if v > max {
        max
    } else {
        v
    }
}

const fn line_rect(full: &RECT, center_y: i32, line_height: i32) -> RECT {
    let top = center_y - (line_height / 2);
    let bottom = center_y + (line_height / 2);
    RECT {
        left: full.left,
        top,
        right: full.right,
        bottom,
    }
}

fn centered_box(full: &RECT, top: i32, width: i32, height: i32) -> RECT {
    let horizontal_margin = ((full.right - full.left) - width).max(0) / 2;
    RECT {
        left: full.left + horizontal_margin,
        top,
        right: full.right - horizontal_margin,
        bottom: top + height,
    }
}

fn skip_status_text(texts: &Texts, snapshot: &Snapshot) -> String {
    match snapshot.feedback {
        Feedback::Completed => texts.countdown_skip_success().to_string(),
        Feedback::Mismatch => texts.countdown_skip_mismatch().to_string(),
        Feedback::Timeout => texts.countdown_skip_timeout().to_string(),
        Feedback::Ready | Feedback::Progress => {
            texts.countdown_skip_progress(snapshot.matched_len, snapshot.total_len)
        }
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
            let countdown_px = clamp_i32(base / 9, 64, 150);
            let star_px = clamp_i32(base / 4, 180, 320);
            let hint_px = clamp_i32(base / 28, 18, 36);
            let phrase_title_px = clamp_i32(base / 34, 18, 30);
            let phrase_px = clamp_i32(base / 26, 24, 42);
            let status_px = clamp_i32(base / 32, 18, 28);

            let title_center_y = rect.top + height * 20 / 100;
            let countdown_center_y = rect.top + height * 32 / 100;
            let star_center_y = rect.top + height * 50 / 100;
            let hint_center_y = rect.top + height * 62 / 100;
            let phrase_title_top = rect.top + height * 71 / 100;
            let phrase_top = rect.top + height * 77 / 100;
            let status_top = rect.top + height * 88 / 100;
            let phrase_box_width = width * 72 / 100;

            let snapshot = with_state_ref(|state| {
                state.countdown_skip_challenge.as_ref().map_or_else(
                    || SkipChallenge::new("").snapshot(),
                    SkipChallenge::snapshot,
                )
            });
            let horizontal_shake = with_state_ref(|state| {
                if !matches!(snapshot.feedback, Feedback::Mismatch | Feedback::Timeout) {
                    return 0;
                }
                let Some(until) = state.countdown_feedback_flash_until else {
                    return 0;
                };
                let Some(remaining) = until.checked_duration_since(Instant::now()) else {
                    return 0;
                };
                if (remaining.as_millis() / 60) % 2 == 0 {
                    -8
                } else {
                    8
                }
            });

            // 绘制标题（自动居中，避免 DPI/字体导致的错位）
            let title =
                Texts::new(with_state_ref(|s| s.config.effective_language())).countdown_title();
            let mut title_wide: Vec<u16> = title.encode_utf16().collect();
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
                &mut title_wide,
                &raw mut title_rect,
                DT_CENTER | DT_VCENTER | DT_SINGLELINE,
            );

            // 绘制倒计时数字
            let mut countdown_wide: Vec<u16> = countdown_text.encode_utf16().collect();
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
                &mut countdown_wide,
                &raw mut countdown_rect,
                DT_CENTER | DT_VCENTER | DT_SINGLELINE,
            );

            // 绘制提肛提醒的大星号
            let mut star_wide: Vec<u16> = "*".encode_utf16().collect();
            let star_font = CreateFontW(
                -star_px,
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
                windows::core::w!("Times New Roman"),
            );
            let _ = SelectObject(hdc, star_font);
            let mut star_rect = line_rect(&rect, star_center_y, star_px * 2);
            let _ = SetTextColor(hdc, COLORREF(0x0025_2525));
            let _ = DrawTextW(
                hdc,
                &mut star_wide,
                &raw mut star_rect,
                DT_CENTER | DT_VCENTER | DT_SINGLELINE,
            );

            // 绘制提示文本
            let hint =
                Texts::new(with_state_ref(|s| s.config.effective_language())).countdown_hint();
            let mut hint_wide: Vec<u16> = hint.encode_utf16().collect();
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
                &mut hint_wide,
                &raw mut hint_rect,
                DT_CENTER | DT_VCENTER | DT_SINGLELINE,
            );

            // 绘制跳过挑战标题
            let skip_title = Texts::new(with_state_ref(|s| s.config.effective_language()))
                .countdown_skip_title();
            let mut skip_title_wide: Vec<u16> = skip_title.encode_utf16().collect();
            let phrase_title_font = CreateFontW(
                -phrase_title_px,
                0,
                0,
                0,
                500,
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
            let _ = SelectObject(hdc, phrase_title_font);
            let _ = SetTextColor(hdc, COLORREF(0x0068_6868));
            let mut skip_title_rect = centered_box(
                &rect,
                phrase_title_top,
                phrase_box_width,
                phrase_title_px * 2,
            );
            skip_title_rect.left += horizontal_shake;
            skip_title_rect.right += horizontal_shake;
            let _ = DrawTextW(
                hdc,
                &mut skip_title_wide,
                &raw mut skip_title_rect,
                DT_LEFT | DT_TOP | DT_SINGLELINE,
            );

            // 绘制目标句子
            let phrase = snapshot.phrase;
            let mut phrase_wide: Vec<u16> = phrase.encode_utf16().collect();
            let phrase_font = CreateFontW(
                -phrase_px,
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
                windows::core::w!("Consolas"),
            );
            let _ = SelectObject(hdc, phrase_font);
            let mut phrase_rect = centered_box(&rect, phrase_top, phrase_box_width, phrase_px * 3);
            phrase_rect.left += horizontal_shake;
            phrase_rect.right += horizontal_shake;
            let _ = SetTextColor(hdc, COLORREF(0x0078_7878));
            let _ = DrawTextW(
                hdc,
                &mut phrase_wide,
                &raw mut phrase_rect,
                DT_LEFT | DT_TOP,
            );

            if snapshot.matched_len > 0 {
                let mut matched_wide: Vec<u16> = snapshot.phrase[..snapshot.matched_len]
                    .encode_utf16()
                    .collect();
                let mut matched_rect = phrase_rect;
                let _ = SetTextColor(hdc, COLORREF(0x0038_654B));
                let _ = DrawTextW(
                    hdc,
                    &mut matched_wide,
                    &raw mut matched_rect,
                    DT_LEFT | DT_TOP | DT_SINGLELINE,
                );
            }

            // 绘制状态
            let status_text = skip_status_text(
                &Texts::new(with_state_ref(|s| s.config.effective_language())),
                &snapshot,
            );
            let mut status_wide: Vec<u16> = status_text.encode_utf16().collect();
            let status_font = CreateFontW(
                -status_px,
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
            let _ = SelectObject(hdc, status_font);
            let status_color =
                if matches!(snapshot.feedback, Feedback::Mismatch | Feedback::Timeout) {
                    COLORREF(0x0032_5AC7)
                } else {
                    COLORREF(0x0078_7878)
                };
            let _ = SetTextColor(hdc, status_color);
            let mut status_rect = centered_box(&rect, status_top, phrase_box_width, status_px * 2);
            status_rect.left += horizontal_shake;
            status_rect.right += horizontal_shake;
            let _ = DrawTextW(
                hdc,
                &mut status_wide,
                &raw mut status_rect,
                DT_LEFT | DT_TOP,
            );

            // 清理
            let _ = SelectObject(hdc, old_font);
            let _ = DeleteObject(title_font);
            let _ = DeleteObject(countdown_font);
            let _ = DeleteObject(star_font);
            let _ = DeleteObject(hint_font);
            let _ = DeleteObject(phrase_title_font);
            let _ = DeleteObject(phrase_font);
            let _ = DeleteObject(status_font);

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
            } else if wparam.0 == COUNTDOWN_FEEDBACK_TIMER_ID {
                let should_continue = with_state(|state| {
                    state
                        .countdown_feedback_flash_until
                        .is_some_and(|until| until > Instant::now())
                });
                if should_continue {
                } else {
                    with_state(|state| {
                        state.countdown_feedback_flash_until = None;
                        state.countdown_feedback_timer_id = None;
                    });
                    let _ = KillTimer(hwnd, COUNTDOWN_FEEDBACK_TIMER_ID);
                }
                let _ = InvalidateRect(hwnd, None, true);
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
        state.countdown_feedback_timer_id = None;
        state.countdown_feedback_flash_until = None;
        state.countdown_skip_challenge = Some(SkipChallenge::random());
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
        if let (Some(hwnd), Some(timer_id)) =
            (state.countdown_hwnd, state.countdown_feedback_timer_id)
        {
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
        state.countdown_feedback_timer_id = None;
        state.countdown_feedback_flash_until = None;
        state.countdown_skip_challenge = None;
    });
}

/// 完成倒计时
pub fn finish_countdown() {
    close_countdown_window();
    play_sound(SoundType::BreakEnd);
}
