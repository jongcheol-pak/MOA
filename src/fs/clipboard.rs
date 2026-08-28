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

use windows::Win32::Foundation::GlobalFree;
#[cfg(test)]
use windows::Win32::Foundation::HGLOBAL;
use windows::Win32::System::Com::{
    DVASPECT_CONTENT, FORMATETC, IDataObject, STGMEDIUM, STGMEDIUM_0, TYMED_HGLOBAL,
};
#[cfg(test)]
use windows::Win32::System::DataExchange::GetClipboardData;
use windows::Win32::System::DataExchange::{
    CloseClipboard, EmptyClipboard, OpenClipboard, RegisterClipboardFormatW, SetClipboardData,
};
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

/// `CF_UNICODETEXT`의 표준 형식 번호 — `CF_HDROP`과 같은 취급이라 상수로 둔다
const CF_UNICODETEXT_ID: u32 = 13;

/// 글자를 클립보드에 담는다 — 원격 항목의 경로 복사가 쓴다 (FR-39).
///
/// **`put`과 형식이 다르다** — 그쪽은 셸이 만든 데이터 객체에 파일 목록을 싣지만
/// 여기는 글자뿐이라 전역 메모리 한 장을 직접 담는다. 원격 경로는 이 PC의 파일이 아니라
/// 셸이 가리킬 대상이 없다.
///
/// 담기지 못하면 `false`이며 **아무것도 알리지 않는다** — 이유·처리는 `put`과 같다
pub fn put_text(text: &str) -> bool {
    let mut wide: Vec<u16> = text.encode_utf16().collect();
    wide.push(0);
    let bytes = std::mem::size_of_val(wide.as_slice());
    // 안전성: 클립보드를 열면 반드시 닫는다. 담은 전역 메모리는 `SetClipboardData`가 성공하면
    // 소유권을 가져가므로 그때는 풀지 않고, 실패한 길에서는 우리가 `GlobalFree`로 돌려준다
    unsafe {
        let Ok(handle) = GlobalAlloc(GMEM_MOVEABLE, bytes) else {
            return false;
        };
        let slot = GlobalLock(handle) as *mut u16;
        if slot.is_null() {
            let _ = GlobalFree(Some(handle));
            return false;
        }
        std::ptr::copy_nonoverlapping(wide.as_ptr(), slot, wide.len());
        let _ = GlobalUnlock(handle);
        if OpenClipboard(None).is_err() {
            let _ = GlobalFree(Some(handle));
            return false;
        }
        let _ = EmptyClipboard();
        let ok = SetClipboardData(
            CF_UNICODETEXT_ID,
            Some(windows::Win32::Foundation::HANDLE(handle.0)),
        )
        .is_ok();
        let _ = CloseClipboard();
        if !ok {
            // 셸이 가져가지 못했으니 우리 몫이다 — 놓아두면 프로세스가 끝날 때까지 남는다
            let _ = GlobalFree(Some(handle));
        }
        ok
    }
}

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
    // 안전성: COM이 STA로 초기화된 UI 스레드에서만 부른다. 효과를 얹는 쪽의 메모리 수명은
    // `set_drop_effect`가 자기 안에서 닫는다(그 doc 참조) — 얹지 못했으면 효과 없이 담기고
    // 받는 쪽이 복사로 본다
    unsafe {
        set_drop_effect(&data, cut);
        OleSetClipboard(&data).is_ok()
    }
}

/// 클립보드가 지금 든 것으로 **화면에 흐리게 그릴 경로**를 정한다 (FR-64).
///
/// 잘라내기로 담긴 것만 표시 대상이다 — 복사로 담겼거나(원본이 그대로 남는다) 담긴 것이
/// 파일이 아니면(`None`) 빈 목록이며, 그것이 곧 **표시를 푼다**는 뜻이다.
///
/// **다른 앱이 클립보드를 가져간 경우가 이 함수의 존재 이유다** — 우리가 잘라낸 뒤 탐색기에서
/// `Ctrl+C`를 누르면 우리 화면의 흐린 줄은 이미 클립보드에 없는 것을 가리킨다. 그것을 그대로
/// 두면 영영 흐린 채 남는다.
///
/// **프레임마다 묻지 않는다** — `OleGetClipboard`는 COM 호출이라 매 프레임 도는 자리에 둘 수
/// 없다. 부르는 쪽이 붙여넣기·담기 시점에만 묻는다
pub fn cut_marks_for(files: Option<&ClipboardFiles>) -> &[PathBuf] {
    match files {
        Some(files) if files.cut => &files.paths,
        _ => &[],
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
/// 안전성: COM이 초기화된 스레드에서 유효한 객체에만 부른다. 잡은 전역 메모리는 **셸에
/// 넘기기 전에 실패하면 이 함수가 되돌리고**(`GlobalFree`), 넘긴 뒤에는 건드리지 않는다 —
/// `SetData(frelease = true)`가 성공하면 주인이 데이터 객체이고, 실패했을 때 누가 주인인지는
/// 구현체마다 갈려 어느 쪽으로 정하든 한쪽은 틀린다. 4바이트 하나라 **놓아두는 쪽**을 골랐다
/// (두 번 풀면 그 즉시 프로세스가 죽는다)
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
            // 아직 셸에 넘기기 전이라 주인이 우리다 — 되돌리지 않으면 그대로 샌다
            let _ = GlobalFree(Some(handle));
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

/// 받은 매체가 정말 전역 메모리인가 — 아니면 되돌리고 거짓을 준다.
///
/// **클립보드는 외부 입력이다** — 담은 것이 어느 앱인지 우리가 고르지 못한다. `FORMATETC`의
/// `tymed`는 **청하는** 값일 뿐이라, 규격을 지키지 않는 앱은 스트림·비트맵 같은 다른 매체를
/// 돌려줄 수 있다. 그것을 확인 없이 `hGlobal`로 읽으면 union의 다른 변형(`IStream` 포인터
/// 등)을 핸들로 오독해 그 값을 `GlobalLock`·`DragQueryFileW`에 넘기게 된다.
///
/// 안전성: `GetData`가 채운 유효한 매체에만 부른다
unsafe fn is_global_medium(medium: &mut STGMEDIUM) -> bool {
    if medium.tymed == TYMED_HGLOBAL.0 as u32 {
        return true;
    }
    // 안전성: 우리가 읽지 않을 매체라도 받은 것은 되돌린다 — 어느 변형이든 이 함수가 푼다
    unsafe {
        ReleaseStgMedium(medium);
    }
    false
}

/// 담긴 것이 잘라내기인가 — 형식이 없으면 `None`(복사로 본다).
///
/// 안전성: COM이 초기화된 스레드에서 유효한 객체에만 부른다. 얻은 매체는 읽은 뒤 되돌린다
unsafe fn read_drop_effect(data: &IDataObject) -> Option<bool> {
    // 안전성: 위 주석 참조
    unsafe {
        let format = preferred_drop_effect_format()?;
        let mut medium = data.GetData(&format).ok()?;
        if !is_global_medium(&mut medium) {
            return None;
        }
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
        if !is_global_medium(&mut medium) {
            return None;
        }
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

    /// 이 PC의 **진짜 클립보드**를 쓰는 시험을 켜는 환경변수.
    ///
    /// 기본은 꺼짐이다 — `cargo test`를 돌릴 때마다 사용자가 어딘가에 붙여넣으려던 것이
    /// 조용히 덮이면 안 된다. 원격 실서버 시험을 환경변수로 여는 것과 같은 관례다
    /// (AGENTS 「원격 기능 테스트」). 켜려면:
    ///
    /// ```text
    /// $env:MOA_TEST_CLIPBOARD = "1"; cargo test fs::clipboard
    /// ```
    const 클립보드_시험_스위치: &str = "MOA_TEST_CLIPBOARD";

    /// 클립보드를 만지는 시험을 한 번에 하나씩 돌리는 잠금 (`i18n`의 시험 잠금과 같은 방식)
    static CLIPBOARD_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    /// 클립보드를 만지는 시험 본문을 **자기 스레드에서** 돌린다.
    ///
    /// 스레드를 새로 여는 이유는 COM **아파트**가 스레드마다 하나이기 때문이다 — 같은
    /// 프로세스의 다른 시험이 먼저 그 스레드를 MTA로 잡아 두면 `OleInitialize`가
    /// `RPC_E_CHANGED_MODE`로 실패한다(단독 실행은 통과하는데 전체 실행에서만 깨지는
    /// 형태로 실제 관측됐다). 새 스레드는 아파트가 비어 있어 그 다툼이 없다.
    ///
    /// 스위치가 꺼져 있으면 **아무것도 하지 않고 통과**한다
    fn 클립보드_시험(body: impl FnOnce() + Send + 'static) {
        if std::env::var_os(클립보드_시험_스위치).is_none() {
            return;
        }
        // 클립보드는 **OS에 하나뿐인 자원**이라 이것을 만지는 시험은 서로 배제해야 한다 —
        // 한쪽이 담고 읽는 사이에 다른 쪽이 비우면 간헐적으로 깨진다. 아래의 전용 스레드는
        // COM 아파트를 가르는 장치일 뿐 이 다툼은 막지 못하므로 **둘 다** 필요하다.
        // 앞 시험이 단언에 실패해 잠금이 오염됐어도 이어서 돈다(각 시험이 클립보드를 새로 세운다)
        let _순서 = CLIPBOARD_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let worker = std::thread::spawn(move || {
            // 안전성: 방금 만든 스레드라 아파트가 비어 있다. 초기화에 성공했을 때만
            // 같은 스레드에서 1회 되돌린다
            let ready = unsafe { OleInitialize(None) }.is_ok();
            assert!(ready, "새 스레드인데도 OLE를 초기화하지 못했다");
            // **단언이 깨져도 정리는 반드시 한다** — 그러지 않으면 시험이 실패한 그 순간의
            // 클립보드(임시 경로 목록)가 사용자 PC에 그대로 남는다. 실패는 아래에서 다시 던진다
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(body));
            // 담아 둔 것을 남기지 않는다 — 시험이 끝나면 클립보드는 비어 있다
            let _ = 클립보드를_비운다();
            // 안전성: 위에서 성공한 초기화와 짝지은 1회 호출
            unsafe {
                OleUninitialize();
            }
            if let Err(panic) = result {
                std::panic::resume_unwind(panic);
            }
        });
        // 시험 스레드의 단언 실패를 이쪽으로 옮긴다 — 그러지 않으면 조용히 통과한다
        if let Err(panic) = worker.join() {
            std::panic::resume_unwind(panic);
        }
    }

    /// 클립보드를 비운다 — 성공했는지 돌려준다
    fn 클립보드를_비운다() -> bool {
        // 안전성: 소유자 창 없이 열고 반드시 닫는다. 창을 주지 않아도 이 프로세스가 주인이 된다
        unsafe {
            if OpenClipboard(None).is_err() {
                return false;
            }
            let emptied = EmptyClipboard().is_ok();
            let _ = CloseClipboard();
            emptied
        }
    }

    #[test]
    fn 담을_것이_없으면_클립보드를_건드리지_않는다() {
        // 빈 목록으로 담으면 지금 담긴 것을 지우게 된다 — 그 전에 걸러야 한다.
        // 고른 것 없이 `Ctrl+C`·`Ctrl+X`를 누르는 길이 여기로 온다 (FR-12)
        assert!(!put(&[], false));
        assert!(!put(&[], true));
    }

    #[test]
    fn 표시_대상은_잘라내기로_담긴_것뿐이다() {
        // FR-64 세 번째 해제 조건 — 다른 앱이 클립보드를 가져가면 우리 표시도 따라간다
        let 잘라냄 = ClipboardFiles {
            paths: vec![PathBuf::from(r"C:\일\a.txt"), PathBuf::from(r"C:\일\b.txt")],
            cut: true,
        };
        assert_eq!(cut_marks_for(Some(&잘라냄)), 잘라냄.paths.as_slice());

        // 다른 앱이 **복사**로 담았다 — 원본이 그대로 남으므로 흐리게 그릴 것이 없다
        let 복사됨 = ClipboardFiles {
            paths: vec![PathBuf::from(r"C:\남의\c.txt")],
            cut: false,
        };
        assert!(cut_marks_for(Some(&복사됨)).is_empty());

        // 다른 앱이 **다른 것을 잘라냈다** — 표시가 그쪽으로 옮겨 간다
        let 남의_잘라내기 = ClipboardFiles {
            paths: vec![PathBuf::from(r"D:\다른\d.txt")],
            cut: true,
        };
        assert_eq!(
            cut_marks_for(Some(&남의_잘라내기)),
            남의_잘라내기.paths.as_slice()
        );

        // 클립보드에 파일이 없다(글자만 담겼거나 다른 앱이 쥐고 있어 읽지 못했다)
        assert!(cut_marks_for(None).is_empty());
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

        클립보드_시험(move || {
            for 잘라내기 in [false, true] {
                assert!(put(&paths, 잘라내기), "담기에 실패했다");
                let read = take().expect("담은 것을 읽지 못했다");
                assert_eq!(read.paths, paths, "경로가 그대로 돌아와야 한다");
                assert_eq!(read.cut, 잘라내기, "복사·잘라내기 구분이 유지돼야 한다");
            }
        });
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// **이 시험도 진짜 클립보드를 쓴다** (위 왕복 시험과 같은 이유·같은 잠금)
    #[test]
    fn 파일이_담기지_않은_클립보드는_읽지_않는다() {
        클립보드_시험(|| {
            // ① 아무것도 없는 클립보드
            assert!(클립보드를_비운다(), "클립보드를 비우지 못했다");
            assert_eq!(take(), None, "빈 클립보드에서는 읽을 것이 없다");

            // ② 파일이 아닌 것만 담긴 클립보드 — 글자를 담아 본다.
            //    `CF_UNICODETEXT`(13)는 `CF_HDROP`(15)이 아니므로 `GetData`가 실패해야 한다.
            //    **본체의 `put_text`를 그대로 쓴다** — 같은 일을 하는 헬퍼를 시험에 따로 두면
            //    본체가 바뀔 때 이 갈래가 옛 방식을 계속 확인하게 된다
            assert!(
                put_text("보고서"),
                "글자를 담지 못해 이 갈래를 확인할 수 없다"
            );
            assert_eq!(take(), None, "글자만 있는 클립보드는 파일 목록이 아니다");
        });
    }

    /// **이 시험도 진짜 클립보드를 쓴다** (위 왕복 시험과 같은 이유·같은 잠금).
    ///
    /// `take`는 파일 전용이라 담은 글자를 되읽지 못하므로 **이 시험이 직접 읽는다** —
    /// 담기는 것만 보면 "무엇이 담겼는지"는 확인되지 않아, 경로가 아닌 것이 담겨도 통과한다
    #[test]
    fn 담은_글자가_그대로_돌아온다() {
        클립보드_시험(|| {
            for 원본 in [
                "/pub/upload/보고서.txt",
                // 여러 줄 — `join_paths`가 만드는 모양이다
                "/pub/a.txt\r\n/pub/하위 폴더\r\n/b.bin",
                // 빈 글자도 담긴다 (고른 것이 없을 때의 방어는 부르는 쪽이 한다)
                "",
            ] {
                assert!(클립보드를_비운다(), "클립보드를 비우지 못했다");
                assert!(put_text(원본), "글자를 담지 못했다: {원본:?}");
                assert_eq!(
                    글자를_읽는다().as_deref(),
                    Some(원본),
                    "담은 글자가 그대로 돌아오지 않았다"
                );
                // 파일 목록이 아니므로 파일 읽기로는 잡히지 않아야 한다
                assert_eq!(take(), None, "글자는 파일 목록이 아니다");
            }
        });
    }

    /// 클립보드의 `CF_UNICODETEXT`를 되읽는다 — **시험 전용**이다.
    ///
    /// 본체에 두지 않는 이유: 앱은 글자를 담기만 하고 읽지는 않는다(읽는 것은 붙여넣는
    /// 다른 프로그램이다). 쓰지 않을 공개 함수를 만들지 않는다
    fn 글자를_읽는다() -> Option<String> {
        // 안전성: 클립보드를 열면 반드시 닫는다. `GetClipboardData`가 준 핸들은 **빌려 온 것**이라
        // 풀지 않으며(클립보드가 주인이다) 닫기 전에만 유효해 그 안에서 복사해 나온다
        unsafe {
            if OpenClipboard(None).is_err() {
                return None;
            }
            let text = GetClipboardData(CF_UNICODETEXT_ID).ok().and_then(|handle| {
                let slot = GlobalLock(HGLOBAL(handle.0)) as *const u16;
                if slot.is_null() {
                    return None;
                }
                let mut len = 0usize;
                while *slot.add(len) != 0 {
                    len += 1;
                }
                let text = String::from_utf16_lossy(std::slice::from_raw_parts(slot, len));
                let _ = GlobalUnlock(HGLOBAL(handle.0));
                Some(text)
            });
            let _ = CloseClipboard();
            text
        }
    }
}
