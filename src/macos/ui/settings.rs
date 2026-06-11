//! 设置界面模块
//!
//! 使用 `WKWebView` 实现现代化的设置界面。

use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2::{MainThreadMarker, MainThreadOnly, define_class, msg_send};
use objc2_app_kit::{NSApplication, NSBackingStoreType, NSColor, NSWindow, NSWindowStyleMask};
use objc2_foundation::{NSObject, NSObjectProtocol, NSPoint, NSRect, NSSize, NSString};
use objc2_web_kit::{
    WKNavigationAction, WKNavigationActionPolicy, WKNavigationDelegate, WKWebView,
    WKWebViewConfiguration,
};

use super::super::config::{Config, clamp_u64};
use super::super::delegate::RestGapDelegate;
use super::super::state::{with_state, with_state_ref};
use crate::i18n::{LanguagePreference, Texts};

define_class!(
    #[unsafe(super(NSWindow))]
    #[thread_kind = MainThreadOnly]
    pub struct SettingsWindow;

    unsafe impl NSObjectProtocol for SettingsWindow {}

    impl SettingsWindow {
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

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    pub struct SettingsNavigationDelegate;

    unsafe impl NSObjectProtocol for SettingsNavigationDelegate {}

    unsafe impl WKNavigationDelegate for SettingsNavigationDelegate {
        #[unsafe(method(webView:decidePolicyForNavigationAction:decisionHandler:))]
        fn web_view_decide_policy_for_navigation_action(
            &self,
            _web_view: &WKWebView,
            navigation_action: &WKNavigationAction,
            decision_handler: &block2::Block<dyn Fn(WKNavigationActionPolicy)>,
        ) {
            let url_str = unsafe {
                navigation_action
                    .request()
                    .URL()
                    .and_then(|u| u.absoluteString())
                    .map(|s| s.to_string())
            };

            if let Some(url_str) = url_str {
                if url_str.starts_with("restgap://") {
                    handle_callback(&url_str);
                    decision_handler.call((WKNavigationActionPolicy::Cancel,));
                    return;
                }
            }

            decision_handler.call((WKNavigationActionPolicy::Allow,));
        }
    }
);

fn handle_callback(url: &str) {
    if let Some(query) = url.strip_prefix("restgap://save?") {
        let mut interval_minutes = None;
        let mut break_seconds = None;
        let mut language = None;
        let mut allow_skip_break = None;

        for pair in query.split('&') {
            let mut parts = pair.split('=');
            let key = parts.next().unwrap_or("");
            let val = parts.next().unwrap_or("");
            match key {
                "interval" => interval_minutes = val.parse::<u64>().ok(),
                "break" => break_seconds = val.parse::<u64>().ok(),
                "language" => {
                    language = match val {
                        "0" => Some(LanguagePreference::Auto),
                        "1" => Some(LanguagePreference::En),
                        "2" => Some(LanguagePreference::Zh),
                        _ => None,
                    };
                }
                "allow_skip" => allow_skip_break = Some(val == "true"),
                _ => {}
            }
        }

        if let (Some(interval), Some(brk), Some(lang), Some(skip)) =
            (interval_minutes, break_seconds, language, allow_skip_break)
        {
            let new_config = Config {
                interval_minutes: clamp_u64(
                    interval,
                    Config::MIN_INTERVAL_MINUTES,
                    Config::MAX_INTERVAL_MINUTES,
                ),
                break_seconds: clamp_u64(brk, Config::MIN_BREAK_SECONDS, Config::MAX_BREAK_SECONDS),
                language: lang,
                allow_skip_break: skip,
            };
            new_config.save();

            with_state(|state| {
                state.config = new_config;
            });

            // 通知 delegate 更新
            if let Some(mtm) = MainThreadMarker::new() {
                let app = NSApplication::sharedApplication(mtm);
                if let Some(delegate) = app.delegate() {
                    unsafe {
                        let _: () = msg_send![&delegate, settingsChanged];
                    }
                }
            }
        }
        close_settings_window();
    } else if url == "restgap://cancel" {
        close_settings_window();
    }
}

pub fn close_settings_window() {
    with_state(|state| {
        if let Some(window) = state.settings_window.take() {
            window.close();
        }
        state.settings_webview = None;
        state.settings_nav_delegate = None;
    });
}

const SETTINGS_HTML_TEMPLATE: &str = r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <style>
        :root {
            --bg: #1C1C1E;
            --card-bg: rgba(255, 255, 255, 0.04);
            --card-hover: rgba(255, 255, 255, 0.08);
            --text: #FFFFFF;
            --text-dim: rgba(255, 255, 255, 0.45);
            --accent: #0A84FF;
            --font-sans: -apple-system, BlinkMacSystemFont, "SF Pro Display", "SF Pro Text", sans-serif;
            --radius: 14px;
        }
        body {
            margin: 0;
            padding: 32px 24px;
            background-color: var(--bg);
            color: var(--text);
            font-family: var(--font-sans);
            user-select: none;
            overflow: hidden;
            -webkit-font-smoothing: antialiased;
        }
        .header {
            margin-bottom: 32px;
            display: flex;
            align-items: center;
            gap: 16px;
        }
        .icon-container {
            width: 48px;
            height: 48px;
            background: linear-gradient(135deg, #3A3A3C, #000000);
            border-radius: 12px;
            display: flex;
            align-items: center;
            justify-content: center;
            box-shadow: 0 8px 16px rgba(0,0,0,0.4);
            position: relative;
            overflow: hidden;
        }
        .icon-glow {
            position: absolute;
            width: 100%;
            height: 100%;
            background: radial-gradient(circle at top left, rgba(255,255,255,0.1), transparent);
        }
        .icon-glyph {
            width: 24px;
            height: 24px;
            border: 2.5px solid #fff;
            border-radius: 50%;
            border-top-color: rgba(255,255,255,0.2);
            position: relative;
        }
        .icon-glyph::after {
            content: '';
            position: absolute;
            top: 6px;
            left: 6px;
            width: 2px;
            height: 8px;
            background: white;
            border-radius: 1px;
            box-shadow: 6px 0 0 white;
        }
        .title-group {
            display: flex;
            flex-direction: column;
        }
        .title {
            font-size: 22px;
            font-weight: 700;
            letter-spacing: -0.02em;
        }
        .subtitle {
            font-size: 11px;
            font-weight: 500;
            color: var(--text-dim);
            text-transform: uppercase;
            letter-spacing: 0.05em;
            margin-top: 2px;
        }
        .section-label {
            font-size: 12px;
            font-weight: 600;
            color: var(--text-dim);
            margin: 0 0 8px 8px;
            text-transform: uppercase;
            letter-spacing: 0.03em;
        }
        .group {
            background: var(--card-bg);
            border-radius: var(--radius);
            padding: 4px;
            margin-bottom: 24px;
            border: 0.5px solid rgba(255,255,255,0.05);
        }
        .row {
            display: flex;
            justify-content: space-between;
            align-items: center;
            padding: 12px 14px;
            border-radius: 10px;
            transition: background 0.2s;
        }
        .row:not(:last-child) {
            border-bottom: 0.5px solid rgba(255,255,255,0.03);
        }
        .row-info {
            display: flex;
            flex-direction: column;
            gap: 2px;
        }
        .label {
            font-size: 15px;
            font-weight: 500;
        }
        .hint {
            font-size: 12px;
            color: var(--text-dim);
        }
        .control {
            display: flex;
            align-items: center;
            gap: 8px;
        }
        input[type="number"] {
            background: rgba(255, 255, 255, 0.06);
            border: 1px solid rgba(255, 255, 255, 0.1);
            border-radius: 8px;
            color: white;
            padding: 6px 10px;
            width: 64px;
            font-size: 14px;
            font-variant-numeric: tabular-nums;
            text-align: right;
            outline: none;
            transition: all 0.2s;
        }
        input[type="number"]:focus {
            background: rgba(255, 255, 255, 0.1);
            border-color: var(--accent);
            box-shadow: 0 0 0 3px rgba(10, 132, 255, 0.2);
        }
        select {
            background: rgba(255, 255, 255, 0.06);
            border: 1px solid rgba(255, 255, 255, 0.1);
            border-radius: 8px;
            color: white;
            padding: 6px 28px 6px 12px;
            font-size: 14px;
            outline: none;
            appearance: none;
            background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='10' height='10' viewBox='0 0 24 24' fill='none' stroke='rgba(255,255,255,0.5)' stroke-width='3' stroke-linecap='round' stroke-linejoin='round'%3E%3Cpolyline points='6 9 12 15 18 9'%3E%3C/polyline%3E%3C/svg%3E");
            background-repeat: no-repeat;
            background-position: right 10px center;
        }
        /* Toggle Switch */
        .switch {
            position: relative;
            display: inline-block;
            width: 42px;
            height: 24px;
        }
        .switch input { opacity: 0; width: 0; height: 0; }
        .slider {
            position: absolute;
            cursor: pointer;
            top: 0; left: 0; right: 0; bottom: 0;
            background-color: rgba(255,255,255,0.1);
            transition: .3s cubic-bezier(0.4, 0, 0.2, 1);
            border-radius: 24px;
        }
        .slider:before {
            position: absolute;
            content: "";
            height: 20px;
            width: 20px;
            left: 2px;
            bottom: 2px;
            background-color: white;
            transition: .3s cubic-bezier(0.4, 0, 0.2, 1);
            border-radius: 50%;
            box-shadow: 0 2px 4px rgba(0,0,0,0.2);
        }
        input:checked + .slider { background-color: var(--accent); }
        input:checked + .slider:before { transform: translateX(18px); }

        .footer {
            display: flex;
            justify-content: flex-end;
            gap: 12px;
            margin-top: 12px;
        }
        button {
            padding: 10px 20px;
            border-radius: 10px;
            font-size: 14px;
            font-weight: 600;
            cursor: pointer;
            border: none;
            transition: all 0.2s cubic-bezier(0.16, 1, 0.3, 1);
        }
        button:active { transform: scale(0.96); }
        .btn-secondary {
            background: rgba(255, 255, 255, 0.08);
            color: white;
        }
        .btn-secondary:hover { background: rgba(255, 255, 255, 0.12); }
        .btn-primary {
            background: var(--accent);
            color: white;
            box-shadow: 0 4px 12px rgba(10, 132, 255, 0.3);
        }
        .btn-primary:hover { background: #007AFF; box-shadow: 0 6px 16px rgba(10, 132, 255, 0.4); }
    </style>
</head>
<body>
    <div class="header">
        <div class="icon-container">
            <div class="icon-glow"></div>
            <div class="icon-glyph"></div>
        </div>
        <div class="title-group">
            <div class="title" id="t-settings-title">__TITLE__</div>
            <div class="subtitle">RESTGAP v1.5.0</div>
        </div>
    </div>

    <div class="section-label">General</div>
    <div class="group">
        <div class="row">
            <div class="row-info">
                <div class="label" id="t-interval-label">__INTERVAL_LABEL__</div>
                <div class="hint">Frequency of break reminders</div>
            </div>
            <div class="control">
                <input type="number" id="interval" value="__INTERVAL_VAL__" min="1" max="240">
                <span class="hint">min</span>
            </div>
        </div>
        <div class="row">
            <div class="row-info">
                <div class="label" id="t-break-label">__BREAK_LABEL__</div>
                <div class="hint">Duration of each break session</div>
            </div>
            <div class="control">
                <input type="number" id="break" value="__BREAK_VAL__" min="5" max="3600">
                <span class="hint">sec</span>
            </div>
        </div>
    </div>

    <div class="section-label">Options</div>
    <div class="group">
        <div class="row">
            <div class="row-info">
                <div class="label" id="t-skip-label">__SKIP_LABEL__</div>
                <div class="hint">Enable typing challenge to skip</div>
            </div>
            <label class="switch">
                <input type="checkbox" id="allow_skip" __SKIP_CHECKED__>
                <span class="slider"></span>
            </label>
        </div>
        <div class="row">
            <div class="row-info">
                <div class="label" id="t-language-label">__LANGUAGE_LABEL__</div>
                <div class="hint">Preferred interface language</div>
            </div>
            <select id="language">
                <option value="0" __LANG_AUTO_SELECTED__>__LANG_AUTO__</option>
                <option value="1" __LANG_EN_SELECTED__>__LANG_EN__</option>
                <option value="2" __LANG_ZH_SELECTED__>__LANG_ZH__</option>
            </select>
        </div>
    </div>

    <div class="footer">
        <button class="btn-secondary" onclick="cancel()" id="t-cancel">__CANCEL__</button>
        <button class="btn-primary" onclick="save()" id="t-save">__SAVE__</button>
    </div>

    <script>
        function save() {
            const interval = document.getElementById('interval').value;
            const breakVal = document.getElementById('break').value;
            const language = document.getElementById('language').value;
            const allowSkip = document.getElementById('allow_skip').checked;
            window.location.href = `restgap://save?interval=${interval}&break=${breakVal}&language=${language}&allow_skip=${allowSkip}`;
        }
        function cancel() {
            window.location.href = 'restgap://cancel';
        }
    </script>
</body>
</html>
"#;

/// 打开配置对话框
#[allow(clippy::too_many_lines)]
pub fn open_settings_dialog(delegate: &RestGapDelegate) {
    let mtm = delegate.mtm();

    let already_open = with_state(|state| {
        state.settings_window.as_ref().is_some_and(|window| {
            window.makeKeyAndOrderFront(None);
            true
        })
    });
    if already_open {
        return;
    }

    let config = with_state_ref(|s| s.config.clone());
    let texts = Texts::new(config.effective_language());

    let window_size = NSSize::new(420.0, 580.0);
    let screen_frame = objc2_app_kit::NSScreen::mainScreen(mtm).map_or(
        NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(1920.0, 1080.0)),
        |s| s.frame(),
    );
    let x = screen_frame.origin.x + (screen_frame.size.width - window_size.width) / 2.0;
    let y = screen_frame.origin.y + (screen_frame.size.height - window_size.height) / 2.0;
    let window_frame = NSRect::new(NSPoint::new(x, y), window_size);

    let style = NSWindowStyleMask::Titled
        | NSWindowStyleMask::Closable
        | NSWindowStyleMask::FullSizeContentView;
    let window: Retained<SettingsWindow> = unsafe {
        msg_send![
            SettingsWindow::alloc(mtm),
            initWithContentRect: window_frame
            styleMask: style
            backing: NSBackingStoreType::Buffered
            defer: false
        ]
    };
    let window: Retained<NSWindow> = window.into_super();

    window.setTitle(&NSString::from_str(texts.settings_title()));
    window.setTitlebarAppearsTransparent(true);
    window.setTitleVisibility(objc2_app_kit::NSWindowTitleVisibility::Hidden);
    window.setBackgroundColor(Some(&NSColor::colorWithSRGBRed_green_blue_alpha(
        0.11, 0.11, 0.12, 1.0,
    )));

    let config_webview = unsafe { WKWebViewConfiguration::new(mtm) };
    let webview: Retained<WKWebView> = unsafe {
        msg_send![
            WKWebView::alloc(mtm),
            initWithFrame: NSRect::new(NSPoint::new(0.0, 0.0), window_size)
            configuration: &*config_webview
        ]
    };

    let nav_delegate: Retained<SettingsNavigationDelegate> =
        unsafe { msg_send![SettingsNavigationDelegate::alloc(mtm), init] };
    unsafe {
        webview.setNavigationDelegate(Some(ProtocolObject::from_ref(&*nav_delegate)));
    }

    let mut html = SETTINGS_HTML_TEMPLATE
        .replace("__TITLE__", texts.settings_title())
        .replace("__INTERVAL_LABEL__", texts.settings_interval_label())
        .replace("__INTERVAL_VAL__", &config.interval_minutes.to_string())
        .replace("__BREAK_LABEL__", texts.settings_break_label())
        .replace("__BREAK_VAL__", &config.break_seconds.to_string())
        .replace("__SKIP_LABEL__", texts.settings_skip_break_label())
        .replace(
            "__SKIP_CHECKED__",
            if config.allow_skip_break {
                "checked"
            } else {
                ""
            },
        )
        .replace("__LANGUAGE_LABEL__", texts.menu_language_header())
        .replace("__LANG_AUTO__", texts.language_auto())
        .replace("__LANG_EN__", texts.language_en())
        .replace("__LANG_ZH__", texts.language_zh())
        .replace("__CANCEL__", texts.settings_cancel_button())
        .replace("__SAVE__", texts.settings_save_button());

    html = html.replace(
        "__LANG_AUTO_SELECTED__",
        if config.language == LanguagePreference::Auto {
            "selected"
        } else {
            ""
        },
    );
    html = html.replace(
        "__LANG_EN_SELECTED__",
        if config.language == LanguagePreference::En {
            "selected"
        } else {
            ""
        },
    );
    html = html.replace(
        "__LANG_ZH_SELECTED__",
        if config.language == LanguagePreference::Zh {
            "selected"
        } else {
            ""
        },
    );

    unsafe {
        let _ = webview.loadHTMLString_baseURL(&NSString::from_str(&html), None);
    }

    if let Some(content_view) = window.contentView() {
        content_view.addSubview(&webview);
    }

    window.makeKeyAndOrderFront(None);
    NSApplication::sharedApplication(mtm).activateIgnoringOtherApps(true);

    with_state(|state| {
        state.settings_window = Some(window);
        state.settings_webview = Some(webview);
        state.settings_nav_delegate = Some(nav_delegate.into_super().into());
    });
}
