//! 셸 컨텍스트 메뉴 — IContextMenu 표준 절차 래퍼 (FR-8, plan D13)
//!
//! SHParseDisplayName → 부모 IShellFolder → GetUIObjectOf(IContextMenu) →
//! QueryContextMenu → TrackPopupMenuEx → InvokeCommand 절차와
//! IContextMenu2/3 소유자 메시지 포워딩을 이 모듈에 격리한다 (unsafe 격리).
//! 복사·삭제·속성 등 실제 파일 작업 UI는 전부 셸이 제공한다 (PRD Out of Scope 위임).
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::Shell::Common::ITEMIDLIST;
use windows::Win32::UI::Shell::{
    CMF_NORMAL, CMINVOKECOMMANDINFO, IContextMenu, IContextMenu2, IContextMenu3, ILFindLastID,
    ILFree, IShellFolder, SHBindToParent, SHGetDesktopFolder, SHParseDisplayName,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreatePopupMenu, DestroyMenu, SW_SHOWNORMAL, TPM_LEFTALIGN, TPM_RETURNCMD, TPM_RIGHTBUTTON,
    TPM_TOPALIGN, TrackPopupMenuEx, WM_DRAWITEM, WM_INITMENUPOPUP, WM_MEASUREITEM, WM_MENUCHAR,
};
use windows::core::{HSTRING, Interface, PCSTR, Result};

/// QueryContextMenu 명령 id 범위 시작 (0은 "선택 없음"과 구분 불가라 1부터)
const ID_FIRST: u32 = 1;
const ID_LAST: u32 = 0x7fff;

thread_local! {
    /// TrackPopupMenuEx 모달 루프 동안 소유자 메시지를 포워딩할 인터페이스.
    /// UI 스레드 전용(STA)이므로 thread_local로 충분하다
    static ACTIVE_MENU: RefCell<Option<(Option<IContextMenu2>, Option<IContextMenu3>)>> =
        const { RefCell::new(None) };
}

/// 셸 컨텍스트 메뉴 표시·실행. items가 비면 folder의 배경 메뉴(새로 만들기 포함).
/// 좌표는 화면 기준. 실패는 조용히 무시한다 — 오류 UI는 셸 위임 (T2 Edge)
pub fn show_context_menu(owner: HWND, folder: &Path, items: &[PathBuf], x: i32, y: i32) {
    // 안전성: COM은 main에서 STA 초기화됨. 내부 포인터 수명은 이 함수 안에서 닫힌다
    unsafe {
        let _ = show_inner(owner, folder, items, x, y);
    }
}

/// 소유자 메시지(WM_INITMENUPOPUP 등)를 활성 IContextMenu2/3에 포워딩.
/// 메뉴 모달 중이 아니면 None → 호출부가 기본 처리로 넘긴다
pub fn forward_menu_msg(msg: u32, wparam: WPARAM, lparam: LPARAM) -> Option<LRESULT> {
    if !matches!(
        msg,
        WM_INITMENUPOPUP | WM_DRAWITEM | WM_MEASUREITEM | WM_MENUCHAR
    ) {
        return None;
    }
    ACTIVE_MENU.with_borrow(|active| {
        let (cm2, cm3) = active.as_ref()?;
        // 안전성: 인터페이스는 모달 루프 동안 유효 (show_inner가 수명 관리)
        unsafe {
            if let Some(cm3) = cm3 {
                let mut lres = LRESULT(0);
                let _ = cm3.HandleMenuMsg2(msg, wparam, lparam, Some(&mut lres));
                return Some(lres);
            }
            if let Some(cm2) = cm2 {
                let _ = cm2.HandleMenuMsg(msg, wparam, lparam);
                return Some(LRESULT(0));
            }
        }
        None
    })
}

/// 절차 본체 — 실패는 Err로 조기 종료(호출부가 무시)
unsafe fn show_inner(owner: HWND, folder: &Path, items: &[PathBuf], x: i32, y: i32) -> Result<()> {
    // 안전성: 이 함수 전체가 COM/Win32 FFI — 포인터는 지역 소유, 해제 경로 명시
    unsafe {
        let context_menu: IContextMenu = if items.is_empty() {
            background_menu(owner, folder)?
        } else {
            items_menu(owner, items)?
        };

        let menu = CreatePopupMenu()?;
        let result = (|| -> Result<()> {
            context_menu
                .QueryContextMenu(menu, 0, ID_FIRST, ID_LAST, CMF_NORMAL)
                .ok()?;

            // 서브메뉴(보내기 등) 동적 채움을 위한 포워딩 대상 등록 (T2 Edge)
            let cm2: Option<IContextMenu2> = context_menu.cast().ok();
            let cm3: Option<IContextMenu3> = context_menu.cast().ok();
            ACTIVE_MENU.set(Some((cm2, cm3)));

            let cmd = TrackPopupMenuEx(
                menu,
                (TPM_RETURNCMD | TPM_RIGHTBUTTON | TPM_LEFTALIGN | TPM_TOPALIGN).0,
                x,
                y,
                owner,
                None,
            );
            ACTIVE_MENU.set(None);

            let cmd = cmd.0 as u32;
            if cmd >= ID_FIRST {
                // 선택된 항목 실행 — 실패(메뉴 표시 중 대상 삭제 등)는 셸이 UI로 알림
                let info = CMINVOKECOMMANDINFO {
                    cbSize: size_of::<CMINVOKECOMMANDINFO>() as u32,
                    hwnd: owner,
                    // MAKEINTRESOURCE 관례: 하위 16비트가 명령 오프셋인 유사 포인터
                    lpVerb: PCSTR((cmd - ID_FIRST) as usize as *const u8),
                    nShow: SW_SHOWNORMAL.0,
                    ..Default::default()
                };
                let _ = context_menu.InvokeCommand(&info);
            }
            Ok(())
        })();
        // 메뉴 핸들은 성공·실패 무관하게 해제
        let _ = DestroyMenu(menu);
        result
    }
}

/// 선택 항목들의 공통 부모 IShellFolder에서 IContextMenu 획득
unsafe fn items_menu(owner: HWND, items: &[PathBuf]) -> Result<IContextMenu> {
    // 안전성: PIDL은 이 함수에서 생성·해제(ILFree). 자식 PIDL 포인터는 절대 PIDL 내부를
    // 가리키므로(ILFindLastID) 절대 PIDL 해제 전까지만 사용한다
    unsafe {
        let mut pidls: Vec<*mut ITEMIDLIST> = Vec::with_capacity(items.len());
        let result = (|| -> Result<IContextMenu> {
            for path in items {
                let mut pidl: *mut ITEMIDLIST = std::ptr::null_mut();
                SHParseDisplayName(
                    &HSTRING::from(path.as_os_str()),
                    None::<&windows::Win32::System::Com::IBindCtx>,
                    &mut pidl,
                    0,
                    None,
                )?;
                pidls.push(pidl);
            }
            // 모든 항목은 같은 폴더(파일 목록 한 화면)에서 왔으므로 부모는 첫 항목 기준
            let parent: IShellFolder = SHBindToParent(pidls[0], None)?;
            let children: Vec<*const ITEMIDLIST> = pidls
                .iter()
                .map(|&p| ILFindLastID(p) as *const ITEMIDLIST)
                .collect();
            parent.GetUIObjectOf(owner, &children, None)
        })();
        for pidl in pidls {
            ILFree(Some(pidl as *const ITEMIDLIST));
        }
        result
    }
}

/// 폴더 자신의 배경 컨텍스트 메뉴(새로 만들기 포함) 획득
unsafe fn background_menu(owner: HWND, folder: &Path) -> Result<IContextMenu> {
    // 안전성: PIDL 생성·해제 이 함수 안에서 완결
    unsafe {
        let mut pidl: *mut ITEMIDLIST = std::ptr::null_mut();
        SHParseDisplayName(
            &HSTRING::from(folder.as_os_str()),
            None::<&windows::Win32::System::Com::IBindCtx>,
            &mut pidl,
            0,
            None,
        )?;
        let result = (|| -> Result<IContextMenu> {
            let desktop = SHGetDesktopFolder()?;
            let shell_folder: IShellFolder =
                desktop.BindToObject(pidl, None::<&windows::Win32::System::Com::IBindCtx>)?;
            shell_folder.CreateViewObject(owner)
        })();
        ILFree(Some(pidl as *const ITEMIDLIST));
        result
    }
}
