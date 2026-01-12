//! Linux platform implementation
//!
//! This is a basic implementation that provides the core functionality
//! for Linux using cross-platform libraries.
//!
//! Note: The timer logic is intentionally duplicated between Windows and Linux
//! implementations to keep platform-specific code isolated. This makes it easier
//! to add platform-specific features in the future (e.g., system tray on Linux,
//! desktop notifications) without affecting the other platforms.

use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use crate::common::Config;
use crate::i18n::Language;

/// Application state for Linux
struct AppState {
    config: Config,
    work_start: Instant,
    is_breaking: bool,
    break_start: Option<Instant>,
}

impl AppState {
    fn new(config: Config) -> Self {
        Self {
            config,
            work_start: Instant::now(),
            is_breaking: false,
            break_start: None,
        }
    }

    fn time_until_break(&self) -> Duration {
        let elapsed = self.work_start.elapsed();
        let work_duration = Duration::from_secs(self.config.interval_minutes * 60);
        work_duration.saturating_sub(elapsed)
    }

    fn break_time_remaining(&self) -> Duration {
        self.break_start.map_or(Duration::ZERO, |break_start| {
            let elapsed = break_start.elapsed();
            let break_duration = Duration::from_secs(self.config.break_seconds);
            break_duration.saturating_sub(elapsed)
        })
    }
}

/// Run the Linux application
pub fn run() {
    let config = Config::load();
    let lang = config.effective_language();

    match lang {
        Language::Zh => {
            println!("息间（RestGap）- Linux 版本");
            println!("==============================");
            println!("配置已加载：");
            println!("  工作间隔：{} 分钟", config.interval_minutes);
            println!("  休息时长：{} 秒", config.break_seconds);
            println!();
            println!("应用已启动，按 Ctrl+C 退出");
            println!();
        }
        Language::En => {
            println!("RestGap - Linux Version");
            println!("==============================");
            println!("Configuration loaded:");
            println!("  Work interval: {} minutes", config.interval_minutes);
            println!("  Break duration: {} seconds", config.break_seconds);
            println!();
            println!("Application started, press Ctrl+C to exit");
            println!();
        }
    }

    let state = Arc::new(Mutex::new(AppState::new(config)));

    // Main loop to manage work/break cycles
    loop {
        thread::sleep(Duration::from_secs(1));

        let mut state = state.lock().expect("Failed to lock state mutex");

        if state.is_breaking {
            let remaining = state.break_time_remaining();
            if remaining == Duration::ZERO {
                // Break is over
                state.is_breaking = false;
                state.break_start = None;
                state.work_start = Instant::now();
                match lang {
                    Language::Zh => println!("\n休息结束，开始工作！\n"),
                    Language::En => println!("\nBreak over, back to work!\n"),
                }
            } else if remaining.as_secs() % 10 == 0 {
                // Print countdown every 10 seconds during break
                let secs = remaining.as_secs();
                match lang {
                    Language::Zh => println!("休息倒计时：{secs} 秒"),
                    Language::En => println!("Break countdown: {secs} seconds"),
                }
            }
        } else {
            let remaining = state.time_until_break();
            if remaining == Duration::ZERO {
                // Time for a break!
                state.is_breaking = true;
                state.break_start = Some(Instant::now());
                let break_secs = state.config.break_seconds;
                match lang {
                    Language::Zh => println!("\n休息时间！请休息 {break_secs} 秒\n"),
                    Language::En => {
                        println!("\nBreak time! Please rest for {break_secs} seconds\n");
                    }
                }

                // In a full implementation, this would show a fullscreen window
                // For now, we just print to console
            } else if remaining.as_secs() % 60 == 0 && remaining.as_secs() > 0 {
                // Print update every minute
                let minutes = remaining.as_secs() / 60;
                match lang {
                    Language::Zh => println!("距离下次休息还有 {minutes} 分钟"),
                    Language::En => println!("{minutes} minutes until next break"),
                }
            }
        }
        drop(state);
    }
}
