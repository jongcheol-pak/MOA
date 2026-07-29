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

/// 창 자체에 대한 요청 — 실행(`ViewportCommand` 전송)은 `ui::app`이 한다
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowRequest {
    Minimize,
    ToggleMaximize,
    Close,
    /// 이 프레임부터 OS에 창 끌기를 맡긴다
    Drag,
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

/// 좌측 — 워크스페이스 사이드바 표시 토글 (T5에서 채운다)
fn show_left(ui: &mut egui::Ui, _state: TitlebarState) -> Option<Command> {
    ui.add_space(LEFT_MARGIN);
    None
}

/// 우측 — 설정 버튼(T5)과 최소화·최대화·닫기.
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
    request
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
