//! 공용 위젯 — 여러 화면 요소가 같은 규칙으로 그리는 조각들.
//!
//! 아이콘 버튼은 타이틀바·탭 스트립·주소창이, 디자인 버튼은 원격 단계 화면과 사이트 관리자가,
//! 폼 조각(라벨 96px + 필드 28px)은 사이트 관리자의 세 탭이 같은 규칙으로 쓴다.
//! 이 규칙을 화면마다 각자 구현하면 hover 색·여백·테두리가 조금씩 갈린다.
use std::sync::Arc;

use crate::remote::connection::TransferDirection;
use crate::ui::theme;
use eframe::egui;

/// 아이콘 글꼴 기본 크기 — 타이틀바 캡션 버튼이 쓰던 값이 기준이다
const DEFAULT_ICON_PX: f32 = 16.0;

/// 전송 방향 글리프 (인벤토리 #44) — 아이콘 글꼴에서 가져온다 (프로젝트 규약)
pub const UPLOAD_GLYPH: &str = egui_phosphor::regular::ARROW_UP;
pub const DOWNLOAD_GLYPH: &str = egui_phosphor::regular::ARROW_DOWN;

/// 전송 방향을 나타내는 글리프와 색 — **전송 큐와 탭 스트립이 함께 쓴다**.
///
/// 큐 화면에만 두지 않는 이유: 탭 스트립도 "이 탭이 받는 곳인가 올리는 곳인가"를 같은 뜻으로
/// 보여야 하는데(FR-54), 각자 글리프를 정하면 같은 개념이 화면마다 다른 모양이 된다
pub fn direction_mark(direction: TransferDirection) -> (&'static str, egui::Color32) {
    match direction {
        TransferDirection::Upload => (UPLOAD_GLYPH, theme::ACCENT),
        TransferDirection::Download => (DOWNLOAD_GLYPH, theme::OK_TEXT),
    }
}

/// 이 글자가 **아이콘 글꼴(phosphor)의 것**인가 — 사용자 정의 영역(U+E000~U+F8FF)에 있는가.
///
/// 이 프로젝트의 아이콘은 전부 `egui_phosphor`에서 가져온다(AGENTS 규약). 원본 디자인의
/// 유니코드 기호(`⏸`·`✕`·`⧉` 등)를 그대로 쓰면 **이 앱의 글꼴에 없어 두부(`?`)로 그려진다** —
/// 실제로 도크 아이콘 셋이 그렇게 나갔다(2026-08-05). 시험이 이 함수로 규약을 지킨다
pub fn is_icon_font(glyph: &str) -> bool {
    let mut chars = glyph.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    chars.next().is_none() && ('\u{E000}'..='\u{F8FF}').contains(&first)
}

/// 아이콘 버튼 hover 배경의 모서리 반경
const HOVER_CORNER_RADIUS: u8 = 4;

/// 아이콘 버튼에 마우스가 올라갔을 때 까는 배경 — **정사각형에 둥근 모서리**다 (사용자 결정).
///
/// 버튼이 차지한 자리는 스트립 높이에 맞춰 세로로 길쭉한 경우가 많은데(탭 닫기·타이틀바 버튼),
/// 그대로 칠하면 버튼마다 배경 모양이 달라 보인다. 자리의 **짧은 변**을 한 변으로 삼아
/// 가운데에 정사각형을 그리면 어느 자리에서든 같은 표식이 된다
pub fn hover_backdrop(painter: &egui::Painter, rect: egui::Rect, fill: egui::Color32) {
    let side = rect.width().min(rect.height());
    let square = egui::Rect::from_center_size(rect.center(), egui::Vec2::splat(side));
    painter.rect_filled(square, HOVER_CORNER_RADIUS, fill);
}

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
        hover_backdrop(ui.painter(), rect, hover_fill);
    }
    // 아이콘 문자열이 비면 영역과 hover 배경만 내준다 — 아이콘을 글리프가 아니라
    // 직접 그리는 버튼(탭 스트립의 분할 버튼)이 이 경로를 쓴다
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

// ── 디자인 버튼 (원본 `FileExplorer-FTP.dc.html:249`·`:409`·`:496-498`) ──

/// 버튼 테두리 두께.
///
/// **폭 계산이 이 값을 알아야 한다** — egui는 버튼 안쪽 여백을 `button_padding − 테두리 두께`로
/// 잡으므로(`Style::button_style`), 이것을 빼지 않으면 `design_button_width`가 실제보다 넓게 센다
const BUTTON_STROKE: f32 = 1.0;

/// 디자인 값으로 그리는 보조 버튼 — 채움 `#252525` · 테두리 `#3A3A3A` · hover `#2E2E2E`.
///
/// egui 기본 버튼 색(`#2A2A2A`·기본 테두리·hover `#383838`)과 다르므로 스타일을 **국소로**
/// 덮는다. 전역 팔레트를 바꾸면 기존 화면의 버튼까지 함께 바뀐다.
/// 글자색은 자리마다 달라(연결 중 취소 `#C8C8C8` · 실패 화면·대화 `#D8D8D8`) 인자로 받는다.
///
/// 폭은 원본이 좌우 여백(`padding 0 Npx`)으로 정하므로 글자에 맞춘다 —
/// 3열 격자처럼 폭이 정해진 자리는 `min_size.x`로 넓힌다
pub fn design_button(
    ui: &mut egui::Ui,
    label: &str,
    text_color: egui::Color32,
    pad_x: f32,
    min_size: egui::Vec2,
) -> egui::Response {
    ui.scope(|ui| {
        ui.spacing_mut().button_padding = egui::vec2(pad_x, 0.0);
        let widgets = &mut ui.style_mut().visuals.widgets;
        for (state, fill) in [
            (&mut widgets.inactive, theme::HEADER_BG),
            (&mut widgets.hovered, theme::ROW_HOT),
            // 눌린 동안은 hover보다 한 단계 밝다 — 같은 색이면 눌렀다는 것이 손에 잡히지
            // 않는다. 값은 팔레트가 이미 "눌림·선택"으로 정해 둔 것을 쓴다 (2026-08-16 검토)
            (&mut widgets.active, theme::CONTROL_ACTIVE),
        ] {
            state.weak_bg_fill = fill;
            state.bg_fill = fill;
            state.bg_stroke = egui::Stroke::new(BUTTON_STROKE, theme::BORDER_CONTROL);
            state.corner_radius = egui::CornerRadius::ZERO;
            // 눌렸을 때 커지지 않는다 — 디자인은 상태에 따라 크기가 변하지 않는다
            state.expansion = 0.0;
        }
        ui.add(egui::Button::new(egui::RichText::new(label).color(text_color)).min_size(min_size))
    })
    .inner
}

/// `design_button`이 차지할 폭 — 가운데 정렬처럼 **그리기 전에** 폭을 알아야 하는 자리가 쓴다.
///
/// 여백에서 테두리 두께를 빼는 것은 egui가 그렇게 그리기 때문이다(`BUTTON_STROKE` 참조) —
/// 빼지 않으면 버튼 행이 계산된 중앙에서 1px 밀린다
pub fn design_button_width(ui: &egui::Ui, label: &str, pad_x: f32) -> f32 {
    let font = egui::TextStyle::Button.resolve(ui.style());
    ui.painter()
        .layout_no_wrap(label.to_owned(), font, theme::TEXT)
        .size()
        .x
        + (pad_x - BUTTON_STROKE) * 2.0
}

// ── 폼 조각 (원본 `FileExplorer-FTP.dc.html:421-434`) ──

/// 라벨 열 폭 — 사이트 관리자 일반 탭의 모든 행이 이 폭에 맞춰 필드를 시작한다
pub const FORM_LABEL_WIDTH: f32 = 96.0;
/// 입력·드롭다운 필드 높이
pub const FORM_FIELD_HEIGHT: f32 = 28.0;
/// 라벨과 필드 사이 간격
pub const FORM_GAP: f32 = 10.0;
/// 필드 안 좌우 여백
const FORM_FIELD_PAD_X: f32 = 8.0;
/// 라벨·필드 글자 크기
pub const FORM_FONT_PX: f32 = 13.0;
/// 드롭다운 오른쪽 캐럿 — 아이콘 글꼴에서 (프로젝트 규약)
const FORM_CARET: &str = egui_phosphor::regular::CARET_DOWN;
const FORM_CARET_PX: f32 = 11.0;
/// 캐럿과 값 사이 간격
const FORM_CARET_GAP: f32 = 8.0;
/// 비활성 필드 배경 — 원본이 채움을 한 단계 죽여 조작할 수 없음을 보인다 (`:863`·`:874`)
const FORM_DISABLED_BG: egui::Color32 = egui::Color32::from_rgb(0x1A, 0x1A, 0x1A);
/// 비밀번호 마스킹 문자 — **egui 기본값(`•`)이 아니라 디자인의 `●`다** (인벤토리 #75).
/// 그래서 `TextEdit::password`가 아니라 아래 `layouter`로 직접 가린다
const MASK_CHAR: char = '●';

/// 라벨 한 칸 — 96px 고정 폭, 세로 가운데 정렬. 비활성이면 흐려진다
pub fn form_label(ui: &mut egui::Ui, text: &str, enabled: bool) {
    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(FORM_LABEL_WIDTH, FORM_FIELD_HEIGHT),
        egui::Sense::hover(),
    );
    inline_label(ui, rect, text, enabled);
}

/// 폭을 글자에 맞추는 라벨 — `포트(P):`처럼 행 가운데 끼어드는 것이 쓴다
pub fn form_inline_label(ui: &mut egui::Ui, text: &str, enabled: bool) {
    let width = ui
        .painter()
        .layout_no_wrap(
            text.to_owned(),
            egui::FontId::proportional(FORM_FONT_PX),
            theme::HEADER_TEXT,
        )
        .size()
        .x;
    let (rect, _) =
        ui.allocate_exact_size(egui::vec2(width, FORM_FIELD_HEIGHT), egui::Sense::hover());
    inline_label(ui, rect, text, enabled);
}

fn inline_label(ui: &egui::Ui, rect: egui::Rect, text: &str, enabled: bool) {
    ui.painter().text(
        egui::pos2(rect.left(), rect.center().y),
        egui::Align2::LEFT_CENTER,
        text,
        egui::FontId::proportional(FORM_FONT_PX),
        if enabled {
            theme::HEADER_TEXT
        } else {
            theme::TEXT_DIM
        },
    );
}

/// 입력 필드 — `#151515` 웰 위의 한 줄 편집기.
///
/// **비활성일 때는 편집기를 두지 않고 값만 흐리게 그린다** — `add_enabled`로 흐리게만 하면
/// 클릭·캐럿이 살아 있어 "눌리는데 안 되는 것"처럼 보인다.
///
/// `id_salt`는 같은 화면의 필드끼리 위젯 id가 겹치지 않게 한다.
/// `masked`면 글자를 `●`로 가린다(값 자체는 그대로 편집된다)
pub fn text_field(
    ui: &mut egui::Ui,
    id_salt: &str,
    value: &mut String,
    size: egui::Vec2,
    enabled: bool,
    masked: bool,
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::hover());
    paint_well(ui, rect, enabled);
    let inner = inner_rect(rect);
    if !enabled {
        ui.painter().text(
            egui::pos2(inner.left(), inner.center().y),
            egui::Align2::LEFT_CENTER,
            display_text(value, masked),
            egui::FontId::proportional(FORM_FONT_PX),
            theme::TEXT_DIM,
        );
        return response;
    }
    let mut mask = |ui: &egui::Ui, text: &dyn egui::TextBuffer, wrap: f32| -> Arc<egui::Galley> {
        ui.painter().layout(
            display_text(text.as_str(), true),
            egui::FontId::proportional(FORM_FONT_PX),
            theme::TEXT,
            wrap,
        )
    };
    let mut edit = egui::TextEdit::singleline(value)
        .id_salt(id_salt)
        .frame(egui::Frame::NONE)
        .font(egui::FontId::proportional(FORM_FONT_PX))
        .text_color(theme::TEXT)
        .desired_width(inner.width())
        .clip_text(true);
    if masked {
        edit = edit.layouter(&mut mask);
    }
    // 웰은 위에서 이미 그리고 자리도 잡았다 — 편집기는 그 안쪽에만 얹는다
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(inner)
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );
    child.add(edit)
}

/// 드롭다운 필드 — 고른 항목의 번호를 돌려준다(고르지 않았으면 `None`).
///
/// 열거형이 아니라 번호를 주고받는 이유: 필드마다 값 타입이 달라 제네릭으로 묶으면
/// 호출부가 오히려 길어진다. 호출부가 자기 목록 순서로 되돌린다
pub fn dropdown_field(
    ui: &mut egui::Ui,
    id_salt: &str,
    current: &str,
    width: f32,
    enabled: bool,
    options: &[&str],
) -> Option<usize> {
    let sense = if enabled {
        egui::Sense::click()
    } else {
        egui::Sense::hover()
    };
    let (rect, _) =
        ui.allocate_exact_size(egui::vec2(width, FORM_FIELD_HEIGHT), egui::Sense::hover());
    // 팝업이 자기 버튼을 기억하는 열쇠가 이 id다 — 자리(rect)에서 자동으로 뽑으면 같은 폼의
    // 필드끼리 겹칠 수 있어 호출부가 준 이름으로 잡는다
    let response = ui.interact(rect, ui.id().with(id_salt), sense);
    let response = response.on_hover_cursor(if enabled {
        egui::CursorIcon::PointingHand
    } else {
        egui::CursorIcon::Default
    });
    paint_well(ui, rect, enabled);
    let inner = inner_rect(rect);
    let text_color = if enabled {
        theme::TEXT
    } else {
        theme::TEXT_DIM
    };
    let caret_color = if enabled {
        theme::TEXT_MUTED
    } else {
        theme::TEXT_DIM
    };
    ui.painter().text(
        egui::pos2(inner.right(), inner.center().y),
        egui::Align2::RIGHT_CENTER,
        FORM_CARET,
        egui::FontId::proportional(FORM_CARET_PX),
        caret_color,
    );
    // 값은 캐럿 자리를 뺀 나머지에만 그린다 — 긴 값이 캐럿을 덮지 않는다
    let caret_width = ui
        .painter()
        .layout_no_wrap(
            FORM_CARET.to_owned(),
            egui::FontId::proportional(FORM_CARET_PX),
            caret_color,
        )
        .size()
        .x;
    let value = ui.painter().layout(
        current.to_owned(),
        egui::FontId::proportional(FORM_FONT_PX),
        text_color,
        (inner.width() - caret_width - FORM_CARET_GAP).max(0.0),
    );
    ui.painter().galley(
        egui::pos2(inner.left(), inner.center().y - value.size().y / 2.0),
        value,
        text_color,
    );

    if !enabled {
        return None;
    }
    let mut chosen = None;
    egui::Popup::menu(&response)
        .frame(
            // 모서리는 적지 않는다 — `Frame::menu`가 테마의 공통 값을 읽는다
            // (`theme::MENU_CORNER_RADIUS`)
            egui::Frame::menu(ui.style())
                .fill(theme::SURFACE_BG)
                .stroke(egui::Stroke::new(1.0, theme::PANE_BORDER)),
        )
        .show(|ui| {
            ui.set_width(width);
            // 항목이 많으면(글꼴은 수십 개다) 목록이 화면 위아래를 넘어 끝을 볼 수 없다.
            // 여기서 높이를 끊고 스크롤을 붙인다 — 항목이 적은 드롭다운은 이 상한에
            // 닿지 않으므로 종전과 같이 보인다
            egui::ScrollArea::vertical()
                .max_height(MENU_MAX_HEIGHT)
                .show(ui, |ui| {
                    ui.set_width(width);
                    for (index, option) in options.iter().enumerate() {
                        if menu_row(ui, option) {
                            chosen = Some(index);
                            ui.close();
                        }
                    }
                });
        });
    chosen
}

// ── 라디오·체크·스피너 (원본 `FileExplorer-FTP.dc.html:446-447`·`:453`·`:458-463`) ──

/// 드롭다운 목록의 높이 상한 — 이 높이를 넘으면 스크롤한다.
///
/// 글꼴 목록은 PC에 따라 수십 개라 상한이 없으면 화면 위아래로 넘쳐 끝을 볼 수 없다.
/// 값은 행 높이(28px)의 열 줄 남짓 — 한눈에 훑을 만하면서 화면을 다 먹지 않는다
const MENU_MAX_HEIGHT: f32 = 280.0;

/// 라디오 원 지름과 안쪽 점 (`:446-447`)
const RADIO_SIZE: f32 = 14.0;
const RADIO_DOT: f32 = 6.0;
/// 체크 사각 한 변과 글리프 (`:453`)
const CHECK_SIZE: f32 = 14.0;
const CHECK_GLYPH: &str = egui_phosphor::regular::CHECK;
const CHECK_GLYPH_PX: f32 = 10.0;
/// 표시와 글자 사이 (`:445`·`:452` `gap:8px`)
const MARK_GAP: f32 = 8.0;
/// 고르지 않은 라디오·체크의 테두리 (`:854`·`:861` `#5A5A5A`)
const MARK_EDGE_OFF: egui::Color32 = egui::Color32::from_rgb(0x5A, 0x5A, 0x5A);
/// 스피너 필드 (`:458-463`)
pub const SPINNER_WIDTH: f32 = 92.0;
pub const SPINNER_HEIGHT: f32 = 26.0;
/// ▲▼ 칸 폭과 글리프 크기
const SPINNER_ARROW_WIDTH: f32 = 16.0;
const SPINNER_ARROW_PX: f32 = 8.0;

/// 라디오 한 줄 — 원 + 라벨. 눌렸으면 `true` (인벤토리 #77~79·#84·#85).
///
/// egui의 `radio_value`를 쓰지 않는 이유: 그것은 원 크기·점 지름·색이 스타일에 묶여 있어
/// 디자인 값(14px 원·6px 점·선택 `#4A9EFF`)으로 맞추려면 결국 직접 그리게 된다
pub fn radio_row(ui: &mut egui::Ui, label: &str, selected: bool, hint: Option<&str>) -> bool {
    let text = ui.painter().layout_no_wrap(
        label.to_owned(),
        egui::FontId::proportional(FORM_FONT_PX),
        theme::TEXT,
    );
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(RADIO_SIZE + MARK_GAP + text.size().x, FORM_FIELD_HEIGHT),
        egui::Sense::click(),
    );
    let painter = ui.painter();
    let center = egui::pos2(rect.left() + RADIO_SIZE / 2.0, rect.center().y);
    painter.circle_stroke(
        center,
        RADIO_SIZE / 2.0,
        egui::Stroke::new(
            1.0,
            if selected {
                theme::ACCENT
            } else {
                MARK_EDGE_OFF
            },
        ),
    );
    if selected {
        painter.circle_filled(center, RADIO_DOT / 2.0, theme::ACCENT);
    }
    painter.galley(
        egui::pos2(
            rect.left() + RADIO_SIZE + MARK_GAP,
            rect.center().y - text.size().y / 2.0,
        ),
        text,
        theme::TEXT,
    );
    let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
    // 낱말만으로 뜻이 닿지 않는 선택지(능동형·수동형)는 한 줄 설명을 단다
    match hint {
        Some(hint) => response.on_hover_text(hint).clicked(),
        None => response.clicked(),
    }
}

/// 체크박스 한 줄 — 사각 + 라벨. 눌렸으면 `true` (인벤토리 #80)
pub fn check_row(ui: &mut egui::Ui, label: &str, checked: bool) -> bool {
    let text = ui.painter().layout_no_wrap(
        label.to_owned(),
        egui::FontId::proportional(FORM_FONT_PX),
        theme::TEXT,
    );
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(CHECK_SIZE + MARK_GAP + text.size().x, FORM_FIELD_HEIGHT),
        egui::Sense::click(),
    );
    let painter = ui.painter();
    let box_rect = egui::Rect::from_min_size(
        egui::pos2(rect.left(), rect.center().y - CHECK_SIZE / 2.0),
        egui::vec2(CHECK_SIZE, CHECK_SIZE),
    );
    painter.rect(
        box_rect,
        0.0,
        if checked {
            theme::ACCENT
        } else {
            theme::WELL_BG
        },
        egui::Stroke::new(
            1.0,
            if checked {
                theme::ACCENT
            } else {
                MARK_EDGE_OFF
            },
        ),
        egui::StrokeKind::Inside,
    );
    if checked {
        painter.text(
            box_rect.center(),
            egui::Align2::CENTER_CENTER,
            CHECK_GLYPH,
            egui::FontId::proportional(CHECK_GLYPH_PX),
            egui::Color32::WHITE,
        );
    }
    painter.galley(
        egui::pos2(
            rect.left() + CHECK_SIZE + MARK_GAP,
            rect.center().y - text.size().y / 2.0,
        ),
        text,
        theme::TEXT,
    );
    response
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .clicked()
}

// ── 토글 스위치 (설정 화면 — FR-47) ──

/// 스위치 트랙의 가로·세로. 세로가 곧 트랙의 지름이라 이 값이 둥글기를 정한다
const TOGGLE_TRACK_W: f32 = 36.0;
const TOGGLE_TRACK_H: f32 = 20.0;
/// 손잡이 반지름 — 트랙 안쪽에 여백(`TOGGLE_PAD`)을 남기고 들어간다
const TOGGLE_KNOB_R: f32 = 7.0;
const TOGGLE_PAD: f32 = 3.0;
// 손잡이가 트랙 두께를 넘으면 위아래가 잘려 보인다. 값을 손볼 때 눈으로 확인하지 않아도
// 되도록 **빌드 자체가 막는다** — 시험으로 두면 상수끼리의 비교라 clippy가 걷어낸다
const _: () = assert!(TOGGLE_KNOB_R + TOGGLE_PAD <= TOGGLE_TRACK_H / 2.0);
/// 손잡이 중심의 x 좌표 — 트랙 왼쪽 끝과 켜짐 여부로 정해진다.
///
/// 그리기와 떼어 둔 이유: 위치 계산은 눈으로 확인하기 어려운데 시험으로는 쉽게 고정된다
fn toggle_knob_x(track_left: f32, on: bool) -> f32 {
    if on {
        track_left + TOGGLE_TRACK_W - TOGGLE_PAD - TOGGLE_KNOB_R
    } else {
        track_left + TOGGLE_PAD + TOGGLE_KNOB_R
    }
}

/// 라벨 + 오른쪽 끝 on/off 스위치 한 줄 (FR-47). 눌렸으면 `true`.
///
/// 체크박스(`check_row`)가 아니라 스위치인 것은 요청 문구가 "on/off 토글 버튼"이기
/// 때문이다. 높이·글꼴·글자색은 `check_row`와 같은 값을 써서 같은 폼 안에서 줄이 어긋나지 않는다.
///
/// **비활성 상태를 두지 않는다** — 설정 화면의 토글은 전부 항상 누를 수 있다(plan Edge Case).
/// 조건부로 잠기는 항목이 생기면 그때 `enabled` 인자를 더한다
pub fn toggle_row(ui: &mut egui::Ui, label: &str, on: bool) -> bool {
    let row_width = ui.available_width().max(TOGGLE_TRACK_W);
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(row_width, FORM_FIELD_HEIGHT),
        egui::Sense::click(),
    );

    // 라벨이 길면 스위치를 밀어내지 않고 말줄임한다 — 스위치는 오른쪽 끝 고정이다
    let label_width = (rect.width() - TOGGLE_TRACK_W - FORM_GAP).max(0.0);
    let mut job = egui::text::LayoutJob::simple(
        label.to_owned(),
        egui::FontId::proportional(FORM_FONT_PX),
        theme::TEXT,
        label_width,
    );
    job.wrap.max_rows = 1;
    job.wrap.break_anywhere = true;
    let text = ui.painter().layout_job(job);

    let track = egui::Rect::from_min_size(
        egui::pos2(
            rect.right() - TOGGLE_TRACK_W,
            rect.center().y - TOGGLE_TRACK_H / 2.0,
        ),
        egui::vec2(TOGGLE_TRACK_W, TOGGLE_TRACK_H),
    );
    let painter = ui.painter();
    painter.galley(
        egui::pos2(rect.left(), rect.center().y - text.size().y / 2.0),
        text,
        theme::TEXT,
    );
    // 꺼짐 채움에 팔레트의 `BORDER_CONTROL`을 그대로 쓴다 — 스위치는 체크박스와 달리
    // 테두리만으로는 "눌러서 켜는 것"임이 드러나지 않아 면으로 보여야 하고, 그 면은
    // 배경보다 밝으면서 켜짐(`ACCENT`)과 구분되면 된다. 같은 값을 지역 상수로 다시 만들면
    // 팔레트 정본이 갈려 나중에 한쪽만 바뀐다(`theme.rs`가 경계하는 상황)
    painter.rect_filled(
        track,
        TOGGLE_TRACK_H / 2.0,
        if on {
            theme::ACCENT
        } else {
            theme::BORDER_CONTROL
        },
    );
    painter.circle_filled(
        egui::pos2(toggle_knob_x(track.left(), on), track.center().y),
        TOGGLE_KNOB_R,
        egui::Color32::WHITE,
    );

    response
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .clicked()
}

/// 값을 ▲▼로 올리고 내리는 필드 (인벤토리 #81).
///
/// 값은 **범위 안으로 클램프**해 돌려준다 — 화살표를 계속 눌러도 밖으로 나가지 않는다.
/// 비활성이면 화살표가 반응하지 않고 채움·글자가 흐려진다
pub fn spinner_field(
    ui: &mut egui::Ui,
    value: u8,
    range: std::ops::RangeInclusive<u8>,
    enabled: bool,
) -> u8 {
    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(SPINNER_WIDTH, SPINNER_HEIGHT),
        egui::Sense::hover(),
    );
    paint_well(ui, rect, enabled);
    let arrows = egui::Rect::from_min_max(
        egui::pos2(rect.right() - SPINNER_ARROW_WIDTH, rect.top() + 1.0),
        egui::pos2(rect.right() - 1.0, rect.bottom() - 1.0),
    );
    let text_color = if enabled {
        theme::TEXT
    } else {
        theme::TEXT_DIM
    };
    // 값은 화살표 칸을 뺀 자리에 오른쪽 정렬한다 (`:459`)
    ui.painter().text(
        egui::pos2(arrows.left() - FORM_FIELD_PAD_X, rect.center().y),
        egui::Align2::RIGHT_CENTER,
        value.to_string(),
        egui::FontId::proportional(FORM_FONT_PX),
        text_color,
    );
    ui.painter().line_segment(
        [
            egui::pos2(arrows.left() - 0.5, rect.top()),
            egui::pos2(arrows.left() - 0.5, rect.bottom()),
        ],
        egui::Stroke::new(1.0, theme::BORDER_CONTROL),
    );

    let mut next = value.clamp(*range.start(), *range.end());
    let half = arrows.height() / 2.0;
    for (index, glyph) in [
        egui_phosphor::regular::CARET_UP,
        egui_phosphor::regular::CARET_DOWN,
    ]
    .into_iter()
    .enumerate()
    {
        let cell = egui::Rect::from_min_size(
            egui::pos2(arrows.left(), arrows.top() + index as f32 * half),
            egui::vec2(arrows.width(), half),
        );
        let response = ui.interact(
            cell,
            ui.id().with(("spinner", index)),
            if enabled {
                egui::Sense::click()
            } else {
                egui::Sense::hover()
            },
        );
        if enabled && response.hovered() {
            ui.painter().rect_filled(cell, 0.0, theme::ROW_HOT);
        }
        ui.painter().text(
            cell.center(),
            egui::Align2::CENTER_CENTER,
            glyph,
            egui::FontId::proportional(SPINNER_ARROW_PX),
            if enabled {
                theme::TEXT_MUTED
            } else {
                theme::TEXT_DIM
            },
        );
        if response.clicked() {
            next = if index == 0 {
                next.saturating_add(1)
            } else {
                next.saturating_sub(1)
            };
        }
    }
    // 두 화살표 사이 구분선 (`:462`)
    ui.painter().line_segment(
        [
            egui::pos2(arrows.left(), arrows.center().y),
            egui::pos2(arrows.right(), arrows.center().y),
        ],
        egui::Stroke::new(1.0, theme::BORDER_CONTROL),
    );
    next.clamp(*range.start(), *range.end())
}

// ── 진행 막대 (원본 `FileExplorer-FTP.dc.html:290`·`:323`) ──

/// 진행 막대의 빈 트랙 색 — 큐 셀과 상태 표시줄이 같은 값을 쓴다
const PROGRESS_TRACK: egui::Color32 = egui::Color32::from_rgb(0x2A, 0x2A, 0x2A);

/// 진행 막대 — 트랙 위에 비율만큼 채운다.
///
/// 큐 셀(110×6)과 상태 표시줄(240×6)이 같은 부품을 쓴다 — 폭·높이·색을 인자로 받는 이유다.
/// `ratio`가 `None`이면 **트랙만 그린다**(크기를 몰라 진행률을 셀 수 없는 전송 — `—`로 보이는 것과 같은 뜻)
pub fn progress_bar(
    ui: &mut egui::Ui,
    size: egui::Vec2,
    ratio: Option<f32>,
    fill: egui::Color32,
) -> egui::Rect {
    let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
    ui.painter().rect_filled(rect, 0.0, PROGRESS_TRACK);
    if let Some(ratio) = ratio {
        let width = rect.width() * ratio.clamp(0.0, 1.0);
        if width > 0.0 {
            ui.painter().rect_filled(
                egui::Rect::from_min_size(rect.min, egui::vec2(width, rect.height())),
                0.0,
                fill,
            );
        }
    }
    rect
}

/// 드롭다운 팝업의 한 줄 — 눌렸으면 `true`
fn menu_row(ui: &mut egui::Ui, label: &str) -> bool {
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), FORM_FIELD_HEIGHT),
        egui::Sense::click(),
    );
    if response.hovered() {
        ui.painter().rect_filled(rect, 0.0, theme::MENU_HOT);
    }
    ui.painter().text(
        egui::pos2(rect.left() + FORM_FIELD_PAD_X, rect.center().y),
        egui::Align2::LEFT_CENTER,
        label,
        egui::FontId::proportional(FORM_FONT_PX),
        theme::TEXT,
    );
    response.clicked()
}

/// 필드 웰 — 채움 + 1px 테두리. 테두리는 **안쪽**으로 그려 높이가 28px를 넘지 않게 한다
fn paint_well(ui: &egui::Ui, rect: egui::Rect, enabled: bool) {
    ui.painter().rect(
        rect,
        0.0,
        if enabled {
            theme::WELL_BG
        } else {
            FORM_DISABLED_BG
        },
        egui::Stroke::new(1.0, theme::BORDER_CONTROL),
        egui::StrokeKind::Inside,
    );
}

/// 웰 안쪽 — 좌우 8px 여백을 뺀 자리
fn inner_rect(rect: egui::Rect) -> egui::Rect {
    egui::Rect::from_min_max(
        egui::pos2(rect.left() + FORM_FIELD_PAD_X, rect.top()),
        egui::pos2(rect.right() - FORM_FIELD_PAD_X, rect.bottom()),
    )
}

/// 화면에 보일 글자 — 가릴 것이면 글자 수만큼 `●`로 바꾼다
fn display_text(value: &str, masked: bool) -> String {
    if masked {
        std::iter::repeat_n(MASK_CHAR, value.chars().count()).collect()
    } else {
        value.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 토글_손잡이는_켜짐_꺼짐에_따라_트랙_양끝에_붙는다() {
        let left = 100.0;
        let off = toggle_knob_x(left, false);
        let on = toggle_knob_x(left, true);
        assert!(off < on, "켜면 손잡이가 오른쪽으로 가야 한다");
        // 양쪽 모두 트랙 안에 완전히 들어가야 한다 — 원이 테두리를 넘으면 잘려 보인다
        assert!(
            off - TOGGLE_KNOB_R >= left,
            "꺼짐 손잡이가 트랙 왼쪽을 넘었다"
        );
        assert!(
            on + TOGGLE_KNOB_R <= left + TOGGLE_TRACK_W,
            "켜짐 손잡이가 트랙 오른쪽을 넘었다"
        );
        // 두 상태가 트랙 중심을 기준으로 대칭이어야 좌우 여백이 같아 보인다
        let center = left + TOGGLE_TRACK_W / 2.0;
        assert!(
            ((center - off) - (on - center)).abs() < f32::EPSILON,
            "좌우 여백이 다르다"
        );
    }

    #[test]
    fn 토글을_누르면_바뀜을_알린다() {
        // Acceptance ① — 배선(`Sense::click()` → `.clicked()`)이 실제로 이어졌는지는
        // 눈으로는 확인되지 않는다. 포인터 이벤트를 직접 넣어 반환값을 본다
        fn press(pos: egui::Pos2, pressed: bool) -> egui::Event {
            egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: egui::Modifiers::NONE,
            }
        }
        fn frame(ctx: &egui::Context, time: f64, events: Vec<egui::Event>) -> bool {
            let input = egui::RawInput {
                time: Some(time),
                events,
                ..Default::default()
            };
            let mut toggled = false;
            let _ = ctx.run_ui(input, |ui| {
                toggled = toggle_row(ui, "닫기 시 트레이로", false);
            });
            toggled
        }

        let ctx = egui::Context::default();
        // 첫 프레임에 위젯이 자리를 잡아야 그다음 클릭이 그 자리에 닿는다
        assert!(
            !frame(&ctx, 0.0, Vec::new()),
            "그리기만 했는데 눌렸다고 한다"
        );
        let spot = egui::pos2(20.0, 20.0);
        assert!(
            !frame(&ctx, 0.05, vec![press(spot, true)]),
            "누르는 중에 이미 바뀜을 알렸다"
        );
        assert!(
            frame(&ctx, 0.10, vec![press(spot, false)]),
            "손을 뗐는데 바뀜을 알리지 않았다"
        );
    }

    #[test]
    fn 토글_줄은_폼_행_높이를_쓴다() {
        // 같은 대화 안에서 드롭다운·체크박스와 줄 높이가 어긋나면 항목이 들쭉날쭉해 보인다
        let ctx = egui::Context::default();
        let mut allocated = 0.0;
        let _ = ctx.run_ui(Default::default(), |ui| {
            // `cursor()` 차이는 항목 사이 간격(`item_spacing`)까지 포함하므로
            // 이 줄이 실제로 차지한 높이는 `min_rect`로 잰다
            toggle_row(ui, "윈도우 시작 시 실행", false);
            allocated = ui.min_rect().height();
        });
        assert_eq!(
            allocated, FORM_FIELD_HEIGHT,
            "토글 줄 높이가 폼 기준과 다르다"
        );
    }

    #[test]
    fn 마스킹은_글자_수만큼_원문자를_만든다() {
        // 인벤토리 #75 — egui 기본 마스킹 문자(`•`)가 아니라 `●`다
        assert_eq!(display_text("abc", true), "●●●");
        assert_eq!(
            display_text("비밀", true),
            "●●",
            "글자 수로 센다(바이트가 아니다)"
        );
        assert_eq!(display_text("", true), "");
        assert_eq!(display_text("abc", false), "abc");
    }

    #[test]
    fn 폼_치수는_원본과_같다() {
        // 원본 `:425-426` — 라벨 96px · 필드 28px · 간격 10px
        assert_eq!(FORM_LABEL_WIDTH, 96.0);
        assert_eq!(FORM_FIELD_HEIGHT, 28.0);
        assert_eq!(FORM_GAP, 10.0);
        assert_eq!(FORM_FIELD_PAD_X, 8.0);
        assert_eq!(FORM_FONT_PX, 13.0);
    }

    /// 가로 배치에서 필드가 차지한 크기를 잰다
    fn field_size(width: f32, enabled: bool) -> egui::Vec2 {
        let ctx = egui::Context::default();
        let mut size = egui::Vec2::ZERO;
        let mut value = "값".to_owned();
        let _ = ctx.run_ui(Default::default(), |ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 0.0;
                    let before = ui.cursor().min.x;
                    let response = text_field(
                        ui,
                        "테스트",
                        &mut value,
                        egui::vec2(width, FORM_FIELD_HEIGHT),
                        enabled,
                        false,
                    );
                    size = egui::vec2(ui.cursor().min.x - before, response.rect.height());
                });
            });
        });
        size
    }

    #[test]
    fn 라디오_체크_스피너_치수는_원본과_같다() {
        // 원본 `:446-447`(14px 원·6px 점)·`:453`(14px 사각)·`:458-463`(92×26·▲▼ 16px)
        assert_eq!(RADIO_SIZE, 14.0);
        assert_eq!(RADIO_DOT, 6.0);
        assert_eq!(CHECK_SIZE, 14.0);
        assert_eq!(MARK_GAP, 8.0);
        assert_eq!(SPINNER_WIDTH, 92.0);
        assert_eq!(SPINNER_HEIGHT, 26.0);
        assert_eq!(SPINNER_ARROW_WIDTH, 16.0);
    }

    /// 스피너를 한 프레임 그리고 돌려준 값을 본다 (아무것도 누르지 않은 상태)
    fn spinner_once(value: u8, range: std::ops::RangeInclusive<u8>) -> u8 {
        let ctx = egui::Context::default();
        let mut out = value;
        let _ = ctx.run_ui(Default::default(), |ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                out = spinner_field(ui, value, range.clone(), true);
            });
        });
        out
    }

    #[test]
    fn 스피너는_범위_밖의_값을_끌어당긴다() {
        // Acceptance ④ — 화살표를 계속 눌러도, 저장된 값이 이상해도 1~10 안에 머문다
        assert_eq!(spinner_once(5, 1..=10), 5);
        assert_eq!(spinner_once(0, 1..=10), 1);
        assert_eq!(spinner_once(200, 1..=10), 10);
    }

    #[test]
    fn 입력_필드는_지정한_폭과_28px_높이를_차지한다() {
        // Acceptance ② — 필드 높이가 디자인 값이어야 라벨 행이 원본과 같은 자리에 온다
        let size = field_size(240.0, true);
        assert_eq!(size.x, 240.0);
        assert!(
            size.y <= FORM_FIELD_HEIGHT,
            "편집기가 웰(28px) 밖으로 자랐다: {}",
            size.y
        );
        // 비활성 필드도 같은 자리를 차지한다 — 활성 여부로 행 높이가 흔들리지 않는다
        assert_eq!(field_size(240.0, false).x, 240.0);
    }

    #[test]
    fn 아이콘_판정은_아이콘_글꼴만_받아들인다() {
        assert!(is_icon_font(egui_phosphor::regular::CHECK));
        assert!(is_icon_font(egui_phosphor::regular::CARET_DOWN));
        // 원본 디자인의 기호들 — 이 앱 글꼴에 없어 두부가 된다
        for glyph in ["\u{23F8}", "\u{2715}", "\u{29C9}", "\u{25B2}", "\u{2713}"] {
            assert!(!is_icon_font(glyph), "{glyph:?}를 아이콘으로 보았다");
        }
        // 글자 여럿·빈 문자열은 아이콘이 아니다
        assert!(!is_icon_font(""));
        assert!(!is_icon_font("가나"));
    }

    #[test]
    fn 화면_코드에_원본_아이콘_기호가_남아_있지_않다() {
        // **규약: 아이콘은 `egui_phosphor`에서만 가져온다** (2026-08-05 사용자 결정).
        // 원본 HTML의 기호를 그대로 쓰면 이 앱의 글꼴(맑은 고딕+phosphor)에 없어 두부가 된다.
        // 소스를 훑어 그 기호가 **문자열 리터럴로** 다시 들어오는 것을 막는다
        const 금지: [(char, &str); 11] = [
            ('\u{23F8}', "일시정지"),
            ('\u{2715}', "닫기"),
            ('\u{29C9}', "복사"),
            ('\u{25B2}', "위 캐럿"),
            ('\u{25BC}', "아래 캐럿"),
            ('\u{2713}', "체크"),
            ('\u{27F3}', "새로 고침"),
            ('\u{23F5}', "메뉴 화살표"),
            ('\u{25BE}', "작은 아래 캐럿"),
            ('\u{25B4}', "작은 위 캐럿"),
            ('\u{2022}', "글머리 점"),
        ];
        let ui_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/ui");
        let mut 발견 = Vec::new();
        for entry in std::fs::read_dir(&ui_dir).expect("ui 디렉터리") {
            let path = entry.expect("항목").path();
            if path.extension().and_then(|ext| ext.to_str()) != Some("rs") {
                continue;
            }
            let text = std::fs::read_to_string(&path).expect("소스 읽기");
            for (line_no, line) in text.lines().enumerate() {
                // 주석은 설명하려고 그 기호를 적을 수 있다 — 코드 부분만 본다
                let code = line.split("//").next().unwrap_or("");
                for (glyph, 이름) in 금지 {
                    if code.contains(&format!("\"{glyph}\"")) {
                        발견.push(format!(
                            "{}:{} {이름}({glyph})",
                            path.file_name().unwrap_or_default().to_string_lossy(),
                            line_no + 1
                        ));
                    }
                }
            }
        }
        assert!(
            발견.is_empty(),
            "아이콘은 `egui_phosphor`에서 가져와야 한다 — 원본 기호가 남았다: {발견:?}"
        );
    }
}
