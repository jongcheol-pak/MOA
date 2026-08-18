//! 파일 목록 — SysListView32 가상 모드(LVS_OWNERDATA) 래퍼 (FR-4·FR-5)
use crate::app::theme;
use crate::fs::enumerate::FileEntry;
use crate::fs::icons::IconCache;
use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    CreateSolidBrush, DT_CENTER, DT_END_ELLIPSIS, DT_LEFT, DT_RIGHT, DT_SINGLELINE, DT_VCENTER,
    DeleteObject, DrawTextW, FillRect, HDC, HGDIOBJ, SelectObject, SetBkMode, SetTextColor,
    TRANSPARENT,
};
use windows::Win32::Storage::FileSystem::{FILE_ATTRIBUTE_HIDDEN, FILE_ATTRIBUTE_SYSTEM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Time::{FileTimeToSystemTime, SystemTimeToTzSpecificLocalTime};
use windows::Win32::UI::Controls::{
    CDDS_ITEMPREPAINT, CDDS_POSTPAINT, CDDS_PREPAINT, CDIS_HOT, CDIS_SELECTED, CDRF_DODEFAULT,
    CDRF_NOTIFYITEMDRAW, CDRF_NOTIFYPOSTPAINT, CDRF_SKIPDEFAULT, HDF_CENTER, HDF_JUSTIFYMASK,
    HDF_RIGHT, HDI_FORMAT, HDI_TEXT, HDITEMW, HDM_GETITEMCOUNT, HDM_GETITEMRECT, HDM_GETITEMW,
    LVCF_FMT, LVCF_TEXT, LVCF_WIDTH, LVCFMT_LEFT, LVCFMT_RIGHT, LVCOLUMNW, LVIF_IMAGE, LVIF_TEXT,
    LVM_GETHEADER, LVM_GETNEXTITEM, LVM_INSERTCOLUMNW, LVM_SETBKCOLOR,
    LVM_SETEXTENDEDLISTVIEWSTYLE, LVM_SETIMAGELIST, LVM_SETITEMCOUNT, LVM_SETTEXTBKCOLOR,
    LVM_SETTEXTCOLOR, LVNI_SELECTED, LVS_EX_DOUBLEBUFFER, LVS_EX_FULLROWSELECT, LVS_OWNERDATA,
    LVS_REPORT, LVS_SHAREIMAGELISTS, LVS_SHOWSELALWAYS, LVSIL_SMALL, NM_CUSTOMDRAW, NMCUSTOMDRAW,
    NMHDR, NMLVDISPINFOW, SetWindowTheme, WC_LISTVIEWW,
};
use windows::Win32::UI::Shell::{DefSubclassProc, SetWindowSubclass, StrCmpLogicalW};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, GetClientRect, MoveWindow, SendMessageW, WINDOW_STYLE, WM_GETFONT, WM_NOTIFY,
    WS_CHILD, WS_CLIPSIBLINGS, WS_EX_CLIENTEDGE, WS_TABSTOP, WS_VISIBLE,
};
use windows::core::{HSTRING, PCWSTR, Result, w};

/// 정렬 열
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SortKey {
    Name,
    Size,
    Type,
    Modified,
}

impl SortKey {
    fn from_column(col: i32) -> Option<SortKey> {
        match col {
            0 => Some(SortKey::Name),
            1 => Some(SortKey::Size),
            2 => Some(SortKey::Type),
            3 => Some(SortKey::Modified),
            _ => None,
        }
    }
}

/// 가상 리스트뷰 + 데이터 모델 (항목은 이 구조체가 소유, 리스트뷰는 표시만)
pub struct FileList {
    hwnd: HWND,
    entries: Vec<FileEntry>,
    sort_key: SortKey,
    ascending: bool,
    icons: IconCache,
    /// 현재 폴더 경로 문자열 — 개별 아이콘 조회(전체 경로) 조립용
    dir_path: String,
    /// 종류 열 캐시 (entries와 같은 인덱스) — 정렬·표시에 공용
    type_names: Vec<String>,
}

impl FileList {
    pub fn create(parent: HWND) -> Result<FileList> {
        // 안전성: 공용 컨트롤 자식 창 생성 — 부모에 귀속되어 함께 파괴된다
        let hwnd = unsafe {
            let instance = GetModuleHandleW(None)?;
            CreateWindowExW(
                WS_EX_CLIENTEDGE,
                WC_LISTVIEWW,
                None,
                WS_CHILD
                    | WS_VISIBLE
                    | WS_CLIPSIBLINGS
                    | WS_TABSTOP
                    | WINDOW_STYLE(
                        LVS_REPORT | LVS_OWNERDATA | LVS_SHOWSELALWAYS | LVS_SHAREIMAGELISTS,
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

        let icons = IconCache::new();
        let list = FileList {
            hwnd,
            entries: Vec::new(),
            sort_key: SortKey::Name,
            ascending: true,
            icons,
            dir_path: String::new(),
            type_names: Vec::new(),
        };

        // 안전성: 유효한 리스트뷰 핸들에 대한 표준 초기화 메시지
        unsafe {
            // 고정 다크 (plan T3) — 목록 본체 배경·글자. 헤더는 아래 서브클래스가 그린다
            let _ = SetWindowTheme(list.hwnd, w!("DarkMode_Explorer"), PCWSTR::null());
            SendMessageW(
                list.hwnd,
                LVM_SETBKCOLOR,
                Some(WPARAM(0)),
                Some(LPARAM(theme::SURFACE_BG.0 as isize)),
            );
            SendMessageW(
                list.hwnd,
                LVM_SETTEXTCOLOR,
                Some(WPARAM(0)),
                Some(LPARAM(theme::TEXT.0 as isize)),
            );
            SendMessageW(
                list.hwnd,
                LVM_SETTEXTBKCOLOR,
                Some(WPARAM(0)),
                Some(LPARAM(theme::SURFACE_BG.0 as isize)),
            );
            // 헤더(SysHeader32) 다크: 헤더 창을 다크로 인식시킨 뒤(부속 UI용), 배경·글자는
            // ListView 서브클래스(list_dark_proc)가 커스텀드로우로 직접 그린다.
            // SetWindowTheme("ItemsView")로 시스템이 다크 배경을 그리게 하는 방식은 이 환경에서
            // 배경이 밝게 남는 것으로 실측 확인돼 쓰지 않는다.
            theme::allow_dark_for_window(list.header_hwnd());
            let _ = SetWindowSubclass(list.hwnd, Some(list_dark_proc), 1, 0);
            SendMessageW(
                list.hwnd,
                LVM_SETEXTENDEDLISTVIEWSTYLE,
                Some(WPARAM(
                    (LVS_EX_FULLROWSELECT | LVS_EX_DOUBLEBUFFER) as usize,
                )),
                Some(LPARAM(
                    (LVS_EX_FULLROWSELECT | LVS_EX_DOUBLEBUFFER) as isize,
                )),
            );
            SendMessageW(
                list.hwnd,
                LVM_SETIMAGELIST,
                Some(WPARAM(LVSIL_SMALL as usize)),
                Some(LPARAM(list.icons.himl().0 as isize)),
            );
            list.insert_column(0, "이름", 320, false);
            list.insert_column(1, "크기", 90, true);
            list.insert_column(2, "종류", 150, false);
            list.insert_column(3, "수정한 날짜", 140, false);
        }
        Ok(list)
    }

    /// 안전성 주의: 유효한 리스트뷰에만 호출 (create 내부 전용)
    unsafe fn insert_column(&self, index: i32, title: &str, width: i32, right: bool) {
        let text = HSTRING::from(title);
        let mut col = LVCOLUMNW {
            mask: LVCF_TEXT | LVCF_WIDTH | LVCF_FMT,
            fmt: if right { LVCFMT_RIGHT } else { LVCFMT_LEFT },
            cx: width,
            pszText: windows::core::PWSTR(text.as_ptr() as *mut u16),
            ..Default::default()
        };
        // 안전성: col·text는 이 호출 동안 살아있는 스택/지역 소유
        unsafe {
            SendMessageW(
                self.hwnd,
                LVM_INSERTCOLUMNW,
                Some(WPARAM(index as usize)),
                Some(LPARAM(&mut col as *mut _ as isize)),
            );
        }
    }

    pub fn hwnd(&self) -> HWND {
        self.hwnd
    }

    /// 목록 헤더(SysHeader32) 핸들 — 다크 테마 적용 대상(create 내부에서 사용)
    pub fn header_hwnd(&self) -> HWND {
        // 안전성: 유효한 리스트뷰에 표준 헤더 조회 메시지
        let h = unsafe { SendMessageW(self.hwnd, LVM_GETHEADER, None, None) };
        HWND(h.0 as *mut core::ffi::c_void)
    }

    /// 새 폴더 내용으로 교체 (정렬 포함).
    /// 카운트 반영은 호출부가 RefCell 차용 해제 후 `apply_item_count`로 수행한다
    /// (LVM_SETITEMCOUNT의 동기 LVN_GETDISPINFO 재진입이 차용과 겹치지 않게 — quality 리뷰 M2)
    pub fn set_entries(&mut self, dir_path: String, entries: Vec<FileEntry>) {
        self.dir_path = dir_path;
        self.entries = entries;
        self.rebuild_type_cache();
        self.resort();
    }

    /// 데이터만 비운다 — 카운트 반영은 set_entries와 동일하게 호출부 몫
    pub fn clear(&mut self) {
        self.entries.clear();
        self.type_names.clear();
    }

    /// 현재 항목 수 — 차용 해제 후 카운트 적용용
    pub fn item_count(&self) -> usize {
        self.entries.len()
    }

    pub fn entry_at(&self, index: usize) -> Option<&FileEntry> {
        self.entries.get(index)
    }

    /// 선택된 항목 인덱스 목록 — 컨텍스트 메뉴 대상 수집용 (FR-8)
    pub fn selected_indices(&self) -> Vec<usize> {
        let mut out = Vec::new();
        let mut current: isize = -1;
        loop {
            // 안전성: 유효한 리스트뷰 핸들에 표준 선택 순회 메시지
            let next = unsafe {
                SendMessageW(
                    self.hwnd,
                    LVM_GETNEXTITEM,
                    Some(WPARAM(current as usize)),
                    Some(LPARAM(LVNI_SELECTED as isize)),
                )
            };
            if next.0 < 0 {
                return out;
            }
            current = next.0;
            out.push(next.0 as usize);
        }
    }

    /// 열 클릭 → 같은 열이면 방향 토글, 다른 열이면 해당 열 오름차순
    pub fn on_column_click(&mut self, col: i32) {
        let Some(key) = SortKey::from_column(col) else {
            return;
        };
        if self.sort_key == key {
            self.ascending = !self.ascending;
        } else {
            self.sort_key = key;
            self.ascending = true;
        }
        self.resort();
    }

    /// LVN_GETDISPINFO — 가상 항목 텍스트·아이콘 공급
    pub fn on_get_disp_info(&mut self, info: &mut NMLVDISPINFOW) {
        let index = info.item.iItem as usize;
        let Some(entry) = self.entries.get(index) else {
            return;
        };
        if (info.item.mask.0 & LVIF_IMAGE.0) != 0 && info.item.iSubItem == 0 {
            info.item.iImage = if entry.is_dir {
                // 폴더는 경로 조립 불필요 (m2 — 불필요한 할당 제거)
                self.icons.icon_index("", true, None)
            } else {
                let ext = entry.extension();
                let full = format!(
                    "{}\\{}",
                    self.dir_path.trim_end_matches('\\'),
                    entry.name_string()
                );
                self.icons.icon_index(&ext, false, Some(&full))
            };
        }
        if (info.item.mask.0 & LVIF_TEXT.0) != 0 {
            let text = match info.item.iSubItem {
                0 => entry.name_string(),
                1 => {
                    if entry.is_dir {
                        String::new()
                    } else {
                        format_size(entry.size)
                    }
                }
                2 => self.type_names.get(index).cloned().unwrap_or_default(),
                3 => format_filetime(entry.modified),
                _ => String::new(),
            };
            write_to_buffer(&text, info.item.pszText, info.item.cchTextMax);
        }
    }

    /// 부모 영역 변화에 맞춰 채움
    pub fn resize(&self, x: i32, y: i32, w: i32, h: i32) {
        // 안전성: 자식 창 이동 — 유효 핸들
        unsafe {
            let _ = MoveWindow(self.hwnd, x, y, w.max(0), h.max(0), true);
        }
    }

    fn rebuild_type_cache(&mut self) {
        // entries와 icons를 동시에 &mut로 쓸 수 없어 (확장자, 폴더 여부)를 먼저 수집한다
        let meta: Vec<(String, bool)> = self
            .entries
            .iter()
            .map(|e| (e.extension(), e.is_dir))
            .collect();
        self.type_names = meta
            .iter()
            .map(|(ext, is_dir)| self.icons.type_name(ext, *is_dir))
            .collect();
    }

    fn resort(&mut self) {
        let key = self.sort_key;
        let asc = self.ascending;
        // 종류 정렬은 캐시 문자열 기준 → (entry, type) 쌍으로 정렬 후 분리
        let mut pairs: Vec<(FileEntry, String)> = self
            .entries
            .drain(..)
            .zip(self.type_names.drain(..))
            .collect();
        pairs.sort_by(|(a, ta), (b, tb)| {
            let ord = compare_entries(a, ta, b, tb, key);
            if asc { ord } else { ord.reverse() }
        });
        for (e, t) in pairs {
            self.entries.push(e);
            self.type_names.push(t);
        }
    }
}

/// 가상 리스트뷰 카운트 갱신 — 반드시 패널 상태의 RefCell 차용을 놓은 뒤 호출한다
/// (동기 LVN_GETDISPINFO 재진입이 try_borrow_mut 실패로 표시 누락되는 것 방지)
/// 헤더 제목 좌우 여백 (시스템 헤더와 비슷한 들여쓰기)
const HEADER_TEXT_PAD: i32 = 6;

/// ListView 서브클래스 프로시저 — 헤더(SysHeader32) 커스텀드로우 다크.
/// 헤더는 NM_CUSTOMDRAW를 자기 부모(=이 ListView)로 보내며 ListView는 이를 패널로 forward하지
/// 않는다. 그래서 여기서 가로챈다. 시스템 테마로는 배경이 밝게 남아(실측), 여백·항목 배경·
/// 구분선·제목을 전부 직접 그리고 CDRF_SKIPDEFAULT로 기본 도색을 막는다.
unsafe extern "system" fn list_dark_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    _id: usize,
    _data: usize,
) -> LRESULT {
    if msg == WM_NOTIFY {
        // 안전성: WM_NOTIFY의 lparam은 OS가 채운 NMHDR 포인터 (처리 동안 유효)
        let hdr = unsafe { &*(lparam.0 as *const NMHDR) };
        if hdr.code == NM_CUSTOMDRAW {
            // 안전성: NM_CUSTOMDRAW 통지의 lparam은 NMCUSTOMDRAW로 확장 해석 가능
            let cd = unsafe { &*(lparam.0 as *const NMCUSTOMDRAW) };
            if cd.dwDrawStage == CDDS_PREPAINT {
                return LRESULT((CDRF_NOTIFYITEMDRAW | CDRF_NOTIFYPOSTPAINT) as isize);
            }
            if cd.dwDrawStage == CDDS_ITEMPREPAINT {
                // 안전성: 항목 배경·구분선·제목을 직접 그린다 (기본 도색은 SKIPDEFAULT로 차단)
                unsafe { draw_header_item(cd) };
                return LRESULT(CDRF_SKIPDEFAULT as isize);
            }
            if cd.dwDrawStage == CDDS_POSTPAINT {
                // 안전성: 마지막 열 오른쪽 여백을 덮는다 (모든 항목을 그린 뒤여야 기본 도색을 덮는다)
                unsafe { fill_header_gap(cd) };
                return LRESULT(CDRF_DODEFAULT as isize);
            }
        }
    }
    // 안전성: 그 외 메시지는 원래 ListView 프로시저로 위임
    unsafe { DefSubclassProc(hwnd, msg, wparam, lparam) }
}

/// 단색 채우기 — 브러시는 생성 즉시 해제
/// 안전성 주의: 유효한 DC에만 호출한다
unsafe fn fill_solid(hdc: HDC, rc: &RECT, color: COLORREF) {
    unsafe {
        let brush = CreateSolidBrush(color);
        FillRect(hdc, rc, brush);
        let _ = DeleteObject(brush.into());
    }
}

/// 마지막 열 오른쪽의 빈 여백을 다크로 채운다.
/// 이 여백은 항목이 아니라 헤더 기본 도색이 그리므로, 항목을 모두 그린 뒤(CDDS_POSTPAINT)
/// 덮어야 한다 — CDDS_PREPAINT에서 채우면 기본 도색이 다시 덮어 흰색으로 남는다(실측).
/// 안전성 주의: 커스텀드로우가 넘긴 유효 DC·헤더 핸들에만 호출한다 (list_dark_proc 전용)
unsafe fn fill_header_gap(cd: &NMCUSTOMDRAW) {
    let header = cd.hdr.hwndFrom;
    unsafe {
        let mut client = RECT::default();
        if GetClientRect(header, &mut client).is_err() {
            return;
        }
        // 열 순서를 바꿔도 안전하도록 모든 항목 rect의 오른쪽 끝 최댓값을 쓴다
        let count = SendMessageW(header, HDM_GETITEMCOUNT, None, None).0;
        let mut used_right = 0;
        for i in 0..count.max(0) {
            let mut rc = RECT::default();
            let ok = SendMessageW(
                header,
                HDM_GETITEMRECT,
                Some(WPARAM(i as usize)),
                Some(LPARAM(&mut rc as *mut _ as isize)),
            );
            if ok.0 != 0 {
                used_right = used_right.max(rc.right);
            }
        }
        if used_right < client.right {
            let gap = RECT {
                left: used_right,
                ..client
            };
            fill_solid(cd.hdc, &gap, theme::HEADER_BG);
        }
    }
}

/// 헤더 항목(열 하나)을 다크로 직접 그린다 — 배경·오른쪽 구분선·제목.
/// 정렬 화살표는 열에 설정하지 않으므로(HDF_SORTUP/DOWN 미사용) 그리지 않는다.
/// 안전성 주의: 커스텀드로우가 넘긴 유효 DC·헤더 핸들에만 호출한다 (list_dark_proc 전용)
unsafe fn draw_header_item(cd: &NMCUSTOMDRAW) {
    let header = cd.hdr.hwndFrom;
    let state = cd.uItemState.0;
    let bg = if (state & CDIS_SELECTED.0) != 0 {
        theme::CONTROL_ACTIVE
    } else if (state & CDIS_HOT.0) != 0 {
        theme::CONTROL_HOT
    } else {
        theme::HEADER_BG
    };
    unsafe {
        fill_solid(cd.hdc, &cd.rc, bg);
        let separator = RECT {
            left: cd.rc.right - 1,
            ..cd.rc
        };
        fill_solid(cd.hdc, &separator, theme::TREE_LINE);

        // 제목·정렬 형식은 헤더에서 읽는다 (SKIPDEFAULT라 시스템이 그려주지 않는다)
        let mut buf = [0u16; 128];
        let mut item = HDITEMW {
            mask: HDI_TEXT | HDI_FORMAT,
            pszText: windows::core::PWSTR(buf.as_mut_ptr()),
            cchTextMax: buf.len() as i32,
            ..Default::default()
        };
        let ok = SendMessageW(
            header,
            HDM_GETITEMW,
            Some(WPARAM(cd.dwItemSpec)),
            Some(LPARAM(&mut item as *mut _ as isize)),
        );
        if ok.0 == 0 {
            return;
        }
        let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
        let just = item.fmt.0 & HDF_JUSTIFYMASK.0;
        let align = if just == HDF_RIGHT.0 {
            DT_RIGHT
        } else if just == HDF_CENTER.0 {
            DT_CENTER
        } else {
            DT_LEFT
        };
        let mut rc = RECT {
            left: cd.rc.left + HEADER_TEXT_PAD,
            right: cd.rc.right - HEADER_TEXT_PAD,
            ..cd.rc
        };
        // 헤더 폰트를 선택해야 기본 도색과 같은 글꼴로 그려진다 (미설정이면 DC 폰트 유지)
        let font = SendMessageW(header, WM_GETFONT, None, None);
        let old_font =
            (font.0 != 0).then(|| SelectObject(cd.hdc, HGDIOBJ(font.0 as *mut core::ffi::c_void)));
        SetTextColor(cd.hdc, theme::HEADER_TEXT);
        SetBkMode(cd.hdc, TRANSPARENT);
        DrawTextW(
            cd.hdc,
            &mut buf[..len],
            &mut rc,
            align | DT_VCENTER | DT_SINGLELINE | DT_END_ELLIPSIS,
        );
        if let Some(old) = old_font {
            SelectObject(cd.hdc, old);
        }
    }
}

pub fn apply_item_count(list: HWND, count: usize) {
    // 안전성: 유효한 리스트뷰 핸들에 표준 메시지 — 전체 무효화로 재요청 유도
    unsafe {
        SendMessageW(list, LVM_SETITEMCOUNT, Some(WPARAM(count)), Some(LPARAM(0)));
    }
}

/// 정렬 비교 — 폴더 우선(D7), 이름은 탐색기와 동일한 숫자 인지 정렬.
/// `pub`인 이유: egui UI 계층(`ui::file_list`)이 같은 정렬 규칙을 쓰기 위해 (이식 plan part1 4-D).
///
/// 주의: 폴더 우선은 여기서 **방향과 무관하게** 결정된다 — 내림차순을 만들려고 반환값 전체를
/// `reverse()`하면 폴더 우선까지 뒤집힌다. 같은 종류끼리만 뒤집어야 한다(part1 D13)
pub fn compare_entries(
    a: &FileEntry,
    type_a: &str,
    b: &FileEntry,
    type_b: &str,
    key: SortKey,
) -> std::cmp::Ordering {
    compare_rows(a, type_a, b, type_b, key)
}

/// 목록 맨 위의 상위 이동 줄 이름 — 로컬·원격이 같은 값을 쓴다.
///
/// 만드는 곳(`ui::panel`)과 해석하는 곳(선택·개수·드래그 제외)이 갈라져 있어 한 자리에 둔다
pub const PARENT_ENTRY: &str = "..";

/// 목록 한 줄이 정렬·표시에 내주어야 하는 것 (plan T7).
///
/// 로컬(`fs::enumerate::FileEntry`)과 원격(`remote::types::RemoteEntry`)은 이름·시각을 담는
/// 방식이 서로 다르지만(널 종단 UTF-16 + FILETIME ↔ `String` + 유닉스 초), **정렬 규칙은
/// 한 벌이어야** 화면이 프로토콜마다 달라지지 않는다. 그 한 벌을 이 트레이트가 잇는다.
///
/// 렌더·선택·아이콘은 여기 두지 않는다 — 정렬과 표시 문자열에 필요한 것만이다.
/// 트레이트 객체가 아니라 **제네릭**으로 쓴다(10만 항목 정렬에 가상 호출을 넣지 않는다).
pub trait ListRow {
    /// 표시용 이름
    fn name(&self) -> String;

    /// 목록에 그릴 이름 — `show_extensions`가 꺼져 있으면 확장자를 뗀다 (FR-52).
    ///
    /// **`name()`을 대신하지 않는다**: 경로 조립(`dir.join(name)`)·정렬 키·선택 복원은
    /// 여전히 원래 이름을 써야 한다. 잘린 이름으로 경로를 만들면 파일 실행과 셸 메뉴가
    /// 통째로 깨진다 (D7). 그래서 **그리는 자리만** 이 메서드를 쓴다.
    ///
    /// 폴더·`..`·앞이 빈 이름(`.gitignore`)·끝이 점인 이름(`a.`)은 확장자 개념이
    /// 없어 그대로 둔다
    fn display_name(&self, show_extensions: bool) -> String {
        let name = self.name();
        if show_extensions || self.is_dir() || self.is_parent() {
            return name;
        }
        match name.rsplit_once('.') {
            // 앞이 비면 확장자가 아니라 이름 자체이고(`.gitignore`),
            // 뒤가 비면 잘라도 얻는 게 없다(`a.`)
            Some((stem, ext)) if !stem.is_empty() && !ext.is_empty() => stem.to_owned(),
            _ => name,
        }
    }

    /// **정렬용 널 종단 UTF-16 이름.**
    ///
    /// 이름 비교는 탐색기와 같은 `StrCmpLogicalW`로 한다("파일2" < "파일10") — 그 API가
    /// 널 종단 UTF-16을 받으므로 여기서 그 형태로 내준다. 로컬 항목은 이미 그 모양이라
    /// 빌려주고, 원격 항목만 그때 만든다
    fn name_sort_key(&self) -> std::borrow::Cow<'_, [u16]>;

    /// 상위 이동(`..`) 줄인가 — 실제 항목이 아니므로 **선택·개수·끌기에서 빠진다**.
    ///
    /// 기본 구현은 표시 이름을 만들어 견준다. 이름을 UTF-16으로 든 로컬 항목은 문자열을
    /// 새로 만들지 않도록 따로 구현한다(10만 항목 폴더에서 매 정렬마다 도는 자리다 — NFR-3)
    fn is_parent(&self) -> bool {
        self.name() == PARENT_ENTRY
    }

    fn is_dir(&self) -> bool;

    /// 숨김 항목인가 — 숨김을 보이는 설정이 꺼져 있으면 목록에서 빠진다 (FR-13).
    ///
    /// 로컬은 `FILE_ATTRIBUTE_HIDDEN`을, 원격은 이름이 `.`으로 시작하는지를 본다 —
    /// 유닉스 계열 서버에는 숨김 속성이 따로 없고 그 관례가 곧 규칙이다.
    ///
    /// **화면 문구를 여기 적지 않는다** — 설정 라벨은 `i18n` 카탈로그가 언어마다 정하므로
    /// 그것을 주석에 박으면 문구를 바꿀 때마다 이 자리가 조용히 낡는다
    fn is_hidden(&self) -> bool;

    /// 시스템 항목인가 — 시스템 항목을 보이는 설정이 꺼져 있으면 목록에서 빠진다 (FR-13).
    ///
    /// **숨김과 따로 묻는 이유**: 두 속성에 각자의 설정이 대응하고, 둘 다 붙은 항목
    /// (`pagefile.sys` 등)은 두 설정이 모두 켜져야 보인다. 원격은 언제나 `false`다 —
    /// 유닉스 계열 서버에 이 속성이 없어 흉내 낼 근거가 없다
    fn is_system(&self) -> bool;

    /// 목록에서 흐리게 그릴 항목인가 — 숨김이거나 시스템이면 보통 항목이 아니다 (FR-13).
    ///
    /// 거르기와 달리 **둘을 함께 본다**: 설정을 켜서 보이기로 한 항목이 어느 쪽이든
    /// 보통 항목과 구분돼야 하고, 그 판정이 그리는 자리마다 흩어지면 보기 모드에 따라
    /// 다르게 보인다
    fn is_dimmed(&self) -> bool {
        self.is_hidden() || self.is_system()
    }

    fn is_symlink(&self) -> bool;

    /// 심볼릭 링크가 가리키는 곳 — 이름 뒤에 `→ 대상`으로 붙는다 (FR-31)
    fn link_target(&self) -> Option<&str>;

    fn size(&self) -> u64;

    /// **정렬 전용 시각 키** — 1601-01-01부터의 100나노초 단위(FILETIME 눈금)다.
    ///
    /// 유닉스 초로 맞추지 않는 이유: 로컬 항목의 시각은 100나노초 정밀도라, 초 단위로 깎으면
    /// **같은 초에 만들어진 파일들의 차례가 바뀐다**(지금 동작이 달라진다). 표시용 변환은
    /// 각 목록이 자기 원본 값으로 한다
    fn modified_key(&self) -> u64;

    /// 소문자 확장자 (`""` = 없음/폴더)
    fn extension(&self) -> String;

    /// POSIX 권한 표기(`rwxr-xr-x`) — **원격 항목만** 갖는다 (FR-31).
    ///
    /// 로컬은 `None`이다: Windows 파일의 권한은 ACL이라 이 아홉 글자로 옮길 수 없고,
    /// 억지로 흉내 내면 화면이 실제 권한과 다른 말을 하게 된다
    fn permissions(&self) -> Option<String>;

    /// 소유자 — **원격 항목만** 갖는다. 서버가 이름을 주지 않으면 숫자 uid 그대로다
    fn owner(&self) -> Option<&str>;
}

/// 유닉스 초(UTC) → FILETIME 눈금. 두 목록이 같은 자로 정렬되게 맞춘다
pub fn unix_seconds_to_filetime(seconds: i64) -> u64 {
    const EPOCH_DIFFERENCE_SECONDS: i64 = 11_644_473_600;
    let since_1601 = seconds.saturating_add(EPOCH_DIFFERENCE_SECONDS).max(0);
    (since_1601 as u64).saturating_mul(10_000_000)
}

impl ListRow for FileEntry {
    fn name(&self) -> String {
        self.name_string()
    }

    fn name_sort_key(&self) -> std::borrow::Cow<'_, [u16]> {
        // 이미 널 종단 UTF-16이라 그대로 빌려준다
        std::borrow::Cow::Borrowed(&self.name)
    }

    /// 널 종단 UTF-16 `..`과 곧바로 견준다 — 표시용 문자열을 만들지 않는다
    fn is_parent(&self) -> bool {
        self.name == [b'.' as u16, b'.' as u16, 0]
    }

    fn is_dir(&self) -> bool {
        self.is_dir
    }

    fn is_hidden(&self) -> bool {
        self.attributes & FILE_ATTRIBUTE_HIDDEN.0 != 0
    }

    fn is_system(&self) -> bool {
        self.attributes & FILE_ATTRIBUTE_SYSTEM.0 != 0
    }

    fn is_symlink(&self) -> bool {
        // 로컬 목록은 링크를 따로 표시하지 않는다 (원격 전용 표시다 — FR-31)
        false
    }

    fn link_target(&self) -> Option<&str> {
        None
    }

    fn size(&self) -> u64 {
        self.size
    }

    fn modified_key(&self) -> u64 {
        self.modified
    }

    fn extension(&self) -> String {
        self.extension()
    }

    /// 로컬 파일의 권한은 ACL이라 아홉 글자 표기로 옮길 수 없다 — 흉내 내지 않는다
    fn permissions(&self) -> Option<String> {
        None
    }

    fn owner(&self) -> Option<&str> {
        None
    }
}

impl ListRow for crate::remote::types::RemoteEntry {
    fn name(&self) -> String {
        self.name.clone()
    }

    fn name_sort_key(&self) -> std::borrow::Cow<'_, [u16]> {
        let mut wide: Vec<u16> = self.name.encode_utf16().collect();
        wide.push(0);
        std::borrow::Cow::Owned(wide)
    }

    fn is_dir(&self) -> bool {
        self.is_dir
    }

    fn is_hidden(&self) -> bool {
        // 유닉스 계열 서버에는 숨김 속성이 없다 — `.`으로 시작하는 이름이 그 관례다.
        // `..`는 이름이 점으로 시작하지만 화면 장치이므로 걸러지면 안 된다
        !self.is_parent() && self.name.starts_with('.')
    }

    /// 원격에는 시스템 속성이 없다 — 서버가 주지 않는 것을 이름으로 흉내 내지 않는다
    fn is_system(&self) -> bool {
        false
    }

    fn is_symlink(&self) -> bool {
        self.is_symlink
    }

    fn link_target(&self) -> Option<&str> {
        self.link_target.as_deref()
    }

    fn size(&self) -> u64 {
        self.size
    }

    fn modified_key(&self) -> u64 {
        self.modified.map_or(0, unix_seconds_to_filetime)
    }

    fn extension(&self) -> String {
        self.extension()
    }

    /// 서버가 권한을 주지 않았으면 `None`이다 — 빈칸으로 보인다 (plan Edge Case).
    /// `0o777` 같은 기본값을 지어내지 않는 것은 서버가 하지 않은 말을 화면이 하지 않게 하기 위함이다
    fn permissions(&self) -> Option<String> {
        self.permissions_string()
    }

    fn owner(&self) -> Option<&str> {
        self.owner.as_deref()
    }
}

/// 정렬 비교 — 로컬·원격이 함께 쓰는 한 벌의 규칙. `compare_entries`가 이것에 위임한다.
///
/// 폴더 우선은 정렬 방향과 무관하게 결정된다 — 위 `compare_entries` 주석과 같은 주의다
pub fn compare_rows<R: ListRow + ?Sized>(
    a: &R,
    type_a: &str,
    b: &R,
    type_b: &str,
    key: SortKey,
) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    match (a.is_dir(), b.is_dir()) {
        (true, false) => return Ordering::Less,
        (false, true) => return Ordering::Greater,
        _ => {}
    }
    let by_name = || logical_name_cmp(&a.name_sort_key(), &b.name_sort_key());
    match key {
        SortKey::Name => by_name(),
        SortKey::Size => a.size().cmp(&b.size()).then_with(by_name),
        SortKey::Type => type_a.cmp(type_b).then_with(by_name),
        SortKey::Modified => a.modified_key().cmp(&b.modified_key()).then_with(by_name),
    }
}

/// StrCmpLogicalW 래퍼 — 널 종단 UTF-16 이름 비교 ("파일2" < "파일10")
fn logical_name_cmp(a: &[u16], b: &[u16]) -> std::cmp::Ordering {
    // 안전성: 두 버퍼 모두 널 종단이 보장된다 — 로컬은 `FileEntry.name` 불변식이,
    // 원격은 `ListRow::name_sort_key` 구현이 끝에 0을 붙여 만든다
    let r = unsafe { StrCmpLogicalW(PCWSTR(a.as_ptr()), PCWSTR(b.as_ptr())) };
    r.cmp(&0)
}

/// 크기 표시: 소수점 둘째자리 + KB·MB·GB 자동 승격.
/// `pub`인 이유: egui UI 계층(자세히·격자 보기와 전송 큐·상태 표시줄)이 같은 표시 규칙을
/// 쓰기 위해 — 복제하면 표시 형식이 두 벌로 갈라진다 (이식 plan part1 4-D)
///
/// **KB 아래 단위(B)는 쓰지 않는다** — 목록의 `크기` 열에 B·KB·MB·GB 넷이 섞이면
/// 자릿수만 보고 크기를 견줄 수 없다 (2026-08-18 사용자 결정)
pub fn format_size(bytes: u64) -> String {
    const STEP: f64 = 1024.0;
    const UNITS: [&str; 3] = ["KB", "MB", "GB"];
    let mut value = bytes as f64 / STEP;
    let mut unit = 0;
    // 승격 판정을 **반올림한 값**으로 한다 — 그러지 않으면 1,048,571바이트가
    // `1024.00 KB`로 나온다(표시값은 한 칸을 채웠는데 단위가 안 올라간다)
    while unit + 1 < UNITS.len() && (value * 100.0).round() / 100.0 >= STEP {
        value /= STEP;
        unit += 1;
    }
    // 0이 아닌데 `0.00 KB`면 빈 파일과 구분되지 않는다 — 최소 한 칸은 채운다
    if bytes > 0 && unit == 0 && value < 0.01 {
        value = 0.01;
    }
    format!("{value:.2} {}", UNITS[unit])
}

/// FILETIME(u64) → 로컬 "yyyy-MM-dd HH:mm".
/// `pub`인 이유는 `format_size`와 동일 (egui UI 계층과 표시 규칙 공유)
pub fn format_filetime(ft: u64) -> String {
    use windows::Win32::Foundation::FILETIME;
    let ft = FILETIME {
        dwLowDateTime: (ft & 0xffff_ffff) as u32,
        dwHighDateTime: (ft >> 32) as u32,
    };
    let mut st_utc = Default::default();
    let mut st_local = Default::default();
    // 안전성: 모든 인자 스택 소유 — 실패 시 빈 문자열 표시
    unsafe {
        if FileTimeToSystemTime(&ft, &mut st_utc).is_err()
            || SystemTimeToTzSpecificLocalTime(None, &st_utc, &mut st_local).is_err()
        {
            return String::new();
        }
    }
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}",
        st_local.wYear, st_local.wMonth, st_local.wDay, st_local.wHour, st_local.wMinute
    )
}

/// 가상 리스트뷰가 준 버퍼에 UTF-16 텍스트 복사 (널 종단 보장)
fn write_to_buffer(text: &str, buf: windows::core::PWSTR, cap: i32) {
    if buf.is_null() || cap <= 0 {
        return;
    }
    let wide: Vec<u16> = text.encode_utf16().collect();
    let n = wide.len().min(cap as usize - 1);
    // 안전성: cap은 리스트뷰가 보장하는 버퍼 크기 — n+1 ≤ cap 내에서만 쓴다
    unsafe {
        std::ptr::copy_nonoverlapping(wide.as_ptr(), buf.0, n);
        *buf.0.add(n) = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::remote::types::RemoteEntry;

    fn entry(name: &str, is_dir: bool, size: u64, modified: u64) -> FileEntry {
        let mut v: Vec<u16> = name.encode_utf16().collect();
        v.push(0);
        FileEntry {
            name: v,
            is_dir,
            size,
            modified,
            attributes: 0,
        }
    }

    #[test]
    fn 폴더가_항상_우선한다() {
        let d = entry("zzz", true, 0, 0);
        let f = entry("aaa.txt", false, 10, 0);
        assert_eq!(
            compare_entries(&d, "폴더", &f, "텍스트", SortKey::Name),
            std::cmp::Ordering::Less
        );
    }

    #[test]
    fn 이름은_숫자_인지_정렬이다() {
        let a = entry("파일2.txt", false, 0, 0);
        let b = entry("파일10.txt", false, 0, 0);
        assert_eq!(
            compare_entries(&a, "", &b, "", SortKey::Name),
            std::cmp::Ordering::Less
        );
    }

    #[test]
    fn 크기_정렬은_수치_비교다() {
        let small = entry("b.bin", false, 512, 0);
        let big = entry("a.bin", false, 2048, 0);
        assert_eq!(
            compare_entries(&small, "", &big, "", SortKey::Size),
            std::cmp::Ordering::Less
        );
    }

    #[test]
    fn 크기_표시는_소수_둘째자리에_단위를_올린다() {
        // 2026-08-18 사용자 결정 — 파일 목록·전송 큐·상태 표시줄이 같은 규칙을 쓴다
        assert_eq!(format_size(0), "0.00 KB");
        assert_eq!(format_size(512), "0.50 KB");
        assert_eq!(format_size(1024), "1.00 KB");
        assert_eq!(format_size(1_234), "1.21 KB");
        assert_eq!(format_size(1_234_567), "1.18 MB");
        assert_eq!(format_size(2 * 1024 * 1024 * 1024), "2.00 GB");
    }

    #[test]
    fn 크기_표시의_경계값() {
        // 0이 아닌데 `0.00 KB`면 빈 파일과 구분되지 않는다
        assert_eq!(format_size(1), "0.01 KB");
        assert_eq!(format_size(10), "0.01 KB");
        // 승격은 **반올림한 값**으로 판정한다 — 아니면 `1024.00 KB`가 나온다
        assert_eq!(format_size(1_048_570), "1023.99 KB");
        assert_eq!(format_size(1_048_571), "1.00 MB");
        // GB 위로는 올릴 단위가 없어 그대로 둔다(지수 표기로 새지 않는다)
        assert_eq!(format_size(u64::MAX), "17179869184.00 GB");
    }

    #[test]
    fn 날짜_정렬은_원시값_비교다() {
        let old = entry("old", false, 0, 100);
        let new = entry("new", false, 0, 200);
        assert_eq!(
            compare_entries(&old, "", &new, "", SortKey::Modified),
            std::cmp::Ordering::Less
        );
    }

    fn remote(name: &str, is_dir: bool, size: u64, modified: Option<i64>) -> RemoteEntry {
        RemoteEntry {
            name: name.to_owned(),
            is_dir,
            is_symlink: false,
            link_target: None,
            size,
            modified,
            mode: None,
            owner: None,
        }
    }

    #[test]
    fn 원격_항목도_같은_규칙으로_정렬된다() {
        // 화면이 프로토콜마다 다르게 줄 세우면 안 된다 (plan T7)
        let mut rows = [
            remote("파일10.txt", false, 10, Some(100)),
            remote("파일2.txt", false, 20, Some(200)),
            remote("zzz", true, 0, Some(0)),
        ];
        rows.sort_by(|a, b| compare_rows(a, "", b, "", SortKey::Name));
        let names: Vec<&str> = rows.iter().map(|r| r.name.as_str()).collect();
        // 폴더 우선 + 숫자 인지 정렬("파일2" < "파일10")
        assert_eq!(names, vec!["zzz", "파일2.txt", "파일10.txt"]);

        rows.sort_by(|a, b| compare_rows(a, "", b, "", SortKey::Size));
        let names: Vec<&str> = rows.iter().map(|r| r.name.as_str()).collect();
        assert_eq!(names, vec!["zzz", "파일10.txt", "파일2.txt"]);
    }

    #[test]
    fn 시각을_주지_않은_원격_항목이_가장_앞선다() {
        let mut rows = [
            remote("시각있음", false, 0, Some(1_700_000_000)),
            remote("시각없음", false, 0, None),
        ];
        rows.sort_by(|a, b| compare_rows(a, "", b, "", SortKey::Modified));
        assert_eq!(rows[0].name, "시각없음");
    }

    #[test]
    fn 정렬용_시각은_초로_깎이지_않는다() {
        // 유닉스 초로 맞추면 같은 초에 만들어진 로컬 파일들의 차례가 바뀐다
        let earlier = entry("a.txt", false, 0, 132_000_000_000_000_000);
        let later = entry("b.txt", false, 0, 132_000_000_005_000_000);
        assert_eq!(
            compare_entries(&earlier, "", &later, "", SortKey::Modified),
            std::cmp::Ordering::Less,
            "0.5초 차이가 같은 것으로 뭉개졌다"
        );
    }

    #[test]
    fn 유닉스_시각_변환은_기준점을_맞춘다() {
        // 1970-01-01 → FILETIME의 1601 기준 100나노초 단위
        assert_eq!(unix_seconds_to_filetime(0), 11_644_473_600 * 10_000_000);
        assert_eq!(
            unix_seconds_to_filetime(1),
            11_644_473_600 * 10_000_000 + 10_000_000
        );
        // 1970년 이전(음수)도 0 아래로 내려가지 않는다
        assert_eq!(unix_seconds_to_filetime(-11_644_473_600), 0);
        assert_eq!(unix_seconds_to_filetime(i64::MIN), 0);
    }

    #[test]
    fn 목록_한_줄의_표시값을_트레이트로_읽는다() {
        let local = entry("보고서.hwp", false, 1024, 7);
        assert_eq!(ListRow::name(&local), "보고서.hwp");
        assert_eq!(ListRow::extension(&local), "hwp");
        assert!(
            !local.is_symlink(),
            "로컬 목록은 링크를 따로 표시하지 않는다"
        );
        assert_eq!(ListRow::link_target(&local), None);
        assert_eq!(local.modified_key(), 7);

        let mut link = remote("current", false, 0, None);
        link.is_symlink = true;
        link.link_target = Some("releases/42".to_owned());
        assert!(link.is_symlink());
        assert_eq!(ListRow::link_target(&link), Some("releases/42"));
        // 폴더와 앞이 빈 이름은 확장자가 없다
        assert_eq!(ListRow::extension(&remote("폴더", true, 0, None)), "");
        assert_eq!(
            ListRow::extension(&remote(".gitignore", false, 0, None)),
            ""
        );
    }

    #[test]
    fn 이름이_같고_종류가_다르면_폴더가_앞선다() {
        let dir = remote("같은이름", true, 0, Some(1));
        let file = remote("같은이름", false, 0, Some(1));
        assert_eq!(
            compare_rows(&dir, "", &file, "", SortKey::Name),
            std::cmp::Ordering::Less
        );
        assert_eq!(
            compare_rows(&dir, "", &file, "", SortKey::Modified),
            std::cmp::Ordering::Less
        );
    }

    /// 속성이 붙은 로컬 항목 — 숨김 판정 시험용
    fn entry_with(name: &str, attributes: u32) -> FileEntry {
        let mut e = entry(name, false, 0, 0);
        e.attributes = attributes;
        e
    }

    #[test]
    fn 로컬_숨김과_시스템은_각자의_속성이다() {
        use windows::Win32::Storage::FileSystem::{FILE_ATTRIBUTE_HIDDEN, FILE_ATTRIBUTE_SYSTEM};
        let 보통 = entry_with("보통.txt", 0);
        assert!(!보통.is_hidden() && !보통.is_system());
        assert!(!보통.is_dimmed(), "보통 항목을 흐리게 그리려 한다");

        // 두 속성에 각자의 설정이 대응한다 — 한쪽 판정이 다른 쪽을 물면 토글이 헛돈다
        let 숨김 = entry_with("숨김.txt", FILE_ATTRIBUTE_HIDDEN.0);
        assert!(숨김.is_hidden() && !숨김.is_system());

        let 시스템 = entry_with("pagefile.sys", FILE_ATTRIBUTE_SYSTEM.0);
        assert!(시스템.is_system() && !시스템.is_hidden());

        // 둘 다 붙은 항목 — 두 설정이 모두 켜져야 보인다(거르는 쪽 판정은 호출부가 한다)
        let 둘다 = entry_with(
            "System Volume Information",
            FILE_ATTRIBUTE_HIDDEN.0 | FILE_ATTRIBUTE_SYSTEM.0,
        );
        assert!(둘다.is_hidden() && 둘다.is_system());

        // 어느 쪽이든 흐리게 그린다
        for e in [&숨김, &시스템, &둘다] {
            assert!(e.is_dimmed(), "{}를 흐리게 그리지 않는다", e.name());
        }

        // 이름이 점으로 시작해도 로컬은 속성으로만 판정한다 — 윈도우의 규칙이다
        assert!(!entry_with(".gitignore", 0).is_hidden());
    }

    #[test]
    fn 원격_숨김은_점으로_시작하는_이름이다() {
        // 유닉스 계열 서버에는 숨김 속성이 없다 (D8)
        assert!(!remote("보통.txt", false, 0, None).is_hidden());
        assert!(remote(".bashrc", false, 0, None).is_hidden());
        assert!(remote(".ssh", true, 0, None).is_hidden());
        // `..`는 이름이 점으로 시작하지만 화면 장치다 — 걸러지면 맨 윗줄이 사라진다
        assert!(!remote("..", true, 0, None).is_hidden());
        // 서버는 시스템 속성을 주지 않는다 — 어떤 이름이든 시스템이 아니다
        for name in ["보통.txt", ".bashrc", "pagefile.sys"] {
            assert!(!remote(name, false, 0, None).is_system());
        }
    }

    #[test]
    fn 확장자를_끄면_이름만_보인다() {
        // 켜져 있으면 손대지 않는다
        assert_eq!(
            entry("보고서.hwp", false, 0, 0).display_name(true),
            "보고서.hwp"
        );
        // 일반 파일만 확장자가 떨어진다
        assert_eq!(
            entry("보고서.hwp", false, 0, 0).display_name(false),
            "보고서"
        );
        // 폴더는 점이 있어도 이름의 일부다
        assert_eq!(entry("v1.2", true, 0, 0).display_name(false), "v1.2");
        // 상위 이동 줄을 건드리면 목록 맨 위가 깨진다
        assert_eq!(entry("..", true, 0, 0).display_name(false), "..");
        // 확장자가 없으면 그대로
        assert_eq!(entry("README", false, 0, 0).display_name(false), "README");
        // 앞이 비면 확장자가 아니라 이름 자체다
        assert_eq!(
            entry(".gitignore", false, 0, 0).display_name(false),
            ".gitignore"
        );
        // 점만 있는 이름·끝이 점인 이름은 잘라도 얻는 게 없다
        assert_eq!(entry(".", false, 0, 0).display_name(false), ".");
        assert_eq!(entry("a.", false, 0, 0).display_name(false), "a.");
        // 확장자가 여러 겹이면 마지막 것만 뗀다
        assert_eq!(entry("a.tar.gz", false, 0, 0).display_name(false), "a.tar");
    }

    #[test]
    fn 원격_항목도_같은_규칙으로_확장자를_뗀다() {
        // 화면이 프로토콜마다 다르면 안 된다 (FR-52)
        assert_eq!(
            remote("보고서.hwp", false, 0, None).display_name(true),
            "보고서.hwp"
        );
        assert_eq!(
            remote("보고서.hwp", false, 0, None).display_name(false),
            "보고서"
        );
        assert_eq!(remote("v1.2", true, 0, None).display_name(false), "v1.2");
        assert_eq!(remote("..", true, 0, None).display_name(false), "..");
        assert_eq!(
            remote("README", false, 0, None).display_name(false),
            "README"
        );
        assert_eq!(
            remote(".bashrc", false, 0, None).display_name(false),
            ".bashrc"
        );
    }

    #[test]
    fn 확장자를_꺼도_정렬은_원래_이름으로_한다() {
        // 표시만 바뀌어야 한다 — 정렬 키가 잘린 이름을 쓰면 순서가 달라진다 (D7)
        let a = entry("파일2.txt", false, 0, 0);
        let b = entry("파일10.txt", false, 0, 0);
        assert_eq!(
            compare_entries(&a, "", &b, "", SortKey::Name),
            std::cmp::Ordering::Less
        );
        // 정렬 키는 확장자를 포함한 원본이다
        assert_eq!(
            a.name_sort_key().as_ref(),
            "파일2.txt\0"
                .encode_utf16()
                .collect::<Vec<u16>>()
                .as_slice()
        );
    }
}
