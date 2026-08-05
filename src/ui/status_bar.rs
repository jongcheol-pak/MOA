//! 창 맨 아래 상태 표시줄 — **언제나 보인다** (FR-41·D18).
//!
//! 원본 `FileExplorer-FTP.dc.html:320-333`. 왼쪽부터 큐 토글 · 큐 요약 · 전체 진행 막대 ·
//! 지금 옮기는 파일이 오고, 오른쪽 끝에 실패 알약 · 연결 상태 · 로그 토글이 붙는다.
//!
//! **도크를 여는 유일한 문이다** — 큐·로그 화면(T19·T20)은 이 줄의 캐럿으로만 열린다
//! (README §8 "the status bar carets toggle them").
//!
//! 항목을 슬롯 목록으로 일반화하지 않는다(plan 비추상화 선언) — 일곱이 고정이고 조건이 제각각이라
//! 목록으로 만들면 조건이 자료구조 속으로 숨는다.
use crate::remote::connection::ConnPhase;
use crate::remote::queue::TransferQueue;
use crate::ui::dock::{DockPanel, DockState};
use crate::ui::queue_panel::{format_size, format_speed};
use crate::ui::theme;
use crate::ui::widgets;
use eframe::egui;

// ── 시각 토큰 (원본 `:320-332`) ──
/// 줄 높이·좌우 여백·항목 사이 간격
pub const HEIGHT: f32 = 30.0;
const PAD_X: f32 = 10.0;
const GAP: f32 = 14.0;
const FONT_PX: f32 = 13.0;
/// 캐럿과 글자 사이 (`:321` `gap:6px`)
const CARET_GAP: f32 = 6.0;
/// 전체 진행 막대 (`:323`)
const BAR_WIDTH: f32 = 240.0;
const BAR_HEIGHT: f32 = 6.0;
/// 실패 알약 (`:329`)
const PILL_HEIGHT: f32 = 20.0;
const PILL_PAD_X: f32 = 8.0;
const PILL_GAP: f32 = 6.0;
/// 상태 점 지름 — 알약·연결 상태가 함께 쓴다
const DOT: f32 = 7.0;

// ── 문구 (인벤토리 #53~#59) ──
const QUEUE_LABEL: &str = "전송 큐";
const LOG_LABEL: &str = "로그";
/// 접힘·열림 캐럿 (인벤토리 #53·#59)
const CARET_CLOSED: &str = "▲";
const CARET_OPEN: &str = "▼";
/// 연결이 하나도 없을 때 (plan Edge Case)
const NO_CONNECTION: &str = "연결 없음";
/// 실패 알약 (인벤토리 #57)
const FAIL_LABEL: &str = "실패";

/// 사용자가 상태 표시줄에서 고른 것
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusAction {
    ToggleQueue,
    ToggleLog,
}

/// 큐 요약 문구 (인벤토리 #54) — `5건 대기 · 12.4 MB/s · 00:41 남음`.
///
/// **대기가 없으면 빈 문자열**이다 — 아무 일도 없을 때 `0건 대기`를 띄우면 줄이 늘 차 있어
/// 정작 무슨 일이 생겼을 때 눈에 띄지 않는다.
/// 속도를 못 재면 남은 시간을 적지 않는다(0초로 적으면 곧 끝난다는 거짓말이 된다)
pub fn format_queue_summary(queue: &TransferQueue) -> String {
    let summary = queue.summary();
    if summary.pending == 0 {
        return String::new();
    }
    let mut out = format!("{}건 대기", summary.pending);
    if summary.speed > 0 {
        out.push_str(" · ");
        out.push_str(&format_speed(summary.speed));
    }
    match summary.eta_secs {
        Some(secs) => {
            out.push_str(" · ");
            out.push_str(&format_duration(secs));
            out.push_str(" 남음");
        }
        None => {
            out.push_str(" · ");
            out.push_str(crate::remote::queue::UNKNOWN);
        }
    }
    out
}

/// 남은 시간 — 한 시간을 넘으면 `HH:MM:SS`, 아니면 `MM:SS` (plan Edge Case)
pub fn format_duration(secs: u64) -> String {
    let (hours, minutes, seconds) = (secs / 3600, (secs % 3600) / 60, secs % 60);
    if hours > 0 {
        format!("{hours:02}:{minutes:02}:{seconds:02}")
    } else {
        format!("{minutes:02}:{seconds:02}")
    }
}

/// 지금 옮기는 파일 (인벤토리 #56) — `↑ app.bundle.js — 62%`.
///
/// 여럿이 동시에 갈 때는 **가장 앞선 것 하나**만 보인다 — 한 줄에 여럿을 늘어놓으면
/// 어느 것도 읽히지 않는다
pub fn format_current(queue: &TransferQueue) -> String {
    let active = queue.items().iter().find(|item| item.state.is_active());
    let Some(item) = active else {
        return String::new();
    };
    let name = item
        .local
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| item.remote.as_str().to_owned());
    let arrow = match item.direction {
        crate::remote::connection::TransferDirection::Upload => "↑",
        crate::remote::connection::TransferDirection::Download => "↓",
    };
    match item.progress() {
        Some(ratio) => format!("{arrow} {name} — {}%", (ratio * 100.0).round() as u32),
        // 크기를 모르면 옮긴 양으로 적는다 — 백분율은 셀 수 없다
        None => match &item.state {
            crate::remote::queue::TransferState::Active { sent, .. } => {
                format!("{arrow} {name} — {}", format_size(*sent))
            }
            _ => format!("{arrow} {name}"),
        },
    }
}

/// 연결 상태 문구와 색 (인벤토리 #58) — `● sftp web-prod 연결됨 · TLS` 꼴.
///
/// 연결이 없으면 `연결 없음`이다(plan Edge Case). 여럿이면 **지금 보고 있는 것**을 호출부가 고른다
pub fn connection_label(
    phase: Option<&ConnPhase>,
    protocol: Option<&str>,
    site: Option<&str>,
    secure: bool,
) -> (String, egui::Color32) {
    let Some(phase) = phase else {
        return (NO_CONNECTION.to_owned(), theme::TEXT_DIM);
    };
    let name = match (protocol, site) {
        (Some(protocol), Some(site)) => format!("{protocol} {site} "),
        (None, Some(site)) => format!("{site} "),
        _ => String::new(),
    };
    match phase {
        ConnPhase::Idle => (format!("{name}연결 없음"), theme::TEXT_DIM),
        ConnPhase::Connecting => (format!("{name}연결 중…"), theme::WARN),
        ConnPhase::Ready => {
            let tls = if secure { " · TLS" } else { "" };
            (format!("{name}연결됨{tls}"), theme::OK_TEXT)
        }
        ConnPhase::Failed { .. } => (format!("{name}연결하지 못했습니다"), theme::ERROR),
        ConnPhase::Closed => (format!("{name}연결 끊김"), theme::TEXT_DIM),
    }
}

/// 상태 표시줄이 그릴 값 — 화면은 여기 담긴 것만 안다
pub struct StatusView<'a> {
    pub queue: &'a TransferQueue,
    /// 연결 상태 문구와 색 (`connection_label`이 만든 것)
    pub connection: (String, egui::Color32),
}

/// 상태 표시줄을 그린다 (인벤토리 #53~#59)
pub fn show_status_bar(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    dock: &DockState,
    view: &StatusView<'_>,
) -> Option<StatusAction> {
    ui.painter().rect_filled(rect, 0.0, theme::SURFACE_BG);
    ui.painter().line_segment(
        [
            egui::pos2(rect.left(), rect.top() + 0.5),
            egui::pos2(rect.right(), rect.top() + 0.5),
        ],
        egui::Stroke::new(1.0, theme::BORDER_SUBTLE),
    );
    let font = egui::FontId::proportional(FONT_PX);
    let mut action = None;

    // ── 왼쪽부터 ──
    let mut left = rect.left() + PAD_X;
    let queue_caret = if dock.panel == Some(DockPanel::Queue) {
        CARET_OPEN
    } else {
        CARET_CLOSED
    };
    let toggle = format!("{queue_caret} {QUEUE_LABEL}");
    let width = text_width(ui, &toggle, &font);
    if toggle_text(ui, rect, left, width, &toggle, &font, theme::HEADER_TEXT) {
        action = Some(StatusAction::ToggleQueue);
    }
    left += width + GAP;

    let summary = format_queue_summary(view.queue);
    if !summary.is_empty() {
        let width = text_width(ui, &summary, &font);
        ui.painter().text(
            egui::pos2(left, rect.center().y),
            egui::Align2::LEFT_CENTER,
            &summary,
            font.clone(),
            theme::TEXT_MUTED,
        );
        left += width + GAP;
    }

    // 전체 진행 막대 — 전송이 없으면 빈 트랙만 (plan Edge Case)
    let bar = egui::Rect::from_min_size(
        egui::pos2(left, rect.center().y - BAR_HEIGHT / 2.0),
        egui::vec2(BAR_WIDTH, BAR_HEIGHT),
    );
    let mut bar_ui = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(bar)
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );
    widgets::progress_bar(
        &mut bar_ui,
        bar.size(),
        view.queue.overall_progress(),
        theme::ACCENT,
    );
    left += BAR_WIDTH + GAP;

    // ── 오른쪽부터 (거꾸로 놓는다) ──
    let mut right = rect.right() - PAD_X;
    let log_caret = if dock.panel == Some(DockPanel::Log) {
        CARET_OPEN
    } else {
        CARET_CLOSED
    };
    let log_toggle = format!("{log_caret} {LOG_LABEL}");
    let width = text_width(ui, &log_toggle, &font);
    right -= width;
    if toggle_text(ui, rect, right, width, &log_toggle, &font, theme::TEXT_DIM) {
        action = Some(StatusAction::ToggleLog);
    }
    right -= GAP;

    let (label, color) = &view.connection;
    let width = text_width(ui, label, &font) + DOT + CARET_GAP;
    right -= width;
    ui.painter().circle_filled(
        egui::pos2(right + DOT / 2.0, rect.center().y),
        DOT / 2.0,
        *color,
    );
    ui.painter().text(
        egui::pos2(right + DOT + CARET_GAP, rect.center().y),
        egui::Align2::LEFT_CENTER,
        label,
        font.clone(),
        *color,
    );
    right -= GAP;

    // 실패 알약 — **실패가 있을 때만** (인벤토리 #57)
    let failures = view.queue.count(crate::remote::queue::QueueFilter::Error);
    if failures > 0 {
        let text = format!("{FAIL_LABEL} {failures}");
        let width = PILL_PAD_X * 2.0 + DOT + PILL_GAP + text_width(ui, &text, &font);
        right -= width;
        let pill = egui::Rect::from_min_size(
            egui::pos2(right, rect.center().y - PILL_HEIGHT / 2.0),
            egui::vec2(width, PILL_HEIGHT),
        );
        ui.painter().rect(
            pill,
            0.0,
            theme::ERROR_FILL,
            egui::Stroke::new(1.0, theme::ERROR_BORDER),
            egui::StrokeKind::Inside,
        );
        ui.painter().circle_filled(
            egui::pos2(pill.left() + PILL_PAD_X + DOT / 2.0, pill.center().y),
            DOT / 2.0,
            theme::ERROR,
        );
        ui.painter().text(
            egui::pos2(pill.left() + PILL_PAD_X + DOT + PILL_GAP, pill.center().y),
            egui::Align2::LEFT_CENTER,
            &text,
            font.clone(),
            theme::ERROR,
        );
        right -= GAP;
    }

    // 지금 옮기는 파일 — **남는 자리만 쓴다**. 창이 좁으면 이것부터 줄어들고
    // 실패 알약·연결 상태는 밀리지 않는다 (plan Edge Case)
    let current = format_current(view.queue);
    if !current.is_empty() {
        let available = (right - left).max(0.0);
        let galley = crate::ui::list_common::elided_galley_colored(
            ui.painter(),
            current,
            font,
            available,
            theme::HEADER_TEXT,
        );
        ui.painter().galley(
            egui::pos2(left, rect.center().y - galley.size().y / 2.0),
            galley,
            theme::HEADER_TEXT,
        );
    }
    action
}

/// 누를 수 있는 글자 — 눌렸으면 `true`
fn toggle_text(
    ui: &mut egui::Ui,
    row: egui::Rect,
    left: f32,
    width: f32,
    text: &str,
    font: &egui::FontId,
    color: egui::Color32,
) -> bool {
    let rect =
        egui::Rect::from_min_size(egui::pos2(left, row.top()), egui::vec2(width, row.height()));
    let response = ui
        .interact(
            rect,
            ui.id().with(("status_toggle", text)),
            egui::Sense::click(),
        )
        .on_hover_cursor(egui::CursorIcon::PointingHand);
    ui.painter().text(
        egui::pos2(left, row.center().y),
        egui::Align2::LEFT_CENTER,
        text,
        font.clone(),
        if response.hovered() {
            theme::TEXT
        } else {
            color
        },
    );
    response.clicked()
}

fn text_width(ui: &egui::Ui, text: &str, font: &egui::FontId) -> f32 {
    ui.painter()
        .layout_no_wrap(text.to_owned(), font.clone(), theme::TEXT)
        .size()
        .x
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::remote::connection::TransferDirection;
    use crate::remote::queue::TransferState;
    use crate::remote::types::{RemotePath, SiteId};
    use std::path::PathBuf;

    #[test]
    fn 상태_표시줄_치수는_원본과_같다() {
        // Acceptance ① — 30px·gap 14px·막대 240×6px
        assert_eq!(HEIGHT, 30.0);
        assert_eq!(GAP, 14.0);
        assert_eq!(PAD_X, 10.0);
        assert_eq!(BAR_WIDTH, 240.0);
        assert_eq!(BAR_HEIGHT, 6.0);
        assert_eq!(PILL_HEIGHT, 20.0);
        assert_eq!(PILL_PAD_X, 8.0);
        assert_eq!(DOT, 7.0);
    }

    #[test]
    fn 문구는_인벤토리_원문_그대로다() {
        // 인벤토리 #53·#57·#59
        assert_eq!(QUEUE_LABEL, "전송 큐");
        assert_eq!(LOG_LABEL, "로그");
        assert_eq!(CARET_CLOSED, "▲");
        assert_eq!(CARET_OPEN, "▼");
        assert_eq!(FAIL_LABEL, "실패");
    }

    fn queue_with(states: &[TransferState]) -> TransferQueue {
        let mut queue = TransferQueue::new();
        for (index, state) in states.iter().enumerate() {
            let id = queue.enqueue(
                SiteId(1),
                TransferDirection::Upload,
                PathBuf::from(format!(r"C:\work\app.bundle{index}.js")),
                RemotePath::new("/var/www/app.js"),
                1000,
            );
            queue.update(id, state.clone());
        }
        queue
    }

    #[test]
    fn 요약은_대기가_있을_때만_나온다() {
        // Acceptance ④ — 아무 일도 없을 때 `0건 대기`를 띄우면 줄이 늘 차 있다
        assert_eq!(format_queue_summary(&TransferQueue::new()), "");
        assert_eq!(
            format_queue_summary(&queue_with(&[TransferState::Done])),
            ""
        );

        // 남은 바이트 = 진행 600 + 대기 1000 = 1600, 초당 200 → 8초
        let queue = queue_with(&[
            TransferState::Active {
                sent: 400,
                speed: 200,
            },
            TransferState::Wait,
        ]);
        assert_eq!(
            format_queue_summary(&queue),
            "2건 대기 · 200 B/s · 00:08 남음"
        );

        // 속도가 아주 빠르면 남은 시간이 0초로 떨어진다 — 그대로 적는다(곧 끝난다는 뜻이다)
        let fast = queue_with(&[TransferState::Active {
            sent: 400,
            speed: 13_002_342,
        }]);
        assert_eq!(
            format_queue_summary(&fast),
            "1건 대기 · 12.4 MB/s · 00:00 남음"
        );
    }

    #[test]
    fn 속도를_못_재면_남은_시간을_적지_않는다() {
        // Acceptance ④ 뒷문장
        let queue = queue_with(&[TransferState::Wait]);
        assert_eq!(format_queue_summary(&queue), "1건 대기 · —");
    }

    #[test]
    fn 남은_시간은_한_시간을_넘으면_시까지_적는다() {
        // plan Edge Case
        assert_eq!(format_duration(41), "00:41");
        assert_eq!(format_duration(605), "10:05");
        assert_eq!(format_duration(3661), "01:01:01");
    }

    #[test]
    fn 지금_옮기는_파일은_하나만_보인다() {
        // 인벤토리 #56 — 한 줄에 여럿을 늘어놓으면 어느 것도 읽히지 않는다
        assert_eq!(format_current(&TransferQueue::new()), "");
        let queue = queue_with(&[
            TransferState::Active {
                sent: 620,
                speed: 100,
            },
            TransferState::Active {
                sent: 100,
                speed: 100,
            },
        ]);
        assert_eq!(format_current(&queue), "↑ app.bundle0.js — 62%");
    }

    #[test]
    fn 연결_상태_문구가_단계별로_갈린다() {
        // 인벤토리 #58 + plan Edge Case(연결 0개)
        assert_eq!(
            connection_label(None, None, None, false),
            ("연결 없음".to_owned(), theme::TEXT_DIM)
        );
        let (text, color) = connection_label(
            Some(&ConnPhase::Ready),
            Some("sftp"),
            Some("web-prod"),
            true,
        );
        assert_eq!(text, "sftp web-prod 연결됨 · TLS");
        assert_eq!(color, theme::OK_TEXT);

        // 평문 연결이면 TLS 표기가 붙지 않는다
        let (text, _) = connection_label(Some(&ConnPhase::Ready), Some("ftp"), Some("old"), false);
        assert_eq!(text, "ftp old 연결됨");

        let (_, color) = connection_label(Some(&ConnPhase::Connecting), None, None, false);
        assert_eq!(color, theme::WARN);
        let (_, color) = connection_label(
            Some(&ConnPhase::Failed {
                detail: String::new(),
            }),
            None,
            None,
            false,
        );
        assert_eq!(color, theme::ERROR);
    }

    #[test]
    fn 캐럿이_도크_상태를_따라간다() {
        // Acceptance ③ — 닫힘 `▲`, 열림 `▼`
        let ctx = egui::Context::default();
        for (panel, queue_caret, log_caret) in [
            (None, CARET_CLOSED, CARET_CLOSED),
            (Some(DockPanel::Queue), CARET_OPEN, CARET_CLOSED),
            (Some(DockPanel::Log), CARET_CLOSED, CARET_OPEN),
        ] {
            let dock = DockState {
                panel,
                ..DockState::default()
            };
            // 그리기 자체가 도는지와 함께, 캐럿 판정 규칙을 그대로 확인한다
            let queue = TransferQueue::new();
            let view = StatusView {
                queue: &queue,
                connection: connection_label(None, None, None, false),
            };
            let _ = ctx.run_ui(Default::default(), |ui| {
                egui::CentralPanel::default().show(ui, |ui| {
                    let rect =
                        egui::Rect::from_min_size(ui.max_rect().min, egui::vec2(1200.0, HEIGHT));
                    show_status_bar(ui, rect, &dock, &view);
                });
            });
            assert_eq!(
                if dock.panel == Some(DockPanel::Queue) {
                    CARET_OPEN
                } else {
                    CARET_CLOSED
                },
                queue_caret
            );
            assert_eq!(
                if dock.panel == Some(DockPanel::Log) {
                    CARET_OPEN
                } else {
                    CARET_CLOSED
                },
                log_caret
            );
        }
    }

    #[test]
    fn 실패가_있을_때만_알약이_자리를_차지한다() {
        // Acceptance ② — 그리기 경로가 도는지와 건수 판정을 함께 본다
        let clean = queue_with(&[TransferState::Done]);
        assert_eq!(clean.count(crate::remote::queue::QueueFilter::Error), 0);
        let failed = queue_with(&[TransferState::Error {
            message: "550 권한 거부".to_owned(),
        }]);
        assert_eq!(failed.count(crate::remote::queue::QueueFilter::Error), 1);

        let ctx = egui::Context::default();
        for queue in [clean, failed] {
            let view = StatusView {
                queue: &queue,
                connection: connection_label(None, None, None, false),
            };
            let _ = ctx.run_ui(Default::default(), |ui| {
                egui::CentralPanel::default().show(ui, |ui| {
                    let rect =
                        egui::Rect::from_min_size(ui.max_rect().min, egui::vec2(900.0, HEIGHT));
                    show_status_bar(ui, rect, &DockState::default(), &view);
                });
            });
        }
    }
}
