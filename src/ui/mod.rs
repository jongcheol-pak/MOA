//! egui(eframe/glow) UI 계층 — 화면 구성·입력 처리 전부.
//!
//! 이 모듈은 `app`(레이아웃 트리·워크스페이스·세션)·`panel`(탭 모델·히스토리·정렬)의
//! 순수 로직과 `fs`(열거·감시·아이콘·셸)를 조립해 그린다. 단방향이며 하위 모듈은 `ui`를 모른다.
pub mod address_bar;
pub mod app;
pub mod dock;
pub mod file_list;
pub mod icon_tex;
pub mod list_common;
pub mod list_details;
pub mod list_grid;
pub mod log_panel;
pub mod menu;
pub mod panel;
pub mod queue_panel;
pub mod remote_menu;
pub mod remote_states;
pub mod session;
pub mod shell_host;
pub mod sidebar;
pub mod site_dropdown;
pub mod site_manager;
pub mod splitter;
pub mod status_bar;
pub mod tabs;
pub mod theme;
pub mod titlebar;
pub mod toast;
pub mod tree;
pub mod view_mode;
pub mod widgets;
pub mod window_start;
