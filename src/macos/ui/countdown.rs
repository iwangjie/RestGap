//! 倒计时窗口模块

use std::ptr;
use std::time::{Duration, Instant};

use block2::global_block;
use objc2::{MainThreadOnly, define_class, msg_send, sel};
use objc2_app_kit::{
    NSEvent, NSEventMask, NSApplication, NSBackingStoreType, NSColor, NSFont, NSScreen,
    NSStatusWindowLevel, NSTextField, NSWindow, NSWindowCollectionBehavior, NSWindowStyleMask,
};
use objc2_core_foundation::CGFloat;
use objc2_foundation::{NSPoint, NSRect, NSSize, NSString, NSTimer, NSObjectProtocol};

use super::super::constants::APP_NAME_ZH;
use super::super::delegate::RestGapDelegate;
use super::super::state::with_state;
use super::super::utils::{format_countdown, play_sound};
use super::status_bar::target_anyobject;

const SKIP_PHRASE_SMART: &str = "i don’t care about my health.";
const SKIP_PHRASE_ASCII: &str = "i don't care about my health.";

define_class!(
    #[unsafe(super(NSWindow))]
    #[thread_kind = MainThreadOnly]
    pub struct CountdownWindow;

    unsafe impl NSObjectProtocol for CountdownWindow {}

    impl CountdownWindow {
        #[unsafe(method(canBecomeKeyWindow))]
        fn can_become_key_window(&self) -> bool {
            true
        }

        #[unsafe(method(canBecomeMainWindow))]
        fn can_become_main_window(&self) -> bool {
            true
        }
    }
);

fn advance_phrase_idx(idx: &mut usize, phrase: &str, ch: char) -> bool {
    let mut buf = [0u8; 4];
    let ch_bytes = ch.encode_utf8(&mut buf).as_bytes();
    let phrase_bytes = phrase.as_bytes();

    if *idx + ch_bytes.len() <= phrase_bytes.len()
        && &phrase_bytes[*idx..(*idx + ch_bytes.len())] == ch_bytes
    {
        *idx += ch_bytes.len();
        return *idx == phrase_bytes.len();
    }

    // 朴素回退：短语以 'I' 开头且几乎不重叠，足够可靠。
    *idx = 0;
    if ch_bytes.len() <= phrase_bytes.len() && &phrase_bytes[..ch_bytes.len()] == ch_bytes {
        *idx = ch_bytes.len();
        return *idx == phrase_bytes.len();
    }
    false
}

fn on_countdown_keydown(event: &NSEvent) {
    let Some(text) = event
        .charactersIgnoringModifiers()
        .map(|s| s.to_string())
    else {
        return;
    };

    with_state(|state| {
        if state.countdown_window.is_none() {
            return;
        }
        if state.countdown_skip_requested {
            return;
        }

        for ch in text.chars() {
            let ch = if ch.is_ascii() {
                ch.to_ascii_lowercase()
            } else {
                ch
            };
            let smart_done =
                advance_phrase_idx(&mut state.countdown_skip_smart_idx, SKIP_PHRASE_SMART, ch);
            let ascii_done =
                advance_phrase_idx(&mut state.countdown_skip_ascii_idx, SKIP_PHRASE_ASCII, ch);
            if smart_done || ascii_done {
                state.countdown_skip_requested = true;
                state.countdown_skip_smart_idx = 0;
                state.countdown_skip_ascii_idx = 0;
                break;
            }
        }
    });
}

global_block! {
    static COUNTDOWN_KEY_MONITOR = |event: core::ptr::NonNull<NSEvent>| -> *mut NSEvent {
        let event_ref: &NSEvent = unsafe { event.as_ref() };
        on_countdown_keydown(event_ref);
        ptr::null_mut()
    };
}

/// 显示倒计时窗口
#[allow(clippy::too_many_lines)]
pub fn show_countdown_window(delegate: &RestGapDelegate, seconds: u64, play_start_sound: bool) {
    let mtm = delegate.mtm();

    // 关闭已存在的倒计时窗口
    close_countdown_window();

    // 播放开始声音
    if play_start_sound {
        play_sound("Glass");
    }

    // 获取主屏幕尺寸
    let screen_frame = NSScreen::mainScreen(mtm).map_or(
        NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(1920.0, 1080.0)),
        |s| s.frame(),
    );

    // 窗口占满屏幕
    let window_width: CGFloat = screen_frame.size.width;
    let window_height: CGFloat = screen_frame.size.height;
    let frame = screen_frame;

    // 创建窗口 - 使用 Borderless 样式隐藏标题栏
    let style = NSWindowStyleMask::Borderless;
    let window: objc2::rc::Retained<CountdownWindow> = unsafe {
        msg_send![CountdownWindow::alloc(mtm), initWithContentRect: frame styleMask: style backing: NSBackingStoreType::Buffered defer: false]
    };
    let window: objc2::rc::Retained<NSWindow> = window.into_super();

    // 设置窗口属性
    window.setLevel(NSStatusWindowLevel); // 使用状态窗口级别，确保在最前
    window.setMovable(false); // 不可移动/拖动

    // 设置窗口在所有工作空间可见，并且始终在最前
    window.setCollectionBehavior(
        NSWindowCollectionBehavior::CanJoinAllSpaces | NSWindowCollectionBehavior::Stationary,
    );

    // 设置背景色
    if let Some(content_view) = window.contentView() {
        content_view.setWantsLayer(true);
    }
    window.setBackgroundColor(Some(&NSColor::windowBackgroundColor()));

    // 标签布局：垂直居中排列
    let center_y = window_height / 2.0;
    let padding: CGFloat = 40.0;

    // 创建标题标签
    let title_frame = NSRect::new(
        NSPoint::new(padding, center_y + 60.0),
        NSSize::new(padding.mul_add(-2.0, window_width), 50.0),
    );
    let title_label = {
        let label = NSTextField::initWithFrame(NSTextField::alloc(mtm), title_frame);
        label.setStringValue(&NSString::from_str(&format!("{APP_NAME_ZH} · 休息倒计时")));
        label.setBezeled(false);
        label.setDrawsBackground(false);
        label.setEditable(false);
        label.setSelectable(false);
        label.setAlignment(objc2_app_kit::NSTextAlignment::Center);
        let font = NSFont::systemFontOfSize(36.0);
        label.setFont(Some(&font));
        label
    };

    // 创建倒计时标签
    let countdown_frame = NSRect::new(
        NSPoint::new(padding, center_y - 40.0),
        NSSize::new(padding.mul_add(-2.0, window_width), 100.0),
    );
    let countdown_label = {
        let label = NSTextField::initWithFrame(NSTextField::alloc(mtm), countdown_frame);
        label.setStringValue(&NSString::from_str(&format_countdown(seconds)));
        label.setBezeled(false);
        label.setDrawsBackground(false);
        label.setEditable(false);
        label.setSelectable(false);
        label.setAlignment(objc2_app_kit::NSTextAlignment::Center);
        // 使用 boldSystemFontOfSize 代替 monospacedDigitSystemFontOfSize_weight
        let font = NSFont::boldSystemFontOfSize(72.0);
        label.setFont(Some(&font));
        label
    };

    // 创建提示标签
    let hint_frame = NSRect::new(
        NSPoint::new(padding, center_y - 120.0),
        NSSize::new(padding.mul_add(-2.0, window_width), 40.0),
    );
    let hint_label = {
        let label = NSTextField::initWithFrame(NSTextField::alloc(mtm), hint_frame);
        label.setStringValue(&NSString::from_str("放松眼睛，伸展身体"));
        label.setBezeled(false);
        label.setDrawsBackground(false);
        label.setEditable(false);
        label.setSelectable(false);
        label.setAlignment(objc2_app_kit::NSTextAlignment::Center);
        let font = NSFont::systemFontOfSize(24.0);
        label.setFont(Some(&font));
        label.setTextColor(Some(&NSColor::secondaryLabelColor()));
        label
    };

    // 添加所有子视图
    if let Some(content_view) = window.contentView() {
        content_view.addSubview(&title_label);
        content_view.addSubview(&countdown_label);
        content_view.addSubview(&hint_label);
    }

    // 让倒计时窗口能获取键盘事件（Borderless 默认不可成为 key window）
    NSApplication::sharedApplication(mtm).activateIgnoringOtherApps(true);

    // 先标记窗口存在，避免用户刚弹窗就开始输入时被忽略
    with_state(|state| {
        state.countdown_window = Some(window.clone());
        state.countdown_skip_smart_idx = 0;
        state.countdown_skip_ascii_idx = 0;
        state.countdown_skip_requested = false;
    });

    // 显示窗口
    window.makeKeyAndOrderFront(None);

    // 仅在倒计时窗口存在时安装本地键盘监听器，避免非休息时任何额外开销。
    let key_monitor = unsafe {
        NSEvent::addLocalMonitorForEventsMatchingMask_handler(
            NSEventMask::KeyDown,
            &COUNTDOWN_KEY_MONITOR,
        )
    };

    // 设置结束时间
    let end_time = Instant::now() + Duration::from_secs(seconds);

    // 创建定时器每秒更新倒计时
    let target = target_anyobject(delegate);
    let timer = unsafe {
        NSTimer::scheduledTimerWithTimeInterval_target_selector_userInfo_repeats(
            1.0,
            target,
            sel!(countdownTick:),
            None,
            true,
        )
    };

    // 保存状态
    with_state(|state| {
        state.countdown_label = Some(countdown_label);
        state.countdown_timer = Some(timer);
        state.countdown_end_time = Some(end_time);
        state.countdown_key_monitor = key_monitor;
    });
}

/// 更新倒计时显示
pub fn update_countdown() -> bool {
    with_state(|state| {
        let Some(end_time) = state.countdown_end_time else {
            return false;
        };
        let Some(label) = state.countdown_label.as_ref() else {
            return false;
        };

        let now = Instant::now();
        if now >= end_time {
            // 倒计时结束
            return false;
        }

        let remaining = end_time.duration_since(now);
        let secs = remaining.as_secs();
        let text = format_countdown(secs);
        label.setStringValue(&NSString::from_str(&text));
        true
    })
}

/// 关闭倒计时窗口
pub fn close_countdown_window() {
    with_state(|state| {
        if let Some(monitor) = state.countdown_key_monitor.take() {
            // Safety: The monitor handle is created by NSEvent::addLocalMonitor...
            unsafe { NSEvent::removeMonitor(&monitor) };
        }

        // 先使定时器无效，防止后续触发
        if let Some(timer) = state.countdown_timer.take() {
            timer.invalidate();
        }
        // 使用 orderOut 而不是 close，避免触发窗口关闭事件导致应用退出
        if let Some(window) = state.countdown_window.take() {
            window.orderOut(None);
        }
        state.countdown_label = None;
        state.countdown_end_time = None;
        state.countdown_skip_smart_idx = 0;
        state.countdown_skip_ascii_idx = 0;
        state.countdown_skip_requested = false;
    });
}

/// 完成倒计时
pub fn finish_countdown() {
    close_countdown_window();
    play_sound("Tink");
}
