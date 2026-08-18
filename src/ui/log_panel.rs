//! 서버 로그 화면 (FR-40) — 원본 `FileExplorer-FTP.dc.html:308-313`.
//!
//! 도크 셸(`ui::dock`)을 큐 화면과 나눠 쓰고, 여기서는 본문만 그린다.
//! 한 줄은 **시각(62px) · 종류(44px) · 본문**이며 고정폭 글꼴 12px/17px이다 — 서버가 준
//! 응답 코드와 경로가 자리를 맞춰 읽히도록.
//!
//! **버퍼를 고치지 않는다** — 읽어서 그리고, `⧉`(복사)는 값으로 돌려준다.
//! 비밀번호 가리기는 이미 버퍼에 들어가기 전에 끝나 있다(D14·T5) — 여기서 다시 하지 않는다.
use crate::remote::log::{LogBuffer, LogKind};
use crate::ui::theme;
use eframe::egui;

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

/// 로그 본문을 그린다. 새 줄이 오면 **맨 아래에 붙는다** (Acceptance ④).
///
/// 사용자가 위로 올려 둔 상태면 따라가지 않는다 — egui의 `stick_to_bottom`이 그 규칙을
/// 그대로 구현한다(바닥에 있을 때만 붙는다)
pub fn show_log(ui: &mut egui::Ui, rect: egui::Rect, log: &LogBuffer) {
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(rect)
            .layout(egui::Layout::top_down(egui::Align::Min)),
    );
    child.set_clip_rect(rect);
    let lines: Vec<&crate::remote::log::LogLine> = log.iter().collect();
    let row_height = LINE_HEIGHT + LINE_GAP;
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .stick_to_bottom(true)
        .show_rows(&mut child, row_height, lines.len(), |ui, range| {
            ui.spacing_mut().item_spacing.y = LINE_GAP;
            for index in range {
                if let Some(line) = lines.get(index) {
                    show_line(ui, line);
                }
            }
        });
}

/// 한 줄 — 시각 · 종류 · 본문.
///
/// 세 열을 **라벨로 놓는다** — `painter`로 그리면 위젯이 아니라 마우스로 끌어 고를 수 없다
/// (2026-08-18 사용자 요청). egui는 여러 라벨에 걸친 선택을 스스로 이어 주므로, 한 줄의 세
/// 열은 공백으로 다음 줄은 개행으로 이어져 복사된다
fn show_line(ui: &mut egui::Ui, line: &crate::remote::log::LogLine) {
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
    selectable_cell(
        ui,
        egui::pos2(time_left, rect.top()),
        TIME_WIDTH,
        &line.time,
        theme::TEXT_MUTED,
    );
    selectable_cell(
        ui,
        egui::pos2(kind_left, rect.top()),
        KIND_WIDTH,
        line.kind.label(),
        color,
    );
    // **본문 색은 종류와 무관하게 하나다** — 원본이 본문 span에 고정색을 준다(`:313`).
    // 종류별 색은 앞의 종류 열에만 붙는다
    selectable_cell(
        ui,
        egui::pos2(text_left, rect.top()),
        rect.right() - PAD_X - text_left,
        &line.text,
        theme::TEXT_LOG,
    );
}

/// 열 한 칸 — 그 자리에 **고를 수 있는 라벨** 하나를 놓는다.
///
/// 길어도 줄바꿈하지 않는다(행 높이가 고정이라 두 줄이 되면 아래 줄과 겹친다).
/// 잘려 보여도 **전체를 고르면 원문이 복사된다** — egui가 갤리의 원문을 준다
fn selectable_cell(
    ui: &mut egui::Ui,
    at: egui::Pos2,
    width: f32,
    text: &str,
    color: egui::Color32,
) {
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(egui::Rect::from_min_size(
                at,
                egui::vec2(width.max(0.0), LINE_HEIGHT),
            ))
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );
    child.add(
        egui::Label::new(
            egui::RichText::new(text)
                .font(egui::FontId::monospace(FONT_PX))
                .color(color),
        )
        .selectable(true)
        .truncate(),
    );
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

    #[test]
    fn 본문_색은_종류와_무관하게_하나다() {
        // spec 리뷰 M1 — 원본은 본문 span에 `#B4B4B4` 고정색을 준다(`:313`).
        // 종류별 색은 **앞의 종류 열에만** 붙는다. 오류 줄 본문까지 빨갛게 칠하면 원본과 다르다
        let ctx = egui::Context::default();
        let mut colors = Vec::new();
        let _ = ctx.run_ui(Default::default(), |ui| {
            for kind in [LogKind::Status, LogKind::Error] {
                let galley = crate::ui::list_common::elided_galley_colored(
                    ui.painter(),
                    format!("{kind:?} 줄"),
                    egui::FontId::monospace(FONT_PX),
                    200.0,
                    theme::TEXT_LOG,
                );
                colors.push(galley.job.sections[0].format.color);
            }
        });
        assert_eq!(colors, vec![theme::TEXT_LOG, theme::TEXT_LOG]);
        // 종류 열은 여전히 종류별로 갈린다
        assert_ne!(
            kind_colors(LogKind::Error).0,
            kind_colors(LogKind::Status).0
        );
    }

    #[test]
    fn 로그_줄은_고를_수_있는_라벨이고_열_x가_원본과_같다() {
        // 2026-08-18 사용자 요청 — `painter`로 그리면 위젯이 아니라 끌어서 고를 수 없다.
        // 라벨로 바꾸면서 열 x가 밀리지 않았는지 함께 본다(시각 10 · 종류 82 · 본문 136)
        let mut log = LogBuffer::new();
        log.push(LogKind::Response, "226 Transfer complete");
        let ctx = egui::Context::default();
        let mut texts: Vec<(String, f32)> = Vec::new();
        let output = ctx.run_ui(Default::default(), |ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                let rect =
                    egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(600.0, 100.0));
                show_log(ui, rect, &log);
            });
        });
        for clipped in &output.shapes {
            if let egui::Shape::Text(text) = &clipped.shape {
                texts.push((text.galley.text().to_owned(), text.pos.x));
            }
        }
        let 찾는다 = |조각: &str| {
            texts
                .iter()
                .find(|(text, _)| text.contains(조각))
                .map(|(_, x)| *x)
                .unwrap_or_else(|| panic!("`{조각}`이 그려지지 않았다: {texts:?}"))
        };
        assert_eq!(찾는다("응답:"), PAD_X + TIME_WIDTH + COLUMN_GAP);
        assert_eq!(
            찾는다("226"),
            PAD_X + TIME_WIDTH + COLUMN_GAP + KIND_WIDTH + COLUMN_GAP
        );
        // 시각은 `HH:MM:SS`라 내용을 단정하지 않고 자리만 본다
        assert!(
            texts.iter().any(|(_, x)| (*x - PAD_X).abs() < 0.01),
            "시각 열이 왼쪽 여백 자리에 없다: {texts:?}"
        );

        // 끌어서 고르는 것은 egui의 라벨 선택이 맡는다 — 그 스위치가 꺼지면 라벨로 바꿔도
        // 소용없다. 앱이 나중에 그것을 끄면 여기서 붉어진다
        let interaction = ctx.style_of(egui::Theme::Dark).interaction.clone();
        assert!(interaction.selectable_labels, "라벨 선택이 꺼져 있다");
        assert!(
            interaction.multi_widget_text_select,
            "여러 줄에 걸친 선택이 꺼져 있다"
        );
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
                    show_log(ui, rect, &buffer);
                });
            });
        }
    }
}
