//! 창 아래에 붙는 도크 — 전송 큐와 서버 로그가 **같은 자리를 번갈아 쓴다** (FR-36·FR-40·D19).
//!
//! 원본 `FileExplorer-FTP.dc.html:262-271`(큐)·`:299-307`(로그). 두 화면의 셸(268px 높이·
//! 28px 탭 스트립·우측 아이콘 버튼)이 같아서 여기 한 벌만 둔다 — 각자 그리면 탭 줄이
//! 화면마다 조금씩 어긋난다.
//!
//! **본문은 여기서 그리지 않는다** — 큐 표는 `ui::queue_panel`, 로그는 `ui::log_panel`(T20)이
//! 그린다. 이 모듈은 자리(높이·탭·닫기)만 정한다.
use crate::remote::connection::ConnectionId;
use crate::remote::queue::{QueueFilter, TransferQueue};
use crate::remote::types::SiteId;
use crate::ui::theme;
use crate::ui::widgets;
use eframe::egui;

// ── 시각 토큰 (원본 `:262-270`) ──
/// 도크 높이 — 원본이 고정으로 정한 값
pub const DOCK_HEIGHT: f32 = 268.0;
/// 도크가 열려도 패널 그리드에 남겨 두는 최소 높이 (plan Edge Case: 창이 낮을 때)
const GRID_MIN_HEIGHT: f32 = 120.0;
/// 탭 스트립 — 호출부가 본문 자리를 계산할 때 함께 쓴다
pub const STRIP_HEIGHT: f32 = 28.0;
const STRIP_PAD_RIGHT: f32 = 6.0;
const TAB_PAD_X: f32 = 14.0;
const TAB_FONT_PX: f32 = 13.0;
/// 우측 아이콘 버튼
const ICON_SIZE: f32 = 26.0;
const ICON_FONT_PX: f32 = 13.0;
/// 닫기(`▼`)만 한 단계 크다 (`:270`)
const CLOSE_FONT_PX: f32 = 14.0;

// ── 문구 (인벤토리 #29~#34) ──
const TAB_QUEUE: &str = "전송 큐";
const TAB_LOG: &str = "서버 로그";
const TAB_DONE: &str = "성공";
const TAB_ERROR: &str = "실패";
/// 큐 패널 우측 버튼 (인벤토리 #33)
const ICON_PAUSE: &str = "⏸";
const ICON_CLEAR: &str = "✕";
const ICON_COLLAPSE: &str = "▼";
/// 로그 패널 우측 버튼 (인벤토리 #34)
const ICON_COPY: &str = "⧉";

/// 도크가 지금 보이는 화면 — **둘은 같은 자리를 배타로 쓴다** (D19)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DockPanel {
    Queue,
    Log,
}

/// 도크의 화면 상태 — 어떤 화면을, 어떤 필터·사이트로 보고 있는가.
///
/// 앱이 들고 프레임마다 넘긴다. 큐·로그 모듈이 각자 들면 탭을 옮길 때 두 상태가 어긋난다
#[derive(Debug, Clone, PartialEq)]
pub struct DockState {
    /// `None`이면 접혀 있다 (상태 표시줄의 캐럿이 다시 편다 — T21)
    pub panel: Option<DockPanel>,
    /// 큐 화면의 성공·실패 필터 (인벤토리 #31·#32)
    pub filter: QueueFilter,
    /// 연결별 탭에서 고른 사이트 — `None`이면 `전체` (인벤토리 #35)
    pub site: Option<SiteId>,
    /// 로그 화면에서 고른 연결 — `None`이면 **지금 보고 있는 연결**을 따라간다.
    ///
    /// 연결이 여럿이면 어느 서버의 로그인지 알 수 없다는 지적(T20 quality m1)을 이 탭이 닫는다
    pub log_conn: Option<ConnectionId>,
}

/// 세션 파일에 적히는 키 — 열거형 이름이 바뀌어도 저장 형식은 그대로여야 한다
const PANEL_QUEUE: &str = "queue";
const PANEL_LOG: &str = "log";
const FILTER_ALL: &str = "all";
const FILTER_DONE: &str = "done";
const FILTER_ERROR: &str = "error";

impl Default for DockState {
    fn default() -> DockState {
        DockState {
            panel: None,
            filter: QueueFilter::All,
            site: None,
            log_conn: None,
        }
    }
}

impl DockState {
    /// 세션에 담을 형태로 (FR-44) — **사이트 고르기는 담지 않는다**.
    /// 연결이 없는 채로 시작하므로 되살려도 가리킬 곳이 없다
    pub fn to_session(&self) -> crate::app::settings::DockSession {
        crate::app::settings::DockSession {
            panel: match self.panel {
                Some(DockPanel::Queue) => PANEL_QUEUE.to_owned(),
                Some(DockPanel::Log) => PANEL_LOG.to_owned(),
                None => String::new(),
            },
            filter: match self.filter {
                QueueFilter::All => FILTER_ALL,
                QueueFilter::Done => FILTER_DONE,
                QueueFilter::Error => FILTER_ERROR,
            }
            .to_owned(),
        }
    }

    /// 저장된 것에서 되살린다 — 모르는 키는 기본값(닫힘·전체)이다
    pub fn from_session(saved: &crate::app::settings::DockSession) -> DockState {
        DockState {
            panel: match saved.panel.as_str() {
                PANEL_QUEUE => Some(DockPanel::Queue),
                PANEL_LOG => Some(DockPanel::Log),
                _ => None,
            },
            filter: match saved.filter.as_str() {
                FILTER_DONE => QueueFilter::Done,
                FILTER_ERROR => QueueFilter::Error,
                _ => QueueFilter::All,
            },
            site: None,
            log_conn: None,
        }
    }

    pub fn is_open(&self) -> bool {
        self.panel.is_some()
    }

    /// 상태 표시줄의 캐럿이 누르는 토글 — 같은 화면을 다시 누르면 접힌다 (README §8)
    pub fn toggle(&mut self, panel: DockPanel) {
        self.panel = if self.panel == Some(panel) {
            None
        } else {
            Some(panel)
        };
    }
}

/// 탭 스트립에서 사용자가 고른 것
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DockAction {
    /// `⏸` — 전송을 멈추거나 다시 시작한다
    TogglePause,
    /// `✕` — 끝난 항목을 치운다
    ClearDone,
    /// `⧉` — 로그를 복사한다 (T20이 처리한다)
    CopyLog,
}

/// 도크가 실제로 차지한 높이 — 창이 낮으면 줄어든다 (plan Edge Case).
///
/// 패널 그리드에 최소 높이를 남기는 이유: 도크가 화면을 통째로 먹으면 파일을 고를 수 없어
/// 전송을 새로 걸 수도 없다
pub fn dock_height(available: f32) -> f32 {
    let room = available - GRID_MIN_HEIGHT;
    DOCK_HEIGHT.min(room.max(0.0))
}

/// 큐·로그가 함께 보는 값 — 탭 라벨의 건수와 연결별 탭이 여기서 나온다
pub struct DockView<'a> {
    pub queue: &'a TransferQueue,
    /// **연결이 실패한 상태로 남아 있는** 사이트들 — 연결별 탭의 점이 빨강이 되는 조건이다.
    ///
    /// "연결이 있는가"가 아니라 "실패했는가"를 받는 이유: 원본은 `phase === "error"`일 때만
    /// 빨강이고 그 밖(연결 중·연결됨·연결 없음)은 초록이다(`:728`). 연결 객체의 유무로
    /// 가르면 **실패한 사이트가 초록**으로, 정상 종료한 사이트가 빨강으로 뒤집힌다
    pub failed: &'a [SiteId],
    /// 지금 열려 있는 연결들 — 로그 화면의 탭이 된다. `(연결, 사이트 이름, 실패 여부)`
    pub connections: &'a [(ConnectionId, String, bool)],
}

/// 연결 탭의 상태 점 지름과 이름까지의 간격 — 사이드바 사이트 행과 같은 값이다
const CONN_DOT: f32 = 6.0;
const CONN_DOT_GAP: f32 = 8.0;

/// 탭 스트립을 그린다. 본문은 호출부가 남은 자리에 그린다.
///
/// 돌려주는 값은 **사용자가 누른 것**이며 상태 변경(탭 전환·접기)은 여기서 `state`에 바로 쓴다 —
/// 탭 전환까지 값으로 돌려주면 호출부가 그것을 다시 `state`에 옮겨 적어야 해 실수가 는다
pub fn show_strip(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    state: &mut DockState,
    view: &DockView<'_>,
) -> Option<DockAction> {
    ui.painter().rect_filled(rect, 0.0, theme::WINDOW_BG);
    let showing_log = state.panel == Some(DockPanel::Log);

    let counts = (
        view.queue.count(QueueFilter::All),
        view.queue.count(QueueFilter::Done),
        view.queue.count(QueueFilter::Error),
    );
    // 원본 `:1018-1022` — 큐 탭 셋은 필터이고 로그 탭만 다른 화면이다
    let tabs = [
        (
            format!("{TAB_QUEUE} ({})", counts.0),
            Some(QueueFilter::All),
            theme::TEXT,
        ),
        (TAB_LOG.to_owned(), None, theme::TEXT),
        (
            format!("{TAB_DONE} ({})", counts.1),
            Some(QueueFilter::Done),
            theme::OK_TEXT,
        ),
        (
            format!("{TAB_ERROR} ({})", counts.2),
            Some(QueueFilter::Error),
            theme::ERROR_TEXT,
        ),
    ];

    let mut left = rect.left();
    for (label, filter, active_color) in tabs {
        let text = ui.painter().layout_no_wrap(
            label,
            egui::FontId::proportional(TAB_FONT_PX),
            theme::TEXT_MUTED,
        );
        let width = text.size().x + TAB_PAD_X * 2.0;
        let tab = egui::Rect::from_min_size(
            egui::pos2(left, rect.top()),
            egui::vec2(width, STRIP_HEIGHT),
        );
        left += width;
        let active = match filter {
            Some(filter) => !showing_log && state.filter == filter,
            None => showing_log,
        };
        // id를 **라벨이 아니라 그 배후의 값**으로 잡는다 — 라벨에는 건수가 들어 있어
        // 누르는 사이에 전송이 끝나면 id가 바뀌어 클릭이 씹힌다 (T19 quality 리뷰 M2)
        let key = filter.map(|filter| filter as u8);
        let response = ui.interact(tab, ui.id().with(("dock_tab", key)), egui::Sense::click());
        if active {
            ui.painter().rect_filled(tab, 0.0, theme::SURFACE_BG);
        }
        let color = if active {
            active_color
        } else if response.hovered() {
            theme::TEXT
        } else {
            theme::TEXT_MUTED
        };
        ui.painter().galley(
            egui::pos2(tab.left() + TAB_PAD_X, tab.center().y - text.size().y / 2.0),
            text,
            color,
        );
        if response.clicked() {
            match filter {
                Some(filter) => {
                    state.panel = Some(DockPanel::Queue);
                    state.filter = filter;
                }
                None => state.panel = Some(DockPanel::Log),
            }
        }
    }

    // 로그 화면일 때만 **연결별 탭**을 잇는다 (사용자 요청 2026-08-05).
    // 큐 화면의 성공·실패 탭과 같은 모양이라 눈이 옮겨 다니지 않는다
    if showing_log {
        let active_conn = state
            .log_conn
            .filter(|id| view.connections.iter().any(|(conn, _, _)| conn == id));
        for (conn, name, failed) in view.connections {
            let text = ui.painter().layout_no_wrap(
                name.clone(),
                egui::FontId::proportional(TAB_FONT_PX),
                theme::TEXT_MUTED,
            );
            let width = text.size().x + TAB_PAD_X * 2.0 + CONN_DOT + CONN_DOT_GAP;
            let tab = egui::Rect::from_min_size(
                egui::pos2(left, rect.top()),
                egui::vec2(width, STRIP_HEIGHT),
            );
            left += width;
            // 고른 것이 없으면 **첫 연결**이 활성으로 보인다 — 화면이 그 연결의 로그를 보이므로
            let active = match active_conn {
                Some(active) => active == *conn,
                None => view.connections.first().map(|(id, _, _)| id) == Some(conn),
            };
            let response = ui.interact(
                tab,
                ui.id().with(("dock_log_conn", conn.0)),
                egui::Sense::click(),
            );
            if active {
                ui.painter().rect_filled(tab, 0.0, theme::SURFACE_BG);
            }
            let dot = if *failed { theme::ERROR } else { theme::OK_DOT };
            ui.painter().circle_filled(
                egui::pos2(tab.left() + TAB_PAD_X + CONN_DOT / 2.0, tab.center().y),
                CONN_DOT / 2.0,
                dot,
            );
            // 고른 탭과 손이 올라간 탭은 같은 밝기다 — 활성 여부는 배경이 말한다
            let color = if active || response.hovered() {
                theme::TEXT
            } else {
                theme::TEXT_MUTED
            };
            ui.painter().galley(
                egui::pos2(
                    tab.left() + TAB_PAD_X + CONN_DOT + CONN_DOT_GAP,
                    tab.center().y - text.size().y / 2.0,
                ),
                text,
                color,
            );
            if response.clicked() {
                state.panel = Some(DockPanel::Log);
                state.log_conn = Some(*conn);
            }
        }
    }

    // 우측 아이콘 — 화면에 따라 구성이 다르다 (인벤토리 #33·#34)
    let icons: &[(&str, f32, Option<DockAction>)] = if showing_log {
        &[
            (ICON_COPY, ICON_FONT_PX, Some(DockAction::CopyLog)),
            (ICON_COLLAPSE, CLOSE_FONT_PX, None),
        ]
    } else {
        &[
            (ICON_PAUSE, ICON_FONT_PX, Some(DockAction::TogglePause)),
            (ICON_CLEAR, ICON_FONT_PX, Some(DockAction::ClearDone)),
            (ICON_COLLAPSE, CLOSE_FONT_PX, None),
        ]
    };
    let mut action = None;
    let mut right = rect.right() - STRIP_PAD_RIGHT;
    // 오른쪽 끝부터 거꾸로 놓는다 — 원본의 순서(⏸ ✕ ▼)가 유지된다
    for (glyph, font_px, kind) in icons.iter().rev() {
        let icon = egui::Rect::from_min_size(
            egui::pos2(right - ICON_SIZE, rect.center().y - ICON_SIZE / 2.0),
            egui::vec2(ICON_SIZE, ICON_SIZE),
        );
        right -= ICON_SIZE;
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(icon));
        let clicked = widgets::icon_button_styled(
            &mut child,
            glyph,
            icon.size(),
            theme::MENU_HOT,
            theme::TEXT_MUTED,
            *font_px,
        )
        .clicked();
        if clicked {
            match kind {
                Some(kind) => action = Some(*kind),
                // `▼`는 접기다 — 값으로 돌려주지 않고 여기서 닫는다
                None => state.panel = None,
            }
        }
    }
    action
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 도크_치수는_원본과_같다() {
        // Acceptance ① — 268px·탭 스트립 28px
        assert_eq!(DOCK_HEIGHT, 268.0);
        assert_eq!(STRIP_HEIGHT, 28.0);
        assert_eq!(ICON_SIZE, 26.0);
        assert_eq!(TAB_PAD_X, 14.0);
    }

    #[test]
    fn 탭_문구는_인벤토리_원문_그대로다() {
        // 인벤토리 #29~#32 — 건수는 호출부가 붙인다
        assert_eq!(TAB_QUEUE, "전송 큐");
        assert_eq!(TAB_LOG, "서버 로그");
        assert_eq!(TAB_DONE, "성공");
        assert_eq!(TAB_ERROR, "실패");
        assert_eq!(ICON_PAUSE, "⏸");
        assert_eq!(ICON_CLEAR, "✕");
        assert_eq!(ICON_COLLAPSE, "▼");
        assert_eq!(ICON_COPY, "⧉");
    }

    #[test]
    fn 창이_낮으면_도크가_줄어든다() {
        // plan Edge Case — 도크가 화면을 통째로 먹으면 파일을 고를 수 없다
        assert_eq!(dock_height(1000.0), DOCK_HEIGHT);
        assert_eq!(dock_height(300.0), 180.0, "그리드 몫 120px를 남긴다");
        assert_eq!(
            dock_height(100.0),
            0.0,
            "남길 자리가 없으면 도크가 사라진다"
        );
    }

    #[test]
    fn 도크는_아래쪽_패널로_자리를_떼야_그리드가_남는다() {
        // T19 quality 리뷰 B1 — 사각형을 직접 잡아 `allocate_rect`로 떼면 위→아래 배치에서
        // 커서가 **화면 바닥 너머**로 밀려 뒤에 오는 그리드가 높이 0이 된다.
        // 앱이 기대는 것은 egui의 아래쪽 패널이 남은 자리를 줄여 준다는 성질이라, 그것을 고정한다
        let ctx = egui::Context::default();
        let mut before = 0.0;
        let mut after = 0.0;
        let height = 100.0;
        let _ = ctx.run_ui(Default::default(), |ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                before = ui.available_rect_before_wrap().height();
                egui::Panel::bottom(egui::Id::new("도크 자리 시험"))
                    .resizable(false)
                    .default_size(height)
                    .size_range(egui::Rangef::new(height, height))
                    .frame(egui::Frame::NONE)
                    .show(ui, |ui| {
                        ui.label("도크");
                    });
                after = ui.available_rect_before_wrap().height();
            });
        });
        assert!(before > 0.0, "시험 자체가 성립하지 않았다");
        assert!(
            after > 0.0,
            "도크를 뗀 뒤 그리드 자리가 사라졌다 — 파일 목록이 통째로 보이지 않게 된다"
        );
        assert!(
            (before - after - height).abs() < 1.0,
            "뗀 높이가 도크 높이와 다르다: {before} → {after}"
        );
    }

    #[test]
    fn 같은_화면을_다시_누르면_접힌다() {
        // README §8 — 상태 표시줄 캐럿의 토글
        let mut state = DockState::default();
        assert!(!state.is_open());
        state.toggle(DockPanel::Queue);
        assert_eq!(state.panel, Some(DockPanel::Queue));
        state.toggle(DockPanel::Log);
        assert_eq!(state.panel, Some(DockPanel::Log), "다른 화면이면 갈아탄다");
        state.toggle(DockPanel::Log);
        assert!(!state.is_open());
    }

    #[test]
    fn 로그_화면에만_연결_탭이_뜨고_고른_연결이_기억된다() {
        // 사용자 요청(2026-08-05): 로그가 어느 연결의 것인지 탭으로 고를 수 있어야 한다
        let queue = TransferQueue::new();
        let connections = vec![
            (ConnectionId(1), "배포 서버".to_owned(), false),
            (ConnectionId(2), "백업 서버".to_owned(), true),
        ];
        let view = DockView {
            queue: &queue,
            failed: &[],
            connections: &connections,
        };
        let ctx = egui::Context::default();

        // 큐 화면에서는 연결 탭을 그리지 않는다 — 큐는 사이트 탭이 따로 있다
        let mut queue_state = DockState {
            panel: Some(DockPanel::Queue),
            ..DockState::default()
        };
        let 큐_글자 = draw_strip(&ctx, &mut queue_state, &view);
        assert!(!큐_글자.contains(&"배포 서버".to_owned()), "{큐_글자:?}");

        // 로그 화면에서는 연결마다 탭이 선다
        let mut log_state = DockState {
            panel: Some(DockPanel::Log),
            ..DockState::default()
        };
        let 로그_글자 = draw_strip(&ctx, &mut log_state, &view);
        assert!(로그_글자.contains(&"배포 서버".to_owned()), "{로그_글자:?}");
        assert!(로그_글자.contains(&"백업 서버".to_owned()), "{로그_글자:?}");

        // 고른 연결은 상태에 남는다 — 화면은 그 연결의 로그를 보인다
        log_state.log_conn = Some(ConnectionId(2));
        let _ = draw_strip(&ctx, &mut log_state, &view);
        assert_eq!(log_state.log_conn, Some(ConnectionId(2)));
    }

    /// 스트립을 한 프레임 그리고 글자를 모은다
    fn draw_strip(ctx: &egui::Context, state: &mut DockState, view: &DockView<'_>) -> Vec<String> {
        let mut texts = Vec::new();
        let output = ctx.run_ui(Default::default(), |ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                let rect =
                    egui::Rect::from_min_size(ui.max_rect().min, egui::vec2(900.0, STRIP_HEIGHT));
                show_strip(ui, rect, state, view);
            });
        });
        for clipped in &output.shapes {
            if let egui::Shape::Text(text) = &clipped.shape {
                texts.push(text.galley.text().to_owned());
            }
        }
        texts
    }
}
