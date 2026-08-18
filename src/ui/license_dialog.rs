//! 오픈소스 라이선스 대화 (FR-57).
//!
//! 타이틀바 설정 메뉴의 `오픈소스 라이선스`가 연다. 왼쪽에 구성 요소 목록, 오른쪽에 고른
//! 것의 라이선스 전문을 두는 좌우 2단이며, 크기는 사이트 관리자와 같은 1080×680이다 —
//! 같은 창에서 뜨는 큰 대화가 서로 다른 크기면 판이 흔들려 보인다.
//!
//! 고지 자료는 `app::licenses`가 exe에 담긴 자산에서 읽는다(대화를 처음 열 때 한 번).
//! 이 모듈은 그리기만 하고 자료를 만들지 않는다.
use crate::app::licenses::{self, CrateEntry, LicenseData};
use crate::i18n;
use crate::ui::dialog;
use crate::ui::theme;
use eframe::egui;

/// 대화 크기 — 사이트 관리자와 같다
const DIALOG_WIDTH: f32 = 1080.0;
const DIALOG_HEIGHT: f32 = 680.0;

/// 헤더 — 설정·사이트 관리자와 같은 높이라 번갈아 열어도 제목 줄이 제자리에 선다
const HEADER_HEIGHT: f32 = 40.0;
const HEADER_PAD_LEFT: f32 = 20.0;
const TITLE_FONT_PX: f32 = 16.0;

/// 본문 여백과 두 열 사이 간격 — 사이트 관리자와 같은 값
const BODY_PAD_X: f32 = 18.0;
const BODY_PAD_TOP: f32 = 6.0;
const BODY_GAP: f32 = 22.0;
/// 좌측 목록 폭 — 사이트 관리자(400)보다 좁다. 담기는 것이 이름 한 줄뿐이고,
/// 남는 자리는 전문을 넓게 읽는 데 쓰는 편이 낫다
const LEFT_WIDTH: f32 = 320.0;

/// 안내 문구와 그 아래 개수 사이
const INTRO_LINE_GAP: f32 = 2.0;
/// 안내와 목록 사이
const INTRO_GAP: f32 = 8.0;
/// 목록 웰 안쪽 여백
const LIST_PAD_X: f32 = 6.0;
const LIST_PAD_Y: f32 = 6.0;
/// 목록 한 줄 높이 — 이름과 버전을 한 줄에 담는다
const ROW_HEIGHT: f32 = 24.0;
/// 이름 왼쪽 여백
const ROW_PAD_X: f32 = 8.0;
/// 이름과 버전 사이
const VERSION_GAP: f32 = 8.0;

/// 본문 글자 크기 — 폼 글꼴과 같다
const BODY_FONT_PX: f32 = 13.0;
/// 전문 글자 크기 — 본문보다 한 단 작게 두어 긴 원문이 한눈에 들어오게 한다
const TEXT_FONT_PX: f32 = 12.0;
/// 전문 위에 붙는 라이선스 이름
const TEXT_TITLE_FONT_PX: f32 = 13.0;
/// 오른쪽 열의 줄 간격
const DETAIL_GAP: f32 = 6.0;
/// 전문 사이를 가르는 여백 — 이중 라이선스는 전문이 둘 이상 이어진다
const TEXT_GAP: f32 = 14.0;

/// 오픈소스 라이선스 대화 — 열림 상태와 고른 자리만 든다.
///
/// 자료를 자기 안에 복사하지 않는 이유: 자산은 `'static`이고 바뀌지 않아 사본이 필요 없다
#[derive(Debug, Default)]
pub struct LicenseDialog {
    open: bool,
    selected: usize,
}

impl LicenseDialog {
    pub fn new() -> LicenseDialog {
        LicenseDialog::default()
    }

    pub fn open(&mut self) {
        self.open = true;
        self.selected = 0;
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn close(&mut self) {
        self.open = false;
    }

    /// 대화를 그린다. 닫혀 있으면 아무것도 그리지 않는다
    pub fn show(&mut self, ctx: &egui::Context) {
        if !self.open {
            return;
        }
        let data = licenses::load();
        self.selected = clamp_selection(self.selected, data.crates.len());

        let buttons = [dialog::ButtonSpec::strong(i18n::close())];
        let shell = dialog::show_fixed(
            ctx,
            egui::Id::new("라이선스 대화"),
            egui::vec2(DIALOG_WIDTH, DIALOG_HEIGHT),
            &buttons,
            |ui, rect| {
                let header =
                    egui::Rect::from_min_size(rect.min, egui::vec2(rect.width(), HEADER_HEIGHT));
                show_header(ui, header);
                let content = egui::Rect::from_min_max(
                    egui::pos2(rect.left() + BODY_PAD_X, header.bottom() + BODY_PAD_TOP),
                    egui::pos2(rect.right() - BODY_PAD_X, rect.bottom()),
                );
                let left = egui::Rect::from_min_size(
                    content.min,
                    egui::vec2(LEFT_WIDTH, content.height()),
                );
                let right = egui::Rect::from_min_max(
                    egui::pos2(left.right() + BODY_GAP, content.top()),
                    content.max,
                );
                self.show_list(ui, left, data);
                self.show_detail(ui, right, data);
            },
        );
        if shell.clicked.is_some() || shell.should_close {
            self.close();
        }
    }

    /// 좌측 — 안내 한 줄과 구성 요소 목록
    fn show_list(&mut self, ui: &mut egui::Ui, column: egui::Rect, data: &LicenseData) {
        // 안내는 열 폭에 맞춰 접힌다 — 언어에 따라 한 줄이 되기도 두 줄이 되기도 해서
        // 높이를 상수로 박지 않고 그려진 것을 재어 아래를 잡는다
        let intro = ui.painter().layout(
            i18n::licenses_intro().to_owned(),
            egui::FontId::proportional(BODY_FONT_PX),
            theme::TEXT_MUTED,
            column.width(),
        );
        let intro_bottom = column.top() + intro.size().y;
        ui.painter().galley(column.min, intro, theme::TEXT_MUTED);
        let count = ui.painter().layout_no_wrap(
            i18n::dynamic::licenses_component_count(data.crates.len()),
            egui::FontId::proportional(BODY_FONT_PX),
            theme::TEXT_DIM,
        );
        let count_bottom = intro_bottom + INTRO_LINE_GAP + count.size().y;
        ui.painter().galley(
            egui::pos2(column.left(), intro_bottom + INTRO_LINE_GAP),
            count,
            theme::TEXT_DIM,
        );
        let well = egui::Rect::from_min_max(
            egui::pos2(column.left(), count_bottom + INTRO_GAP),
            column.max,
        );
        ui.painter().rect(
            well,
            0.0,
            theme::WELL_BG,
            egui::Stroke::new(1.0, theme::PANE_BORDER),
            egui::StrokeKind::Inside,
        );
        if data.crates.is_empty() {
            ui.painter().text(
                well.center(),
                egui::Align2::CENTER_CENTER,
                i18n::licenses_unavailable(),
                egui::FontId::proportional(BODY_FONT_PX),
                theme::TEXT_DIM,
            );
            return;
        }

        let rows = egui::Rect::from_min_max(
            egui::pos2(well.left() + LIST_PAD_X, well.top() + LIST_PAD_Y),
            egui::pos2(well.right() - LIST_PAD_X, well.bottom() - LIST_PAD_Y),
        );
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(rows));
        child.set_clip_rect(rows);
        egui::ScrollArea::vertical()
            .id_salt("라이선스 목록")
            .auto_shrink([false; 2])
            .show_rows(&mut child, ROW_HEIGHT, data.crates.len(), |ui, range| {
                ui.spacing_mut().item_spacing.y = 0.0;
                for index in range {
                    if show_row(ui, &data.crates[index], index == self.selected) {
                        self.selected = index;
                    }
                }
            });
    }

    /// 우측 — 고른 항목의 이름·SPDX·저작권과 전문들
    fn show_detail(&self, ui: &mut egui::Ui, column: egui::Rect, data: &LicenseData) {
        let Some(entry) = data.crates.get(self.selected) else {
            return;
        };
        let mut child = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(column)
                .layout(egui::Layout::top_down(egui::Align::Min)),
        );
        child.set_clip_rect(column);
        child.spacing_mut().item_spacing.y = DETAIL_GAP;
        egui::ScrollArea::vertical()
            .id_salt("라이선스 전문")
            .auto_shrink([false; 2])
            .show(&mut child, |ui| {
                ui.label(
                    egui::RichText::new(format!("{} {}", entry.name, entry.version))
                        .size(TITLE_FONT_PX)
                        .color(theme::TEXT),
                );
                ui.label(
                    egui::RichText::new(&entry.spdx)
                        .size(BODY_FONT_PX)
                        .color(theme::TEXT_MUTED),
                );
                if !entry.authors.is_empty() {
                    ui.label(
                        egui::RichText::new(format!(
                            "{}: {}",
                            i18n::licenses_copyright(),
                            entry.authors.join(", ")
                        ))
                        .size(BODY_FONT_PX)
                        .color(theme::TEXT_MUTED),
                    );
                }
                if entry.bundled {
                    ui.label(
                        egui::RichText::new(i18n::licenses_bundled_note())
                            .size(BODY_FONT_PX)
                            .color(theme::TEXT_DIM),
                    );
                }
                if entry.standard_text {
                    ui.label(
                        egui::RichText::new(i18n::licenses_standard_note())
                            .size(BODY_FONT_PX)
                            .color(theme::TEXT_DIM),
                    );
                }
                // 이중 라이선스는 전문이 둘 이상이다 — 선언 그대로 전부 보인다 (plan D4)
                for text in entry.texts(data) {
                    ui.add_space(TEXT_GAP);
                    ui.separator();
                    ui.label(
                        egui::RichText::new(&text.spdx)
                            .size(TEXT_TITLE_FONT_PX)
                            .color(theme::TEXT_BUTTON),
                    );
                    ui.label(
                        egui::RichText::new(&text.body)
                            .size(TEXT_FONT_PX)
                            .color(theme::TEXT_LOG),
                    );
                }
            });
    }
}

/// 제목 줄 — 설정 메뉴 항목과 같은 문구를 쓴다
fn show_header(ui: &mut egui::Ui, rect: egui::Rect) {
    ui.painter().text(
        egui::pos2(rect.left() + HEADER_PAD_LEFT, rect.center().y),
        egui::Align2::LEFT_CENTER,
        i18n::titlebar_licenses(),
        egui::FontId::proportional(TITLE_FONT_PX),
        theme::TEXT,
    );
}

/// 목록 한 줄 — 눌렸으면 `true`
fn show_row(ui: &mut egui::Ui, entry: &CrateEntry, selected: bool) -> bool {
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), ROW_HEIGHT),
        egui::Sense::click(),
    );
    if selected {
        ui.painter().rect_filled(rect, 0.0, theme::ROW_HOT);
    } else if response.hovered() {
        ui.painter().rect_filled(rect, 0.0, theme::BORDER_SUBTLE);
    }
    let name_color = if selected {
        theme::TEXT_SELECTED
    } else {
        theme::TEXT
    };
    let version = ui.painter().layout_no_wrap(
        entry.version.clone(),
        egui::FontId::proportional(BODY_FONT_PX),
        theme::TEXT_DIM,
    );
    // 버전을 오른쪽에 붙이고 이름이 그 앞까지만 쓰게 한다 — 긴 이름은 말줄임된다
    let name_width = (rect.width() - ROW_PAD_X * 2.0 - version.size().x - VERSION_GAP).max(0.0);
    let name = ui.painter().layout(
        entry.name.clone(),
        egui::FontId::proportional(BODY_FONT_PX),
        name_color,
        name_width,
    );
    ui.painter().galley(
        egui::pos2(
            rect.left() + ROW_PAD_X,
            rect.center().y - name.size().y / 2.0,
        ),
        name,
        name_color,
    );
    ui.painter().galley(
        egui::pos2(
            rect.right() - ROW_PAD_X - version.size().x,
            rect.center().y - version.size().y / 2.0,
        ),
        version,
        theme::TEXT_DIM,
    );
    response.clicked()
}

/// 고른 자리를 목록 안으로 되돌린다 — 목록이 비었으면 0이다
fn clamp_selection(selected: usize, count: usize) -> usize {
    if count == 0 || selected < count {
        selected.min(count.saturating_sub(1))
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::licenses::{LicenseData, LicenseText};

    fn 항목(name: &str, indices: Vec<usize>) -> CrateEntry {
        CrateEntry {
            name: name.into(),
            version: "1.0.0".into(),
            spdx: "MIT OR Apache-2.0".into(),
            authors: Vec::new(),
            text_indices: indices,
            standard_text: false,
            bundled: false,
        }
    }

    #[test]
    fn 열면_첫_항목이_골라진다() {
        let mut dialog = LicenseDialog::new();
        assert!(!dialog.is_open());
        dialog.open();
        assert!(dialog.is_open());
        assert_eq!(dialog.selected, 0);
        dialog.close();
        assert!(!dialog.is_open());
    }

    /// 닫힌 대화가 아무것도 그리지 않는지 — `show`의 첫 줄이 그 가드다.
    ///
    /// `egui::Context`를 만들어 한 프레임 돌려 셰이프가 하나도 나오지 않는 것으로 잰다
    #[test]
    fn 닫혀_있으면_아무것도_그리지_않는다() {
        let ctx = egui::Context::default();
        let mut dialog = LicenseDialog::new();
        let output = ctx.run_ui(Default::default(), |ctx| dialog.show(ctx));
        assert!(!dialog.is_open());
        let shapes: usize = output
            .shapes
            .iter()
            .filter(|shape| !matches!(shape.shape, egui::epaint::Shape::Noop))
            .count();
        assert_eq!(shapes, 0, "닫힌 대화가 무언가를 그렸다");
    }

    #[test]
    fn 고른_자리가_목록을_벗어나면_되돌린다() {
        assert_eq!(clamp_selection(0, 0), 0);
        assert_eq!(clamp_selection(7, 0), 0);
        assert_eq!(clamp_selection(3, 10), 3);
        assert_eq!(clamp_selection(10, 10), 0);
        assert_eq!(clamp_selection(99, 3), 0);
    }

    #[test]
    fn 이중_라이선스는_전문을_차례대로_모두_준다() {
        let data = LicenseData {
            schema: 1,
            lock_fingerprint: 0,
            crates: vec![항목("foo", vec![1, 0])],
            texts: vec![
                LicenseText {
                    spdx: "MIT".into(),
                    body: "엠아이티".into(),
                },
                LicenseText {
                    spdx: "Apache-2.0".into(),
                    body: "아파치".into(),
                },
            ],
        };
        let labels: Vec<&str> = data.crates[0]
            .texts(&data)
            .iter()
            .map(|text| text.spdx.as_str())
            .collect();
        // `text_indices` 차례대로다 — 자산이 담은 순서를 화면이 바꾸지 않는다
        assert_eq!(labels, ["Apache-2.0", "MIT"]);
    }
}
