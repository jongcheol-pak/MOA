//! 서버 로그 화면 (FR-40) — 원본 `FileExplorer-FTP.dc.html:308-313`.
//!
//! 도크 셸(`ui::dock`)을 큐 화면과 나눠 쓰고, 여기서는 본문만 그린다.
//! 한 줄은 **시각(62px) · 종류(44px) · 본문**이며 고정폭 글꼴 12px/17px이다 — 서버가 준
//! 응답 코드와 경로가 자리를 맞춰 읽히도록.
//!
//! **버퍼를 고치지 않는다** — 읽어서 그리고, `⧉`(복사)는 값으로 돌려준다.
//! 비밀번호 가리기는 이미 버퍼에 들어가기 전에 끝나 있다(D14·T5) — 여기서 다시 하지 않는다.
//!
//! **선택은 egui의 라벨 선택을 그대로 쓴다** — 우리 선택 모델을 따로 만들지 않는다.
//! 다만 egui는 오른쪽 버튼 누름에도 선택 범위를 접어 버려(`text_cursor_state`의
//! `pointer_interaction`이 버튼을 가리지 않는다), 우클릭 메뉴를 띄우면 그 순간 고른 글자가
//! 사라진다. 그래서 그 프레임에만 **라벨이 포인터를 못 보게 가린다**(`suppress_pointer`·
//! `doctor`). 복사도 우리가 문자열을 만들지 않고 egui에게 시킨다(`Event::Copy`) —
//! 선택 범위가 그쪽 안에만 있어 밖에서 읽을 길이 없다.
use crate::remote::log::{LogBuffer, LogKind};
use crate::ui::theme;
use eframe::egui;
use egui::text_selection::LabelSelectionState;

// ── 시각 토큰 (원본 `:308-313`) ──
/// 본문 안쪽 여백
const PAD_X: f32 = 10.0;
const PAD_Y: f32 = 6.0;
/// 줄 사이 간격
const LINE_GAP: f32 = 2.0;
/// 글꼴 — 고정폭 12px, 줄 높이 17px
const FONT_PX: f32 = 12.0;
const LINE_HEIGHT: f32 = 17.0;
/// 시각 열·종류 열 폭
const TIME_WIDTH: f32 = 62.0;
const KIND_WIDTH: f32 = 44.0;
/// 열 사이 간격 (`:310` `gap:10px`)
const COLUMN_GAP: f32 = 10.0;

/// 고정폭 글꼴 이름 — 없는 시스템에서는 egui 기본 글꼴로 떨어진다 (plan Edge Case).
///
/// egui는 이름으로 글꼴을 고르지 않고 **미리 등록된 가족**(`FontFamily::Monospace`)에서
/// 고르므로, 원본이 적은 `Consolas`·`D2Coding`은 그 가족에 무엇이 실려 있든 같은 자리를
/// 가리킨다. 이름을 상수로 남겨 두는 것은 디자인 근거를 잃지 않기 위함이다
#[cfg(test)]
const FONT_STACK: [&str; 2] = ["Consolas", "D2Coding"];

/// 종류별 글자색과 배경 (인벤토리 #49~#52, `:734-743`).
///
/// **오류 줄만 배경이 깔린다** — 서버가 거부한 줄은 스크롤 속에서 눈에 띄어야 한다
pub fn kind_colors(kind: LogKind) -> (egui::Color32, Option<egui::Color32>) {
    match kind {
        LogKind::Status => (theme::HEADER_TEXT, None),
        // 명령은 파랑 계열이지만 강조 파랑(`#4A9EFF`)보다 한 단계 밝다 (`:737`)
        LogKind::Command => (COMMAND_COLOR, None),
        LogKind::Response => (theme::TEXT_MUTED, None),
        LogKind::Error => (theme::ERROR, Some(theme::ERROR_FILL)),
    }
}

/// 명령 줄 색 (`:737` `#6FA8FF`) — 팔레트의 강조 파랑과 다른 값이라 여기 둔다
const COMMAND_COLOR: egui::Color32 = egui::Color32::from_rgb(0x6F, 0xA8, 0xFF);

/// 프레임 사이에 이어져야 하는 두 가지 — 앱이 들고 프레임마다 넘긴다.
///
/// 화면에 담지 않는 이유: 둘 다 **한두 프레임짜리 임시값**이라 세션에 담을 것이 없고,
/// 도크 상태(`DockState`)에 섞으면 저장 형식과 무관한 값이 그 구조에 들어온다
#[derive(Debug, Default, Clone, PartialEq)]
pub struct MenuState {
    /// 직전 프레임에 메뉴가 떠 있었는가 — 억제를 그동안 유지하는 데 쓴다
    menu_open: bool,
    /// 다음 프레임에 복사를 시켜야 하는가 — `복사`를 누른 프레임에는 아직 시키지 못한다
    pending_copy: bool,
}

/// 이번 프레임에 라벨이 포인터를 보면 안 되는가.
///
/// 오른쪽 버튼이 눌려 있는 동안과 **메뉴가 떠 있는 동안** 참이다. 뒤쪽이 필요한 이유:
/// 메뉴 항목을 누르는 프레임에 로그 라벨이 hover가 아니면 egui가 `any_pressed && !any_hovered`
/// 로 보고 선택을 통째로 버린다(`on_end_pass`)
fn suppress_pointer(ctx: &egui::Context, state: &MenuState) -> bool {
    state.menu_open
        || ctx.input(|i| {
            i.pointer.button_down(egui::PointerButton::Secondary)
                || i.pointer.button_released(egui::PointerButton::Secondary)
        })
}

/// 라벨이 포인터를 못 보게 가린 사본.
///
/// `CONTAINS_POINTER`를 지우면 `pointer_interaction`이 아예 불리지 않고,
/// `IS_POINTER_BUTTON_DOWN_ON`을 지우면 끌기로 오인해 범위를 옮기는 갈래도 멈춘다.
/// **`HOVERED`는 세운다** — egui가 그것으로 「어느 라벨엔가 마우스가 있다」를 세어
/// 선택을 통째로 버릴지 정하기 때문이다. 대가는 메뉴가 떠 있는 동안 커서가 I빔이 되는 것뿐이고,
/// 그 대신 얻는 것은 고른 글자가 사라지지 않는 것이다.
///
/// **`Response.flags`는 egui가 `#[doc(hidden)]`으로 둔 자리다** — 안정 계약이 아니라서
/// 판올림 때 사라질 수 있다. 그때는 **컴파일이 막혀** 조용히 새지 않는다
fn doctor(response: &egui::Response) -> egui::Response {
    use egui::response::Flags;
    let mut doctored = response.clone();
    doctored.flags.remove(Flags::CONTAINS_POINTER);
    doctored.flags.remove(Flags::IS_POINTER_BUTTON_DOWN_ON);
    doctored.flags.insert(Flags::HOVERED);
    doctored
}

/// 고른 글자가 있는가 — `복사`를 흐리게 할지 가른다.
///
/// **egui가 주는 것은 「선택 상태가 살아 있는가」다**(범위가 아니다) — 한 번 클릭만 해도
/// 참이 되며, 그 상태로 복사하면 egui가 그 칸 전문을 담는다. 범위를 물을 길이 없어
/// 이 경계는 더 좁힐 수 없다.
///
/// **그 상태는 창 전체에 하나다** — 어느 화면의 라벨을 골랐든 같은 값을 본다. 지금은
/// 고를 수 있는 라벨이 이 화면뿐이라(`selectable(true)`를 쓰는 곳이 여기 하나다) 로그 밖의
/// 선택이 있을 수 없지만, 다른 화면에 그런 라벨이 생기면 이 메뉴가 **그쪽 선택**을 복사하게
/// 된다 — 그때 이 자리를 다시 봐야 한다
fn copy_enabled(ctx: &egui::Context) -> bool {
    ctx.plugin_opt::<LabelSelectionState>()
        .map(|plugin| plugin.lock().has_selection())
        .unwrap_or(false)
}

/// 로그 본문을 그린다. 새 줄이 오면 **맨 아래에 붙는다** (Acceptance ④).
///
/// 사용자가 위로 올려 둔 상태면 따라가지 않는다 — egui의 `stick_to_bottom`이 그 규칙을
/// 그대로 구현한다(바닥에 있을 때만 붙는다).
///
/// **프레임 안 차례가 정해져 있다**: ① 첫머리에서 `pending_copy`를 소비한다 ② 줄을 그린다
/// ③ 끝에서 `menu_open`을 갱신한다. 소비 지점이 하나뿐이라 순서가 뒤바뀔 여지가 없다
pub fn show_log(ui: &mut egui::Ui, rect: egui::Rect, log: &LogBuffer, state: &mut MenuState) {
    // ① 예약된 복사를 소비한다 — **줄을 그리기 전에** 밀어야 라벨이 그것을 본다.
    // 그 사이에 선택이 죽었으면(메뉴를 고른 프레임과 여기가 한 프레임 떨어져 있다) 밀지
    // 않는다 — 그대로 밀면 포커스를 쥔 입력칸의 복사만 남아 엉뚱한 값이 클립보드에 간다
    if std::mem::take(&mut state.pending_copy) && copy_enabled(ui.ctx()) {
        ui.ctx().input_mut(|i| i.events.push(egui::Event::Copy));
    }
    let suppress = suppress_pointer(ui.ctx(), state);

    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(rect)
            .layout(egui::Layout::top_down(egui::Align::Min)),
    );
    child.set_clip_rect(rect);
    let lines: Vec<&crate::remote::log::LogLine> = log.iter().collect();
    let row_height = LINE_HEIGHT + LINE_GAP;
    let mut menu_open = false;
    let mut pending_copy = false;
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .stick_to_bottom(true)
        .show_rows(&mut child, row_height, lines.len(), |ui, range| {
            ui.spacing_mut().item_spacing.y = LINE_GAP;
            for index in range {
                if let Some(line) = lines.get(index) {
                    let outcome = show_line(ui, line, suppress);
                    menu_open |= outcome.menu_open;
                    pending_copy |= outcome.copy_picked;
                }
            }
        });
    // ③ 다음 프레임이 볼 값을 남긴다
    state.menu_open = menu_open;
    state.pending_copy |= pending_copy;
}

/// 한 줄이 이번 프레임에 관측한 것
#[derive(Default)]
struct LineOutcome {
    /// 이 줄의 메뉴가 떠 있는가
    menu_open: bool,
    /// 그 메뉴에서 `복사`를 눌렀는가
    copy_picked: bool,
}

/// 한 줄 — 시각 · 종류 · 본문.
///
/// 세 열을 **라벨로 놓는다** — `painter`로 그리면 위젯이 아니라 마우스로 끌어 고를 수 없다
/// (2026-08-18 사용자 요청). egui는 여러 라벨에 걸친 선택을 스스로 이어 주므로, 한 줄의 세
/// 열은 공백으로 다음 줄은 개행으로 이어져 복사된다
fn show_line(ui: &mut egui::Ui, line: &crate::remote::log::LogLine, suppress: bool) -> LineOutcome {
    let (color, background) = kind_colors(line.kind);
    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), LINE_HEIGHT),
        egui::Sense::hover(),
    );
    if let Some(background) = background {
        ui.painter().rect_filled(rect, 0.0, background);
    }
    // 열 자리는 **절대 좌표로 잡는다** — 커서 배치(`add_sized`·`allocate_ui_with_layout`)는
    // 위젯을 셀 가운데에 놓거나 내용 크기만큼만 차지해 열 x가 밀린다
    // (2026-08-18 실측: 시각 10 → 41 / 10, 종류 82 → 104 / 20). 원본 치수는
    // 시각 10 · 종류 82 · 본문 136이며 이 계산이 그것을 그대로 낸다
    let time_left = rect.left() + PAD_X;
    let kind_left = time_left + TIME_WIDTH + COLUMN_GAP;
    let text_left = kind_left + KIND_WIDTH + COLUMN_GAP;
    let mut outcome = LineOutcome::default();
    // 세 칸 모두에 메뉴를 단다 — 사용자가 어느 칸을 우클릭하든 떠야 한다
    for (at, width, text, color) in [
        (time_left, TIME_WIDTH, line.time.as_str(), theme::TEXT_MUTED),
        (kind_left, KIND_WIDTH, line.kind.label(), color),
        // **본문 색은 종류와 무관하게 하나다** — 원본이 본문 span에 고정색을 준다(`:313`).
        // 종류별 색은 앞의 종류 열에만 붙는다
        (
            text_left,
            rect.right() - PAD_X - text_left,
            line.text.as_str(),
            theme::TEXT_LOG,
        ),
    ] {
        let response =
            selectable_cell(ui, egui::pos2(at, rect.top()), width, text, color, suppress);
        let picked = attach_copy_menu(ui, &response);
        outcome.menu_open |= picked.menu_open;
        outcome.copy_picked |= picked.copy_picked;
    }
    outcome
}

/// 그 칸에 `복사` 한 줄짜리 메뉴를 단다 — **팝업을 여는 구문을 적는 유일한 자리다**.
///
/// 칸마다 인라인으로 적으면 팝업 규약 시험(`ui::theme`의 소스 훑기)이 「팝업을 여는 구문 수」와
/// 「공통 스타일 호출 수」를 견주다 어긋난다. 여기 한 번만 적으면 셋이 같은 길을 쓴다.
///
/// **메뉴는 라벨 응답에 단다** — 줄 전체를 잡은 응답은 `Sense::hover()`라 우클릭이 서지 않고,
/// 누름을 세게 해도 라벨이 먼저 포인터를 가져간다(그쪽이 나중에 할당돼 히트 테스트에 이긴다)
fn attach_copy_menu(ui: &egui::Ui, response: &egui::Response) -> LineOutcome {
    let enabled = copy_enabled(ui.ctx());
    let mut copy_picked = false;
    let inner = response.context_menu(|ui| {
        theme::menu_style(ui);
        if ui
            .add_enabled(enabled, egui::Button::new(crate::i18n::menu_copy()))
            .clicked()
        {
            copy_picked = true;
            ui.close();
        }
    });
    LineOutcome {
        menu_open: inner.is_some(),
        copy_picked,
    }
}

/// 열 한 칸 — 그 자리에 **고를 수 있는 라벨** 하나를 놓는다.
///
/// 길어도 줄바꿈하지 않는다(행 높이가 고정이라 두 줄이 되면 아래 줄과 겹친다).
/// 잘려 보여도 **전체를 고르면 원문이 복사된다** — egui가 갤리의 원문을 준다.
///
/// **`ui.add`로 놓지 않고 `Label::ui`가 하는 일을 손으로 편다** — 그 사이에서 응답을 가려야
/// 우클릭이 선택을 접지 않는다. `Label::ui`가 더 하는 일은 접근성 정보와 잘렸을 때의 툴팁인데,
/// **툴팁은 여기서 되살린다**(없으면 긴 줄의 원문을 마우스로 볼 길이 사라진다)
fn selectable_cell(
    ui: &mut egui::Ui,
    at: egui::Pos2,
    width: f32,
    text: &str,
    color: egui::Color32,
    suppress: bool,
) -> egui::Response {
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(egui::Rect::from_min_size(
                at,
                egui::vec2(width.max(0.0), LINE_HEIGHT),
            ))
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );
    let label = egui::Label::new(
        egui::RichText::new(text)
            .font(egui::FontId::monospace(FONT_PX))
            .color(color),
    )
    .selectable(true)
    .truncate();
    let (galley_pos, galley, mut response) = label.layout_in_ui(&mut child);
    // 접근성 트리에 이 칸이 무엇인지 알린다 — `Label::ui`가 하던 일이며, `accesskit`이
    // egui의 필수 의존성이라 직접 그리기로 바꾸면 이 정보만 조용히 빠진다
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Label, child.is_enabled(), galley.text())
    });
    // 그리기와 툴팁은 **보이는 자리에서만** — `Label::ui`와 같은 가드다. `show_rows`가 이미
    // 보이는 줄만 넘기므로 지금 걸리는 프레임은 거의 없지만, 원본과 자리를 맞춰 두면
    // 스크롤 경계에서 예상 밖 동작이 생기지 않는다
    if child.is_rect_visible(response.rect) {
        let seen = if suppress {
            doctor(&response)
        } else {
            response.clone()
        };
        LabelSelectionState::label_text_selection(
            &child,
            &seen,
            galley_pos,
            galley.clone(),
            color,
            egui::Stroke::NONE,
        );
        // 잘린 칸은 마우스를 올리면 원문을 보인다 — `Label::ui`의 `show_tooltip_when_elided`가
        // 기본으로 하던 일이다
        if galley.elided {
            let job = egui::text::LayoutJob {
                sections: galley.job.sections.clone(),
                text: galley.job.text.clone(),
                ..egui::text::LayoutJob::default()
            };
            response = response.on_hover_text(job);
        }
    }
    response
}

/// 본문 위쪽 여백 — 호출부가 자리를 잡을 때 쓴다 (`:308` `padding:6px 10px`)
pub const BODY_PAD_Y: f32 = PAD_Y;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 로그_치수는_원본과_같다() {
        // Acceptance ① — 시각 62px·종류 44px·12px/17px 고정폭
        assert_eq!(TIME_WIDTH, 62.0);
        assert_eq!(KIND_WIDTH, 44.0);
        assert_eq!(FONT_PX, 12.0);
        assert_eq!(LINE_HEIGHT, 17.0);
        assert_eq!(PAD_X, 10.0);
        assert_eq!(PAD_Y, 6.0);
        assert_eq!(LINE_GAP, 2.0);
        // 원본이 적은 글꼴 이름 — 근거를 잃지 않게 남긴다
        assert_eq!(FONT_STACK, ["Consolas", "D2Coding"]);
    }

    #[test]
    fn 종류별_색이_인벤토리와_같다() {
        // Acceptance ② (인벤토리 #49~#52, `:734-743`)
        assert_eq!(kind_colors(LogKind::Status), (theme::HEADER_TEXT, None));
        assert_eq!(kind_colors(LogKind::Command).0, COMMAND_COLOR);
        assert_eq!(
            COMMAND_COLOR,
            egui::Color32::from_rgb(0x6F, 0xA8, 0xFF),
            "명령 줄 색이 강조 파랑과 섞였다"
        );
        assert_ne!(COMMAND_COLOR, theme::ACCENT);
        assert_eq!(kind_colors(LogKind::Response), (theme::TEXT_MUTED, None));
        // 오류 줄만 배경이 깔린다
        assert_eq!(
            kind_colors(LogKind::Error),
            (theme::ERROR, Some(theme::ERROR_FILL))
        );
    }

    #[test]
    fn 종류_표기는_디자인_원문_그대로다() {
        // 인벤토리 #49~#52
        assert_eq!(LogKind::Status.label(), "상태:");
        assert_eq!(LogKind::Command.label(), "명령:");
        assert_eq!(LogKind::Response.label(), "응답:");
        assert_eq!(LogKind::Error.label(), "오류:");
    }

    /// 로그를 한 프레임 그리고 글자마다 (내용, x, 색)을 모은다.
    ///
    /// **실제 렌더 경로로 잰다** — 유틸리티를 따로 부르면 화면이 그것을 더 이상 쓰지 않게 된
    /// 뒤에도 시험이 통과한다(2026-08-18 리뷰가 그 상태를 잡았다)
    fn draw_log(log: &LogBuffer) -> Vec<(String, f32, egui::Color32)> {
        let ctx = egui::Context::default();
        let output = ctx.run_ui(Default::default(), |ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                let rect =
                    egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(600.0, 200.0));
                show_log(ui, rect, log, &mut MenuState::default());
            });
        });
        let mut drawn = Vec::new();
        for clipped in &output.shapes {
            if let egui::Shape::Text(text) = &clipped.shape {
                let color = text
                    .galley
                    .job
                    .sections
                    .first()
                    .map(|section| section.format.color)
                    .unwrap_or(text.fallback_color);
                drawn.push((text.galley.text().to_owned(), text.pos.x, color));
            }
        }
        drawn
    }

    /// 그려진 것 중 그 조각을 담은 첫 글자
    fn 찾는다(drawn: &[(String, f32, egui::Color32)], 조각: &str) -> (f32, egui::Color32) {
        drawn
            .iter()
            .find(|(text, _, _)| text.contains(조각))
            .map(|(_, x, color)| (*x, *color))
            .unwrap_or_else(|| panic!("`{조각}`이 그려지지 않았다: {drawn:?}"))
    }

    #[test]
    fn 본문_색은_종류와_무관하게_하나다() {
        // spec 리뷰 M1 — 원본은 본문 span에 `#B4B4B4` 고정색을 준다(`:313`).
        // 종류별 색은 **앞의 종류 열에만** 붙는다. 오류 줄 본문까지 빨갛게 칠하면 원본과 다르다
        let mut log = LogBuffer::new();
        log.push(LogKind::Status, "상태 본문");
        log.push(LogKind::Error, "오류 본문");
        let drawn = draw_log(&log);

        assert_eq!(찾는다(&drawn, "상태 본문").1, theme::TEXT_LOG);
        assert_eq!(찾는다(&drawn, "오류 본문").1, theme::TEXT_LOG);
        // 종류 열은 여전히 종류별로 갈린다
        assert_eq!(찾는다(&drawn, "상태:").1, kind_colors(LogKind::Status).0);
        assert_eq!(찾는다(&drawn, "오류:").1, kind_colors(LogKind::Error).0);
        assert_ne!(
            kind_colors(LogKind::Error).0,
            kind_colors(LogKind::Status).0
        );
    }

    #[test]
    fn 로그_줄은_고를_수_있는_라벨이고_열_x가_원본과_같다() {
        // 2026-08-18 사용자 요청 — `painter`로 그리면 위젯이 아니라 끌어서 고를 수 없다.
        // 라벨로 바꾸면서 열 x가 밀리지 않았는지 함께 본다(시각 10 · 종류 82 · 본문 136).
        // 커서 배치(`add_sized`·`allocate_ui_with_layout`)는 이 값을 밀었다 — 그 실측이
        // `show_line`의 주석에 있다
        let mut log = LogBuffer::new();
        log.push(LogKind::Response, "226 Transfer complete");
        let drawn = draw_log(&log);

        assert_eq!(찾는다(&drawn, "응답:").0, PAD_X + TIME_WIDTH + COLUMN_GAP);
        assert_eq!(
            찾는다(&drawn, "226").0,
            PAD_X + TIME_WIDTH + COLUMN_GAP + KIND_WIDTH + COLUMN_GAP
        );
        // 시각은 `HH:MM:SS`라 내용을 단정하지 않고 자리만 본다
        assert!(
            drawn.iter().any(|(_, x, _)| (*x - PAD_X).abs() < 0.01),
            "시각 열이 왼쪽 여백 자리에 없다: {drawn:?}"
        );

        // 끌어서 고르는 것은 egui의 라벨 선택이 맡는다. **두 스위치의 처지가 갈린다.**
        //
        // `selectable_labels`는 `Label::ui`가 보는데 이 화면은 우클릭 때 응답을 가리려고
        // 그 함수를 거치지 않으므로 더 이상 이 경로에 영향이 없다 — 대신 직접 호출이 실제로
        // 있는지를 본다(끌어 고르기가 되는지는 아래 선택 유지 시험이 잰다).
        //
        // 반면 `multi_widget_text_select`는 **`label_text_selection` 자신이** 본다
        // (egui `label_text_selection.rs:353`·`:596`·`:601`) — 꺼지면 시각·종류·본문 세 칸에
        // 걸친 선택이 한 칸으로 좁혀지므로 그 단정은 그대로 남긴다
        let source = 생산_코드();
        assert_eq!(
            source
                .matches("LabelSelectionState::label_text_selection(")
                .count(),
            1,
            "라벨 선택을 직접 부르지 않으면 끌어 고를 수 없다"
        );
        assert!(
            egui::Context::default()
                .style_of(egui::Theme::Dark)
                .interaction
                .multi_widget_text_select,
            "여러 칸에 걸친 선택이 꺼져 있다 — 한 줄을 통째로 고를 수 없게 된다"
        );
    }

    /// 이 파일의 **생산 코드만** — 시험 모듈 아래는 시험이 쓴 문자열이라 함께 세면
    /// 시험 자신이 적은 이름 때문에 건수가 부풀어 규약 검사가 뜻을 잃는다.
    ///
    /// 자르는 표식이 `#[cfg(test)]`가 아니라 `mod tests {` 인 이유가 둘이다: 파일 앞쪽에도
    /// 시험 전용 상수(`FONT_STACK`)가 같은 특성으로 서 있어 첫 번째에서 자르면 생산 코드가
    /// 통째로 빠지고, 특성과 `mod`를 개행으로 이어 찾으면 줄바꿈 방식에 따라 어긋난다
    fn 생산_코드() -> &'static str {
        let source = include_str!("log_panel.rs");
        source
            .split_once("mod tests {")
            .map(|(앞, _)| 앞)
            .unwrap_or(source)
    }

    /// 로그를 한 프레임 그린다 — 시험이 입력을 먹이고 출력을 받는 유일한 길
    fn 한_프레임(
        ctx: &egui::Context,
        log: &LogBuffer,
        state: &mut MenuState,
        time: f64,
        events: Vec<egui::Event>,
    ) -> egui::FullOutput {
        let input = egui::RawInput {
            time: Some(time),
            events,
            screen_rect: Some(화면()),
            ..Default::default()
        };
        ctx.run_ui(input, |ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                let rect =
                    egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(600.0, 200.0));
                show_log(ui, rect, log, state);
            });
        })
    }

    /// 그 문구로 그려진 글자의 한가운데 — 메뉴 항목을 누를 자리다
    fn 글자_자리(output: &egui::FullOutput, 문구: &str) -> Option<egui::Pos2> {
        for clipped in &output.shapes {
            if let egui::Shape::Text(text) = &clipped.shape
                && text.galley.text() == 문구
            {
                return Some(egui::pos2(
                    text.pos.x + text.galley.size().x / 2.0,
                    text.pos.y + text.galley.size().y / 2.0,
                ));
            }
        }
        None
    }

    /// 시험 화면 — 로그 자리(600x200)보다 넉넉해야 그리기가 잘리지 않는다
    fn 화면() -> egui::Rect {
        egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(800.0, 400.0))
    }

    fn 눌렀다(at: egui::Pos2, button: egui::PointerButton, pressed: bool) -> egui::Event {
        egui::Event::PointerButton {
            pos: at,
            button,
            pressed,
            modifiers: egui::Modifiers::NONE,
        }
    }

    /// 첫 줄 본문 글자가 **실제로 그려진 자리** — 자리 계산을 손으로 다시 하면
    /// 스크롤·여백이 바뀔 때 조용히 어긋난다
    fn 본문_시작(log: &LogBuffer, 조각: &str) -> egui::Pos2 {
        let ctx = egui::Context::default();
        let mut state = MenuState::default();
        let output = 한_프레임(&ctx, log, &mut state, 0.1, Vec::new());
        for clipped in &output.shapes {
            if let egui::Shape::Text(text) = &clipped.shape
                && text.galley.text().contains(조각)
            {
                return egui::pos2(text.pos.x + 2.0, text.pos.y + text.galley.size().y / 2.0);
            }
        }
        panic!("`{조각}`이 그려지지 않았다");
    }

    fn 두_줄() -> LogBuffer {
        let mut log = LogBuffer::new();
        log.push(LogKind::Response, "226 Transfer complete");
        log.push(LogKind::Error, "550 Permission denied");
        log
    }

    /// 그 자리를 왼쪽 버튼으로 눌러 **글자 커서를 놓는다** — 선택 상태가 살아난다.
    ///
    /// **범위를 끌어 만들지는 않는다** — 이 시험 환경에서는 egui의 끌기 판정이 재현되지
    /// 않는다(대장 `[2026-08-18]`의 같은 벽 · 이번 회차가 세 번째 관측). 범위가 비어 있으면
    /// egui가 그 칸 전문을 복사하므로, 아래 시험들이 견주는 것은 **문구가 우클릭·메뉴 조작을
    /// 건너서도 그대로인가**다 — 억제가 없으면 그 문구가 아예 나오지 않는다(선택이 버려진다)
    fn 커서를_놓는다(at: egui::Pos2) -> Vec<Vec<egui::Event>> {
        vec![
            vec![egui::Event::PointerMoved(at)],
            vec![눌렀다(at, egui::PointerButton::Primary, true)],
            vec![눌렀다(at, egui::PointerButton::Primary, false)],
        ]
    }

    #[test]
    fn 우클릭해도_고른_글자가_남는다() {
        // Acceptance ① — egui는 오른쪽 버튼 누름에도 선택을 건드린다.
        // 우클릭을 건넌 뒤에도 같은 문구가 복사되고 선택 상태가 살아 있어야 한다
        let log = 두_줄();
        let 본문 = 본문_시작(&log, "226");
        let ctx = egui::Context::default();
        let mut state = MenuState::default();
        let mut time = 0.0;

        let mut 고른것 = None;
        for events in 커서를_놓는다(본문)
            .into_iter()
            .chain([vec![egui::Event::Copy]])
        {
            time += 0.1;
            고른것 = 복사된것(한_프레임(&ctx, &log, &mut state, time, events)).or(고른것);
        }
        let 고른것 = 고른것.expect("커서를 놓았는데 복사할 것이 없다");
        assert!(copy_enabled(&ctx), "시험 자체가 성립하지 않았다");

        // 우클릭 — 누름·뗌
        for events in [
            vec![눌렀다(본문, egui::PointerButton::Secondary, true)],
            vec![눌렀다(본문, egui::PointerButton::Secondary, false)],
        ] {
            time += 0.1;
            let _ = 한_프레임(&ctx, &log, &mut state, time, events);
        }
        assert!(copy_enabled(&ctx), "우클릭이 고른 것을 통째로 버렸다");

        time += 0.1;
        let 우클릭뒤 = 복사된것(한_프레임(
            &ctx,
            &log,
            &mut state,
            time,
            vec![egui::Event::Copy],
        ));
        assert_eq!(우클릭뒤, Some(고른것), "우클릭 뒤 복사되는 것이 달라졌다");
    }

    #[test]
    fn 메뉴의_복사가_고른_글자를_클립보드로_보낸다() {
        // Acceptance ② — **억제의 실질이 여기서 드러난다.** 메뉴 항목을 누르는 프레임에
        // 로그 라벨이 hover가 아니면 egui가 선택을 통째로 버려(`on_end_pass`), 다음 프레임의
        // 복사가 아무것도 내놓지 못한다. 억제가 그 프레임까지 hover를 유지한다
        let _guard =
            crate::i18n::LanguageGuard::lock(crate::app::settings::LanguageSetting::Korean);
        let log = 두_줄();
        let 본문 = 본문_시작(&log, "226");
        let ctx = egui::Context::default();
        let mut state = MenuState::default();
        let mut time = 0.0;

        let mut 고른것 = None;
        for events in 커서를_놓는다(본문)
            .into_iter()
            .chain([vec![egui::Event::Copy]])
        {
            time += 0.1;
            고른것 = 복사된것(한_프레임(&ctx, &log, &mut state, time, events)).or(고른것);
        }
        let 고른것 = 고른것.expect("커서를 놓았는데 복사할 것이 없다");

        // 우클릭으로 메뉴를 띄우고 `복사` 항목의 자리를 잰다
        let mut 항목 = None;
        for events in [
            vec![눌렀다(본문, egui::PointerButton::Secondary, true)],
            vec![눌렀다(본문, egui::PointerButton::Secondary, false)],
            vec![],
        ] {
            time += 0.1;
            let output = 한_프레임(&ctx, &log, &mut state, time, events);
            항목 = 항목.or_else(|| 글자_자리(&output, crate::i18n::menu_copy()));
        }
        let at = 항목.expect("메뉴에 `복사`가 없다");

        // 그 자리를 누르고, 다음 프레임들에서 복사가 나가는지 본다
        let mut 복사본 = None;
        for events in [
            vec![egui::Event::PointerMoved(at)],
            vec![눌렀다(at, egui::PointerButton::Primary, true)],
            vec![눌렀다(at, egui::PointerButton::Primary, false)],
            vec![],
            vec![],
        ] {
            time += 0.1;
            복사본 = 복사된것(한_프레임(&ctx, &log, &mut state, time, events)).or(복사본);
        }
        assert_eq!(
            복사본,
            Some(고른것),
            "메뉴의 `복사`가 고른 글자를 클립보드로 보내지 않았다"
        );
    }

    fn 복사된것(output: egui::FullOutput) -> Option<String> {
        output
            .platform_output
            .commands
            .into_iter()
            .find_map(|command| match command {
                egui::OutputCommand::CopyText(text) => Some(text),
                _ => None,
            })
    }

    #[test]
    fn 고른_것이_없으면_복사가_흐리다() {
        // Acceptance ③ — 무엇이 복사될지 알 수 없는 상태에서 클립보드를 덮지 않는다.
        // **재는 것은 그 판정 함수의 답**이다(그려진 버튼의 색은 egui 스타일이 정한다)
        let log = 두_줄();
        let 본문 = 본문_시작(&log, "226");
        let ctx = egui::Context::default();
        let mut state = MenuState::default();
        let _ = 한_프레임(&ctx, &log, &mut state, 0.1, Vec::new());
        assert!(
            !copy_enabled(&ctx),
            "아무것도 고르지 않았는데 `복사`가 눌린다"
        );

        let mut time = 0.1;
        for events in 커서를_놓는다(본문) {
            time += 0.1;
            let _ = 한_프레임(&ctx, &log, &mut state, time, events);
        }
        assert!(copy_enabled(&ctx), "골랐는데 `복사`가 흐리다");
    }

    #[test]
    fn 억제는_오른쪽_버튼과_메뉴가_떠_있는_동안_걸린다() {
        // Acceptance ①의 짝 — 억제 조건 자체를 못 박는다.
        // 메뉴가 떠 있는 동안까지 유지해야 **메뉴 항목을 누르는 프레임**을 덮는다
        let ctx = egui::Context::default();
        let 조용함 = MenuState::default();
        let 메뉴가_떴다 = MenuState {
            menu_open: true,
            ..MenuState::default()
        };
        // 아무 입력도 없는 프레임
        let _ = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(화면()),
                ..Default::default()
            },
            |_| {},
        );
        assert!(!suppress_pointer(&ctx, &조용함), "가만있는데 가렸다");
        assert!(
            suppress_pointer(&ctx, &메뉴가_떴다),
            "메뉴가 떠 있는데 가리지 않았다 — 항목을 누르는 프레임에 선택이 버려진다"
        );

        // 오른쪽 버튼을 누른 프레임
        let at = egui::pos2(50.0, 50.0);
        let _ = ctx.run_ui(
            egui::RawInput {
                time: Some(0.2),
                screen_rect: Some(화면()),
                events: vec![
                    egui::Event::PointerMoved(at),
                    눌렀다(at, egui::PointerButton::Secondary, true),
                ],
                ..Default::default()
            },
            |_| {},
        );
        assert!(
            suppress_pointer(&ctx, &조용함),
            "오른쪽 버튼이 눌렸는데 가리지 않았다"
        );
    }

    #[test]
    fn 가린_응답은_포인터를_못_보되_hover는_남는다() {
        // `doctor`가 손보는 세 플래그의 뜻을 못 박는다 — 앞의 둘이 선택을 접는 두 갈래를
        // 막고(`pointer_interaction`·`is_dragging`), `HOVERED`가 통째로 버리기를 막는다
        use egui::response::Flags;
        let ctx = egui::Context::default();
        let mut 원본 = None;
        let _ = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(화면()),
                ..Default::default()
            },
            |ui| {
                let (_, response) =
                    ui.allocate_exact_size(egui::vec2(10.0, 10.0), egui::Sense::click_and_drag());
                let mut 세운것 = response;
                세운것.flags.insert(Flags::CONTAINS_POINTER);
                세운것.flags.insert(Flags::IS_POINTER_BUTTON_DOWN_ON);
                세운것.flags.remove(Flags::HOVERED);
                원본 = Some(세운것);
            },
        );
        let 원본 = 원본.expect("응답을 만들지 못했다");
        assert!(원본.contains_pointer() && 원본.is_pointer_button_down_on());

        let 가린것 = doctor(&원본);
        assert!(!가린것.contains_pointer(), "포인터가 여전히 보인다");
        assert!(
            !가린것.is_pointer_button_down_on(),
            "누른 대상으로 남아 끌기로 오인된다"
        );
        assert!(가린것.hovered(), "hover가 꺼지면 선택이 통째로 버려진다");
        assert_eq!(가린것.id, 원본.id, "다른 위젯으로 바뀌었다");
    }

    #[test]
    fn 메뉴는_팝업_규약과_카탈로그를_거친다() {
        // Acceptance ④ — 팝업을 여는 구문과 공통 스타일 호출이 **한 곳**에 모여 있어야
        // `ui::theme`의 소스 훑기 시험(팝업 수 ≤ 스타일 호출 수)이 통과한다
        let source = 생산_코드();
        assert_eq!(
            source.matches(".context_menu(").count(),
            1,
            "팝업을 여는 자리가 여럿이면 칸마다 스타일을 다시 불러야 한다"
        );
        assert_eq!(
            source.matches("theme::menu_style(").count(),
            1,
            "팝업이 공통 항목 스타일을 거치지 않는다"
        );
        // 문구는 카탈로그를 거친다 — 셸 메뉴의 `복사`와 같은 키를 쓴다
        assert!(source.contains("crate::i18n::menu_copy()"));
    }

    #[test]
    fn 잘린_칸은_원문을_툴팁으로_보인다() {
        // Acceptance ⑦ — `Label::ui`가 기본으로 하던 일(`show_tooltip_when_elided`)을
        // 직접 그리기로 바꾸면서 잃었다. 되살린 자리를 소스로 고정한다.
        // 툴팁 자체를 렌더로 잡지 않는 이유: egui의 hover 지연·팝업 레이어에 기대 불안정하다
        let source = 생산_코드();
        assert_eq!(
            source.matches("galley.elided").count(),
            1,
            "잘렸는지 보지 않으면 툴팁이 서지 않는다"
        );
        assert_eq!(source.matches("on_hover_text(").count(), 1);
    }

    #[test]
    fn 복사본에도_비밀번호가_없다() {
        // Acceptance ③ — 로그는 클립보드로 그대로 나가므로 한 번 들어가면 회수할 수 없다.
        // 가리기는 버퍼에 쌓을 때 이미 끝나 있고(D14·T5), 복사는 그 버퍼를 그대로 옮긴다
        let mut log = LogBuffer::new();
        log.push(LogKind::Command, "PASS 진짜비밀번호");
        log.push(
            LogKind::Status,
            "sftp://deploy:진짜비밀번호@example.test:22 에 연결 중...",
        );

        let copied = log.to_text();
        assert!(
            !copied.contains("진짜비밀번호"),
            "복사본에 평문이 남았다: {copied}"
        );
        assert!(copied.contains("PASS"), "명령 자체는 남아야 한다");
    }

    #[test]
    fn 본문이_한_프레임을_그린다() {
        // 자리 계산이 뒤집힌 사각형 없이 도는지 본다 — 빈 로그와 찬 로그 둘 다
        let mut log = LogBuffer::new();
        let ctx = egui::Context::default();
        for (index, kind) in [
            LogKind::Status,
            LogKind::Command,
            LogKind::Response,
            LogKind::Error,
        ]
        .into_iter()
        .enumerate()
        {
            log.push(
                kind,
                format!("{index}번째 줄 — 아주 긴 경로 /var/www/html/app.bundle.js"),
            );
        }
        for buffer in [LogBuffer::new(), log] {
            let _ = ctx.run_ui(Default::default(), |ui| {
                egui::CentralPanel::default().show(ui, |ui| {
                    let rect =
                        egui::Rect::from_min_size(ui.max_rect().min, egui::vec2(900.0, 120.0));
                    show_log(ui, rect, &buffer, &mut MenuState::default());
                });
            });
        }
    }
}
