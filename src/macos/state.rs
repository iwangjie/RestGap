//! 应用状态管理模块
//!
//! 使用线程本地存储管理全局应用状态。

use std::cell::RefCell;
use std::time::{Instant, SystemTime};

use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2_app_kit::{NSMenuItem, NSStatusItem, NSTextField, NSWindow};
use objc2_foundation::NSTimer;

use super::config::Config;

/// 工作阶段
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Working,
    Breaking,
}

/// 通知事件类型
#[derive(Clone, Copy, Debug)]
pub enum NotifyEvent {
    BreakStart,
    BreakEnd,
}

/// 应用状态
#[derive(Debug)]
pub struct AppState {
    pub config: Config,
    pub phase: Phase,
    pub phase_deadline_mono: Option<Instant>,
    pub phase_deadline_wall: Option<SystemTime>,
    pub timer: Option<Retained<NSTimer>>,
    pub status_item: Option<Retained<NSStatusItem>>,
    pub header_item: Option<Retained<NSMenuItem>>,
    pub rest_now_item: Option<Retained<NSMenuItem>>,
    pub next_break_item: Option<Retained<NSMenuItem>>,
    pub remaining_break_item: Option<Retained<NSMenuItem>>,
    pub settings_item: Option<Retained<NSMenuItem>>,
    pub language_auto_item: Option<Retained<NSMenuItem>>,
    pub language_en_item: Option<Retained<NSMenuItem>>,
    pub language_zh_item: Option<Retained<NSMenuItem>>,
    pub about_item: Option<Retained<NSMenuItem>>,
    pub quit_item: Option<Retained<NSMenuItem>>,
    // Countdown window state
    pub countdown_window: Option<Retained<NSWindow>>,
    pub countdown_label: Option<Retained<NSTextField>>,
    pub countdown_timer: Option<Retained<NSTimer>>,
    pub countdown_end_time: Option<Instant>,
    // Hidden skip phrase state (only used during breaks)
    pub countdown_key_monitor: Option<Retained<AnyObject>>,
    pub countdown_skip_smart_idx: usize,
    pub countdown_skip_ascii_idx: usize,
    pub countdown_skip_requested: bool,
}

impl AppState {
    pub const fn new(config: Config) -> Self {
        Self {
            config,
            phase: Phase::Working,
            phase_deadline_mono: None,
            phase_deadline_wall: None,
            timer: None,
            status_item: None,
            header_item: None,
            rest_now_item: None,
            next_break_item: None,
            remaining_break_item: None,
            settings_item: None,
            language_auto_item: None,
            language_en_item: None,
            language_zh_item: None,
            about_item: None,
            quit_item: None,
            countdown_window: None,
            countdown_label: None,
            countdown_timer: None,
            countdown_end_time: None,
            countdown_key_monitor: None,
            countdown_skip_smart_idx: 0,
            countdown_skip_ascii_idx: 0,
            countdown_skip_requested: false,
        }
    }
}

thread_local! {
    static STATE: RefCell<Option<AppState>> = const { RefCell::new(None) };
}

/// 初始化全局状态
pub fn init_state(config: Config) {
    STATE.with(|cell| {
        *cell.borrow_mut() = Some(AppState::new(config));
    });
}

/// 可变访问全局状态
pub fn with_state<R>(f: impl FnOnce(&mut AppState) -> R) -> R {
    STATE.with(|cell| {
        let mut state = cell.borrow_mut();
        let state = state.as_mut().expect("STATE not initialized");
        f(state)
    })
}

/// 只读访问全局状态
pub fn with_state_ref<R>(f: impl FnOnce(&AppState) -> R) -> R {
    STATE.with(|cell| {
        let state = cell.borrow();
        let state = state.as_ref().expect("STATE not initialized");
        f(state)
    })
}
