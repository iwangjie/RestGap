//! UI 模块
//!
//! 包含所有用户界面相关的子模块。

pub mod countdown;
pub mod dialogs;
pub mod status_bar;

pub use countdown::{finish_countdown, show_countdown_window, update_countdown};
pub use dialogs::{open_settings_dialog, show_about_dialog};
pub use status_bar::{
    refresh_header_title, refresh_menu_info, refresh_status_title, set_rest_now_enabled,
    setup_status_item, target_anyobject,
};
