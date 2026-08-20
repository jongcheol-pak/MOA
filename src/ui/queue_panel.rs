//! 전송 큐 표 (FR-36) — 원본 `FileExplorer-FTP.dc.html:272-294`.
//!
//! 연결별 탭 한 줄 · 머리글 한 줄 · 항목 행들로 이뤄진다. 열 폭은 **일곱 열 모두 고정 픽셀**
//! (`34/280/300/120/84/118/150`)이고 합이 표 폭보다 좁으면 마지막 `상태` 열이 그 차이를
//! 흡수한다 — 원본은 `로컬 파일`이 `1fr`이었으나, 흡수 열이 앞자리면 그 오른쪽 경계를 끌어도
//! 흡수분이 같은 양을 반대로 먹어 폭 조절이 성립하지 않는다 (2026-08-18 plan D6).
//! 그래도 `ui::list_details`의 열 부품과 합치지는 않는다(plan 비추상화 선언) — 인덱싱 축이
//! 열거값 대 자리 번호로 다르고 사례가 둘뿐이다.
//!
//! **큐를 고치지 않는다** — 읽어서 그리고, 사용자가 고른 것은 값으로 돌려준다.
use crate::remote::connection::TransferId;
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
/// 빈 표 안내가 머리글 아래에서 떨어지는 거리
const EMPTY_HINT_TOP: f32 = 24.0;
/// 진행 막대 (`:290`)
const BAR_WIDTH: f32 = 110.0;
const BAR_HEIGHT: f32 = 6.0;
/// 열 기본 폭 — 원본 `34px 1fr 300px 120px 84px 118px 150px` (`:279`)에서 `1fr`(로컬 파일)만
/// 고정값으로 바꿨다. 흡수 열이 앞자리에 있으면 그 오른쪽 경계를 끌어도 흡수분이 같은 양을
/// 반대로 먹어 **잡은 경계가 제자리에 선다** — 폭 조절 자체가 성립하지 않는다 (plan D6).
/// 합은 1086px이라 기본 창 폭(1100px)에서 여유가 남는다
const COLUMNS: [f32; 7] = [34.0, 280.0, 300.0, 120.0, 84.0, 118.0, 150.0];
/// 열 경계 드래그 핸들 폭 — 경계 중심에서 좌우로 절반씩.
/// `list_details::HANDLE_WIDTH`는 private이라 같은 값을 여기 둔다
const HANDLE_WIDTH: f32 = 6.0;

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
// 상태 문구(인벤토리 #45~#47)와 행 우클릭 메뉴는 카탈로그에서 가져온다.
// 그 메뉴는 **디자인에 진입점이 없어 이 구현이 정한 문구**다 — 큐 항목을 하나씩
// 다시 걸거나 그만두는 길이 달리 없다(`⏸`·`✕`는 큐 전체를 다룬다)

/// 사용자가 큐에서 고른 조작.
///
/// `…All`은 **지금 보고 있는 목록**(상단 거르개 ∩ 연결별 탭)이 대상이다 — 대상 계산은
/// 화면이 아니라 앱이 한다(`ui::app::apply_queue_action`). 이 모듈은 큐를 고치지 않는다
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueueAction {
    /// 실패한 항목을 다시 대기로 되돌린다
    Retry(TransferId),
    /// 보이는 목록의 실패한 항목을 모두 다시 대기로
    RetryAll,
    /// 아직 끝나지 않은 항목을 그만둔다
    Cancel(TransferId),
    /// 끝난 항목을 목록에서 지운다
    Remove(TransferId),
    /// 보이는 목록을 통째로 지운다 (진행 중인 것도 멈추고 지운다)
    RemoveAll,
}

/// 행 우클릭 메뉴에 설 항목 — **탭이 아니라 행 상태가 정한다** (2026-08-18 사용자 결정).
///
/// `전송 취소`와 `삭제`는 동작이 같아(전송 중단 + 목록 제거 + `.part` 삭제) 나란히 두면
/// 무엇을 눌러야 할지 헷갈린다 — 상태별로 한 쪽만 보인다
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowMenuItem {
    Retry,
    RetryAll,
    Cancel,
    Remove,
    RemoveAll,
}

/// 이 행에 보일 메뉴 항목들 (순수 판정 — 그리기와 나눠 두어 시험할 수 있게 한다).
///
/// `has_error_in_view`는 **보이는 목록에 실패가 하나라도 있는가**다. 없으면
/// `전체 다시 시도`를 내지 않는다 — 눌러도 아무 일이 없는 메뉴는 두지 않는다
pub fn row_menu_items(state: &TransferState, has_error_in_view: bool) -> Vec<RowMenuItem> {
    let mut items = Vec::new();
    if state.is_error() {
        items.push(RowMenuItem::Retry);
    }
    if state.is_pending() {
        items.push(RowMenuItem::Cancel);
    }
    if has_error_in_view {
        items.push(RowMenuItem::RetryAll);
    }
    // 진행 중·대기는 `전송 취소`가 그 자리를 맡는다
    if !state.is_pending() {
        items.push(RowMenuItem::Remove);
    }
    items.push(RowMenuItem::RemoveAll);
    items
}

/// 얼룩 규칙 — 원본은 **거른 뒤의 자리 번호**(0부터)가 홀수인 행을 칠한다 (`:721`)
fn stripe(index: usize) -> bool {
    !index.is_multiple_of(2)
}

/// 큐의 크기 표기 — 표시 규칙은 파일 목록과 **한 벌**이고(`panel::file_list::format_size`),
/// 여기서는 큐에만 있는 판정 하나를 앞에 둔다.
///
/// 0은 "크기를 모른다"는 뜻이라 `—`다 — 목록에서는 같은 0이 "빈 파일"이라 `0.00 KB`로
/// 나가야 해서, 이 갈래를 코어 함수에 넣지 않는다 (plan D2)
pub fn format_size(bytes: u64) -> String {
    if bytes == 0 {
        return UNKNOWN.to_owned();
    }
    crate::panel::file_list::format_size(bytes)
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
        TransferState::Done => theme::OK_BAR,
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

/// 큐 표의 열 폭 — **일곱 열 모두 고정 폭**을 갖고, 합이 표 폭보다 좁을 때만
/// 마지막 열(`상태`)이 그 차이를 표시 폭으로 흡수한다 (plan D6 · `list_details::Columns`와 같은 규칙).
///
/// 흡수를 마지막 열에 둔 이유는 위 `COLUMNS` 주석에 있다. 넘칠 때는 저장 폭 그대로 그려
/// 오른쪽이 잘린다 — 가로 스크롤은 두지 않는다(사용자가 폭을 줄이면 돌아온다)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct QueueColumns {
    widths: [f32; 7],
}

impl Default for QueueColumns {
    fn default() -> QueueColumns {
        QueueColumns { widths: COLUMNS }
    }
}

/// 그 열이 줄 수 있는 하한 — 대개 `MIN_COL_WIDTH`(40px)지만, **기본 폭이 그보다 좁은 열**
/// (`방향` 34px)은 그 기본값이 하한이다.
///
/// 하한을 일괄로 40px에 맞추면 저장했다 되살릴 때 `방향`이 34 → 40으로 넓어져
/// **사용자가 맞춰 둔 화면이 그대로 돌아오지 않는다**(2026-08-18 시험이 잡았다)
fn min_column_width(slot: usize) -> f32 {
    let floor = crate::ui::list_details::MIN_COL_WIDTH;
    COLUMNS.get(slot).copied().unwrap_or(floor).min(floor)
}

impl QueueColumns {
    /// 저장된 폭으로 되살린다 (FR-11 세션 복원).
    ///
    /// **앞에서부터 있는 만큼만 받는다** — 열 수가 달라진 옛 세션이 와도 나머지는 기본값이다.
    /// 유한하지 않은 값은 그 자리만 되돌린다(설정 파일이 손상돼도 표를 못 그리지 않게)
    pub fn from_saved(saved: &[f32]) -> QueueColumns {
        let mut widths = COLUMNS;
        for (slot, (width, &value)) in widths.iter_mut().zip(saved).enumerate() {
            if value.is_finite() {
                *width = value.max(min_column_width(slot));
            }
        }
        QueueColumns { widths }
    }

    /// 세션에 저장할 폭
    pub fn to_saved(self) -> Vec<f32> {
        self.widths.to_vec()
    }

    /// 실제로 그릴 폭. 합이 표 폭보다 좁으면 **마지막 열만 늘려** 오른쪽 빈틈을 없앤다.
    /// 늘리는 것은 표시뿐이며 저장 폭은 그대로다 — 창 크기를 바꿀 때마다 사용자가 정한
    /// 폭이 덮어써지면 안 된다
    fn effective(self, total: f32) -> [f32; 7] {
        let mut widths = self.widths;
        let slack = total - widths.iter().sum::<f32>();
        if slack > 0.0
            && let Some(last) = widths.last_mut()
        {
            *last += slack;
        }
        widths
    }

    /// 경계 드래그 — 그 **왼쪽 열**의 폭을 바꾼다. 최소 폭 아래로는 줄지 않는다.
    ///
    /// 마지막 열의 오른쪽에는 핸들이 없어 `상태`의 저장 폭은 여기서 바뀌지 않는다
    fn apply_drag(&mut self, slot: usize, delta: f32) {
        let floor = min_column_width(slot);
        if let Some(width) = self.widths.get_mut(slot) {
            *width = (*width + delta).max(floor);
        }
    }
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
    show_site_tabs(ui, site_row, state, view, sites, true);

    let header = egui::Rect::from_min_size(
        egui::pos2(rect.left(), site_row.bottom()),
        egui::vec2(rect.width(), HEADER_HEIGHT),
    );
    let widths = state.columns.effective(rect.width());
    let guide_x = show_header(ui, header, &widths, &mut state.columns);

    let body = egui::Rect::from_min_max(
        egui::pos2(rect.left(), header.bottom()),
        egui::pos2(rect.right(), rect.bottom()),
    );
    // 가이드 선은 행을 다 그린 뒤에 긋는다 (`show_header` 주석) — 빈 목록 갈래에서도 그어야
    // 끌던 손이 허공에 뜨지 않으므로, 그리는 자리를 닫는 헬퍼로 둔다
    let draw_guide = |ui: &egui::Ui, bottom: f32| {
        if let Some(x) = guide_x {
            ui.painter().vline(
                x,
                header.top()..=bottom,
                egui::Stroke::new(1.0, theme::ACCENT),
            );
        }
    };
    let items = visible_items(view.queue, state.filter, state.site);
    // 보일 것이 없으면 그 사실을 적는다 (2026-08-16 검토) — 머리글만 남은 표는
    // 아직 아무것도 안 한 것인지 거른 결과가 없는 것인지 알려 주지 않는다
    if items.is_empty() {
        ui.painter().text(
            egui::pos2(body.center().x, body.top() + EMPTY_HINT_TOP),
            egui::Align2::CENTER_TOP,
            crate::i18n::queue_empty(),
            egui::FontId::proportional(FONT_PX),
            theme::TEXT_MUTED,
        );
        draw_guide(ui, body.bottom());
        return None;
    }
    // `전체 다시 시도`를 낼지 — 보이는 목록 전체를 본다(화면 밖 행도 대상이므로
    // 그려지는 범위가 아니라 거른 목록 전량에서 판정한다)
    let has_error_in_view = items.iter().any(|item| item.state.is_error());
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
                    && let Some(picked) =
                        show_row(ui, item, index, &widths, sites, has_error_in_view)
                {
                    action = Some(picked);
                }
            }
        });
    draw_guide(ui, body.bottom());
    action
}

/// 연결별 탭 한 줄 (인벤토리 #35·#36) — **큐와 로그가 함께 쓴다**(도크에 줄은 하나다).
///
/// `show_counts`가 꺼지면 이름만 적는다 — 로그 화면에는 셀 대상이 없어 `(N)`이 붙으면
/// 그 수가 무엇의 개수인지 알 수 없다 (2026-08-18 사용자 결정)
pub fn show_site_tabs(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    state: &mut DockState,
    view: &DockView<'_>,
    sites: &SiteStore,
    show_counts: bool,
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
    // **멤버십과 건수를 따로 센다** — 멤버십까지 거르면 그 거르개에 항목이 없는 서버가
    // 탭에서 사라져, `성공` 탭에서 실패만 있는 서버를 고를 수 없게 된다
    let members = view.queue.counts_by_site(QueueFilter::All);
    let counts = view.queue.counts_by_site(state.filter);
    let mut order: Vec<SiteId> = sites
        .sites()
        .iter()
        .map(|record| record.id)
        .filter(|id| members.contains_key(id) || view.connected.contains(id))
        .collect();
    // 저장소에 없는 사이트의 항목도 빠뜨리지 않는다(지운 사이트의 잔여 전송)
    let mut extra: Vec<SiteId> = members
        .keys()
        .copied()
        .chain(view.connected.iter().copied())
        .filter(|id| !order.contains(id))
        .collect();
    extra.dedup();
    extra.sort();
    order.append(&mut extra);

    let mut left = rect.left() + SITE_ROW_PAD_X;
    // 건수는 지금 고른 거르개를 따른다 — `전체` 탭도 큐 전량이 아니라 그 거르개의 수다
    let label_with_count = |name: &str, count: usize| {
        if show_counts {
            format!("{name} ({count})")
        } else {
            name.to_owned()
        }
    };
    let all_label = label_with_count(
        crate::i18n::queue_filter_all(),
        view.queue.count(state.filter),
    );
    let tabs: Vec<(Option<SiteId>, String)> = std::iter::once((None, all_label))
        .chain(order.into_iter().map(|id| {
            let name = sites
                .get(id)
                .map(|record| record.name.clone())
                .unwrap_or_else(|| crate::i18n::dynamic::queue_site_fallback(id.0));
            (
                Some(id),
                label_with_count(&name, *counts.get(&id).unwrap_or(&0)),
            )
        }))
        .collect();

    for (site, label) in tabs {
        // 색은 굽지 않고 그릴 때 정한다 — 구워 두면 아래 `galley`의 색이 무시된다
        // (도크 탭과 같은 함정. `list_common`의 주석 참고)
        let text = ui.painter().layout_no_wrap(
            label,
            egui::FontId::proportional(FONT_PX),
            egui::Color32::PLACEHOLDER,
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
            if active {
                theme::TEXT_SELECTED
            } else if response.hovered() {
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

/// 머리글 (인벤토리 #37~#43) — 열 경계에 구분선을 긋고 그 위에서 폭을 조절한다.
///
/// 돌려주는 값은 **지금 끌고 있는 경계의 x**다. 그 선은 여기서 긋지 않는다 —
/// 머리글이 본문 행보다 먼저 그려져 행 배경(`ROW_HOT`·얼룩)이 같은 레이어에서 덮어 버린다.
/// 호출부가 행을 다 그린 뒤에 긋는다
fn show_header(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    widths: &[f32; 7],
    columns: &mut QueueColumns,
) -> Option<f32> {
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

    // 평소에도 경계가 보여야 어디를 잡을지 알 수 있다 (2026-08-18 사용자 보고).
    // 마지막 열의 오른쪽 끝에는 긋지 않는다 — 그것은 표 바깥 경계다
    let mut boundary = rect.left();
    let mut dragging = None;
    for (slot, width) in widths.iter().take(widths.len() - 1).enumerate() {
        boundary += width;
        ui.painter().vline(
            boundary,
            rect.top()..=rect.bottom(),
            egui::Stroke::new(1.0, theme::BORDER_SUBTLE),
        );
        let handle = egui::Rect::from_min_size(
            egui::pos2(boundary - HANDLE_WIDTH / 2.0, rect.top()),
            egui::vec2(HANDLE_WIDTH, rect.height()),
        );
        let response = ui.interact(
            handle,
            ui.id().with(("queue_col_handle", slot)),
            egui::Sense::click_and_drag(),
        );
        if response.hovered() || response.dragged() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
        }
        if response.dragged() {
            columns.apply_drag(slot, response.drag_delta().x);
            dragging = Some(boundary);
        }
    }
    dragging
}

/// 항목 한 줄
fn show_row(
    ui: &mut egui::Ui,
    item: &TransferItem,
    index: usize,
    widths: &[f32; 7],
    sites: &SiteStore,
    has_error_in_view: bool,
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

    let (glyph, glyph_color) = widgets::direction_mark(item.direction);
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

    // 항목을 다시 걸거나 지우는 길 — 디자인에 진입점이 없어 우클릭으로 둔다
    let mut action = None;
    response.context_menu(|ui| {
        theme::menu_style(ui);
        for entry in row_menu_items(&item.state, has_error_in_view) {
            let (label, picked) = match entry {
                RowMenuItem::Retry => (crate::i18n::queue_retry(), QueueAction::Retry(item.id)),
                RowMenuItem::RetryAll => (crate::i18n::queue_retry_all(), QueueAction::RetryAll),
                RowMenuItem::Cancel => (crate::i18n::queue_cancel(), QueueAction::Cancel(item.id)),
                RowMenuItem::Remove => (crate::i18n::queue_remove(), QueueAction::Remove(item.id)),
                RowMenuItem::RemoveAll => (crate::i18n::queue_remove_all(), QueueAction::RemoveAll),
            };
            if ui.button(label).clicked() {
                action = Some(picked);
                ui.close();
            }
        }
    });
    action
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::remote::connection::TransferDirection;
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
        // 원본의 `1fr` 자리만 고정값이 됐다 — 합이 기본 창 폭(1100px)보다 좁아야
        // 흡수가 실제로 돈다 (plan D6)
        assert_eq!(COLUMNS[1], 280.0);
        assert_eq!(COLUMNS.iter().sum::<f32>(), 1086.0);
    }

    #[test]
    fn 남는_자리는_마지막_열이_갖는다() {
        // plan D6 — 흡수 열이 앞자리면 그 오른쪽 경계가 손을 따라오지 않아
        // 폭 조절 자체가 성립하지 않는다. 그래서 `상태`(마지막)가 잔여를 먹는다
        let columns = QueueColumns::default();
        let widths = columns.effective(1200.0);
        assert_eq!(widths.iter().sum::<f32>(), 1200.0);
        assert_eq!(widths[6], 150.0 + (1200.0 - 1086.0));
        assert_eq!(&widths[..6], &COLUMNS[..6], "앞 여섯 열은 그대로다");

        // 합이 표 폭을 넘으면 저장 폭 그대로 그리고 오른쪽이 잘린다(가로 스크롤 없음)
        let widths = columns.effective(800.0);
        assert_eq!(widths, COLUMNS);
    }

    #[test]
    fn 머리글_열_경계마다_구분선이_선다() {
        // 2026-08-18 사용자 보고 — 선이 없어 어디를 끌어야 할지 알 수 없었다.
        // 파일 목록(`list_details`)과 같은 규칙이라 같은 방식으로 잰다
        let widths = [34.0f32, 280.0, 300.0, 120.0, 84.0, 118.0, 150.0];
        let mut columns = QueueColumns::default();
        let ctx = egui::Context::default();
        let output = ctx.run_ui(Default::default(), |ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                let rect = egui::Rect::from_min_size(
                    egui::pos2(0.0, 0.0),
                    egui::vec2(widths.iter().sum(), HEADER_HEIGHT),
                );
                show_header(ui, rect, &widths, &mut columns);
            });
        });
        let mut 세로선 = Vec::new();
        for clipped in &output.shapes {
            if let egui::Shape::LineSegment { points, stroke } = &clipped.shape
                && (points[0].x - points[1].x).abs() < 0.01
                && stroke.color == theme::BORDER_SUBTLE
            {
                세로선.push(points[0].x);
            }
        }
        // 일곱 열이면 선은 여섯이다 — **마지막 열의 오른쪽 끝에는 긋지 않는다**
        let mut 기대 = Vec::new();
        let mut acc = 0.0;
        for width in &widths[..widths.len() - 1] {
            acc += width;
            기대.push(acc);
        }
        assert_eq!(세로선, 기대);

        // 끌고 있지 않으면 강조선(가이드)은 없다 — 그 선은 행을 다 그린 뒤 호출부가 긋는다
        assert!(
            !output.shapes.iter().any(|clipped| matches!(
                &clipped.shape,
                egui::Shape::LineSegment { stroke, .. } if stroke.color == theme::ACCENT
            )),
            "끌지 않았는데 가이드가 그려졌다"
        );
    }

    #[test]
    fn 경계_드래그는_왼쪽_열을_바꾸고_하한을_지킨다() {
        // plan D6 — 경계 k는 열 k−1의 폭을 바꾼다. 마지막 열은 핸들이 없어 여기 오지 않는다
        let mut columns = QueueColumns::default();
        columns.apply_drag(4, 30.0);
        assert_eq!(columns.effective(2000.0)[4], 84.0 + 30.0);

        // 최소 폭 아래로는 줄지 않는다
        columns.apply_drag(4, -1000.0);
        assert_eq!(
            columns.effective(2000.0)[4],
            crate::ui::list_details::MIN_COL_WIDTH
        );

        // **기본이 하한보다 좁은 열은 그 기본값이 하한**이다 — `방향`(34px)을 40px로 올리면
        // 저장했다 되살릴 때 화면이 넓어진다
        columns.apply_drag(0, -1000.0);
        assert_eq!(columns.effective(2000.0)[0], 34.0);
    }

    #[test]
    fn 열_폭이_세션을_왕복한다() {
        // FR-11 — 파일 목록 열 폭과 같은 관례
        let mut columns = QueueColumns::default();
        columns.apply_drag(1, 40.0);
        let back = QueueColumns::from_saved(&columns.to_saved());
        assert_eq!(back, columns);

        // 저장된 것이 없으면 기본값이다(옛 세션 파일)
        assert_eq!(QueueColumns::from_saved(&[]), QueueColumns::default());
        // 개수가 모자라면 앞에서부터 받고 나머지는 기본값
        let 부분 = QueueColumns::from_saved(&[50.0, 60.0]);
        assert_eq!(부분.to_saved()[..2], [50.0, 60.0]);
        assert_eq!(부분.to_saved()[2..], COLUMNS[2..]);
        // 유한하지 않은 값은 그 자리만 되돌리고, 하한 미만은 하한으로 올린다
        let 손상 = QueueColumns::from_saved(&[f32::NAN, 5.0]);
        assert_eq!(손상.to_saved()[0], COLUMNS[0]);
        assert_eq!(손상.to_saved()[1], crate::ui::list_details::MIN_COL_WIDTH);
        // `방향`은 기본 34px이 곧 하한이라 그 값으로 되살아난다(왕복이 깨지지 않는다)
        assert_eq!(QueueColumns::from_saved(&[34.0]).to_saved()[0], 34.0);
    }

    #[test]
    fn 머리글_문구는_인벤토리_원문_그대로다() {
        let _guard =
            crate::i18n::LanguageGuard::lock(crate::app::settings::LanguageSetting::Korean);
        // 인벤토리 #37~#43
        assert_eq!(
            headers(),
            [
                "방향",
                "로컬 파일",
                "원격 파일",
                "서버",
                "크기",
                "진행률",
                "상태"
            ]
        );
        assert_eq!(crate::i18n::queue_filter_all(), "전체");
        // 방향 표시는 **아이콘 글꼴**에서 온다 (프로젝트 규약 — 원본 화살표는 두부가 된다)
        assert!(crate::ui::widgets::is_icon_font(widgets::UPLOAD_GLYPH));
        assert!(crate::ui::widgets::is_icon_font(widgets::DOWNLOAD_GLYPH));
        assert_ne!(widgets::UPLOAD_GLYPH, widgets::DOWNLOAD_GLYPH);
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
        assert_eq!(text, "대기 중");
        assert_eq!(color, theme::TEXT_MUTED);

        let (text, color) = state_text(&TransferState::Active {
            sent: 10,
            speed: 13_002_342,
        });
        assert_eq!(text, "전송 중 · 12.4 MB/s");
        assert_eq!(color, theme::ACCENT);

        // 속도를 아직 못 쟀으면 군더더기를 붙이지 않는다
        let (text, _) = state_text(&TransferState::Active { sent: 0, speed: 0 });
        assert_eq!(text, "전송 중");

        let (text, color) = state_text(&TransferState::Done);
        assert_eq!(text, "완료");
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
        assert_eq!(bar_color(&TransferState::Done), theme::OK_BAR);
        assert_eq!(
            bar_color(&TransferState::Error {
                message: String::new()
            }),
            theme::ERROR
        );
    }

    #[test]
    fn 크기와_속도_표기가_원본_꼴이다() {
        // 크기는 파일 목록과 같은 규칙이다 — 소수 둘째자리 + KB·MB·GB (2026-08-18)
        assert_eq!(format_size(12 * 1024), "12.00 KB");
        assert_eq!(format_size(900 * 1024), "900.00 KB");
        // MB·GB로 올라간다 — KB에 고정하면 `1,887,437 KB` 같은 수가 나온다 (2026-08-16 검토)
        assert_eq!(format_size(1_884_160), "1.80 MB");
        assert_eq!(format_size(2 * 1024 * 1024 * 1024), "2.00 GB");
        assert_eq!(format_size(1), "0.01 KB", "1KB 미만도 한 칸은 채운다");
        // 큐에서만 0이 "모른다"다 — 파일 목록의 같은 0은 빈 파일이라 `0.00 KB`다
        assert_eq!(format_size(0), "—", "크기를 모르면 표기가 없다");
        assert_eq!(crate::panel::file_list::format_size(0), "0.00 KB");

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
                show_site_tabs(ui, rect, &mut state, &view, &sites, true);
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

    #[test]
    fn 행_메뉴는_탭이_아니라_행_상태가_정한다() {
        // 2026-08-18 사용자 결정(D5) — `전송 취소`와 `삭제`는 동작이 같아 한 쪽만 보인다
        use RowMenuItem::*;
        let 실패 = TransferState::Error {
            message: "550".to_owned(),
        };

        assert_eq!(
            row_menu_items(&실패, true),
            vec![Retry, RetryAll, Remove, RemoveAll]
        );
        assert_eq!(
            row_menu_items(&TransferState::Wait, true),
            vec![Cancel, RetryAll, RemoveAll],
            "진행 중·대기에는 `삭제`가 없다 — 그 자리는 `전송 취소`가 맡는다"
        );
        assert_eq!(
            row_menu_items(&TransferState::Active { sent: 1, speed: 1 }, true),
            vec![Cancel, RetryAll, RemoveAll]
        );
        assert_eq!(
            row_menu_items(&TransferState::Done, true),
            vec![RetryAll, Remove, RemoveAll]
        );

        // 보이는 목록에 실패가 없으면 `전체 다시 시도`를 내지 않는다
        assert_eq!(
            row_menu_items(&TransferState::Done, false),
            vec![Remove, RemoveAll]
        );
        assert_eq!(
            row_menu_items(&TransferState::Wait, false),
            vec![Cancel, RemoveAll]
        );
    }

    #[test]
    fn 행_메뉴_문구는_카탈로그를_거친다() {
        let _guard =
            crate::i18n::LanguageGuard::lock(crate::app::settings::LanguageSetting::Korean);
        // 사용자 요청 원문 그대로다 — `모두 …`가 아니라 `전체 …`
        assert_eq!(crate::i18n::queue_retry(), "다시 시도");
        assert_eq!(crate::i18n::queue_retry_all(), "전체 다시 시도");
        assert_eq!(crate::i18n::queue_cancel(), "전송 취소");
        assert_eq!(crate::i18n::queue_remove(), "삭제");
        assert_eq!(crate::i18n::queue_remove_all(), "전체 삭제");
    }

    #[test]
    fn 연결별_탭_건수가_거르개를_따르고_로그에서는_사라진다() {
        let _guard =
            crate::i18n::LanguageGuard::lock(crate::app::settings::LanguageSetting::Korean);
        // 사용자 보고(2026-08-18): `성공` 탭인데 아래 줄이 `전체 (1) · LG (1)`이었다
        let mut sites = SiteStore::new();
        let lg = sites.add("LG");
        let mut queue = TransferQueue::new();
        let 실패 = queue.enqueue(
            lg,
            TransferDirection::Upload,
            PathBuf::from(r"C:\a"),
            RemotePath::new("/a"),
            1,
        );
        queue.update(
            실패,
            TransferState::Error {
                message: "550".to_owned(),
            },
        );
        // 연결은 없다 — 그래도 큐에 항목이 있으니 탭 자리는 남아야 한다
        let view = DockView {
            queue: &queue,
            failed: &[],
            connected: &[],
        };

        let 그린다 = |filter: QueueFilter, show_counts: bool| {
            let mut state = DockState {
                filter,
                ..DockState::default()
            };
            let ctx = egui::Context::default();
            let mut texts = Vec::new();
            let output = ctx.run_ui(Default::default(), |ui| {
                egui::CentralPanel::default().show(ui, |ui| {
                    let rect = egui::Rect::from_min_size(
                        ui.max_rect().min,
                        egui::vec2(900.0, SITE_ROW_HEIGHT),
                    );
                    show_site_tabs(ui, rect, &mut state, &view, &sites, show_counts);
                });
            });
            for clipped in &output.shapes {
                if let egui::Shape::Text(text) = &clipped.shape {
                    texts.push(text.galley.text().to_owned());
                }
            }
            texts
        };

        let 실패_탭 = 그린다(QueueFilter::Error, true);
        assert!(실패_탭.contains(&"전체 (1)".to_owned()), "{실패_탭:?}");
        assert!(실패_탭.contains(&"LG (1)".to_owned()), "{실패_탭:?}");

        let 성공_탭 = 그린다(QueueFilter::Done, true);
        assert!(성공_탭.contains(&"전체 (0)".to_owned()), "{성공_탭:?}");
        assert!(
            성공_탭.contains(&"LG (0)".to_owned()),
            "성공 0건이어도 LG 탭은 남고 건수만 0이어야 한다: {성공_탭:?}"
        );

        // 로그 화면 — 셀 대상이 없어 건수를 적지 않는다
        let 로그 = 그린다(QueueFilter::All, false);
        assert!(로그.contains(&"전체".to_owned()), "{로그:?}");
        assert!(로그.contains(&"LG".to_owned()), "{로그:?}");
        assert!(
            !로그.iter().any(|text| text.contains('(')),
            "로그 화면에 건수가 남았다: {로그:?}"
        );
    }
}
