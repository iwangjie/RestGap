//! 倒计时窗口模块

use std::ptr;
use std::time::{Duration, Instant};

use block2::global_block;
use objc2::{MainThreadOnly, define_class, msg_send, sel};
use objc2_app_kit::{
    NSApplication, NSBackingStoreType, NSColor, NSEvent, NSEventMask, NSScreen,
    NSStatusWindowLevel, NSWindow, NSWindowCollectionBehavior, NSWindowStyleMask,
};
use objc2_foundation::{NSObjectProtocol, NSPoint, NSRect, NSSize, NSString, NSTimer};
use objc2_web_kit::WKWebView;

use super::super::delegate::RestGapDelegate;
use super::super::state::with_state;
use super::super::utils::{format_countdown, play_sound};
use super::status_bar::target_anyobject;
use crate::i18n::Texts;

const SKIP_PHRASE_SMART: &str = "i don’t care about my health.";
const SKIP_PHRASE_ASCII: &str = "i don't care about my health.";

const KEGEL_HTML_TEMPLATE: &str = r#"<!DOCTYPE html>
<html lang="zh-CN">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Aligned Kegel Guide</title>
    <style>
        :root {
            --card-bg: #f2f0e9;
            --text-main: #333333;
            --text-sub: #757575;
            --font-serif: 'Times New Roman', Times, serif;
            --font-sans: 'Helvetica Neue', Helvetica, Arial, sans-serif;
            --anim-duration: 4s;
        }

        * {
            box-sizing: border-box;
        }

        body {
            margin: 0;
            padding: 0;
            height: 100vh;
            background-color: #000;
            display: flex;
            justify-content: center;
            align-items: center;
            user-select: none;
        }

        .screen {
            width: 100%;
            height: 100%;
            display: flex;
            flex-direction: column;
            align-items: center;
            justify-content: center;
            gap: clamp(16px, 3vh, 32px);
            padding: clamp(24px, 5vh, 64px) 24px;
        }

        .title {
            font-family: var(--font-sans);
            font-size: clamp(18px, 2.4vw, 28px);
            color: #e8e5dc;
            text-align: center;
        }

        .countdown {
            font-family: var(--font-sans);
            font-size: clamp(48px, 7vw, 96px);
            font-weight: 700;
            font-variant-numeric: tabular-nums;
            color: #f2f0e9;
            line-height: 1;
        }

        .card {
            background-color: var(--card-bg);
            width: min(90vw, 600px, calc(70vh * 4 / 3.2));
            aspect-ratio: 4 / 3.2;
            border-radius: 30px;
            display: flex;
            flex-direction: column;
            justify-content: center;
            align-items: center;
            position: relative;
            box-shadow: 0 20px 50px rgba(0, 0, 0, 0.5);
        }

        .symbol-area {
            flex: 1;
            display: flex;
            align-items: center;
            justify-content: center;
            width: 100%;
            padding-top: 40px;
        }

        .symbol-text {
            font-family: var(--font-serif);
            font-size: clamp(120px, 18vw, 176px);
            line-height: 1;
            color: var(--text-main);
            display: flex;
            align-items: center;
        }

        .bracket {
            display: inline-block;
            font-weight: 300;
            animation: bracket-move var(--anim-duration) cubic-bezier(0.45, 0, 0.55, 1) infinite;
        }

        .star {
            display: inline-block;
            margin: 0 20px;
            font-weight: 400;
            transform: translateY(25px) scale(1);
            transform-origin: center calc(50% + 25px);
            animation: star-breathe var(--anim-duration) cubic-bezier(0.45, 0, 0.55, 1) infinite;
        }

        .text-area {
            height: 120px;
            display: flex;
            flex-direction: column;
            justify-content: flex-start;
            align-items: center;
            padding-bottom: 30px;
        }

        .status-group {
            display: flex;
            flex-direction: column;
            align-items: center;
            text-align: center;
            font-family: var(--font-sans);
            font-size: 0.8rem;
            letter-spacing: 0.2em;
            font-weight: 500;
            color: var(--text-sub);
            position: absolute;
            transition: opacity 0.5s;
        }

        .slash {
            margin: 4px 0;
            opacity: 0.5;
            font-weight: 300;
        }

        .hint {
            font-family: var(--font-sans);
            font-size: clamp(14px, 2vw, 22px);
            color: #b5b5b5;
            text-align: center;
            max-width: 80vw;
        }

        @keyframes bracket-move {
            0%, 100% { transform: translateX(0); }
            40%, 70% { transform: translateX(var(--dir)); }
        }
        .bracket.left { --dir: 40px; }
        .bracket.right { --dir: -40px; }

        @keyframes star-breathe {
            0%, 100% {
                transform: translateY(25px) scale(1.1);
                opacity: 0.8;
            }
            40%, 70% {
                transform: translateY(25px) scale(0.65);
                opacity: 1;
                color: #111;
            }
        }

        .status-group.relax { animation: fade-relax var(--anim-duration) infinite; }
        .status-group.tight { animation: fade-tight var(--anim-duration) infinite; }

        @keyframes fade-relax {
            0%, 20%, 90%, 100% { opacity: 1; filter: blur(0); }
            30%, 80% { opacity: 0; filter: blur(4px); }
        }

        @keyframes fade-tight {
            0%, 25%, 85%, 100% { opacity: 0; filter: blur(4px); }
            35%, 75% { opacity: 1; filter: blur(0); }
        }
    </style>
</head>
<body>
<div class="screen">
    <div class="title" id="title">__TITLE__</div>
    <div class="countdown" id="countdown">__COUNTDOWN__</div>
    <div class="card">
        <div class="symbol-area">
            <div class="symbol-text">
                <span class="bracket left">{</span>
                <span class="star">*</span>
                <span class="bracket right">}</span>
            </div>
        </div>

        <div class="text-area">
            <div class="status-group relax">
                <span>RELAX</span>
                <span class="slash">/</span>
                <span>INHALE</span>
            </div>

            <div class="status-group tight">
                <span>TIGHTEN</span>
                <span class="slash">/</span>
                <span>HOLD</span>
            </div>
        </div>
    </div>
    <div class="hint" id="hint">__HINT__</div>
</div>
<script>
    window.setCountdown = (value) => {
        const el = document.getElementById('countdown');
        if (el) {
            el.textContent = value;
        }
    };
    window.setTitle = (value) => {
        const el = document.getElementById('title');
        if (el) {
            el.textContent = value;
        }
    };
    window.setHint = (value) => {
        const el = document.getElementById('hint');
        if (el) {
            el.textContent = value;
        }
    };
</script>
</body>
</html>
"#;

fn escape_html(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&#39;"),
            _ => escaped.push(ch),
        }
    }
    escaped
}

fn build_kegel_html(title: &str, countdown: &str, hint: &str) -> String {
    let mut html = KEGEL_HTML_TEMPLATE.replace("__TITLE__", &escape_html(title));
    html = html.replace("__COUNTDOWN__", &escape_html(countdown));
    html.replace("__HINT__", &escape_html(hint))
}

fn update_kegel_countdown(webview: &WKWebView, text: &str) {
    let js_value = serde_json::to_string(text).unwrap_or_else(|_| "\"\"".to_string());
    let script = format!("window.setCountdown({});", js_value);
    let script = NSString::from_str(&script);
    unsafe {
        webview.evaluateJavaScript_completionHandler(&script, None);
    }
}

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
    let Some(text) = event.charactersIgnoringModifiers().map(|s| s.to_string()) else {
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
    let texts = Texts::new(with_state(|state| state.config.effective_language()));

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
    let frame = screen_frame;

    // 创建窗口 - 使用 Borderless 样式隐藏标题栏
    let style = NSWindowStyleMask::Borderless;
    let window: objc2::rc::Retained<CountdownWindow> = unsafe {
        msg_send![
            CountdownWindow::alloc(mtm),
            initWithContentRect: frame
            styleMask: style
            backing: NSBackingStoreType::Buffered
            defer: false
        ]
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
    window.setBackgroundColor(Some(&NSColor::blackColor()));

    let html = build_kegel_html(
        &texts.countdown_title(),
        &format_countdown(seconds),
        &texts.countdown_hint(),
    );
    let view_frame = NSRect::new(NSPoint::new(0.0, 0.0), frame.size);
    let webview = unsafe { WKWebView::initWithFrame(WKWebView::alloc(mtm), view_frame) };
    let html = NSString::from_str(&html);
    unsafe {
        let _ = webview.loadHTMLString_baseURL(&html, None);
    }
    if let Some(content_view) = window.contentView() {
        content_view.addSubview(&webview);
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

    // 仅在倒计时窗口存在时安装本地键盘监听器，
    // 避免非休息时任何额外开销。
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
        state.countdown_webview = Some(webview);
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

        let now = Instant::now();
        if now >= end_time {
            // 倒计时结束
            return false;
        }

        let remaining = end_time.duration_since(now);
        let secs = remaining.as_secs();
        let text = format_countdown(secs);
        if let Some(webview) = state.countdown_webview.as_ref() {
            update_kegel_countdown(webview, &text);
        }
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
        state.countdown_webview = None;
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
