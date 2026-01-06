//! 定时器和阶段管理模块

use std::time::{Instant, SystemTime};

use windows::Win32::UI::WindowsAndMessaging::{KillTimer, SetTimer};

use super::constants::PHASE_TIMER_ID;
use super::state::{NotifyEvent, Phase, with_state};
use super::ui::{
    finish_countdown, refresh_header_title, refresh_menu_info, refresh_status_title,
    set_rest_now_enabled, show_countdown_window,
};
use crate::common::Config;

/// 调度阶段定时器
pub fn schedule_phase(phase: Phase) {
    let (hwnd, duration_ms) = with_state(|state| {
        // 取消现有定时器
        if let (Some(hwnd), Some(_)) = (state.main_hwnd, state.phase_timer_id.take()) {
            unsafe {
                let _ = KillTimer(hwnd, PHASE_TIMER_ID);
            }
        }

        state.phase = phase;
        let duration = match phase {
            Phase::Working => state.config.work_interval(),
            Phase::Breaking => state.config.break_duration(),
        };

        state.phase_deadline_mono = Some(Instant::now() + duration);
        state.phase_deadline_wall = Some(SystemTime::now() + duration);

        #[allow(clippy::cast_possible_truncation)]
        let duration_ms = duration.as_millis().min(u32::MAX as u128) as u32;
        (state.main_hwnd, duration_ms)
    });

    // 设置新定时器
    if let Some(hwnd) = hwnd {
        unsafe {
            let _ = SetTimer(hwnd, PHASE_TIMER_ID, duration_ms, None);
        }
        with_state(|state| {
            state.phase_timer_id = Some(PHASE_TIMER_ID);
        });
    }

    refresh_status_title();
    refresh_header_title();
    set_rest_now_enabled(phase == Phase::Working);
    refresh_menu_info();
}

/// 发送通知
fn notify(event: NotifyEvent, config: &Config) {
    match event {
        NotifyEvent::BreakStart => {
            // 休息开始: 弹出原生倒计时窗口
            show_countdown_window(config.break_seconds, true);
        }
        NotifyEvent::BreakEnd => {
            // 休息结束: 关闭倒计时窗口并显示通知
            finish_countdown();
        }
    }
}

/// 定时器触发时的阶段转换
pub fn transition_on_timer() {
    let (next_phase, event, config) = with_state(|state| {
        state.phase_timer_id.take();
        let config = state.config.clone();
        match state.phase {
            Phase::Working => (Phase::Breaking, NotifyEvent::BreakStart, config),
            Phase::Breaking => (Phase::Working, NotifyEvent::BreakEnd, config),
        }
    });

    notify(event, &config);
    schedule_phase(next_phase);
}

/// 立即开始休息
pub fn start_break_now() {
    let (should_start, config) = with_state(|state| {
        let config = state.config.clone();
        let should_start = state.phase == Phase::Working;
        (should_start, config)
    });

    if should_start {
        notify(NotifyEvent::BreakStart, &config);
        schedule_phase(Phase::Breaking);
    }
}
