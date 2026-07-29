//! 탭 스트립 — 패널별 독립 탭 (FR-3)과 분할 버튼 (FR-1 진입점).
//!
//! 상태는 `panel::tabs::TabsModel`(순수 모델)이 갖고, 이 파일은 그리기와 입력만 한다.
//!
//! 탭 하나는 **한 덩어리 위젯**이다 — 제목과 닫기 버튼을 각각 `Button`으로 두면
//! 저마다 프레임·여백을 그려 두 개의 사각형으로 보인다(Windows 11 탐색기는 한 탭 안에
//! 아이콘·제목·닫기가 들어 있다). 그래서 영역만 잡고 내용은 직접 그린다.
use crate::panel::tabs::{TabsModel, tab_title};
use crate::ui::menu::{self, Command, PanelMenuState};
use crate::ui::theme;
use crate::ui::widgets;
use eframe::egui;
use std::path::Path;

// ── 시각 토큰 (plan `## 시각 요소 분해` 1:1, 96DPI 기준 고정 px) ──
/// 탭·버튼의 높이 — 스트립 한 줄의 기준
const STRIP_HEIGHT: f32 = 28.0;
/// 탭 왼쪽 여백
const TAB_PAD_LEFT: f32 = 6.0;
/// 폴더 아이콘 구역 폭
const TAB_ICON_WIDTH: f32 = 16.0;
/// 아이콘과 제목 사이 간격
const TAB_ICON_GAP: f32 = 4.0;
/// 닫기 버튼 구역 폭
const TAB_CLOSE_WIDTH: f32 = 20.0;
/// 탭 오른쪽 여백
const TAB_PAD_RIGHT: f32 = 4.0;
/// 탭 최소 폭 — 제목이 0폭이 되어도 아이콘·닫기 구역은 남는다
const TAB_MIN_WIDTH: f32 =
    TAB_PAD_LEFT + TAB_ICON_WIDTH + TAB_ICON_GAP + TAB_CLOSE_WIDTH + TAB_PAD_RIGHT;
/// 폴더 아이콘 글꼴 크기
const TAB_ICON_PX: f32 = 14.0;
/// 닫기 아이콘 글꼴 크기
const TAB_CLOSE_PX: f32 = 12.0;
/// 새 탭 버튼 폭
const NEW_TAB_WIDTH: f32 = 24.0;
/// 탭 사이 구분선이 차지하는 높이 비율 — 위아래를 띄워 선이 스트립을 가르지 않게 한다
const SEPARATOR_RATIO: f32 = 0.6;

/// 분할 버튼 아이콘(사각형) 한 변 — 글리프가 아니라 직접 그린다 (split-4way plan D8)
const SPLIT_ICON_SIZE: f32 = 12.0;

/// 분할 버튼 아이콘 선 두께
const SPLIT_ICON_STROKE: f32 = 1.0;

/// 탭 스트립이 상위(패널)에 돌려주는 조작.
/// 탭 조작과 메뉴 명령은 서로 독립이라 한 프레임에 함께 나올 수 있다
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct TabStripOutcome {
    pub tab: Option<TabAction>,
    /// 패널 메뉴에서 고른 명령 — 대상은 **이 스트립이 속한 패널**이다 (plan D16)
    pub command: Option<Command>,
}

/// 탭 하나에 대한 조작 — 스트립 전체 결과는 `TabStripOutcome`이 담는다
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TabAction {
    Switch(usize),
    Close(usize),
    New,
}

/// 탭 하나를 세 구역으로 나눈 결과 — 그리기와 히트 판정이 같은 좌표를 쓰게 한다
#[derive(Clone, Copy, PartialEq, Debug)]
struct TabParts {
    icon: egui::Rect,
    label: egui::Rect,
    close: egui::Rect,
}

/// 탭 하나에서 이번 프레임에 일어난 일
#[derive(Clone, Copy, PartialEq, Debug)]
struct TabHit {
    switch: bool,
    close: bool,
    /// 그려진 탭 영역 — 구분선을 어디에 그릴지 정하는 데 쓴다
    rect: egui::Rect,
}

/// 탭 스트립을 그리고 이번 프레임의 조작을 반환한다.
///
/// 탭이 많으면 왼쪽 영역이 가로로 스크롤되고, **메뉴 버튼은 오른쪽 끝에 고정**된다 —
/// 버튼까지 스크롤되면 탭이 늘어날수록 메뉴에 닿기 어려워진다 (split-4way plan D6)
pub fn show_tab_strip(
    ui: &mut egui::Ui,
    model: &TabsModel,
    menu_state: PanelMenuState,
) -> TabStripOutcome {
    let mut outcome = TabStripOutcome::default();
    egui::Sides::new().shrink_left().height(STRIP_HEIGHT).show(
        ui,
        |ui| show_tabs(ui, model, &mut outcome.tab),
        |ui| show_menu_button(ui, menu_state, &mut outcome.command),
    );
    outcome
}

/// 탭 목록과 새 탭 버튼 — 폭이 모자라면 가로로 스크롤된다
fn show_tabs(ui: &mut egui::Ui, model: &TabsModel, action: &mut Option<TabAction>) {
    egui::ScrollArea::horizontal()
        .auto_shrink([false, true])
        .show(ui, |ui| {
            // 탭끼리 붙어야 한 줄로 이어져 보인다 — 사이를 벌리면 다시 낱개 버튼처럼 보인다
            ui.spacing_mut().item_spacing.x = 0.0;
            ui.horizontal(|ui| {
                let paths = model.paths();
                let active_index = model.active_index();
                for (index, path) in paths.iter().enumerate() {
                    let active = index == active_index;
                    let hit = show_tab(ui, index, &tab_title(path), path, active);
                    if hit.close {
                        *action = Some(TabAction::Close(index));
                    } else if hit.switch {
                        *action = Some(TabAction::Switch(index));
                    }
                    // 구분선은 **양옆이 모두 비활성일 때만** 그린다 —
                    // 활성 탭은 배경 자체가 경계 역할을 해서 선까지 그으면 지저분해진다
                    let next = index + 1;
                    if next < paths.len() && !active && next != active_index {
                        draw_separator(ui.painter(), hit.rect);
                    }
                }
                if widgets::icon_button(
                    ui,
                    egui_phosphor::regular::PLUS,
                    egui::vec2(NEW_TAB_WIDTH, STRIP_HEIGHT),
                    theme::CONTROL_HOT,
                )
                .on_hover_text("새 탭")
                .clicked()
                {
                    *action = Some(TabAction::New);
                }
            });
        });
}

/// 탭 하나를 그린다 — 폴더 아이콘·제목·닫기 ×가 한 영역 안에 들어간다.
///
/// 내부 요소는 위젯이 아니라 painter로 그린다. `icon_button` 같은 위젯을 쓰면
/// 자기 자리를 새로 잡아 탭 밖에 배치되고, 그 자리에서 탭 클릭도 삼켜진다
fn show_tab(ui: &mut egui::Ui, index: usize, title: &str, path: &Path, active: bool) -> TabHit {
    let text = elide(title);
    let font = egui::TextStyle::Button.resolve(ui.style());
    let text_width = ui
        .painter()
        .layout_no_wrap(text.clone(), font.clone(), theme::TEXT)
        .size()
        .x;
    // 제목이 짧아도 아이콘·닫기 구역은 지켜야 하므로 최소 폭을 보장한다 (tab_parts의 전제)
    let width = (TAB_PAD_LEFT
        + TAB_ICON_WIDTH
        + TAB_ICON_GAP
        + text_width
        + TAB_CLOSE_WIDTH
        + TAB_PAD_RIGHT)
        .max(TAB_MIN_WIDTH);
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(width, STRIP_HEIGHT), egui::Sense::click());
    let parts = tab_parts(rect);

    // 활성 탭만 배경을 채운다 — 비활성은 스트립 배경 그대로 둔다
    if active {
        ui.painter().rect_filled(rect, 0.0, theme::CONTROL_BG);
    }
    let painter = ui.painter();
    painter.text(
        parts.icon.center(),
        egui::Align2::CENTER_CENTER,
        egui_phosphor::regular::FOLDER,
        egui::FontId::proportional(TAB_ICON_PX),
        theme::FOLDER_ICON,
    );
    painter.text(
        parts.label.left_center(),
        egui::Align2::LEFT_CENTER,
        &text,
        font,
        theme::TEXT,
    );

    // 닫기 구역은 탭 위에 얹힌 별도 위젯이다 — 나중에 등록해야 탭보다 클릭이 우선한다
    let close = ui.interact(
        parts.close,
        ui.id().with(("tab_close", index)),
        egui::Sense::click(),
    );
    if close.hovered() {
        ui.painter()
            .rect_filled(parts.close, 0.0, theme::CONTROL_HOT);
    }
    ui.painter().text(
        parts.close.center(),
        egui::Align2::CENTER_CENTER,
        egui_phosphor::regular::X,
        egui::FontId::proportional(TAB_CLOSE_PX),
        theme::TEXT,
    );
    let close_clicked = close.clicked();
    close.on_hover_text("탭 닫기");

    // 닫기 자리를 누른 클릭은 전환으로 세지 않는다. 위 `interact`가 이미 가로채지만,
    // 히트 우선순위가 아니라 좌표로도 한 번 더 막는다 — 이 규칙이 깨지면
    // ×를 눌러도 탭만 바뀌는 회귀가 조용히 들어온다
    let clicked_on_close = response
        .interact_pointer_pos()
        .is_some_and(|pos| is_close_hit(&parts, pos));
    let switch = response.clicked() && !clicked_on_close;
    // 가운데 버튼 클릭으로도 닫는다 (브라우저 관례)
    let middle_close = response.middle_clicked();
    response.on_hover_text(path.to_string_lossy());

    TabHit {
        switch,
        close: close_clicked || middle_close,
        rect,
    }
}

/// 탭 영역을 아이콘·제목·닫기 세 구역으로 나눈다.
///
/// **폭이 `TAB_MIN_WIDTH` 이상임을 전제한다**(호출부가 보장) — 여기서 다시 늘리면
/// 반환 구역이 입력 영역을 벗어나 "닫기 구역은 탭 안에 있다"는 계약이 깨진다
fn tab_parts(rect: egui::Rect) -> TabParts {
    let icon = egui::Rect::from_min_size(
        egui::pos2(rect.left() + TAB_PAD_LEFT, rect.top()),
        egui::vec2(TAB_ICON_WIDTH, rect.height()),
    );
    let close = egui::Rect::from_min_size(
        egui::pos2(rect.right() - TAB_PAD_RIGHT - TAB_CLOSE_WIDTH, rect.top()),
        egui::vec2(TAB_CLOSE_WIDTH, rect.height()),
    );
    let label_left = icon.right() + TAB_ICON_GAP;
    let label = egui::Rect::from_min_max(
        egui::pos2(label_left, rect.top()),
        egui::pos2(close.left().max(label_left), rect.bottom()),
    );
    TabParts { icon, label, close }
}

/// 포인터가 닫기 구역 안인가 — 닫기가 탭 전환보다 우선한다는 규칙의 정본
fn is_close_hit(parts: &TabParts, pos: egui::Pos2) -> bool {
    parts.close.contains(pos)
}

/// 비활성 탭 사이 구분선 — 탭 오른쪽 경계에 세로로 짧게 긋는다
fn draw_separator(painter: &egui::Painter, tab: egui::Rect) {
    let half = tab.height() * SEPARATOR_RATIO / 2.0;
    let center = tab.center().y;
    painter.vline(
        tab.right(),
        center - half..=center + half,
        egui::Stroke::new(1.0, theme::TREE_LINE),
    );
}

/// 패널 메뉴 버튼 — 누르면 보기·분할·새로 고침·새 파일/폴더·닫기 메뉴가 뜬다 (FR-26).
///
/// `MenuButton`을 쓰지 않는 이유: 그것은 `Button` 위젯을 요구해 프레임을 함께 그린다.
/// 타이틀바 설정 버튼과 같이 아이콘 버튼에 팝업만 붙인다.
/// **아이콘은 분할 도형 그대로 둔다** — 사용자가 이미 이 자리를 메뉴 진입점으로 쓰고 있어
/// 모양이 바뀌면 찾지 못한다 (plan D10)
fn show_menu_button(ui: &mut egui::Ui, state: PanelMenuState, command: &mut Option<Command>) {
    let response = widgets::icon_button(
        ui,
        "",
        egui::vec2(STRIP_HEIGHT, STRIP_HEIGHT),
        theme::CONTROL_HOT,
    )
    .on_hover_text("메뉴");
    draw_split_icon(ui.painter(), response.rect);
    egui::Popup::menu(&response).show(|ui| {
        menu::panel_menu_items(ui, state, command);
    });
}

/// 메뉴 버튼 아이콘 — 사각형 테두리와 세로 중앙선(분할을 뜻하던 도형을 그대로 쓴다).
/// 글리프(`◫`)를 쓰지 않는 이유: 폰트에 그 문자가 없으면 두부(□)로 보이는데,
/// 폰트 지원 여부는 화면을 띄우기 전에는 알 수 없다 (split-4way plan D8)
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

    /// 탭 영역 하나 — 왼쪽 위 모서리를 원점에서 떼어 좌표 계산 실수를 드러낸다
    fn tab_rect(width: f32) -> egui::Rect {
        egui::Rect::from_min_size(egui::pos2(100.0, 50.0), egui::vec2(width, STRIP_HEIGHT))
    }

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

    #[test]
    fn 세_구역은_서로_겹치지_않는다() {
        for width in [TAB_MIN_WIDTH, 120.0, 300.0] {
            let rect = tab_rect(width);
            let parts = tab_parts(rect);
            assert!(
                parts.icon.right() <= parts.label.left(),
                "폭 {width}: 아이콘과 제목이 겹친다"
            );
            assert!(
                parts.label.right() <= parts.close.left(),
                "폭 {width}: 제목과 닫기가 겹친다"
            );
        }
    }

    #[test]
    fn 닫기_구역은_항상_탭_안에_있다() {
        // 탭이 좁아져 닫기가 밖으로 밀리면 탭을 닫을 수 없게 된다
        for width in [TAB_MIN_WIDTH, 120.0, 300.0] {
            let rect = tab_rect(width);
            let parts = tab_parts(rect);
            assert!(
                rect.contains_rect(parts.close),
                "폭 {width}: 닫기 구역이 탭을 벗어났다"
            );
        }
    }

    #[test]
    fn 최소_폭에서도_아이콘과_닫기_크기가_유지된다() {
        // 좁아질 때 줄어드는 것은 제목뿐이다 — 아이콘·닫기가 함께 찌그러지면 누를 수 없다
        let parts = tab_parts(tab_rect(TAB_MIN_WIDTH));
        assert_eq!(parts.icon.width(), TAB_ICON_WIDTH);
        assert_eq!(parts.close.width(), TAB_CLOSE_WIDTH);
        assert_eq!(parts.label.width(), 0.0);
    }

    #[test]
    fn 닫기_구역_안의_클릭만_닫기로_친다() {
        // 이 판정이 뒤집히면 ×를 눌러도 탭 전환만 되는 회귀가 들어온다
        let parts = tab_parts(tab_rect(200.0));
        assert!(is_close_hit(&parts, parts.close.center()));
        assert!(!is_close_hit(&parts, parts.label.center()));
        assert!(!is_close_hit(&parts, parts.icon.center()));
    }
}
