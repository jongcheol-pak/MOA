//! Windows 기본 압축 — `보내기 > 압축(ZIP) 폴더`가 하는 일을 메뉴를 거치지 않고 부른다 (FR-8).
//!
//! **왜 메뉴를 거치지 않는가**: 우리가 읽는 레거시 컨텍스트 메뉴(`IContextMenu`)에는
//! 이 PC에서 **하위 메뉴가 한 줄도 오지 않는다**(2026-08-26 실측 — `fs::shell_menu`의
//! `verb_dump`가 그 수를 함께 찍는다). `보내기`도 평면 항목이라 그 아래의 `압축(ZIP) 폴더`에
//! 닿을 손잡이가 없다. Windows 11의 `다음으로 압축`은 새 메뉴(`IExplorerCommand`) 전용이라
//! 역시 오지 않는다.
//!
//! **대신 `SendTo` 폴더의 그 항목을 셸에서 직접 얻는다.** 그것은 `IDropTarget`이고, 거기에
//! 고른 파일들을 놓으면 탐색기에서 그 줄을 눌렀을 때와 **같은 일**이 일어난다 —
//! 압축은 `zipfldr.dll`이 하고 이름 충돌·진행률·취소 대화가 모두 Windows 것이다.
use std::path::{Path, PathBuf};

use windows::Win32::Foundation::{HWND, POINTL};
use windows::Win32::System::Ole::IDropTarget;
use windows::Win32::System::SystemServices::MODIFIERKEYS_FLAGS;
use windows::Win32::UI::Shell::Common::ITEMIDLIST;
use windows::Win32::UI::Shell::{
    ILFindLastID, ILFree, IShellFolder, SHBindToParent, SHParseDisplayName,
};
use windows::core::{HSTRING, Result};

/// `SendTo` 폴더 안의 그 항목 — **파일 이름은 표시 언어와 무관하게 영어다**(실측).
///
/// 화면에는 `압축(ZIP) 폴더`로 보이지만 파일 자체는 이 이름이라, 한국어 문구로 찾으면 없다
const ZIP_SENDTO_FILE: &str = "Compressed (zipped) Folder.ZFSendToTarget";

/// 고른 것들을 Windows 기본 압축에 넘긴다 — 만들어지는 자리는 셸이 정한다(같은 폴더).
///
/// **실패는 조용하다** — `SendTo` 항목이 없거나 셸이 거절하면 아무 일도 일어나지 않는다.
/// 이 모듈의 다른 셸 위임과 같은 규칙이며(`fs::shell_menu`의 `invoke`), 오류 대화는 띄울
/// 것이 있을 때 셸이 스스로 띄운다.
///
/// 돌려주는 값은 **넘기는 데 성공했는가**이지 압축이 끝났는가가 아니다 — 셸이 자기 진행률
/// 창에서 이어서 하고, 우리 폴더 감시(FR-32)가 만들어진 파일을 잡는다
pub fn compress_to_zip(paths: &[PathBuf]) -> bool {
    if paths.is_empty() {
        return false;
    }
    let Some(target) = zip_drop_target() else {
        return false;
    };
    let Some(data) = crate::fs::drag_source::data_object(paths) else {
        return false;
    };
    // 안전성: 위에서 얻은 유효한 인터페이스에 표준 순서(DragEnter → Drop)로 건다. 좌표는
    // 놓는 자리를 뜻하는데 이 대상에는 자리 개념이 없어 원점을 준다(탐색기도 같다)
    unsafe {
        let mut effect = windows::Win32::System::Ole::DROPEFFECT_COPY;
        let pt = POINTL { x: 0, y: 0 };
        if target
            .DragEnter(&data, MODIFIERKEYS_FLAGS(0), pt, &mut effect)
            .is_err()
        {
            return false;
        }
        target
            .Drop(&data, MODIFIERKEYS_FLAGS(0), pt, &mut effect)
            .is_ok()
    }
}

/// `SendTo`의 압축 항목을 놓기 대상으로 얻는다.
///
/// **매번 다시 얻는다** — 캐시해 두면 그 인터페이스를 앱 수명 내내 들고 있어야 하는데,
/// 압축은 자주 쓰는 동작이 아니라 그 값을 치를 이유가 없다
fn zip_drop_target() -> Option<IDropTarget> {
    let appdata = std::env::var_os("APPDATA")?;
    let path = Path::new(&appdata)
        .join("Microsoft")
        .join("Windows")
        .join("SendTo")
        .join(ZIP_SENDTO_FILE);

    // 안전성: PIDL은 이 함수에서 만들고 되돌린다. 자식 PIDL은 절대 PIDL 내부를 가리키므로
    // (`ILFindLastID`) 해제 전까지만 쓴다 — `fs::shell_menu`의 같은 관례다
    unsafe {
        let mut pidl: *mut ITEMIDLIST = std::ptr::null_mut();
        SHParseDisplayName(
            &HSTRING::from(path.as_os_str()),
            None::<&windows::Win32::System::Com::IBindCtx>,
            &mut pidl,
            0,
            None,
        )
        .ok()?;
        let result = (|| -> Result<IDropTarget> {
            let parent: IShellFolder = SHBindToParent(pidl, None)?;
            let child = ILFindLastID(pidl) as *const ITEMIDLIST;
            parent.GetUIObjectOf(HWND::default(), &[child], None)
        })();
        ILFree(Some(pidl as *const ITEMIDLIST));
        result.ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 빈_목록은_아무것도_하지_않는다() {
        // 셸을 부르기 전에 걸러야 한다 — 빈 `IDataObject`를 놓으면 셸이 무엇을 할지 모른다
        assert!(!compress_to_zip(&[]));
    }

    #[test]
    fn 항목_파일_이름은_영어다() {
        // 화면 문구(`압축(ZIP) 폴더`)로 찾으면 없다 — 실측으로 확인한 이름이며, 한국어로
        // 바꾸면 대상을 못 찾아 그 줄이 조용히 죽는다
        assert_eq!(ZIP_SENDTO_FILE, "Compressed (zipped) Folder.ZFSendToTarget");
        assert!(ZIP_SENDTO_FILE.ends_with(".ZFSendToTarget"));
    }
}
