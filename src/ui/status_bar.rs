//! 창 맨 아래 상태 표시줄 — **언제나 보인다** (FR-41·D18).
//!
//! 원본 `FileExplorer-FTP.dc.html:320-333`. 왼쪽부터 큐 토글 · 큐 요약 · 전체 진행 막대 ·
//! 지금 옮기는 파일이 오고, 오른쪽 끝에 실패 알약이 붙는다.
//!
//! 오른쪽 끝에 있던 **연결 상태와 로그 토글은 뺐다**(사용자 결정) — 연결 상태는 사이드바의
//! 상태 점과 탭 배지가 이미 알리고, 서버 로그는 도크 안 `서버 로그` 탭으로 연다.
//!
//! **도크를 여는 문이다** — 큐·로그 화면(T19·T20)은 이 줄의 캐럿으로 도크를 연 뒤 그 안에서 고른다
//! (README §8 "the status bar carets toggle them").
//!
//! 항목을 슬롯 목록으로 일반화하지 않는다(plan 비추상화 선언) — 다섯이 고정이고 조건이 제각각이라
//! 목록으로 만들면 조건이 자료구조 속으로 숨는다.
use crate::remote::queue::TransferQueue;
use crate::ui::dock::DockState;
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

/// 접힘·열림 캐럿 (인벤토리 #53) — **아이콘은 아이콘 글꼴(phosphor)에서만 가져온다**
/// (프로젝트 규약, AGENTS 참조). 원본 HTML의 문자를 그대로 쓰면 글꼴에 없을 때 두부가 된다
const CARET_CLOSED: &str = egui_phosphor::regular::CARET_UP;
const CARET_OPEN: &str = egui_phosphor::regular::CARET_DOWN;
/// 사용자가 상태 표시줄에서 고른 것 — 지금은 큐 토글 하나뿐이다.
///
/// 열거형으로 남겨 둔다: 이 줄이 도크를 여는 문이라 나중에 다른 화면이 붙을 자리다
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusAction {
    ToggleQueue,
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
    // 조각을 이어 붙이지 않고 문장을 통째로 만든다 — 영어는 `3 pending · 1분 left`처럼
    // 조각 순서가 달라 여기서 붙이면 어순이 깨진다 (D2)
    let speed = (summary.speed > 0).then(|| format_speed(summary.speed));
    let eta = summary.eta_secs.map(format_duration);
    crate::i18n::dynamic::queue_summary(
        summary.pending,
        speed.as_deref(),
        eta.as_deref(),
        crate::remote::queue::UNKNOWN,
    )
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
        crate::remote::connection::TransferDirection::Upload => egui_phosphor::regular::ARROW_UP,
        crate::remote::connection::TransferDirection::Download => {
            egui_phosphor::regular::ARROW_DOWN
        }
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

/// 이 줄 가운데에 보일 것 — 실패 사유가 있으면 **그것이 먼저**다 (FR-39·인벤토리 #56).
///
/// 실패는 잠깐 떴다 사라지고 전송 진행은 큐 화면에도 있다 — 둘이 겹칠 때 급한 쪽은 실패다
pub fn status_message(view: &StatusView<'_>) -> (String, egui::Color32) {
    if let Some(notice) = view.notice {
        return (notice.to_owned(), theme::ERROR);
    }
    // 펼치는 동안에는 그 사실을 먼저 알린다 — 큐에 들어가기 전이라 진행 문구로는 드러나지
    // 않는 구간이다(이미 도는 전송이 있으면 그 문구를 잠깐 가린다).
    // **건수를 적지 않는다** — 셀 수 있는 것은 끌어다 놓은 묶음 수이지 파일 수가 아니라,
    // `1건`이 1만 개를 뜻할 수 있어 오해를 부른다 (F-7 2라운드 m1)
    if view.expanding > 0 {
        return (
            crate::i18n::status_expanding().to_owned(),
            theme::TEXT_MUTED,
        );
    }
    (format_current(view.queue), theme::HEADER_TEXT)
}

/// 상태 표시줄이 그릴 값 — 화면은 여기 담긴 것만 안다
pub struct StatusView<'a> {
    pub queue: &'a TransferQueue,
    /// 지금 펼치고 있는 폴더 수 (T22 Edge Case) — 0이면 표시하지 않는다.
    ///
    /// 큰 폴더를 끌어다 놓으면 큐가 채워질 때까지 화면에 아무 변화가 없어, 사용자가
    /// 아무 일도 안 일어난 줄 알고 다시 끈다 (F-7 리뷰 M1)
    pub expanding: usize,
    /// 방금 실패한 파일 작업의 사유 (FR-39·D22) — 있으면 "지금 옮기는 파일" 자리를 대신 쓴다.
    ///
    /// 실패는 잠깐이고 전송은 계속되므로 자리를 새로 만들지 않는다 — 이 줄에서 가장 급한
    /// 소식 하나만 보이면 된다
    pub notice: Option<&'a str>,
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
    // 큐 캐럿은 **도크가 열려 있으면** 아래를 가리킨다 — 로그를 보고 있어도 마찬가지다.
    // 원본이 그렇게 정했다(`:1034` `queueCaret: isQueue || isLog ? "▼" : "▲"`) — 이 캐럿은
    // "큐를 보고 있다"가 아니라 "아래 도크가 열려 있다"는 뜻이다
    let queue_caret = if dock.is_open() {
        CARET_OPEN
    } else {
        CARET_CLOSED
    };
    let toggle = format!("{queue_caret} {}", crate::i18n::status_queue());
    let width = text_width(ui, &toggle, &font);
    if toggle_text(
        ui,
        rect,
        left,
        width,
        (&toggle, "queue"),
        &font,
        theme::HEADER_TEXT,
    ) {
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

    // 실패 알약 — **실패가 있을 때만** (인벤토리 #57)
    let failures = view.queue.count(crate::remote::queue::QueueFilter::Error);
    if failures > 0 {
        let text = crate::i18n::dynamic::status_failed_count(failures);
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
    let (message, color) = status_message(view);
    if !message.is_empty() {
        let available = (right - left).max(0.0);
        let galley = crate::ui::list_common::elided_galley_colored(
            ui.painter(),
            message,
            font,
            available,
            color,
        );
        ui.painter().galley(
            egui::pos2(left, rect.center().y - galley.size().y / 2.0),
            galley,
            color,
        );
    }
    action
}

/// 누를 수 있는 글자 — 눌렸으면 `true`.
///
/// `key`는 **문구와 따로** 받는다 — 문구에는 캐럿(`▲`/`▼`)이 들어 있어 도크를 여닫는 순간
/// 바뀌는데, 그것을 id로 쓰면 누르는 사이에 id가 달라져 클릭이 씹힌다 (T19 리뷰가 같은 것을 짚었다)
fn toggle_text(
    ui: &mut egui::Ui,
    row: egui::Rect,
    left: f32,
    width: f32,
    (text, key): (&str, &'static str),
    font: &egui::FontId,
    color: egui::Color32,
) -> bool {
    let rect =
        egui::Rect::from_min_size(egui::pos2(left, row.top()), egui::vec2(width, row.height()));
    let response = ui
        .interact(
            rect,
            ui.id().with(("status_toggle", key)),
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
        let _guard =
            crate::i18n::LanguageGuard::lock(crate::app::settings::LanguageSetting::Korean);
        // 인벤토리 #53·#57
        assert_eq!(crate::i18n::status_queue(), "전송 큐");
        // 캐럿은 **아이콘 글꼴의 것**이어야 한다 — 원본 기호(U+25B2·U+25BC)를 그대로 쓰면
        // 이 앱 글꼴에 없어 두부가 된다 (프로젝트 규약)
        assert!(widgets::is_icon_font(CARET_OPEN) && widgets::is_icon_font(CARET_CLOSED));
        assert_ne!(CARET_OPEN, CARET_CLOSED, "여닫힘이 같은 글리프다");
        assert_eq!(crate::i18n::dynamic::status_failed_count(2), "실패 2건");
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
        // 방향 표시는 아이콘 글꼴의 것이다 — 글리프를 적어 두면 규약이 바뀔 때 여기가 먼저 깨진다
        assert_eq!(
            format_current(&queue),
            format!("{} app.bundle0.js — 62%", egui_phosphor::regular::ARROW_UP)
        );
    }

    #[test]
    fn 캐럿이_도크_상태를_따라간다() {
        // Acceptance ③ — 닫힘 `▲`, 열림 `▼`
        let ctx = egui::Context::default();
        // 캐럿은 "아래 도크가 열려 있다"는 뜻이다 — 어느 화면을 보고 있든 같다(원본 `:1034`)
        for (panel, queue_caret) in [
            (None, CARET_CLOSED),
            (Some(crate::ui::dock::DockPanel::Queue), CARET_OPEN),
            (Some(crate::ui::dock::DockPanel::Log), CARET_OPEN),
        ] {
            let dock = DockState {
                panel,
                ..DockState::default()
            };
            // 그리기 자체가 도는지와 함께, 캐럿 판정 규칙을 그대로 확인한다
            let queue = TransferQueue::new();
            let view = StatusView {
                queue: &queue,
                expanding: 0,
                notice: None,
            };
            let _ = ctx.run_ui(Default::default(), |ui| {
                egui::CentralPanel::default().show(ui, |ui| {
                    let rect =
                        egui::Rect::from_min_size(ui.max_rect().min, egui::vec2(1200.0, HEIGHT));
                    show_status_bar(ui, rect, &dock, &view);
                });
            });
            assert_eq!(
                if dock.is_open() {
                    CARET_OPEN
                } else {
                    CARET_CLOSED
                },
                queue_caret
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
                expanding: 0,
                notice: None,
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

    #[test]
    fn 실패_사유가_있으면_진행_문구_대신_그것이_보인다() {
        // Acceptance ④ — 파일 작업 실패 사유가 상태 줄에 남는다 (FR-39·D22)
        let queue = queue_with(&[TransferState::Active { sent: 50, speed: 0 }]);
        let plain = StatusView {
            queue: &queue,
            expanding: 0,
            notice: None,
        };
        let (text, color) = status_message(&plain);
        assert_eq!(color, theme::HEADER_TEXT);
        assert!(
            text.contains(egui_phosphor::regular::ARROW_UP)
                || text.contains(egui_phosphor::regular::ARROW_DOWN),
            "진행 문구: {text}"
        );

        let notice =
            "권한 바꾸기 실패 — 서버가 'SITE CHMOD'을(를) 지원하지 않습니다 — 500 Unknown command";
        let failed = StatusView {
            queue: &queue,
            expanding: 0,
            notice: Some(notice),
        };
        let (text, color) = status_message(&failed);
        assert_eq!(text, notice, "전송이 도는 중에도 실패 사유가 먼저다");
        assert_eq!(color, theme::ERROR, "실패인지 색으로도 구분돼야 한다");
    }

    #[test]
    fn 펼치는_중에는_그_사실을_먼저_알린다() {
        // F-7 리뷰 M1 (T22 Edge Case) — 큐가 채워지기 전까지 화면에 아무 표시가 없으면
        // 사용자는 아무 일도 안 일어난 줄 알고 다시 끈다
        let queue = TransferQueue::new();
        let view = StatusView {
            queue: &queue,
            expanding: 3,
            notice: None,
        };
        let (text, color) = status_message(&view);
        assert_eq!(text, "펼치는 중…", "건수는 묶음 수라 적지 않는다");
        assert_eq!(color, theme::TEXT_MUTED);

        // 다 펼치면 사라진다
        let done = StatusView {
            expanding: 0,
            ..view
        };
        assert_eq!(status_message(&done).0, "");

        // 실패 사유가 있으면 그것이 먼저다 — 급한 쪽이 앞선다
        let queue2 = TransferQueue::new();
        let failed = StatusView {
            queue: &queue2,
            expanding: 3,
            notice: Some("삭제 실패 — 550"),
        };
        assert_eq!(status_message(&failed).0, "삭제 실패 — 550");
    }
}
