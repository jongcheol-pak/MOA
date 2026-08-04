//! 자세히 보기 — 열 머리글·열 폭 조절·가로 스크롤·행 렌더 (FR-4).
//!
//! `ui::file_list`가 목록 상태(항목·선택·정렬)를 소유하고, 이 모듈은 **자세히 보기의 그리기와
//! 열 조작만** 담당한다. 상태를 바꾸지 않고 이번 프레임의 조작을 `DetailsOutcome`으로 돌려준다 —
//! 그리기 루프가 목록을 빌린 채로는 선택·정렬을 고칠 수 없기 때문이다.
use crate::fs::icons::IconCache;
use crate::panel::file_list::{ListRow, SortKey, format_filetime, format_size_kb};
use crate::ui::icon_tex::IconTextures;
use crate::ui::list_common::{FileListAction, elided_galley};
use crate::ui::theme;
use eframe::egui;
use std::collections::BTreeSet;
use std::path::Path;

/// 행 높이 — 16px 시스템 아이콘이 들어갈 여유를 둔다
pub const ROW_HEIGHT: f32 = 20.0;
/// 헤더 높이
pub const HEADER_HEIGHT: f32 = 22.0;
/// 아이콘 크기·좌측 여백
const ICON_SIZE: f32 = 16.0;
const ICON_X: f32 = 4.0;
/// 이름 텍스트 시작 x (아이콘 뒤)
const NAME_X: f32 = 24.0;
/// 셀 사이 여백
const CELL_PAD: f32 = 6.0;
/// 열 최소 폭 — 머리글이 잘려도 드래그 핸들은 잡히는 하한 (plan 시각 속성 표)
pub const MIN_COL_WIDTH: f32 = 40.0;
/// 열 경계 드래그 핸들 폭 — 경계 중심에서 좌우로 절반씩
const HANDLE_WIDTH: f32 = 6.0;

/// 자세히 보기의 열 — 이름·크기·종류·수정한 날짜 4개로 고정이다.
///
/// **네 열 모두 고정 폭을 갖는다.** 종전에는 마지막 열이 "남는 폭 전부"였는데, 그러면 콘텐츠
/// 폭이 뷰포트 폭에 의존해 가로 스크롤과 순환한다(스크롤 폭을 정하려면 콘텐츠 폭이 필요한데
/// 콘텐츠 폭이 다시 뷰포트에 의존). 합이 뷰포트보다 좁을 때만 `effective`가 마지막 열을
/// 늘려 빈틈을 없앤다 (plan D2)
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Columns {
    widths: [f32; Columns::COUNT],
}

impl Default for Columns {
    fn default() -> Columns {
        Columns::new()
    }
}

impl Columns {
    pub const COUNT: usize = 4;
    /// 기본 폭 (plan 시각 속성 표) — 앞 세 값은 종전 상수를 그대로 잇는다
    const DEFAULT: [f32; Columns::COUNT] = [320.0, 90.0, 150.0, 150.0];

    pub fn new() -> Columns {
        Columns {
            widths: Columns::DEFAULT,
        }
    }

    /// 저장된 폭으로 되살린다 (FR-11 세션 복원).
    /// 길이가 다르거나 유한하지 않은 값이 섞이면 그 값만 기본값으로 되돌린다 —
    /// 설정 파일이 손상돼도 목록이 못 그려지는 상태로 시작하지 않게 한다
    pub fn from_saved(saved: &[f32]) -> Columns {
        if saved.len() != Columns::COUNT {
            return Columns::new();
        }
        let mut widths = Columns::DEFAULT;
        for (slot, &value) in widths.iter_mut().zip(saved) {
            if value.is_finite() {
                *slot = value.max(MIN_COL_WIDTH);
            }
        }
        Columns { widths }
    }

    /// 세션에 저장할 폭
    pub fn to_saved(self) -> Vec<f32> {
        self.widths.to_vec()
    }

    /// 조절된 그대로의 폭 — 뷰포트를 반영하지 않은 값
    pub fn widths(&self) -> [f32; Columns::COUNT] {
        self.widths
    }

    /// 네 열의 폭 합 — 가로 스크롤 콘텐츠 폭의 근거
    pub fn content_width(&self) -> f32 {
        self.widths.iter().sum()
    }

    /// 실제로 그릴 폭. 합이 뷰포트보다 좁으면 **마지막 열만 늘려** 오른쪽 빈틈을 없앤다.
    /// 늘리는 것은 표시뿐이며 저장되는 폭(`widths`)은 그대로다 — 창 크기를 바꿀 때마다
    /// 사용자가 정한 폭이 덮어써지면 안 된다
    pub fn effective(&self, viewport_width: f32) -> [f32; Columns::COUNT] {
        let mut widths = self.widths;
        let slack = viewport_width - self.content_width();
        if slack > 0.0
            && let Some(last) = widths.last_mut()
        {
            *last += slack;
        }
        widths
    }

    /// 드래그로 `index` 열의 폭을 바꾼다. 최소 폭 아래로는 줄지 않는다
    pub fn apply_drag(&mut self, index: usize, delta: f32) {
        if let Some(width) = self.widths.get_mut(index) {
            *width = (*width + delta).max(MIN_COL_WIDTH);
        }
    }
}

/// 열 왼쪽 경계의 x 오프셋 (첫 열은 0). 헤더와 행이 같은 x를 쓰게 하는 계산의 정본
fn x_offsets(widths: &[f32; Columns::COUNT]) -> [f32; Columns::COUNT] {
    let mut offsets = [0.0; Columns::COUNT];
    let mut acc = 0.0;
    for (offset, width) in offsets.iter_mut().zip(widths) {
        *offset = acc;
        acc += width;
    }
    offsets
}

/// 자세히 보기가 그리는 데 필요한 목록 상태 — 소유하지 않고 빌려 쓴다
pub struct DetailsInput<'a, R: ListRow> {
    pub dir: &'a Path,
    pub entries: &'a [R],
    pub type_names: &'a [String],
    /// 보이는 행에 도달했을 때 채우는 지연 캐시라 가변으로 받는다
    pub icon_indices: &'a mut Vec<Option<i32>>,
    pub selection: &'a BTreeSet<usize>,
    pub sort_key: SortKey,
    pub ascending: bool,
    pub columns: &'a mut Columns,
    /// 항목이 로컬 파일인가. 원격이면 **전체 경로로 하는 일**(셸 아이콘 정밀 조회)을 하지 않는다 —
    /// 원격 이름을 로컬 경로에 이어 붙이면 있지도 않은 파일을 셸에 묻게 된다 (D11)
    pub local_paths: bool,
}

/// 이번 프레임에 일어난 조작 — 목록 상태 변경은 호출부가 한다
#[derive(Default)]
pub struct DetailsOutcome {
    pub action: FileListAction,
    /// 선택 요청 (행 인덱스, 수식 키)
    pub select_request: Option<(usize, egui::Modifiers)>,
    /// 머리글 클릭으로 요청된 정렬 기준
    pub sort_click: Option<SortKey>,
    /// 빈 영역 클릭 — 선택 해제
    pub clear_selection: bool,
}

/// 자세히 보기를 그린다.
///
/// 헤더와 행을 **하나의 `ScrollArea::both()` 안에** 넣는다 — 따로 두면 가로 스크롤 시
/// 머리글과 본문의 x가 어긋난다. 대신 세로 스크롤에서도 머리글이 함께 올라간다
/// (세로만 고정하려면 두 영역의 오프셋을 수동 동기화해야 해 이번 범위 밖 — plan Deferred)
pub fn show<R: ListRow>(
    ui: &mut egui::Ui,
    input: DetailsInput<'_, R>,
    icons: &mut IconCache,
    textures: &mut IconTextures,
) -> DetailsOutcome {
    let mut outcome = DetailsOutcome::default();
    let himl = icons.himl();
    let ctx = ui.ctx().clone();
    let row_count = input.entries.len();
    let DetailsInput {
        dir,
        entries,
        type_names,
        icon_indices,
        selection,
        sort_key,
        ascending,
        columns,
        local_paths,
    } = input;

    let scroll = egui::ScrollArea::both().auto_shrink([false, false]);
    let output = scroll.show_viewport(ui, |ui, viewport| {
        // 뷰포트 폭은 **스크롤 영역 안에서** 잰다 — 바깥의 `available_width`는 세로 스크롤 막대
        // 폭을 빼지 않아, 그만큼 마지막 열이 넓어져 늘 가로 스크롤이 생긴다
        let widths = columns.effective(viewport.width());
        let offsets = x_offsets(&widths);
        let content_width = widths.iter().sum::<f32>();
        let content_height = HEADER_HEIGHT + row_count as f32 * ROW_HEIGHT;
        // 콘텐츠 전체를 한 번에 잡는다 — 스크롤 막대의 범위가 이 크기에서 나온다
        let (_, content) =
            ui.allocate_space(egui::vec2(content_width, content_height.max(HEADER_HEIGHT)));
        let (left, top) = (content.left(), content.top());

        show_header(
            ui,
            columns,
            &widths,
            &offsets,
            left,
            top,
            sort_key,
            ascending,
            &mut outcome,
        );

        // 보이는 행만 그린다(가상 스크롤) — 헤더가 차지한 높이를 뺀 뒤 행 높이로 나눈다
        let first = ((viewport.top() - HEADER_HEIGHT) / ROW_HEIGHT)
            .floor()
            .max(0.0) as usize;
        let last =
            (((viewport.bottom() - HEADER_HEIGHT) / ROW_HEIGHT).ceil() as usize + 1).min(row_count);
        let font = egui::TextStyle::Body.resolve(ui.style());
        let stripe = ui.visuals().faint_bg_color;
        let hover_bg = ui.visuals().widgets.hovered.bg_fill;
        let sel_bg = ui.visuals().selection.bg_fill;
        for index in first..last {
            let row_top = top + HEADER_HEIGHT + index as f32 * ROW_HEIGHT;
            let rect = egui::Rect::from_min_size(
                egui::pos2(left, row_top),
                egui::vec2(content_width, ROW_HEIGHT),
            );
            let resp = ui.interact(rect, ui.id().with(("row", index)), egui::Sense::click());
            if resp.clicked() {
                outcome.select_request = Some((index, ui.input(|i| i.modifiers)));
            }
            if resp.double_clicked() {
                outcome.action = FileListAction::Open(index);
            }
            if resp.secondary_clicked()
                && let Some(pos) = resp.interact_pointer_pos()
            {
                // 선택되지 않은 행을 우클릭하면 그 행을 단독 선택한 뒤 메뉴를 연다
                if !selection.contains(&index) {
                    outcome.select_request = Some((index, egui::Modifiers::NONE));
                }
                outcome.action = FileListAction::Context {
                    index: Some(index),
                    pos,
                };
            }
            if !ui.is_rect_visible(rect) {
                continue;
            }

            let painter = ui.painter();
            if index % 2 == 1 {
                painter.rect_filled(rect, 0.0, stripe);
            }
            if selection.contains(&index) {
                painter.rect_filled(rect, 0.0, sel_bg);
            } else if resp.hovered() {
                painter.rect_filled(rect, 0.0, hover_bg);
            }

            let entry = &entries[index];
            let y = rect.center().y;
            // 보이는 행에 한해 아이콘 인덱스를 조회한다 — 로드 시 전체를 미리 계산하면
            // exe가 많은 폴더에서 로드가 길어진다(PoC 실측 585ms → 84ms)
            let icon_index = match icon_indices[index] {
                Some(cached) => cached,
                None => {
                    let full =
                        local_paths.then(|| dir.join(entry.name()).to_string_lossy().into_owned());
                    let looked_up =
                        icons.icon_index(&entry.extension(), entry.is_dir(), full.as_deref());
                    icon_indices[index] = Some(looked_up);
                    looked_up
                }
            };
            if let Some(tex) = textures.get(&ctx, himl, icon_index) {
                let icon_rect = egui::Rect::from_min_size(
                    egui::pos2(left + ICON_X, y - ICON_SIZE / 2.0),
                    egui::vec2(ICON_SIZE, ICON_SIZE),
                );
                // painter를 다시 얻는다 — textures가 ui를 빌리는 사이 앞의 painter가 무효화된다
                ui.painter().image(
                    tex.id(),
                    icon_rect,
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    egui::Color32::WHITE,
                );
            }

            let size_text = if entry.is_dir() {
                String::new()
            } else {
                format_size_kb(entry.size())
            };
            let cells = [
                (
                    entry.name(),
                    left + offsets[0] + NAME_X,
                    widths[0] - NAME_X - CELL_PAD,
                ),
                (
                    size_text,
                    left + offsets[1] + CELL_PAD,
                    widths[1] - CELL_PAD * 2.0,
                ),
                (
                    type_names[index].clone(),
                    left + offsets[2] + CELL_PAD,
                    widths[2] - CELL_PAD * 2.0,
                ),
                (
                    format_filetime(entry.modified_key()),
                    left + offsets[3] + CELL_PAD,
                    widths[3] - CELL_PAD * 2.0,
                ),
            ];
            let painter = ui.painter();
            for (text, x, width) in cells {
                if text.is_empty() || width <= 0.0 {
                    continue;
                }
                let galley = elided_galley(painter, text, font.clone(), width);
                painter.galley(
                    egui::pos2(x, y - galley.size().y / 2.0),
                    galley,
                    theme::TEXT,
                );
            }
        }
        content
    });

    // 목록 아래 빈 공간 우클릭 → 폴더 배경 메뉴.
    // 대상은 **콘텐츠가 끝난 지점부터** 뷰포트 바닥까지다 — 뷰포트 전체를 잡으면 이 위젯이
    // 행보다 나중에 등록돼 행 클릭을 통째로 가로챈다(egui 히트 테스트는 겹칠 때 나중에
    // 등록된 위젯을 위로 본다). 항목이 화면을 채우면 빈 공간이 없어 배경 메뉴도 열리지
    // 않는데, 이는 Windows 탐색기와 같은 동작이다
    let content_bottom = output.inner.bottom();
    if content_bottom < output.inner_rect.bottom() {
        let empty = egui::Rect::from_min_max(
            egui::pos2(output.inner_rect.left(), content_bottom),
            output.inner_rect.max,
        );
        let resp = ui.interact(empty, ui.id().with("list_bg"), egui::Sense::click());
        if resp.secondary_clicked()
            && let Some(pos) = resp.interact_pointer_pos()
        {
            outcome.action = FileListAction::Context { index: None, pos };
        }
        if resp.clicked() {
            outcome.clear_selection = true;
        }
    }
    outcome
}

/// 열 머리글 — 클릭으로 정렬, 경계 드래그로 폭 조절, 현재 정렬 열에 방향 표시
#[allow(clippy::too_many_arguments)]
fn show_header(
    ui: &mut egui::Ui,
    columns: &mut Columns,
    widths: &[f32; Columns::COUNT],
    offsets: &[f32; Columns::COUNT],
    left: f32,
    top: f32,
    sort_key: SortKey,
    ascending: bool,
    outcome: &mut DetailsOutcome,
) {
    let header_rect = egui::Rect::from_min_size(
        egui::pos2(left, top),
        egui::vec2(widths.iter().sum(), HEADER_HEIGHT),
    );
    ui.painter().rect_filled(header_rect, 0.0, theme::HEADER_BG);
    let labels = [
        (SortKey::Name, "이름"),
        (SortKey::Size, "크기"),
        (SortKey::Type, "종류"),
        (SortKey::Modified, "수정한 날짜"),
    ];
    let font = egui::TextStyle::Body.resolve(ui.style());

    for (index, (key, label)) in labels.into_iter().enumerate() {
        let width = widths[index];
        if width <= 0.0 {
            continue;
        }
        let x = left + offsets[index];
        let cell = egui::Rect::from_min_size(egui::pos2(x, top), egui::vec2(width, HEADER_HEIGHT));
        if ui
            .interact(cell, ui.id().with(("head", label)), egui::Sense::click())
            .clicked()
        {
            outcome.sort_click = Some(key);
        }
        let arrow = if sort_key == key {
            if ascending { " ▲" } else { " ▼" }
        } else {
            ""
        };
        let galley = elided_galley(
            ui.painter(),
            format!("{label}{arrow}"),
            font.clone(),
            width - CELL_PAD * 2.0,
        );
        ui.painter().galley(
            egui::pos2(x + CELL_PAD, cell.center().y - galley.size().y / 2.0),
            galley,
            theme::HEADER_TEXT,
        );
    }

    // 드래그 핸들은 머리글 셀보다 **나중에** 등록한다 — egui는 겹칠 때 나중 위젯을 위로 보므로
    // 경계 위에서 누른 것이 정렬 클릭으로 새지 않는다
    for index in 0..Columns::COUNT {
        let boundary = left + offsets[index] + widths[index];
        let handle = egui::Rect::from_min_size(
            egui::pos2(boundary - HANDLE_WIDTH / 2.0, top),
            egui::vec2(HANDLE_WIDTH, HEADER_HEIGHT),
        );
        // `click_and_drag`로 잡아 **클릭까지 이 핸들이 삼키게** 한다 — 드래그만 감지하면
        // 경계를 톡 눌렀을 때 아래 머리글 셀로 새어 의도치 않게 정렬이 바뀐다
        let resp = ui.interact(
            handle,
            ui.id().with(("col_handle", index)),
            egui::Sense::click_and_drag(),
        );
        if resp.hovered() || resp.dragged() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
        }
        if resp.dragged() {
            columns.apply_drag(index, resp.drag_delta().x);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 기본_폭은_네_열_모두_양수다() {
        let columns = Columns::new();
        for width in columns.widths() {
            assert!(width >= MIN_COL_WIDTH, "기본 폭이 최소 폭보다 작다");
        }
    }

    #[test]
    fn 드래그로_폭이_바뀐다() {
        let mut columns = Columns::new();
        let before = columns.widths()[0];
        columns.apply_drag(0, 50.0);
        assert_eq!(columns.widths()[0], before + 50.0);
    }

    #[test]
    fn 최소_폭_아래로는_줄지_않는다() {
        // 열이 0폭이 되면 다시 넓힐 핸들조차 잡을 수 없다
        let mut columns = Columns::new();
        columns.apply_drag(1, -10_000.0);
        assert_eq!(columns.widths()[1], MIN_COL_WIDTH);
    }

    #[test]
    fn 없는_열_인덱스는_무시된다() {
        let mut columns = Columns::new();
        let before = columns.widths();
        columns.apply_drag(Columns::COUNT, 100.0);
        assert_eq!(columns.widths(), before);
    }

    #[test]
    fn 콘텐츠_폭은_네_열의_합이다() {
        let columns = Columns::new();
        assert_eq!(
            columns.content_width(),
            columns.widths().iter().sum::<f32>()
        );
    }

    #[test]
    fn 뷰포트가_넓으면_마지막_열만_늘어난다() {
        // 오른쪽에 빈 띠가 남지 않게 한다. 늘어나는 것은 표시뿐이라 저장 폭은 그대로다
        let columns = Columns::new();
        let viewport = columns.content_width() + 200.0;
        let effective = columns.effective(viewport);
        assert_eq!(effective.iter().sum::<f32>(), viewport);
        assert_eq!(effective[..3], columns.widths()[..3]);
        assert_eq!(effective[3], columns.widths()[3] + 200.0);
        assert_eq!(
            columns.widths(),
            Columns::new().widths(),
            "저장 폭이 바뀌었다"
        );
    }

    #[test]
    fn 뷰포트가_좁으면_폭을_줄이지_않는다() {
        // 줄이면 가로 스크롤이 생길 이유가 사라진다 — 좁을 때는 스크롤로 본다
        let columns = Columns::new();
        let effective = columns.effective(100.0);
        assert_eq!(effective, columns.widths());
        assert!(columns.content_width() > 100.0);
    }

    #[test]
    fn 열_오프셋은_앞_열_폭의_누적이다() {
        let widths = [100.0, 50.0, 70.0, 30.0];
        assert_eq!(x_offsets(&widths), [0.0, 100.0, 150.0, 220.0]);
    }

    #[test]
    fn 저장된_폭을_되살린다() {
        let saved = vec![200.0, 60.0, 120.0, 90.0];
        assert_eq!(
            Columns::from_saved(&saved).widths(),
            [200.0, 60.0, 120.0, 90.0]
        );
    }

    #[test]
    fn 길이가_다른_저장값은_기본값으로_돌아간다() {
        // 열 구성이 바뀐 옛 세션 파일 — 일부만 적용하면 어느 열의 폭인지 알 수 없다
        assert_eq!(
            Columns::from_saved(&[1.0, 2.0]).widths(),
            Columns::new().widths()
        );
    }

    #[test]
    fn 손상된_저장값은_안전한_폭으로_보정된다() {
        let saved = vec![f32::NAN, -50.0, f32::INFINITY, 120.0];
        let columns = Columns::from_saved(&saved);
        for width in columns.widths() {
            assert!(
                width.is_finite() && width >= MIN_COL_WIDTH,
                "보정되지 않은 폭: {width}"
            );
        }
        assert_eq!(columns.widths()[3], 120.0, "정상 값까지 바뀌었다");
    }
}
