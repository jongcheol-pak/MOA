//! 자세히 보기를 뺀 나머지 보기 렌더 — 아이콘 4종·목록·타일·내용 (FR-23).
//!
//! 배치 계산은 `ui::view_mode`(순수 로직)가 하고 이 모듈은 그 자리에 그리기만 한다.
//! 선택·더블클릭·우클릭은 `ui::list_details`(자세히 보기)와 **같은 규칙**을 따른다 —
//! 보기 모드를 바꿨다고 조작법이 달라지면 안 되기 때문이다.
use crate::fs::icons::{IconCache, IconSize};
use crate::panel::file_list::{ListRow, format_filetime, format_size_kb};
use crate::ui::icon_tex::{IconTextures, ThumbnailTextures};
use crate::ui::list_common::{FileListAction, elided_galley_rows};
use crate::ui::theme;
use crate::ui::view_mode::{GRID_NAME_ROWS, ViewMode, grid_metrics};
use eframe::egui;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

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
pub struct GridInput<'a, R: ListRow> {
    pub dir: &'a Path,
    pub entries: &'a [R],
    /// 보이는 항목에 도달했을 때 채우는 지연 캐시라 가변으로 받는다
    pub icon_indices: &'a mut Vec<Option<i32>>,
    pub selection: &'a BTreeSet<usize>,
    /// 종류 열 문자열 (entries와 같은 인덱스) — 타일 보기가 함께 보인다
    pub type_names: &'a [String],
    pub mode: ViewMode,
    /// 준비된 썸네일 텍스처 (FR-24). 없으면 형식 아이콘으로 그린다 (plan D7)
    pub thumbnails: &'a ThumbnailTextures,
    /// 이번 프레임에 화면에 보인 파일들. 호출부가 이것으로 썸네일을 **요청하고 동시에
    /// 최근 사용으로 올린다** — 보이는 것만 담아야 큰 폴더에서 요청이 폭주하지 않고,
    /// 이미 준비된 것까지 담아야 보고 있는 썸네일이 축출되지 않는다
    pub visible: &'a mut Vec<PathBuf>,
    /// 항목이 로컬 파일인가. 원격이면 **전체 경로로 하는 일**(썸네일 요청·셸 아이콘 정밀 조회)을
    /// 하지 않는다 — 원격은 썸네일 비대상이고, 이름을 로컬 경로에 이어 붙이면 없는 파일을 묻게 된다 (D11)
    pub local_paths: bool,
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
pub fn show<R: ListRow>(
    ui: &mut egui::Ui,
    input: GridInput<'_, R>,
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
        thumbnails,
        visible,
        local_paths,
    } = input;
    let wants_thumbnails = local_paths && mode.uses_thumbnails();
    let count = entries.len();
    // 모드가 요구하는 크기보다 작지 않은 이미지 리스트를 고른다 — 늘린 아이콘은 뭉개진다 (T9)
    let himl = icons.himl_for(IconSize::for_px(mode.icon_px()));

    // 모드마다 스크롤 상태를 따로 둔다 — 셀 높이가 20px↔320px로 달라지는데 오프셋을
    // 공유하면 모드를 바꿨을 때 엉뚱한 위치가 보인다 (T8 Edge Case)
    let scroll = egui::ScrollArea::both()
        .id_salt(mode.as_key())
        .auto_shrink([false, false]);
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
            // **보이지 않는 칸은 위젯 등록도 하지 않는다** — 세로 흐름(목록)은 가로로 늘어나
            // `visible_range`가 전 범위를 돌려주므로, 여기서 거르지 않으면 10만 항목 폴더에서
            // 프레임마다 10만 번 `interact`가 돈다 (NFR-3). 보이지 않는 것은 클릭될 일도 없다
            if !ui.is_rect_visible(cell) {
                continue;
            }
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
            if selection.contains(&index) {
                ui.painter().rect_filled(cell, 2.0, sel_bg);
            } else if resp.hovered() {
                ui.painter().rect_filled(cell, 2.0, hover_bg);
            }

            let entry = &entries[index];
            // 썸네일은 파일만 — 폴더는 폴더 아이콘이 맞다
            let thumb = if wants_thumbnails && !entry.is_dir() {
                let path = dir.join(entry.name());
                let ready = thumbnails.get(&path).map(|tex| tex.id());
                // **텍스처가 이미 있어도 담는다** — 이 목록은 "요청 대상"이자 "지금 화면에
                // 보인다"는 신호다. 없을 때만 담으면 텍스처가 올라간 뒤로는 최근 사용
                // 갱신이 멈춰, 화면에 떠 있는 썸네일이 축출됐다 다시 만들어지길 반복한다
                visible.push(path);
                ready
            } else {
                None
            };
            // 준비된 썸네일이 있으면 그것을, 아니면 형식 아이콘을 그린다 (plan D7)
            let texture = match thumb {
                Some(id) => Some(id),
                None => {
                    let icon_index =
                        resolve_icon(dir, entry, index, icon_indices, icons, local_paths);
                    textures.get(&ctx, himl, icon_index).map(|tex| tex.id())
                }
            };
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
fn resolve_icon<R: ListRow>(
    dir: &Path,
    entry: &R,
    index: usize,
    icon_indices: &mut [Option<i32>],
    icons: &mut IconCache,
    local_paths: bool,
) -> i32 {
    match icon_indices[index] {
        Some(cached) => cached,
        None => {
            let full = local_paths.then(|| dir.join(entry.name()).to_string_lossy().into_owned());
            let looked_up = icons.icon_index(&entry.extension(), entry.is_dir(), full.as_deref());
            icon_indices[index] = Some(looked_up);
            looked_up
        }
    }
}

/// 칸 하나를 그린다 — 아이콘과 이름의 배치는 흐름에 따라 갈린다.
///
/// 가로로 흐르는 큰 아이콘들은 **아이콘 위·이름 아래**(가운데 정렬)로 놓고,
/// 한 줄짜리 칸(작은 아이콘·목록)은 **아이콘 왼쪽·이름 오른쪽**으로 놓는다
fn draw_cell<R: ListRow>(
    ui: &mut egui::Ui,
    cell: egui::Rect,
    mode: ViewMode,
    entry: &R,
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
            entry.name(),
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
    let galley = elided_galley_rows(ui.painter(), entry.name(), font, text_width, GRID_NAME_ROWS);
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
fn draw_multiline_cell<R: ListRow>(
    ui: &mut egui::Ui,
    cell: egui::Rect,
    mode: ViewMode,
    entry: &R,
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
    let size_text = if entry.is_dir() {
        String::new()
    } else {
        format_size_kb(entry.size())
    };

    if mode == ViewMode::Content {
        // 이름은 왼쪽, 부가 정보는 오른쪽 끝 — 폭이 남는 모드라 한 줄에 나눠 담는다
        let meta_left = (cell.right() - CELL_PAD_X - CONTENT_META_WIDTH).max(text_left);
        let name = elided_galley_rows(
            ui.painter(),
            entry.name(),
            font.clone(),
            (meta_left - ROW_ICON_GAP - text_left).max(0.0),
            1,
        );
        ui.painter().galley(
            egui::pos2(text_left, cell.center().y - name.size().y / 2.0),
            name,
            theme::TEXT,
        );
        let meta = [format_filetime(entry.modified_key()), size_text];
        draw_stacked(ui, &meta, meta_left, cell, font, CONTENT_META_WIDTH);
        return;
    }

    // 타일 — 이름·종류·크기 세 줄
    let lines = [entry.name(), type_name.to_owned(), size_text];
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

    use crate::fs::enumerate::FileEntry;
    use crate::fs::thumbnail::{ThumbnailCache, ThumbnailImage};
    use std::collections::BTreeSet;

    fn entry(name: &str, is_dir: bool) -> FileEntry {
        let mut wide: Vec<u16> = name.encode_utf16().collect();
        wide.push(0);
        FileEntry {
            name: wide,
            is_dir,
            size: 10,
            modified: 0,
        }
    }

    /// 격자를 한 프레임 그리고 이번 프레임에 "보인 파일"로 수집된 목록을 돌려준다.
    /// `preloaded`에 담긴 이름은 텍스처가 이미 올라간 상태로 만든다
    fn visible_after_draw(mode: ViewMode, names: &[&str], preloaded: &[&str]) -> Vec<PathBuf> {
        let dir = PathBuf::from(r"C:\테스트");
        let ctx = egui::Context::default();
        crate::ui::app::install_fonts(&ctx);

        // 텍스처를 미리 올려 둔다 — 픽셀 캐시에 넣고 sync로 승격시킨다
        let mut cache = ThumbnailCache::new();
        for name in preloaded {
            cache.accept_for_test(
                dir.join(name),
                Some(ThumbnailImage {
                    width: 2,
                    height: 2,
                    rgba: vec![255; 16],
                }),
            );
        }
        let mut textures = ThumbnailTextures::new();
        textures.sync(&ctx, &cache);
        assert_eq!(
            textures.len(),
            preloaded.len(),
            "사전 준비한 텍스처가 올라가지 않았다"
        );

        let entries: Vec<FileEntry> = names.iter().map(|name| entry(name, false)).collect();
        let mut icon_indices = vec![None; entries.len()];
        let type_names: Vec<String> = names.iter().map(|_| "파일".to_owned()).collect();
        let selection = BTreeSet::new();
        let mut visible = Vec::new();
        let mut icons = IconCache::new();
        let mut icon_textures = IconTextures::new();

        let _ = ctx.run_ui(Default::default(), |ui| {
            visible.clear();
            show(
                ui,
                GridInput {
                    dir: &dir,
                    entries: &entries,
                    icon_indices: &mut icon_indices,
                    selection: &selection,
                    type_names: &type_names,
                    mode,
                    thumbnails: &textures,
                    visible: &mut visible,
                    local_paths: true,
                },
                &mut icons,
                &mut icon_textures,
            );
        });
        visible
    }

    #[test]
    fn 텍스처가_이미_있어도_보이는_것으로_보고한다() {
        // 이 목록이 곧 "최근 사용" 갱신 대상이다 — 텍스처가 없을 때만 담으면, 텍스처가
        // 올라간 뒤로 갱신이 멈춰 **화면에 떠 있는 썸네일이 축출됐다 다시 만들어지길
        // 반복한다**(200장 넘는 폴더에서 재현). T14 quality 리뷰 M2
        let visible = visible_after_draw(
            ViewMode::MediumIcons,
            &["사진1.jpg", "사진2.jpg"],
            &["사진1.jpg"], // 첫 장은 텍스처가 이미 올라간 상태
        );
        assert!(
            visible.iter().any(|p| p.ends_with("사진1.jpg")),
            "텍스처가 있다는 이유로 보고에서 빠졌다 — LRU 갱신이 멈춘다: {visible:?}"
        );
        assert!(visible.iter().any(|p| p.ends_with("사진2.jpg")));
    }

    #[test]
    fn 자세히와_목록에서는_썸네일을_요청하지_않는다() {
        // 16px 자리에 미리보기는 알아볼 수 없고 디스크만 읽는다 (FR-24)
        let visible = visible_after_draw(ViewMode::List, &["사진1.jpg"], &[]);
        assert!(
            visible.is_empty(),
            "목록 보기가 썸네일을 요청했다: {visible:?}"
        );
    }

    #[test]
    fn 폴더는_썸네일을_요청하지_않는다() {
        // 폴더는 폴더 아이콘이 맞다 — 셸에 물어도 의미가 없다
        let dir = PathBuf::from(r"C:\테스트");
        let ctx = egui::Context::default();
        crate::ui::app::install_fonts(&ctx);
        let entries = vec![entry("문서", true), entry("사진.jpg", false)];
        let mut icon_indices = vec![None; entries.len()];
        let type_names = vec!["폴더".to_owned(), "파일".to_owned()];
        let selection = BTreeSet::new();
        let mut visible = Vec::new();
        let textures = ThumbnailTextures::new();
        let mut icons = IconCache::new();
        let mut icon_textures = IconTextures::new();

        let _ = ctx.run_ui(Default::default(), |ui| {
            visible.clear();
            show(
                ui,
                GridInput {
                    dir: &dir,
                    entries: &entries,
                    icon_indices: &mut icon_indices,
                    selection: &selection,
                    type_names: &type_names,
                    mode: ViewMode::MediumIcons,
                    thumbnails: &textures,
                    visible: &mut visible,
                    local_paths: true,
                },
                &mut icons,
                &mut icon_textures,
            );
        });
        assert_eq!(visible.len(), 1, "폴더까지 요청했다: {visible:?}");
        assert!(visible[0].ends_with("사진.jpg"));
    }

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

    #[test]
    fn 원격_항목도_격자로_그려진다() {
        // 렌더가 구체 타입을 모르게 됐는지 확인한다 — 원격 항목은 썸네일 비대상이라
        // 로컬 경로로 하는 일(썸네일 요청)을 하지 않아야 한다 (D11)
        use crate::remote::types::RemoteEntry;

        let ctx = egui::Context::default();
        crate::ui::app::install_fonts(&ctx);
        let rows = vec![
            RemoteEntry {
                name: "public_html".to_owned(),
                is_dir: true,
                is_symlink: false,
                link_target: None,
                size: 0,
                modified: Some(1_700_000_000),
                mode: Some(0o755),
                owner: Some("deploy".to_owned()),
            },
            RemoteEntry {
                name: "app.bundle.js".to_owned(),
                is_dir: false,
                is_symlink: false,
                link_target: None,
                size: 4096,
                modified: Some(1_700_000_100),
                mode: Some(0o644),
                owner: Some("deploy".to_owned()),
            },
        ];
        let mut icon_indices = vec![None; rows.len()];
        let type_names: Vec<String> = vec!["폴더".to_owned(), "JS 파일".to_owned()];
        let selection = BTreeSet::new();
        let mut visible = Vec::new();
        let mut icons = IconCache::new();
        let mut icon_textures = IconTextures::new();
        let textures = ThumbnailTextures::new();
        let dir = PathBuf::new();

        let _ = ctx.run_ui(Default::default(), |ui| {
            show(
                ui,
                GridInput {
                    dir: &dir,
                    entries: &rows,
                    icon_indices: &mut icon_indices,
                    selection: &selection,
                    type_names: &type_names,
                    mode: ViewMode::LargeIcons,
                    thumbnails: &textures,
                    visible: &mut visible,
                    local_paths: false,
                },
                &mut icons,
                &mut icon_textures,
            );
        });

        assert!(
            visible.is_empty(),
            "원격 항목은 썸네일을 요청하지 않아야 한다: {visible:?}"
        );
    }
}
