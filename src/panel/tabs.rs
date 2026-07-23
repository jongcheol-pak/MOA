//! 패널별 탭 — 순수 모델(TabsModel, 단위테스트 대상) + WC_TABCONTROL 래퍼 (FR-3)
use crate::panel::history::History;
use std::path::{Path, PathBuf};
use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Controls::{
    TCIF_TEXT, TCITEMW, TCM_DELETEITEM, TCM_GETCURSEL, TCM_INSERTITEMW, TCM_SETCURSEL,
    TCM_SETITEMW, WC_TABCONTROLW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, MoveWindow, SendMessageW, WINDOW_EX_STYLE, WS_CHILD, WS_CLIPSIBLINGS,
    WS_TABSTOP, WS_VISIBLE,
};
use windows::core::{HSTRING, Result};

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

    // part2 T4(세션 저장 — 탭 순회)에서 소비 예정. 테스트는 이미 사용
    #[cfg_attr(not(test), expect(dead_code))]
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
                WS_CHILD | WS_VISIBLE | WS_CLIPSIBLINGS | WS_TABSTOP,
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
