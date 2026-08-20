//! 로컬 파일·폴더 복사 (FR-60·FR-61) — **Windows 셸에 맡긴다**.
//!
//! 직접 재귀 복사를 만들지 않고 `IFileOperation`을 부르는 이유가 이 모듈의 전부다:
//! 진행률 대화·같은 이름 충돌 대화·되돌리기(`Ctrl+Z`)·권한 승격·긴 경로·심볼릭 링크를
//! 전부 OS가 처리한다. PRD의 「자체 파일 작업 UI는 셸에 위임」 원칙과 같은 방향이며,
//! 그래서 로컬끼리의 복사에는 FR-55의 자체 확인 대화를 띄우지 않는다(셸이 자기 대화로 묻는다).
//!
//! **일은 워커 스레드에서 한다** — `PerformOperations`는 복사가 끝날 때까지 돌아오지 않아
//! UI 스레드에서 부르면 대용량 복사 내내 앱이 굳는다(AGENTS: UI 스레드 블로킹 I/O 금지).
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc::Sender;

use windows::Win32::Foundation::HWND;
use windows::Win32::System::Com::{
    CLSCTX_ALL, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx, CoUninitialize,
};
use windows::Win32::UI::Shell::{
    FOF_ALLOWUNDO, FOF_NOCONFIRMMKDIR, FileOperation, IFileOperation, IShellItem,
    SHCreateItemFromParsingName,
};
use windows::core::{HSTRING, PCWSTR};

/// 복사 한 번의 결과 — 화면이 알릴 것만 담는다.
///
/// **성공 건수를 세지 않는다** — 셸이 자기 진행률 대화로 이미 알렸고, 몇 개가 실제로
/// 복사됐는지는 사용자가 충돌 대화에서 무엇을 골랐는지에 달려 셸만 안다. 앱이 알아야
/// 하는 것은 "탈이 났는가"뿐이다
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CopyOutcome {
    /// 복사를 걸었던 항목 수 — 알림 문구가 규모를 적는 데 쓴다
    pub requested: usize,
    /// 사용자가 셸 대화에서 그만뒀는가. **오류가 아니다**
    pub cancelled: bool,
    /// 셸이 준 실패 사유 — 없으면 탈 없이 끝났다
    pub error: Option<String>,
}

/// 워커를 깨워 화면을 다시 그리게 하는 손잡이 — `remote::connection::Wake`와 같은 모양이다
pub type Wake = Arc<dyn Fn() + Send + Sync>;

/// HWND를 워커 스레드로 넘기기 위한 래퍼.
///
/// 안전성: HWND는 값 타입 핸들이고 `IFileOperation::SetOwnerWindow`는 그 핸들을 대화의
/// 소유자로 적기만 한다. 창이 이미 파괴됐으면 셸이 소유자 없이 띄운다
/// (`fs::enumerate`의 같은 래퍼와 같은 근거)
struct HwndSend(isize);
// 안전성: 위 주석 참조 — 핸들 값 자체는 스레드 간 이동해도 무해하다
unsafe impl Send for HwndSend {}

/// `sources`를 `dest` 폴더로 복사한다 — **곧바로 돌아오고** 결과는 `done`으로 온다 (FR-60).
///
/// 이동이 아니라 언제나 복사다(FR-60 — 보조키로 가르지 않는다). 같은 이름이 이미 있으면
/// 셸이 자기 대화로 묻는다.
///
/// `sources`가 비면 워커를 띄우지 않고 `false`를 돌려준다 — 부르는 쪽이 헛되이 결과를
/// 기다리지 않게 한다
pub fn copy_into(
    dest: PathBuf,
    sources: Vec<PathBuf>,
    owner: HWND,
    done: Sender<CopyOutcome>,
    wake: Wake,
) -> bool {
    if sources.is_empty() {
        return false;
    }
    let owner = HwndSend(owner.0 as isize);
    std::thread::spawn(move || {
        let requested = sources.len();
        // 안전성: 이 스레드에서 초기화하고 **성공했을 때만** 같은 스레드에서 해제한다 —
        // 실패한 초기화를 짝지어 해제하면 COM 참조 수가 어긋난다
        // (`fs::thumbnail`·`fs::drives`의 워커와 같은 방식)
        let initialized = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) }.is_ok();
        let outcome = run_copy(&dest, &sources, &owner, requested);
        if initialized {
            // 안전성: 위에서 성공한 초기화와 같은 스레드에서 1회 호출
            unsafe {
                CoUninitialize();
            }
        }
        // 받을 쪽이 사라졌으면(앱 종료) 조용히 끝난다
        if done.send(outcome).is_ok() {
            wake();
        }
    });
    true
}

/// 셸에 복사를 걸고 결과를 읽는다 — COM이 초기화된 워커 스레드에서만 부른다.
///
/// 실패는 사유를 문자열로 담아 돌려준다. **여기서 화면 문구를 만들지 않는다** —
/// 이 층은 `ui`를 모르고(AGENTS 계층 규약), 셸이 준 사유는 그대로 옮길 값이다
fn run_copy(dest: &Path, sources: &[PathBuf], owner: &HwndSend, requested: usize) -> CopyOutcome {
    match perform(dest, sources, owner) {
        Ok(cancelled) => CopyOutcome {
            requested,
            cancelled,
            error: None,
        },
        Err(err) => CopyOutcome {
            requested,
            cancelled: false,
            error: Some(err),
        },
    }
}

/// 실제 COM 호출 — 성공하면 `사용자가 그만뒀는가`를 돌려준다.
///
/// **읽지 못하는 원본은 그것만 건너뛴다** — 여러 개를 끌어다 놓았는데 그 사이 하나가
/// 사라졌다고 나머지를 버릴 이유가 없다. 하나도 걸지 못했으면 실패로 본다
fn perform(dest: &Path, sources: &[PathBuf], owner: &HwndSend) -> Result<bool, String> {
    // 안전성: 아래 호출은 모두 COM이 STA로 초기화된 이 스레드에서만 돌고, 얻은 인터페이스는
    // 이 함수 안에서만 살다 `Drop`으로 해제된다. 경로 문자열은 호출이 끝날 때까지 지역 소유다
    unsafe {
        let op: IFileOperation =
            CoCreateInstance(&FileOperation, None, CLSCTX_ALL).map_err(|err| err.message())?;
        // `FOF_ALLOWUNDO`가 `Ctrl+Z`를 만든다. `FOF_NOCONFIRMMKDIR`는 대상 폴더를 만들 때만
        // 묻지 않는 것이라, 같은 이름 충돌 확인은 그대로 뜬다(FR-60이 셸에 맡긴 그 대화다)
        op.SetOperationFlags(FOF_ALLOWUNDO | FOF_NOCONFIRMMKDIR)
            .map_err(|err| err.message())?;
        // 소유자를 주면 대화가 앱 창 위에 뜬다. 창이 없으면(headless·파괴됨) 주지 않는다
        if owner.0 != 0 {
            let _ = op.SetOwnerWindow(HWND(owner.0 as *mut core::ffi::c_void));
        }
        let folder: IShellItem = shell_item(dest)?;
        let mut queued = 0usize;
        for source in sources {
            let Ok(item) = shell_item(source) else {
                // 그 사이 사라진 원본 — 나머지는 그대로 보낸다
                continue;
            };
            if op
                .CopyItem(&item, &folder, PCWSTR::null(), None)
                .map_err(|err| err.message())
                .is_ok()
            {
                queued += 1;
            }
        }
        if queued == 0 {
            return Err(crate::i18n::copy_no_source().to_owned());
        }
        op.PerformOperations().map_err(|err| err.message())?;
        Ok(op
            .GetAnyOperationsAborted()
            .map(|aborted| aborted.as_bool())
            .unwrap_or(false))
    }
}

/// 경로 하나를 셸 항목으로 바꾼다.
///
/// 안전성: COM이 초기화된 스레드에서만 부른다. `HSTRING`은 호출이 끝날 때까지 지역 소유다
fn shell_item(path: &Path) -> Result<IShellItem, String> {
    let wide = HSTRING::from(path.as_os_str());
    // 안전성: 위 주석 참조 — 유효한 널 종단 문자열 하나를 넘긴다
    unsafe { SHCreateItemFromParsingName(PCWSTR(wide.as_ptr()), None).map_err(|err| err.message()) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc::channel;

    fn 가짜_깨우기() -> Wake {
        Arc::new(|| {})
    }

    #[test]
    fn 원본이_없으면_워커를_띄우지_않는다() {
        // 부르는 쪽이 오지 않을 결과를 기다리지 않게 한다
        let (tx, rx) = channel();
        let started = copy_into(
            PathBuf::from(r"C:\down"),
            Vec::new(),
            HWND(std::ptr::null_mut()),
            tx,
            가짜_깨우기(),
        );
        assert!(!started);
        assert!(rx.try_recv().is_err(), "결과도 오지 않는다");
    }

    #[test]
    fn 결과는_걸었던_항목_수를_그대로_싣는다() {
        // 화면이 규모를 적는 데 쓴다 — 셸이 실제로 몇 개를 복사했는지는 셸만 안다
        let outcome = CopyOutcome {
            requested: 3,
            cancelled: false,
            error: None,
        };
        assert_eq!(outcome.requested, 3);
        assert!(outcome.error.is_none());
    }

    #[test]
    fn 취소는_오류가_아니다() {
        // 사용자가 셸 대화에서 그만둔 것과 셸이 실패한 것은 화면에서 다르게 알린다
        let 취소 = CopyOutcome {
            requested: 1,
            cancelled: true,
            error: None,
        };
        assert!(취소.cancelled && 취소.error.is_none());
    }
}
