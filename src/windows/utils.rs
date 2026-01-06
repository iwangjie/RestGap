//! 工具函数模块

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use windows::Win32::Media::Audio::{PlaySoundW, SND_ALIAS, SND_ASYNC};
use windows::core::PCWSTR;

/// 格式化时间为 HH:MM 格式
pub fn format_hhmm(t: SystemTime) -> String {
    let Ok(duration) = t.duration_since(UNIX_EPOCH) else {
        return "--:--".to_string();
    };

    // 简单实现：使用 UTC 时间加上本地时区偏移
    // Windows 上可以使用更精确的 API，但这里保持简单
    let secs = duration.as_secs();
    // 获取本地时间（简化处理，假设 UTC+8）
    // 实际应用中应该使用 Windows API 获取本地时间
    let local_secs = secs + 8 * 3600; // UTC+8
    let hours = (local_secs / 3600) % 24;
    let minutes = (local_secs / 60) % 60;
    format!("{hours:02}:{minutes:02}")
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
pub fn play_sound(sound_type: SoundType) {
    let sound_name = match sound_type {
        SoundType::BreakStart => "SystemAsterisk",
        SoundType::BreakEnd => "SystemExclamation",
    };

    let wide: Vec<u16> = sound_name
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();
    unsafe {
        let _ = PlaySoundW(PCWSTR(wide.as_ptr()), None, SND_ALIAS | SND_ASYNC);
    }
}

/// 声音类型
#[derive(Clone, Copy, Debug)]
pub enum SoundType {
    BreakStart,
    BreakEnd,
}

/// 将 Rust 字符串转换为宽字符串（以 null 结尾）
pub fn to_wide_string(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// 将 Rust 字符串转换为固定长度的宽字符数组
pub fn to_wide_array<const N: usize>(s: &str) -> [u16; N] {
    let mut arr = [0u16; N];
    for (i, c) in s.encode_utf16().take(N - 1).enumerate() {
        arr[i] = c;
    }
    arr
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
