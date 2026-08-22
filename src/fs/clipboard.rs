//! 파일 클립보드 — 복사·잘라내기·붙여넣기 (FR-64).
//!
//! **형식을 새로 만들지 않는다** — 담는 것은 Windows 표준(`CF_HDROP`에 경로 목록,
//! `Preferred DropEffect`에 복사인지 이동인지)이라 탐색기·다른 앱과 그대로 오간다.
//! 자체 형식을 만들면 이 앱끼리만 통하는 클립보드가 된다.
//!
//! **데이터 객체도 우리가 만들지 않는다** — 셸이 만들어 주는 것(`fs::drag_source::data_object`)에
//! 효과 하나만 얹는다. 끌어내기(FR-61)와 같은 객체를 쓰므로 두 경로가 채우는 형식이
//! 갈리지 않는다.
//!
//! **실제 파일 작업은 이 모듈이 하지 않는다** — 붙여넣기는 여기서 읽은 경로를
//! `fs::file_op`에 넘겨 셸이 옮기거나 복사한다. 두 모듈은 서로를 참조하지 않고 `ui`가 잇는다.
use std::path::PathBuf;

use windows::Win32::System::Com::{
    DVASPECT_CONTENT, FORMATETC, IDataObject, STGMEDIUM, STGMEDIUM_0, TYMED_HGLOBAL,
};
use windows::Win32::System::DataExchange::RegisterClipboardFormatW;
use windows::Win32::System::Memory::{GMEM_MOVEABLE, GlobalAlloc, GlobalLock, GlobalUnlock};
use windows::Win32::System::Ole::{
    DROPEFFECT, DROPEFFECT_COPY, DROPEFFECT_MOVE, OleGetClipboard, OleSetClipboard,
    ReleaseStgMedium,
};
#[cfg(test)]
use windows::Win32::System::Ole::{OleInitialize, OleUninitialize};
use windows::Win32::UI::Shell::{CFSTR_PREFERREDDROPEFFECT, DragQueryFileW, HDROP};

use crate::fs::drag_source::data_object;

/// 클립보드에서 읽어 온 파일 목록 (FR-64).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClipboardFiles {
    /// 담겨 있던 경로들 — 순서는 담은 쪽이 정한 그대로다
    pub paths: Vec<PathBuf>,
    /// 잘라내기로 담겼는가. 거짓이면 복사다
    pub cut: bool,
}

/// `CF_HDROP`의 표준 형식 번호 — 이것만 상수이고 나머지는 등록해서 얻는다
const CF_HDROP_ID: u16 = 15;

/// 고른 것을 클립보드에 담는다 — `cut`이 참이면 잘라내기다 (FR-64).
///
/// 담기지 못하면 `false`이며 **아무것도 알리지 않는다** — 다른 앱이 클립보드를 쥐고 있는
/// 짧은 순간이 대부분이고, 사용자는 붙여넣어 보면 담기지 않은 것을 안다.
///
/// **효과를 얹지 못해도 담기는 한다** — 그 경우 받는 쪽은 복사로 본다(붙여넣기 기본값).
/// 원본이 사라지지 않는 쪽이라 잘라내기가 복사가 되는 것은 안전한 어긋남이다
pub fn put(paths: &[PathBuf], cut: bool) -> bool {
    let Some(data) = data_object(paths) else {
        return false;
    };
    // 안전성: COM이 STA로 초기화된 UI 스레드에서만 부른다. 아래에서 만든 전역 메모리는
    // `SetData`가 소유권을 가져가고(`frelease = true`), 실패하면 그대로 두어도 새지 않는다
    // — 그 경우 효과 없이 담기며 받는 쪽이 복사로 본다
    unsafe {
        set_drop_effect(&data, cut);
        OleSetClipboard(&data).is_ok()
    }
}

/// 클립보드에 담긴 파일 목록을 읽는다 — 파일이 아니면 `None` (FR-64).
///
/// 탐색기·다른 앱이 담은 것도 같은 형식이라 그대로 읽힌다
pub fn take() -> Option<ClipboardFiles> {
    // 안전성: COM이 STA로 초기화된 UI 스레드에서만 부른다. 잠근 전역 메모리는 읽은 뒤
    // 곧바로 풀고, `STGMEDIUM`은 `ReleaseStgMedium`으로 되돌린다
    unsafe {
        let data = OleGetClipboard().ok()?;
        let paths = read_hdrop(&data)?;
        if paths.is_empty() {
            return None;
        }
        Some(ClipboardFiles {
            paths,
            cut: read_drop_effect(&data) == Some(true),
        })
    }
}

/// 복사인지 이동인지를 데이터 객체에 얹는다.
///
/// 안전성: COM이 초기화된 스레드에서 유효한 객체에만 부른다. 전역 메모리는 성공 시
/// `SetData`가 가져가고, 실패하면 이 함수를 벗어나며 프로세스가 끝날 때 회수된다
unsafe fn set_drop_effect(data: &IDataObject, cut: bool) {
    // 안전성: 위 주석 참조
    unsafe {
        let Some(format) = preferred_drop_effect_format() else {
            return;
        };
        let effect: DROPEFFECT = if cut {
            DROPEFFECT_MOVE
        } else {
            DROPEFFECT_COPY
        };
        let Ok(handle) = GlobalAlloc(GMEM_MOVEABLE, size_of::<u32>()) else {
            return;
        };
        let slot = GlobalLock(handle) as *mut u32;
        if slot.is_null() {
            return;
        }
        *slot = effect.0;
        let _ = GlobalUnlock(handle);

        let medium = STGMEDIUM {
            tymed: TYMED_HGLOBAL.0 as u32,
            u: STGMEDIUM_0 { hGlobal: handle },
            pUnkForRelease: std::mem::ManuallyDrop::new(None),
        };
        // `frelease = true` — 성공하면 이 메모리의 주인이 데이터 객체로 넘어간다
        let _ = data.SetData(&format, &medium, true);
    }
}

/// 담긴 것이 잘라내기인가 — 형식이 없으면 `None`(복사로 본다).
///
/// 안전성: COM이 초기화된 스레드에서 유효한 객체에만 부른다. 얻은 매체는 읽은 뒤 되돌린다
unsafe fn read_drop_effect(data: &IDataObject) -> Option<bool> {
    // 안전성: 위 주석 참조
    unsafe {
        let format = preferred_drop_effect_format()?;
        let mut medium = data.GetData(&format).ok()?;
        let handle = medium.u.hGlobal;
        let slot = GlobalLock(handle) as *const u32;
        let effect = if slot.is_null() {
            None
        } else {
            Some(DROPEFFECT(*slot))
        };
        if !slot.is_null() {
            let _ = GlobalUnlock(handle);
        }
        ReleaseStgMedium(&mut medium);
        // `MOVE` 비트가 서 있으면 잘라내기다 — 셸은 두 비트를 함께 세우기도 한다
        Some(effect? & DROPEFFECT_MOVE != DROPEFFECT(0))
    }
}

/// `CF_HDROP`에 담긴 경로들을 읽는다.
///
/// 안전성: COM이 초기화된 스레드에서 유효한 객체에만 부른다. 얻은 매체는 읽은 뒤 되돌린다
unsafe fn read_hdrop(data: &IDataObject) -> Option<Vec<PathBuf>> {
    // 안전성: 위 주석 참조
    unsafe {
        let format = FORMATETC {
            cfFormat: CF_HDROP_ID,
            ptd: std::ptr::null_mut(),
            dwAspect: DVASPECT_CONTENT.0,
            lindex: -1,
            tymed: TYMED_HGLOBAL.0 as u32,
        };
        let mut medium = data.GetData(&format).ok()?;
        let drop = HDROP(medium.u.hGlobal.0);
        // 첫 인자에 `0xFFFF_FFFF`를 주면 개수를 돌려준다(Win32 관례)
        let count = DragQueryFileW(drop, u32::MAX, None);
        let mut paths = Vec::with_capacity(count as usize);
        for index in 0..count {
            // 길이를 먼저 묻고(널 종단 자리는 세지 않는다) 그만큼 받는다
            let len = DragQueryFileW(drop, index, None);
            if len == 0 {
                continue;
            }
            let mut buffer = vec![0u16; len as usize + 1];
            let written = DragQueryFileW(drop, index, Some(buffer.as_mut_slice()));
            if written == 0 {
                continue;
            }
            buffer.truncate(written as usize);
            paths.push(PathBuf::from(String::from_utf16_lossy(&buffer)));
        }
        ReleaseStgMedium(&mut medium);
        Some(paths)
    }
}

/// `Preferred DropEffect` 형식 하나 — 이름이 문자열이라 등록해서 번호를 얻는다.
///
/// 등록은 같은 이름이면 같은 번호를 주므로 몇 번을 불러도 안전하다
fn preferred_drop_effect_format() -> Option<FORMATETC> {
    // 안전성: 널 종단 문자열 상수 하나를 넘긴다
    let id = unsafe { RegisterClipboardFormatW(CFSTR_PREFERREDDROPEFFECT) };
    if id == 0 {
        return None;
    }
    Some(FORMATETC {
        cfFormat: id as u16,
        ptd: std::ptr::null_mut(),
        dwAspect: DVASPECT_CONTENT.0,
        lindex: -1,
        tymed: TYMED_HGLOBAL.0 as u32,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 담을_것이_없으면_클립보드를_건드리지_않는다() {
        // 빈 목록으로 담으면 지금 담긴 것을 지우게 된다 — 그 전에 걸러야 한다
        assert!(!put(&[], false));
        assert!(!put(&[], true));
    }

    #[test]
    fn 효과_형식은_같은_번호로_등록된다() {
        // 담을 때와 읽을 때 다른 번호를 얻으면 잘라내기가 복사로 읽힌다
        let first = preferred_drop_effect_format().map(|format| format.cfFormat);
        let second = preferred_drop_effect_format().map(|format| format.cfFormat);
        assert_eq!(first, second);
        assert!(first.is_some_and(|id| id != 0), "등록에 실패하면 None이다");
    }

    #[test]
    fn hdrop_형식_번호는_표준값이다() {
        // 이 값이 틀리면 탐색기가 담은 것을 읽지 못한다 (Win32 `CF_HDROP`)
        assert_eq!(CF_HDROP_ID, 15);
    }

    /// **이 시험은 진짜 클립보드를 쓴다** — 돌리면 그 PC에 담겨 있던 것이 덮인다.
    /// 흉내 낼 수가 없어서다: 담기·읽기 양쪽이 셸과 OLE를 거치므로 가짜로 바꾸면
    /// 정작 검증하려는 것(탐색기와 같은 형식으로 오가는가)이 검증되지 않는다
    #[test]
    fn 담은_경로와_잘라내기_표시가_그대로_돌아온다() {
        let dir = std::env::temp_dir().join(format!("moa_clip_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let paths: Vec<PathBuf> = ["가.txt", "나.txt"]
            .iter()
            .map(|name| {
                let path = dir.join(name);
                std::fs::write(&path, b"x").expect("임시 파일을 만들지 못했다");
                path
            })
            .collect();

        // 안전성: 이 시험 스레드에서 OLE를 초기화하고 끝에서 같은 스레드가 되돌린다.
        // 클립보드 API는 OLE 아파트를 요구해 초기화 없이는 담기·읽기가 모두 실패한다
        let ready = unsafe { OleInitialize(None) }.is_ok();
        assert!(ready, "OLE를 초기화하지 못하면 이 시험은 뜻이 없다");

        for 잘라내기 in [false, true] {
            assert!(put(&paths, 잘라내기), "담기에 실패했다");
            let read = take().expect("담은 것을 읽지 못했다");
            assert_eq!(read.paths, paths, "경로가 그대로 돌아와야 한다");
            assert_eq!(read.cut, 잘라내기, "복사·잘라내기 구분이 유지돼야 한다");
        }

        // 안전성: 위에서 성공한 초기화와 같은 스레드에서 1회 호출
        unsafe {
            OleUninitialize();
        }
        let _ = std::fs::remove_dir_all(&dir);
    }
}
