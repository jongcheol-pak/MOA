//! 앱 설정 대화 (FR-47).
//!
//! 타이틀바 설정 메뉴의 `설정`이 연다. 항목이 일곱 개뿐이라 좌측 카테고리 목록을 두지 않고
//! **한 면에 그룹을 세로로 나열**한다(사용자 결정) — 카테고리를 두면 목록 하나에 항목이
//! 하나씩 매달려 클릭만 늘고 빈 공간이 많아진다.
//!
//! 바꾼 값은 **즉시 반영·저장**된다(사용자 결정). 일곱 항목이 모두 독립적인 토글·선택이라
//! 서로 엮어 검증할 것이 없고, 글꼴·언어는 바뀌는 결과를 보며 고르는 편이 확실하다.
//! 그래서 바닥 버튼은 `닫기` 하나이고 `취소`가 없다.
use crate::app::settings::AppSettings;
use crate::ui::theme;
use crate::ui::widgets;
use eframe::egui;

/// 대화 크기 — 사이트 관리자(1080×680)보다 훨씬 작다. 항목이 적어 그만한 판이 필요 없다
const DIALOG_WIDTH: f32 = 420.0;
const DIALOG_HEIGHT: f32 = 400.0;

/// 뒤 화면을 덮는 어둠 — 사이트 관리자와 같은 값을 쓴다(대화가 둘로 보이지 않게)
const SCRIM_ALPHA: u8 = 140;
const SHADOW_OFFSET_Y: i8 = 18;
const SHADOW_BLUR: u8 = 60;
const SHADOW_ALPHA: u8 = 153;

const HEADER_HEIGHT: f32 = 40.0;
const FOOTER_HEIGHT: f32 = 58.0;
/// 본문 좌우 여백
const BODY_PAD_X: f32 = 20.0;
/// 헤더 제목의 왼쪽 여백 — 본문과 같은 선에서 시작한다
const HEADER_PAD_LEFT: f32 = 20.0;
const TITLE_FONT_PX: f32 = 14.0;
/// 그룹 제목 — 본문 글자보다 작고 흐리다(항목이 아니라 묶음 이름임을 보인다)
const GROUP_FONT_PX: f32 = 12.0;
/// 그룹 제목 위 여백 — 앞 그룹과 떨어뜨린다
const GROUP_GAP_TOP: f32 = 14.0;
/// 그룹 제목과 첫 항목 사이
const GROUP_GAP_BOTTOM: f32 = 6.0;
/// 닫기 버튼 좌우 여백
const CLOSE_PAD_X: f32 = 16.0;
const CLOSE_MIN_WIDTH: f32 = 72.0;

// ── 문구 ──
const TITLE: &str = "설정";
const GROUP_APPEARANCE: &str = "모양";
const GROUP_STARTUP: &str = "시작";
const GROUP_EXIT: &str = "종료";
const GROUP_FILES: &str = "파일 보기";
const LABEL_SHOW_EXTENSIONS: &str = "파일 확장명";
const LABEL_SHOW_HIDDEN: &str = "숨김 항목";
const BUTTON_CLOSE: &str = "닫기";
/// 아직 항목이 붙지 않은 그룹의 자리 표시 — T5·T6·T7이 각자 채운다
const PENDING_HINT: &str = "준비 중";

/// 대화가 한 프레임에 만들어 낸 결과.
///
/// **부수 효과가 필요한 항목마다 필드를 따로 둔다** — 값을 저장하는 것과, 그 값을 바깥
/// 세계(글꼴 등록·레지스트리·트레이 아이콘)에 반영하는 것은 서로 다른 일이라
/// `changed` 하나로는 무엇을 해야 하는지 알 수 없다. 필드는 그 일이 생기는 task가 더한다
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SettingsOutcome {
    /// 값이 하나라도 바뀌었다 — 세션을 저장해야 한다
    pub changed: bool,
}

/// 설정 대화 (FR-47) — 열림 상태만 들고 값은 빌려 쓴다.
///
/// 값을 자기 안에 복사해 두지 않는 이유: 즉시 반영이라 초안이 필요 없고, 사본을 두면
/// 화면과 실제 설정이 어긋날 수 있는 자리가 하나 더 생긴다(사이트 관리자의 `Draft`는
/// `확인`을 눌러야 반영되는 구조라 사본이 필요했다 — 여기는 그렇지 않다)
#[derive(Debug, Default)]
pub struct SettingsDialog {
    open: bool,
}

impl SettingsDialog {
    pub fn new() -> SettingsDialog {
        SettingsDialog::default()
    }

    pub fn open(&mut self) {
        self.open = true;
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn close(&mut self) {
        self.open = false;
    }

    /// 대화를 그린다. 닫혀 있으면 아무것도 그리지 않는다
    pub fn show(&mut self, ctx: &egui::Context, settings: &mut AppSettings) -> SettingsOutcome {
        if !self.open {
            return SettingsOutcome::default();
        }
        let mut outcome = SettingsOutcome::default();
        let mut close_requested = false;
        let response = egui::Modal::new(egui::Id::new("앱 설정"))
            .backdrop_color(egui::Color32::from_black_alpha(SCRIM_ALPHA))
            .frame(
                egui::Frame::new()
                    .fill(theme::SURFACE_BG)
                    .stroke(egui::Stroke::new(1.0, theme::BORDER_CONTROL))
                    .corner_radius(0)
                    .shadow(egui::epaint::Shadow {
                        offset: [0, SHADOW_OFFSET_Y],
                        blur: SHADOW_BLUR,
                        spread: 0,
                        color: egui::Color32::from_black_alpha(SHADOW_ALPHA),
                    }),
            )
            .show(ctx, |ui| {
                let (rect, _) = ui.allocate_exact_size(
                    egui::vec2(DIALOG_WIDTH, DIALOG_HEIGHT),
                    egui::Sense::hover(),
                );
                let header =
                    egui::Rect::from_min_size(rect.min, egui::vec2(rect.width(), HEADER_HEIGHT));
                let footer = egui::Rect::from_min_max(
                    egui::pos2(rect.left(), rect.bottom() - FOOTER_HEIGHT),
                    rect.max,
                );
                let body = egui::Rect::from_min_max(
                    egui::pos2(rect.left() + BODY_PAD_X, header.bottom()),
                    egui::pos2(rect.right() - BODY_PAD_X, footer.top()),
                );
                show_header(ui, header);
                outcome = show_body(ui, body, settings);
                if show_footer(ui, footer) {
                    close_requested = true;
                }
            });
        // 배경 클릭·`Esc`도 닫기다 — egui가 그 판정을 해 준다
        if close_requested || response.should_close() {
            self.close();
        }
        outcome
    }
}

/// 헤더 — 제목만 둔다. 닫기 버튼을 겹쳐 두지 않는 것은 바닥에 `닫기`가 이미 있어서다
fn show_header(ui: &egui::Ui, rect: egui::Rect) {
    ui.painter().text(
        egui::pos2(rect.left() + HEADER_PAD_LEFT, rect.center().y),
        egui::Align2::LEFT_CENTER,
        TITLE,
        egui::FontId::proportional(TITLE_FONT_PX),
        theme::TEXT,
    );
}

/// 본문 — 그룹을 세로로 쌓는다.
///
/// **항목을 배열+반복으로 묶지 않는다**(plan 비추상화 선언) — 그룹마다 컨트롤 종류와
/// 부수 효과(글꼴 재등록·레지스트리 쓰기·트레이 아이콘)가 달라, 묶으면 채우는 순간 다시 풀어야 한다
fn show_body(ui: &mut egui::Ui, rect: egui::Rect, settings: &mut AppSettings) -> SettingsOutcome {
    let mut outcome = SettingsOutcome::default();
    let mut body = ui.new_child(egui::UiBuilder::new().max_rect(rect));

    // 모양 — 글꼴 (T5가 드롭다운을 넣는다)
    group_title(&mut body, GROUP_APPEARANCE);
    pending_hint(&mut body);

    // 시작 — 자동 실행 (T6)
    group_title(&mut body, GROUP_STARTUP);
    pending_hint(&mut body);

    // 종료 — 트레이 전환 (T7)
    group_title(&mut body, GROUP_EXIT);
    pending_hint(&mut body);

    // 파일 보기 — 확장명·숨김 항목. 값을 그대로 뒤집기만 하면 되는 자리라 여기서 배선한다
    group_title(&mut body, GROUP_FILES);
    if widgets::toggle_row(&mut body, LABEL_SHOW_EXTENSIONS, settings.show_extensions) {
        settings.show_extensions = !settings.show_extensions;
        outcome.changed = true;
    }
    if widgets::toggle_row(&mut body, LABEL_SHOW_HIDDEN, settings.show_hidden) {
        settings.show_hidden = !settings.show_hidden;
        outcome.changed = true;
    }

    outcome
}

/// 그룹 제목 한 줄 — 위에 여백을 두어 앞 그룹과 떨어뜨린다
fn group_title(ui: &mut egui::Ui, text: &str) {
    ui.add_space(GROUP_GAP_TOP);
    ui.painter().text(
        egui::pos2(ui.cursor().left(), ui.cursor().top()),
        egui::Align2::LEFT_TOP,
        text,
        egui::FontId::proportional(GROUP_FONT_PX),
        theme::TEXT_MUTED,
    );
    ui.add_space(GROUP_FONT_PX + GROUP_GAP_BOTTOM);
}

/// 아직 항목이 없는 그룹의 자리 — 빈 그룹 제목만 떠 있으면 고장으로 보인다
fn pending_hint(ui: &mut egui::Ui) {
    ui.painter().text(
        egui::pos2(ui.cursor().left(), ui.cursor().top()),
        egui::Align2::LEFT_TOP,
        PENDING_HINT,
        egui::FontId::proportional(widgets::FORM_FONT_PX),
        theme::TEXT_DIM,
    );
    ui.add_space(widgets::FORM_FIELD_HEIGHT);
}

/// 푸터 — 오른쪽 끝 `닫기`. 눌렸으면 `true`
fn show_footer(ui: &mut egui::Ui, rect: egui::Rect) -> bool {
    let width = widgets::design_button_width(ui, BUTTON_CLOSE, CLOSE_PAD_X).max(CLOSE_MIN_WIDTH);
    let button_rect = egui::Rect::from_min_size(
        egui::pos2(
            rect.right() - BODY_PAD_X - width,
            rect.center().y - widgets::FORM_FIELD_HEIGHT / 2.0,
        ),
        egui::vec2(width, widgets::FORM_FIELD_HEIGHT),
    );
    let mut footer = ui.new_child(egui::UiBuilder::new().max_rect(button_rect));
    widgets::design_button(
        &mut footer,
        BUTTON_CLOSE,
        theme::TEXT_BUTTON,
        CLOSE_PAD_X,
        egui::vec2(width, widgets::FORM_FIELD_HEIGHT),
    )
    .clicked()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 닫힌_대화는_아무것도_그리지_않는다() {
        let ctx = egui::Context::default();
        let mut dialog = SettingsDialog::new();
        let mut settings = AppSettings::default();
        assert!(!dialog.is_open());
        let _ = ctx.run_ui(Default::default(), |_ui| {});
        assert_eq!(
            dialog.show(&ctx, &mut settings),
            SettingsOutcome::default(),
            "닫혀 있는데 결과가 나왔다"
        );
    }

    #[test]
    fn 열고_닫는_상태가_바뀐다() {
        let mut dialog = SettingsDialog::new();
        assert!(!dialog.is_open(), "처음에는 닫혀 있어야 한다");
        dialog.open();
        assert!(dialog.is_open());
        dialog.close();
        assert!(!dialog.is_open());
    }

    #[test]
    fn 파일_보기_토글을_누르면_값이_뒤집히고_저장을_알린다() {
        // 즉시 반영이 이 화면의 계약이다 — 누른 그 프레임에 값이 바뀌고 저장 신호가 서야 한다
        fn press(pos: egui::Pos2, pressed: bool) -> egui::Event {
            egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: egui::Modifiers::NONE,
            }
        }

        // 모달은 화면 가운데에 떠서 자리가 화면 크기에 딸린다. 여기서 보려는 것은
        // "토글이 눌리면 값이 뒤집히고 저장 신호가 서는가"이므로 본문만 떼어 그린다
        let ctx = egui::Context::default();
        let mut settings = AppSettings::default();
        assert!(settings.show_extensions, "기본값이 바뀌었다");

        let mut outcome = SettingsOutcome::default();
        let _ = ctx.run_ui(Default::default(), |ui| {
            outcome = show_body(ui, ui.max_rect(), &mut settings);
        });
        assert!(!outcome.changed, "누르지도 않았는데 바뀌었다고 한다");

        // `show_body`는 `max_rect` 왼쪽 위부터 그룹을 쌓는다 —
        // 네 번째 그룹(`파일 보기`)의 첫 줄이 `파일 확장명` 토글이다
        let y = 4.0 * (GROUP_GAP_TOP + GROUP_FONT_PX + GROUP_GAP_BOTTOM)
            + 3.0 * widgets::FORM_FIELD_HEIGHT
            + widgets::FORM_FIELD_HEIGHT / 2.0;
        let spot = egui::pos2(40.0, y);
        for (time, event) in [(0.05, press(spot, true)), (0.10, press(spot, false))] {
            let input = egui::RawInput {
                time: Some(time),
                events: vec![event],
                ..Default::default()
            };
            let _ = ctx.run_ui(input, |ui| {
                outcome = show_body(ui, ui.max_rect(), &mut settings);
            });
        }
        assert!(outcome.changed, "토글을 눌렀는데 저장 신호가 서지 않았다");
        assert!(
            !settings.show_extensions,
            "토글을 눌렀는데 값이 뒤집히지 않았다"
        );
    }
}
