//! 공용 위젯 — 여러 화면 요소가 같은 규칙으로 그리는 조각들.
//!
//! 지금은 아이콘 버튼 하나뿐이다. 타이틀바·탭 스트립·주소창이 모두
//! "평소에는 배경 없이 아이콘만, 마우스가 올라오면 배경을 칠한다"는 같은 규칙을 쓰는데,
//! 이 규칙을 세 곳이 각자 구현하면 hover 색·크기 처리가 조금씩 갈린다.
use crate::ui::theme;
use eframe::egui;

/// 아이콘 글꼴 기본 크기 — 타이틀바 캡션 버튼이 쓰던 값이 기준이다
const DEFAULT_ICON_PX: f32 = 16.0;

/// 프레임 없는 아이콘 버튼 — 기본 글자색·글꼴 크기로 그린다
pub fn icon_button(
    ui: &mut egui::Ui,
    icon: &str,
    size: egui::Vec2,
    hover_fill: egui::Color32,
) -> egui::Response {
    icon_button_styled(ui, icon, size, hover_fill, theme::TEXT, DEFAULT_ICON_PX)
}

/// 글자색·글꼴 크기를 지정하는 변형.
///
/// 비활성 버튼은 `tint`를 흐린 색으로, `hover_fill`을 투명으로 넘겨 표현한다 —
/// egui의 `add_enabled`를 쓰지 않는 이유는 그것이 버튼 프레임까지 함께 그리기 때문이다.
///
/// **이 함수는 자기 rect를 새로 할당하고 클릭을 감지하는 독립 버튼용이다.**
/// 다른 위젯 *안쪽*에 그리는 요소(탭 안의 아이콘 등)에는 쓰지 않는다 —
/// 쓰면 그 요소가 바깥 위젯 밖에 배치되거나, 그 자리에서 바깥 위젯의 클릭이 삼켜진다
pub fn icon_button_styled(
    ui: &mut egui::Ui,
    icon: &str,
    size: egui::Vec2,
    hover_fill: egui::Color32,
    tint: egui::Color32,
    font_px: f32,
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
    if response.hovered() {
        ui.painter().rect_filled(rect, 0.0, hover_fill);
    }
    // 아이콘 없이 부르는 호출부가 있다 — 영역과 hover 배경만 받고 그림은 직접 그리는 경우다
    if !icon.is_empty() {
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            icon,
            egui::FontId::proportional(font_px),
            tint,
        );
    }
    response
}
