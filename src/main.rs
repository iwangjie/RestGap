//! `RestGap` (息间) - macOS 菜单栏休息提醒应用
//!
//! 纯 Rust 实现的 macOS 菜单栏休息提醒应用，使用事件驱动架构而非轮询，
//! 追求极低的 CPU 和内存占用。

#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("息间（RestGap）仅支持 macOS。");
}

#[cfg(target_os = "macos")]
mod macos;

#[cfg(target_os = "macos")]
fn main() {
    macos::run();
}
