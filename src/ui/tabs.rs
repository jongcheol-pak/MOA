//! 탭 스트립 — 패널별 독립 탭 (FR-3)과 분할 버튼 (FR-1 진입점).
//!
//! 상태는 `panel::tabs::TabsModel`(순수 모델)이 갖고, 이 파일은 그리기와 입력만 한다.
use crate::panel::tabs::{TabsModel, tab_title};
use crate::ui::menu::{self, Command, SplitTo};
use crate::ui::theme;
use eframe::egui;

/// 탭·분할 버튼의 높이 — 스트립 한 줄의 기준
const STRIP_HEIGHT: f32 = 22.0;

/// 분할 버튼 아이콘(사각형) 한 변 — 글리프가 아니라 직접 그린다 (plan D8)
const SPLIT_ICON_SIZE: f32 = 12.0;

/// 분할 버튼 아이콘 선 두께
const SPLIT_ICON_STROKE: f32 = 1.0;

/// 탭 스트립이 상위(패널)에 돌려주는 조작.
/// 탭 조작과 분할 요청은 서로 독립이라 한 프레임에 함께 나올 수 있다
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct TabStripOutcome {
    pub tab: Option<TabAction>,
    /// 분할 버튼 메뉴에서 고른 방향 — 대상은 **이 스트립이 속한 패널**이다 (plan D3)
    pub split: Option<SplitTo>,
}

/// 탭 스트립이 상위(패널)에 돌려주는 조작
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TabAction {
    Switch(usize),
    Close(usize),
    New,
}

/// 탭 스트립을 그리고 이번 프레임의 조작을 반환한다.
///
/// 탭이 많으면 왼쪽 영역이 가로로 스크롤되고, **분할 버튼은 오른쪽 끝에 고정**된다 —
/// 버튼까지 스크롤되면 탭이 늘어날수록 분할이 어려워진다 (plan D6)
pub fn show_tab_strip(ui: &mut egui::Ui, model: &TabsModel) -> TabStripOutcome {
    let mut outcome = TabStripOutcome::default();
    egui::Sides::new().shrink_left().height(STRIP_HEIGHT).show(
        ui,
        |ui| show_tabs(ui, model, &mut outcome.tab),
        |ui| show_split_button(ui, &mut outcome.split),
    );
    outcome
}

/// 탭 목록과 새 탭 버튼 — 폭이 모자라면 가로로 스크롤된다
fn show_tabs(ui: &mut egui::Ui, model: &TabsModel, action: &mut Option<TabAction>) {
    egui::ScrollArea::horizontal()
        .auto_shrink([false, true])
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                for (index, path) in model.paths().iter().enumerate() {
                    let active = index == model.active_index();
                    let title = tab_title(path);
                    // 활성 탭만 밝은 배경으로 구분한다
                    let fill = if active {
                        theme::CONTROL_ACTIVE
                    } else {
                        theme::CONTROL_BG
                    };
                    let button = egui::Button::new(elide(&title))
                        .fill(fill)
                        .min_size(egui::vec2(0.0, STRIP_HEIGHT));
                    let resp = ui.add(button).on_hover_text(path.to_string_lossy());
                    if resp.clicked() {
                        *action = Some(TabAction::Switch(index));
                    }
                    // 가운데 버튼 클릭으로도 닫는다 (브라우저 관례)
                    if resp.middle_clicked() {
                        *action = Some(TabAction::Close(index));
                    }
                    if ui.small_button("×").on_hover_text("탭 닫기").clicked() {
                        *action = Some(TabAction::Close(index));
                    }
                    ui.add_space(2.0);
                }
                if ui.small_button("+").on_hover_text("새 탭").clicked() {
                    *action = Some(TabAction::New);
                }
            });
        });
}

/// 분할 버튼 — 누르면 네 방향 메뉴가 뜬다. 항목은 보기 메뉴와 같은 목록을 쓴다
fn show_split_button(ui: &mut egui::Ui, split: &mut Option<SplitTo>) {
    let mut command = None;
    let button = egui::Button::new("")
        .min_size(egui::vec2(STRIP_HEIGHT, STRIP_HEIGHT))
        .fill(theme::CONTROL_BG);
    let (response, _) = egui::containers::menu::MenuButton::from_button(button).ui(ui, |ui| {
        menu::split_items(ui, &mut command);
    });
    draw_split_icon(ui.painter(), response.rect);
    response.on_hover_text("분할");
    if let Some(Command::Split(to)) = command {
        *split = Some(to);
    }
}

/// 분할 아이콘 — 사각형 테두리와 세로 중앙선.
/// 글리프(`◫`)를 쓰지 않는 이유: 폰트에 그 문자가 없으면 두부(□)로 보이는데,
/// 폰트 지원 여부는 화면을 띄우기 전에는 알 수 없다 (plan D8)
fn draw_split_icon(painter: &egui::Painter, rect: egui::Rect) {
    let icon = egui::Rect::from_center_size(rect.center(), egui::Vec2::splat(SPLIT_ICON_SIZE));
    let stroke = egui::Stroke::new(SPLIT_ICON_STROKE, theme::TEXT);
    painter.rect_stroke(icon, 0.0, stroke, egui::StrokeKind::Inside);
    painter.vline(icon.center().x, icon.y_range(), stroke);
}

/// 탭 제목 말줄임 — 긴 폴더명이 스트립을 다 차지하지 않게 한다
fn elide(title: &str) -> String {
    // 문자 수 기준 — 한글은 폭이 넓어 대략 절반으로 잡는다
    const MAX_CHARS: usize = 16;
    let count = title.chars().count();
    if count <= MAX_CHARS {
        return title.to_owned();
    }
    let kept: String = title.chars().take(MAX_CHARS - 1).collect();
    format!("{kept}…")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 짧은_제목은_그대로_둔다() {
        assert_eq!(elide("문서"), "문서");
        assert_eq!(elide("Downloads"), "Downloads");
    }

    #[test]
    fn 긴_제목은_말줄임한다() {
        let long = "아주아주아주아주아주아주긴폴더이름입니다";
        let out = elide(long);
        assert!(out.ends_with('…'));
        assert_eq!(out.chars().count(), 16);
    }

    #[test]
    fn 경계_길이는_자르지_않는다() {
        let exact: String = "가".repeat(16);
        assert_eq!(elide(&exact), exact);
    }
}
