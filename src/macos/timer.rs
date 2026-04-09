//! 定时器和阶段管理模块

use std::time::{Instant, SystemTime};

use objc2::sel;
use objc2_foundation::NSTimer;

use super::config::Config;
use super::delegate::RestGapDelegate;
use super::state::{NotifyEvent, Phase, with_state};
use super::ui::{
    finish_countdown, refresh_header_title, refresh_menu_info, refresh_status_title,
    set_rest_now_enabled, show_countdown_window, target_anyobject,
};

/// 调度阶段定时器
pub fn schedule_phase(delegate: &RestGapDelegate, phase: Phase) {
    let target = target_anyobject(delegate);

    let (seconds, tolerance) = with_state(|state| {
        if let Some(timer) = state.timer.take() {
            timer.invalidate();
        }

        state.phase = phase;
        let started_at = Instant::now();
        let (duration, tolerance) = match phase {
            Phase::Working => (state.config.work_interval(), state.config.work_tolerance()),
            Phase::Breaking => (
                state.config.break_duration(),
                state.config.break_tolerance(),
            ),
        };

        state.phase_started_at_mono = Some(started_at);
        state.phase_deadline_mono = Some(started_at + duration);
        state.phase_deadline_wall = Some(SystemTime::now() + duration);

        (duration.as_secs_f64(), tolerance.as_secs_f64())
    });

    let timer = unsafe {
        NSTimer::scheduledTimerWithTimeInterval_target_selector_userInfo_repeats(
            seconds,
            target,
            sel!(timerFired:),
            None,
            false,
        )
    };
    timer.setTolerance(tolerance);

    with_state(|state| {
        state.timer = Some(timer);
    });

    refresh_status_title();
    refresh_header_title();
    set_rest_now_enabled(phase == Phase::Working);
    refresh_menu_info();
}

/// 发送通知
fn notify(event: NotifyEvent, config: &Config, delegate: &RestGapDelegate) {
    match event {
        NotifyEvent::BreakStart => {
            // 休息开始: 弹出原生倒计时窗口
            show_countdown_window(delegate, config.break_seconds, true);
        }
        NotifyEvent::BreakEnd => {
            // 休息结束: 关闭倒计时窗口并显示通知
            finish_countdown();
        }
    }
}

/// 定时器触发时的阶段转换
pub fn transition_on_timer(delegate: &RestGapDelegate) {
    let transition = with_state(|state| {
        state.timer.take();
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
        notify(event, &config, delegate);
    }
    schedule_phase(delegate, next_phase);
}

/// 立即开始休息
pub fn start_break_now(delegate: &RestGapDelegate) {
    let (should_start, config) = with_state(|state| {
        let config = state.config.clone();
        let should_start = state.phase == Phase::Working;
        (should_start, config)
    });

    if should_start {
        notify(NotifyEvent::BreakStart, &config, delegate);
        schedule_phase(delegate, Phase::Breaking);
    }
}

/// 跳过当前休息（仅在 Breaking 阶段生效）
pub fn skip_break(delegate: &RestGapDelegate) {
    let should_skip = with_state(|state| state.phase == Phase::Breaking);
    if !should_skip {
        return;
    }

    // 提前结束休息：关闭倒计时窗口并回到工作阶段
    finish_countdown();
    schedule_phase(delegate, Phase::Working);
}
