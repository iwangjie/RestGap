//! `RestGap` (息间) - 跨平台休息提醒应用
//!
//! 支持 macOS、Windows 和 Linux 的休息提醒应用，使用事件驱动架构而非轮询，
//! 追求极低的 CPU 和内存占用。

// Windows: 隐藏控制台窗口，使用纯 GUI 模式
#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

#[cfg(any(target_os = "windows", target_os = "linux"))]
mod common;

pub(crate) mod i18n;

#[cfg(target_os = "macos")]
mod macos;

#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "linux")]
mod linux;

#[cfg(target_os = "macos")]
fn main() {
    macos::run();
}

#[cfg(target_os = "windows")]
fn main() {
    windows::run();
}

#[cfg(target_os = "linux")]
fn main() {
    linux::run();
}

#[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
fn main() {
    eprintln!("息间（RestGap）仅支持 macOS、Windows 和 Linux。");
    eprintln!("RestGap only supports macOS, Windows, and Linux.");
}
