//! 패널 — 탐색 단위 (파일 목록·주소창·탭·트리)
pub mod address_bar;
pub mod file_list;
pub mod history;
// plan Files 명세(src/panel/panel.rs)를 유지 — 모듈명 중복 lint만 허용
#[allow(clippy::module_inception)]
pub mod panel;
pub mod tabs;
