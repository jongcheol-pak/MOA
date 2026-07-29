//! 커스텀 타이틀바 (FR-22) — OS 기본 창 장식 대신 앱이 직접 그리는 제목 줄.
//!
//! 창 장식을 끈 이유는 그 줄 안에 앱 버튼(사이드바 토글·설정)을 두기 위해서다.
//! 대신 창 이동·최대화·크기 조절을 앱이 직접 처리해야 한다 — 다만 실제 동작은
//! OS에 위임한다(`ViewportCommand::StartDrag`/`BeginResize`가 winit을 거쳐
//! `WM_NCLBUTTONDOWN`으로 넘어간다). 그래서 가장자리 스냅 같은 Windows 동작이 그대로 산다.
//!
//! 이 모듈은 상태를 바꾸지 않는다 — 무엇을 하라는 **요청만 값으로 돌려주고**,
//! 실행은 `ui::app`이 한다(`ui::menu`·`ui::tabs`와 같은 규약).
use crate::ui::menu::Command;
use crate::ui::theme;
use eframe::egui;

// ── 시각 토큰 (plan `## 시각 요소 분해` 1:1, 96DPI 기준 고정 px) ──
/// 타이틀바 한 줄 높이 — 사이드바 헤더(36px)와 같은 단을 쓴다
pub const TITLEBAR_HEIGHT: f32 = 36.0;
/// 사이드바 토글·설정 버튼 한 변
const BUTTON_SIZE: f32 = 36.0;
/// 최소화·최대화·닫기 버튼 폭 (Windows 캡션 버튼 관례)
const CAPTION_WIDTH: f32 = 46.0;
/// 우측 버튼군 전체 폭 — 설정 1개 + 캡션 3개
const RIGHT_GROUP_WIDTH: f32 = BUTTON_SIZE + CAPTION_WIDTH * 3.0;
/// 좌측 토글 버튼 앞 여백
const LEFT_MARGIN: f32 = 2.0;
/// 좌측 버튼군 전체 폭 — 여백 + 사이드바 토글 1개
const LEFT_GROUP_WIDTH: f32 = LEFT_MARGIN + BUTTON_SIZE;
const ICON_FONT_PX: f32 = 16.0;
const TITLE_FONT_PX: f32 = 14.0;

/// 창 가장자리에서 크기 조절을 받는 폭. 좁게 잡는다 —
/// 넓히면 목록 스크롤바·스플리터처럼 창 끝에 닿는 위젯을 자주 가로챈다
const RESIZE_MARGIN: f32 = 4.0;

/// 창 자체에 대한 요청 — 실행(`ViewportCommand` 전송)은 `ui::app`이 한다
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowRequest {
    Minimize,
    ToggleMaximize,
    Close,
    /// 이 프레임부터 OS에 창 끌기를 맡긴다
    Drag,
    /// 이 프레임부터 OS에 창 크기 조절을 맡긴다
    Resize(egui::ResizeDirection),
}

/// 타이틀바를 그리는 데 필요한 현재 상태
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TitlebarState {
    /// 최대화 여부 — 최대화 버튼 아이콘이 복원 모양으로 바뀐다
    pub maximized: bool,
    /// 사이드바가 접혀 있는가 — 좌측 토글 버튼의 안내 문구가 갈린다
    pub sidebar_collapsed: bool,
}

/// 타이틀바가 이번 프레임에 낸 요청. 창 조작과 앱 명령은 서로 독립이다
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TitlebarOutcome {
    pub command: Option<Command>,
    pub window: Option<WindowRequest>,
}

/// 타이틀바 한 줄을 그리고 이번 프레임의 요청을 돌려준다.
///
/// `title`은 활성 워크스페이스 이름이다 — 작업 표시줄에 뜨는 OS 창 제목("파일 탐색기")과는
/// 별개다(D8). 좌우 버튼을 뺀 폭에서 넘치면 egui가 말줄임한다
pub fn show_titlebar(ui: &mut egui::Ui, title: &str, state: TitlebarState) -> TitlebarOutcome {
    let mut outcome = TitlebarOutcome::default();
    let bar = ui.max_rect();

    // 끌기 판정은 버튼이 놓인 좌·우 끝을 **뺀 빈 영역**에서만 한다.
    // 바 전체를 잡으면 버튼을 누른 순간에도 끌기 신호가 함께 나간다 — egui는 클릭 위젯과
    // 끌기 위젯을 동시에 히트로 잡고(`hit_test`), `is_pointer_button_down_on()`은 둘 중
    // 하나만 걸려도 참이기 때문이다(`context.rs`). 그러면 OS 창 이동 루프가 열리면서
    // 그 프레임의 버튼 클릭이 삼켜진다
    let drag_left = bar.min.x + LEFT_GROUP_WIDTH;
    let drag_right = (bar.max.x - RIGHT_GROUP_WIDTH).max(drag_left);
    let drag_area = egui::Rect::from_min_max(
        egui::pos2(drag_left, bar.min.y),
        egui::pos2(drag_right, bar.max.y),
    );
    let drag = ui.interact(
        drag_area,
        ui.id().with("titlebar_drag"),
        egui::Sense::click_and_drag(),
    );
    if drag.double_clicked() {
        outcome.window = Some(WindowRequest::ToggleMaximize);
    } else if drag.is_pointer_button_down_on() {
        // 더블클릭한 프레임에는 끌기를 요청하지 않는다 — 둘 다 보내면 OS 끌기 루프가
        // 먼저 잡혀 더블클릭이 삼켜진다 (D4)
        outcome.window = Some(WindowRequest::Drag);
    }

    // 좌우 클로저는 각자 자기 결과만 돌려준다 — 하나의 `outcome`을 양쪽에서 빌릴 수 없다
    let (command, window) = egui::Sides::new().height(TITLEBAR_HEIGHT).show(
        ui,
        |ui| show_left(ui, state),
        |ui| show_right(ui, state),
    );
    outcome.command = command;
    // 버튼을 눌렀으면 그 요청이 배경 끌기보다 우선한다
    if window.is_some() {
        outcome.window = window;
    }

    // 제목은 좌우 버튼군 배치와 무관하게 **바 한가운데**에 둔다. 최대 폭은 좌우 중
    // 넓은 쪽(우측 버튼군)을 양쪽에서 뺀 값이라야 가운데 정렬과 버튼 비침범이 함께 성립한다 (D14)
    let title_width = (bar.width() - 2.0 * RIGHT_GROUP_WIDTH).max(0.0);
    let title_rect =
        egui::Rect::from_center_size(bar.center(), egui::vec2(title_width, TITLEBAR_HEIGHT));
    ui.put(
        title_rect,
        egui::Label::new(
            egui::RichText::new(title)
                .size(TITLE_FONT_PX)
                .color(theme::TEXT),
        )
        .truncate()
        .selectable(false),
    );

    outcome
}

/// 좌측 — 워크스페이스 목록 표시 토글.
/// 사이드바가 접히면 그 안의 접기 버튼도 함께 사라지므로, 다시 펼 수 있는 자리가 여기다
fn show_left(ui: &mut egui::Ui, state: TitlebarState) -> Option<Command> {
    ui.add_space(LEFT_MARGIN);
    let hint = if state.sidebar_collapsed {
        "워크스페이스 목록 보이기"
    } else {
        "워크스페이스 목록 숨기기"
    };
    icon_button(
        ui,
        egui_phosphor::regular::SIDEBAR_SIMPLE,
        BUTTON_SIZE,
        theme::CONTROL_HOT,
    )
    .on_hover_text(hint)
    .clicked()
    .then_some(Command::ToggleSidebar)
}

/// 우측 — 설정 버튼과 최소화·최대화·닫기.
/// `Sides`의 오른쪽 영역은 오른쪽부터 채워지므로 **닫기를 먼저** 추가해야
/// 화면에서 왼→오 순서가 설정·최소화·최대화·닫기가 된다
fn show_right(ui: &mut egui::Ui, state: TitlebarState) -> Option<WindowRequest> {
    let mut request = None;
    if caption_button(ui, egui_phosphor::regular::X, theme::CLOSE_HOT)
        .on_hover_text("닫기")
        .clicked()
    {
        request = Some(WindowRequest::Close);
    }
    // 최대화된 창에서는 되돌리기가 다음 동작이므로 아이콘·안내를 함께 바꾼다
    let (restore_icon, restore_hint) = if state.maximized {
        (egui_phosphor::regular::CORNERS_IN, "이전 크기로")
    } else {
        (egui_phosphor::regular::SQUARE, "최대화")
    };
    if caption_button(ui, restore_icon, theme::CONTROL_HOT)
        .on_hover_text(restore_hint)
        .clicked()
    {
        request = Some(WindowRequest::ToggleMaximize);
    }
    if caption_button(ui, egui_phosphor::regular::MINUS, theme::CONTROL_HOT)
        .on_hover_text("최소화")
        .clicked()
    {
        request = Some(WindowRequest::Minimize);
    }
    show_settings_menu(ui);
    request
}

/// 설정 메뉴 — 항목은 **표시만** 하고 각 기능은 아직 만들지 않았다.
/// 고를 수 있는 것이 없으므로 돌려줄 결과도 없다.
///
/// 다섯 항목을 배열+반복으로 묶지 않은 이유: 각 항목이 곧 서로 다른 화면·동작으로 갈라질
/// 자리라, 지금 묶으면 채우는 순간 다시 풀어야 한다
fn show_settings_menu(ui: &mut egui::Ui) {
    let response = icon_button(
        ui,
        egui_phosphor::regular::GEAR,
        BUTTON_SIZE,
        theme::CONTROL_HOT,
    )
    .on_hover_text("설정");
    egui::Popup::menu(&response).show(|ui| {
        pending_item(ui, "설정");
        pending_item(ui, "업데이트");
        pending_item(ui, "릴리즈 노트");
        ui.separator();
        pending_item(ui, "오픈소스 라이선스");
        pending_item(ui, "정보");
    });
}

/// 아직 기능이 없는 메뉴 항목 — 비활성으로 두어 준비 중임을 눈으로 알린다.
/// 활성처럼 보이면서 눌러도 반응이 없으면 고장으로 오인된다
fn pending_item(ui: &mut egui::Ui, label: &str) {
    ui.add_enabled(false, egui::Button::new(label));
}

/// 창 가장자리 크기 조절 (FR-22) — 매 프레임 포인터 위치를 보고 커서를 바꾸며,
/// 가장자리에서 왼쪽 버튼이 눌리면 OS에 크기 조절을 넘긴다.
///
/// 최대화 상태에서는 아무것도 하지 않는다 — 그 상태의 창은 크기를 바꿀 수 없다.
/// 가장자리 4px는 그 아래 위젯보다 크기 조절이 우선한다(그러지 않으면 창 끝에 닿는
/// 목록·스플리터 때문에 크기를 잡을 자리가 사라진다)
pub fn show_resize_handles(ctx: &egui::Context, maximized: bool) -> Option<WindowRequest> {
    if maximized {
        return None;
    }
    let pointer = ctx.pointer_latest_pos()?;
    let window = ctx.viewport_rect();
    let direction = resize_direction(pointer, window, RESIZE_MARGIN)?;
    // 위쪽 **변**이 타이틀바 버튼과 겹치는 구간은 버튼에 양보한다 — 그러지 않으면 버튼 위쪽
    // 4px를 누른 순간 크기 조절 루프가 열려 그 클릭이 삼켜진다(끌기 영역을 좁힌 것과 같은 이유).
    // 모서리는 양보하지 않는다: 거기까지 내주면 대각선으로 창을 잡을 자리가 사라진다
    if direction == egui::ResizeDirection::North && over_titlebar_button(pointer.x, window) {
        return None;
    }
    ctx.set_cursor_icon(resize_cursor(direction));
    // 누른 **직후**에 넘겨야 OS 크기 조절 루프가 열린다 (`BeginResize` 계약)
    let pressed = ctx.input(|input| input.pointer.primary_pressed());
    pressed.then_some(WindowRequest::Resize(direction))
}

/// 포인터가 창 가장자리 어느 쪽에 있는지 판정한다. 없으면 `None`(창 안쪽).
///
/// **모서리를 변보다 먼저 본다** — 두 변이 겹치는 자리에서 변으로 판정하면
/// 대각선 크기 조절을 할 수 없다
fn resize_direction(
    pointer: egui::Pos2,
    window: egui::Rect,
    margin: f32,
) -> Option<egui::ResizeDirection> {
    if !window.contains(pointer) {
        return None;
    }
    let left = pointer.x - window.min.x <= margin;
    let right = window.max.x - pointer.x <= margin;
    let top = pointer.y - window.min.y <= margin;
    let bottom = window.max.y - pointer.y <= margin;
    let direction = match (left, right, top, bottom) {
        (true, _, true, _) => egui::ResizeDirection::NorthWest,
        (_, true, true, _) => egui::ResizeDirection::NorthEast,
        (true, _, _, true) => egui::ResizeDirection::SouthWest,
        (_, true, _, true) => egui::ResizeDirection::SouthEast,
        (true, ..) => egui::ResizeDirection::West,
        (_, true, ..) => egui::ResizeDirection::East,
        (_, _, true, _) => egui::ResizeDirection::North,
        (.., true) => egui::ResizeDirection::South,
        _ => return None,
    };
    Some(direction)
}

/// 타이틀바 버튼이 놓인 좌·우 구간인가 — 위쪽 변 크기 조절이 이 구간을 비켜준다
fn over_titlebar_button(x: f32, window: egui::Rect) -> bool {
    x - window.min.x < LEFT_GROUP_WIDTH || window.max.x - x < RIGHT_GROUP_WIDTH
}

/// 방향에 맞는 마우스 커서
fn resize_cursor(direction: egui::ResizeDirection) -> egui::CursorIcon {
    match direction {
        egui::ResizeDirection::North => egui::CursorIcon::ResizeNorth,
        egui::ResizeDirection::South => egui::CursorIcon::ResizeSouth,
        egui::ResizeDirection::East => egui::CursorIcon::ResizeEast,
        egui::ResizeDirection::West => egui::CursorIcon::ResizeWest,
        egui::ResizeDirection::NorthEast => egui::CursorIcon::ResizeNorthEast,
        egui::ResizeDirection::NorthWest => egui::CursorIcon::ResizeNorthWest,
        egui::ResizeDirection::SouthEast => egui::CursorIcon::ResizeSouthEast,
        egui::ResizeDirection::SouthWest => egui::CursorIcon::ResizeSouthWest,
    }
}

/// 캡션 버튼 하나 — 최소화·최대화·닫기가 같은 폭·같은 그리기 규칙을 쓴다
fn caption_button(ui: &mut egui::Ui, icon: &str, hover_fill: egui::Color32) -> egui::Response {
    icon_button(ui, icon, CAPTION_WIDTH, hover_fill)
}

/// 타이틀바 버튼 공통 그리기 — 평소에는 배경 없이 아이콘만, 마우스가 올라오면 배경을 칠한다
fn icon_button(
    ui: &mut egui::Ui,
    icon: &str,
    width: f32,
    hover_fill: egui::Color32,
) -> egui::Response {
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(width, TITLEBAR_HEIGHT), egui::Sense::click());
    if response.hovered() {
        ui.painter().rect_filled(rect, 0.0, hover_fill);
    }
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        icon,
        egui::FontId::proportional(ICON_FONT_PX),
        theme::TEXT,
    );
    response
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 100×100 창 — 가장자리 판정만 보므로 위치는 원점으로 둔다
    fn window() -> egui::Rect {
        egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(100.0, 100.0))
    }

    #[test]
    fn 창_안쪽에서는_크기_조절이_아니다() {
        assert_eq!(
            resize_direction(egui::pos2(50.0, 50.0), window(), 4.0),
            None
        );
    }

    #[test]
    fn 창_밖은_판정하지_않는다() {
        // 다른 창 위에 있는 포인터까지 잡으면 엉뚱한 곳에서 커서가 바뀐다
        assert_eq!(
            resize_direction(egui::pos2(-5.0, 50.0), window(), 4.0),
            None
        );
    }

    #[test]
    fn 네_변을_각각_판정한다() {
        let (w, m) = (window(), 4.0);
        assert_eq!(
            resize_direction(egui::pos2(1.0, 50.0), w, m),
            Some(egui::ResizeDirection::West)
        );
        assert_eq!(
            resize_direction(egui::pos2(99.0, 50.0), w, m),
            Some(egui::ResizeDirection::East)
        );
        assert_eq!(
            resize_direction(egui::pos2(50.0, 1.0), w, m),
            Some(egui::ResizeDirection::North)
        );
        assert_eq!(
            resize_direction(egui::pos2(50.0, 99.0), w, m),
            Some(egui::ResizeDirection::South)
        );
    }

    #[test]
    fn 모서리는_변보다_먼저_판정된다() {
        // 두 변이 겹치는 자리를 변으로 판정하면 대각선 크기 조절을 할 수 없다
        let (w, m) = (window(), 4.0);
        assert_eq!(
            resize_direction(egui::pos2(1.0, 1.0), w, m),
            Some(egui::ResizeDirection::NorthWest)
        );
        assert_eq!(
            resize_direction(egui::pos2(99.0, 1.0), w, m),
            Some(egui::ResizeDirection::NorthEast)
        );
        assert_eq!(
            resize_direction(egui::pos2(1.0, 99.0), w, m),
            Some(egui::ResizeDirection::SouthWest)
        );
        assert_eq!(
            resize_direction(egui::pos2(99.0, 99.0), w, m),
            Some(egui::ResizeDirection::SouthEast)
        );
    }

    #[test]
    fn 위쪽_변은_버튼_구간을_비켜준다() {
        // 버튼 위쪽 4px가 크기 조절에 먹히면 그 버튼을 누를 수 없다.
        // 창 폭 100px에서 좌측 38px·우측 174px 구간이 버튼 자리이므로 여기서는 폭을 넉넉히 잡는다
        let wide = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(600.0, 100.0));
        assert!(over_titlebar_button(10.0, wide)); // 좌측 토글 자리
        assert!(over_titlebar_button(590.0, wide)); // 우측 캡션 버튼 자리
        assert!(!over_titlebar_button(300.0, wide)); // 가운데 — 제목 자리
    }

    #[test]
    fn 경계_바로_안쪽까지_잡는다() {
        // margin과 정확히 같은 거리도 가장자리로 본다 — 4px 띠가 3px로 좁아지면 잡기 어려워진다
        let (w, m) = (window(), 4.0);
        assert_eq!(
            resize_direction(egui::pos2(4.0, 50.0), w, m),
            Some(egui::ResizeDirection::West)
        );
        assert_eq!(resize_direction(egui::pos2(4.1, 50.0), w, m), None);
    }
}
