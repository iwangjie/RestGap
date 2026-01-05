//! 应用委托模块
//!
//! 定义 `NSApplicationDelegate` 实现。

use objc2::rc::Retained;
use objc2::runtime::{AnyObject, NSObject};
use objc2::{MainThreadMarker, MainThreadOnly, define_class, msg_send};

use objc2_app_kit::{NSApplication, NSApplicationDelegate, NSMenu, NSMenuDelegate};
use objc2_foundation::{NSNotification, NSObjectProtocol, NSTimer};

use super::state::{Phase, with_state, with_state_ref};
use super::timer::{schedule_phase, start_break_now, transition_on_timer};
use super::ui::{
    close_countdown_window, finish_countdown, open_settings_dialog, refresh_header_title,
    refresh_menu_info, refresh_status_title, set_rest_now_enabled, setup_status_item,
    show_about_dialog, update_countdown,
};
use super::utils::play_sound;

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    pub struct RestGapDelegate;

    unsafe impl NSObjectProtocol for RestGapDelegate {}
    unsafe impl NSApplicationDelegate for RestGapDelegate {}
    unsafe impl NSMenuDelegate for RestGapDelegate {}

    impl RestGapDelegate {
        #[unsafe(method(applicationDidFinishLaunching:))]
        fn application_did_finish_launching(&self, _notification: &NSNotification) {
            setup_status_item(self);
            schedule_phase(self, Phase::Working);
        }

        #[unsafe(method(applicationShouldTerminateAfterLastWindowClosed:))]
        fn application_should_terminate_after_last_window_closed(
            &self,
            _app: &NSApplication,
        ) -> bool {
            // 倒计时窗口关闭后不要退出应用
            false
        }

        #[unsafe(method(timerFired:))]
        fn timer_fired(&self, _timer: &NSTimer) {
            transition_on_timer(self);
        }

        #[unsafe(method(menuWillOpen:))]
        fn menu_will_open(&self, _menu: &NSMenu) {
            // 无轮询：仅在用户打开菜单时刷新显示。
            refresh_status_title();
            refresh_header_title();
            set_rest_now_enabled(with_state_ref(|s| s.phase == Phase::Working));
            refresh_menu_info();
        }

        #[unsafe(method(restNow:))]
        fn rest_now(&self, _sender: Option<&AnyObject>) {
            start_break_now(self);
        }

        #[unsafe(method(openSettings:))]
        fn open_settings(&self, _sender: Option<&AnyObject>) {
            open_settings_dialog(self);
        }

        #[unsafe(method(about:))]
        fn about(&self, _sender: Option<&AnyObject>) {
            show_about_dialog(self);
        }

        #[unsafe(method(quit:))]
        fn quit(&self, _sender: Option<&AnyObject>) {
            with_state(|state| {
                if let Some(timer) = state.timer.take() {
                    timer.invalidate();
                }
            });

            if let Some(mtm) = MainThreadMarker::new() {
                let app = NSApplication::sharedApplication(mtm);
                app.terminate(None);
            }
        }

        #[unsafe(method(countdownTick:))]
        fn countdown_tick(&self, _timer: &NSTimer) {
            // 检查窗口是否仍然存在，防止重复调用
            let window_exists = with_state_ref(|state| state.countdown_window.is_some());
            if !window_exists {
                return;
            }

            if !update_countdown() {
                // 倒计时结束
                finish_countdown();
            }
        }

        #[unsafe(method(skipBreak:))]
        fn skip_break(&self, _sender: Option<&AnyObject>) {
            // 用户点击跳过休息按钮
            close_countdown_window();
            play_sound("Tink");
        }
    }
);

/// 创建并返回 delegate 实例
pub fn create_delegate(mtm: MainThreadMarker) -> Retained<RestGapDelegate> {
    unsafe { msg_send![RestGapDelegate::alloc(mtm), init] }
}
