//! egui 다크 팔레트 — 탐색기 고정 다크 스타일 (FR-21).
//!
//! 색 값은 현행 Win32 판(`app::theme`)과 **같은 화면색**을 내도록 그대로 옮긴 것이다.
//! 타입만 다르다: Win32는 `COLORREF`(0x00BBGGRR 바이트 순서)라 egui에서 그대로 쓸 수 없어
//! `Color32`로 재정의한다. 전환 UI는 없다(PRD Out of Scope).
use eframe::egui;

/// 창 배경·스플리터 틈
pub const WINDOW_BG: egui::Color32 = egui::Color32::from_rgb(0x1B, 0x1B, 0x1B);
/// 목록·트리·입력 컨트롤 배경
pub const SURFACE_BG: egui::Color32 = egui::Color32::from_rgb(0x1E, 0x1E, 0x1E);
/// 기본 글자색
pub const TEXT: egui::Color32 = egui::Color32::from_rgb(0xE8, 0xE8, 0xE8);
/// 목록 헤더 배경
pub const HEADER_BG: egui::Color32 = egui::Color32::from_rgb(0x25, 0x25, 0x25);
/// 목록 헤더 글자
pub const HEADER_TEXT: egui::Color32 = egui::Color32::from_rgb(0xC8, 0xC8, 0xC8);
/// 트리 연결선
pub const TREE_LINE: egui::Color32 = egui::Color32::from_rgb(0x45, 0x45, 0x45);
/// 버튼·컨트롤 기본 배경
pub const CONTROL_BG: egui::Color32 = egui::Color32::from_rgb(0x2A, 0x2A, 0x2A);
/// 버튼 hover 배경
pub const CONTROL_HOT: egui::Color32 = egui::Color32::from_rgb(0x38, 0x38, 0x38);
/// 버튼 눌림·선택 배경
pub const CONTROL_ACTIVE: egui::Color32 = egui::Color32::from_rgb(0x45, 0x45, 0x45);
/// 비활성 글자색
pub const TEXT_DIM: egui::Color32 = egui::Color32::from_rgb(0x6A, 0x6A, 0x6A);
/// 타이틀바 닫기 버튼 hover 배경 — Windows 11 캡션 닫기 버튼과 같은 빨강 (FR-22)
pub const CLOSE_HOT: egui::Color32 = egui::Color32::from_rgb(0xC4, 0x2B, 0x1C);

/// 고정 다크 팔레트를 egui 컨텍스트에 적용한다.
/// egui 기본 다크를 토대로, 위 상수로 현행 앱과 같은 색을 덮어쓴다.
pub fn apply_dark(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = WINDOW_BG;
    visuals.window_fill = SURFACE_BG;
    visuals.extreme_bg_color = SURFACE_BG;
    visuals.faint_bg_color = HEADER_BG;
    visuals.override_text_color = Some(TEXT);

    // 위젯 상태별 배경 — 버튼·입력창이 현행 오너드로우와 같은 명도 단계를 갖게 한다
    visuals.widgets.noninteractive.bg_fill = SURFACE_BG;
    visuals.widgets.inactive.bg_fill = CONTROL_BG;
    visuals.widgets.hovered.bg_fill = CONTROL_HOT;
    visuals.widgets.active.bg_fill = CONTROL_ACTIVE;
    visuals.selection.bg_fill = CONTROL_ACTIVE;

    ctx.set_visuals(visuals);
}
