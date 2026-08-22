//! 앱에서 탐색기·바탕화면으로 끌어내기 (FR-61 내보내기).
//!
//! **데이터 객체를 직접 구현하지 않는다** — 경로마다 PIDL을 얻어 셸이 만들어 주는
//! `IDataObject`를 쓴다(`SHCreateShellItemArrayFromIDLists` → `BindToHandler`). 그래야
//! 받는 쪽이 기대하는 형식(`CF_HDROP` 등)을 셸이 알아서 채운다.
//!
//! 우리가 **구현하는** COM 인터페이스는 `IDropSource` 하나이며 그마저 두 메서드뿐이다 —
//! 언제 그만둘지와 어떤 커서를 보일지. 아래의 드래그 이미지 관리자도 구현 대상이 아니다:
//! 그것은 셸이 내주는 것을 `CoCreateInstance`로 받아 쓴다.
//!
//! **끄는 동안 보일 그림도 셸에 맡긴다** — 끌기를 열기 전에 셸의 드래그 이미지 관리자
//! (`IDragSourceHelper`)에 첫 항목의 그림을 얹으면 그것이 커서를 따라온다. 그림을 만드는
//! 것은 `fs::drag_image`이고 여기서는 얹기만 한다. 어느 단계가 실패하든 **아무것도 알리지
//! 않고 그림 없이 끈다** — 그림은 거들 뿐이라 없다고 복사를 막을 이유가 없다.
//!
//! **`DoDragDrop`은 자기 메시지 루프를 돌린다** — 놓을 때까지 돌아오지 않으므로 부르는
//! 쪽은 셸 컨텍스트 메뉴(`TrackPopupMenuEx`)와 같은 자리, 즉 **그리기가 모두 끝난 뒤**에
//! 불러야 한다. 위젯 트리가 절반만 구성된 상태로 재진입시키면 안 된다
use std::path::PathBuf;

use windows::Win32::Foundation::{COLORREF, DRAGDROP_S_CANCEL, DRAGDROP_S_DROP, POINT, S_OK, SIZE};
use windows::Win32::System::Com::{
    CLSCTX_INPROC_SERVER, CoCreateInstance, CoTaskMemFree, IDataObject,
};
use windows::Win32::System::Ole::{
    DROPEFFECT, DROPEFFECT_COPY, DoDragDrop, IDropSource, IDropSource_Impl,
};
use windows::Win32::System::SystemServices::{MK_LBUTTON, MODIFIERKEYS_FLAGS};
use windows::Win32::UI::Shell::Common::ITEMIDLIST;
use windows::Win32::UI::Shell::{
    BHID_DataObject, CLSID_DragDropHelper, DSH_ALLOWDROPDESCRIPTIONTEXT, IDragSourceHelper,
    IDragSourceHelper2, SHCreateShellItemArrayFromIDLists, SHDRAGIMAGE, SHParseDisplayName,
};
use windows::core::{BOOL, HSTRING, Interface, PCWSTR, implement};

use crate::fs::drag_image;

/// 끌기를 시작한다 — 놓거나 취소할 때까지 **돌아오지 않는다** (FR-61).
///
/// 돌려주는 값은 "실제로 복사가 일어났는가"다. 경로를 하나도 셸 항목으로 만들지 못했으면
/// 시작조차 하지 않고 `false`다 — 그 사이 파일이 전부 사라졌거나 셸이 다루지 못하는
/// 경로다.
///
/// **효과는 복사만 허용한다**(`DROPEFFECT_COPY`) — FR-61이 내보내기도 복사로 못박았다.
/// 이동을 함께 허용하면 받는 쪽이 이동을 골랐을 때 원본이 사라진다.
///
/// `preview_px`는 끄는 동안 보일 그림의 한 변을 **물리 픽셀**로 청하는 값이다 — `fs`는 화면
/// 배율을 모르므로 부르는 쪽이 정해 내려보낸다
pub fn start_copy_drag(paths: &[PathBuf], preview_px: i32) -> bool {
    let Some(data) = data_object(paths) else {
        return false;
    };
    // 안전성: 아래 호출은 모두 COM이 STA로 초기화된 UI 스레드에서 돌고(`ui::app`이
    // 그리기를 마친 뒤 부른다), 얻은 인터페이스는 이 함수 안에서만 살다 `Drop`으로 해제된다
    unsafe {
        // 끌기를 열기 전에 그림을 얹는다 — 실패해도 아래 흐름은 그대로다
        attach_drag_image(&data, paths, preview_px);
        let source: IDropSource = CopyDragSource.into();
        let mut effect = DROPEFFECT::default();
        let result = DoDragDrop(&data, &source, DROPEFFECT_COPY, &mut effect);
        // `DRAGDROP_S_DROP`만 실제로 놓인 것이다 — 취소(`DRAGDROP_S_CANCEL`)와
        // 오류는 아무 일도 일어나지 않은 것과 같다
        result == DRAGDROP_S_DROP
    }
}

/// 경로 목록을 담은 **셸이 만든 데이터 객체**를 얻는다 (FR-61·FR-64).
///
/// 받는 쪽이 기대하는 형식(`CF_HDROP`·`CFSTR_SHELLIDLIST` 등)을 셸이 알아서 채우므로
/// 우리가 `IDataObject`를 구현하지 않는다(이 모듈 첫머리의 그 판단이다).
///
/// **끌기(FR-61)와 클립보드(FR-64)가 함께 쓴다** — 두 경로가 같은 객체를 원하고, 각자
/// 만들면 한쪽이 채우는 형식이 조용히 달라진다. 읽지 못하는 경로가 섞여 있으면 그것만
/// 빠지고, 하나도 읽지 못했으면 `None`이다
pub(crate) fn data_object(paths: &[PathBuf]) -> Option<IDataObject> {
    if paths.is_empty() {
        return None;
    }
    let pidls = Pidls::parse(paths);
    if pidls.is_empty() {
        return None;
    }
    // 안전성: COM이 STA로 초기화된 스레드에서만 부른다. PIDL은 `Pidls`가 소유해 이 함수를
    // 벗어날 때 `CoTaskMemFree`로 되돌리며, 셸이 만든 객체는 그것과 무관하게 산다
    unsafe {
        let items = SHCreateShellItemArrayFromIDLists(pidls.as_slice()).ok()?;
        items
            .BindToHandler::<_, IDataObject>(None, &BHID_DataObject)
            .ok()
    }
}

/// 끌기 동안 셸이 묻는 두 가지에 답하는 최소 구현 (FR-61).
///
/// 판정은 표준 그대로다 — `Esc`를 눌렀으면 취소, 왼쪽 버튼을 놓았으면 놓기,
/// 그 밖에는 계속. 커서 모양은 셸이 알아서 그리게 둔다 — `DRAGDROP_S_USEDEFAULTCURSORS`는
/// 기본 커서를 쓰겠다는 뜻이고, **드래그 이미지 관리자가 얹은 그림이 보이려면 바로 이 값이어야
/// 한다**(`S_OK`로 두면 커서를 우리가 그리겠다는 뜻이 되어 그림도 함께 사라진다)
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
        // 기본 커서를 쓴다 — 그림은 셸의 드래그 이미지 관리자가 그린다
        windows::Win32::Foundation::DRAGDROP_S_USEDEFAULTCURSORS
    }
}

/// 끄는 동안 보일 그림을 셸의 드래그 이미지 관리자에 얹는다 (FR-61).
///
/// **여러 개를 끌어도 첫 항목 하나만** 보인다(2026-08-21 사용자 결정 — 여러 장을 겹쳐 쌓지
/// 않는다). 개수는 셸이 그리는 드롭 설명 문구가 대신 알린다.
///
/// **헬퍼를 먼저 얻고 그림은 그 다음에 만든다** — 순서가 반대면 그림을 만들어 놓고 헬퍼가
/// 없어 버리는 갈래가 생긴다.
///
/// 안전성: COM이 초기화된 UI 스레드에서만 부른다(`start_copy_drag`이 그 자리다). 얻은
/// 인터페이스는 이 함수 안에서만 살다 `Drop`으로 해제되고, **얹지 못한 비트맵은 여기서
/// 되돌린다** — 얹은 뒤에는 지우지 않는다(소유권이 넘어갔는지 문서에 서술이 없어, 재활용된
/// 핸들을 남의 것과 함께 지우는 쪽이 더 위험하다)
unsafe fn attach_drag_image(data: &IDataObject, paths: &[PathBuf], preview_px: i32) {
    unsafe {
        let Some(first) = paths.first() else {
            return;
        };
        let Ok(helper) = CoCreateInstance::<_, IDragSourceHelper>(
            &CLSID_DragDropHelper,
            None,
            CLSCTX_INPROC_SERVER,
        ) else {
            return;
        };
        // `복사 → 바탕 화면` 같은 설명 문구를 함께 켠다. 문구는 윈도우가 자기 언어로 그리므로
        // 우리 카탈로그에 더할 것이 없다. 이 인터페이스가 없는 환경이면 그림만 붙는다
        if let Ok(helper2) = helper.cast::<IDragSourceHelper2>() {
            let _ = helper2.SetFlags(DSH_ALLOWDROPDESCRIPTIONTEXT.0 as u32);
        }
        let Some(image) = drag_image::build(first, preview_px) else {
            return;
        };
        let shdi = SHDRAGIMAGE {
            // 셸이 청한 것보다 작은 그림을 줄 수 있어 **실제 크기**를 적는다
            sizeDragImage: SIZE {
                cx: image.width,
                cy: image.height,
            },
            // 커서를 그림 한가운데 둔다 — 끌기를 시작한 지점의 항목 안 좌표를 알 수 없어
            // (경로 목록만 받는다) 한가운데가 치우침 없는 기본값이다
            ptOffset: POINT {
                x: image.width / 2,
                y: image.height / 2,
            },
            hbmpDragImage: image.bitmap,
            // 32bpp 알파 비트맵이라 투명은 알파가 정한다 — 색으로 뚫을 자리가 없다(`CLR_NONE`)
            crColorKey: COLORREF(0xFFFF_FFFF),
        };
        if helper.InitializeFromBitmap(&shdi, data).is_err() {
            image.delete();
        }
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
        assert!(!start_copy_drag(&[], 96));
    }

    #[test]
    fn 읽지_못한_경로는_목록에서_빠진다() {
        // 그 사이 사라진 파일이 섞여도 나머지로 끈다. 하나도 못 얻으면 시작하지 않는다
        let 없는_경로 = PathBuf::from(r"C:\이런 폴더는 없다\없는 파일.txt");
        assert!(Pidls::parse(&[없는_경로]).is_empty());
    }
}
