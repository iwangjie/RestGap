//! 低功耗闲置跳过判定。
//!
//! 仅在工作阶段结束时查询一次系统空闲时长，不做持续轮询。

use std::time::Duration;

const MAX_ALLOWED_ACTIVE_TIME: Duration = Duration::from_secs(8);

pub fn should_skip_break(cycle_elapsed: Duration) -> bool {
    let Some(idle_duration) = current_idle_duration() else {
        return false;
    };
    should_skip_break_with_idle(cycle_elapsed, idle_duration)
}

fn should_skip_break_with_idle(cycle_elapsed: Duration, idle_duration: Duration) -> bool {
    if cycle_elapsed <= MAX_ALLOWED_ACTIVE_TIME {
        return false;
    }

    idle_duration + MAX_ALLOWED_ACTIVE_TIME >= cycle_elapsed
}

#[cfg(target_os = "macos")]
#[allow(unsafe_code)]
fn current_idle_duration() -> Option<Duration> {
    use std::time::Duration;

    type CGEventSourceStateID = u32;
    type CGEventType = u32;

    const K_CG_EVENT_SOURCE_STATE_COMBINED_SESSION_STATE: CGEventSourceStateID = 0;
    const K_CG_ANY_INPUT_EVENT_TYPE: CGEventType = !0;

    #[link(name = "ApplicationServices", kind = "framework")]
    unsafe extern "C" {
        fn CGEventSourceSecondsSinceLastEventType(
            source: CGEventSourceStateID,
            event_type: CGEventType,
        ) -> f64;
    }

    let seconds = unsafe {
        CGEventSourceSecondsSinceLastEventType(
            K_CG_EVENT_SOURCE_STATE_COMBINED_SESSION_STATE,
            K_CG_ANY_INPUT_EVENT_TYPE,
        )
    };

    if !seconds.is_finite() || seconds.is_sign_negative() {
        return None;
    }

    Some(Duration::from_secs_f64(seconds))
}

#[cfg(target_os = "windows")]
#[allow(unsafe_code)]
fn current_idle_duration() -> Option<Duration> {
    use windows::Win32::System::SystemInformation::GetTickCount;
    use windows::Win32::UI::Input::KeyboardAndMouse::{GetLastInputInfo, LASTINPUTINFO};

    let mut last_input = LASTINPUTINFO {
        cbSize: u32::try_from(std::mem::size_of::<LASTINPUTINFO>()).ok()?,
        ..Default::default()
    };

    if !unsafe { GetLastInputInfo(&raw mut last_input) }.as_bool() {
        return None;
    }

    let now = unsafe { GetTickCount() };
    let idle_ms = now.wrapping_sub(last_input.dwTime);
    Some(Duration::from_millis(u64::from(idle_ms)))
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
const fn current_idle_duration() -> Option<Duration> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skips_only_when_idle_almost_covers_cycle() {
        let cycle = Duration::from_secs(30 * 60);
        let almost_all_idle = cycle.checked_sub(Duration::from_secs(5)).unwrap();
        assert!(should_skip_break_with_idle(cycle, almost_all_idle));
        assert!(!should_skip_break_with_idle(
            cycle,
            cycle.checked_sub(Duration::from_secs(20)).unwrap()
        ));
    }
}
