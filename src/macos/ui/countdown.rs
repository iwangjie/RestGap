use std::ptr;
use std::time::{Duration, Instant};

use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2::{MainThreadMarker, MainThreadOnly, define_class, msg_send, sel};
use objc2_app_kit::{
    NSApplication, NSBackingStoreType, NSColor, NSEvent, NSMenu, NSScreen, NSStatusWindowLevel,
    NSWindow, NSWindowCollectionBehavior, NSWindowStyleMask,
};
use objc2_foundation::{
    NSObject, NSObjectProtocol, NSPoint, NSRect, NSSize, NSString, NSTimer, NSUInteger,
};
use objc2_web_kit::{
    WKNavigationAction, WKNavigationActionPolicy, WKNavigationDelegate, WKWebView,
};

use super::super::delegate::RestGapDelegate;
use super::super::state::with_state;
use super::super::utils::{format_countdown, play_sound};
use super::status_bar::target_anyobject;
use crate::i18n::Texts;

const COUNTDOWN_HTML_TEMPLATE: &str = r##"<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <style>
        :root {
            --bg: #000000;
            --text: #ffffff;
            --text-dim: rgba(255, 255, 255, 0.4);
            --line: rgba(255, 255, 255, 0.1);
            --font-sans: -apple-system, BlinkMacSystemFont, "SF Pro Display", sans-serif;
            --font-mono: "SF Mono", "SFMono-Regular", ui-monospace, monospace;
        }

        body {
            margin: 0;
            padding: 0;
            height: 100vh;
            overflow: hidden;
            background-color: var(--bg);
            color: var(--text);
            font-family: var(--font-sans);
            display: flex;
            justify-content: center;
            align-items: center;
            -webkit-font-smoothing: antialiased;
            user-select: none;
        }

        .bg-glow {
            position: absolute;
            width: 150vmax;
            height: 150vmax;
            background: radial-gradient(circle at center, rgba(255, 255, 255, 0.05) 0%, transparent 50%);
            animation: breathe 12s infinite ease-in-out;
            pointer-events: none;
        }

        @keyframes breathe {
            0%, 100% { transform: scale(1); opacity: 0.3; }
            50% { transform: scale(1.1); opacity: 0.7; }
        }

        .container {
            display: flex;
            flex-direction: column;
            align-items: center;
            gap: 48px;
            z-index: 1;
            width: 100%;
            max-width: 800px;
        }

        .timer-group {
            display: flex;
            flex-direction: column;
            align-items: center;
            gap: 12px;
        }

        .countdown {
            font-size: 160px;
            font-weight: 100;
            line-height: 0.9;
            letter-spacing: -0.04em;
            font-variant-numeric: tabular-nums;
        }

        .title {
            font-size: 14px;
            font-weight: 500;
            letter-spacing: 0.3em;
            text-transform: uppercase;
            color: var(--text-dim);
        }

        .hint {
            font-size: 16px;
            color: var(--text-dim);
            max-width: 480px;
            line-height: 1.6;
            text-align: center;
            display: none;
        }

        /* 极简工位训练卡片 */
        .exercise-card {
            width: 460px;
            padding: 24px 28px;
            background: rgba(255, 255, 255, 0.02);
            border: 1px solid rgba(255, 255, 255, 0.06);
            border-radius: 24px;
            backdrop-filter: blur(30px);
            -webkit-backdrop-filter: blur(30px);
            display: flex;
            flex-direction: column;
            gap: 18px;
            box-shadow: 0 8px 32px 0 rgba(0, 0, 0, 0.37);
        }
        .exercise-header {
            display: flex;
            align-items: center;
            gap: 16px;
        }
        .exercise-icon {
            width: 52px;
            height: 52px;
            background: rgba(255, 255, 255, 0.04);
            border-radius: 12px;
            display: flex;
            align-items: center;
            justify-content: center;
            flex-shrink: 0;
        }
        .exercise-meta {
            display: flex;
            flex-direction: column;
            gap: 4px;
            text-align: left;
        }
        .exercise-label {
            font-size: 10px;
            font-weight: 700;
            color: #0A84FF;
            letter-spacing: 0.1em;
            text-transform: uppercase;
        }
        .exercise-title {
            margin: 0;
            font-size: 18px;
            font-weight: 600;
            color: #ffffff;
            letter-spacing: -0.01em;
        }
        .exercise-steps {
            display: flex;
            flex-direction: column;
            gap: 10px;
            text-align: left;
        }
        .exercise-step {
            font-size: 13px;
            line-height: 1.5;
            color: rgba(255, 255, 255, 0.85);
        }
        .exercise-step strong {
            color: #ffffff;
        }
        .exercise-divider {
            height: 1px;
            background: rgba(255, 255, 255, 0.06);
            margin: 2px 0;
        }


        /* 隐藏的跳过按钮 */
        .skip-btn {
            position: absolute;
            top: 24px;
            right: 24px;
            padding: 8px 16px;
            font-size: 13px;
            font-weight: 500;
            color: rgba(255, 255, 255, 0.15);
            cursor: pointer;
            border: 1px solid rgba(255, 255, 255, 0.05);
            border-radius: 8px;
            background: transparent;
            transition: all 0.3s ease;
            user-select: none;
            z-index: 100;
            display: none;
        }
        .skip-btn:hover {
            color: rgba(255, 255, 255, 0.6);
            border-color: rgba(255, 255, 255, 0.2);
            background: rgba(255, 255, 255, 0.05);
        }

        /* 跳过确认弹窗 */
        .skip-modal {
            position: fixed;
            top: 0;
            left: 0;
            width: 100vw;
            height: 100vh;
            background: rgba(0, 0, 0, 0.85);
            backdrop-filter: blur(20px);
            -webkit-backdrop-filter: blur(20px);
            display: flex;
            justify-content: center;
            align-items: center;
            opacity: 0;
            pointer-events: none;
            transition: opacity 0.3s ease;
            z-index: 200;
        }
        .skip-modal.show {
            opacity: 1;
            pointer-events: auto;
        }
        .skip-modal-content {
            width: 380px;
            padding: 32px;
            background: rgba(20, 20, 20, 0.8);
            border: 1px solid rgba(255, 255, 255, 0.08);
            border-radius: 20px;
            box-shadow: 0 20px 50px rgba(0, 0, 0, 0.6);
            display: flex;
            flex-direction: column;
            gap: 20px;
            text-align: center;
            transform: scale(0.95);
            transition: transform 0.3s cubic-bezier(0.16, 1, 0.3, 1);
        }
        .skip-modal.show .skip-modal-content {
            transform: scale(1);
        }
        .skip-modal-title {
            font-size: 18px;
            font-weight: 600;
            letter-spacing: -0.01em;
        }
        .skip-modal-prompt {
            font-size: 13px;
            color: var(--text-dim);
            line-height: 1.5;
        }
        .skip-modal-prompt strong {
            color: #ff453a;
            background: rgba(255, 69, 58, 0.15);
            padding: 2px 8px;
            border-radius: 4px;
            font-family: var(--font-mono);
            font-weight: bold;
            margin: 0 2px;
            display: inline-block;
        }
        .skip-input {
            width: 100%;
            box-sizing: border-box;
            background: rgba(255, 255, 255, 0.05);
            border: 1px solid rgba(255, 255, 255, 0.1);
            border-radius: 10px;
            color: #fff;
            padding: 10px 14px;
            font-size: 15px;
            outline: none;
            text-align: center;
            transition: all 0.2s;
        }
        .skip-input:focus {
            border-color: rgba(255, 255, 255, 0.3);
            background: rgba(255, 255, 255, 0.08);
        }
        .skip-modal-actions {
            display: flex;
            gap: 12px;
        }
        .modal-btn {
            flex: 1;
            padding: 10px 16px;
            border-radius: 10px;
            font-size: 14px;
            font-weight: 600;
            cursor: pointer;
            border: none;
            transition: all 0.2s;
        }
        .btn-cancel {
            background: rgba(255, 255, 255, 0.08);
            color: #fff;
        }
        .btn-cancel:hover {
            background: rgba(255, 255, 255, 0.12);
        }
        .btn-confirm {
            background: #ff453a;
            color: #fff;
            opacity: 0.3;
            pointer-events: none;
        }
        .btn-confirm.active {
            opacity: 1;
            pointer-events: auto;
            box-shadow: 0 4px 12px rgba(255, 69, 58, 0.3);
        }
        .btn-confirm.active:hover {
            background: #ff3b30;
        }
    </style>
</head>
<body>
    <div class="bg-glow"></div>
    <div class="skip-btn" id="skip-btn" onclick="openSkipModal()">Skip</div>

    <div class="container">
        <div class="timer-group">
            <div class="countdown" id="countdown">__COUNTDOWN__</div>
            <div class="title" id="title">__TITLE__</div>
        </div>
        <div class="hint" id="hint">__HINT__</div>

        <!-- 极简工位拉伸卡片 -->
        <div class="exercise-card" id="exercise-card">
            <div class="exercise-header">
                <div class="exercise-icon" id="exercise-icon"></div>
                <div class="exercise-meta">
                    <span class="exercise-label" id="exercise-label">STRETCH & MOBILITY</span>
                    <h3 class="exercise-title" id="exercise-title">---</h3>
                </div>
            </div>
            <div class="exercise-steps" id="exercise-steps"></div>
        </div>
    </div>

    <!-- 跳过确认弹窗 -->
    <div class="skip-modal" id="skip-modal">
        <div class="skip-modal-content">
            <div class="skip-modal-title" id="skip-modal-title">Confirm Skip</div>
            <div class="skip-modal-prompt" id="skip-modal-prompt">Type to confirm:</div>
            <input type="text" class="skip-input" id="skip-input" autocomplete="off" spellcheck="false" placeholder="..." oninput="checkSkipInput()">
            <div class="skip-modal-actions">
                <button class="modal-btn btn-cancel" onclick="closeSkipModal()" id="btn-cancel">Cancel</button>
                <button class="modal-btn btn-confirm" id="btn-confirm" onclick="confirmSkip()" disabled>Confirm</button>
            </div>
        </div>
    </div>

<script>
    const skipEnabled = __SKIP_ENABLED__;
    const currentLang = "__LANG__";

    window.addEventListener('contextmenu', (e) => e.preventDefault());
    window.setCountdown = (v) => {
        const el = document.getElementById('countdown');
        if (el) el.textContent = v;
    };
    window.setTitle = (v) => {
        const el = document.getElementById('title');
        if (el) el.textContent = v;
    };
    window.setHint = (v) => {
        const el = document.getElementById('hint');
        if (el) el.textContent = v;
    };

    // 配置跳过按钮
    if (skipEnabled) {
        const skipBtn = document.getElementById('skip-btn');
        if (skipBtn) {
            skipBtn.textContent = currentLang === 'zh' ? '跳过' : 'Skip';
            skipBtn.style.display = 'block';
        }
    }

    // 训练动作数据
    const exercises = [
        {
            id: "desk-plus",
            label: currentLang === 'zh' ? "前锯肌激活 · 拯救背痛" : "SERRATUS ANTERIOR · BACK PAIN RELIEF",
            title: currentLang === 'zh' ? "办公桌“推击加壳” (Desk Plus)" : "Desk Plus (Desk Press & Push)",
            steps: currentLang === 'zh' ? [
                "身体前倾，将双手掌或整个小臂平放在办公桌上。保持<strong>手肘绝对伸直不弯曲</strong>。",
                "<strong>呼气：</strong>双手用力向下、向远按压办公桌，利用反作用力让整个<strong>上背部向后鼓起</strong>（像猫咪弓背）。",
                "<strong>吸气：</strong>保持手臂伸直，放松并让胸口向下沉，去感受背部夹紧。",
                "每次连续做 <strong>10</strong> 次。"
            ] : [
                "Lean forward, place hands or forearms flat on the desk. Keep <strong>elbows locked straight</strong>.",
                "<strong>Exhale:</strong> Press down & forward firmly. Use reaction force to <strong>arch your upper back backward</strong> (like a cat stretch).",
                "<strong>Inhale:</strong> Keep arms straight, relax, and let your chest sink down to feel your back squeeze.",
                "Perform <strong>10</strong> times consecutively."
            ],
            iconSvg: `<svg width="36" height="36" viewBox="0 0 64 64" fill="none">
                <line x1="8" y1="48" x2="56" y2="48" stroke="rgba(255,255,255,0.2)" stroke-width="2" stroke-linecap="round"/>
                <line x1="32" y1="48" x2="32" y2="36" stroke="rgba(255,255,255,0.4)" stroke-width="2" stroke-linecap="round"/>
                <path d="M 32 36 Q 22 26 32 16" stroke="#0A84FF" stroke-width="3" stroke-linecap="round" fill="none">
                    <animate attributeName="d" 
                             values="M 32 36 Q 22 26 32 16; M 32 36 Q 14 26 32 16; M 32 36 Q 22 26 32 16" 
                             dur="4s" repeatCount="indefinite" />
                </path>
            </svg>`
        },
        {
            id: "seated-punch-plus",
            label: currentLang === 'zh' ? "稳定肌群激活 · 弹响改善" : "SHOULDER STABILIZERS · ALIGNMENT",
            title: currentLang === 'zh' ? "空气“V字盲推” (Seated Punch Plus)" : "Air V-Push (Seated Punch Plus)",
            steps: currentLang === 'zh' ? [
                "背部离开靠背挺直，双手在不产生弹响的<strong>斜前方30°-45°（V字形）</strong>举起，大拇指尖朝上。",
                "保持手臂伸直，用肩膀的力量将双手<strong>拼命向前伸</strong>（够大屏幕），悬停 1 秒。",
                "<strong>原路收回</strong>肩膀，但手不要放下来。",
                "每次做 <strong>8-10</strong> 次，在安全的无响声区间内重新建立肌肉记忆。"
            ] : [
                "Sit tall away from the backrest. Raise arms at a <strong>30°-45° diagonal angle (V-shape)</strong>, thumbs pointing up.",
                "Keep arms straight. Use shoulder blades to <strong>push hands forward</strong> (reach for screen), hold for 1s.",
                "<strong>Retract shoulders</strong> back to starting position (keep arms raised).",
                "Perform <strong>8-10</strong> times. Rebuilds shoulder motor patterns safely without clicking."
            ],
            iconSvg: `<svg width="36" height="36" viewBox="0 0 64 64" fill="none">
                <line x1="20" y1="44" x2="44" y2="44" stroke="rgba(255,255,255,0.2)" stroke-width="2" stroke-linecap="round" />
                <line x1="24" y1="44" x2="14" y2="24" stroke="#0A84FF" stroke-width="3" stroke-linecap="round">
                    <animate attributeName="x2" values="14; 10; 14" dur="3s" repeatCount="indefinite" />
                    <animate attributeName="y2" values="24; 16; 24" dur="3s" repeatCount="indefinite" />
                </line>
                <line x1="40" y1="44" x2="50" y2="24" stroke="#0A84FF" stroke-width="3" stroke-linecap="round">
                    <animate attributeName="x2" values="50; 54; 50" dur="3s" repeatCount="indefinite" />
                    <animate attributeName="y2" values="24; 16; 24" dur="3s" repeatCount="indefinite" />
                </line>
                <circle cx="14" cy="24" r="3" fill="#ffffff">
                    <animate attributeName="cx" values="14; 10; 14" dur="3s" repeatCount="indefinite" />
                    <animate attributeName="cy" values="24; 16; 24" dur="3s" repeatCount="indefinite" />
                </circle>
                <circle cx="50" cy="24" r="3" fill="#ffffff">
                    <animate attributeName="cx" values="50; 54; 50" dur="3s" repeatCount="indefinite" />
                    <animate attributeName="cy" values="24; 16; 24" dur="3s" repeatCount="indefinite" />
                </circle>
            </svg>`
        },
        {
            id: "chair-depressions",
            label: currentLang === 'zh' ? "下斜方肌增肌 · 消除耸肩" : "LOWER TRAPS · DEFUSE TECH NECK",
            title: currentLang === 'zh' ? "办公椅“反向撑体” (Chair Depressions)" : "Chair Depressions",
            steps: currentLang === 'zh' ? [
                "双手撑在椅子的扶手（或屁股两侧椅面边缘），手臂伸直，挺胸。",
                "<strong>呼气：</strong>双手发力将扶手狠狠往下压，借助反作用力将原本高耸的<strong>肩膀用力沉下去，让脖子无限拔长</strong>。",
                "能力强的人可以让屁股<strong>微微悬空 1 厘米</strong>。",
                "每组保持 <strong>3</strong> 秒，重复 <strong>8</strong> 次。"
            ] : [
                "Place hands on armrests or seat edges. Arms straight, chest up.",
                "<strong>Exhale:</strong> Press down firmly. Use reaction force to <strong>draw shoulders down, lengthening your neck</strong>.",
                "If strong enough, let your hips <strong>hover 1 cm</strong> off the seat.",
                "Hold for <strong>3</strong> seconds, repeat <strong>8</strong> times."
            ],
            iconSvg: `<svg width="36" height="36" viewBox="0 0 64 64" fill="none">
                <line x1="12" y1="38" x2="20" y2="38" stroke="rgba(255,255,255,0.3)" stroke-width="2" stroke-linecap="round" />
                <line x1="44" y1="38" x2="52" y2="38" stroke="rgba(255,255,255,0.3)" stroke-width="2" stroke-linecap="round" />
                <g>
                    <animateTransform attributeName="transform" type="translate"
                                      values="0,0; 0,-5; 0,0" dur="4s" repeatCount="indefinite" />
                    <line x1="16" y1="38" x2="16" y2="30" stroke="rgba(255,255,255,0.5)" stroke-width="2" />
                    <line x1="48" y1="38" x2="48" y2="30" stroke="rgba(255,255,255,0.5)" stroke-width="2" />
                    <line x1="16" y1="30" x2="48" y2="30" stroke="#0A84FF" stroke-width="3" stroke-linecap="round" />
                    <line x1="32" y1="30" x2="32" y2="20" stroke="#ffffff" stroke-width="2" />
                    <circle cx="32" cy="16" r="4" fill="#ffffff" />
                </g>
            </svg>`
        }
    ];

    // 随机选择动作并渲染
    const randomIdx = Math.floor(Math.random() * exercises.length);
    const ex = exercises[randomIdx];

    document.getElementById('exercise-icon').innerHTML = ex.iconSvg;
    document.getElementById('exercise-label').textContent = ex.label;
    document.getElementById('exercise-title').textContent = ex.title;

    const stepsHtml = ex.steps.map(step => `<div class="exercise-step">${step}</div>`).join('<div class="exercise-divider"></div>');
    document.getElementById('exercise-steps').innerHTML = stepsHtml;

    // 跳过确认弹窗逻辑
    const targetText = currentLang === 'zh' ? '紧急情况，跳过休息' : 'Emergency, skip break';
    
    window.openSkipModal = () => {
        const modal = document.getElementById('skip-modal');
        if (modal) {
            modal.classList.add('show');
            
            document.getElementById('skip-modal-title').textContent = currentLang === 'zh' ? '跳过休息确认' : 'Confirm Skip';
            document.getElementById('skip-modal-prompt').innerHTML = currentLang === 'zh' 
                ? `确认跳过休息？请输入以下文字确认：<br><strong>${targetText}</strong>` 
                : `Are you sure you want to skip? Please type the following to confirm:<br><strong>${targetText}</strong>`;
            document.getElementById('btn-cancel').textContent = currentLang === 'zh' ? '取消' : 'Cancel';
            document.getElementById('btn-confirm').textContent = currentLang === 'zh' ? '确定' : 'Confirm';

            const input = document.getElementById('skip-input');
            if (input) {
                input.value = '';
                input.focus();
            }
            window.checkSkipInput();
        }
    };

    window.closeSkipModal = () => {
        const modal = document.getElementById('skip-modal');
        if (modal) modal.classList.remove('show');
    };

    window.checkSkipInput = () => {
        const input = document.getElementById('skip-input');
        const confirmBtn = document.getElementById('btn-confirm');
        if (input && confirmBtn) {
            if (input.value === targetText) {
                confirmBtn.classList.add('active');
                confirmBtn.disabled = false;
            } else {
                confirmBtn.classList.remove('active');
                confirmBtn.disabled = true;
            }
        }
    };

    window.confirmSkip = () => {
        window.location.href = 'restgap://skip';
    };
</script>
</body>
</html>
"##;

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

fn build_countdown_html(
    title: &str,
    countdown: &str,
    hint: &str,
    skip_enabled: bool,
    lang: &str,
) -> String {
    let mut html = COUNTDOWN_HTML_TEMPLATE.replace("__TITLE__", &escape_html(title));
    html = html.replace("__TITLE__", &escape_html(title));
    html = html.replace("__COUNTDOWN__", &escape_html(countdown));
    html = html.replace("__HINT__", &escape_html(hint));
    html = html.replace(
        "__SKIP_ENABLED__",
        if skip_enabled { "true" } else { "false" },
    );
    html.replace("__LANG__", lang)
}

fn update_countdown_text(webview: &WKWebView, text: &str) {
    let js_value = serde_json::to_string(text).unwrap_or_else(|_| "\"\"".to_string());
    let script = format!("window.setCountdown({js_value});");
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
define_class!(
    #[unsafe(super(WKWebView))]
    #[thread_kind = MainThreadOnly]
    pub struct CountdownWebView;

    unsafe impl NSObjectProtocol for CountdownWebView {}

    impl CountdownWebView {
        #[unsafe(method(menuForEvent:))]
        fn menu_for_event(&self, _event: &NSEvent) -> *mut NSMenu {
            ptr::null_mut()
        }
    }
);

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    pub struct CountdownNavigationDelegate;

    unsafe impl NSObjectProtocol for CountdownNavigationDelegate {}

    unsafe impl WKNavigationDelegate for CountdownNavigationDelegate {
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
                if url_str == "restgap://skip" {
                    if let Some(mtm) = MainThreadMarker::new() {
                        let app = NSApplication::sharedApplication(mtm);
                        if let Some(delegate) = app.delegate() {
                            unsafe {
                                let _: () = msg_send![&delegate, skipBreak: Option::<&objc2::runtime::AnyObject>::None];
                            }
                        }
                    }
                    decision_handler.call((WKNavigationActionPolicy::Cancel,));
                    return;
                }
            }

            decision_handler.call((WKNavigationActionPolicy::Allow,));
        }
    }
);

/// 显示倒计时窗口
#[allow(clippy::too_many_lines)]
pub fn show_countdown_window(delegate: &RestGapDelegate, seconds: u64, play_start_sound: bool) {
    let mtm = delegate.mtm();
    let (texts, allow_skip_break) = with_state(|state| {
        let language = state.config.effective_language();
        (Texts::new(language), state.config.allow_skip_break)
    });
    let lang_str = match texts.language() {
        crate::i18n::Language::En => "en",
        crate::i18n::Language::Zh => "zh",
    };

    // 关闭已存在的倒计时窗口
    close_countdown_window();

    // 播放开始声音
    if play_start_sound {
        play_sound("Glass");
    }

    let background = NSColor::colorWithSRGBRed_green_blue_alpha(0.0, 0.0, 0.0, 1.0);
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
    let html = build_countdown_html(
        &texts.countdown_title(),
        &format_countdown(seconds),
        texts.countdown_hint(),
        allow_skip_break,
        lang_str,
    );
    let html = NSString::from_str(&html);

    let nav_delegate: Retained<CountdownNavigationDelegate> =
        unsafe { msg_send![CountdownNavigationDelegate::alloc(mtm), init] };

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
        let webview: objc2::rc::Retained<CountdownWebView> =
            unsafe { msg_send![CountdownWebView::alloc(mtm), initWithFrame: view_frame] };
        let webview: objc2::rc::Retained<WKWebView> = webview.into_super();
        unsafe {
            webview.setNavigationDelegate(Some(ProtocolObject::from_ref(&*nav_delegate)));
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
        state.countdown_nav_delegate = Some(nav_delegate.into_super().into());
    });

    // 显示窗口
    for window in &windows {
        window.makeKeyAndOrderFront(None);
    }

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
            update_countdown_text(webview, &text);
        }
        true
    })
}

/// 关闭倒计时窗口
pub fn close_countdown_window() {
    with_state(|state| {
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
        state.countdown_nav_delegate = None;
    });
}

/// 完成倒计时
pub fn finish_countdown() {
    close_countdown_window();
    play_sound("Tink");
}
