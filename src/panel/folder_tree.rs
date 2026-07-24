//! 폴더 트리 — SysTreeView32 지연 확장 래퍼 (FR-9)
//!
//! 드라이브 루트만 미리 나열하고, 노드 확장 시 해당 폴더의 하위 1단계만
//! 워커 스레드로 열거한다 (plan D14 — UI 무정지). 열거 완료는 패널 창의
//! WM_APP_ENUM_DONE로 통지되며, 데이터는 트리 전용 채널로 받는다.
//! 목록→트리 역방향 동기화는 하지 않는다 (D14 — 단방향).
use crate::app::theme;
use crate::fs::enumerate::{EnumOutcome, EnumResult, FileEntry, spawn_enumerate};
use crate::fs::icons::IconCache;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender, channel};
use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::Storage::FileSystem::GetLogicalDrives;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Controls::{
    HTREEITEM, NMTREEVIEWW, SetWindowTheme, TVC_UNKNOWN, TVE_EXPAND, TVI_LAST, TVI_ROOT,
    TVIF_CHILDREN, TVIF_IMAGE, TVIF_SELECTEDIMAGE, TVIF_TEXT, TVINSERTSTRUCTW, TVITEMEXW_CHILDREN,
    TVITEMW, TVM_EXPAND, TVM_INSERTITEMW, TVM_SETBKCOLOR, TVM_SETIMAGELIST, TVM_SETITEMW,
    TVM_SETLINECOLOR, TVM_SETTEXTCOLOR, TVS_HASBUTTONS, TVS_HASLINES, TVS_LINESATROOT,
    TVS_SHOWSELALWAYS, TVSIL_NORMAL, WC_TREEVIEWW,
};
use windows::Win32::UI::Shell::StrCmpLogicalW;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, MoveWindow, SW_HIDE, SW_SHOW, SendMessageW, ShowWindow, WINDOW_STYLE,
    WS_CHILD, WS_CLIPSIBLINGS, WS_EX_CLIENTEDGE, WS_TABSTOP,
};
use windows::core::{HSTRING, PCWSTR, Result, w};

/// 트리 고정 폭 (plan T1 Design — 조절은 v2)
pub const TREE_WIDTH: i32 = 200;

/// 폴더 트리 상태 — 패널이 소유하고 배치·통지를 배선한다
pub struct FolderTree {
    hwnd: HWND,
    /// 폴더 아이콘 인덱스 (IconCache 재사용 — 트리 전용 아이콘 처리 없음, plan T1 Design ④)
    dir_icon: i32,
    /// 트리 항목 → 절대 경로. v1은 항목을 삭제하지 않으므로 누적만 한다
    paths: HashMap<isize, PathBuf>,
    /// 하위 1단계 열거를 이미 마친 항목 (재확장 시 재열거 안 함)
    populated: HashSet<isize>,
    /// 열거 세대 → 대상 항목 — 동시 다발 확장을 세대로 구분
    pending: HashMap<u64, isize>,
    generation: u64,
    visible: bool,
    tx: Sender<EnumResult>,
    rx: Receiver<EnumResult>,
}

impl FolderTree {
    /// 숨김 상태로 생성하고 드라이브 루트를 채운다 (표시는 toggle)
    pub fn create(parent: HWND) -> Result<FolderTree> {
        // 안전성: 공용 컨트롤 자식 창 생성 — 부모에 귀속되어 함께 파괴된다
        let hwnd = unsafe {
            let instance = GetModuleHandleW(None)?;
            CreateWindowExW(
                WS_EX_CLIENTEDGE,
                WC_TREEVIEWW,
                None,
                WS_CHILD
                    | WS_CLIPSIBLINGS
                    | WS_TABSTOP
                    | WINDOW_STYLE(
                        TVS_HASBUTTONS | TVS_HASLINES | TVS_LINESATROOT | TVS_SHOWSELALWAYS,
                    ),
                0,
                0,
                0,
                0,
                Some(parent),
                None,
                Some(instance.into()),
                None,
            )?
        };

        // 시스템 공유 이미지 리스트 + 폴더 아이콘 (IconCache 재사용 — 인스턴스는 보관 불필요)
        let mut icons = IconCache::new();
        let dir_icon = icons.icon_index("", true, None);
        // 안전성: 유효한 트리 핸들에 표준 초기화 메시지 + 고정 다크 색 (plan T4)
        unsafe {
            let _ = SetWindowTheme(hwnd, w!("DarkMode_Explorer"), PCWSTR::null());
            SendMessageW(
                hwnd,
                TVM_SETBKCOLOR,
                Some(WPARAM(0)),
                Some(LPARAM(theme::SURFACE_BG.0 as isize)),
            );
            SendMessageW(
                hwnd,
                TVM_SETTEXTCOLOR,
                Some(WPARAM(0)),
                Some(LPARAM(theme::TEXT.0 as isize)),
            );
            SendMessageW(
                hwnd,
                TVM_SETLINECOLOR,
                Some(WPARAM(0)),
                Some(LPARAM(theme::TREE_LINE.0 as isize)),
            );
            SendMessageW(
                hwnd,
                TVM_SETIMAGELIST,
                Some(WPARAM(TVSIL_NORMAL as usize)),
                Some(LPARAM(icons.himl().0)),
            );
        }

        let (tx, rx) = channel();
        let mut tree = FolderTree {
            hwnd,
            dir_icon,
            paths: HashMap::new(),
            populated: HashSet::new(),
            pending: HashMap::new(),
            generation: 0,
            visible: false,
            tx,
            rx,
        };
        tree.populate_drives();
        Ok(tree)
    }

    pub fn hwnd(&self) -> HWND {
        self.hwnd
    }

    pub fn visible(&self) -> bool {
        self.visible
    }

    /// 표시/숨김 토글 — 재배치는 호출부(relayout) 몫
    pub fn toggle(&mut self) {
        self.visible = !self.visible;
        // 안전성: 유효한 자기 핸들 표시 상태 변경
        unsafe {
            let _ = ShowWindow(self.hwnd, if self.visible { SW_SHOW } else { SW_HIDE });
        }
    }

    pub fn resize(&self, x: i32, y: i32, w: i32, h: i32) {
        // 안전성: 자식 창 이동 — 유효 핸들
        unsafe {
            let _ = MoveWindow(self.hwnd, x, y, w.max(0), h.max(0), true);
        }
    }

    /// TVN_ITEMEXPANDING — true를 반환하면 확장을 보류한다(열거 완료 후 apply_expand로 재개).
    /// notify는 열거 완료 통지를 받을 패널 창
    pub fn on_expanding(&mut self, nmtv: &NMTREEVIEWW, notify: HWND) -> bool {
        if nmtv.action.0 != TVE_EXPAND.0 {
            return false; // 접기 등은 그대로 진행
        }
        let item = nmtv.itemNew.hItem.0;
        if self.populated.contains(&item) {
            return false; // 이미 채워짐 — 즉시 확장 허용
        }
        if self.pending.values().any(|&h| h == item) {
            return true; // 열거 진행 중 — 중복 spawn 방지, 확장은 계속 보류
        }
        let Some(path) = self.paths.get(&item).cloned() else {
            return false;
        };
        self.generation += 1;
        self.pending.insert(self.generation, item);
        spawn_enumerate(path, self.generation, self.tx.clone(), notify);
        true
    }

    /// TVN_SELCHANGED — 사용자 조작(마우스·키보드)일 때만 대상 경로 반환.
    /// 프로그램적 선택(TVC_UNKNOWN)은 무시해 삽입·복원 중 원치 않는 탐색을 막는다
    pub fn on_sel_changed(&self, nmtv: &NMTREEVIEWW) -> Option<PathBuf> {
        if nmtv.action.0 == TVC_UNKNOWN.0 {
            return None;
        }
        self.paths.get(&nmtv.itemNew.hItem.0).cloned()
    }

    /// 열거 완료 통지 처리 — 채널을 비우고 하위 폴더 항목을 삽입한다.
    /// 반환값은 차용 해제 후 apply_expand로 확장할 항목들
    /// (TVM_EXPAND는 TVN_ITEMEXPANDING을 동기 재진입시키므로 차용 중 호출 금지)
    pub fn on_enum_done(&mut self) -> Vec<isize> {
        let mut to_expand = Vec::new();
        while let Ok(r) = self.rx.try_recv() {
            let Some(item) = self.pending.remove(&r.generation) else {
                continue;
            };
            self.populated.insert(item);
            match r.outcome {
                EnumOutcome::Ok(entries) => {
                    let mut dirs: Vec<&FileEntry> = entries.iter().filter(|e| e.is_dir).collect();
                    dirs.sort_by(|a, b| logical_name_cmp(&a.name, &b.name));
                    if dirs.is_empty() {
                        self.set_no_children(item);
                    } else {
                        let parent_path = self.paths.get(&item).cloned().unwrap_or_default();
                        for d in dirs {
                            let name = d.name_string();
                            self.insert_item(HTREEITEM(item), &name, parent_path.join(&name));
                        }
                        to_expand.push(item);
                    }
                }
                // 접근 거부·삭제·준비 안 된 드라이브 — 하위 없음 표시로 무해 처리 (T1 Edge)
                _ => self.set_no_children(item),
            }
        }
        to_expand
    }

    /// 드라이브 루트 나열 (A:~Z: 비트마스크 순 — 알파벳 정렬과 동일)
    fn populate_drives(&mut self) {
        // 안전성: 인자 없는 조회 — 비트마스크만 반환
        let mask = unsafe { GetLogicalDrives() };
        for i in 0..26u32 {
            if mask & (1 << i) != 0 {
                let letter = (b'A' + i as u8) as char;
                let path = format!("{letter}:\\");
                self.insert_item(TVI_ROOT, &path, PathBuf::from(&path));
            }
        }
    }

    /// 항목 삽입 — 확장 버튼을 위해 cChildren=1로 시작(실제 하위 유무는 확장 시 판정)
    fn insert_item(&mut self, parent: HTREEITEM, text: &str, path: PathBuf) {
        let wide = HSTRING::from(text);
        let mut ins = TVINSERTSTRUCTW {
            hParent: parent,
            hInsertAfter: TVI_LAST,
            ..Default::default()
        };
        ins.Anonymous.item = TVITEMW {
            mask: TVIF_TEXT | TVIF_IMAGE | TVIF_SELECTEDIMAGE | TVIF_CHILDREN,
            pszText: windows::core::PWSTR(wide.as_ptr() as *mut u16),
            iImage: self.dir_icon,
            iSelectedImage: self.dir_icon,
            cChildren: TVITEMEXW_CHILDREN(1),
            ..Default::default()
        };
        // 안전성: ins·wide는 호출 동안 살아있는 스택/지역 소유 — 트리가 텍스트를 복사한다
        let h = unsafe {
            SendMessageW(
                self.hwnd,
                TVM_INSERTITEMW,
                Some(WPARAM(0)),
                Some(LPARAM(&ins as *const _ as isize)),
            )
        };
        if h.0 != 0 {
            self.paths.insert(h.0, path);
        }
    }

    /// 하위 없음 표시 — 확장 버튼 제거
    fn set_no_children(&self, item: isize) {
        let tvi = TVITEMW {
            mask: TVIF_CHILDREN,
            hItem: HTREEITEM(item),
            cChildren: TVITEMEXW_CHILDREN(0),
            ..Default::default()
        };
        // 안전성: tvi는 스택 소유 — 트리가 즉시 반영
        unsafe {
            SendMessageW(
                self.hwnd,
                TVM_SETITEMW,
                Some(WPARAM(0)),
                Some(LPARAM(&tvi as *const _ as isize)),
            );
        }
    }
}

/// 보류했던 확장을 실행한다 — 반드시 패널 상태 RefCell 차용을 놓은 뒤 호출
/// (TVM_EXPAND의 동기 TVN_ITEMEXPANDING 재진입이 차용과 겹치지 않게 — apply_item_count와 동일 계약)
pub fn apply_expand(tree: HWND, items: &[isize]) {
    for &item in items {
        // 안전성: 유효한 트리 핸들에 표준 확장 메시지
        unsafe {
            SendMessageW(
                tree,
                TVM_EXPAND,
                Some(WPARAM(TVE_EXPAND.0 as usize)),
                Some(LPARAM(item)),
            );
        }
    }
}

/// StrCmpLogicalW 래퍼 — 널 종단 UTF-16 이름 비교 (file_list와 동일 패턴,
/// 공통화는 3회 반복 문턱 미달로 보류)
fn logical_name_cmp(a: &[u16], b: &[u16]) -> std::cmp::Ordering {
    // 안전성: 두 버퍼 모두 널 종단 보장(FileEntry 불변식)
    let r = unsafe { StrCmpLogicalW(PCWSTR(a.as_ptr()), PCWSTR(b.as_ptr())) };
    r.cmp(&0)
}
