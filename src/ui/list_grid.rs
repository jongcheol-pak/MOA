//! 자세히 보기를 뺀 나머지 보기 렌더 — 아이콘 4종·목록·타일·내용 (FR-23).
//!
//! 배치 계산은 `ui::view_mode`(순수 로직)가 하고 이 모듈은 그 자리에 그리기만 한다.
//! 선택·더블클릭·우클릭은 `ui::list_details`(자세히 보기)와 **같은 규칙**을 따른다 —
//! 보기 모드를 바꿨다고 조작법이 달라지면 안 되기 때문이다.
use crate::fs::enumerate::FileEntry;
use crate::fs::icons::{IconCache, IconSize};
use crate::panel::file_list::{format_filetime, format_size_kb};
use crate::ui::icon_tex::IconTextures;
use crate::ui::list_common::{FileListAction, elided_galley_rows};
use crate::ui::theme;
use crate::ui::view_mode::{GRID_NAME_ROWS, ViewMode, grid_metrics};
use eframe::egui;
use std::collections::BTreeSet;
use std::path::Path;

/// 아이콘과 이름 사이 간격 (세로 배치)
const ICON_TEXT_GAP: f32 = 4.0;
/// 셀 안쪽 좌우 여백 — 이름이 칸 가장자리에 닿지 않게 한다
const CELL_PAD_X: f32 = 4.0;
/// 셀 위쪽 여백
const CELL_PAD_TOP: f32 = 6.0;
/// 가로 배치(작은 아이콘·목록)에서 아이콘 왼쪽 여백
const ROW_ICON_X: f32 = 4.0;
/// 가로 배치에서 아이콘과 이름 사이 간격
const ROW_ICON_GAP: f32 = 4.0;
/// 타일·내용의 여러 줄 텍스트 사이 간격
const LINE_GAP: f32 = 2.0;
/// 내용 보기에서 오른쪽에 붙는 부가 정보(수정한 날짜·크기)가 차지할 폭.
/// 자세히 보기의 같은 두 열(각 150px, `list_details::Columns::DEFAULT`)보다 여유를 뒀다 —
/// 거기서는 열마다 따로 자르지만 여기서는 한 자리에 세로로 쌓기 때문이다
const CONTENT_META_WIDTH: f32 = 240.0;

/// 격자 렌더에 필요한 목록 상태 — 소유하지 않고 빌려 쓴다
pub struct GridInput<'a> {
    pub dir: &'a Path,
    pub entries: &'a [FileEntry],
    /// 보이는 항목에 도달했을 때 채우는 지연 캐시라 가변으로 받는다
    pub icon_indices: &'a mut Vec<Option<i32>>,
    pub selection: &'a BTreeSet<usize>,
    /// 종류 열 문자열 (entries와 같은 인덱스) — 타일 보기가 함께 보인다
    pub type_names: &'a [String],
    pub mode: ViewMode,
}

/// 이번 프레임에 일어난 조작 — 목록 상태 변경은 호출부가 한다
#[derive(Default)]
pub struct GridOutcome {
    pub action: FileListAction,
    pub select_request: Option<(usize, egui::Modifiers)>,
    pub clear_selection: bool,
}

/// 격자 보기를 그린다.
///
/// 가로로 채우는 모드는 세로로 길어지고, 세로로 채우는 모드(목록)는 가로로 길어진다 —
/// 어느 쪽이든 `ScrollArea::both()`면 필요한 축만 스크롤 막대가 생긴다
pub fn show(
    ui: &mut egui::Ui,
    input: GridInput<'_>,
    icons: &mut IconCache,
    textures: &mut IconTextures,
) -> GridOutcome {
    let mut outcome = GridOutcome::default();
    let ctx = ui.ctx().clone();
    let GridInput {
        dir,
        entries,
        icon_indices,
        selection,
        type_names,
        mode,
    } = input;
    let count = entries.len();
    // 모드가 요구하는 크기보다 작지 않은 이미지 리스트를 고른다 — 늘린 아이콘은 뭉개진다 (T9)
    let himl = icons.himl_for(IconSize::for_px(mode.icon_px()));

    let scroll = egui::ScrollArea::both().auto_shrink([false, false]);
    let output = scroll.show_viewport(ui, |ui, viewport| {
        let metrics = grid_metrics(mode, viewport.size(), count);
        let content = metrics.content_size();
        // 콘텐츠 전체를 한 번에 잡는다 — 스크롤 막대의 범위가 이 크기에서 나온다.
        // 항목이 뷰포트보다 적어도 뷰포트만큼은 잡아 빈 영역 클릭이 먹히게 한다
        let (_, area) = ui.allocate_space(egui::vec2(
            content.x.max(viewport.width()),
            content.y.max(viewport.height()),
        ));
        let origin = area.min.to_vec2();

        let font = egui::TextStyle::Body.resolve(ui.style());
        let hover_bg = ui.visuals().widgets.hovered.bg_fill;
        let sel_bg = ui.visuals().selection.bg_fill;
        for index in metrics.visible_range(viewport.top(), viewport.bottom(), count) {
            let cell = metrics.item_rect(index).translate(origin);
            let resp = ui.interact(cell, ui.id().with(("cell", index)), egui::Sense::click());
            if resp.clicked() {
                outcome.select_request = Some((index, ui.input(|i| i.modifiers)));
            }
            if resp.double_clicked() {
                outcome.action = FileListAction::Open(index);
            }
            if resp.secondary_clicked()
                && let Some(pos) = resp.interact_pointer_pos()
            {
                // 선택되지 않은 항목을 우클릭하면 그것을 단독 선택한 뒤 메뉴를 연다
                if !selection.contains(&index) {
                    outcome.select_request = Some((index, egui::Modifiers::NONE));
                }
                outcome.action = FileListAction::Context {
                    index: Some(index),
                    pos,
                };
            }
            if !ui.is_rect_visible(cell) {
                continue;
            }

            if selection.contains(&index) {
                ui.painter().rect_filled(cell, 2.0, sel_bg);
            } else if resp.hovered() {
                ui.painter().rect_filled(cell, 2.0, hover_bg);
            }

            let entry = &entries[index];
            let icon_index = resolve_icon(dir, entry, index, icon_indices, icons);
            let texture = textures.get(&ctx, himl, icon_index).map(|tex| tex.id());
            draw_cell(
                ui,
                cell,
                mode,
                entry,
                &type_names[index],
                texture,
                font.clone(),
            );
        }
        area
    });

    // 항목 밖 빈 영역 클릭 — 선택 해제, 우클릭이면 폴더 배경 메뉴 (자세히 보기와 같은 규칙)
    let resp = ui.interact(
        output.inner_rect,
        ui.id().with("grid_bg"),
        egui::Sense::click(),
    );
    if outcome.select_request.is_none() && outcome.action == FileListAction::None {
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

/// 보이는 항목에 한해 아이콘 인덱스를 조회한다 — 로드 시 전체를 미리 계산하면
/// exe가 많은 폴더에서 로드가 길어진다(PoC 실측 585ms → 84ms)
fn resolve_icon(
    dir: &Path,
    entry: &FileEntry,
    index: usize,
    icon_indices: &mut [Option<i32>],
    icons: &mut IconCache,
) -> i32 {
    match icon_indices[index] {
        Some(cached) => cached,
        None => {
            let full = dir.join(entry.name_string());
            let looked_up = icons.icon_index(
                &entry.extension(),
                entry.is_dir,
                Some(&full.to_string_lossy()),
            );
            icon_indices[index] = Some(looked_up);
            looked_up
        }
    }
}

/// 칸 하나를 그린다 — 아이콘과 이름의 배치는 흐름에 따라 갈린다.
///
/// 가로로 흐르는 큰 아이콘들은 **아이콘 위·이름 아래**(가운데 정렬)로 놓고,
/// 한 줄짜리 칸(작은 아이콘·목록)은 **아이콘 왼쪽·이름 오른쪽**으로 놓는다
fn draw_cell(
    ui: &mut egui::Ui,
    cell: egui::Rect,
    mode: ViewMode,
    entry: &FileEntry,
    type_name: &str,
    texture: Option<egui::TextureId>,
    font: egui::FontId,
) {
    let icon_px = mode.icon_px();
    let uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
    if matches!(mode, ViewMode::Tiles | ViewMode::Content) {
        draw_multiline_cell(ui, cell, mode, entry, type_name, texture, font);
        return;
    }
    if is_single_row(mode) {
        let icon_rect = egui::Rect::from_min_size(
            egui::pos2(cell.left() + ROW_ICON_X, cell.center().y - icon_px / 2.0),
            egui::Vec2::splat(icon_px),
        );
        if let Some(id) = texture {
            ui.painter().image(id, icon_rect, uv, egui::Color32::WHITE);
        }
        let text_left = icon_rect.right() + ROW_ICON_GAP;
        let galley = elided_galley_rows(
            ui.painter(),
            entry.name_string(),
            font,
            (cell.right() - CELL_PAD_X - text_left).max(0.0),
            1,
        );
        ui.painter().galley(
            egui::pos2(text_left, cell.center().y - galley.size().y / 2.0),
            galley,
            theme::TEXT,
        );
        return;
    }

    // 아이콘은 칸 위쪽 가운데
    let icon_rect = egui::Rect::from_center_size(
        egui::pos2(cell.center().x, cell.top() + CELL_PAD_TOP + icon_px / 2.0),
        egui::Vec2::splat(icon_px),
    );
    if let Some(id) = texture {
        ui.painter().image(id, icon_rect, uv, egui::Color32::WHITE);
    }
    // 이름은 아이콘 아래, 두 줄까지 (plan 시각 속성 표)
    let text_width = (cell.width() - CELL_PAD_X * 2.0).max(0.0);
    let galley = elided_galley_rows(
        ui.painter(),
        entry.name_string(),
        font,
        text_width,
        GRID_NAME_ROWS,
    );
    let text_x = cell.center().x - galley.size().x / 2.0;
    ui.painter().galley(
        egui::pos2(text_x, icon_rect.bottom() + ICON_TEXT_GAP),
        galley,
        theme::TEXT,
    );
}

/// 아이콘 오른쪽에 여러 줄을 놓는 칸 — 타일과 내용 보기.
///
/// 두 모드는 줄 구성이 다르다: 타일은 **이름·종류·크기**를 세로로 쌓고,
/// 내용은 **이름**을 왼쪽에 두고 **수정한 날짜·크기**를 오른쪽 끝에 붙인다.
/// 폴더는 크기 칸이 비는데(자세히 보기와 같은 규칙) 그 줄을 지우지 않고 비워 둬야
/// 항목마다 줄 위치가 흔들리지 않는다
fn draw_multiline_cell(
    ui: &mut egui::Ui,
    cell: egui::Rect,
    mode: ViewMode,
    entry: &FileEntry,
    type_name: &str,
    texture: Option<egui::TextureId>,
    font: egui::FontId,
) {
    let icon_px = mode.icon_px();
    let uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
    let icon_rect = egui::Rect::from_min_size(
        egui::pos2(cell.left() + ROW_ICON_X, cell.center().y - icon_px / 2.0),
        egui::Vec2::splat(icon_px),
    );
    if let Some(id) = texture {
        ui.painter().image(id, icon_rect, uv, egui::Color32::WHITE);
    }
    let text_left = icon_rect.right() + ROW_ICON_GAP;
    let size_text = if entry.is_dir {
        String::new()
    } else {
        format_size_kb(entry.size)
    };

    if mode == ViewMode::Content {
        // 이름은 왼쪽, 부가 정보는 오른쪽 끝 — 폭이 남는 모드라 한 줄에 나눠 담는다
        let meta_left = (cell.right() - CELL_PAD_X - CONTENT_META_WIDTH).max(text_left);
        let name = elided_galley_rows(
            ui.painter(),
            entry.name_string(),
            font.clone(),
            (meta_left - ROW_ICON_GAP - text_left).max(0.0),
            1,
        );
        ui.painter().galley(
            egui::pos2(text_left, cell.center().y - name.size().y / 2.0),
            name,
            theme::TEXT,
        );
        let meta = [format_filetime(entry.modified), size_text];
        draw_stacked(ui, &meta, meta_left, cell, font, CONTENT_META_WIDTH);
        return;
    }

    // 타일 — 이름·종류·크기 세 줄
    let lines = [entry.name_string(), type_name.to_owned(), size_text];
    let width = (cell.right() - CELL_PAD_X - text_left).max(0.0);
    draw_stacked(ui, &lines, text_left, cell, font, width);
}

/// 여러 줄을 칸 세로 가운데에 쌓아 그린다. 빈 줄도 자리를 차지해 줄 위치가 흔들리지 않는다
fn draw_stacked(
    ui: &mut egui::Ui,
    lines: &[String],
    left: f32,
    cell: egui::Rect,
    font: egui::FontId,
    width: f32,
) {
    let galleys: Vec<_> = lines
        .iter()
        .map(|text| elided_galley_rows(ui.painter(), text.clone(), font.clone(), width, 1))
        .collect();
    let line_height = galleys
        .iter()
        .map(|g| g.size().y)
        .fold(0.0_f32, f32::max)
        .max(1.0);
    let total = line_height * galleys.len() as f32 + LINE_GAP * (galleys.len() - 1) as f32;
    let mut y = cell.center().y - total / 2.0;
    for galley in galleys {
        // 빈 줄은 그리지 않지만 자리는 차지한다 (폴더의 크기 칸)
        if !galley.is_empty() {
            ui.painter()
                .galley(egui::pos2(left, y), galley, theme::TEXT);
        }
        y += line_height + LINE_GAP;
    }
}

/// 한 줄짜리 칸인가 — 아이콘과 이름이 가로로 나란히 놓이는 모드
fn is_single_row(mode: ViewMode) -> bool {
    matches!(mode, ViewMode::SmallIcons | ViewMode::List)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::view_mode::Flow;

    #[test]
    fn 한_줄_칸은_작은_아이콘과_목록뿐이다() {
        // 나머지는 아이콘 위·이름 아래로 놓인다 — 이 판정이 뒤집히면 큰 아이콘이
        // 좁은 한 줄에 눌려 들어간다
        assert!(is_single_row(ViewMode::SmallIcons));
        assert!(is_single_row(ViewMode::List));
        for mode in [
            ViewMode::ExtraLargeIcons,
            ViewMode::LargeIcons,
            ViewMode::MediumIcons,
            ViewMode::Tiles,
        ] {
            assert!(
                !is_single_row(mode),
                "{mode:?}가 한 줄 칸으로 잘못 분류됐다"
            );
        }
    }

    #[test]
    fn 세로_배치에서_아이콘과_이름이_칸을_넘지_않는다() {
        // 아이콘(위) + 간격 + 이름 두 줄이 칸 높이 안에 들어가야 아래 칸을 침범하지 않는다.
        // 글꼴 높이는 대략 14px로 잡는다 — 실제 값은 화면에서 확인한다(F-8)
        const APPROX_LINE: f32 = 14.0;
        for mode in [
            ViewMode::ExtraLargeIcons,
            ViewMode::LargeIcons,
            ViewMode::MediumIcons,
        ] {
            let cell = mode.cell_size();
            let needed =
                CELL_PAD_TOP + mode.icon_px() + ICON_TEXT_GAP + APPROX_LINE * GRID_NAME_ROWS as f32;
            assert!(
                needed <= cell.y,
                "{mode:?}: 아이콘+이름({needed}px)이 칸 높이({}px)를 넘는다",
                cell.y
            );
        }
    }

    #[test]
    fn 한_줄_칸에서_아이콘과_이름이_겹치지_않는다() {
        for mode in [ViewMode::SmallIcons, ViewMode::List] {
            let cell = mode.cell_size();
            let text_left = ROW_ICON_X + mode.icon_px() + ROW_ICON_GAP;
            assert!(
                text_left + CELL_PAD_X < cell.x,
                "{mode:?}: 이름이 칸({}px) 밖에서 시작한다",
                cell.x
            );
            assert!(mode.icon_px() <= cell.y, "{mode:?}: 아이콘이 칸보다 높다");
        }
    }

    #[test]
    fn 타일과_내용은_여러_줄_칸이다() {
        // 한 줄 칸으로 분류되면 종류·크기 줄이 사라진다
        for mode in [ViewMode::Tiles, ViewMode::Content] {
            assert!(!is_single_row(mode), "{mode:?}");
        }
    }

    #[test]
    fn 타일_칸에_세_줄이_들어간다() {
        // 이름·종류·크기 3줄 + 간격이 칸 높이(64px)를 넘으면 아래 칸을 침범한다
        const APPROX_LINE: f32 = 14.0;
        let cell = ViewMode::Tiles.cell_size();
        let needed = APPROX_LINE * 3.0 + LINE_GAP * 2.0;
        assert!(
            needed <= cell.y,
            "3줄({needed}px)이 칸({}px)을 넘는다",
            cell.y
        );
        assert!(ViewMode::Tiles.icon_px() <= cell.y, "아이콘이 칸보다 높다");
    }

    #[test]
    fn 내용_보기는_이름과_부가정보_자리가_겹치지_않는다() {
        // 부가 정보 폭이 칸을 다 먹으면 이름이 0폭이 된다
        let text_left = ROW_ICON_X + ViewMode::Content.icon_px() + ROW_ICON_GAP;
        let wide = 900.0_f32;
        let meta_left = (wide - CELL_PAD_X - CONTENT_META_WIDTH).max(text_left);
        assert!(
            meta_left > text_left + 100.0,
            "일반적인 폭에서 이름 자리가 지나치게 좁다"
        );
        // 아주 좁은 패널에서도 겹치지 않고 최소한 이름 시작점 이상이어야 한다
        let narrow = 200.0_f32;
        let narrow_meta = (narrow - CELL_PAD_X - CONTENT_META_WIDTH).max(text_left);
        assert!(narrow_meta >= text_left);
    }

    #[test]
    fn 가로_흐름_모드는_격자로_배치된다() {
        // 흐름과 셀 크기는 view_mode가 정하고 이 모듈은 그대로 쓴다 — 계약 확인
        for mode in [
            ViewMode::ExtraLargeIcons,
            ViewMode::LargeIcons,
            ViewMode::MediumIcons,
            ViewMode::SmallIcons,
        ] {
            assert_eq!(mode.flow(), Flow::Horizontal, "{mode:?}");
        }
    }
}
