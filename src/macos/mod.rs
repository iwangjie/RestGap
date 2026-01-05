//! macOS 平台模块
//!
//! 包含所有 macOS 特定的实现。

#![allow(deprecated)]
#![allow(unsafe_code)] // macOS API 调用需要 unsafe

pub mod config;
pub mod constants;
pub mod delegate;
pub mod error;
pub mod log;
pub mod state;
pub mod timer;
pub mod ui;
pub mod utils;

use objc2::rc::autoreleasepool;
use objc2::runtime::ProtocolObject;
use objc2::MainThreadMarker;

use objc2_app_kit::{NSApplication, NSApplicationActivationPolicy, NSApplicationDelegate};

use config::Config;
use delegate::create_delegate;
use state::init_state;

/// 运行应用
pub fn run() {
    autoreleasepool(|_| {
        let config = Config::load();
        init_state(config);

        let mtm = MainThreadMarker::new().expect("must be on the main thread");
        let app = NSApplication::sharedApplication(mtm);

        // 只显示菜单栏图标，不显示 Dock 图标
        let _ = app.setActivationPolicy(NSApplicationActivationPolicy::Accessory);

        let delegate = create_delegate(mtm);
        let delegate_ref: &delegate::RestGapDelegate = &*delegate;
        app.setDelegate(Some(ProtocolObject::<dyn NSApplicationDelegate>::from_ref(
            delegate_ref,
        )));

        // NSApplication 的 delegate 是 weak 引用；这里必须持有它直到 app 退出。
        let _keep_delegate_alive = delegate;

        app.run();
    });
}
