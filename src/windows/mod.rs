//! Windows 平台模块
//!
//! 包含所有 Windows 特定的实现。
//! 使用 windows-rs 直接调用 Windows API，实现原生 GUI 应用。

#![allow(unsafe_code)] // Windows API 调用需要 unsafe

pub mod constants;
pub mod state;
pub mod timer;
pub mod ui;
pub mod utils;
pub mod wndproc;

use windows::Win32::Foundation::{HINSTANCE, HWND};
use windows::Win32::Graphics::Gdi::{GetStockObject, HBRUSH, WHITE_BRUSH};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DispatchMessageW, GetMessageW, MSG, RegisterClassW, TranslateMessage,
    WNDCLASSW, WS_EX_TOOLWINDOW, WS_OVERLAPPEDWINDOW,
};
use windows::core::PCWSTR;

use crate::common::Config;
use constants::MAIN_WINDOW_CLASS;
use state::init_state;
use ui::countdown::register_countdown_class;
use utils::to_wide_string;
use wndproc::main_wndproc;

/// 运行应用
pub fn run() {
    // 加载配置
    let config = Config::load();
    init_state(config);

    // 获取模块句柄
    let hmodule = unsafe { GetModuleHandleW(None) }.expect("Failed to get module handle");
    let hinstance = HINSTANCE(hmodule.0);

    // 注册主窗口类
    let class_name = to_wide_string(MAIN_WINDOW_CLASS);
    let wc = WNDCLASSW {
        lpfnWndProc: Some(main_wndproc),
        hInstance: hinstance,
        lpszClassName: PCWSTR(class_name.as_ptr()),
        hbrBackground: unsafe { HBRUSH(GetStockObject(WHITE_BRUSH).0) },
        ..Default::default()
    };

    let atom = unsafe { RegisterClassW(&wc) };
    if atom == 0 {
        panic!("Failed to register main window class");
    }

    // 注册倒计时窗口类
    if !register_countdown_class() {
        panic!("Failed to register countdown window class");
    }

    // 创建隐藏的主窗口（用于接收消息）
    let hwnd = unsafe {
        CreateWindowExW(
            WS_EX_TOOLWINDOW, // 不显示在任务栏
            PCWSTR(class_name.as_ptr()),
            PCWSTR::null(),
            WS_OVERLAPPEDWINDOW, // 不显示窗口
            0,
            0,
            0,
            0,
            None,
            None,
            Some(hinstance),
            None,
        )
    };

    let Ok(_hwnd) = hwnd else {
        panic!("Failed to create main window");
    };

    // 消息循环
    let mut msg = MSG::default();
    unsafe {
        while GetMessageW(&mut msg, HWND::default(), 0, 0).as_bool() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}
