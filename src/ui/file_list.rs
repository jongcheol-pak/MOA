//! 파일 목록 위젯 — 가상 스크롤·열 정렬·다중 선택·시스템 아이콘 (FR-4·FR-5, NFR-3).
//!
//! 표시 규칙(크기·날짜 문자열)과 정렬 비교는 `panel::file_list`의 순수 함수를 그대로 쓴다 —
//! 복제하면 두 벌로 갈라진다. 이 파일은 그리기와 입력만 담당한다.
use crate::fs::enumerate::FileEntry;
use crate::fs::icons::IconCache;
use crate::panel::file_list::{SortKey, compare_entries, format_filetime, format_size_kb};
use crate::ui::icon_tex::IconTextures;
use crate::ui::theme;
use eframe::egui;
use std::collections::BTreeSet;
use std::path::PathBuf;

/// 행 높이 — 16px 시스템 아이콘이 들어갈 여유를 둔다
const ROW_HEIGHT: f32 = 20.0;
/// 헤더 높이
const HEADER_HEIGHT: f32 = 22.0;
/// 열 폭 (헤더와 행이 같은 x를 쓰도록 공유). 마지막 열은 남는 폭 전부
const COL_NAME_W: f32 = 320.0;
const COL_SIZE_W: f32 = 90.0;
const COL_TYPE_W: f32 = 150.0;
/// 아이콘 크기·좌측 여백
const ICON_SIZE: f32 = 16.0;
const ICON_X: f32 = 4.0;
/// 이름 텍스트 시작 x (아이콘 뒤)
const NAME_X: f32 = 24.0;
/// 셀 사이 여백
const CELL_PAD: f32 = 6.0;

/// 목록이 상위(패널)에 돌려주는 사용자 조작.
/// 즉시 모드라 콜백을 등록하지 않고 이번 프레임의 조작을 값으로 반환한다
#[derive(Clone, PartialEq, Debug)]
pub enum FileListAction {
    None,
    /// 항목 실행 — 폴더면 진입, 파일이면 연결 프로그램 (호출부가 판정)
    Open(usize),
    /// 컨텍스트 메뉴 요청 — `index`가 `None`이면 빈 영역(폴더 배경 메뉴)
    Context {
        index: Option<usize>,
        pos: egui::Pos2,
    },
}

/// 파일 목록 뷰 — 항목·종류 문자열·아이콘 인덱스·선택 상태를 함께 소유한다.
/// 셋은 인덱스로 짝지어지므로 따로 두면 정렬 시 어긋난다
pub struct FileListView {
    dir: PathBuf,
    entries: Vec<FileEntry>,
    /// 종류 열 문자열 (entries와 같은 인덱스) — 표시·정렬에 공용
    type_names: Vec<String>,
    /// 아이콘 인덱스 (entries와 같은 인덱스). 보이는 행에 도달했을 때 채운다(지연 조회)
    icon_indices: Vec<Option<i32>>,
    sort_key: SortKey,
    ascending: bool,
    selection: BTreeSet<usize>,
    /// Shift 범위 선택의 기준점
    anchor: Option<usize>,
}

impl Default for FileListView {
    fn default() -> FileListView {
        FileListView::new()
    }
}

impl FileListView {
    pub fn new() -> FileListView {
        FileListView {
            dir: PathBuf::new(),
            entries: Vec::new(),
            type_names: Vec::new(),
            icon_indices: Vec::new(),
            sort_key: SortKey::Name,
            ascending: true,
            selection: BTreeSet::new(),
            anchor: None,
        }
    }

    /// 새 폴더의 항목으로 교체한다. 정렬은 현재 정렬 기준을 유지한다
    pub fn set_entries(&mut self, dir: PathBuf, entries: Vec<FileEntry>, icons: &mut IconCache) {
        self.dir = dir;
        self.type_names = entries
            .iter()
            .map(|e| icons.type_name(&e.extension(), e.is_dir))
            .collect();
        self.icon_indices = vec![None; entries.len()];
        self.entries = entries;
        self.selection.clear();
        self.anchor = None;
        self.resort();
    }

    /// 목록을 비운다 (열거 실패 시)
    pub fn clear(&mut self) {
        self.entries.clear();
        self.type_names.clear();
        self.icon_indices.clear();
        self.selection.clear();
        self.anchor = None;
    }

    pub fn entry_at(&self, index: usize) -> Option<&FileEntry> {
        self.entries.get(index)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// 선택된 항목들의 전체 경로 — 셸 컨텍스트 메뉴 대상 (FR-8)
    pub fn selected_paths(&self) -> Vec<PathBuf> {
        self.selection
            .iter()
            .filter_map(|&i| self.entries.get(i))
            .map(|e| self.dir.join(e.name_string()))
            .collect()
    }

    /// 현재 정렬 기준으로 항목을 다시 배열한다.
    ///
    /// 폴더/파일 판정을 먼저 하고 **같은 종류끼리만** 방향을 뒤집는다 —
    /// `compare_entries` 반환값 전체를 뒤집으면 폴더 우선까지 뒤집힌다 (part1 D13)
    fn resort(&mut self) {
        let (key, asc) = (self.sort_key, self.ascending);
        let mut rows: Vec<(FileEntry, String, Option<i32>)> = self
            .entries
            .drain(..)
            .zip(self.type_names.drain(..))
            .zip(self.icon_indices.drain(..))
            .map(|((e, t), i)| (e, t, i))
            .collect();
        rows.sort_by(|(a, ta, _), (b, tb, _)| match (a.is_dir, b.is_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => {
                let ord = compare_entries(a, ta, b, tb, key);
                if asc { ord } else { ord.reverse() }
            }
        });
        for (e, t, i) in rows {
            self.entries.push(e);
            self.type_names.push(t);
            self.icon_indices.push(i);
        }
        // 정렬로 인덱스가 바뀌므로 선택·기준점을 유지할 수 없다
        self.selection.clear();
        self.anchor = None;
    }

    /// 헤더 클릭 처리 — 같은 열이면 방향 토글, 다른 열이면 그 열 오름차순
    fn apply_sort(&mut self, key: SortKey) {
        if self.sort_key == key {
            self.ascending = !self.ascending;
        } else {
            self.sort_key = key;
            self.ascending = true;
        }
        self.resort();
    }

    /// 클릭 선택 — Ctrl은 토글, Shift는 기준점부터 범위
    fn select(&mut self, index: usize, modifiers: egui::Modifiers) {
        if modifiers.shift
            && let Some(anchor) = self.anchor
        {
            let (lo, hi) = if anchor <= index {
                (anchor, index)
            } else {
                (index, anchor)
            };
            self.selection = (lo..=hi).collect();
            return;
        }
        if modifiers.ctrl {
            if !self.selection.remove(&index) {
                self.selection.insert(index);
            }
        } else {
            self.selection.clear();
            self.selection.insert(index);
        }
        self.anchor = Some(index);
    }

    /// 열 머리글 — 클릭으로 정렬, 현재 정렬 열에 방향 표시
    fn header(&mut self, ui: &mut egui::Ui) {
        let (_, rect) = ui.allocate_space(egui::vec2(ui.available_width(), HEADER_HEIGHT));
        ui.painter().rect_filled(rect, 0.0, theme::HEADER_BG);
        let columns = [
            (SortKey::Name, "이름", rect.left(), COL_NAME_W),
            (SortKey::Size, "크기", rect.left() + COL_NAME_W, COL_SIZE_W),
            (
                SortKey::Type,
                "종류",
                rect.left() + COL_NAME_W + COL_SIZE_W,
                COL_TYPE_W,
            ),
            (
                SortKey::Modified,
                "수정한 날짜",
                rect.left() + COL_NAME_W + COL_SIZE_W + COL_TYPE_W,
                rect.right() - (rect.left() + COL_NAME_W + COL_SIZE_W + COL_TYPE_W),
            ),
        ];
        let font = egui::TextStyle::Body.resolve(ui.style());
        let mut clicked = None;
        for (key, label, x, width) in columns {
            if width <= 0.0 {
                continue;
            }
            let cell = egui::Rect::from_min_size(
                egui::pos2(x, rect.top()),
                egui::vec2(width, HEADER_HEIGHT),
            );
            let id = ui.id().with(("head", label));
            if ui.interact(cell, id, egui::Sense::click()).clicked() {
                clicked = Some(key);
            }
            let arrow = if self.sort_key == key {
                if self.ascending { " ▲" } else { " ▼" }
            } else {
                ""
            };
            ui.painter().text(
                egui::pos2(x + CELL_PAD, cell.center().y),
                egui::Align2::LEFT_CENTER,
                format!("{label}{arrow}"),
                font.clone(),
                theme::HEADER_TEXT,
            );
        }
        if let Some(key) = clicked {
            self.apply_sort(key);
        }
    }

    /// 목록 본문 — 보이는 행만 그린다(가상 스크롤).
    /// `icons`·`textures`는 앱 전역에서 공유하므로 인자로 받는다(패널마다 캐시를 두면 낭비)
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        icons: &mut IconCache,
        textures: &mut IconTextures,
    ) -> FileListAction {
        self.header(ui);
        let mut action = FileListAction::None;
        // 선택 변경은 그리기 루프가 self를 빌린 뒤에 적용한다
        let mut select_request: Option<(usize, egui::Modifiers)> = None;
        let himl = icons.himl();
        let ctx = ui.ctx().clone();

        let scroll = egui::ScrollArea::vertical().auto_shrink([false, false]);
        let output = scroll.show_rows(ui, ROW_HEIGHT, self.entries.len(), |ui, range| {
            let font = egui::TextStyle::Body.resolve(ui.style());
            let stripe = ui.visuals().faint_bg_color;
            let hover_bg = ui.visuals().widgets.hovered.bg_fill;
            let sel_bg = ui.visuals().selection.bg_fill;
            for index in range {
                let entry = &self.entries[index];
                // 행 전체를 하나의 클릭 대상으로 잡는다 — 어느 열을 눌러도 같은 행이 된다
                let (id, rect) = ui.allocate_space(egui::vec2(ui.available_width(), ROW_HEIGHT));
                let resp = ui.interact(rect, id, egui::Sense::click());
                if resp.clicked() {
                    select_request = Some((index, ui.input(|i| i.modifiers)));
                }
                if resp.double_clicked() {
                    action = FileListAction::Open(index);
                }
                if resp.secondary_clicked()
                    && let Some(pos) = resp.interact_pointer_pos()
                {
                    // 선택되지 않은 행을 우클릭하면 그 행을 단독 선택한 뒤 메뉴를 연다
                    if !self.selection.contains(&index) {
                        select_request = Some((index, egui::Modifiers::NONE));
                    }
                    action = FileListAction::Context {
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
                if self.selection.contains(&index) {
                    painter.rect_filled(rect, 0.0, sel_bg);
                } else if resp.hovered() {
                    painter.rect_filled(rect, 0.0, hover_bg);
                }

                let y = rect.center().y;
                // 보이는 행에 한해 아이콘 인덱스를 조회한다 — 로드 시 전체를 미리 계산하면
                // exe가 많은 폴더에서 로드가 길어진다(PoC 실측 585ms → 84ms)
                let icon_index = match self.icon_indices[index] {
                    Some(cached) => cached,
                    None => {
                        let full = self.dir.join(entry.name_string());
                        let looked_up = icons.icon_index(
                            &entry.extension(),
                            entry.is_dir,
                            Some(&full.to_string_lossy()),
                        );
                        self.icon_indices[index] = Some(looked_up);
                        looked_up
                    }
                };
                if let Some(tex) = textures.get(&ctx, himl, icon_index) {
                    let icon_rect = egui::Rect::from_min_size(
                        egui::pos2(rect.left() + ICON_X, y - ICON_SIZE / 2.0),
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

                let size_text = if entry.is_dir {
                    String::new()
                } else {
                    format_size_kb(entry.size)
                };
                let cells = [
                    (
                        entry.name_string(),
                        rect.left() + NAME_X,
                        COL_NAME_W - NAME_X - CELL_PAD,
                    ),
                    (
                        size_text,
                        rect.left() + COL_NAME_W + CELL_PAD,
                        COL_SIZE_W - CELL_PAD * 2.0,
                    ),
                    (
                        self.type_names[index].clone(),
                        rect.left() + COL_NAME_W + COL_SIZE_W + CELL_PAD,
                        COL_TYPE_W - CELL_PAD * 2.0,
                    ),
                    (
                        format_filetime(entry.modified),
                        rect.left() + COL_NAME_W + COL_SIZE_W + COL_TYPE_W + CELL_PAD,
                        rect.right() - (rect.left() + COL_NAME_W + COL_SIZE_W + COL_TYPE_W),
                    ),
                ];
                let painter = ui.painter();
                for (text, x, width) in cells {
                    if text.is_empty() || width <= 0.0 {
                        continue;
                    }
                    // 셀 폭을 넘는 글자는 잘라 그린다 — 다음 열을 침범하지 않게
                    let galley = painter.layout(text, font.clone(), theme::TEXT, width.max(0.0));
                    painter.galley(
                        egui::pos2(x, y - galley.size().y / 2.0),
                        galley,
                        theme::TEXT,
                    );
                }
            }
        });

        // 빈 영역 우클릭 → 폴더 배경 메뉴. 행에서 이미 메뉴를 요청했으면 덮지 않는다
        if action == FileListAction::None {
            let below = egui::Rect::from_min_max(
                egui::pos2(output.inner_rect.left(), output.inner_rect.top()),
                output.inner_rect.max,
            );
            let resp = ui.interact(below, ui.id().with("list_bg"), egui::Sense::click());
            if resp.secondary_clicked()
                && let Some(pos) = resp.interact_pointer_pos()
            {
                action = FileListAction::Context { index: None, pos };
            }
            if resp.clicked() {
                self.selection.clear();
                self.anchor = None;
            }
        }
        if let Some((index, modifiers)) = select_request {
            self.select(index, modifiers);
        }
        action
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(name: &str, is_dir: bool, size: u64, modified: u64) -> FileEntry {
        let mut wide: Vec<u16> = name.encode_utf16().collect();
        wide.push(0);
        FileEntry {
            name: wide,
            is_dir,
            size,
            modified,
        }
    }

    /// `IconCache`(Win32 셸 호출) 없이 정렬만 검증하기 위한 조립
    fn view(rows: Vec<(FileEntry, &str)>) -> FileListView {
        let mut v = FileListView::new();
        v.icon_indices = vec![None; rows.len()];
        for (e, t) in rows {
            v.entries.push(e);
            v.type_names.push(t.to_owned());
        }
        v.resort();
        v
    }

    fn names(v: &FileListView) -> Vec<String> {
        v.entries.iter().map(|e| e.name_string()).collect()
    }

    #[test]
    fn 폴더가_파일보다_앞에_온다() {
        let v = view(vec![
            (entry("zeta.txt", false, 0, 0), "텍스트"),
            (entry("alpha", true, 0, 0), "폴더"),
        ]);
        assert_eq!(names(&v), vec!["alpha", "zeta.txt"]);
    }

    #[test]
    fn 이름은_숫자_인지_정렬이다() {
        let v = view(vec![
            (entry("파일10.txt", false, 0, 0), "텍스트"),
            (entry("파일2.txt", false, 0, 0), "텍스트"),
        ]);
        assert_eq!(names(&v), vec!["파일2.txt", "파일10.txt"]);
    }

    #[test]
    fn 빈_목록도_안전하다() {
        let v = view(Vec::new());
        assert!(v.is_empty());
    }

    #[test]
    fn 내림차순에서도_폴더가_먼저다() {
        let mut v = view(vec![
            (entry("b.txt", false, 0, 0), "텍스트"),
            (entry("a폴더", true, 0, 0), "폴더"),
        ]);
        // 같은 열을 다시 클릭 → 내림차순
        v.apply_sort(SortKey::Name);
        assert_eq!(names(&v), vec!["a폴더", "b.txt"]);
    }

    #[test]
    fn 크기_정렬은_작은_것부터다() {
        let v = {
            let mut v = view(vec![
                (entry("big.bin", false, 9000, 0), "파일"),
                (entry("small.bin", false, 10, 0), "파일"),
            ]);
            v.apply_sort(SortKey::Size);
            v
        };
        assert_eq!(names(&v), vec!["small.bin", "big.bin"]);
    }

    #[test]
    fn 수정일_정렬은_오래된_것부터다() {
        let mut v = view(vec![
            (entry("new.txt", false, 0, 200), "텍스트"),
            (entry("old.txt", false, 0, 100), "텍스트"),
        ]);
        v.apply_sort(SortKey::Modified);
        assert_eq!(names(&v), vec!["old.txt", "new.txt"]);
    }

    #[test]
    fn 다른_열을_누르면_오름차순으로_시작한다() {
        let mut v = view(Vec::new());
        v.apply_sort(SortKey::Name); // 같은 열 → 내림차순
        assert!(!v.ascending);
        v.apply_sort(SortKey::Size); // 다른 열 → 오름차순
        assert!(v.ascending);
        assert_eq!(v.sort_key, SortKey::Size);
    }
}
