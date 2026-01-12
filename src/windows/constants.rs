//! 应用常量定义

pub const APP_NAME_DISPLAY: &str = "息间（RestGap）";

// 窗口类名
pub const MAIN_WINDOW_CLASS: &str = "RestGapMainWindow";
pub const COUNTDOWN_WINDOW_CLASS: &str = "RestGapCountdownWindow";

// 定时器 ID
pub const PHASE_TIMER_ID: usize = 1;
pub const COUNTDOWN_TIMER_ID: usize = 2;

// 托盘图标 ID
pub const TRAY_ICON_ID: u32 = 1;

// 自定义消息
pub const WM_TRAY_CALLBACK: u32 = 0x0400 + 1; // WM_USER + 1

// 菜单项 ID
pub const ID_MENU_HEADER: u16 = 100;
pub const ID_MENU_NEXT_BREAK: u16 = 101;
pub const ID_MENU_REMAINING: u16 = 102;
pub const ID_MENU_REST_NOW: u16 = 103;
pub const ID_MENU_SETTINGS: u16 = 104;
pub const ID_MENU_ABOUT: u16 = 105;
pub const ID_MENU_QUIT: u16 = 106;
pub const ID_MENU_LANGUAGE_HEADER: u16 = 107;
pub const ID_MENU_LANGUAGE_AUTO: u16 = 108;
pub const ID_MENU_LANGUAGE_EN: u16 = 109;
pub const ID_MENU_LANGUAGE_ZH: u16 = 110;
