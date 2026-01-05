#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("息间（RestGap）仅支持 macOS。");
}

#[cfg(target_os = "macos")]
mod macos {
    #![allow(deprecated)]

    const APP_NAME_ZH: &str = "息间";
    const APP_NAME_DISPLAY: &str = "息间（RestGap）";

    use std::cell::RefCell;
    use std::process::Command;
    use std::time::Duration;
    use std::time::{Instant, SystemTime, UNIX_EPOCH};

    use objc2::ffi::NSInteger;
    use objc2::rc::{Retained, autoreleasepool};
    use objc2::runtime::{AnyObject, NSObject, ProtocolObject};
    use objc2::{ClassType, MainThreadMarker, MainThreadOnly, define_class, msg_send, sel};

    use objc2_app_kit::{
        NSAlert, NSAlertFirstButtonReturn, NSAlertSecondButtonReturn, NSApplication,
        NSApplicationActivationPolicy, NSApplicationDelegate, NSBackingStoreType, NSColor, NSFont,
        NSMenu, NSMenuDelegate, NSMenuItem, NSScreen, NSStatusBar, NSStatusItem,
        NSStatusWindowLevel, NSTextField, NSVariableStatusItemLength, NSView, NSWindow,
        NSWindowCollectionBehavior, NSWindowStyleMask,
    };
    use objc2_core_foundation::CGFloat;
    use objc2_foundation::{
        NSNotification, NSObjectProtocol, NSPoint, NSRect, NSSize, NSString, NSTimer,
        NSUserDefaults,
    };

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum Phase {
        Working,
        Breaking,
    }

    #[derive(Clone, Debug)]
    struct Config {
        interval_minutes: u64,
        break_seconds: u64,
    }

    impl Config {
        const DEFAULT_INTERVAL_MINUTES: u64 = 30;
        const DEFAULT_BREAK_SECONDS: u64 = 120;

        const MIN_INTERVAL_MINUTES: u64 = 1;
        const MAX_INTERVAL_MINUTES: u64 = 240;

        const MIN_BREAK_SECONDS: u64 = 5;
        const MAX_BREAK_SECONDS: u64 = 3600;

        const KEY_INTERVAL_MINUTES: &'static str = "restgap.interval_minutes";
        const KEY_BREAK_SECONDS: &'static str = "restgap.break_seconds";

        const LEGACY_KEY_INTERVAL_MINUTES: &'static str = "restp.interval_minutes";
        const LEGACY_KEY_BREAK_SECONDS: &'static str = "restp.break_seconds";

        fn load() -> Self {
            let defaults = NSUserDefaults::standardUserDefaults();

            let interval_key = NSString::from_str(Self::KEY_INTERVAL_MINUTES);
            let break_key = NSString::from_str(Self::KEY_BREAK_SECONDS);

            let legacy_interval_key = NSString::from_str(Self::LEGACY_KEY_INTERVAL_MINUTES);
            let legacy_break_key = NSString::from_str(Self::LEGACY_KEY_BREAK_SECONDS);

            let interval_raw = defaults.integerForKey(&interval_key);
            let break_raw = defaults.integerForKey(&break_key);

            let interval_raw = if interval_raw <= 0 {
                defaults.integerForKey(&legacy_interval_key)
            } else {
                interval_raw
            };
            let break_raw = if break_raw <= 0 {
                defaults.integerForKey(&legacy_break_key)
            } else {
                break_raw
            };

            let interval_minutes = if interval_raw <= 0 {
                Self::DEFAULT_INTERVAL_MINUTES
            } else {
                interval_raw as u64
            };

            let break_seconds = if break_raw <= 0 {
                Self::DEFAULT_BREAK_SECONDS
            } else {
                break_raw as u64
            };

            Self {
                interval_minutes: clamp_u64(
                    interval_minutes,
                    Self::MIN_INTERVAL_MINUTES,
                    Self::MAX_INTERVAL_MINUTES,
                ),
                break_seconds: clamp_u64(
                    break_seconds,
                    Self::MIN_BREAK_SECONDS,
                    Self::MAX_BREAK_SECONDS,
                ),
            }
        }

        fn save(&self) {
            let defaults = NSUserDefaults::standardUserDefaults();
            let interval_key = NSString::from_str(Self::KEY_INTERVAL_MINUTES);
            let break_key = NSString::from_str(Self::KEY_BREAK_SECONDS);

            defaults.setInteger_forKey(self.interval_minutes as NSInteger, &interval_key);
            defaults.setInteger_forKey(self.break_seconds as NSInteger, &break_key);
        }

        fn work_interval(&self) -> Duration {
            Duration::from_secs(self.interval_minutes.saturating_mul(60))
        }

        fn break_duration(&self) -> Duration {
            Duration::from_secs(self.break_seconds)
        }

        fn work_tolerance(&self) -> Duration {
            // 允许系统合并计时器唤醒，降低占用（精度不追求极致）。
            let secs = (self.work_interval().as_secs_f64() * 0.10).min(120.0);
            Duration::from_secs_f64(secs.max(1.0))
        }

        fn break_tolerance(&self) -> Duration {
            let secs = (self.break_duration().as_secs_f64() * 0.10).min(5.0);
            Duration::from_secs_f64(secs.max(0.5))
        }
    }

    #[derive(Debug)]
    struct AppState {
        config: Config,
        phase: Phase,
        phase_deadline_mono: Option<Instant>,
        phase_deadline_wall: Option<SystemTime>,
        timer: Option<Retained<NSTimer>>,
        status_item: Option<Retained<NSStatusItem>>,
        header_item: Option<Retained<NSMenuItem>>,
        rest_now_item: Option<Retained<NSMenuItem>>,
        next_break_item: Option<Retained<NSMenuItem>>,
        remaining_break_item: Option<Retained<NSMenuItem>>,
        // Countdown window state
        countdown_window: Option<Retained<NSWindow>>,
        countdown_label: Option<Retained<NSTextField>>,
        countdown_timer: Option<Retained<NSTimer>>,
        countdown_end_time: Option<Instant>,
    }

    impl AppState {
        fn new(config: Config) -> Self {
            Self {
                config,
                phase: Phase::Working,
                phase_deadline_mono: None,
                phase_deadline_wall: None,
                timer: None,
                status_item: None,
                header_item: None,
                rest_now_item: None,
                next_break_item: None,
                remaining_break_item: None,
                countdown_window: None,
                countdown_label: None,
                countdown_timer: None,
                countdown_end_time: None,
            }
        }
    }

    thread_local! {
        static STATE: RefCell<Option<AppState>> = const { RefCell::new(None) };
    }

    fn init_state(config: Config) {
        STATE.with(|cell| {
            *cell.borrow_mut() = Some(AppState::new(config));
        });
    }

    fn with_state<R>(f: impl FnOnce(&mut AppState) -> R) -> R {
        STATE.with(|cell| {
            let mut state = cell.borrow_mut();
            let state = state.as_mut().expect("STATE not initialized");
            f(state)
        })
    }

    fn with_state_ref<R>(f: impl FnOnce(&AppState) -> R) -> R {
        STATE.with(|cell| {
            let state = cell.borrow();
            let state = state.as_ref().expect("STATE not initialized");
            f(state)
        })
    }

    fn clamp_u64(v: u64, min: u64, max: u64) -> u64 {
        v.max(min).min(max)
    }

    fn target_anyobject(delegate: &RestGapDelegate) -> &AnyObject {
        delegate.as_super().as_super()
    }

    fn format_hhmm(t: SystemTime) -> String {
        let Ok(duration) = t.duration_since(UNIX_EPOCH) else {
            return "--:--".to_string();
        };

        let mut tm: libc::tm = unsafe { std::mem::zeroed() };
        let mut seconds: libc::time_t = duration.as_secs() as libc::time_t;
        let tm_ptr = unsafe { libc::localtime_r(&mut seconds, &mut tm) };
        if tm_ptr.is_null() {
            return "--:--".to_string();
        }

        let mut buf = [0u8; 6]; // "HH:MM\0"
        let fmt = b"%H:%M\0";
        let written =
            unsafe { libc::strftime(buf.as_mut_ptr().cast(), buf.len(), fmt.as_ptr().cast(), &tm) };
        if written == 0 {
            return "--:--".to_string();
        }
        String::from_utf8_lossy(&buf[..written]).into_owned()
    }

    fn approx_duration(d: Duration) -> String {
        let secs = d.as_secs();
        if secs >= 3600 {
            let hours = secs / 3600;
            let minutes = (secs % 3600) / 60;
            return format!("≈{}h{}m", hours, minutes);
        }
        if secs >= 600 {
            // >= 10m, round to 5m
            let minutes = ((secs + 150) / 300) * 5;
            return format!("≈{}m", minutes);
        }
        if secs >= 120 {
            // >= 2m, round to 1m
            let minutes = (secs + 30) / 60;
            return format!("≈{}m", minutes);
        }
        if secs >= 60 {
            return "≈1m".to_string();
        }
        // < 60s, round to 10s
        let rounded = ((secs + 5) / 10) * 10;
        format!("≈{}s", rounded.max(10))
    }

    fn refresh_status_title() {
        with_state(|state| {
            let Some(status_item) = state.status_item.as_ref() else {
                return;
            };

            let title = match state.phase {
                Phase::Working => {
                    let hm = state
                        .phase_deadline_wall
                        .map(format_hhmm)
                        .unwrap_or_else(|| "--:--".to_string());
                    format!("⏰ {}", hm)
                }
                Phase::Breaking => {
                    let remaining = state
                        .phase_deadline_mono
                        .and_then(|t| t.checked_duration_since(Instant::now()))
                        .unwrap_or(Duration::from_secs(0));
                    format!("☕ {}", approx_duration(remaining))
                }
            };
            let ns_title = NSString::from_str(&title);
            status_item.setTitle(Some(&ns_title));
        });
    }

    fn set_rest_now_enabled(enabled: bool) {
        with_state(|state| {
            let Some(item) = state.rest_now_item.as_ref() else {
                return;
            };
            item.setEnabled(enabled);
        });
    }

    fn refresh_menu_info() {
        let now = Instant::now();
        with_state(|state| {
            let Some(next_item) = state.next_break_item.as_ref() else {
                return;
            };
            let Some(remaining_item) = state.remaining_break_item.as_ref() else {
                return;
            };

            let phase_deadline_mono = state.phase_deadline_mono;
            let phase_deadline_wall = state.phase_deadline_wall;

            let (next_break_in, next_break_wall) = match state.phase {
                Phase::Working => {
                    let in_dur = phase_deadline_mono
                        .and_then(|t| t.checked_duration_since(now))
                        .unwrap_or(Duration::from_secs(0));
                    (in_dur, phase_deadline_wall)
                }
                Phase::Breaking => {
                    let remaining_break = phase_deadline_mono
                        .and_then(|t| t.checked_duration_since(now))
                        .unwrap_or(Duration::from_secs(0));
                    let in_dur = remaining_break + state.config.work_interval();
                    let wall = phase_deadline_wall
                        .and_then(|t| t.checked_add(state.config.work_interval()))
                        .or(phase_deadline_wall);
                    (in_dur, wall)
                }
            };

            let next_hm = next_break_wall
                .map(format_hhmm)
                .unwrap_or_else(|| "--:--".to_string());
            let next_title = format!(
                "下次休息：{}（{}）",
                next_hm,
                approx_duration(next_break_in)
            );
            next_item.setTitle(&NSString::from_str(&next_title));

            match state.phase {
                Phase::Working => {
                    remaining_item.setTitle(&NSString::from_str("休息剩余：—"));
                }
                Phase::Breaking => {
                    let remaining = phase_deadline_mono
                        .and_then(|t| t.checked_duration_since(now))
                        .unwrap_or(Duration::from_secs(0));
                    let end_hm = phase_deadline_wall
                        .map(format_hhmm)
                        .unwrap_or_else(|| "--:--".to_string());
                    let title =
                        format!("休息剩余：{}（至 {}）", approx_duration(remaining), end_hm);
                    remaining_item.setTitle(&NSString::from_str(&title));
                }
            }
        });
    }

    fn refresh_header_title() {
        with_state(|state| {
            let Some(item) = state.header_item.as_ref() else {
                return;
            };
            let title = format!(
                "{APP_NAME_ZH} · 每 {} 分钟休息 {} 秒",
                state.config.interval_minutes, state.config.break_seconds
            );
            item.setTitle(&NSString::from_str(&title));
        });
    }

    fn schedule_phase(delegate: &RestGapDelegate, phase: Phase) {
        let target = target_anyobject(delegate);

        let (seconds, tolerance) = with_state(|state| {
            if let Some(timer) = state.timer.take() {
                timer.invalidate();
            }

            state.phase = phase;
            let (duration, tolerance) = match phase {
                Phase::Working => (state.config.work_interval(), state.config.work_tolerance()),
                Phase::Breaking => (
                    state.config.break_duration(),
                    state.config.break_tolerance(),
                ),
            };

            state.phase_deadline_mono = Some(Instant::now() + duration);
            state.phase_deadline_wall = Some(SystemTime::now() + duration);

            (duration.as_secs_f64(), tolerance.as_secs_f64())
        });

        let timer = unsafe {
            NSTimer::scheduledTimerWithTimeInterval_target_selector_userInfo_repeats(
                seconds,
                target,
                sel!(timerFired:),
                None,
                false,
            )
        };
        timer.setTolerance(tolerance);

        with_state(|state| {
            state.timer = Some(timer);
        });

        refresh_status_title();
        refresh_header_title();
        set_rest_now_enabled(phase == Phase::Working);
        refresh_menu_info();
    }

    #[derive(Clone, Copy, Debug)]
    enum NotifyEvent {
        BreakStart,
        BreakEnd,
    }

    fn play_sound(sound_name: &str) {
        let path = format!("/System/Library/Sounds/{}.aiff", sound_name);
        let _ = Command::new("afplay").arg(&path).spawn();
    }

    fn show_countdown_window(delegate: &RestGapDelegate, seconds: u64, play_start_sound: bool) {
        let mtm = delegate.mtm();

        // 关闭已存在的倒计时窗口
        close_countdown_window();

        // 播放开始声音
        if play_start_sound {
            play_sound("Glass");
        }

        // 获取主屏幕尺寸
        let screen_frame = NSScreen::mainScreen(mtm)
            .map(|s| s.frame())
            .unwrap_or(NSRect::new(
                NSPoint::new(0.0, 0.0),
                NSSize::new(1920.0, 1080.0),
            ));

        // 窗口占满屏幕
        let window_width: CGFloat = screen_frame.size.width;
        let window_height: CGFloat = screen_frame.size.height;
        let frame = screen_frame;

        // 创建窗口 - 使用 Borderless 样式隐藏标题栏
        let style = NSWindowStyleMask::Borderless;
        let window = unsafe {
            NSWindow::initWithContentRect_styleMask_backing_defer(
                NSWindow::alloc(mtm),
                frame,
                style,
                NSBackingStoreType::Buffered,
                false,
            )
        };

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
            NSSize::new(window_width - padding * 2.0, 50.0),
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
            NSSize::new(window_width - padding * 2.0, 100.0),
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
            NSSize::new(window_width - padding * 2.0, 40.0),
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

        // 显示窗口
        window.makeKeyAndOrderFront(None);

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
            state.countdown_window = Some(window);
            state.countdown_label = Some(countdown_label);
            state.countdown_timer = Some(timer);
            state.countdown_end_time = Some(end_time);
        });
    }

    fn format_countdown(seconds: u64) -> String {
        let mins = seconds / 60;
        let secs = seconds % 60;
        format!("{:02}:{:02}", mins, secs)
    }

    fn update_countdown() -> bool {
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

    fn close_countdown_window() {
        with_state(|state| {
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
        });
    }

    fn finish_countdown() {
        close_countdown_window();
        play_sound("Tink");
    }

    fn notify(event: NotifyEvent, config: &Config, delegate: &RestGapDelegate) {
        match event {
            NotifyEvent::BreakStart => {
                // 休息开始: 弹出原生倒计时窗口
                show_countdown_window(delegate, config.break_seconds, true);
            }
            NotifyEvent::BreakEnd => {
                // 休息结束: 关闭倒计时窗口并显示通知
                finish_countdown();
            }
        }
    }
    fn transition_on_timer(delegate: &RestGapDelegate) {
        let (next_phase, event, config) = with_state(|state| {
            state.timer.take();
            let config = state.config.clone();
            match state.phase {
                Phase::Working => (Phase::Breaking, NotifyEvent::BreakStart, config),
                Phase::Breaking => (Phase::Working, NotifyEvent::BreakEnd, config),
            }
        });

        notify(event, &config, delegate);
        schedule_phase(delegate, next_phase);
    }

    fn start_break_now(delegate: &RestGapDelegate) {
        let (should_start, config) = with_state(|state| {
            let config = state.config.clone();
            let should_start = state.phase == Phase::Working;
            (should_start, config)
        });

        if should_start {
            notify(NotifyEvent::BreakStart, &config, delegate);
            schedule_phase(delegate, Phase::Breaking);
        }
    }

    fn setup_status_item(delegate: &RestGapDelegate) {
        let mtm = delegate.mtm();
        let status_item =
            NSStatusBar::systemStatusBar().statusItemWithLength(NSVariableStatusItemLength);

        let menu = NSMenu::new(mtm);
        menu.setAutoenablesItems(false);
        menu.setDelegate(Some(ProtocolObject::<dyn NSMenuDelegate>::from_ref(
            delegate,
        )));

        let header = format!(
            "{APP_NAME_ZH} · 每 {} 分钟休息 {} 秒",
            with_state_ref(|s| s.config.interval_minutes),
            with_state_ref(|s| s.config.break_seconds),
        );
        let header_item = NSMenuItem::sectionHeaderWithTitle(&NSString::from_str(&header), mtm);
        menu.addItem(&header_item);

        let next_break_item = unsafe {
            menu.addItemWithTitle_action_keyEquivalent(
                &NSString::from_str("下次休息：--:--"),
                None,
                &NSString::from_str(""),
            )
        };
        next_break_item.setEnabled(false);

        let remaining_break_item = unsafe {
            menu.addItemWithTitle_action_keyEquivalent(
                &NSString::from_str("休息剩余：—"),
                None,
                &NSString::from_str(""),
            )
        };
        remaining_break_item.setEnabled(false);

        menu.addItem(&NSMenuItem::separatorItem(mtm));

        let rest_now_item = unsafe {
            menu.addItemWithTitle_action_keyEquivalent(
                &NSString::from_str("现在休息"),
                Some(sel!(restNow:)),
                &NSString::from_str(""),
            )
        };
        unsafe { rest_now_item.setTarget(Some(target_anyobject(delegate))) };

        let settings_item = unsafe {
            menu.addItemWithTitle_action_keyEquivalent(
                &NSString::from_str("配置"),
                Some(sel!(openSettings:)),
                &NSString::from_str(""),
            )
        };
        unsafe { settings_item.setTarget(Some(target_anyobject(delegate))) };

        let about_item = unsafe {
            menu.addItemWithTitle_action_keyEquivalent(
                &NSString::from_str(&format!("关于 {APP_NAME_ZH}")),
                Some(sel!(about:)),
                &NSString::from_str(""),
            )
        };
        unsafe { about_item.setTarget(Some(target_anyobject(delegate))) };

        menu.addItem(&NSMenuItem::separatorItem(mtm));

        let quit_item = unsafe {
            menu.addItemWithTitle_action_keyEquivalent(
                &NSString::from_str("退出"),
                Some(sel!(quit:)),
                &NSString::from_str(""),
            )
        };
        unsafe { quit_item.setTarget(Some(target_anyobject(delegate))) };

        status_item.setMenu(Some(&menu));

        with_state(|state| {
            state.status_item = Some(status_item);
            state.header_item = Some(header_item);
            state.rest_now_item = Some(rest_now_item);
            state.next_break_item = Some(next_break_item);
            state.remaining_break_item = Some(remaining_break_item);
        });
    }

    fn show_invalid_settings_alert(delegate: &RestGapDelegate) {
        let mtm = delegate.mtm();
        let alert: Retained<NSAlert> = unsafe { msg_send![NSAlert::alloc(mtm), init] };
        alert.setMessageText(&NSString::from_str("配置无效"));
        alert.setInformativeText(&NSString::from_str(
            "请输入有效的数字：每 N 分钟休息 N 秒。",
        ));
        let _ = alert.addButtonWithTitle(&NSString::from_str("好"));
        let _ = alert.runModal();
    }

    fn open_settings_dialog(delegate: &RestGapDelegate) {
        let mtm = delegate.mtm();

        let current = with_state_ref(|s| s.config.clone());

        let alert: Retained<NSAlert> = unsafe { msg_send![NSAlert::alloc(mtm), init] };
        alert.setMessageText(&NSString::from_str("配置"));
        alert.setInformativeText(&NSString::from_str("保存后将从现在开始重新计时。"));
        let _ = alert.addButtonWithTitle(&NSString::from_str("保存"));
        let _ = alert.addButtonWithTitle(&NSString::from_str("取消"));

        let accessory_frame = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(320.0, 78.0));
        let accessory = NSView::initWithFrame(NSView::alloc(mtm), accessory_frame);

        let label_style = |label: &NSTextField| {
            label.setBezeled(false);
            label.setDrawsBackground(false);
            label.setEditable(false);
            label.setSelectable(false);
        };

        let minutes_label_frame = NSRect::new(NSPoint::new(0.0, 44.0), NSSize::new(160.0, 20.0));
        let minutes_label =
            NSTextField::initWithFrame(NSTextField::alloc(mtm), minutes_label_frame);
        minutes_label.setStringValue(&NSString::from_str("每 N 分钟休息："));
        label_style(&minutes_label);

        let minutes_input_frame = NSRect::new(NSPoint::new(170.0, 40.0), NSSize::new(130.0, 24.0));
        let minutes_input =
            NSTextField::initWithFrame(NSTextField::alloc(mtm), minutes_input_frame);
        minutes_input.setStringValue(&NSString::from_str(&current.interval_minutes.to_string()));

        let seconds_label_frame = NSRect::new(NSPoint::new(0.0, 10.0), NSSize::new(160.0, 20.0));
        let seconds_label =
            NSTextField::initWithFrame(NSTextField::alloc(mtm), seconds_label_frame);
        seconds_label.setStringValue(&NSString::from_str("休息 N 秒："));
        label_style(&seconds_label);

        let seconds_input_frame = NSRect::new(NSPoint::new(170.0, 6.0), NSSize::new(130.0, 24.0));
        let seconds_input =
            NSTextField::initWithFrame(NSTextField::alloc(mtm), seconds_input_frame);
        seconds_input.setStringValue(&NSString::from_str(&current.break_seconds.to_string()));

        accessory.addSubview(&minutes_label);
        accessory.addSubview(&minutes_input);
        accessory.addSubview(&seconds_label);
        accessory.addSubview(&seconds_input);

        alert.setAccessoryView(Some(&accessory));

        let resp = alert.runModal();
        if resp != NSAlertFirstButtonReturn {
            return;
        }

        let interval_minutes = minutes_input
            .stringValue()
            .to_string()
            .trim()
            .parse::<u64>()
            .ok();
        let break_seconds = seconds_input
            .stringValue()
            .to_string()
            .trim()
            .parse::<u64>()
            .ok();

        let (Some(interval_minutes), Some(break_seconds)) = (interval_minutes, break_seconds)
        else {
            show_invalid_settings_alert(delegate);
            return;
        };

        if interval_minutes == 0 || break_seconds == 0 {
            show_invalid_settings_alert(delegate);
            return;
        }

        let new_config = Config {
            interval_minutes: clamp_u64(
                interval_minutes,
                Config::MIN_INTERVAL_MINUTES,
                Config::MAX_INTERVAL_MINUTES,
            ),
            break_seconds: clamp_u64(
                break_seconds,
                Config::MIN_BREAK_SECONDS,
                Config::MAX_BREAK_SECONDS,
            ),
        };
        new_config.save();

        let phase = with_state(|state| {
            state.config = new_config.clone();
            state.phase
        });

        // 从现在开始重新计时（避免显示与实际触发不一致）。
        schedule_phase(delegate, phase);

        // 若当前正在休息，则同步更新倒计时窗口（不播放开始声音）。
        if phase == Phase::Breaking {
            show_countdown_window(delegate, new_config.break_seconds, false);
        }
    }

    fn show_about_dialog(delegate: &RestGapDelegate) {
        let mtm = delegate.mtm();
        let alert: Retained<NSAlert> = unsafe { msg_send![NSAlert::alloc(mtm), init] };
        alert.setMessageText(&NSString::from_str(APP_NAME_DISPLAY));
        alert.setInformativeText(&NSString::from_str(&format!(
            "版本：{}\nmacOS 菜单栏休息提醒（事件驱动 / 非轮询）。",
            env!("CARGO_PKG_VERSION")
        )));

        let _ = alert.addButtonWithTitle(&NSString::from_str("好"));
        let _ = alert.addButtonWithTitle(&NSString::from_str("访问主页"));

        let resp = alert.runModal();
        if resp == NSAlertSecondButtonReturn {
            let _ = Command::new("open")
                .arg("https://github.com/iwangjie")
                .spawn();
        }
    }

    define_class!(
        #[unsafe(super(NSObject))]
        #[thread_kind = MainThreadOnly]
        struct RestGapDelegate;

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

    pub fn run() {
        autoreleasepool(|_| {
            let config = Config::load();
            init_state(config);

            let mtm = MainThreadMarker::new().expect("must be on the main thread");
            let app = NSApplication::sharedApplication(mtm);

            // 只显示菜单栏图标，不显示 Dock 图标
            let _ = app.setActivationPolicy(NSApplicationActivationPolicy::Accessory);

            let delegate: Retained<RestGapDelegate> =
                unsafe { msg_send![RestGapDelegate::alloc(mtm), init] };
            let delegate_ref: &RestGapDelegate = &*delegate;
            app.setDelegate(Some(ProtocolObject::<dyn NSApplicationDelegate>::from_ref(
                delegate_ref,
            )));

            // NSApplication 的 delegate 是 weak 引用；这里必须持有它直到 app 退出。
            let _keep_delegate_alive = delegate;

            app.run();
        });
    }
}

#[cfg(target_os = "macos")]
fn main() {
    macos::run();
}
