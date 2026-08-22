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
use crate::ui::widgets::icon_button;
use eframe::egui;

// ── 시각 토큰 (plan `## 시각 요소 분해` 1:1, 96DPI 기준 고정 px) ──
/// 타이틀바 한 줄 높이 — 사이드바 헤더(36px)와 같은 단을 쓴다
pub const TITLEBAR_HEIGHT: f32 = 36.0;
/// 사이드바 토글·설정 버튼 한 변
const BUTTON_SIZE: f32 = 36.0;
/// 최소화·최대화·닫기 버튼 폭 (Windows 캡션 버튼 관례)
const CAPTION_WIDTH: f32 = 46.0;
/// 우측 버튼군의 **기본** 폭 — 설정 1개 + 캡션 3개.
///
/// **업데이트 배지(FR-62)가 서면 그만큼 넓어진다** — 배지 문구는 언어·글꼴에 따라 폭이
/// 달라져 컴파일 시점에 정해지지 않으므로, 실제 폭은 매 프레임 재서 `TitlebarOutcome`에
/// 실어 내보낸다. 이 값을 쓰는 세 자리(창 끌기 영역·제목 최대 폭·위쪽 변 크기 조절
/// 비켜주기)가 모두 그 실측값을 받아야 한다 — 하나라도 빠뜨리면 배지 위에서 창이 끌리거나
/// 배지 위쪽 4px에서 크기 조절 루프가 열려 클릭이 삼켜진다
const RIGHT_GROUP_BASE: f32 = BUTTON_SIZE + CAPTION_WIDTH * 3.0;
/// 배지 아이콘과 글자 사이
const BADGE_ICON_GAP: f32 = 4.0;
/// 배지 좌우 여백 — 캡션 버튼(46px)보다 좁게 두어 글자가 테두리에 붙지 않게만 한다
const BADGE_PAD_X: f32 = 10.0;
/// 좌측 토글 버튼 앞 여백
const LEFT_MARGIN: f32 = 2.0;
/// 앱 아이콘 한 변 — 36px 줄에서 위아래 여백이 남게 잡는다
const APP_ICON_SIZE: f32 = 20.0;
/// 앱 아이콘 좌우 여백
const APP_ICON_GAP: f32 = 6.0;
/// 앱 아이콘이 차지하는 폭 (좌우 여백 포함)
const APP_ICON_WIDTH: f32 = APP_ICON_SIZE + APP_ICON_GAP * 2.0;
/// 좌측 영역 전체 폭 — 여백 + 앱 아이콘 + 사이드바 토글 1개
const LEFT_GROUP_WIDTH: f32 = LEFT_MARGIN + APP_ICON_WIDTH + BUTTON_SIZE;
const TITLE_FONT_PX: f32 = 14.0;
/// 타이틀바 아래 구분선 두께
const SEPARATOR_THICKNESS: f32 = 1.0;

/// 설정 메뉴 최소 폭 — 라벨이 짧아도 이보다 좁아지지 않는다.
/// 좁은 메뉴는 눌러야 할 자리가 작아 보여 다른 메뉴들과 나란히 놓였을 때 어색하다
const SETTINGS_MENU_MIN_WIDTH: f32 = 160.0;
/// 라벨이 테두리에 닿아 보이지 않게 두는 숨 쉴 자리 — 라벨 폭을 딱 맞춰 주면
/// 마지막 글자가 잘릴락 말락 하게 그려진다.
///
/// 항목 여백(아래 `SETTINGS_MENU_PADDING`)과 **따로 세는 이유**: 여백이 커지면 라벨이
/// 쓸 자리가 그만큼 줄어드는데, 합쳐 두면 그 사실이 가려져 여유가 0이 된 것을 아무도
/// 모른다. 실제로 종전 값 24는 「메뉴 안 여백 2px×2 + 여유 20」이었고, 여백만 12px로
/// 올리면서 합을 24로 두면 여유가 사라져 라벨이 두 줄로 접히던 상태(2026-08-19 보고)로
/// 돌아간다
const SETTINGS_MENU_BREATH: f32 = 20.0;
/// 라벨 좌우로 두는 여백 — 항목 여백 두 번 + 숨 쉴 자리.
///
/// 항목은 `ui.button`이라 좌우로 `button_padding.x`씩 쓰는데, 그 값은 `theme::menu_style`이
/// 세운 공통 토큰이다(메뉴 안에서는 egui 기본이 덮이므로 전역 기본값을 세면 틀린다)
const SETTINGS_MENU_PADDING: f32 = theme::MENU_ITEM_PAD_X * 2.0 + SETTINGS_MENU_BREATH;

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

/// 업데이트 표시에 필요한 최소 상태 (FR-62).
///
/// 타이틀바는 `app::update`를 모른다 — 무엇을 그릴지만 값으로 받는다(`ui::app`이 상태를
/// 이 둘로 옮겨 준다). 그래야 화면 코드가 업데이트 상태 기계에 묶이지 않는다
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct UpdateBadge {
    /// 배지를 세우는가 — 새 판이 있거나 받는 중일 때만 참이다
    pub visible: bool,
    /// 받는 중인가 — 문구가 갈리고 누를 수 없게 된다
    pub downloading: bool,
    /// 이 실행에서 업데이트 기능을 쓰는가 — 설정 메뉴의 `업데이트` 항목 활성이 이것을 따른다.
    /// 배지와 달리 **상태가 아니라 실행 환경**이라 확인 결과와 무관하게 고정이다 (D4)
    pub update_enabled: bool,
}

/// 타이틀바가 이번 프레임에 낸 요청. 창 조작과 앱 명령은 서로 독립이다.
///
/// **`Eq`를 파생하지 않는다** — 폭이 `f32`라 성립하지 않는다(부동소수는 자기 자신과
/// 같지 않은 값이 있다). 이 타입을 같은지 견주는 곳은 없고 시험은 필드별로 단언한다
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct TitlebarOutcome {
    pub command: Option<Command>,
    pub window: Option<WindowRequest>,
    /// 이번 프레임에 우측 버튼군이 실제로 차지한 폭 — **배지가 있으면 그만큼 넓다**.
    ///
    /// `ui::app`이 이 값을 같은 프레임의 `show_resize_handles`에 넘긴다. 한 프레임 늦게
    /// 넘기면(직전 값을 기억해 두면) 언어를 바꾼 첫 프레임에 판정이 어긋난다
    pub right_group_width: f32,
}

/// 타이틀바 한 줄을 그리고 이번 프레임의 요청을 돌려준다.
///
/// `title`은 활성 워크스페이스 이름이다 — 작업 표시줄에 뜨는 OS 창 제목(앱 이름,
/// `i18n::app_name`)과는 별개다(D8). 좌우 버튼을 뺀 폭에서 넘치면 egui가 말줄임한다
pub fn show_titlebar(
    ui: &mut egui::Ui,
    title: &str,
    state: TitlebarState,
    icon: Option<egui::TextureId>,
    badge: UpdateBadge,
) -> TitlebarOutcome {
    let mut outcome = TitlebarOutcome::default();
    let bar = ui.max_rect();
    // 배지 폭을 **먼저 재서** 아래 세 자리가 같은 값을 쓰게 한다 (D13)
    let right_group = RIGHT_GROUP_BASE + update_badge_width(ui, badge);
    outcome.right_group_width = right_group;

    // 끌기 판정은 버튼이 놓인 좌·우 끝을 **뺀 빈 영역**에서만 한다.
    // 바 전체를 잡으면 버튼 위에서 끌었을 때 끌기 신호가 함께 나간다 — egui는 클릭 위젯과
    // 끌기 위젯을 동시에 히트로 잡기 때문이다(`hit_test`). 그러면 OS 창 이동 루프가 열리면서
    // 그 프레임의 버튼 클릭이 삼켜진다
    let drag_left = bar.min.x + LEFT_GROUP_WIDTH;
    let drag_right = (bar.max.x - right_group).max(drag_left);
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
    } else if drag.drag_started() {
        // 누른 것만으로는 끌지 않는다 — 포인터가 실제로 움직여 egui가 끌기로 판정한
        // 프레임에만 요청한다. 누르자마자(`is_pointer_button_down_on`) 보내면 클릭·더블클릭에서도
        // OS 창 이동 루프가 열려, 손이 미세하게 흔들린 만큼 창이 따라 움직인 뒤 최대화가
        // 겹쳐 화면이 떨리며 바뀐다. 시작 프레임 한 번이면 충분하다 —
        // 그 뒤 창 이동은 OS 루프가 맡는다 (D4)
        outcome.window = Some(WindowRequest::Drag);
    }

    // 좌우 클로저는 각자 자기 결과만 돌려준다 — 하나의 `outcome`을 양쪽에서 빌릴 수 없다
    let (left_command, (window, right_command)) = egui::Sides::new().height(TITLEBAR_HEIGHT).show(
        ui,
        |ui| show_left(ui, state, icon),
        |ui| show_right(ui, state, badge),
    );
    // 좌우에서 동시에 명령이 나올 수는 없다(한 프레임에 두 버튼을 누를 수 없다)
    outcome.command = left_command.or(right_command);
    // 버튼을 눌렀으면 그 요청이 배경 끌기보다 우선한다
    if window.is_some() {
        outcome.window = window;
    }

    // 제목은 좌우 버튼군 배치와 무관하게 **바 한가운데**에 둔다. 최대 폭은 좌우 중
    // 넓은 쪽(우측 버튼군)을 양쪽에서 뺀 값이라야 가운데 정렬과 버튼 비침범이 함께 성립한다 (D14)
    let title_width = (bar.width() - 2.0 * right_group).max(0.0);
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

    // 타이틀바와 그 아래 본문을 가르는 구분선. egui Panel의 기본 구분선(전역 visuals 색)이 아니라
    // 여기서 직접 그린다 — 그 색은 다른 패널 구분선과 공유돼, 이 선만 조정할 수 없기 때문이다.
    // 바 **안쪽 끝**에 그린다(중심을 반 두께만큼 올린다) — 경계 밖은 패널 클립에 잘린다
    ui.painter().hline(
        bar.x_range(),
        bar.max.y - SEPARATOR_THICKNESS / 2.0,
        egui::Stroke::new(SEPARATOR_THICKNESS, theme::PANE_BORDER),
    );

    outcome
}

/// 앱 아이콘 — 워크스페이스 토글 왼쪽에 놓이는 표시 전용 그림(누를 수 없다).
/// 줄 높이만큼 자리를 잡고 그 안에서 세로 가운데에 그린다.
///
/// **아이콘이 없어도 자리는 잡는다** — 그래야 토글 버튼 위치가 `LEFT_GROUP_WIDTH`와
/// 어긋나지 않는다(그 값이 창 끌기·크기 조절이 비켜야 할 구간을 정한다)
fn show_app_icon(ui: &mut egui::Ui, icon: Option<egui::TextureId>) {
    ui.add_space(APP_ICON_GAP);
    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(APP_ICON_SIZE, TITLEBAR_HEIGHT),
        egui::Sense::hover(),
    );
    if let Some(icon) = icon {
        ui.painter().image(
            icon,
            egui::Rect::from_center_size(rect.center(), egui::Vec2::splat(APP_ICON_SIZE)),
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            egui::Color32::WHITE,
        );
    }
    ui.add_space(APP_ICON_GAP);
}

/// 좌측 — 워크스페이스 목록 표시 토글.
/// 사이드바가 접히면 그 안의 접기 버튼도 함께 사라지므로, 다시 펼 수 있는 자리가 여기다
fn show_left(
    ui: &mut egui::Ui,
    state: TitlebarState,
    icon: Option<egui::TextureId>,
) -> Option<Command> {
    ui.add_space(LEFT_MARGIN);
    show_app_icon(ui, icon);
    let hint = if state.sidebar_collapsed {
        crate::i18n::titlebar_show_workspaces()
    } else {
        crate::i18n::titlebar_hide_workspaces()
    };
    icon_button(
        ui,
        egui_phosphor::regular::SIDEBAR_SIMPLE,
        egui::vec2(BUTTON_SIZE, TITLEBAR_HEIGHT),
        theme::CONTROL_HOT,
    )
    .on_hover_text(hint)
    .clicked()
    .then_some(Command::ToggleSidebar)
}

/// 우측 — 설정 버튼과 최소화·최대화·닫기.
/// `Sides`의 오른쪽 영역은 오른쪽부터 채워지므로 **닫기를 먼저** 추가해야
/// 화면에서 왼→오 순서가 설정·최소화·최대화·닫기가 된다.
///
/// 창 조작과 명령을 함께 돌려주는 것은 설정 메뉴가 이쪽에 있기 때문이다 —
/// 그 메뉴는 창을 건드리지 않고 `Command`를 낸다
fn show_right(
    ui: &mut egui::Ui,
    state: TitlebarState,
    badge: UpdateBadge,
) -> (Option<WindowRequest>, Option<Command>) {
    let mut request = None;
    let mut command = None;
    if caption_button(ui, egui_phosphor::regular::X, theme::CLOSE_HOT)
        .on_hover_text(crate::i18n::close())
        .clicked()
    {
        request = Some(WindowRequest::Close);
    }
    // 최대화된 창에서는 되돌리기가 다음 동작이므로 아이콘·안내를 함께 바꾼다
    let (restore_icon, restore_hint) = if state.maximized {
        (
            egui_phosphor::regular::CORNERS_IN,
            crate::i18n::titlebar_restore(),
        )
    } else {
        (
            egui_phosphor::regular::SQUARE,
            crate::i18n::titlebar_maximize(),
        )
    };
    if caption_button(ui, restore_icon, theme::CONTROL_HOT)
        .on_hover_text(restore_hint)
        .clicked()
    {
        request = Some(WindowRequest::ToggleMaximize);
    }
    if caption_button(ui, egui_phosphor::regular::MINUS, theme::CONTROL_HOT)
        .on_hover_text(crate::i18n::titlebar_minimize())
        .clicked()
    {
        request = Some(WindowRequest::Minimize);
    }
    show_settings_menu(ui, &mut command, badge.update_enabled);
    // 배지는 설정 버튼 **뒤에** 더한다 — 이 영역은 오른쪽부터 채워지므로 나중에 더한 것이
    // 화면에서 더 왼쪽에 선다
    if let Some(badge_command) = show_update_badge(ui, badge) {
        command = Some(badge_command);
    }
    (request, command)
}

/// 업데이트 배지 (FR-62) — 새 판이 있으면 `↓ 업데이트`, 받는 중이면 도는 표시와
/// `다운로드 중...`을 설정 버튼 왼쪽에 세운다.
///
/// **받는 중에는 누를 수 없다** — 두 번 누르면 두 번 받는다
fn show_update_badge(ui: &mut egui::Ui, badge: UpdateBadge) -> Option<Command> {
    if !badge.visible {
        return None;
    }
    let width = update_badge_width(ui, badge);
    let sense = if badge.downloading {
        egui::Sense::hover()
    } else {
        egui::Sense::click()
    };
    let (rect, response) = ui.allocate_exact_size(egui::vec2(width, TITLEBAR_HEIGHT), sense);
    // **받는 중에는 hover 배경을 칠하지 않는다** — 누를 수 없는데 눌릴 것처럼 보이면
    // 반응이 없는 것이 고장으로 읽힌다(`Sense::hover()`도 hover 자체는 참이 된다)
    if !badge.downloading && response.hovered() {
        crate::ui::widgets::hover_backdrop(ui.painter(), rect, theme::CONTROL_HOT);
    }

    let (icon, label) = badge_parts(badge);
    let font = egui::FontId::proportional(TITLE_FONT_PX);
    let icon_width = text_width(ui, icon, &font);
    // 아이콘과 글자를 한 덩어리로 보고 그 덩어리를 가운데 놓는다
    let label_width = text_width(ui, label, &font);
    let content_left = rect.center().x - (icon_width + BADGE_ICON_GAP + label_width) / 2.0;
    let icon_center = egui::pos2(content_left + icon_width / 2.0, rect.center().y);
    if badge.downloading {
        draw_spinner(ui, icon, icon_center, &font);
    } else {
        ui.painter().text(
            icon_center,
            egui::Align2::CENTER_CENTER,
            icon,
            font.clone(),
            theme::TEXT,
        );
    }
    ui.painter().text(
        egui::pos2(content_left + icon_width + BADGE_ICON_GAP, rect.center().y),
        egui::Align2::LEFT_CENTER,
        label,
        font,
        theme::TEXT,
    );

    response.clicked().then_some(Command::StartUpdate)
}

/// 받는 중을 알리는 도는 아이콘 (시각 요소 분해 — `CIRCLE_NOTCH` + 회전).
///
/// **도는 것 자체가 「아직 끝나지 않았다」는 신호다** — 멈춰 있으면 굳은 화면으로 보인다.
/// 시간으로 각도를 내고 다음 프레임을 청해 계속 돌게 한다(그 사이 다른 것이 화면을
/// 갱신하지 않아도 이 아이콘은 움직여야 한다)
fn draw_spinner(ui: &egui::Ui, icon: &str, center: egui::Pos2, font: &egui::FontId) {
    // 한 바퀴에 1초 — 더 빠르면 어지럽고 더 느리면 멈춘 것처럼 보인다
    const TURN_SECONDS: f64 = 1.0;
    let time = ui.input(|input| input.time);
    let angle = ((time % TURN_SECONDS) / TURN_SECONDS) as f32 * std::f32::consts::TAU;
    let galley = ui
        .painter()
        .layout_no_wrap(icon.to_owned(), font.clone(), theme::TEXT);
    ui.painter().add(
        egui::epaint::TextShape::new(center, galley, theme::TEXT)
            .with_angle_and_anchor(angle, egui::Align2::CENTER_CENTER),
    );
    ui.ctx().request_repaint();
}

/// 배지에 그릴 아이콘과 글자 — 받는 중인지에 따라 갈린다.
///
/// 아이콘은 **phosphor에서만** 가져온다(AGENTS 규약 — 다른 글리프는 두부가 된다)
fn badge_parts(badge: UpdateBadge) -> (&'static str, &'static str) {
    if badge.downloading {
        (
            egui_phosphor::regular::CIRCLE_NOTCH,
            crate::i18n::update_downloading(),
        )
    } else {
        (
            egui_phosphor::regular::ARROW_CIRCLE_DOWN,
            crate::i18n::update_available(),
        )
    }
}

/// 배지가 차지하는 폭 — **그 자리에서 잰다**(설정 메뉴 폭과 같은 방식).
///
/// 배지가 없으면 0이라 우측 버튼군 폭이 종전 그대로가 된다.
/// 고정 상수로 두지 않는 이유는 문구가 언어(`업데이트`/`Update`)와 사용자 글꼴(FR-48)에
/// 따라 달라지기 때문이다 — 상수로 박으면 어느 조합에서 글자가 잘리는지 추정에 기대게 된다
pub fn update_badge_width(ui: &egui::Ui, badge: UpdateBadge) -> f32 {
    if !badge.visible {
        return 0.0;
    }
    let (icon, label) = badge_parts(badge);
    let font = egui::FontId::proportional(TITLE_FONT_PX);
    text_width(ui, icon, &font) + BADGE_ICON_GAP + text_width(ui, label, &font) + BADGE_PAD_X * 2.0
}

/// 글자가 차지하는 가로 폭
fn text_width(ui: &egui::Ui, text: &str, font: &egui::FontId) -> f32 {
    ui.painter()
        .layout_no_wrap(text.to_owned(), font.clone(), theme::TEXT)
        .size()
        .x
}

/// 설정 메뉴 — 다섯 항목이 모두 동작한다
/// (FR-22·FR-47·FR-57·FR-58·**FR-62·FR-63**).
///
/// 다섯 항목을 배열+반복으로 묶지 않은 이유: 각 항목이 곧 서로 다른 화면·동작으로 갈라질
/// 자리라, 지금 묶으면 채우는 순간 다시 풀어야 한다
fn show_settings_menu(ui: &mut egui::Ui, out: &mut Option<Command>, update_enabled: bool) {
    let response = icon_button(
        ui,
        egui_phosphor::regular::GEAR,
        egui::vec2(BUTTON_SIZE, TITLEBAR_HEIGHT),
        theme::CONTROL_HOT,
    )
    .on_hover_text(crate::i18n::settings_title());
    egui::Popup::menu(&response).show(|ui| {
        theme::menu_style(ui);
        ui.set_width(settings_menu_width(ui));
        if ui.button(crate::i18n::settings_title()).clicked() {
            *out = Some(Command::OpenAppSettings);
            ui.close();
        }
        // **설치본에서만 누를 수 있다** (D4·FR-62) — 개발 실행에서는 확인 자체를 하지
        // 않으므로 눌러도 아무 일이 없다. 활성처럼 보이면서 반응이 없으면 고장으로 오인된다.
        // `릴리즈 노트`는 이 조건이 없다 — 브라우저를 여는 것뿐이라 설치본과 무관하다(FR-63)
        if ui
            .add_enabled(
                update_enabled,
                egui::Button::new(crate::i18n::titlebar_updates()),
            )
            .clicked()
        {
            *out = Some(Command::CheckUpdate);
            ui.close();
        }
        if ui.button(crate::i18n::titlebar_release_notes()).clicked() {
            *out = Some(Command::OpenReleaseNotes);
            ui.close();
        }
        ui.separator();
        if ui.button(crate::i18n::titlebar_licenses()).clicked() {
            *out = Some(Command::OpenLicenses);
            ui.close();
        }
        if ui.button(crate::i18n::titlebar_about()).clicked() {
            *out = Some(Command::OpenAbout);
            ui.close();
        }
    });
}

/// 설정 메뉴 폭 — 다섯 라벨 중 가장 넓은 것에 맞춰 **그 자리에서 잰다**.
///
/// **폭을 주지 않으면 언어를 바꾼 뒤 항목이 두 줄로 접힌다** — `egui::Area`는 직전 프레임의
/// 크기를 기억해 다음 프레임의 레이아웃 한계로 쓰므로(`egui`의 `containers/area.rs`),
/// 한국어 폭 안에서 더 긴 영어 라벨(`Open source licenses`)이 줄바꿈된 채 굳는다.
/// 처음부터 영어로 켜면 기억된 크기가 없어 한 줄로 잡히던 것이 그 증거다 (2026-08-19 사용자 보고).
///
/// 고정 상수 대신 재는 이유: 화면 글꼴은 맑은 고딕이고 사용자가 바꿀 수도 있어(FR-48)
/// 상수로 박으면 그 값이 맞는지 추정에 기대게 된다. 여기서 재면 어떤 글꼴·언어에서도
/// 라벨이 그대로 든다 (`remote_states::badge_width`가 같은 방식을 쓴다).
///
/// **다섯 라벨을 여기서만 모은다** — 그리는 쪽은 항목마다 동작이 달라 배열로 묶지 않는다
fn settings_menu_width(ui: &egui::Ui) -> f32 {
    let font = egui::TextStyle::Button.resolve(ui.style());
    let widest = [
        crate::i18n::settings_title(),
        crate::i18n::titlebar_updates(),
        crate::i18n::titlebar_release_notes(),
        crate::i18n::titlebar_licenses(),
        crate::i18n::titlebar_about(),
    ]
    .into_iter()
    .map(|label| {
        ui.painter()
            .layout_no_wrap(label.to_owned(), font.clone(), theme::TEXT)
            .size()
            .x
    })
    .fold(0.0_f32, f32::max);
    (widest + SETTINGS_MENU_PADDING).max(SETTINGS_MENU_MIN_WIDTH)
}

/// 창 가장자리 크기 조절 (FR-22) — 매 프레임 포인터 위치를 보고 커서를 바꾸며,
/// 가장자리에서 왼쪽 버튼이 눌리면 OS에 크기 조절을 넘긴다.
///
/// 최대화 상태에서는 아무것도 하지 않는다 — 그 상태의 창은 크기를 바꿀 수 없다.
/// 가장자리 4px는 그 아래 위젯보다 크기 조절이 우선한다(그러지 않으면 창 끝에 닿는
/// 목록·스플리터 때문에 크기를 잡을 자리가 사라진다)
pub fn show_resize_handles(
    ctx: &egui::Context,
    maximized: bool,
    right_group_width: f32,
) -> Option<WindowRequest> {
    if maximized {
        return None;
    }
    let pointer = ctx.pointer_latest_pos()?;
    let window = ctx.viewport_rect();
    let direction = resize_direction(pointer, window, RESIZE_MARGIN)?;
    // 위쪽 **변**이 타이틀바 버튼과 겹치는 구간은 버튼에 양보한다 — 그러지 않으면 버튼 위쪽
    // 4px를 누른 순간 크기 조절 루프가 열려 그 클릭이 삼켜진다(끌기 영역을 좁힌 것과 같은 이유).
    // 모서리는 양보하지 않는다: 거기까지 내주면 대각선으로 창을 잡을 자리가 사라진다
    if direction == egui::ResizeDirection::North
        && over_titlebar_button(pointer.x, window, right_group_width)
    {
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
fn over_titlebar_button(x: f32, window: egui::Rect, right_group_width: f32) -> bool {
    x - window.min.x < LEFT_GROUP_WIDTH || window.max.x - x < right_group_width
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
    icon_button(
        ui,
        icon,
        egui::vec2(CAPTION_WIDTH, TITLEBAR_HEIGHT),
        hover_fill,
    )
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
    fn 최대화_중에는_크기_조절을_받지_않는다() {
        // 최대화된 창은 크기를 바꿀 수 없다 — 가드가 빠지면 가장자리에서 커서만 바뀌고 동작하지 않는
        // 어정쩡한 상태가 된다. 이 분기는 `resize_direction`이 아니라 여기 있어 Context가 필요하다
        let ctx = egui::Context::default();
        assert_eq!(show_resize_handles(&ctx, true, RIGHT_GROUP_BASE), None);
    }

    #[test]
    fn 받는_중에는_문구가_갈리고_눌리지_않는다() {
        // 두 번 누르면 두 번 받는다 — 그래서 받는 동안에는 클릭 자체가 나오지 않아야 한다
        let downloading = UpdateBadge {
            visible: true,
            downloading: true,
            update_enabled: true,
        };
        let idle = UpdateBadge {
            visible: true,
            downloading: false,
            update_enabled: true,
        };
        // 문구가 갈린다
        assert_eq!(badge_parts(idle).1, crate::i18n::update_available());
        assert_eq!(
            badge_parts(downloading).1,
            crate::i18n::update_downloading()
        );
        assert_ne!(
            badge_parts(idle).0,
            badge_parts(downloading).0,
            "아이콘도 갈린다"
        );

        // 배지 자리를 눌러도 명령이 나오지 않는다
        let ctx = egui::Context::default();
        let first = run_frame_with(&ctx, 0.0, Vec::new(), downloading);
        let badge_x = 800.0 - RIGHT_GROUP_BASE - 1.0;
        assert!(first.right_group_width > RIGHT_GROUP_BASE);
        let frames = [
            run_frame_with(
                &ctx,
                0.05,
                vec![egui::Event::PointerMoved(egui::pos2(badge_x, 18.0))],
                downloading,
            ),
            run_frame_with(
                &ctx,
                0.10,
                vec![press(egui::pos2(badge_x, 18.0), true)],
                downloading,
            ),
            run_frame_with(
                &ctx,
                0.15,
                vec![press(egui::pos2(badge_x, 18.0), false)],
                downloading,
            ),
        ];
        let commands: Vec<_> = frames.iter().map(|frame| frame.command).collect();
        assert!(
            !commands.contains(&Some(Command::StartUpdate)),
            "받는 중에 눌려 명령이 나왔다: {commands:?}"
        );
    }

    #[test]
    fn 배지_자리에서_눌러도_창이_끌리지_않는다() {
        // D13이 겨냥한 회귀의 나머지 절반 — 끌기 영역이 배지 폭을 셈하지 않으면
        // 배지를 누르는 순간 OS 창 이동 루프가 열려 그 클릭이 삼켜진다
        let badge = UpdateBadge {
            visible: true,
            downloading: false,
            update_enabled: true,
        };
        let ctx = egui::Context::default();
        // 첫 프레임으로 그 프레임의 우측 폭을 얻는다(폭은 글꼴에 따라 달라 미리 알 수 없다)
        let first = run_frame_with(&ctx, 0.0, Vec::new(), badge);
        assert!(
            first.right_group_width > RIGHT_GROUP_BASE,
            "배지가 서면 우측 폭이 넓어져야 한다"
        );
        // 배지가 놓인 자리 — 설정 버튼(36px) 왼쪽
        let badge_x = 800.0 - RIGHT_GROUP_BASE - 1.0;
        let frames = [
            run_frame_with(
                &ctx,
                0.05,
                vec![egui::Event::PointerMoved(egui::pos2(badge_x, 18.0))],
                badge,
            ),
            run_frame_with(
                &ctx,
                0.10,
                vec![press(egui::pos2(badge_x, 18.0), true)],
                badge,
            ),
            run_frame_with(
                &ctx,
                0.15,
                vec![egui::Event::PointerMoved(egui::pos2(badge_x - 30.0, 18.0))],
                badge,
            ),
        ];
        let windows: Vec<_> = frames.iter().map(|frame| frame.window).collect();
        assert!(
            !windows.contains(&Some(WindowRequest::Drag)),
            "배지 위에서 끌기를 요청했다: {windows:?}"
        );
        // **같은 자리**에서 위쪽 변 크기 조절도 비켜야 한다 — 둘 중 하나만 지키면
        // 그 자리를 누르는 클릭이 여전히 삼켜진다
        let window = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(800.0, 600.0));
        assert!(
            over_titlebar_button(badge_x, window, first.right_group_width),
            "배지가 놓인 x에서 크기 조절이 비켜주지 않는다"
        );
    }

    #[test]
    fn 배지가_없으면_우측_폭이_종전과_같다() {
        // 기존 시험(`위쪽_변은_버튼_구간을_비켜준다`)이 174를 전제하므로 이 값이 흔들리면
        // 그 시험이 조용히 다른 것을 재게 된다
        let ctx = egui::Context::default();
        let outcome = run_frame_with(&ctx, 0.0, Vec::new(), UpdateBadge::default());
        assert_eq!(outcome.right_group_width, RIGHT_GROUP_BASE);
    }

    #[test]
    fn 배지가_서면_비켜주는_구간도_함께_넓어진다() {
        // D13이 겨냥한 회귀 — 배지가 우측 그룹을 넓혔는데 이 판정만 기본 폭에 머물면
        // 배지 위쪽 4px에서 크기 조절 루프가 열려 그 클릭이 삼켜진다
        let wide = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(600.0, 100.0));
        let badge_width = 100.0;
        let with_badge = RIGHT_GROUP_BASE + badge_width;
        // 배지가 놓이는 자리 — 기본 폭 밖이지만 넓어진 폭 안이다
        let x = wide.max.x - RIGHT_GROUP_BASE - badge_width / 2.0;

        assert!(
            !over_titlebar_button(x, wide, RIGHT_GROUP_BASE),
            "기본 폭만 보면 이 자리는 버튼 구간 밖이다"
        );
        assert!(
            over_titlebar_button(x, wide, with_badge),
            "배지 폭을 더한 값을 넘기면 비켜줘야 한다"
        );
    }

    #[test]
    fn 위쪽_변은_버튼_구간을_비켜준다() {
        // 버튼 위쪽 4px가 크기 조절에 먹히면 그 버튼을 누를 수 없다.
        // 창 폭 100px에서 좌측 38px·우측 174px 구간이 버튼 자리이므로 여기서는 폭을 넉넉히 잡는다
        let wide = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(600.0, 100.0));
        assert!(over_titlebar_button(10.0, wide, RIGHT_GROUP_BASE)); // 좌측 토글 자리
        assert!(over_titlebar_button(590.0, wide, RIGHT_GROUP_BASE)); // 우측 캡션 버튼 자리
        assert!(!over_titlebar_button(300.0, wide, RIGHT_GROUP_BASE)); // 가운데 — 제목 자리
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

    // ── 타이틀바 끌기·더블클릭 판정 (헤드리스 egui) ──

    /// 드래그 영역 안의 한 점 — 좌우 버튼군을 피한 빈 자리
    const DRAG_POS: egui::Pos2 = egui::pos2(300.0, 18.0);

    /// 타이틀바 한 프레임을 돌리고 이번 프레임의 창 요청을 돌려준다.
    /// 창 조작 판정은 프레임에 걸친 포인터 상태에서 나오므로 한 프레임만으로는 볼 수 없다
    fn run_frame(
        ctx: &egui::Context,
        time: f64,
        events: Vec<egui::Event>,
    ) -> Option<WindowRequest> {
        run_frame_with(ctx, time, events, UpdateBadge::default()).window
    }

    /// 배지를 세운 채 한 프레임 돌리고 **결과 전체**를 돌려준다 —
    /// 배지가 놓인 x를 알려면 그 프레임의 우측 폭(`right_group_width`)이 필요하다
    fn run_frame_with(
        ctx: &egui::Context,
        time: f64,
        events: Vec<egui::Event>,
        badge: UpdateBadge,
    ) -> TitlebarOutcome {
        let input = egui::RawInput {
            time: Some(time),
            events,
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(800.0, TITLEBAR_HEIGHT),
            )),
            ..Default::default()
        };
        let mut outcome = TitlebarOutcome::default();
        let _ = ctx.run_ui(input, |ui| {
            outcome = show_titlebar(
                ui,
                "제목",
                TitlebarState {
                    maximized: false,
                    sidebar_collapsed: false,
                },
                None,
                badge,
            );
        });
        outcome
    }

    fn press(pos: egui::Pos2, pressed: bool) -> egui::Event {
        egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        }
    }

    #[test]
    fn 제자리_클릭은_창을_끌지_않는다() {
        // 누른 즉시 끌기를 요청하면 OS 창 이동 루프가 열려, 손이 미세하게 흔들린 만큼
        // 창이 따라 움직인다 — 최대화 토글이 그 위에 겹치면 화면이 떨리며 바뀐다
        let ctx = egui::Context::default();
        let frames = [
            run_frame(&ctx, 0.0, vec![egui::Event::PointerMoved(DRAG_POS)]),
            run_frame(&ctx, 0.05, vec![press(DRAG_POS, true)]),
            run_frame(&ctx, 0.10, vec![press(DRAG_POS, false)]),
        ];
        assert!(
            !frames.contains(&Some(WindowRequest::Drag)),
            "제자리 클릭에서 끌기를 요청했다: {frames:?}"
        );
    }

    #[test]
    fn 더블클릭은_끌기_없이_최대화만_토글한다() {
        // 프레임 간격은 egui가 더블클릭으로 묶는 시간(`max_double_click_delay`, 0.3초) 안이어야 한다
        let ctx = egui::Context::default();
        let frames = [
            run_frame(&ctx, 0.0, vec![egui::Event::PointerMoved(DRAG_POS)]),
            run_frame(&ctx, 0.05, vec![press(DRAG_POS, true)]),
            run_frame(&ctx, 0.10, vec![press(DRAG_POS, false)]),
            run_frame(&ctx, 0.15, vec![press(DRAG_POS, true)]),
            run_frame(&ctx, 0.20, vec![press(DRAG_POS, false)]),
        ];
        assert!(
            !frames.contains(&Some(WindowRequest::Drag)),
            "더블클릭 도중 끌기를 요청했다: {frames:?}"
        );
        assert!(
            frames.contains(&Some(WindowRequest::ToggleMaximize)),
            "더블클릭이 최대화 토글로 이어지지 않았다: {frames:?}"
        );
    }

    #[test]
    fn 설정_메뉴_폭_안에서_라벨이_한_줄로_그려진다() {
        // 사용자가 본 결함이 그대로 재현되는 자리다 — 폭이 모자라면 `Open source licenses`가
        // 두 줄로 접힌다. **폭 계산을 되풀이해 견주지 않는다**(그러면 늘 참이라 아무것도
        // 지키지 못한다) — 잰 폭을 줄바꿈 한계로 삼아 **실제로 몇 줄이 나오는지**를 본다
        let ctx = egui::Context::default();
        for language in [
            crate::app::settings::LanguageSetting::Korean,
            crate::app::settings::LanguageSetting::English,
        ] {
            let _guard = crate::i18n::LanguageGuard::lock(language);
            let mut folded: Vec<(String, usize)> = Vec::new();
            let _ = ctx.run_ui(Default::default(), |ui| {
                // **실제 팝업과 같은 순서로 스타일을 세운다** — egui가 메뉴 스타일을 먼저
                // 입히고 그 위에 앱 토큰이 온다. 이 순서를 빼고 재면 여백이 전역 기본값
                // (4px)으로 잡혀, 실제로는 12px씩 떼는 화면과 다른 폭을 검사하게 된다
                egui::containers::menu::menu_style(ui.style_mut());
                theme::menu_style(ui);
                let width = settings_menu_width(ui);
                let font = egui::TextStyle::Button.resolve(ui.style());
                // 항목은 버튼이라 좌우 여백을 먼저 떼고 남은 자리에 글자가 앉는다
                let wrap = width - ui.style().spacing.button_padding.x * 2.0;
                for label in [
                    crate::i18n::settings_title(),
                    crate::i18n::titlebar_updates(),
                    crate::i18n::titlebar_release_notes(),
                    crate::i18n::titlebar_licenses(),
                    crate::i18n::titlebar_about(),
                ] {
                    let rows = ui
                        .painter()
                        .layout(label.to_owned(), font.clone(), theme::TEXT, wrap)
                        .rows
                        .len();
                    if rows != 1 {
                        folded.push((label.to_owned(), rows));
                    }
                }
            });
            assert!(
                folded.is_empty(),
                "{language:?}에서 접힌 항목이 있다: {folded:?}"
            );
        }
    }

    #[test]
    fn 충분히_움직이면_창을_끈다() {
        // 끌기 자체는 살아 있어야 한다 — egui는 클릭도 받는 위젯에서 포인터가
        // 조금 움직인 뒤에야 끌기로 판정한다(`max_click_dist`)
        let ctx = egui::Context::default();
        let moved = DRAG_POS + egui::vec2(20.0, 0.0);
        let frames = [
            run_frame(&ctx, 0.0, vec![egui::Event::PointerMoved(DRAG_POS)]),
            run_frame(&ctx, 0.05, vec![press(DRAG_POS, true)]),
            run_frame(&ctx, 0.10, vec![egui::Event::PointerMoved(moved)]),
            run_frame(&ctx, 0.15, vec![egui::Event::PointerMoved(moved)]),
        ];
        assert!(
            frames.contains(&Some(WindowRequest::Drag)),
            "충분히 움직였는데 끌기를 요청하지 않았다: {frames:?}"
        );
    }
}
