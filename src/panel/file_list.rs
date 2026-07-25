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
                        format_size_kb(entry.size)
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

/// 정렬 비교 — 폴더 우선(D7), 이름은 탐색기와 동일한 숫자 인지 정렬
fn compare_entries(
    a: &FileEntry,
    type_a: &str,
    b: &FileEntry,
    type_b: &str,
    key: SortKey,
) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    // 폴더는 정렬 방향과 무관하게 항상 우선
    match (a.is_dir, b.is_dir) {
        (true, false) => return Ordering::Less,
        (false, true) => return Ordering::Greater,
        _ => {}
    }
    let by_name = |x: &FileEntry, y: &FileEntry| logical_name_cmp(&x.name, &y.name);
    match key {
        SortKey::Name => by_name(a, b),
        SortKey::Size => a.size.cmp(&b.size).then_with(|| by_name(a, b)),
        SortKey::Type => type_a.cmp(type_b).then_with(|| by_name(a, b)),
        SortKey::Modified => a.modified.cmp(&b.modified).then_with(|| by_name(a, b)),
    }
}

/// StrCmpLogicalW 래퍼 — 널 종단 UTF-16 이름 비교 ("파일2" < "파일10")
fn logical_name_cmp(a: &[u16], b: &[u16]) -> std::cmp::Ordering {
    // 안전성: 두 버퍼 모두 널 종단 보장(FileEntry 불변식)
    let r = unsafe { StrCmpLogicalW(PCWSTR(a.as_ptr()), PCWSTR(b.as_ptr())) };
    r.cmp(&0)
}

/// 탐색기식 크기 표시: KB 올림 + 천 단위 구분.
/// `pub`인 이유: egui 이식 PoC 바이너리(별도 crate)가 같은 표시 규칙을 쓰기 위해 —
/// 복제하면 표시 형식이 두 벌로 갈라진다 (plan 2026-07-25-egui-poc D3)
pub fn format_size_kb(bytes: u64) -> String {
    let kb = bytes.div_ceil(1024).max(if bytes > 0 { 1 } else { 0 });
    let s = kb.to_string();
    let mut out = String::with_capacity(s.len() + s.len() / 3 + 3);
    let offset = s.len() % 3;
    for (i, c) in s.chars().enumerate() {
        if i != 0 && (i + 3 - offset) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    out.push_str("KB");
    out.insert(out.len() - 2, ' ');
    out
}

/// FILETIME(u64) → 로컬 "yyyy-MM-dd HH:mm".
/// `pub`인 이유는 `format_size_kb`와 동일 (PoC 바이너리와 표시 규칙 공유)
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

    fn entry(name: &str, is_dir: bool, size: u64, modified: u64) -> FileEntry {
        let mut v: Vec<u16> = name.encode_utf16().collect();
        v.push(0);
        FileEntry {
            name: v,
            is_dir,
            size,
            modified,
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
    fn 크기_표시는_kb_올림_천단위_구분() {
        assert_eq!(format_size_kb(0), "0 KB");
        assert_eq!(format_size_kb(1), "1 KB");
        assert_eq!(format_size_kb(1024), "1 KB");
        assert_eq!(format_size_kb(1025), "2 KB");
        assert_eq!(format_size_kb(1_234_567), "1,206 KB");
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
}
