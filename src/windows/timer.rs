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
        let started_at = Instant::now();
        let duration = match phase {
            Phase::Working => state.config.work_interval(),
            Phase::Breaking => state.config.break_duration(),
        };

        state.phase_started_at_mono = Some(started_at);
        state.phase_deadline_mono = Some(started_at + duration);
        state.phase_deadline_wall = Some(SystemTime::now() + duration);

        #[allow(clippy::cast_possible_truncation)]
        let duration_ms = duration.as_millis().min(u128::from(u32::MAX)) as u32;
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
    let transition = with_state(|state| {
        state.phase_timer_id.take();
        let config = state.config.clone();
        match state.phase {
            Phase::Working => {
                let should_skip = state
                    .phase_started_at_mono
                    .is_some_and(|started_at| crate::idle::should_skip_break(started_at.elapsed()));
                if should_skip {
                    (Phase::Working, None, config)
                } else {
                    (Phase::Breaking, Some(NotifyEvent::BreakStart), config)
                }
            }
            Phase::Breaking => (Phase::Working, Some(NotifyEvent::BreakEnd), config),
        }
    });

    let (next_phase, event, config) = transition;
    if let Some(event) = event {
        notify(event, &config);
    }
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

/// 跳过当前休息（仅在 Breaking 阶段生效）
pub fn skip_break() {
    let should_skip = with_state(|state| state.phase == Phase::Breaking);
    if !should_skip {
        return;
    }

    // 提前结束休息：关闭倒计时窗口并回到工作阶段
    finish_countdown();
    schedule_phase(Phase::Working);
}
