//! 倒计时窗口模块

use std::ptr;
use std::time::{Duration, Instant};

use block2::global_block;
use objc2::{MainThreadOnly, define_class, msg_send, sel};
use objc2_app_kit::{
    NSApplication, NSBackingStoreType, NSColor, NSEvent, NSEventMask, NSScreen,
    NSStatusWindowLevel, NSWindow, NSWindowCollectionBehavior, NSWindowStyleMask,
};
use objc2_foundation::{NSObjectProtocol, NSPoint, NSRect, NSSize, NSString, NSTimer, NSUInteger};
use objc2_web_kit::WKWebView;

use super::super::delegate::RestGapDelegate;
use super::super::state::with_state;
use super::super::utils::{format_countdown, play_sound};
use super::status_bar::target_anyobject;
use crate::i18n::Texts;
use crate::skip_challenge::{Feedback, SkipChallenge, Snapshot};

const KEGEL_HTML_TEMPLATE: &str = r#"<!DOCTYPE html>
<html lang="zh-CN">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Aligned Kegel Guide</title>
    <style>
        :root {
            --page-bg: #f2f0e9;
            --card-bg: #f2f0e9;
            --text-main: #333333;
            --text-sub: #757575;
            --text-strong: #1f1f1f;
            --text-muted: #6b6b6b;
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
            background-color: var(--page-bg);
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
            color: var(--text-strong);
            text-align: center;
        }

        .countdown {
            font-family: var(--font-sans);
            font-size: clamp(48px, 7vw, 96px);
            font-weight: 700;
            font-variant-numeric: tabular-nums;
            color: var(--text-strong);
            line-height: 1;
        }

        .card {
            background-color: var(--card-bg);
            width: min(90vw, 600px, calc(70vh * 4 / 3.2));
            aspect-ratio: 4 / 3.2;
            border-radius: 0;
            display: flex;
            flex-direction: column;
            justify-content: center;
            align-items: center;
            position: relative;
            box-shadow: none;
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
            font-size: clamp(180px, 26vw, 300px);
            line-height: 1;
            color: var(--text-main);
            display: flex;
            align-items: center;
        }

        .star {
            display: inline-block;
            margin: 0;
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
            color: var(--text-muted);
            text-align: center;
            max-width: 80vw;
        }

        .skip-card {
            width: min(80vw, 880px);
            padding: 18px 22px 20px;
            border: 1px solid rgba(31, 31, 31, 0.12);
            background: rgba(255, 255, 255, 0.48);
            display: flex;
            flex-direction: column;
            gap: 12px;
            transition: transform 0.25s ease, box-shadow 0.25s ease, border-color 0.25s ease;
        }

        .skip-card.failure {
            border-color: rgba(199, 73, 52, 0.55);
            box-shadow: 0 16px 36px rgba(199, 73, 52, 0.12);
            animation: skip-shake 0.42s ease;
        }

        .skip-title {
            font-family: var(--font-sans);
            font-size: clamp(14px, 1.8vw, 18px);
            letter-spacing: 0.08em;
            text-transform: uppercase;
            color: var(--text-sub);
        }

        .skip-phrase {
            font-family: ui-monospace, "SF Mono", SFMono-Regular, Menlo, monospace;
            font-size: clamp(18px, 2vw, 28px);
            line-height: 1.6;
            color: var(--text-main);
            white-space: pre-wrap;
            word-break: break-word;
        }

        .skip-char {
            display: inline-block;
            transition: transform 0.2s ease, color 0.2s ease, background-color 0.2s ease;
            padding: 0 1px;
            border-radius: 4px;
        }

        .skip-char.matched {
            color: #244c39;
            transform: translateY(-2px);
        }

        .skip-char.current {
            background: rgba(36, 76, 57, 0.12);
            color: #16241b;
        }

        .skip-char.pending {
            color: #7c7c7c;
        }

        .skip-status {
            font-family: var(--font-sans);
            font-size: clamp(14px, 1.8vw, 18px);
            color: var(--text-muted);
        }

        @keyframes star-breathe {
            0%, 100% {
                transform: translateY(25px) scale(1.35);
                opacity: 0.8;
            }
            40%, 70% {
                transform: translateY(25px) scale(0.82);
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

        @keyframes skip-shake {
            0%, 100% { transform: translateX(0); }
            20% { transform: translateX(-10px); }
            40% { transform: translateX(8px); }
            60% { transform: translateX(-6px); }
            80% { transform: translateX(4px); }
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
                <span class="star">*</span>
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
    <div class="skip-card" id="skip-card">
        <div class="skip-title" id="skip-title">__SKIP_TITLE__</div>
        <div class="skip-phrase" id="skip-phrase">__SKIP_PHRASE_HTML__</div>
        <div class="skip-status" id="skip-status">__SKIP_STATUS__</div>
    </div>
</div>
<script>
    let lastFailureSeq = 0;
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
    window.setSkipChallenge = (payload) => {
        const phraseEl = document.getElementById('skip-phrase');
        const statusEl = document.getElementById('skip-status');
        const panelEl = document.getElementById('skip-card');
        if (phraseEl) {
            phraseEl.innerHTML = payload.phraseHtml;
        }
        if (statusEl) {
            statusEl.textContent = payload.status;
        }
        if (panelEl && payload.failureSeq !== lastFailureSeq) {
            panelEl.classList.remove('failure');
            void panelEl.offsetWidth;
            panelEl.classList.add('failure');
            lastFailureSeq = payload.failureSeq;
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

fn render_skip_phrase_html(snapshot: &Snapshot) -> String {
    let mut html = String::new();
    for (idx, ch) in snapshot.phrase.chars().enumerate() {
        let class = if idx < snapshot.matched_len {
            "matched"
        } else if idx == snapshot.matched_len && snapshot.feedback != Feedback::Completed {
            "current"
        } else {
            "pending"
        };
        html.push_str("<span class=\"skip-char ");
        html.push_str(class);
        html.push_str("\">");
        html.push_str(&escape_html(&ch.to_string()));
        html.push_str("</span>");
    }
    html
}

fn skip_status_text(texts: &Texts, snapshot: &Snapshot) -> String {
    match snapshot.feedback {
        Feedback::Completed => texts.countdown_skip_success().to_string(),
        Feedback::Mismatch => texts.countdown_skip_mismatch().to_string(),
        Feedback::Timeout => texts.countdown_skip_timeout().to_string(),
        Feedback::Ready | Feedback::Progress => {
            texts.countdown_skip_progress(snapshot.matched_len, snapshot.total_len)
        }
    }
}

fn build_kegel_html(
    title: &str,
    countdown: &str,
    hint: &str,
    skip_title: &str,
    skip_phrase_html: &str,
    skip_status: &str,
) -> String {
    let mut html = KEGEL_HTML_TEMPLATE.replace("__TITLE__", &escape_html(title));
    html = html.replace("__COUNTDOWN__", &escape_html(countdown));
    html = html.replace("__HINT__", &escape_html(hint));
    html = html.replace("__SKIP_TITLE__", &escape_html(skip_title));
    html = html.replace("__SKIP_PHRASE_HTML__", skip_phrase_html);
    html.replace("__SKIP_STATUS__", &escape_html(skip_status))
}

fn update_kegel_countdown(webview: &WKWebView, text: &str) {
    let js_value = serde_json::to_string(text).unwrap_or_else(|_| "\"\"".to_string());
    let script = format!("window.setCountdown({js_value});");
    let script = NSString::from_str(&script);
    unsafe {
        webview.evaluateJavaScript_completionHandler(&script, None);
    }
}

fn update_kegel_skip_challenge(webview: &WKWebView, texts: &Texts, snapshot: &Snapshot) {
    let payload = serde_json::json!({
        "phraseHtml": render_skip_phrase_html(snapshot),
        "status": skip_status_text(texts, snapshot),
        "failureSeq": snapshot.failure_seq,
    });
    let script = format!("window.setSkipChallenge({payload});");
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

fn on_countdown_keydown(event: &NSEvent) {
    let Some(text) = event.charactersIgnoringModifiers().map(|s| s.to_string()) else {
        return;
    };

    let texts = Texts::new(with_state(|state| state.config.effective_language()));
    let mut render = None;
    with_state(|state| {
        if state.countdown_windows.is_empty() {
            return;
        }
        if state.countdown_skip_requested {
            return;
        }
        let Some(challenge) = state.countdown_skip_challenge.as_mut() else {
            return;
        };

        for ch in text.chars() {
            let result = challenge.register_char(ch, Instant::now());
            render = Some((state.countdown_webviews.clone(), result.snapshot));
            if result.completed {
                state.countdown_skip_requested = true;
                break;
            }
        }
    });

    if let Some((webviews, snapshot)) = render {
        for webview in &webviews {
            update_kegel_skip_challenge(webview, &texts, &snapshot);
        }
    }
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
    let skip_challenge = SkipChallenge::random();
    let skip_snapshot = skip_challenge.snapshot();

    // 关闭已存在的倒计时窗口
    close_countdown_window();

    // 播放开始声音
    if play_start_sound {
        play_sound("Glass");
    }

    let background = NSColor::colorWithSRGBRed_green_blue_alpha(
        242.0 / 255.0,
        240.0 / 255.0,
        233.0 / 255.0,
        1.0,
    );
    // 收集所有屏幕的 frame（多屏幕时逐屏覆盖）
    let screens = NSScreen::screens(mtm);
    let screen_count = screens.count();
    let mut frames = Vec::new();
    if screen_count == 0 {
        frames.push(NSScreen::mainScreen(mtm).map_or(
            NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(1920.0, 1080.0)),
            |s| s.frame(),
        ));
    } else {
        let mut idx: NSUInteger = 0;
        while idx < screen_count {
            let screen = screens.objectAtIndex(idx);
            frames.push(screen.frame());
            idx += 1;
        }
    }

    let mut windows = Vec::with_capacity(frames.len());
    let mut webviews = Vec::with_capacity(frames.len());
    let html = build_kegel_html(
        &texts.countdown_title(),
        &format_countdown(seconds),
        texts.countdown_hint(),
        texts.countdown_skip_title(),
        &render_skip_phrase_html(&skip_snapshot),
        &skip_status_text(&texts, &skip_snapshot),
    );
    let html = NSString::from_str(&html);
    for frame in frames {
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
        window.setBackgroundColor(Some(&background));

        let view_frame = NSRect::new(NSPoint::new(0.0, 0.0), frame.size);
        let webview = unsafe { WKWebView::initWithFrame(WKWebView::alloc(mtm), view_frame) };
        unsafe {
            let _ = webview.loadHTMLString_baseURL(&html, None);
        }
        if let Some(content_view) = window.contentView() {
            content_view.addSubview(&webview);
        }

        windows.push(window);
        webviews.push(webview);
    }

    // 让倒计时窗口能获取键盘事件（Borderless 默认不可成为 key window）
    NSApplication::sharedApplication(mtm).activateIgnoringOtherApps(true);

    // 先标记窗口存在，避免用户刚弹窗就开始输入时被忽略
    let windows_for_state = windows.clone();
    with_state(|state| {
        state.countdown_windows = windows_for_state;
        state.countdown_webviews = webviews;
        state.countdown_skip_challenge = Some(skip_challenge);
        state.countdown_skip_requested = false;
    });

    // 显示窗口
    for window in &windows {
        window.makeKeyAndOrderFront(None);
    }

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
        for webview in &state.countdown_webviews {
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
        for window in state.countdown_windows.drain(..) {
            window.orderOut(None);
        }
        state.countdown_webviews.clear();
        state.countdown_end_time = None;
        state.countdown_skip_challenge = None;
        state.countdown_skip_requested = false;
    });
}

/// 完成倒计时
pub fn finish_countdown() {
    close_countdown_window();
    play_sound("Tink");
}
