//! egui(eframe/glow) UI 계층 — 화면 구성·입력 처리 전부.
//!
//! 이 모듈은 `app`(레이아웃 트리·워크스페이스·세션)·`panel`(탭 모델·히스토리·정렬)의
//! 순수 로직과 `fs`(열거·감시·아이콘·셸)를 조립해 그린다. 단방향이며 하위 모듈은 `ui`를 모른다.
pub mod address_bar;
pub mod app;
pub mod file_list;
pub mod icon_tex;
pub mod panel;
pub mod shell_host;
pub mod sidebar;
pub mod splitter;
pub mod tabs;
pub mod theme;
pub mod tree;
