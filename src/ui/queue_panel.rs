//! 전송 큐 표 (FR-36) — 원본 `FileExplorer-FTP.dc.html:272-294`.
//!
//! 연결별 탭 한 줄 · 머리글 한 줄 · 항목 행들로 이뤄진다. 열 폭은 디자인이 픽셀로 못 박아
//! 두었으므로(`34px 1fr 300px 120px 84px 118px 150px`) 일반 표 부품으로 만들지 않는다
//! (plan 비추상화 선언) — 자세히 보기 표와 요구가 다르다.
//!
//! **큐를 고치지 않는다** — 읽어서 그리고, 사용자가 고른 것은 값으로 돌려준다.
use crate::remote::connection::{TransferDirection, TransferId};
use crate::remote::queue::{QueueFilter, TransferItem, TransferState, UNKNOWN};
use crate::remote::sites::SiteStore;
use crate::remote::types::SiteId;
use crate::ui::dock::{DockState, DockView};
use crate::ui::list_common::elided_galley_colored;
use crate::ui::theme;
use crate::ui::widgets;
use eframe::egui;

// ── 시각 토큰 (원본 `:272-292`) ──
/// 연결별 탭 행 (`:272`)
pub const SITE_ROW_HEIGHT: f32 = 28.0;
const SITE_ROW_PAD_X: f32 = 4.0;
const SITE_TAB_GAP: f32 = 2.0;
const SITE_TAB_PAD_X: f32 = 12.0;
const SITE_TAB_INNER_GAP: f32 = 7.0;
const SITE_DOT: f32 = 6.0;
/// 활성 탭 아래 강조 띠
const SITE_TAB_EDGE: f32 = 2.0;
/// 머리글·항목 행 (`:279`·`:283`)
const HEADER_HEIGHT: f32 = 22.0;
const ROW_HEIGHT: f32 = 24.0;
const CELL_PAD_X: f32 = 6.0;
const FONT_PX: f32 = 13.0;
/// 진행 막대 (`:290`)
const BAR_WIDTH: f32 = 110.0;
const BAR_HEIGHT: f32 = 6.0;
/// 열 폭 — `1fr`(로컬 파일)은 남는 자리를 갖는다 (`:279`)
const COLUMNS: [f32; 7] = [34.0, 0.0, 300.0, 120.0, 84.0, 118.0, 150.0];
/// 남는 자리를 갖는 열의 자리 번호
const FLEX_COLUMN: usize = 1;
/// 그 열이 아무리 좁아도 유지할 폭
const FLEX_MIN: f32 = 120.0;

// ── 문구 (인벤토리 #35~#48) ──
/// 머리글 (인벤토리 #37~#43) — 문구가 언어를 따르므로 상수가 아니라 그때그때 만든다
fn headers() -> [&'static str; 7] {
    [
        crate::i18n::queue_column_direction(),
        crate::i18n::queue_column_local(),
        crate::i18n::queue_column_remote(),
        crate::i18n::queue_column_server(),
        crate::i18n::queue_column_size(),
        crate::i18n::queue_column_progress(),
        crate::i18n::queue_column_state(),
    ]
}
/// 방향 글리프 (인벤토리 #44)
/// 전송 방향 표시 — 아이콘 글꼴에서 가져온다 (프로젝트 규약)
const UPLOAD_GLYPH: &str = egui_phosphor::regular::ARROW_UP;
const DOWNLOAD_GLYPH: &str = egui_phosphor::regular::ARROW_DOWN;
// 상태 문구(인벤토리 #45~#47)와 행 우클릭 메뉴는 카탈로그에서 가져온다.
// 그 메뉴는 **디자인에 진입점이 없어 이 구현이 정한 문구**다 — 큐 항목을 하나씩
// 다시 걸거나 그만두는 길이 달리 없다(`⏸`·`✕`는 큐 전체를 다룬다)

/// 사용자가 큐에서 고른 조작
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueueAction {
    /// 실패한 항목을 다시 대기로 되돌린다
    Retry(TransferId),
    /// 아직 끝나지 않은 항목을 그만둔다
    Cancel(TransferId),
}

/// 얼룩 규칙 — 원본은 **거른 뒤의 자리 번호**(0부터)가 홀수인 행을 칠한다 (`:721`)
fn stripe(index: usize) -> bool {
    !index.is_multiple_of(2)
}

/// 크기 표기 — 원본이 `1,840 KB` 꼴로 KB 단위에 자릿수 구분을 넣는다 (`:704`).
///
/// 0은 "모른다"는 뜻이라 `—`다 (plan Edge Case)
pub fn format_size(bytes: u64) -> String {
    if bytes == 0 {
        return UNKNOWN.to_owned();
    }
    // 1KB 미만도 1KB로 보인다 — 원본이 KB 아래를 쓰지 않는다
    let kb = bytes.div_ceil(1024);
    format!("{} KB", group_digits(kb))
}

/// 세 자리마다 쉼표 — `1840` → `1,840`
fn group_digits(value: u64) -> String {
    let digits = value.to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (index, ch) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            out.push(',');
        }
        out.push(ch);
    }
    out
}

/// 속도 표기 — 원본 `12.4 MB/s` (`:704`).
///
/// GB/s까지 올린다 — 로컬 네트워크·SSD 사이에서는 실제로 그 단위가 나오는데, MB/s에서 멈추면
/// `2048.0 MB/s` 같은 읽기 어려운 숫자가 된다 (plan T21 Edge Case)
pub fn format_speed(bytes_per_sec: u64) -> String {
    const GB: f64 = 1024.0 * 1024.0 * 1024.0;
    const MB: f64 = 1024.0 * 1024.0;
    const KB: f64 = 1024.0;
    let value = bytes_per_sec as f64;
    if value >= GB {
        format!("{:.1} GB/s", value / GB)
    } else if value >= MB {
        format!("{:.1} MB/s", value / MB)
    } else if value >= KB {
        format!("{:.1} KB/s", value / KB)
    } else {
        format!("{bytes_per_sec} B/s")
    }
}

/// 상태 열의 문구와 색 (인벤토리 #45~#48).
///
/// 실패는 **서버가 준 사유를 그대로** 보인다 — 우리가 다듬으면 사용자가 서버 관리자에게
/// 전할 말이 사라진다
pub fn state_text(state: &TransferState) -> (String, egui::Color32) {
    match state {
        TransferState::Wait => (
            crate::i18n::queue_state_pending().to_owned(),
            theme::TEXT_MUTED,
        ),
        TransferState::Active { speed, .. } => {
            let speed = if *speed > 0 {
                format!(" · {}", format_speed(*speed))
            } else {
                String::new()
            };
            (
                format!("{}{speed}", crate::i18n::queue_state_active()),
                theme::ACCENT,
            )
        }
        TransferState::Done => (crate::i18n::queue_state_done().to_owned(), theme::OK_TEXT),
        TransferState::Error { message } => (message.clone(), theme::ERROR),
    }
}

/// 진행 막대 채움 색 — 상태별로 갈린다 (`:701`)
fn bar_color(state: &TransferState) -> egui::Color32 {
    match state {
        TransferState::Error { .. } => theme::ERROR,
        TransferState::Done => theme::PRIMARY_FILL,
        _ => theme::ACCENT,
    }
}

/// 연결별 탭의 점 색 — **실패한 연결만 빨강**이고 그 밖(연결 중·연결됨·연결 없음)은 초록이다.
///
/// 원본 `:728`이 `phase === "error"`만 빨강으로 가른다. 연결 객체의 유무로 가르면 실패한
/// 사이트가 초록으로, 정상 종료한 사이트가 빨강으로 뒤집힌다
fn site_dot_color(failed: &[SiteId], site: SiteId) -> egui::Color32 {
    if failed.contains(&site) {
        theme::ERROR
    } else {
        theme::OK_DOT
    }
}

/// 방향 글리프와 색 (`:699`)
fn direction_mark(direction: TransferDirection) -> (&'static str, egui::Color32) {
    match direction {
        TransferDirection::Upload => (UPLOAD_GLYPH, theme::ACCENT),
        TransferDirection::Download => (DOWNLOAD_GLYPH, theme::OK_TEXT),
    }
}

/// 지금 화면에 보일 항목들 — 필터와 연결별 탭을 함께 적용한다.
///
/// 둘을 한자리에서 거르는 이유: 화면·건수·빈 상태 판정이 전부 같은 목록을 봐야 한다
pub fn visible_items(
    queue: &crate::remote::queue::TransferQueue,
    filter: QueueFilter,
    site: Option<SiteId>,
) -> Vec<&TransferItem> {
    queue
        .filter(filter)
        .into_iter()
        .filter(|item| site.is_none_or(|site| item.site == site))
        .collect()
}

/// 각 열이 차지할 폭 — `1fr` 열이 남는 자리를 갖는다
fn column_widths(total: f32) -> [f32; 7] {
    let fixed: f32 = COLUMNS
        .iter()
        .enumerate()
        .filter(|(index, _)| *index != FLEX_COLUMN)
        .map(|(_, width)| width)
        .sum();
    let mut widths = COLUMNS;
    widths[FLEX_COLUMN] = (total - fixed).max(FLEX_MIN);
    widths
}

/// 큐 표를 그린다 (인벤토리 #35~#48)
pub fn show_queue(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    state: &mut DockState,
    view: &DockView<'_>,
    sites: &SiteStore,
) -> Option<QueueAction> {
    let site_row = egui::Rect::from_min_size(rect.min, egui::vec2(rect.width(), SITE_ROW_HEIGHT));
    show_site_tabs(ui, site_row, state, view, sites);

    let header = egui::Rect::from_min_size(
        egui::pos2(rect.left(), site_row.bottom()),
        egui::vec2(rect.width(), HEADER_HEIGHT),
    );
    let widths = column_widths(rect.width());
    show_header(ui, header, &widths);

    let body = egui::Rect::from_min_max(
        egui::pos2(rect.left(), header.bottom()),
        egui::pos2(rect.right(), rect.bottom()),
    );
    let items = visible_items(view.queue, state.filter, state.site);
    let mut action = None;
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(body)
            .layout(egui::Layout::top_down(egui::Align::Min)),
    );
    child.set_clip_rect(body);
    // 1만 건에서도 보이는 만큼만 그린다 (plan Edge Case)
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show_rows(&mut child, ROW_HEIGHT, items.len(), |ui, range| {
            ui.spacing_mut().item_spacing.y = 0.0;
            for index in range {
                if let Some(item) = items.get(index)
                    && let Some(picked) = show_row(ui, item, index, &widths, sites)
                {
                    action = Some(picked);
                }
            }
        });
    action
}

/// 연결별 탭 한 줄 (인벤토리 #35·#36) — **큐와 로그가 함께 쓴다**(도크에 줄은 하나다)
pub fn show_site_tabs(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    state: &mut DockState,
    view: &DockView<'_>,
    sites: &SiteStore,
) {
    ui.painter().rect_filled(rect, 0.0, theme::SURFACE_BG);
    ui.painter().line_segment(
        [
            egui::pos2(rect.left(), rect.bottom() - 0.5),
            egui::pos2(rect.right(), rect.bottom() - 0.5),
        ],
        egui::Stroke::new(1.0, theme::BORDER_SUBTLE),
    );

    // `전체` 다음에 **큐에 항목이 있거나 지금 연결된** 사이트들이 온다.
    // 원본은 큐에서만 이름을 모으지만(`:722`), 그러면 연결만 하고 아직 아무것도 옮기지 않은
    // 서버가 탭에 없어 고를 수 없다 (2026-08-05 사용자 보고)
    let counts = view.queue.counts_by_site();
    let mut order: Vec<SiteId> = sites
        .sites()
        .iter()
        .map(|record| record.id)
        .filter(|id| counts.contains_key(id) || view.connected.contains(id))
        .collect();
    // 저장소에 없는 사이트의 항목도 빠뜨리지 않는다(지운 사이트의 잔여 전송)
    let mut extra: Vec<SiteId> = counts
        .keys()
        .copied()
        .chain(view.connected.iter().copied())
        .filter(|id| !order.contains(id))
        .collect();
    extra.dedup();
    extra.sort();
    order.append(&mut extra);

    let mut left = rect.left() + SITE_ROW_PAD_X;
    let all_label = format!("{} ({})", crate::i18n::queue_filter_all(), view.queue.len());
    let tabs: Vec<(Option<SiteId>, String)> = std::iter::once((None, all_label))
        .chain(order.into_iter().map(|id| {
            let name = sites
                .get(id)
                .map(|record| record.name.clone())
                .unwrap_or_else(|| crate::i18n::dynamic::queue_site_fallback(id.0));
            (
                Some(id),
                format!("{name} ({})", counts.get(&id).unwrap_or(&0)),
            )
        }))
        .collect();

    for (site, label) in tabs {
        let text = ui.painter().layout_no_wrap(
            label,
            egui::FontId::proportional(FONT_PX),
            theme::TEXT_MUTED,
        );
        // `전체`는 점이 없다 (인벤토리 #35) — 그만큼 폭도 줄어든다
        let dot_width = if site.is_some() {
            SITE_DOT + SITE_TAB_INNER_GAP
        } else {
            0.0
        };
        let width = SITE_TAB_PAD_X * 2.0 + dot_width + text.size().x;
        let tab = egui::Rect::from_min_size(
            egui::pos2(left, rect.top()),
            egui::vec2(width, SITE_ROW_HEIGHT),
        );
        left += width + SITE_TAB_GAP;
        let active = state.site == site;
        let response = ui.interact(
            tab,
            ui.id().with(("queue_site", site.map(|id| id.0))),
            egui::Sense::click(),
        );
        if active {
            ui.painter().rect_filled(tab, 0.0, theme::HEADER_BG);
            ui.painter().rect_filled(
                egui::Rect::from_min_size(
                    egui::pos2(tab.left(), tab.bottom() - SITE_TAB_EDGE),
                    egui::vec2(tab.width(), SITE_TAB_EDGE),
                ),
                0.0,
                theme::ACCENT,
            );
        }
        let mut text_left = tab.left() + SITE_TAB_PAD_X;
        if let Some(id) = site {
            ui.painter().circle_filled(
                egui::pos2(text_left + SITE_DOT / 2.0, tab.center().y),
                SITE_DOT / 2.0,
                site_dot_color(view.failed, id),
            );
            text_left += SITE_DOT + SITE_TAB_INNER_GAP;
        }
        ui.painter().galley(
            egui::pos2(text_left, tab.center().y - text.size().y / 2.0),
            text,
            if active || response.hovered() {
                theme::TEXT
            } else {
                theme::TEXT_MUTED
            },
        );
        if response.clicked() {
            state.site = site;
        }
    }
}

/// 머리글 (인벤토리 #37~#43)
fn show_header(ui: &mut egui::Ui, rect: egui::Rect, widths: &[f32; 7]) {
    ui.painter().rect_filled(rect, 0.0, theme::HEADER_BG);
    let mut left = rect.left();
    for (index, label) in headers().iter().enumerate() {
        ui.painter().text(
            egui::pos2(left + CELL_PAD_X, rect.center().y),
            egui::Align2::LEFT_CENTER,
            label,
            egui::FontId::proportional(FONT_PX),
            theme::HEADER_TEXT,
        );
        left += widths[index];
    }
}

/// 항목 한 줄
fn show_row(
    ui: &mut egui::Ui,
    item: &TransferItem,
    index: usize,
    widths: &[f32; 7],
    sites: &SiteStore,
) -> Option<QueueAction> {
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), ROW_HEIGHT),
        egui::Sense::click(),
    );
    if response.hovered() {
        ui.painter().rect_filled(rect, 0.0, theme::ROW_HOT);
    } else if stripe(index) {
        ui.painter().rect_filled(rect, 0.0, theme::HEADER_BG);
    }

    let mut left = rect.left();
    let mut cell = |width: f32| {
        let at = egui::Rect::from_min_size(
            egui::pos2(left + CELL_PAD_X, rect.top()),
            egui::vec2((width - CELL_PAD_X * 2.0).max(0.0), rect.height()),
        );
        left += width;
        at
    };

    let (glyph, glyph_color) = direction_mark(item.direction);
    let at = cell(widths[0]);
    ui.painter().text(
        egui::pos2(at.left(), at.center().y),
        egui::Align2::LEFT_CENTER,
        glyph,
        egui::FontId::proportional(FONT_PX),
        glyph_color,
    );

    // 경로 셋은 길면 끝을 줄인다 (plan Edge Case)
    for (width_index, text, color) in [
        (
            1usize,
            item.local.to_string_lossy().into_owned(),
            theme::HEADER_TEXT,
        ),
        (2, item.remote.as_str().to_owned(), theme::HEADER_TEXT),
        (
            3,
            sites
                .get(item.site)
                .map(|record| record.name.clone())
                .unwrap_or_else(|| crate::i18n::dynamic::queue_site_fallback(item.site.0)),
            theme::TEXT_MUTED,
        ),
    ] {
        let at = cell(widths[width_index]);
        // 색을 갤리에 구워 넣는다 — 그리면서 넘기는 색은 갤리가 기본색이면 무시된다(T20 리뷰)
        let galley = elided_galley_colored(
            ui.painter(),
            text,
            egui::FontId::proportional(FONT_PX),
            at.width(),
            color,
        );
        ui.painter().galley(
            egui::pos2(at.left(), at.center().y - galley.size().y / 2.0),
            galley,
            color,
        );
    }

    let at = cell(widths[4]);
    ui.painter().text(
        egui::pos2(at.left(), at.center().y),
        egui::Align2::LEFT_CENTER,
        format_size(item.size),
        egui::FontId::proportional(FONT_PX),
        theme::TEXT,
    );

    let at = cell(widths[5]);
    let mut bar_ui = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(egui::Rect::from_min_size(
                egui::pos2(at.left(), at.center().y - BAR_HEIGHT / 2.0),
                egui::vec2(BAR_WIDTH, BAR_HEIGHT),
            ))
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );
    widgets::progress_bar(
        &mut bar_ui,
        egui::vec2(BAR_WIDTH, BAR_HEIGHT),
        item.progress(),
        bar_color(&item.state),
    );

    let at = cell(widths[6]);
    let (text, color) = state_text(&item.state);
    let galley = elided_galley_colored(
        ui.painter(),
        text,
        egui::FontId::proportional(FONT_PX),
        at.width(),
        color,
    );
    ui.painter().galley(
        egui::pos2(at.left(), at.center().y - galley.size().y / 2.0),
        galley,
        color,
    );

    // 항목 하나를 다시 걸거나 그만두는 길 — 디자인에 진입점이 없어 우클릭으로 둔다
    let mut action = None;
    response.context_menu(|ui| {
        if item.state.is_error() && ui.button(crate::i18n::queue_retry()).clicked() {
            action = Some(QueueAction::Retry(item.id));
            ui.close();
        }
        if item.state.is_pending() && ui.button(crate::i18n::queue_cancel()).clicked() {
            action = Some(QueueAction::Cancel(item.id));
            ui.close();
        }
    });
    action
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::remote::queue::TransferQueue;
    use crate::remote::types::RemotePath;
    use std::path::PathBuf;

    #[test]
    fn 표_치수는_원본과_같다() {
        // Acceptance ①·② — 머리글 22px·행 24px·열 폭 `34/1fr/300/120/84/118/150`
        assert_eq!(SITE_ROW_HEIGHT, 28.0);
        assert_eq!(HEADER_HEIGHT, 22.0);
        assert_eq!(ROW_HEIGHT, 24.0);
        assert_eq!(BAR_WIDTH, 110.0);
        assert_eq!(BAR_HEIGHT, 6.0);
        assert_eq!(COLUMNS[0], 34.0);
        assert_eq!(&COLUMNS[2..], &[300.0, 120.0, 84.0, 118.0, 150.0]);
    }

    #[test]
    fn 남는_자리는_로컬_파일_열이_갖는다() {
        // Acceptance ② — `1fr`이 그 열이다
        let widths = column_widths(1200.0);
        let fixed: f32 = 34.0 + 300.0 + 120.0 + 84.0 + 118.0 + 150.0;
        assert_eq!(widths[FLEX_COLUMN], 1200.0 - fixed);
        assert_eq!(widths.iter().sum::<f32>(), 1200.0);
        // 창이 좁아도 최소 폭은 지킨다 — 0이 되면 경로가 통째로 사라진다
        assert_eq!(column_widths(200.0)[FLEX_COLUMN], FLEX_MIN);
    }

    #[test]
    fn 머리글_문구는_인벤토리_원문_그대로다() {
        let _guard =
            crate::i18n::LanguageGuard::lock(crate::app::settings::LanguageSetting::Korean);
        // 인벤토리 #37~#43
        assert_eq!(
            headers(),
            [
                crate::i18n::queue_column_direction(),
                crate::i18n::queue_column_local(),
                crate::i18n::queue_column_remote(),
                crate::i18n::queue_column_server(),
                crate::i18n::queue_column_size(),
                crate::i18n::queue_column_progress(),
                crate::i18n::queue_column_state()
            ]
        );
        assert_eq!(crate::i18n::queue_filter_all(), "전체");
        // 방향 표시는 **아이콘 글꼴**에서 온다 (프로젝트 규약 — 원본 화살표는 두부가 된다)
        assert!(crate::ui::widgets::is_icon_font(UPLOAD_GLYPH));
        assert!(crate::ui::widgets::is_icon_font(DOWNLOAD_GLYPH));
        assert_ne!(UPLOAD_GLYPH, DOWNLOAD_GLYPH);
        assert_eq!(crate::i18n::queue_state_pending(), "대기 중");
        assert_eq!(crate::i18n::queue_state_done(), "완료");
        assert_eq!(crate::i18n::queue_state_active(), "전송 중");
    }

    #[test]
    fn 얼룩은_거른_뒤의_홀수_자리다() {
        // Acceptance ⑥ — 원본은 거른 목록의 0부터 센 자리 번호가 홀수인 행을 칠한다 (`:721`)
        assert!(!stripe(0));
        assert!(stripe(1));
        assert!(!stripe(2));
        assert!(stripe(3));
    }

    #[test]
    fn 상태_문구와_색이_상태별로_갈린다() {
        let _guard =
            crate::i18n::LanguageGuard::lock(crate::app::settings::LanguageSetting::Korean);
        // Acceptance ⑦ (인벤토리 #45~#48)
        let (text, color) = state_text(&TransferState::Wait);
        assert_eq!(text, crate::i18n::queue_state_pending());
        assert_eq!(color, theme::TEXT_MUTED);

        let (text, color) = state_text(&TransferState::Active {
            sent: 10,
            speed: 13_002_342,
        });
        assert_eq!(text, "전송 중 · 12.4 MB/s");
        assert_eq!(color, theme::ACCENT);

        // 속도를 아직 못 쟀으면 군더더기를 붙이지 않는다
        let (text, _) = state_text(&TransferState::Active { sent: 0, speed: 0 });
        assert_eq!(text, crate::i18n::queue_state_active());

        let (text, color) = state_text(&TransferState::Done);
        assert_eq!(text, crate::i18n::queue_state_done());
        assert_eq!(color, theme::OK_TEXT);

        // 실패는 서버가 준 사유를 그대로 보인다
        let (text, color) = state_text(&TransferState::Error {
            message: "550 권한 거부".to_owned(),
        });
        assert_eq!(text, "550 권한 거부");
        assert_eq!(color, theme::ERROR);
    }

    #[test]
    fn 진행_막대_색이_상태별로_갈린다() {
        // Acceptance ⑦ (`:701`)
        assert_eq!(bar_color(&TransferState::Wait), theme::ACCENT);
        assert_eq!(bar_color(&TransferState::Done), theme::PRIMARY_FILL);
        assert_eq!(
            bar_color(&TransferState::Error {
                message: String::new()
            }),
            theme::ERROR
        );
    }

    #[test]
    fn 크기와_속도_표기가_원본_꼴이다() {
        // 원본 `1,840 KB`·`12.4 MB/s` (`:704`)
        assert_eq!(format_size(1_884_160), "1,840 KB");
        assert_eq!(format_size(12 * 1024), "12 KB");
        assert_eq!(format_size(1), "1 KB", "1KB 미만도 한 칸으로 보인다");
        assert_eq!(format_size(0), "—", "크기를 모르면 표기가 없다");
        assert_eq!(group_digits(1_234_567), "1,234,567");

        assert_eq!(format_speed(13_002_342), "12.4 MB/s");
        assert_eq!(format_speed(2048), "2.0 KB/s");
        assert_eq!(format_speed(512), "512 B/s");
        // 로컬 네트워크·SSD 사이에서는 GB/s가 실제로 나온다 (T21 Edge Case)
        assert_eq!(format_speed(2 * 1024 * 1024 * 1024), "2.0 GB/s");
    }

    fn queue_with_two_sites() -> (TransferQueue, SiteStore, SiteId, SiteId) {
        let mut sites = SiteStore::new();
        let first = sites.add("web-prod");
        let second = sites.add("cdn-assets");
        let mut queue = TransferQueue::new();
        for _ in 0..3 {
            queue.enqueue(
                first,
                TransferDirection::Upload,
                PathBuf::from(r"C:\a.js"),
                RemotePath::new("/a.js"),
                1024,
            );
        }
        let done = queue.enqueue(
            second,
            TransferDirection::Download,
            PathBuf::from(r"C:\b.log"),
            RemotePath::new("/b.log"),
            2048,
        );
        queue.update(done, TransferState::Done);
        (queue, sites, first, second)
    }

    #[test]
    fn 필터와_연결별_탭이_함께_걸린다() {
        // Acceptance ④·⑤ — 성공·실패 탭이 같은 집합을 거르고, 연결별 탭이 그 위에 얹힌다
        let (queue, _, first, second) = queue_with_two_sites();
        assert_eq!(visible_items(&queue, QueueFilter::All, None).len(), 4);
        assert_eq!(
            visible_items(&queue, QueueFilter::All, Some(first)).len(),
            3
        );
        assert_eq!(visible_items(&queue, QueueFilter::Done, None).len(), 1);
        assert_eq!(
            visible_items(&queue, QueueFilter::Done, Some(first)).len(),
            0,
            "그 사이트에는 끝난 것이 없다"
        );
        assert_eq!(
            visible_items(&queue, QueueFilter::Done, Some(second)).len(),
            1
        );
    }

    #[test]
    fn 연결별_탭의_점은_실패한_사이트만_빨강이다() {
        // spec 리뷰 M1 — 연결 객체의 유무로 가르면 **실패한 사이트가 초록**으로 뒤집힌다.
        // 원본은 `phase === "error"`일 때만 빨강이고 그 밖은 전부 초록이다 (`:728`)
        assert_eq!(site_dot_color(&[SiteId(1)], SiteId(1)), theme::ERROR);
        assert_eq!(site_dot_color(&[SiteId(1)], SiteId(2)), theme::OK_DOT);
        assert_eq!(site_dot_color(&[], SiteId(1)), theme::OK_DOT);
    }

    #[test]
    fn 큐가_비면_그릴_행이_없다() {
        // plan Edge Case — 머리글만 남는다
        let queue = TransferQueue::new();
        assert!(visible_items(&queue, QueueFilter::All, None).is_empty());
    }

    #[test]
    fn 표가_한_프레임을_그린다() {
        // 자리 계산이 뒤집힌 사각형·id 충돌 없이 도는지 본다 (Acceptance ⑧)
        let (queue, sites, first, _) = queue_with_two_sites();
        let ctx = egui::Context::default();
        let mut state = DockState {
            panel: Some(crate::ui::dock::DockPanel::Queue),
            ..DockState::default()
        };
        let view = DockView {
            connected: &[],
            queue: &queue,
            failed: &[first],
        };
        let _ = ctx.run_ui(Default::default(), |ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                let rect = egui::Rect::from_min_size(ui.max_rect().min, egui::vec2(1200.0, 200.0));
                show_queue(ui, rect, &mut state, &view, &sites);
            });
        });
        // 연결별 탭을 고르지 않았으면 `전체`가 그대로다
        assert_eq!(state.site, None);
    }

    #[test]
    fn 연결된_서버는_큐가_비어도_탭에_선다() {
        let _guard =
            crate::i18n::LanguageGuard::lock(crate::app::settings::LanguageSetting::Korean);
        // 사용자 보고(2026-08-05): 연결한 서버가 탭 줄에 없어 고를 수 없었다 —
        // 원본대로 **큐에 든 항목**에서만 이름을 모았기 때문이다
        let mut sites = SiteStore::new();
        let 연결된 = sites.add("web-prod");
        let 연결_없는 = sites.add("legacy");
        let queue = TransferQueue::new();
        let view = DockView {
            queue: &queue,
            failed: &[],
            connected: &[연결된],
        };
        let mut state = DockState::default();
        let ctx = egui::Context::default();
        let mut texts = Vec::new();
        let output = ctx.run_ui(Default::default(), |ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                let rect = egui::Rect::from_min_size(
                    ui.max_rect().min,
                    egui::vec2(900.0, SITE_ROW_HEIGHT),
                );
                show_site_tabs(ui, rect, &mut state, &view, &sites);
            });
        });
        for clipped in &output.shapes {
            if let egui::Shape::Text(text) = &clipped.shape {
                texts.push(text.galley.text().to_owned());
            }
        }
        assert!(
            texts.iter().any(|text| text.starts_with("web-prod")),
            "연결된 서버가 탭에 없다: {texts:?}"
        );
        assert!(
            texts
                .iter()
                .any(|text| text.starts_with(crate::i18n::queue_filter_all())),
            "`전체` 탭이 없다: {texts:?}"
        );
        // 연결도 없고 큐에도 없는 사이트는 서지 않는다 — 목록이 등록한 사이트 전부로 늘어나면
        // 지금 무엇에 붙어 있는지가 오히려 안 보인다
        let _ = 연결_없는;
        assert!(
            !texts.iter().any(|text| text.starts_with("legacy")),
            "연결도 전송도 없는 사이트가 탭에 섰다: {texts:?}"
        );
    }
}
