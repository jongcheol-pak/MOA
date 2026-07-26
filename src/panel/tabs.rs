//! 패널별 탭 — 순수 모델(TabsModel, 단위테스트 대상) + WC_TABCONTROL 래퍼 (FR-3)
use crate::app::theme;
use crate::panel::history::History;
use std::path::{Path, PathBuf};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    CreateSolidBrush, DT_CENTER, DT_SINGLELINE, DT_VCENTER, DeleteObject, DrawTextW, FillRect, HDC,
    SetBkMode, SetTextColor, TRANSPARENT,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Controls::{
    DRAWITEMSTRUCT, ODS_SELECTED, TCIF_TEXT, TCITEMW, TCM_DELETEITEM, TCM_GETCURSEL, TCM_GETITEMW,
    TCM_INSERTITEMW, TCM_SETCURSEL, TCM_SETITEMW, TCS_OWNERDRAWFIXED, WC_TABCONTROLW,
};
use windows::Win32::UI::Shell::{DefSubclassProc, SetWindowSubclass};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, GetClientRect, MoveWindow, SendMessageW, WINDOW_EX_STYLE, WINDOW_STYLE,
    WM_ERASEBKGND, WS_CHILD, WS_CLIPSIBLINGS, WS_TABSTOP, WS_VISIBLE,
};
use windows::core::{HSTRING, PWSTR, Result};

/// 탭 스트립 높이
pub const TAB_HEIGHT: i32 = 26;

/// 탭 하나의 탐색 상태 — 탭별 독립 경로·히스토리 (FR-3)
pub struct TabState {
    pub committed: PathBuf,
    pub history: History,
}

impl TabState {
    pub fn new(path: PathBuf) -> TabState {
        TabState {
            history: History::new(path.clone()),
            committed: path,
        }
    }
}

/// 탭 닫기 결과
#[derive(Debug, PartialEq, Eq)]
pub enum CloseOutcome {
    /// 탭 제거됨 — 새 활성 인덱스
    Removed(usize),
    /// 마지막 탭 — 패널 닫기로 연결해야 함 (호출부 몫)
    LastTab,
}

/// 탭 목록 순수 모델 — UI 비의존 (단위테스트 대상)
pub struct TabsModel {
    tabs: Vec<TabState>,
    active: usize,
}

impl TabsModel {
    pub fn new(first: TabState) -> TabsModel {
        TabsModel {
            tabs: vec![first],
            active: 0,
        }
    }

    /// 세션 복원용 재구성 — 빈 목록이면 None, 활성 인덱스는 범위로 클램프
    pub fn from_tabs(tabs: Vec<TabState>, active: usize) -> Option<TabsModel> {
        if tabs.is_empty() {
            return None;
        }
        let active = active.min(tabs.len() - 1);
        Some(TabsModel { tabs, active })
    }

    /// 탭별 커밋 경로 (탭 순서 유지) — 세션 저장용
    pub fn paths(&self) -> Vec<PathBuf> {
        self.tabs.iter().map(|t| t.committed.clone()).collect()
    }

    /// 탭 수 — 세션 저장(part2 T4)·테스트가 소비.
    /// is_empty는 제공하지 않음 — 탭은 항상 1개 이상(불변식)이라 항상 false로 오해만 낳는다
    #[expect(clippy::len_without_is_empty)]
    pub fn len(&self) -> usize {
        self.tabs.len()
    }

    pub fn active_index(&self) -> usize {
        self.active
    }

    pub fn active(&self) -> &TabState {
        &self.tabs[self.active]
    }

    pub fn active_mut(&mut self) -> &mut TabState {
        &mut self.tabs[self.active]
    }

    /// 새 탭 — 현재 활성 탭 바로 뒤에 추가하고 활성화 (plan D4: 현재 경로 복제는 호출부가 전달)
    pub fn add(&mut self, state: TabState) -> usize {
        let at = self.active + 1;
        self.tabs.insert(at, state);
        self.active = at;
        at
    }

    /// 활성 탭 닫기 — 마지막 1개면 LastTab (패널 닫기로 연결)
    pub fn close_active(&mut self) -> CloseOutcome {
        if self.tabs.len() <= 1 {
            return CloseOutcome::LastTab;
        }
        self.tabs.remove(self.active);
        if self.active >= self.tabs.len() {
            self.active = self.tabs.len() - 1;
        }
        CloseOutcome::Removed(self.active)
    }

    /// 지정한 탭 닫기 — **보고 있던 탭은 그대로 유지**한다 (마지막 1개면 LastTab).
    ///
    /// `close_active`는 "활성 탭 자신을 닫는" 경우만 상정한 인덱스 보정을 하므로,
    /// 배경 탭을 닫으려고 `switch` 후 `close_active`를 부르면 활성 탭이 옆으로 튄다.
    /// 임의 탭에 닫기 버튼이 달린 UI(egui 탭 스트립)는 이 함수를 쓴다.
    ///
    /// 주의: **범위 밖 인덱스는 아무것도 제거하지 않고** 현재 활성 인덱스를 그대로 담아
    /// `Removed`를 돌려준다 — 반환값만 보고 "탭이 하나 줄었다"고 단정하면 안 된다
    pub fn close(&mut self, index: usize) -> CloseOutcome {
        if self.tabs.len() <= 1 {
            return CloseOutcome::LastTab;
        }
        if index >= self.tabs.len() {
            return CloseOutcome::Removed(self.active);
        }
        self.tabs.remove(index);
        self.active = match self.active.cmp(&index) {
            // 닫힌 탭보다 앞이면 인덱스가 그대로다
            std::cmp::Ordering::Less => self.active,
            // 활성 탭 자신을 닫았으면 그 자리를 이어받되 끝을 넘지 않는다
            std::cmp::Ordering::Equal => self.active.min(self.tabs.len() - 1),
            // 닫힌 탭보다 뒤면 한 칸 당겨진다 — 보던 탭이 그대로 유지된다
            std::cmp::Ordering::Greater => self.active - 1,
        };
        CloseOutcome::Removed(self.active)
    }

    /// 탭 전환 (범위 밖 인덱스는 무시)
    pub fn switch(&mut self, index: usize) -> bool {
        if index < self.tabs.len() && index != self.active {
            self.active = index;
            true
        } else {
            false
        }
    }
}

/// 탭 제목 — 폴더 이름 (루트는 드라이브 표기 그대로)
pub fn tab_title(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_string_lossy().into_owned())
}

/// 탭 컨트롤 서브클래스 — 스트립 배경 다크 (WM_ERASEBKGND).
/// 오너드로우 탭 항목(draw_tab)이 그 위에 그려지므로 항목 밖 여백만 이 다크색으로 남는다.
unsafe extern "system" fn tab_dark_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    _id: usize,
    _data: usize,
) -> LRESULT {
    if msg == WM_ERASEBKGND {
        // 안전성: WM_ERASEBKGND의 wparam은 대상 DC. 클라이언트 전체를 다크로 채우고 처리 완료(1) 반환
        unsafe {
            let hdc = HDC(wparam.0 as *mut core::ffi::c_void);
            let mut rc = RECT::default();
            if GetClientRect(hwnd, &mut rc).is_ok() {
                let brush = CreateSolidBrush(theme::WINDOW_BG);
                FillRect(hdc, &rc, brush);
                let _ = DeleteObject(brush.into());
            }
        }
        return LRESULT(1);
    }
    // 안전성: 그 외 메시지는 원래 탭 프로시저로 위임
    unsafe { DefSubclassProc(hwnd, msg, wparam, lparam) }
}

/// 탭 오너드로우 다크 — 부모 패널의 WM_DRAWITEM에서 호출한다 (옵션 B).
/// 활성/비활성 탭 배경을 다크색으로 채우고 제목을 중앙에 그린다.
pub fn draw_tab(dis: &DRAWITEMSTRUCT) {
    let selected = (dis.itemState.0 & ODS_SELECTED.0) != 0;
    let bg = if selected {
        theme::SURFACE_BG
    } else {
        theme::WINDOW_BG
    };
    let mut rc = dis.rcItem;
    // 안전성: 오너드로우가 넘긴 유효 DC에 배경·제목을 그린다. 브러시는 생성 즉시 해제
    unsafe {
        let brush = CreateSolidBrush(bg);
        FillRect(dis.hDC, &rc, brush);
        let _ = DeleteObject(brush.into());
        // 탭 제목(itemID번 탭)을 버퍼로 읽어 중앙에 그린다
        let mut buf = [0u16; 256];
        let mut item = TCITEMW {
            mask: TCIF_TEXT,
            pszText: PWSTR(buf.as_mut_ptr()),
            cchTextMax: buf.len() as i32,
            ..Default::default()
        };
        SendMessageW(
            dis.hwndItem,
            TCM_GETITEMW,
            Some(WPARAM(dis.itemID as usize)),
            Some(LPARAM(&mut item as *mut _ as isize)),
        );
        let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
        SetTextColor(
            dis.hDC,
            if selected {
                theme::TEXT
            } else {
                theme::TEXT_DIM
            },
        );
        SetBkMode(dis.hDC, TRANSPARENT);
        DrawTextW(
            dis.hDC,
            &mut buf[..len],
            &mut rc,
            DT_CENTER | DT_VCENTER | DT_SINGLELINE,
        );
    }
}

/// WC_TABCONTROL 래퍼 — 표시만 담당, 진실은 TabsModel
pub struct TabStrip {
    hwnd: HWND,
}

impl TabStrip {
    pub fn create(parent: HWND) -> Result<TabStrip> {
        // 안전성: 공용 컨트롤 자식 생성
        let hwnd = unsafe {
            let instance = GetModuleHandleW(None)?;
            CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                WC_TABCONTROLW,
                None,
                // TCS_OWNERDRAWFIXED: 탭은 SetWindowTheme(uxtheme)로 다크가 안 먹으므로
                // 부모 WM_DRAWITEM에서 draw_tab이 직접 그린다 (옵션 B — 오너드로우 복원)
                WS_CHILD
                    | WS_VISIBLE
                    | WS_CLIPSIBLINGS
                    | WS_TABSTOP
                    | WINDOW_STYLE(TCS_OWNERDRAWFIXED),
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
        // 탭 스트립 배경 다크 — 오너드로우(draw_tab)는 탭 항목만 그리므로, 항목 밖 스트립 여백은
        // 서브클래스의 WM_ERASEBKGND에서 다크로 채운다 (SetWindowTheme는 탭에 안 먹음).
        // 안전성: 방금 생성한 유효한 탭 핸들에 표준 서브클래스 등록
        unsafe {
            let _ = SetWindowSubclass(hwnd, Some(tab_dark_proc), 1, 0);
        }
        Ok(TabStrip { hwnd })
    }

    pub fn hwnd(&self) -> HWND {
        self.hwnd
    }

    /// 탭 삽입 (표시용)
    pub fn insert(&self, index: usize, title: &str) {
        let text = HSTRING::from(title);
        let mut item = TCITEMW {
            mask: TCIF_TEXT,
            pszText: windows::core::PWSTR(text.as_ptr() as *mut u16),
            ..Default::default()
        };
        // 안전성: item·text는 호출 동안 살아있는 스택/지역 소유
        unsafe {
            SendMessageW(
                self.hwnd,
                TCM_INSERTITEMW,
                Some(WPARAM(index)),
                Some(LPARAM(&mut item as *mut _ as isize)),
            );
        }
    }

    /// 탭 제목 갱신 (탐색 커밋 시 폴더명 반영)
    pub fn set_title(&self, index: usize, title: &str) {
        let text = HSTRING::from(title);
        let mut item = TCITEMW {
            mask: TCIF_TEXT,
            pszText: windows::core::PWSTR(text.as_ptr() as *mut u16),
            ..Default::default()
        };
        // 안전성: item·text는 호출 동안 살아있는 스택/지역 소유
        unsafe {
            SendMessageW(
                self.hwnd,
                TCM_SETITEMW,
                Some(WPARAM(index)),
                Some(LPARAM(&mut item as *mut _ as isize)),
            );
        }
    }

    pub fn remove(&self, index: usize) {
        // 안전성: 표준 탭 삭제 메시지
        unsafe {
            SendMessageW(
                self.hwnd,
                TCM_DELETEITEM,
                Some(WPARAM(index)),
                Some(LPARAM(0)),
            );
        }
    }

    pub fn set_selection(&self, index: usize) {
        // 안전성: 표준 선택 메시지
        unsafe {
            SendMessageW(
                self.hwnd,
                TCM_SETCURSEL,
                Some(WPARAM(index)),
                Some(LPARAM(0)),
            );
        }
    }

    pub fn selection(&self) -> i32 {
        // 안전성: 표준 조회 메시지
        unsafe { SendMessageW(self.hwnd, TCM_GETCURSEL, Some(WPARAM(0)), Some(LPARAM(0))).0 as i32 }
    }

    pub fn resize(&self, x: i32, y: i32, w: i32, h: i32) {
        // 안전성: 자식 창 이동
        unsafe {
            let _ = MoveWindow(self.hwnd, x, y, w.max(0), h.max(0), true);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tab(p: &str) -> TabState {
        TabState::new(PathBuf::from(p))
    }

    #[test]
    fn 새_탭은_활성_뒤에_추가되고_활성화된다() {
        let mut m = TabsModel::new(tab("C:\\a"));
        m.add(tab("C:\\b"));
        assert_eq!(m.len(), 2);
        assert_eq!(m.active_index(), 1);
        assert_eq!(m.active().committed, PathBuf::from("C:\\b"));

        // 첫 탭으로 돌아가 중간 삽입 확인
        m.switch(0);
        m.add(tab("C:\\c"));
        assert_eq!(m.active_index(), 1);
        assert_eq!(m.active().committed, PathBuf::from("C:\\c"));
        assert_eq!(m.len(), 3);
    }

    #[test]
    fn 탭_닫기와_활성_보정() {
        let mut m = TabsModel::new(tab("C:\\a"));
        m.add(tab("C:\\b"));
        m.add(tab("C:\\c")); // 활성 = 2 (c)
        assert_eq!(m.close_active(), CloseOutcome::Removed(1)); // c 제거 → b 활성
        assert_eq!(m.active().committed, PathBuf::from("C:\\b"));
        assert_eq!(m.close_active(), CloseOutcome::Removed(0)); // b 제거 → a 활성
        assert_eq!(m.close_active(), CloseOutcome::LastTab); // 마지막 — 제거 안 됨
        assert_eq!(m.len(), 1);
    }

    #[test]
    fn 배경_탭을_닫아도_보던_탭이_유지된다() {
        let mut m = TabsModel::new(tab("C:\\a"));
        m.add(tab("C:\\b"));
        m.add(tab("C:\\c"));
        m.add(tab("C:\\d")); // [a, b, c, d], 활성 = 3 (d)

        // 활성보다 앞의 배경 탭(b)을 닫아도 계속 d를 보고 있어야 한다
        assert_eq!(m.close(1), CloseOutcome::Removed(2));
        assert_eq!(m.active().committed, PathBuf::from("C:\\d"));

        // 활성보다 뒤는 없지만, 앞쪽(a)을 하나 더 닫아도 마찬가지
        assert_eq!(m.close(0), CloseOutcome::Removed(1));
        assert_eq!(m.active().committed, PathBuf::from("C:\\d"));
    }

    #[test]
    fn 활성_탭을_인덱스로_닫으면_옆_탭이_활성된다() {
        let mut m = TabsModel::new(tab("C:\\a"));
        m.add(tab("C:\\b"));
        m.add(tab("C:\\c")); // 활성 = 2 (c)

        assert_eq!(m.close(2), CloseOutcome::Removed(1));
        assert_eq!(m.active().committed, PathBuf::from("C:\\b"));

        // 활성(1=b)보다 뒤가 없으니 앞의 a를 닫으면 b가 0번으로 당겨진다
        assert_eq!(m.close(0), CloseOutcome::Removed(0));
        assert_eq!(m.active().committed, PathBuf::from("C:\\b"));
    }

    #[test]
    fn 마지막_탭은_인덱스로도_닫히지_않는다() {
        let mut m = TabsModel::new(tab("C:\\a"));
        assert_eq!(m.close(0), CloseOutcome::LastTab);
        assert_eq!(m.len(), 1);
    }

    #[test]
    fn 범위_밖_인덱스_닫기는_무시된다() {
        let mut m = TabsModel::new(tab("C:\\a"));
        m.add(tab("C:\\b")); // 활성 = 1
        assert_eq!(m.close(9), CloseOutcome::Removed(1));
        assert_eq!(m.len(), 2);
        assert_eq!(m.active().committed, PathBuf::from("C:\\b"));
    }

    #[test]
    fn 탭_전환은_범위_검사한다() {
        let mut m = TabsModel::new(tab("C:\\a"));
        m.add(tab("C:\\b"));
        assert!(m.switch(0));
        assert!(!m.switch(0)); // 동일 인덱스 — 변화 없음
        assert!(!m.switch(9)); // 범위 밖
        assert_eq!(m.active_index(), 0);
    }

    #[test]
    fn 탭별_히스토리는_독립이다() {
        let mut m = TabsModel::new(tab("C:\\a"));
        m.active_mut().history.push(PathBuf::from("C:\\a\\1"));
        m.add(tab("C:\\b"));
        assert!(!m.active().history.can_back()); // 새 탭은 독립 히스토리
        m.switch(0);
        assert!(m.active().history.can_back());
    }

    #[test]
    fn 탭_제목은_폴더_이름이다() {
        assert_eq!(tab_title(Path::new(r"C:\Users\me\문서")), "문서");
        assert_eq!(tab_title(Path::new(r"C:\")), r"C:\");
    }
}
