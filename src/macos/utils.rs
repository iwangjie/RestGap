//! 工具函数模块

use std::process::Command;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// 格式化时间为 HH:MM 格式
pub fn format_hhmm(t: SystemTime) -> String {
    let Ok(duration) = t.duration_since(UNIX_EPOCH) else {
        return "--:--".to_string();
    };

    let mut tm: libc::tm = unsafe { std::mem::zeroed() };
    let seconds: libc::time_t = libc::time_t::try_from(duration.as_secs()).unwrap_or_default();
    let tm_ptr =
        unsafe { libc::localtime_r(std::ptr::addr_of!(seconds), std::ptr::addr_of_mut!(tm)) };
    if tm_ptr.is_null() {
        return "--:--".to_string();
    }

    let mut buf = [0u8; 6]; // "HH:MM\0"
    let fmt = b"%H:%M\0";
    let written = unsafe {
        libc::strftime(
            buf.as_mut_ptr().cast(),
            buf.len(),
            fmt.as_ptr().cast(),
            std::ptr::addr_of!(tm),
        )
    };
    if written == 0 {
        return "--:--".to_string();
    }
    String::from_utf8_lossy(&buf[..written]).into_owned()
}

/// 格式化时长为近似字符串
pub fn approx_duration(d: Duration) -> String {
    let secs = d.as_secs();
    if secs >= 3600 {
        let hours = secs / 3600;
        let minutes = (secs % 3600) / 60;
        return format!("≈{hours}h{minutes}m");
    }
    if secs >= 600 {
        // >= 10m, round to 5m
        let minutes = ((secs + 150) / 300) * 5;
        return format!("≈{minutes}m");
    }
    if secs >= 120 {
        // >= 2m, round to 1m
        let minutes = (secs + 30) / 60;
        return format!("≈{minutes}m");
    }
    if secs >= 60 {
        return "≈1m".to_string();
    }
    // < 60s, round to 10s
    let rounded = ((secs + 5) / 10) * 10;
    format!("≈{}s", rounded.max(10))
}

/// 格式化倒计时为 MM:SS 格式
pub fn format_countdown(seconds: u64) -> String {
    let mins = seconds / 60;
    let secs = seconds % 60;
    format!("{mins:02}:{secs:02}")
}

/// 播放系统声音
pub fn play_sound(sound_name: &str) {
    let path = format!("/System/Library/Sounds/{sound_name}.aiff");
    let _ = Command::new("afplay").arg(&path).spawn();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_approx_duration_hours() {
        assert_eq!(approx_duration(Duration::from_secs(3700)), "≈1h1m");
        assert_eq!(approx_duration(Duration::from_secs(7200)), "≈2h0m");
    }

    #[test]
    fn test_approx_duration_minutes() {
        assert_eq!(approx_duration(Duration::from_secs(600)), "≈10m");
        assert_eq!(approx_duration(Duration::from_secs(900)), "≈15m");
    }

    #[test]
    fn test_approx_duration_seconds() {
        assert_eq!(approx_duration(Duration::from_secs(45)), "≈50s");
        assert_eq!(approx_duration(Duration::from_secs(5)), "≈10s");
    }

    #[test]
    fn test_format_countdown() {
        assert_eq!(format_countdown(0), "00:00");
        assert_eq!(format_countdown(65), "01:05");
        assert_eq!(format_countdown(3661), "61:01");
    }
}
