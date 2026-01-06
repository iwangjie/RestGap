//! 应用状态管理模块
//!
//! 使用线程本地存储管理全局应用状态。

use std::cell::RefCell;
use std::time::{Instant, SystemTime};

use windows::Win32::Foundation::HWND;
use windows::Win32::UI::Shell::NOTIFYICONDATAW;
use windows::Win32::UI::WindowsAndMessaging::HMENU;

use crate::common::Config;

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
pub struct AppState {
    pub config: Config,
    pub phase: Phase,
    pub phase_deadline_mono: Option<Instant>,
    pub phase_deadline_wall: Option<SystemTime>,
    // Windows handles
    pub main_hwnd: Option<HWND>,
    pub tray_menu: Option<HMENU>,
    pub tray_icon_data: Option<NOTIFYICONDATAW>,
    pub phase_timer_id: Option<usize>,
    // Countdown window state
    pub countdown_hwnd: Option<HWND>,
    pub countdown_timer_id: Option<usize>,
    pub countdown_end_time: Option<Instant>,
}

impl AppState {
    pub const fn new(config: Config) -> Self {
        Self {
            config,
            phase: Phase::Working,
            phase_deadline_mono: None,
            phase_deadline_wall: None,
            main_hwnd: None,
            tray_menu: None,
            tray_icon_data: None,
            phase_timer_id: None,
            countdown_hwnd: None,
            countdown_timer_id: None,
            countdown_end_time: None,
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
