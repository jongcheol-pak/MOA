//! 파일 목록 — 항목·선택·정렬 상태의 소유자 (FR-4·FR-5, NFR-3).
//!
//! 표시 규칙(크기·날짜 문자열)과 정렬 비교는 `panel::file_list`의 순수 함수를 그대로 쓴다 —
//! 복제하면 두 벌로 갈라진다. **그리기는 보기 모드별 모듈이 맡고**(자세히 보기는
//! `ui::list_details`) 이 파일은 상태를 들고 그 모듈에 넘긴 뒤, 돌아온 조작을 상태에 반영한다.
use crate::fs::enumerate::FileEntry;
use crate::fs::icons::IconCache;
use crate::panel::file_list::{SortKey, compare_entries};
use crate::ui::icon_tex::IconTextures;
use crate::ui::list_details::{self, Columns, DetailsInput};
use crate::ui::theme;
use eframe::egui;
use std::collections::{BTreeSet, HashSet};
use std::path::PathBuf;

/// 목록이 상위(패널)에 돌려주는 사용자 조작.
/// 즉시 모드라 콜백을 등록하지 않고 이번 프레임의 조작을 값으로 반환한다
#[derive(Clone, PartialEq, Debug, Default)]
pub enum FileListAction {
    #[default]
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
    /// 자세히 보기 열 폭 — 패널마다 독립이며 세션에 저장된다 (plan D3)
    columns: Columns,
    /// 폴더 개수 — 항목이 바뀔 때(`resort`) 한 번만 센다.
    /// 프레임마다 다시 세면 10만 항목 폴더에서 그 비용이 매 프레임 붙는다 (NFR-3)
    dir_count: usize,
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
            columns: Columns::new(),
            dir_count: 0,
        }
    }

    /// 저장된 열 폭을 되살린다 (FR-11)
    pub fn set_columns(&mut self, saved: &[f32]) {
        self.columns = Columns::from_saved(saved);
    }

    /// 세션에 저장할 열 폭
    pub fn columns(&self) -> Vec<f32> {
        self.columns.to_saved()
    }

    /// 새 폴더의 항목으로 교체한다. 정렬은 현재 정렬 기준을 유지한다.
    ///
    /// **같은 폴더를 다시 읽은 경우에는 선택을 이름 기준으로 되살린다** — 변경 감시가
    /// 갱신할 때마다 선택이 풀리면(FR-10), 여러 파일을 고르는 동안 다른 앱이 그 폴더에
    /// 파일 하나만 만들어도 고르던 것이 사라진다. 지워진 항목은 자연히 빠진다
    pub fn set_entries(&mut self, dir: PathBuf, entries: Vec<FileEntry>, icons: &mut IconCache) {
        let keep: Option<HashSet<Vec<u16>>> = (dir == self.dir).then(|| {
            self.selection
                .iter()
                .filter_map(|&index| self.entries.get(index))
                .map(|entry| entry.name.clone())
                .collect()
        });
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
        if let Some(keep) = keep {
            // 기준점(anchor)은 복원하지 않는다 — 다음 클릭이 새로 잡으며,
            // 없는 상태가 엉뚱한 위치를 가리키는 것보다 낫다
            self.selection = restore_selection(&self.entries, &keep);
        }
    }

    pub fn entry_at(&self, index: usize) -> Option<&FileEntry> {
        self.entries.get(index)
    }

    /// 항목 총 개수. 상태 줄이 `counts()`로 옮겨가 지금은 호출부가 없지만,
    /// 격자 보기(T10·T11)가 배치 계산에 총 개수를 쓰므로 남겨둔다
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// 폴더 수와 파일 수 — 목록 위 상태 줄에 쓴다
    pub fn counts(&self) -> (usize, usize) {
        (self.dir_count, self.entries.len() - self.dir_count)
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
        // 폴더 수는 여기서 센다 — 항목이 바뀌는 경로(`set_entries`)가 반드시 이 함수를 지나므로
        // 집계가 목록과 어긋날 수 없다. 정렬 때마다 다시 세지만 정렬은 사용자 클릭 시에만
        // 일어나고 비용도 이미 정렬(O(n log n))에 묻힌다
        self.dir_count = self.entries.iter().filter(|entry| entry.is_dir).count();
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

    /// 목록을 그리고 이번 프레임의 조작을 반영한다.
    /// `icons`·`textures`는 앱 전역에서 공유하므로 인자로 받는다(패널마다 캐시를 두면 낭비)
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        icons: &mut IconCache,
        textures: &mut IconTextures,
    ) -> FileListAction {
        let outcome = list_details::show(
            ui,
            DetailsInput {
                dir: &self.dir,
                entries: &self.entries,
                type_names: &self.type_names,
                icon_indices: &mut self.icon_indices,
                selection: &self.selection,
                sort_key: self.sort_key,
                ascending: self.ascending,
                columns: &mut self.columns,
            },
            icons,
            textures,
        );
        // 상태 변경은 그리기가 끝난 뒤에 한다 — 그리는 동안에는 목록이 빌려진 상태다
        if let Some(key) = outcome.sort_click {
            self.apply_sort(key);
        } else if let Some((index, modifiers)) = outcome.select_request {
            // 정렬이 일어나면 인덱스가 통째로 바뀌므로 같은 프레임의 선택은 버린다
            self.select(index, modifiers);
        }
        if outcome.clear_selection {
            self.selection.clear();
            self.anchor = None;
        }
        outcome.action
    }
}

/// 셀 텍스트를 **한 줄로만** 배치하고, 폭을 넘으면 끝을 `…`로 줄인 갤리를 만든다.
///
/// `Painter::layout`을 쓰면 안 된다 — 그 함수의 폭 인자는 자르는 폭이 아니라 **줄바꿈 폭**이라
/// 긴 이름이 여러 줄이 된다. 행 높이는 `ROW_HEIGHT` 고정이므로 2줄이 되는 순간 아래 행과 겹쳐
/// 글자가 포개져 보인다(사용자 보고 4번). `max_rows: 1`이 그 겹침과 말줄임을 함께 해결한다
pub(crate) fn elided_galley(
    painter: &egui::Painter,
    text: String,
    font: egui::FontId,
    max_width: f32,
) -> std::sync::Arc<egui::Galley> {
    let mut job = egui::text::LayoutJob::simple(text, font, theme::TEXT, max_width);
    // max_rows=1 + break_anywhere + overflow_character('…')를 한 번에 준다.
    // 파일 이름은 공백 없는 긴 토큰이 흔해 단어 단위로만 끊으면 폭을 넘는 채로 잘린다
    job.wrap = egui::text::TextWrapping::truncate_at_width(max_width);
    painter.layout_job(job)
}

/// 갱신 전 선택 이름들이 새 목록의 어느 자리인지 되찾는다 (정렬이 끝난 뒤의 인덱스).
/// 목록 길이에 비례해 한 번만 훑는다 — 10만 항목에서 선택이 많아도 비용이 튀지 않는다
fn restore_selection(entries: &[FileEntry], keep: &HashSet<Vec<u16>>) -> BTreeSet<usize> {
    entries
        .iter()
        .enumerate()
        .filter(|(_, entry)| keep.contains(&entry.name))
        .map(|(index, _)| index)
        .collect()
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
    fn 갱신_후에도_선택은_이름으로_되살아난다() {
        // 감시 갱신(FR-10)에서 목록이 통째로 바뀌어도 고르던 파일은 그대로여야 한다
        let entries = vec![
            entry("사진", true, 0, 0),
            entry("a.txt", false, 0, 0),
            entry("b.txt", false, 0, 0),
        ];
        let keep: HashSet<Vec<u16>> = ["b.txt", "사진"]
            .iter()
            .map(|n| {
                let mut w: Vec<u16> = n.encode_utf16().collect();
                w.push(0);
                w
            })
            .collect();
        let selection = restore_selection(&entries, &keep);
        assert_eq!(selection.into_iter().collect::<Vec<_>>(), vec![0, 2]);
    }

    #[test]
    fn 사라진_항목은_선택에서_빠진다() {
        let entries = vec![entry("남은.txt", false, 0, 0)];
        let mut keep = HashSet::new();
        for name in ["남은.txt", "지워진.txt"] {
            let mut w: Vec<u16> = name.encode_utf16().collect();
            w.push(0);
            keep.insert(w);
        }
        let selection = restore_selection(&entries, &keep);
        assert_eq!(selection.into_iter().collect::<Vec<_>>(), vec![0]);
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

    /// 폭 하나로 셀 텍스트를 배치해 (줄 수, 말줄임 여부)를 돌려준다.
    ///
    /// **앱과 같은 글꼴을 설치한 뒤 배치한다** — 이 crate는 egui 기본 글꼴 기능을 끄고
    /// (`eframe` default-features 해제) 맑은 고딕을 직접 등록하므로, 글꼴 없이 배치하면
    /// 모든 글자 폭이 0이 되어 말줄임이 일어나지 않는다(폭 기준 검증이 무의미해진다)
    fn layout_rows(text: &str, width: f32) -> (usize, bool) {
        let ctx = egui::Context::default();
        let has_font = crate::ui::app::install_fonts(&ctx);
        let mut result = (0, false);
        let _ = ctx.run_ui(Default::default(), |ui| {
            let font = egui::TextStyle::Body.resolve(ui.style());
            let galley = elided_galley(ui.painter(), text.to_owned(), font, width);
            result = (galley.rows.len(), galley.elided);
        });
        // 글꼴을 못 읽는 환경에서는 폭이 0이라 말줄임 판정이 성립하지 않는다
        assert!(has_font, "맑은 고딕을 읽지 못해 폭 기준 검증을 할 수 없다");
        result
    }

    #[test]
    fn 폴더와_파일을_따로_센다() {
        let v = view(vec![
            (entry("문서", true, 0, 0), "폴더"),
            (entry("사진", true, 0, 0), "폴더"),
            (entry("a.txt", false, 10, 0), "텍스트"),
        ]);
        assert_eq!(v.counts(), (2, 1));
    }

    #[test]
    fn 빈_폴더는_둘_다_0이다() {
        assert_eq!(view(Vec::new()).counts(), (0, 0));
    }

    #[test]
    fn 파일만_있으면_폴더는_0이다() {
        let v = view(vec![(entry("a.txt", false, 10, 0), "텍스트")]);
        assert_eq!(v.counts(), (0, 1));
    }

    #[test]
    fn 긴_이름은_한_줄로_줄어든다() {
        // 2줄이 되면 행 높이(ROW_HEIGHT)를 넘어 아래 행과 글자가 겹친다 — 사용자 보고 4번
        let long = "NTUSER.DAT{71e7eeb8-8e0f-11f0-80fa-000d3aa7ca88}.TM.blf";
        let (rows, elided) = layout_rows(long, 100.0);
        assert_eq!(rows, 1, "긴 이름이 여러 줄로 배치됐다");
        assert!(elided, "폭을 넘었는데 말줄임되지 않았다");
    }

    #[test]
    fn 짧은_이름은_줄이지_않는다() {
        let (rows, elided) = layout_rows("a.txt", 300.0);
        assert_eq!(rows, 1);
        assert!(!elided, "폭에 들어가는 이름까지 말줄임됐다");
    }

    #[test]
    fn 아주_좁은_폭에서도_패닉하지_않는다() {
        // 열을 최소까지 좁히거나(T2) 마지막 열이 음수 폭이 되는 경우 — 그려지지 않더라도 죽으면 안 된다
        for width in [0.0, 1.0, 3.0, -10.0] {
            let (rows, _) = layout_rows("아주긴한글파일이름.txt", width);
            assert!(rows <= 1, "폭 {width}: 한 줄을 넘겼다");
        }
    }

    #[test]
    fn 한글도_폭_기준으로_줄어든다() {
        // 문자 수가 아니라 픽셀 폭 기준이어야 한다 — 한글은 영문보다 넓다
        let (_, elided) = layout_rows("가나다라마바사아자차카타파하", 40.0);
        assert!(elided);
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
