//! `RestGap` (息间) - macOS 休息提醒应用
//!
//! 使用原生 AppKit / WebKit 构建，仅支持 macOS，
//! 采用事件驱动架构而非轮询，追求极低的 CPU 和内存占用。

pub(crate) mod i18n;
pub(crate) mod idle;
pub(crate) mod skip_challenge;

mod macos;
fn main() {
    macos::run();
}

#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("息间（RestGap）当前仅支持 macOS。");
}
