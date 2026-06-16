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

const COUNTDOWN_HTML_TEMPLATE: &str = r#"<!DOCTYPE html>
<html class="__THEME_CLASS__">
<head>
    <meta charset="UTF-8">
    <style>
        :root {
            --bg: #0b0b0f;
            --text: #ffffff;
            --text-dim: rgba(255, 255, 255, 0.4);
            --line: rgba(255, 255, 255, 0.1);
            
            --svg-main: rgba(255, 255, 255, 0.6);
            --svg-sub: rgba(255, 255, 255, 0.4);
            --svg-bg: rgba(255, 255, 255, 0.15);
            --svg-bg-dim: rgba(255, 255, 255, 0.08);
            --skip-btn-bg: rgba(255, 255, 255, 0.01);
            --skip-btn-border: rgba(255, 255, 255, 0.06);
            --skip-btn-text: rgba(255, 255, 255, 0.25);
            --skip-btn-hover-bg: rgba(255, 255, 255, 0.05);
            --skip-btn-hover-border: rgba(255, 255, 255, 0.15);
            --badge-bg: rgba(255, 255, 255, 0.04);
            
            --modal-bg: rgba(20, 20, 22, 0.65);
            --modal-border: rgba(255, 255, 255, 0.08);
            --modal-overlay: rgba(0, 0, 0, 0.75);

            --gradient-start: #ffffff;
            --gradient-end: rgba(255, 255, 255, 0.4);

            --font-sans: -apple-system, BlinkMacSystemFont, "SF Pro Display", "SF Pro Text", "Helvetica Neue", sans-serif;
            --font-mono: "SF Mono", "SFMono-Regular", ui-monospace, monospace;
            /* Dynamic variables set by JS */
            --accent: #00d2ff;
            --accent-rgb: 0, 210, 255;
        }

        html.light {
            --bg: #f5f5f7;
            --text: #1d1d1f;
            --text-dim: rgba(29, 29, 31, 0.55);
            --line: rgba(0, 0, 0, 0.08);

            --svg-main: rgba(29, 29, 31, 0.7);
            --svg-sub: rgba(29, 29, 31, 0.5);
            --svg-bg: rgba(29, 29, 31, 0.15);
            --svg-bg-dim: rgba(29, 29, 31, 0.08);
            --skip-btn-bg: rgba(0, 0, 0, 0.02);
            --skip-btn-border: rgba(0, 0, 0, 0.08);
            --skip-btn-text: rgba(29, 29, 31, 0.45);
            --skip-btn-hover-bg: rgba(0, 0, 0, 0.06);
            --skip-btn-hover-border: rgba(0, 0, 0, 0.18);
            --badge-bg: rgba(0, 0, 0, 0.04);

            --modal-bg: rgba(255, 255, 255, 0.85);
            --modal-border: rgba(0, 0, 0, 0.08);
            --modal-overlay: rgba(255, 255, 255, 0.4);

            --gradient-start: #1d1d1f;
            --gradient-end: rgba(29, 29, 31, 0.5);
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



        .container {
            display: flex;
            flex-direction: column;
            align-items: center;
            gap: 40px;
            z-index: 1;
            width: 100%;
            max-width: 800px;
        }

        .timer-group {
            display: flex;
            flex-direction: column;
            align-items: center;
            gap: 16px;
        }

        .countdown {
            font-size: 140px;
            font-weight: 100;
            line-height: 0.9;
            letter-spacing: -0.04em;
            font-variant-numeric: tabular-nums;
            background: linear-gradient(180deg, var(--gradient-start) 40%, var(--gradient-end) 100%);
            -webkit-background-clip: text;
            -webkit-text-fill-color: transparent;
        }

        .title {
            font-size: 13px;
            font-weight: 600;
            letter-spacing: 0.4em;
            text-transform: uppercase;
            color: var(--accent);
            transition: all 0.5s ease;
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
            width: 720px;
            background: transparent;
            border: none;
            border-radius: 32px;
            display: flex;
            gap: 40px;
            padding: 40px;
            transition: all 0.6s cubic-bezier(0.16, 1, 0.3, 1);
            box-sizing: border-box;
        }

        .exercise-left {
            width: 220px;
            height: 220px;
            background: transparent;
            border: none;
            border-radius: 24px;
            display: flex;
            align-items: center;
            justify-content: center;
            flex-shrink: 0;
            position: relative;
            overflow: hidden;
        }

        .exercise-left::before {
            content: '';
            position: absolute;
            width: 140px;
            height: 140px;
            background: radial-gradient(circle, rgba(var(--accent-rgb), 0.15) 0%, transparent 70%);
            pointer-events: none;
        }

        .exercise-illustration {
            z-index: 1;
            display: flex;
            align-items: center;
            justify-content: center;
        }

        .exercise-right {
            display: flex;
            flex-direction: column;
            justify-content: space-between;
            flex-grow: 1;
            text-align: left;
        }

        .exercise-header {
            display: flex;
            flex-direction: column;
            gap: 10px;
        }

        .exercise-badge {
            align-self: flex-start;
            font-size: 11px;
            font-weight: 700;
            color: var(--accent);
            background: rgba(var(--accent-rgb), 0.12);
            padding: 5px 12px;
            border-radius: 100px;
            letter-spacing: 0.1em;
            text-transform: uppercase;
            border: none;
        }

        .exercise-title {
            margin: 0;
            font-size: 28px;
            font-weight: 700;
            color: var(--text);
            letter-spacing: -0.02em;
        }

        .exercise-steps {
            display: flex;
            flex-direction: column;
            gap: 18px;
            margin: 24px 0;
        }

        .exercise-step {
            display: flex;
            align-items: flex-start;
            gap: 16px;
        }

        .step-number {
            width: 32px;
            height: 32px;
            border-radius: 50%;
            background: rgba(var(--accent-rgb), 0.08);
            border: none;
            color: var(--accent);
            display: flex;
            align-items: center;
            justify-content: center;
            font-size: 14px;
            font-weight: 700;
            flex-shrink: 0;
            margin-top: 2px;
        }

        .step-content {
            display: flex;
            flex-direction: column;
            gap: 4px;
        }

        .step-title {
            font-size: 18px;
            font-weight: 600;
            color: var(--text);
            letter-spacing: -0.01em;
        }

        .step-desc {
            font-size: 14px;
            color: var(--text-dim);
            line-height: 1.5;
            font-weight: 400;
        }

        .exercise-footer {
            display: flex;
            align-items: center;
            margin-top: 4px;
        }

        .exercise-reps-badge {
            font-size: 14px;
            font-weight: 600;
            color: var(--text);
            background: var(--badge-bg);
            border: none;
            padding: 8px 18px;
            border-radius: 14px;
            display: inline-flex;
            align-items: center;
            gap: 8px;
        }

        .exercise-reps-badge::before {
            content: '';
            width: 8px;
            height: 8px;
            background-color: var(--accent);
            border-radius: 50%;
        }

        /* 隐藏的跳过按钮 */
        .skip-btn {
            position: absolute;
            top: 32px;
            right: 32px;
            padding: 10px 20px;
            font-size: 14px;
            font-weight: 500;
            color: var(--skip-btn-text);
            cursor: pointer;
            border: 1px solid var(--skip-btn-border);
            border-radius: 12px;
            background: var(--skip-btn-bg);
            backdrop-filter: blur(20px);
            -webkit-backdrop-filter: blur(20px);
            transition: all 0.3s cubic-bezier(0.16, 1, 0.3, 1);
            user-select: none;
            z-index: 100;
            display: none;
        }
        .skip-btn:hover {
            color: var(--text);
            border-color: var(--skip-btn-hover-border);
            background: var(--skip-btn-hover-bg);
            box-shadow: 0 8px 24px rgba(0, 0, 0, 0.12);
            transform: translateY(-1px);
        }

        /* 跳过确认弹窗 */
        .skip-modal {
            position: fixed;
            top: 0;
            left: 0;
            width: 100vw;
            height: 100vh;
            background: var(--modal-overlay);
            backdrop-filter: blur(30px);
            -webkit-backdrop-filter: blur(30px);
            display: flex;
            justify-content: center;
            align-items: center;
            opacity: 0;
            pointer-events: none;
            transition: opacity 0.4s cubic-bezier(0.16, 1, 0.3, 1);
            z-index: 200;
        }
        .skip-modal.show {
            opacity: 1;
            pointer-events: auto;
        }
        .skip-modal-content {
            width: 400px;
            padding: 40px;
            background: var(--modal-bg);
            border: 1px solid var(--modal-border);
            border-radius: 28px;
            box-shadow: 0 32px 80px rgba(0, 0, 0, 0.3);
            display: flex;
            flex-direction: column;
            gap: 24px;
            text-align: center;
            transform: scale(0.92);
            transition: transform 0.4s cubic-bezier(0.16, 1, 0.3, 1);
        }
        .skip-modal.show .skip-modal-content {
            transform: scale(1);
        }
        .skip-modal-title {
            font-size: 20px;
            font-weight: 700;
            color: var(--text);
            letter-spacing: -0.01em;
        }
        .skip-modal-prompt {
            font-size: 14px;
            color: var(--text-dim);
            line-height: 1.6;
        }
        .skip-modal-prompt strong {
            color: #ff453a;
            background: rgba(255, 69, 58, 0.12);
            border: 1px solid rgba(255, 69, 58, 0.2);
            padding: 4px 10px;
            border-radius: 8px;
            font-family: var(--font-mono);
            font-weight: 600;
            margin: 6px 0;
            display: inline-block;
            box-shadow: 0 2px 8px rgba(255, 69, 58, 0.08);
        }
        .skip-input {
            width: 100%;
            box-sizing: border-box;
            background: var(--badge-bg);
            border: 1px solid var(--line);
            border-radius: 12px;
            color: var(--text);
            padding: 12px 16px;
            font-size: 16px;
            outline: none;
            text-align: center;
            transition: all 0.3s cubic-bezier(0.16, 1, 0.3, 1);
        }
        .skip-input:focus {
            border-color: rgba(255, 69, 58, 0.4);
            background: var(--badge-bg);
            box-shadow: 0 0 16px rgba(255, 69, 58, 0.1);
        }
        .skip-modal-actions {
            display: flex;
            gap: 14px;
        }
        .modal-btn {
            flex: 1;
            padding: 12px 20px;
            border-radius: 12px;
            font-size: 15px;
            font-weight: 600;
            cursor: pointer;
            border: none;
            transition: all 0.2s;
        }
        .btn-cancel {
            background: var(--badge-bg);
            color: var(--text);
            border: 1px solid var(--line);
        }
        .btn-cancel:hover {
            background: var(--card-hover);
            transform: translateY(-1px);
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
            box-shadow: 0 8px 20px rgba(255, 69, 58, 0.4);
        }
        .btn-confirm.active:hover {
            background: #ff3b30;
            transform: translateY(-1px);
        }

        /* 响应式适配 */
        @media (max-width: 768px) {
            .exercise-card {
                width: 90%;
                max-width: 500px;
                flex-direction: column;
                gap: 24px;
                padding: 24px;
            }
            .exercise-left {
                width: 100%;
                height: 180px;
            }
            .countdown {
                font-size: 100px;
            }
        }
    </style>
</head>
<body>

    <div class="skip-btn" id="skip-btn" onclick="openSkipModal()">Skip</div>

    <div class="container">
        <div class="timer-group">
            <div class="countdown" id="countdown">__COUNTDOWN__</div>
            <div class="title" id="title">__TITLE__</div>
        </div>
        <div class="hint" id="hint">__HINT__</div>

        <!-- 极简工位拉伸卡片 -->
        <div class="exercise-card" id="exercise-card">
            <div class="exercise-left">
                <div class="exercise-illustration" id="exercise-illustration"></div>
            </div>
            <div class="exercise-right">
                <div class="exercise-header">
                    <span class="exercise-badge" id="exercise-label">STRETCH & MOBILITY</span>
                    <h3 class="exercise-title" id="exercise-title">---</h3>
                </div>
                <div class="exercise-steps" id="exercise-steps"></div>
                <div class="exercise-footer">
                    <div class="exercise-reps-badge" id="exercise-reps">---</div>
                </div>
            </div>
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
            reps: currentLang === 'zh' ? "建议重复 10 次" : "10 Reps Recommended",
            steps: currentLang === 'zh' ? [
                { title: "双手平放", desc: "双手平放桌面，身体前倾，手肘伸直" },
                { title: "向下推撑", desc: "用力下压桌子，使上背部向后拱起" },
                { title: "放松还原", desc: "放松让胸口下沉，感受背部夹紧" }
            ] : [
                { title: "Place Hands", desc: "Hands flat on desk, lean forward, elbows straight" },
                { title: "Press Down", desc: "Press desk down, arching upper back" },
                { title: "Relax & Sink", desc: "Relax, let chest sink to squeeze back" }
            ],
            iconSvg: `<svg width="120" height="120" viewBox="0 0 80 80" fill="none">
                <!-- Desk -->
                <line x1="10" y1="60" x2="50" y2="60" stroke="var(--svg-bg)" stroke-width="3" stroke-linecap="round"/>
                <!-- Arms (straight) -->
                <line x1="30" y1="60" x2="45" y2="35" stroke="var(--svg-sub)" stroke-width="3" stroke-linecap="round"/>
                <!-- Spine (Arching/sinking) -->
                <path d="M 60 55 Q 56 42 45 35" stroke="var(--accent)" stroke-width="4" stroke-linecap="round" fill="none">
                    <animate attributeName="d"
                             values="M 60 55 Q 56 42 45 35; M 60 55 Q 65 42 45 35; M 60 55 Q 50 42 45 35; M 60 55 Q 56 42 45 35"
                             dur="5s" repeatCount="indefinite" />
                </path>
                <!-- Head -->
                <circle cx="40" cy="23" r="6" stroke="var(--svg-main)" stroke-width="3">
                    <animate attributeName="cx"
                             values="40; 38; 42; 40"
                             dur="5s" repeatCount="indefinite" />
                    <animate attributeName="cy"
                             values="23; 25; 22; 23"
                             dur="5s" repeatCount="indefinite" />
                </circle>
            </svg>`
        },
        {
            id: "seated-punch-plus",
            label: currentLang === 'zh' ? "稳定肌群激活 · 弹响改善" : "SHOULDER STABILIZERS · ALIGNMENT",
            title: currentLang === 'zh' ? "空气“V字盲推” (Seated Punch Plus)" : "Air V-Push (Seated Punch Plus)",
            reps: currentLang === 'zh' ? "建议重复 8-10 次" : "8-10 Reps Recommended",
            steps: currentLang === 'zh' ? [
                { title: "V字举手", desc: "挺直坐姿，双手向斜前方举起，大拇指朝上" },
                { title: "前推肩膀", desc: "手臂伸直，用肩膀力量将双手向前伸展1秒" },
                { title: "收回还原", desc: "收回肩膀，保持手臂抬起" }
            ] : [
                { title: "Raise V-Arms", desc: "Sit tall, raise arms diagonally, thumbs up" },
                { title: "Push Forward", desc: "Extend arms and shoulders forward, hold 1s" },
                { title: "Retract Back", desc: "Retract shoulders, keeping arms raised" }
            ],
            iconSvg: `<svg width="120" height="120" viewBox="0 0 80 80" fill="none">
                <!-- Chair backrest outline (aesthetic context) -->
                <path d="M 65 70 L 65 35" stroke="var(--svg-bg-dim)" stroke-width="3" stroke-linecap="round" />
                <!-- Hips/Seat -->
                <line x1="45" y1="60" x2="65" y2="60" stroke="var(--svg-bg-dim)" stroke-width="3" stroke-linecap="round" />
                <!-- Spine (sitting tall) -->
                <line x1="55" y1="60" x2="55" y2="35" stroke="var(--svg-sub)" stroke-width="3" stroke-linecap="round">
                    <animate attributeName="x2" values="55; 51; 57; 55" dur="4s" repeatCount="indefinite" />
                </line>
                <!-- Arm extending (V-push forward/back) -->
                <line x1="55" y1="35" x2="25" y2="25" stroke="var(--accent)" stroke-width="4" stroke-linecap="round">
                    <animate attributeName="x1" values="55; 51; 57; 55" dur="4s" repeatCount="indefinite" />
                    <animate attributeName="x2" values="25; 18; 29; 25" dur="4s" repeatCount="indefinite" />
                </line>
                <!-- Head -->
                <circle cx="55" cy="21" r="6" stroke="var(--svg-main)" stroke-width="3">
                    <animate attributeName="cx" values="55; 52; 56; 55" dur="4s" repeatCount="indefinite" />
                </circle>
                <!-- Arrow showing motion direction -->
                <g>
                    <path d="M 25 15 L 17 15 M 17 15 L 21 11 M 17 15 L 21 19" stroke="var(--accent)" stroke-width="2" stroke-linecap="round" />
                    <animateTransform attributeName="transform" type="translate" values="0,0; -6,0; 2,0; 0,0" dur="4s" repeatCount="indefinite" />
                </g>
            </svg>`
        },
        {
            id: "chair-depressions",
            label: currentLang === 'zh' ? "下斜方肌增肌 · 消除耸肩" : "LOWER TRAPS · DEFUSE TECH NECK",
            title: currentLang === 'zh' ? "办公椅“反向撑体” (Chair Depressions)" : "Chair Depressions",
            reps: currentLang === 'zh' ? "每次 3 秒 · 建议重复 8 次" : "3s Hold · 8 Reps Recommended",
            steps: currentLang === 'zh' ? [
                { title: "双手撑扶", desc: "双手撑在扶手或椅面边缘，手臂伸直" },
                { title: "用力下沉", desc: "用力下压，肩膀下沉，脖子拉长" },
                { title: "屁股悬空", desc: "可选：屁股微微悬空，保持3秒" }
            ] : [
                { title: "Grip Armrests", desc: "Hands on armrests or seat edges, arms straight" },
                { title: "Press & Depress", desc: "Press down, drawing shoulders down, lengthening neck" },
                { title: "Hover Hips", desc: "Optional: lift hips slightly, hold 3s" }
            ],
            iconSvg: `<svg width="120" height="120" viewBox="0 0 80 80" fill="none">
                <!-- Chair Seat (fixed) -->
                <line x1="20" y1="62" x2="60" y2="62" stroke="var(--svg-bg-dim)" stroke-width="3" stroke-linecap="round" />
                <!-- Chair Armrests (fixed) -->
                <line x1="16" y1="48" x2="26" y2="48" stroke="var(--svg-bg)" stroke-width="3" stroke-linecap="round" />
                <line x1="54" y1="48" x2="64" y2="48" stroke="var(--svg-bg)" stroke-width="3" stroke-linecap="round" />

                <!-- Spine/Torso Center Line -->
                <line x1="40" y1="58" x2="40" y2="38" stroke="var(--svg-bg)" stroke-width="3">
                    <animate attributeName="y1" values="58; 52; 58" dur="4s" repeatCount="indefinite" />
                    <animate attributeName="y2" values="38; 32; 38" dur="4s" repeatCount="indefinite" />
                </line>

                <!-- Arms (connected to fixed hands at 21,48 and 59,48) -->
                <line x1="21" y1="48" x2="32" y2="38" stroke="var(--svg-sub)" stroke-width="3" stroke-linecap="round">
                    <animate attributeName="y2" values="38; 32; 38" dur="4s" repeatCount="indefinite" />
                </line>
                <line x1="59" y1="48" x2="48" y2="38" stroke="var(--svg-sub)" stroke-width="3" stroke-linecap="round">
                    <animate attributeName="y2" values="38; 32; 38" dur="4s" repeatCount="indefinite" />
                </line>

                <!-- Shoulders Line -->
                <line x1="32" y1="38" x2="48" y2="38" stroke="var(--svg-sub)" stroke-width="3" stroke-linecap="round">
                    <animate attributeName="y1" values="38; 32; 38" dur="4s" repeatCount="indefinite" />
                    <animate attributeName="y2" values="38; 32; 38" dur="4s" repeatCount="indefinite" />
                </line>

                <!-- Neck (Highlighting the elongation) -->
                <line x1="40" y1="38" x2="40" y2="26" stroke="var(--accent)" stroke-width="4" stroke-linecap="round">
                    <animate attributeName="y1" values="38; 32; 38" dur="4s" repeatCount="indefinite" />
                    <animate attributeName="y2" values="26; 16; 26" dur="4s" repeatCount="indefinite" />
                </line>

                <!-- Head -->
                <circle cx="40" cy="20" r="6" stroke="var(--svg-main)" stroke-width="3">
                    <animate attributeName="cy" values="20; 10; 20" dur="4s" repeatCount="indefinite" />
                </circle>

                <!-- Arrows indicating shoulders pressing down / body rising -->
                <g>
                    <path d="M 40 44 L 40 50 M 40 50 L 37 47 M 40 50 L 43 47" stroke="var(--accent)" stroke-width="2" stroke-linecap="round" />
                    <animateTransform attributeName="transform" type="translate" values="0,0; 0,4; 0,0" dur="4s" repeatCount="indefinite" />
                </g>
            </svg>`
        }
    ];

    // 随机选择动作并渲染
    const randomIdx = Math.floor(Math.random() * exercises.length);
    const ex = exercises[randomIdx];

    // 设置动态主题颜色
    let accentColor, accentRgb;
    if (ex.id === 'desk-plus') {
        accentColor = '#00d2ff';
        accentRgb = '0, 210, 255';
    } else if (ex.id === 'seated-punch-plus') {
        accentColor = '#ff5e62';
        accentRgb = '255, 94, 98';
    } else {
        accentColor = '#00ff87';
        accentRgb = '0, 255, 135';
    }
    document.documentElement.style.setProperty('--accent', accentColor);
    document.documentElement.style.setProperty('--accent-rgb', accentRgb);

    document.getElementById('exercise-illustration').innerHTML = ex.iconSvg;
    document.getElementById('exercise-label').textContent = ex.label;
    document.getElementById('exercise-title').textContent = ex.title;
    document.getElementById('exercise-reps').textContent = ex.reps;

    const stepsHtml = ex.steps.map((step, idx) => `
        <div class="exercise-step">
            <div class="step-number">${idx + 1}</div>
            <div class="step-content">
                <div class="step-title">${step.title}</div>
                <div class="step-desc">${step.desc}</div>
            </div>
        </div>
    `).join('');
    document.getElementById('exercise-steps').innerHTML = stepsHtml;

    // 跳过确认弹窗逻辑
    const targetText = currentLang === 'zh' ? '紧急情况' : 'Emergency';
    
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
                window.confirmSkip();
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

fn build_countdown_html(
    title: &str,
    countdown: &str,
    hint: &str,
    skip_enabled: bool,
    lang: &str,
    theme: crate::macos::config::Theme,
) -> String {
    let mut html = COUNTDOWN_HTML_TEMPLATE.replace("__TITLE__", &escape_html(title));
    html = html.replace("__TITLE__", &escape_html(title));
    html = html.replace("__COUNTDOWN__", &escape_html(countdown));
    html = html.replace("__HINT__", &escape_html(hint));
    html = html.replace(
        "__SKIP_ENABLED__",
        if skip_enabled { "true" } else { "false" },
    );
    html = html.replace("__LANG__", lang);
    html.replace(
        "__THEME_CLASS__",
        match theme {
            crate::macos::config::Theme::Dark => "dark",
            crate::macos::config::Theme::Light => "light",
        },
    )
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
    let (texts, allow_skip_break, theme) = with_state(|state| {
        let language = state.config.effective_language();
        (
            Texts::new(language),
            state.config.allow_skip_break,
            state.config.theme,
        )
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

    let background = match theme {
        crate::macos::config::Theme::Dark => {
            NSColor::colorWithSRGBRed_green_blue_alpha(0.043, 0.043, 0.059, 1.0)
        }
        crate::macos::config::Theme::Light => {
            NSColor::colorWithSRGBRed_green_blue_alpha(0.96, 0.96, 0.97, 1.0)
        }
    };
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
        theme,
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
