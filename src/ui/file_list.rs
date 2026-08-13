//! 파일 목록 — 항목·선택·정렬 상태의 소유자 (FR-4·FR-5, NFR-3).
//!
//! 표시 규칙(크기·날짜 문자열)과 정렬 비교는 `panel::file_list`의 순수 함수를 그대로 쓴다 —
//! 복제하면 두 벌로 갈라진다. **그리기는 보기 모드별 모듈이 맡고**(자세히 보기는
//! `ui::list_details`) 이 파일은 상태를 들고 그 모듈에 넘긴 뒤, 돌아온 조작을 상태에 반영한다.
use crate::fs::enumerate::FileEntry;
use crate::fs::icons::IconCache;
use crate::panel::file_list::{ListRow, SortKey, compare_rows};
use crate::remote::types::RemoteEntry;
use crate::ui::icon_tex::{IconTextures, ThumbnailTextures};
use crate::ui::list_details::{self, ColumnFlags, ColumnKind, Columns, DetailsInput};
use crate::ui::list_grid::{self, GridInput};
use crate::ui::view_mode::ViewMode;
use eframe::egui;
use std::collections::{BTreeSet, HashSet};
use std::path::PathBuf;

// 조작 타입은 보기 모드별 렌더 모듈이 함께 쓰므로 공용 모듈에 둔다.
// 호출부(`ui::panel`)가 종전 경로를 그대로 쓰도록 여기서 다시 내보낸다
use crate::ui::list_common::DragItem;
pub use crate::ui::list_common::FileListAction;
use crate::ui::remote_menu::RemoteTarget;

/// 목록이 담은 항목 — 로컬 폴더의 것이거나 원격 폴더의 것이다 (plan T8).
///
/// 하나의 슬라이스로 합치지 않는 이유: 두 항목 타입은 이름·시각을 담는 방식이 아예 달라
/// (널 종단 UTF-16 + FILETIME ↔ `String` + 유닉스 초) 한 벌로 만들면 어느 한쪽이 손해를 본다.
/// 대신 **정렬·표시 규칙만** `ListRow`로 공유하고, 그리기는 종류별로 한 번씩 찍어 낸다
/// (트레이트 객체가 아니라 제네릭 — 10만 항목에 가상 호출을 넣지 않는다).
pub enum ListModel {
    Local(Vec<FileEntry>),
    Remote(Vec<RemoteEntry>),
}

impl ListModel {
    pub fn len(&self) -> usize {
        match self {
            ListModel::Local(rows) => rows.len(),
            ListModel::Remote(rows) => rows.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// 로컬 폴더의 항목인가 — 전체 경로로 하는 일(썸네일·셸 아이콘)이 이것으로 갈린다
    pub fn is_local(&self) -> bool {
        matches!(self, ListModel::Local(_))
    }
}

/// 파일 목록 뷰 — 항목·종류 문자열·아이콘 인덱스·선택 상태를 함께 소유한다.
/// 셋은 인덱스로 짝지어지므로 따로 두면 정렬 시 어긋난다
pub struct FileListView {
    dir: PathBuf,
    model: ListModel,
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
    /// 켤 수 있는 열(권한·소유자)의 표시 여부 — 원격 패널에서만 쓰인다 (FR-31)
    column_flags: ColumnFlags,
    /// 폴더·파일 개수 — 항목이 바뀔 때(`resort`) 한 번만 센다.
    /// 프레임마다 다시 세면 10만 항목 폴더에서 그 비용이 매 프레임 붙는다 (NFR-3).
    /// 둘의 합이 줄 수와 다를 수 있다 — 상위 이동(`..`) 줄은 어느 쪽도 아니다
    dir_count: usize,
    file_count: usize,
    /// 보기 모드 — 패널마다 독립이며 세션에 저장된다 (FR-23)
    view_mode: ViewMode,
    /// 이름 뒤 확장자를 보일지 (FR-52) — 앱 설정에서 매 프레임 받는다(패널마다 다르지 않다)
    show_extensions: bool,
    /// 숨김·시스템 항목을 보일지 (FR-13) — 같은 경로로 받는다.
    ///
    /// **거르기는 항목을 받는 자리에서 한 번만** 한다: `type_names`·`icon_indices`가
    /// 항목과 인덱스로 짝지어져 있어, 그리는 자리에서 거르면 셋이 어긋난다
    show_hidden: bool,
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
            model: ListModel::Local(Vec::new()),
            type_names: Vec::new(),
            icon_indices: Vec::new(),
            sort_key: SortKey::Name,
            ascending: true,
            selection: BTreeSet::new(),
            anchor: None,
            columns: Columns::new(),
            column_flags: ColumnFlags::default(),
            dir_count: 0,
            file_count: 0,
            view_mode: ViewMode::default(),
            show_extensions: true,
            show_hidden: true,
        }
    }

    /// 앱 설정의 표시 규칙을 받는다 (FR-52) — 패널마다 다르지 않으므로 매 프레임 넘겨받는다.
    ///
    /// 목록이 스스로 설정을 읽지 않는 이유: `panel`·`ui::file_list`는 앱 설정을 모르는 층이고,
    /// 여기서 읽게 하면 그 의존이 목록 전체로 번진다
    pub fn set_show_extensions(&mut self, show: bool) {
        self.show_extensions = show;
    }

    /// 숨김 항목 표시 여부를 받는다 (FR-13). **바뀌었으면 `true`** — 그때는 호출부가
    /// 폴더를 다시 읽어야 한다.
    ///
    /// 이 자리에서 되돌릴 수 없는 이유: 거른 항목을 따로 쥐고 있지 않다. 쥐려면
    /// 항목뿐 아니라 `type_names`·`icon_indices`까지 같은 순서로 함께 쥐어야 해서,
    /// 다시 읽는 편이 훨씬 단순하다(로컬 열거는 감시 갱신 때마다 이미 하는 일이다)
    pub fn set_show_hidden(&mut self, show: bool) -> bool {
        let changed = self.show_hidden != show;
        self.show_hidden = show;
        changed
    }

    /// 지금 쓰는 보기 모드 — 메뉴의 현재 표시와 세션 저장에 쓴다
    pub fn view_mode(&self) -> ViewMode {
        self.view_mode
    }

    /// 보기 모드를 바꾼다 (FR-23)
    pub fn set_view_mode(&mut self, mode: ViewMode) {
        self.view_mode = mode;
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
    pub fn set_entries(
        &mut self,
        dir: PathBuf,
        mut entries: Vec<FileEntry>,
        icons: &mut IconCache,
    ) {
        if !self.show_hidden {
            entries.retain(|e| !e.is_hidden());
        }
        let keep = (dir == self.dir).then(|| self.selected_name_keys());
        self.dir = dir;
        self.type_names = entries
            .iter()
            .map(|e| icons.type_name(&e.extension(), e.is_dir))
            .collect();
        self.icon_indices = vec![None; entries.len()];
        self.model = ListModel::Local(entries);
        self.selection.clear();
        self.anchor = None;
        self.resort();
        if let Some(keep) = keep {
            // 기준점(anchor)은 복원하지 않는다 — 다음 클릭이 새로 잡으며,
            // 없는 상태가 엉뚱한 위치를 가리키는 것보다 낫다
            self.selection = self.matching_selection(&keep);
        }
    }

    /// 목록을 비운다 — 아직 무엇을 보여 줄지 모르는 구간에서 **옛 항목이 남지 않게** 한다.
    ///
    /// 선택도 함께 지운다: 지우지 않으면 없는 항목을 고른 상태가 되어, 다음 목록이 도착했을 때
    /// 엉뚱한 줄이 골라진 것처럼 보인다
    pub fn clear_entries(&mut self) {
        self.type_names.clear();
        self.icon_indices.clear();
        self.model = ListModel::Remote(Vec::new());
        self.selection.clear();
        self.anchor = None;
    }

    /// 원격 폴더의 항목으로 교체한다 (FR-31).
    ///
    /// 로컬과 달리 **선택을 되살리지 않는다** — 원격 목록은 사용자가 새로 고침을 눌렀을 때만
    /// 바뀌고(변경 감시가 없다 — Deferred), 그때는 선택이 풀리는 것이 자연스럽다
    pub fn set_remote_entries(&mut self, mut entries: Vec<RemoteEntry>, icons: &mut IconCache) {
        if !self.show_hidden {
            entries.retain(|e| !e.is_hidden());
        }
        self.type_names = entries
            .iter()
            .map(|e| icons.type_name(&e.extension(), e.is_dir))
            .collect();
        self.icon_indices = vec![None; entries.len()];
        self.model = ListModel::Remote(entries);
        self.selection.clear();
        self.anchor = None;
        self.resort();
    }

    /// 지금 담긴 항목들 — 종류별 분기가 필요한 호출부가 쓴다
    pub fn model(&self) -> &ListModel {
        &self.model
    }

    /// 원격 항목 하나 — 로컬 목록이면 `None`이다. 더블클릭으로 폴더에 들어갈 때 쓴다
    pub fn remote_at(&self, index: usize) -> Option<&RemoteEntry> {
        match &self.model {
            ListModel::Remote(rows) => rows.get(index),
            ListModel::Local(_) => None,
        }
    }

    /// 로컬 항목 하나. **원격 목록이면 `None`**이다 — 셸 메뉴처럼 로컬 파일에만 있는 일의
    /// 진입점이라, 원격을 억지로 끼워 넣으면 없는 경로를 셸에 넘기게 된다 (D21)
    pub fn entry_at(&self, index: usize) -> Option<&FileEntry> {
        match &self.model {
            ListModel::Local(rows) => rows.get(index),
            ListModel::Remote(_) => None,
        }
    }

    /// 항목 총 개수. 상태 줄이 `counts()`로 옮겨가 지금은 호출부가 없지만,
    /// 격자 보기(T10·T11)가 배치 계산에 총 개수를 쓰므로 남겨둔다
    pub fn len(&self) -> usize {
        self.model.len()
    }

    pub fn is_empty(&self) -> bool {
        self.model.is_empty()
    }

    /// 폴더 수와 파일 수 — 목록 위 상태 줄에 쓴다.
    /// 상위 이동(`..`) 줄은 실제 항목이 아니라 어느 쪽에도 세지 않는다
    pub fn counts(&self) -> (usize, usize) {
        (self.dir_count, self.file_count)
    }

    /// 선택된 항목들의 전체 경로 — 셸 컨텍스트 메뉴 대상 (FR-8)
    pub fn selected_paths(&self) -> Vec<PathBuf> {
        // 원격 항목에는 로컬 경로가 없다 — 셸 메뉴는 로컬 전용이고(D21), 원격 선택은
        // 원격 경로로 다루는 별도 경로(T22·T23)가 맡는다
        let ListModel::Local(rows) = &self.model else {
            return Vec::new();
        };
        self.selection
            .iter()
            .filter_map(|&i| rows.get(i))
            // 상위 이동은 대상이 아니다 — 셸 메뉴로 지우거나 복사할 것이 아니다
            .filter(|e| !e.is_parent())
            .map(|e| self.dir.join(e.name_string()))
            .collect()
    }

    /// 지금 선택된 항목들의 이름 키 — 갱신 뒤 선택을 되살리는 데 쓴다
    fn selected_name_keys(&self) -> HashSet<Vec<u16>> {
        fn collect<R: ListRow>(rows: &[R], selection: &BTreeSet<usize>) -> HashSet<Vec<u16>> {
            selection
                .iter()
                .filter_map(|&index| rows.get(index))
                .map(|row| row.name_sort_key().into_owned())
                .collect()
        }
        match &self.model {
            ListModel::Local(rows) => collect(rows, &self.selection),
            ListModel::Remote(rows) => collect(rows, &self.selection),
        }
    }

    /// 이름 키가 남아 있는 항목들의 새 인덱스
    fn matching_selection(&self, keep: &HashSet<Vec<u16>>) -> BTreeSet<usize> {
        match &self.model {
            ListModel::Local(rows) => restore_selection(rows, keep),
            ListModel::Remote(rows) => restore_selection(rows, keep),
        }
    }

    /// 현재 정렬 기준으로 항목을 다시 배열한다.
    ///
    /// 폴더/파일 판정을 먼저 하고 **같은 종류끼리만** 방향을 뒤집는다 —
    /// `compare_rows` 반환값 전체를 뒤집으면 폴더 우선까지 뒤집힌다 (part1 D13)
    fn resort(&mut self) {
        let (key, asc) = (self.sort_key, self.ascending);
        let (type_names, icon_indices) = (&mut self.type_names, &mut self.icon_indices);
        // 폴더 수는 여기서 센다 — 항목이 바뀌는 경로(`set_entries`)가 반드시 이 함수를 지나므로
        // 집계가 목록과 어긋날 수 없다. 정렬 때마다 다시 세지만 정렬은 사용자 클릭 시에만
        // 일어나고 비용도 이미 정렬(O(n log n))에 묻힌다
        (self.dir_count, self.file_count) = match &mut self.model {
            ListModel::Local(rows) => sort_rows(rows, type_names, icon_indices, key, asc),
            ListModel::Remote(rows) => sort_rows(rows, type_names, icon_indices, key, asc),
        };
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
        thumbnails: &ThumbnailTextures,
        visible: &mut Vec<PathBuf>,
    ) -> ListInteraction {
        // 필드를 미리 풀어 둔다 — 모델을 빌리는 동안 나머지 필드도 함께 빌려야 한다
        let FileListView {
            dir,
            model,
            type_names,
            icon_indices,
            selection,
            sort_key,
            ascending,
            columns,
            column_flags,
            view_mode,
            ..
        } = self;
        let request = RenderRequest {
            dir,
            type_names,
            icon_indices,
            selection,
            sort_key: *sort_key,
            ascending: *ascending,
            columns,
            column_flags: *column_flags,
            view_mode: *view_mode,
            thumbnails,
            visible,
            local_paths: model.is_local(),
            show_extensions: self.show_extensions,
        };
        let outcome = match model {
            ListModel::Local(rows) => render_rows(ui, rows, request, icons, textures),
            ListModel::Remote(rows) => render_rows(ui, rows, request, icons, textures),
        };

        // 상태 변경은 그리기가 끝난 뒤에 한다 — 그리는 동안에는 목록이 빌려진 상태다
        if let Some(kind) = outcome.column_toggle {
            self.column_flags.toggle(kind);
        }
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
        ListInteraction {
            action: outcome.action,
            drag_started: outcome.drag_started,
        }
    }

    /// 끌어 옮길 항목들 (FR-38).
    ///
    /// 끌기 시작한 항목이 **선택 밖이면 그것 하나만** 끈다 — 탐색기와 같은 규칙이다.
    /// 선택 안이면 선택 전체가 따라간다.
    ///
    /// 원격 목록은 자기가 어느 폴더를 보고 있는지 모른다(그것은 탭이 든다) — 그래서
    /// `remote_dir`를 받는다. 로컬 목록은 `dir`를 스스로 안다
    pub fn drag_items(
        &self,
        started: usize,
        remote_dir: Option<&crate::remote::types::RemotePath>,
    ) -> Vec<DragItem> {
        let indices: Vec<usize> = if self.selection.contains(&started) {
            self.selection.iter().copied().collect()
        } else {
            vec![started]
        };
        match &self.model {
            ListModel::Local(rows) => indices
                .iter()
                .filter_map(|index| rows.get(*index))
                // 상위 이동은 실을 것이 아니다 — 끌어다 놓으면 위 폴더째 옮기게 된다
                .filter(|row| !row.is_parent())
                .map(|row| DragItem::Local {
                    path: self.dir.join(row.name()),
                    is_dir: row.is_dir(),
                })
                .collect(),
            ListModel::Remote(rows) => {
                let Some(dir) = remote_dir else {
                    return Vec::new();
                };
                indices
                    .iter()
                    .filter_map(|index| rows.get(*index))
                    .filter(|row| !row.is_parent())
                    .map(|row| DragItem::Remote {
                        path: dir.join(&row.name()),
                        is_dir: row.is_dir(),
                        size: row.size(),
                    })
                    .collect()
            }
        }
    }
}

impl FileListView {
    /// 로컬 목록에서 고른 항목들 — 경로와 폴더 여부. 원격 목록이면 빈 벡터다.
    ///
    /// 원격 메뉴의 `올리기`가 반대편 패널에서 이것을 읽는다 (FR-39)
    pub fn selected_local(&self) -> Vec<(PathBuf, bool)> {
        let ListModel::Local(rows) = &self.model else {
            return Vec::new();
        };
        self.selection
            .iter()
            .filter_map(|index| rows.get(*index))
            // 상위 이동은 올릴 것이 아니다 — 위 폴더를 통째로 보내게 된다
            .filter(|row| !row.is_parent())
            .map(|row| (self.dir.join(row.name()), row.is_dir()))
            .collect()
    }

    /// 선택된 원격 항목들 — 원격 메뉴가 대상으로 삼는다 (FR-39).
    ///
    /// 로컬 목록이면 빈 벡터다(로컬은 셸 메뉴가 맡는다 — D21)
    pub fn selected_remote(&self, dir: &crate::remote::types::RemotePath) -> Vec<RemoteTarget> {
        let ListModel::Remote(rows) = &self.model else {
            return Vec::new();
        };
        self.selection
            .iter()
            .filter_map(|index| rows.get(*index))
            // 상위 이동은 대상이 아니다 — 지우거나 이름을 바꿀 것이 아니다
            .filter(|row| !row.is_parent())
            .map(|row| RemoteTarget {
                path: dir.join(&row.name()),
                is_dir: row.is_dir(),
                size: row.size(),
                mode: row.mode,
            })
            .collect()
    }
}

/// 목록에서 이번 프레임에 일어난 것 — 조작 하나와 끌기 시작 여부
#[derive(Debug, Default, Clone, PartialEq)]
pub struct ListInteraction {
    pub action: FileListAction,
    /// 끌기가 시작된 항목 — 무엇을 실을지는 `drag_items`가 정한다
    pub drag_started: Option<usize>,
}

/// 갱신 전 선택 이름들이 새 목록의 어느 자리인지 되찾는다 (정렬이 끝난 뒤의 인덱스).
/// 목록 길이에 비례해 한 번만 훑는다 — 10만 항목에서 선택이 많아도 비용이 튀지 않는다.
///
/// 이름을 **정렬 키(널 종단 UTF-16)로 견주는** 이유: 로컬 이름은 원래 그 모양이라
/// 표시 문자열로 바꿔 견주면 변환 손실이 끼어들 수 있다
fn restore_selection<R: ListRow>(rows: &[R], keep: &HashSet<Vec<u16>>) -> BTreeSet<usize> {
    rows.iter()
        .enumerate()
        .filter(|(_, row)| keep.contains(&*row.name_sort_key()))
        .map(|(index, _)| index)
        .collect()
}

/// 정렬 — 항목·종류 문자열·아이콘 인덱스를 함께 옮겨 인덱스 짝이 어긋나지 않게 한다.
/// 돌려주는 값은 (폴더 수, 파일 수)이며 상위 이동(`..`) 줄은 어느 쪽에도 세지 않는다
fn sort_rows<R: ListRow>(
    rows: &mut Vec<R>,
    type_names: &mut Vec<String>,
    icon_indices: &mut Vec<Option<i32>>,
    key: SortKey,
    ascending: bool,
) -> (usize, usize) {
    let mut zipped: Vec<(R, String, Option<i32>)> = rows
        .drain(..)
        .zip(type_names.drain(..))
        .zip(icon_indices.drain(..))
        .map(|((row, type_name), icon)| (row, type_name, icon))
        .collect();
    zipped.sort_by(|(a, ta, _), (b, tb, _)| {
        // 상위 이동(`..`)은 **어느 열로 어느 방향으로 정렬하든 맨 위**다 — 정렬 대상이 아니라
        // 목록 밖으로 나가는 문이라, 내림차순에서 바닥으로 밀려나면 매번 찾아 내려가야 한다.
        // 폴더 우선보다도 앞에 둔다(그것도 방향과 무관한 규칙이다 — part1 D13)
        match (a.is_parent(), b.is_parent()) {
            (true, false) => return std::cmp::Ordering::Less,
            (false, true) => return std::cmp::Ordering::Greater,
            _ => {}
        }
        match (a.is_dir(), b.is_dir()) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => {
                let ord = compare_rows(a, ta, b, tb, key);
                if ascending { ord } else { ord.reverse() }
            }
        }
    });
    let (mut dir_count, mut file_count) = (0, 0);
    for (row, type_name, icon) in zipped {
        // 상위 이동 줄은 세지 않는다 — 이 폴더에 실제로 든 것이 아니다
        if !row.is_parent() {
            if row.is_dir() {
                dir_count += 1;
            } else {
                file_count += 1;
            }
        }
        rows.push(row);
        type_names.push(type_name);
        icon_indices.push(icon);
    }
    (dir_count, file_count)
}

/// 그리기에 필요한 것들 — 모델 종류마다 같은 것을 넘기므로 한 묶음으로 든다
struct RenderRequest<'a> {
    dir: &'a std::path::Path,
    type_names: &'a [String],
    icon_indices: &'a mut Vec<Option<i32>>,
    selection: &'a BTreeSet<usize>,
    sort_key: SortKey,
    ascending: bool,
    columns: &'a mut Columns,
    column_flags: ColumnFlags,
    view_mode: ViewMode,
    thumbnails: &'a ThumbnailTextures,
    visible: &'a mut Vec<PathBuf>,
    local_paths: bool,
    /// 이름 뒤 확장자를 보일지 (FR-52)
    show_extensions: bool,
}

/// 이번 프레임에 그리고 돌려받은 것 — 상태 반영은 호출부가 한다
struct RenderOutcome {
    action: FileListAction,
    sort_click: Option<SortKey>,
    /// 열 메뉴에서 고른 열 — 자세히 보기에만 있다
    column_toggle: Option<ColumnKind>,
    select_request: Option<(usize, egui::Modifiers)>,
    clear_selection: bool,
    /// 끌기가 시작된 항목 (FR-38)
    drag_started: Option<usize>,
}

/// 보기 모드에 맞는 렌더 모듈에 넘긴다. **모델 종류마다 한 번씩 찍히는 유일한 자리**다
fn render_rows<R: ListRow>(
    ui: &mut egui::Ui,
    rows: &[R],
    request: RenderRequest<'_>,
    icons: &mut IconCache,
    textures: &mut IconTextures,
) -> RenderOutcome {
    // 자세히 보기만 열·머리글을 갖는다 — 나머지는 격자 렌더가 맡는다 (FR-23)
    if request.view_mode.is_details() {
        let outcome = list_details::show(
            ui,
            DetailsInput {
                dir: request.dir,
                entries: rows,
                type_names: request.type_names,
                icon_indices: request.icon_indices,
                selection: request.selection,
                sort_key: request.sort_key,
                ascending: request.ascending,
                columns: request.columns,
                // 원격 목록을 담고 있다는 것이 곧 원격 패널이라는 뜻이다 —
                // 별도 플래그를 하나 더 들면 둘이 어긋날 수 있다
                is_remote: !request.local_paths,
                column_flags: request.column_flags,
                local_paths: request.local_paths,
                show_extensions: request.show_extensions,
            },
            icons,
            textures,
        );
        RenderOutcome {
            action: outcome.action,
            sort_click: outcome.sort_click,
            column_toggle: outcome.column_toggle,
            select_request: outcome.select_request,
            clear_selection: outcome.clear_selection,
            drag_started: outcome.drag_started,
        }
    } else {
        let outcome = list_grid::show(
            ui,
            GridInput {
                dir: request.dir,
                entries: rows,
                icon_indices: request.icon_indices,
                selection: request.selection,
                type_names: request.type_names,
                mode: request.view_mode,
                thumbnails: request.thumbnails,
                visible: request.visible,
                local_paths: request.local_paths,
                show_extensions: request.show_extensions,
            },
            icons,
            textures,
        );
        RenderOutcome {
            action: outcome.action,
            sort_click: None,
            // 열은 자세히 보기에만 있다 — 격자 보기에는 머리글이 없다
            column_toggle: None,
            select_request: outcome.select_request,
            clear_selection: outcome.clear_selection,
            drag_started: outcome.drag_started,
        }
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
            attributes: 0,
        }
    }

    /// `IconCache`(Win32 셸 호출) 없이 정렬만 검증하기 위한 조립
    fn view(rows: Vec<(FileEntry, &str)>) -> FileListView {
        let mut v = FileListView::new();
        v.icon_indices = vec![None; rows.len()];
        let mut entries = Vec::with_capacity(rows.len());
        for (e, t) in rows {
            entries.push(e);
            v.type_names.push(t.to_owned());
        }
        v.model = ListModel::Local(entries);
        v.resort();
        v
    }

    fn names(v: &FileListView) -> Vec<String> {
        match v.model() {
            ListModel::Local(rows) => rows.iter().map(|e| e.name_string()).collect(),
            ListModel::Remote(rows) => rows.iter().map(|e| e.name.clone()).collect(),
        }
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

    #[test]
    fn 보기_모드는_기본이_자세히고_바꾸면_남는다() {
        // 메뉴에서 고른 모드가 상태에 반영되지 않으면 화면이 그대로다 (FR-23)
        let mut view = FileListView::new();
        assert_eq!(
            view.view_mode(),
            ViewMode::Details,
            "기본값이 자세히가 아니다"
        );
        view.set_view_mode(ViewMode::Tiles);
        assert_eq!(view.view_mode(), ViewMode::Tiles);
        // 다른 모드로 다시 바꿔도 마지막 것이 남는다
        view.set_view_mode(ViewMode::List);
        assert_eq!(view.view_mode(), ViewMode::List);
    }

    #[test]
    fn 보기_모드를_바꿔도_항목과_정렬은_그대로다() {
        // 모드는 표시 방식일 뿐이다 — 목록이 다시 읽히거나 정렬이 풀리면 안 된다
        let mut view = view(vec![
            (entry("문서", true, 0, 0), "폴더"),
            (entry("a.txt", false, 10, 0), "텍스트"),
        ]);
        let before = names(&view);
        let counts = view.counts();
        view.set_view_mode(ViewMode::ExtraLargeIcons);
        assert_eq!(names(&view), before);
        assert_eq!(view.counts(), counts);
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
    fn 다른_열을_누르면_오름차순으로_시작한다() {
        let mut v = view(Vec::new());
        v.apply_sort(SortKey::Name); // 같은 열 → 내림차순
        assert!(!v.ascending);
        v.apply_sort(SortKey::Size); // 다른 열 → 오름차순
        assert!(v.ascending);
        assert_eq!(v.sort_key, SortKey::Size);
    }

    fn remote(name: &str, is_dir: bool, size: u64) -> RemoteEntry {
        RemoteEntry {
            name: name.to_owned(),
            is_dir,
            is_symlink: false,
            link_target: None,
            size,
            modified: Some(1_700_000_000),
            mode: None,
            owner: None,
        }
    }

    /// 원격 항목을 담은 뷰 — 정렬까지 마친 상태로 돌려준다
    fn remote_view(rows: Vec<(RemoteEntry, &str)>) -> FileListView {
        let mut v = FileListView::new();
        v.icon_indices = vec![None; rows.len()];
        let mut entries = Vec::with_capacity(rows.len());
        for (row, type_name) in rows {
            entries.push(row);
            v.type_names.push(type_name.to_owned());
        }
        v.model = ListModel::Remote(entries);
        v.resort();
        v
    }

    #[test]
    fn 원격_모델도_같은_규칙으로_정렬되고_세어진다() {
        let v = remote_view(vec![
            (remote("app.js", false, 4096), "JS 파일"),
            (remote("public_html", true, 0), "폴더"),
        ]);
        assert_eq!(names(&v), vec!["public_html", "app.js"], "폴더가 앞선다");
        assert_eq!(v.counts(), (1, 1));
        assert_eq!(v.len(), 2);
        assert!(!v.is_empty());
    }

    #[test]
    fn 원격_모델은_로컬_전용_진입점을_내주지_않는다() {
        // 셸 메뉴는 로컬 파일에만 있는 일이다 (D21) — 없는 경로를 넘기면 안 된다
        let mut v = remote_view(vec![(remote("app.js", false, 10), "JS 파일")]);
        assert!(v.entry_at(0).is_none());
        v.selection.insert(0);
        assert!(v.selected_paths().is_empty());
    }

    #[test]
    fn 상위_이동_줄은_어떤_정렬에서도_맨_위다() {
        // 사용자 보고(2026-08-13): 내림차순으로 정렬하면 `..`가 목록 바닥으로 내려가
        // 위로 올라가려면 매번 끝까지 스크롤해야 했다
        let mut v = view(vec![
            (entry("..", true, 0, 0), "파일 폴더"),
            (entry("docs", true, 0, 0), "파일 폴더"),
            (entry("zzz", true, 0, 0), "파일 폴더"),
            (entry("a.txt", false, 10, 5), "텍스트"),
        ]);
        assert_eq!(names(&v)[0], "..", "이름 오름차순에서 첫 줄이 아니다");

        // 같은 열을 다시 누르면 내림차순 — 그래도 `..`는 그대로 맨 위다
        v.apply_sort(SortKey::Name);
        assert_eq!(names(&v), vec!["..", "zzz", "docs", "a.txt"]);

        // 다른 열로 정렬해도, 그 열을 뒤집어도 마찬가지다
        for key in [SortKey::Size, SortKey::Type, SortKey::Modified] {
            v.apply_sort(key);
            assert_eq!(names(&v)[0], "..", "{key:?} 오름차순에서 밀려났다");
            v.apply_sort(key);
            assert_eq!(names(&v)[0], "..", "{key:?} 내림차순에서 밀려났다");
        }
    }

    #[test]
    fn 상위_이동_줄은_세지도_고르지도_않는다() {
        // 로컬 목록의 `..`는 이 폴더에 든 것이 아니다 — 개수에서 빠지고, 골라도
        // 셸 메뉴·올리기·끌기의 대상이 되지 않는다(위 폴더째 지우거나 옮기게 된다)
        let mut v = view(vec![
            (entry("..", true, 0, 0), "파일 폴더"),
            (entry("docs", true, 0, 0), "파일 폴더"),
            (entry("a.txt", false, 10, 0), "텍스트"),
        ]);
        assert_eq!(names(&v), vec!["..", "docs", "a.txt"]);
        assert_eq!(v.counts(), (1, 1), "상위 이동 줄까지 세고 있다");

        v.dir = PathBuf::from(r"C:\Users\Public");
        v.selection.insert(0);
        assert!(
            v.selected_paths().is_empty(),
            "셸 메뉴 대상에 `..`가 들었다"
        );
        assert!(v.selected_local().is_empty(), "올리기 대상에 `..`가 들었다");
        assert!(v.drag_items(0, None).is_empty(), "끌기에 `..`가 실렸다");

        // 함께 고른 실제 항목은 그대로 대상이다
        v.selection.insert(2);
        assert_eq!(
            v.selected_paths(),
            vec![PathBuf::from(r"C:\Users\Public\a.txt")]
        );
        assert_eq!(v.drag_items(0, None).len(), 1);
    }

    #[test]
    fn 모델을_읽어도_항목이_복사되지_않는다() {
        // 10만 항목에서 매 프레임 복사하면 그 자체로 프레임이 무너진다 (NFR-3).
        // 크기와 무관하게 "같은 자리를 가리키는가"로 확인한다
        let v = view(vec![
            (entry("a.txt", false, 1, 0), "텍스트"),
            (entry("b.txt", false, 2, 0), "텍스트"),
        ]);
        let ListModel::Local(rows) = v.model() else {
            panic!("로컬 모델이어야 한다");
        };
        let first = rows.as_ptr();
        let ListModel::Local(again) = v.model() else {
            panic!("로컬 모델이어야 한다");
        };
        assert_eq!(first, again.as_ptr(), "모델을 읽을 때마다 복사되고 있다");
    }

    #[test]
    fn 공개_표면은_그대로다() {
        // T9가 배선하기 전까지 `ui::panel`이 이 다섯 가지를 종전 시그니처로 부른다
        let mut icons = IconCache::new();
        let mut v = FileListView::new();
        v.set_entries(PathBuf::from(r"C:\테스트"), Vec::new(), &mut icons);
        assert!(v.entry_at(0).is_none());
        assert!(v.selected_paths().is_empty());
        assert_eq!(v.counts(), (0, 0));
        assert_eq!(v.len(), 0);
    }

    #[test]
    fn 숨김을_끄면_목록에서_빠지고_개수도_거른_뒤_기준이다() {
        use windows::Win32::Storage::FileSystem::{FILE_ATTRIBUTE_HIDDEN, FILE_ATTRIBUTE_SYSTEM};
        let mut icons = IconCache::new();
        let mut hidden_dir = entry("숨긴폴더", true, 0, 0);
        hidden_dir.attributes = FILE_ATTRIBUTE_HIDDEN.0;
        let mut system_file = entry("pagefile.sys", false, 0, 0);
        system_file.attributes = FILE_ATTRIBUTE_SYSTEM.0;
        let rows = || {
            vec![
                entry("보통.txt", false, 0, 0),
                entry("폴더", true, 0, 0),
                hidden_dir.clone(),
                system_file.clone(),
            ]
        };

        // 켜져 있으면 전부 보인다 — 지금까지의 동작이다
        let mut v = FileListView::new();
        v.set_entries(PathBuf::from(r"C:\문서"), rows(), &mut icons);
        assert_eq!(v.len(), 4);
        assert_eq!(v.counts(), (2, 2));

        // 끄면 숨김·시스템이 빠지고 **개수도 거른 뒤 기준**이다
        let mut v = FileListView::new();
        assert!(v.set_show_hidden(false), "값이 바뀌었는데 알리지 않았다");
        v.set_entries(PathBuf::from(r"C:\문서"), rows(), &mut icons);
        assert_eq!(names(&v), vec!["폴더", "보통.txt"]);
        assert_eq!(v.counts(), (1, 1));

        // 같은 값을 다시 주면 바뀌지 않았다고 알린다 — 매 프레임 폴더를 다시 읽으면 안 된다
        assert!(!v.set_show_hidden(false), "안 바뀌었는데 바뀌었다고 한다");
    }

    #[test]
    fn 숨김을_끄면_그_항목은_선택에서도_빠진다() {
        // plan Edge Case — 보이지 않는 항목이 선택돼 있으면 삭제·복사가 예상 밖으로 돈다
        use windows::Win32::Storage::FileSystem::FILE_ATTRIBUTE_HIDDEN;
        let mut icons = IconCache::new();
        let mut hidden = entry("숨김.txt", false, 0, 0);
        hidden.attributes = FILE_ATTRIBUTE_HIDDEN.0;
        let dir = PathBuf::from(r"C:\문서");

        let mut v = FileListView::new();
        v.set_entries(
            dir.clone(),
            vec![entry("보통.txt", false, 0, 0), hidden.clone()],
            &mut icons,
        );
        v.selection = (0..2).collect();

        // 같은 폴더를 숨김 없이 다시 읽는다 — 선택은 이름으로 되살아나지만 없는 것은 못 살아난다
        v.set_show_hidden(false);
        v.set_entries(
            dir,
            vec![entry("보통.txt", false, 0, 0), hidden],
            &mut icons,
        );
        assert_eq!(v.selected_paths(), vec![PathBuf::from(r"C:\문서\보통.txt")]);
    }

    #[test]
    fn 확장자를_꺼도_경로와_정렬은_원본_이름을_쓴다() {
        // **규약: `display_name()`은 그리는 자리에만** (D7). 경로를 잘린 이름으로 만들면
        // 확장자를 끈 순간 파일 실행·셸 메뉴·끌어놓기가 통째로 깨진다
        let mut v = view(vec![
            (entry("파일10.txt", false, 0, 0), "텍스트"),
            (entry("파일2.txt", false, 0, 0), "텍스트"),
        ]);
        v.set_show_extensions(false);
        v.dir = PathBuf::from(r"C:\문서");
        v.selection = (0..2).collect();

        // 셸 메뉴·삭제·복사가 받는 경로는 확장자를 포함한 원본이다
        assert_eq!(
            v.selected_paths(),
            vec![
                PathBuf::from(r"C:\문서\파일2.txt"),
                PathBuf::from(r"C:\문서\파일10.txt"),
            ]
        );
        // 정렬도 원본 기준 — 숫자 인지 정렬이 확장자를 뗀 뒤 무너지면 순서가 달라진다
        assert_eq!(names(&v), vec!["파일2.txt", "파일10.txt"]);
        // 끌어놓기가 싣는 경로도 같다
        let items = v.drag_items(0, None);
        assert!(
            matches!(&items[0], DragItem::Local { path, .. } if path.ends_with("파일2.txt")),
            "끌어놓기 경로가 원본 이름이 아니다: {items:?}"
        );
    }
}
