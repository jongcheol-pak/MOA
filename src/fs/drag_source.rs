//! 앱에서 탐색기·바탕화면으로 끌어내기 (FR-61 내보내기).
//!
//! **데이터 객체를 직접 구현하지 않는다** — 경로마다 PIDL을 얻어 셸이 만들어 주는
//! `IDataObject`를 쓴다(`SHCreateShellItemArrayFromIDLists` → `BindToHandler`). 그래야
//! 받는 쪽이 기대하는 형식(`CF_HDROP` 등)을 셸이 알아서 채운다.
//!
//! 직접 만드는 것은 `IDropSource` 하나이며 그마저 두 메서드뿐이다 — 언제 그만둘지와
//! 어떤 커서를 보일지.
//!
//! **`DoDragDrop`은 자기 메시지 루프를 돌린다** — 놓을 때까지 돌아오지 않으므로 부르는
//! 쪽은 셸 컨텍스트 메뉴(`TrackPopupMenuEx`)와 같은 자리, 즉 **그리기가 모두 끝난 뒤**에
//! 불러야 한다. 위젯 트리가 절반만 구성된 상태로 재진입시키면 안 된다
use std::path::PathBuf;

use windows::Win32::Foundation::{DRAGDROP_S_CANCEL, DRAGDROP_S_DROP, S_OK};
use windows::Win32::System::Com::{CoTaskMemFree, IDataObject};
use windows::Win32::System::Ole::{
    DROPEFFECT, DROPEFFECT_COPY, DoDragDrop, IDropSource, IDropSource_Impl,
};
use windows::Win32::System::SystemServices::{MK_LBUTTON, MODIFIERKEYS_FLAGS};
use windows::Win32::UI::Shell::Common::ITEMIDLIST;
use windows::Win32::UI::Shell::{
    BHID_DataObject, SHCreateShellItemArrayFromIDLists, SHParseDisplayName,
};
use windows::core::{BOOL, HSTRING, PCWSTR, implement};

/// 끌기를 시작한다 — 놓거나 취소할 때까지 **돌아오지 않는다** (FR-61).
///
/// 돌려주는 값은 "실제로 복사가 일어났는가"다. 경로를 하나도 셸 항목으로 만들지 못했으면
/// 시작조차 하지 않고 `false`다 — 그 사이 파일이 전부 사라졌거나 셸이 다루지 못하는
/// 경로다.
///
/// **효과는 복사만 허용한다**(`DROPEFFECT_COPY`) — FR-61이 내보내기도 복사로 못박았다.
/// 이동을 함께 허용하면 받는 쪽이 이동을 골랐을 때 원본이 사라진다
pub fn start_copy_drag(paths: &[PathBuf]) -> bool {
    if paths.is_empty() {
        return false;
    }
    let pidls = Pidls::parse(paths);
    if pidls.is_empty() {
        return false;
    }
    // 안전성: 아래 호출은 모두 COM이 STA로 초기화된 UI 스레드에서 돌고(`ui::app`이
    // 그리기를 마친 뒤 부른다), 얻은 인터페이스는 이 함수 안에서만 살다 `Drop`으로
    // 해제된다. PIDL은 `Pidls`가 소유해 함수를 벗어날 때 `CoTaskMemFree`로 되돌린다
    unsafe {
        let Ok(items) = SHCreateShellItemArrayFromIDLists(pidls.as_slice()) else {
            return false;
        };
        let Ok(data) = items.BindToHandler::<_, IDataObject>(None, &BHID_DataObject) else {
            return false;
        };
        let source: IDropSource = CopyDragSource.into();
        let mut effect = DROPEFFECT::default();
        let result = DoDragDrop(&data, &source, DROPEFFECT_COPY, &mut effect);
        // `DRAGDROP_S_DROP`만 실제로 놓인 것이다 — 취소(`DRAGDROP_S_CANCEL`)와
        // 오류는 아무 일도 일어나지 않은 것과 같다
        result == DRAGDROP_S_DROP
    }
}

/// 끌기 동안 셸이 묻는 두 가지에 답하는 최소 구현 (FR-61).
///
/// 판정은 표준 그대로다 — `Esc`를 눌렀으면 취소, 왼쪽 버튼을 놓았으면 놓기,
/// 그 밖에는 계속. 커서 모양은 셸이 알아서 그리게 둔다(`DRAGDROP_S_USEDEFAULTCURSORS`는
/// `S_FALSE`가 아니라 그 상수여야 하는데, 기본 커서를 쓰겠다는 뜻이라 `S_OK`로 두면
/// 우리가 그려야 한다 — 그릴 것이 없으므로 기본을 쓴다)
#[implement(IDropSource)]
struct CopyDragSource;

impl IDropSource_Impl for CopyDragSource_Impl {
    fn QueryContinueDrag(
        &self,
        escape_pressed: BOOL,
        key_state: MODIFIERKEYS_FLAGS,
    ) -> windows::core::HRESULT {
        if escape_pressed.as_bool() {
            return DRAGDROP_S_CANCEL;
        }
        // 왼쪽 버튼을 놓았으면 그 자리에 놓는 것이다
        if (key_state & MK_LBUTTON) == MODIFIERKEYS_FLAGS(0) {
            return DRAGDROP_S_DROP;
        }
        S_OK
    }

    fn GiveFeedback(&self, _effect: DROPEFFECT) -> windows::core::HRESULT {
        // 기본 커서를 쓴다 — 우리가 그릴 것이 없다
        windows::Win32::Foundation::DRAGDROP_S_USEDEFAULTCURSORS
    }
}

/// 경로에서 얻은 PIDL들 — **떨어질 때 셸 힙에 되돌린다**.
///
/// `SHParseDisplayName`이 준 메모리는 부르는 쪽이 `CoTaskMemFree`로 풀어야 한다.
/// 중간에 실패해 일찍 돌아가는 길이 여럿이라, 손으로 풀면 한 갈래를 빠뜨려 샌다
struct Pidls(Vec<*const ITEMIDLIST>);

impl Pidls {
    /// 읽지 못하는 경로는 건너뛴다 — 여러 개를 끄는데 그 사이 하나가 사라졌다고
    /// 나머지를 버릴 이유가 없다
    fn parse(paths: &[PathBuf]) -> Pidls {
        let mut out = Vec::with_capacity(paths.len());
        for path in paths {
            let wide = HSTRING::from(path.as_os_str());
            let mut pidl: *mut ITEMIDLIST = std::ptr::null_mut();
            // 안전성: 유효한 널 종단 문자열과 스택의 포인터 하나를 넘긴다.
            // 성공했을 때만 담고, 담은 것은 `Drop`이 해제한다
            let ok = unsafe {
                SHParseDisplayName(PCWSTR(wide.as_ptr()), None, &mut pidl, 0, None).is_ok()
            };
            if ok && !pidl.is_null() {
                out.push(pidl as *const ITEMIDLIST);
            }
        }
        Pidls(out)
    }

    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    fn as_slice(&self) -> &[*const ITEMIDLIST] {
        &self.0
    }
}

impl Drop for Pidls {
    fn drop(&mut self) {
        for pidl in self.0.drain(..) {
            // 안전성: `SHParseDisplayName`이 셸 힙에 잡아 준 것을 같은 힙에 되돌린다.
            // 각 포인터는 여기서 한 번만 해제된다(`drain`이 목록을 비운다)
            unsafe {
                CoTaskMemFree(Some(pidl as *const core::ffi::c_void));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 끌_것이_없으면_시작하지_않는다() {
        // COM 호출에 닿기 전에 돌아간다 — 시험이 실제 드래그를 열지 않는다
        assert!(!start_copy_drag(&[]));
    }

    #[test]
    fn 읽지_못한_경로는_목록에서_빠진다() {
        // 그 사이 사라진 파일이 섞여도 나머지로 끈다. 하나도 못 얻으면 시작하지 않는다
        let 없는_경로 = PathBuf::from(r"C:\이런 폴더는 없다\없는 파일.txt");
        assert!(Pidls::parse(&[없는_경로]).is_empty());
    }
}
