//! 로컬 파일·폴더 복사·이름 바꾸기·삭제 (FR-60·FR-61·FR-64) — **Windows 셸에 맡긴다**.
//!
//! 직접 재귀 복사를 만들지 않고 `IFileOperation`을 부르는 이유가 이 모듈의 전부다:
//! 진행률 대화·같은 이름 충돌 대화·되돌리기(`Ctrl+Z`)·권한 승격·긴 경로·심볼릭 링크를
//! 전부 OS가 처리한다. PRD의 「자체 파일 작업 UI는 셸에 위임」 원칙과 같은 방향이며,
//! 그래서 로컬끼리의 복사에는 FR-55의 자체 확인 대화를 띄우지 않는다(셸이 자기 대화로 묻는다).
//!
//! **일은 워커 스레드에서 한다** — `PerformOperations`는 걸어 둔 작업이 끝날 때까지
//! 돌아오지 않아, UI 스레드에서 부르면 대용량 복사·수백 항목 삭제 내내 앱이 굳는다
//! (AGENTS: UI 스레드 블로킹 I/O 금지). 그 골격은 `spawn_shell_op` 한 곳에 있다.
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc::Sender;

use windows::Win32::Foundation::HWND;
use windows::Win32::System::Com::{
    CLSCTX_ALL, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx, CoUninitialize,
};
use windows::Win32::UI::Shell::{
    FILEOPERATION_FLAGS, FOF_ALLOWUNDO, FOF_NOCONFIRMMKDIR, FileOperation, IFileOperation,
    IShellItem, SHCreateItemFromParsingName,
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
/// 안전성: 핸들 값 자체는 스레드 간 옮겨도 무해하고, **다른 스레드가 만든 창을
/// `SetOwnerWindow`에 주는 것은 `IFileOperation`을 워커에서 돌릴 때의 표준 방식**이다 —
/// 진행률 대화는 이 워커 스레드에서 뜨고 소유자만 UI 스레드의 창을 가리킨다. 입력 큐를
/// 붙이지(`AttachThreadInput`) 않고, UI 스레드도 이 워커를 기다리지 않아(결과는 채널로
/// 온다) 서로 막을 자리가 없다. 창이 이미 파괴됐으면 셸이 소유자 없이 띄운다.
///
/// **`fs::enumerate`의 같은 이름 래퍼와는 근거가 다르다** — 그쪽은 `PostMessageW`가
/// 어느 스레드에서나 안전하다는 그 API의 성질이 근거다
struct HwndSend(isize);
// 안전성: 위 주석 참조
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
    let requested = sources.len();
    spawn_shell_op(done, wake, move || {
        // 결과 타입이 `FileOpOutcome`과 달라 조립만 따로 한다 — 워커 껍데기는 같이 쓴다
        match perform(&dest, &sources, &owner, Transfer::Copy) {
            Ok(cancelled) => CopyOutcome {
                requested,
                cancelled,
                error: None,
            },
            Err(error) => CopyOutcome {
                requested,
                cancelled: false,
                error: Some(error),
            },
        }
    })
}

/// `sources`를 `dest` 폴더로 **옮긴다** — 곧바로 돌아오고 결과는 `done`으로 온다 (FR-64).
///
/// 잘라낸 것을 붙여넣는 길이다. `copy_into`와 갈라 두는 이유는 뜻이 다르기 때문이며
/// (저쪽은 드래그 복사 FR-60, 이쪽은 클립보드 FR-64), 실제로 갈리는 것은 셸에 거는 명령
/// 하나(`CopyItem` ↔ `MoveItem`)다 — 그것을 `perform`이 인자로 받는다.
///
/// `sources`가 비면 워커를 띄우지 않고 `false`를 돌려준다(`copy_into`와 같은 계약)
pub fn move_into(
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
    let requested = sources.len();
    spawn_shell_op(done, wake, move || {
        match perform(&dest, &sources, &owner, Transfer::Move) {
            Ok(cancelled) => CopyOutcome {
                requested,
                cancelled,
                error: None,
            },
            Err(error) => CopyOutcome {
                requested,
                cancelled: false,
                error: Some(error),
            },
        }
    })
}

/// 셸에 걸 전송 방식 — 복사인가 이동인가.
///
/// 불리언 대신 두는 이유: 호출부에서 `perform(dest, sources, owner, true)`가 무엇의
/// 참인지 읽히지 않는다
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Transfer {
    Copy,
    Move,
}

/// 실제 COM 호출 — 성공하면 `사용자가 그만뒀는가`를 돌려준다.
///
/// **읽지 못하는 원본은 그것만 건너뛴다** — 여러 개를 끌어다 놓았는데 그 사이 하나가
/// 사라졌다고 나머지를 버릴 이유가 없다. 하나도 걸지 못했으면 실패로 본다.
///
/// **여기서 화면 문구를 만들지 않는다** — 이 층은 `ui`를 모르고(AGENTS 계층 규약), 셸이
/// 준 사유는 그대로 옮길 값이다
fn perform(
    dest: &Path,
    sources: &[PathBuf],
    owner: &HwndSend,
    how: Transfer,
) -> Result<bool, String> {
    // 안전성: 아래 호출은 모두 COM이 STA로 초기화된 이 스레드에서만 돌고, 얻은 인터페이스는
    // 이 함수 안에서만 살다 `Drop`으로 해제된다. 경로 문자열은 호출이 끝날 때까지 지역 소유다
    unsafe {
        // `FOF_ALLOWUNDO`가 `Ctrl+Z`를 만든다. `FOF_NOCONFIRMMKDIR`는 대상 폴더를 만들 때만
        // 묻지 않는 것이라, 같은 이름 충돌 확인은 그대로 뜬다(FR-60이 셸에 맡긴 그 대화다)
        let op = new_operation(FOF_ALLOWUNDO | FOF_NOCONFIRMMKDIR, owner)?;
        let folder: IShellItem = shell_item(dest)?;
        let mut queued = 0usize;
        for source in sources {
            let Ok(item) = shell_item(source) else {
                // 그 사이 사라진 원본 — 나머지는 그대로 보낸다
                continue;
            };
            // 사유를 만들지 않는다 — 이 실패는 그 항목만 건너뛰는 것이라 아무도 읽지 않는다.
            // 하나도 걸지 못한 경우의 사유는 아래에서 따로 만든다
            let queued_one = match how {
                Transfer::Copy => op.CopyItem(&item, &folder, PCWSTR::null(), None),
                Transfer::Move => op.MoveItem(&item, &folder, PCWSTR::null(), None),
            };
            if queued_one.is_ok() {
                queued += 1;
            }
        }
        if queued == 0 {
            return Err(crate::i18n::copy_no_source().to_owned());
        }
        op.PerformOperations().map_err(|err| err.message())?;
        Ok(aborted(&op))
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

/// 이름 바꾸기·삭제 한 번의 결과 (FR-64).
///
/// 이름에 `File`을 붙인 것은 `ui::app::remote::OpOutcome`이 이미 있고 **뜻이 전혀 다르기**
/// 때문이다(그쪽은 원격 명령 뒤에 목록을 다시 읽을지 정하는 값이다).
///
/// **`CopyOutcome`과 필드가 같지만 합치지 않는다** — 바뀌는 이유가 다르다. 저쪽은 드래그
/// 복사(FR-60·FR-61)가 쥐고 있고 이쪽은 메뉴·단축키(FR-64)가 쥔다. 하나로 묶으면 한쪽
/// 요구가 바뀔 때마다 다른 쪽 호출부까지 흔들린다
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileOpOutcome {
    /// 걸었던 항목 수 — 알림 문구가 규모를 적는 데 쓴다
    pub requested: usize,
    /// 사용자가 셸 대화에서 그만뒀는가. **오류가 아니다**
    pub cancelled: bool,
    /// 셸이 준 실패 사유 — 없으면 탈 없이 끝났다
    pub error: Option<String>,
}

/// 파일 이름에 쓸 수 없는 글자 — 셸에 걸기 전에 우리가 먼저 거른다.
///
/// 걸어 보고 실패를 받는 편이 짧지만, 그러면 셸이 자기 말로 알리는 오류 대화가 떠서
/// **어느 글자가 문제인지** 사용자가 알기 어렵다
const INVALID_NAME_CHARS: [char; 9] = ['\\', '/', ':', '*', '?', '"', '<', '>', '|'];

/// 바꿀 이름을 셸에 걸 수 있는가 — 걸 수 없으면 화면에 보일 사유를 돌려준다.
///
/// **예약된 장치 이름(`CON`·`NUL` 등)과 끝에 붙은 점·공백은 여기서 보지 않는다** —
/// 그것들은 글자 자체가 금지된 것이 아니라 자리에 따라 갈려, 판정을 흉내 내면 실제 규칙과
/// 어긋난 채 굳는다. 그런 이름은 셸이 자기 대화로 알린다
pub fn invalid_name_reason(new_name: &str) -> Option<String> {
    new_name
        .contains(INVALID_NAME_CHARS)
        .then(|| crate::i18n::rename_invalid_chars().to_owned())
}

/// 삭제에 줄 셸 플래그 (FR-64).
///
/// **휴지통으로 보내는 것이 기본**이고 `FOF_ALLOWUNDO`가 그것을 만든다 — 그 플래그를 빼면
/// 곧바로 지워진다. 확인 대화를 띄울지는 우리가 정하지 않는다(셸이 자기 정책을 따른다)
fn delete_flags(permanent: bool) -> FILEOPERATION_FLAGS {
    if permanent {
        FILEOPERATION_FLAGS(0)
    } else {
        FOF_ALLOWUNDO
    }
}

/// `path`의 이름을 `new_name`으로 바꾼다 — **곧바로 돌아오고** 결과는 `done`으로 온다 (FR-64).
///
/// 돌려주는 값은 "결과를 기다려야 하는가"다. **빈 이름이거나 지금 이름과 같으면 `false`** —
/// 아무 일도 하지 않고 결과도 보내지 않는다(사용자가 편집을 열었다 그대로 확정한 경우라
/// 알릴 것이 없다). 쓸 수 없는 글자가 있으면 워커를 띄우지 않고 **사유만** 보낸 뒤 `true`다
pub fn rename_item(
    path: PathBuf,
    new_name: String,
    owner: HWND,
    done: Sender<FileOpOutcome>,
    wake: Wake,
) -> bool {
    let same_name = path
        .file_name()
        .is_some_and(|current| current == std::ffi::OsStr::new(&new_name));
    if new_name.is_empty() || same_name {
        return false;
    }
    if let Some(reason) = invalid_name_reason(&new_name) {
        let outcome = FileOpOutcome {
            requested: 1,
            cancelled: false,
            error: Some(reason),
        };
        if done.send(outcome).is_ok() {
            wake();
        }
        return true;
    }
    let owner = HwndSend(owner.0 as isize);
    spawn_shell_op(done, wake, move || {
        outcome_of(1, perform_rename(&path, &new_name, &owner))
    })
}

/// `paths`를 지운다 — **곧바로 돌아오고** 결과는 `done`으로 온다 (FR-64).
///
/// `permanent`가 거짓이면 휴지통으로 보내고, 참이면 곧바로 지운다. 확인 대화는 셸이 띄운다.
/// `paths`가 비면 워커를 띄우지 않고 `false`를 돌려준다 — 부르는 쪽이 헛되이 결과를
/// 기다리지 않게 한다(`copy_into`와 같은 계약)
pub fn delete_items(
    paths: Vec<PathBuf>,
    permanent: bool,
    owner: HWND,
    done: Sender<FileOpOutcome>,
    wake: Wake,
) -> bool {
    if paths.is_empty() {
        return false;
    }
    let owner = HwndSend(owner.0 as isize);
    let requested = paths.len();
    spawn_shell_op(done, wake, move || {
        outcome_of(requested, perform_delete(&paths, permanent, &owner))
    })
}

/// 셸 작업 하나를 COM이 잡힌 워커에서 돌리고 결과를 채널로 보낸다.
///
/// 세 진입점(`copy_into`·`rename_item`·`delete_items`)이 **똑같은 껍데기**를 쓰기 때문에
/// 한 곳으로 모았다 — COM 초기화, **성공했을 때만** 짝지어 해제(실패한 초기화를 해제하면
/// 참조 수가 어긋난다), 결과 송신과 다시 그리기 요청이 그것이다. 안에서 무엇을 하는지는
/// `work`가 정하며 이 함수는 모른다.
///
/// 돌려주는 값은 언제나 `true`(=결과를 기다려야 한다)다 — 워커를 띄우지 않는 조건은
/// 부르는 쪽이 먼저 걸러 낸다
fn spawn_shell_op<T, F>(done: Sender<T>, wake: Wake, work: F) -> bool
where
    T: Send + 'static,
    F: FnOnce() -> T + Send + 'static,
{
    std::thread::spawn(move || {
        // 안전성: 이 스레드에서 초기화하고 성공했을 때만 같은 스레드에서 1회 해제한다
        // (`fs::thumbnail`·`fs::drives`의 워커와 같은 방식)
        let initialized = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) }.is_ok();
        let outcome = work();
        if initialized {
            // 안전성: 위에서 성공한 초기화와 짝지은 1회 호출
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

/// 셸 호출 결과를 화면이 읽는 값으로 바꾼다 — 성공은 `그만뒀는가`, 실패는 사유다
fn outcome_of(requested: usize, result: Result<bool, String>) -> FileOpOutcome {
    match result {
        Ok(cancelled) => FileOpOutcome {
            requested,
            cancelled,
            error: None,
        },
        Err(error) => FileOpOutcome {
            requested,
            cancelled: false,
            error: Some(error),
        },
    }
}

/// 이름 바꾸기 COM 호출 — 성공하면 `사용자가 그만뒀는가`를 돌려준다.
///
/// 안전성: COM이 STA로 초기화된 워커 스레드에서만 부른다. 얻은 인터페이스는 이 함수 안에서만
/// 살다 `Drop`으로 해제되고, 이름 문자열은 호출이 끝날 때까지 지역 소유다
fn perform_rename(path: &Path, new_name: &str, owner: &HwndSend) -> Result<bool, String> {
    let wide = HSTRING::from(new_name);
    // 안전성: 위 주석 참조
    unsafe {
        let op = new_operation(FOF_ALLOWUNDO, owner)?;
        let item = shell_item(path)?;
        op.RenameItem(&item, PCWSTR(wide.as_ptr()), None)
            .map_err(|err| err.message())?;
        op.PerformOperations().map_err(|err| err.message())?;
        Ok(aborted(&op))
    }
}

/// 삭제 COM 호출 — 성공하면 `사용자가 그만뒀는가`를 돌려준다.
///
/// **그 사이 사라진 항목은 그것만 건너뛴다** — 여러 개를 골랐는데 하나가 없어졌다고
/// 나머지를 버릴 이유가 없다(`perform`의 같은 규칙). 하나도 걸지 못했으면 실패로 본다
fn perform_delete(paths: &[PathBuf], permanent: bool, owner: &HwndSend) -> Result<bool, String> {
    // 안전성: 위 `perform_rename`과 같다
    unsafe {
        let op = new_operation(delete_flags(permanent), owner)?;
        let mut queued = 0usize;
        for path in paths {
            let Ok(item) = shell_item(path) else {
                // 그 사이 사라진 항목 — 나머지는 그대로 지운다
                continue;
            };
            if op.DeleteItem(&item, None).is_ok() {
                queued += 1;
            }
        }
        if queued == 0 {
            return Err(crate::i18n::delete_no_source().to_owned());
        }
        op.PerformOperations().map_err(|err| err.message())?;
        Ok(aborted(&op))
    }
}

/// 플래그와 소유자 창을 세운 `IFileOperation` 하나.
///
/// 안전성: COM이 초기화된 스레드에서만 부른다. 창이 없으면(headless·파괴됨) 소유자를 주지
/// 않아 셸이 자기 대화를 소유자 없이 띄운다
unsafe fn new_operation(
    flags: FILEOPERATION_FLAGS,
    owner: &HwndSend,
) -> Result<IFileOperation, String> {
    // 안전성: 위 주석 참조
    unsafe {
        let op: IFileOperation =
            CoCreateInstance(&FileOperation, None, CLSCTX_ALL).map_err(|err| err.message())?;
        op.SetOperationFlags(flags).map_err(|err| err.message())?;
        if owner.0 != 0 {
            let _ = op.SetOwnerWindow(HWND(owner.0 as *mut core::ffi::c_void));
        }
        Ok(op)
    }
}

/// 사용자가 셸 대화에서 그만뒀는가 — 물어보지 못하면 "그만두지 않았다"로 본다.
///
/// 안전성: `PerformOperations`가 끝난 뒤의 유효한 인터페이스에만 부른다
unsafe fn aborted(op: &IFileOperation) -> bool {
    // 안전성: 위 주석 참조
    unsafe {
        op.GetAnyOperationsAborted()
            .map(|aborted| aborted.as_bool())
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc::channel;

    fn 가짜_깨우기() -> Wake {
        Arc::new(|| {})
    }

    #[test]
    fn 지울_것이_없으면_워커를_띄우지_않는다() {
        // `copy_into`와 같은 계약 — 부르는 쪽이 오지 않을 결과를 기다리지 않게 한다
        let (tx, rx) = channel();
        let started = delete_items(
            Vec::new(),
            false,
            HWND(std::ptr::null_mut()),
            tx,
            가짜_깨우기(),
        );
        assert!(!started);
        assert!(rx.try_recv().is_err(), "결과도 오지 않는다");
    }

    #[test]
    fn 휴지통과_영구_삭제는_되돌리기_플래그로_갈린다() {
        // `FOF_ALLOWUNDO`가 곧 휴지통이다 — 이 판정이 뒤집히면 지운 파일을 되찾을 수 없다
        assert_eq!(delete_flags(false), FOF_ALLOWUNDO, "기본은 휴지통");
        assert_eq!(
            delete_flags(true),
            FILEOPERATION_FLAGS(0),
            "Shift+Delete는 되돌리기 없이 곧바로 지운다"
        );
    }

    #[test]
    fn 쓸_수_없는_글자가_있는_이름은_셸에_걸지_않는다() {
        let _guard =
            crate::i18n::LanguageGuard::lock(crate::app::settings::LanguageSetting::Korean);
        for 이름 in [
            r"a\b", "a/b", "a:b", "a*b", "a?b", "a\"b", "a<b", "a>b", "a|b",
        ] {
            assert!(
                invalid_name_reason(이름).is_some(),
                "{이름} 은 거부돼야 한다"
            );
        }
        assert_eq!(
            invalid_name_reason("보고서 (최종).txt"),
            None,
            "괄호·공백·점은 쓸 수 있다"
        );
    }

    #[test]
    fn 예약된_장치_이름은_이_계층이_판정하지_않는다() {
        // 자리에 따라 갈리는 규칙이라 흉내 내면 실제와 어긋난 채 굳는다 — 셸이 알린다
        assert_eq!(invalid_name_reason("CON"), None);
        assert_eq!(invalid_name_reason("보고서."), None);
    }

    #[test]
    fn 빈_이름과_같은_이름은_아무_일도_하지_않는다() {
        // 편집을 열었다 그대로 확정한 경우다 — 알릴 것이 없으므로 결과도 보내지 않는다
        for 새_이름 in ["", "app.js"] {
            let (tx, rx) = channel();
            let started = rename_item(
                PathBuf::from(r"C:\work\app.js"),
                새_이름.to_owned(),
                HWND(std::ptr::null_mut()),
                tx,
                가짜_깨우기(),
            );
            assert!(!started, "{새_이름:?} 는 워커를 띄우지 않는다");
            assert!(rx.try_recv().is_err(), "결과도 오지 않는다");
        }
    }

    #[test]
    fn 대상이_없으면_워커를_띄우지_않는다() {
        // 고른 것 없이 `Delete`·`Ctrl+V`를 눌렀을 때다 (FR-12) — 워커도, 결과도 없어야
        // 부르는 쪽이 헛되이 기다리지 않는다
        let (tx, rx) = channel();
        assert!(!delete_items(
            Vec::new(),
            false,
            HWND(std::ptr::null_mut()),
            tx,
            가짜_깨우기()
        ));
        assert!(rx.try_recv().is_err());

        for 옮기기 in [false, true] {
            let (tx, rx) = channel();
            let 시작 = if 옮기기 {
                move_into(
                    PathBuf::from(r"C:\일"),
                    Vec::new(),
                    HWND(std::ptr::null_mut()),
                    tx,
                    가짜_깨우기(),
                )
            } else {
                copy_into(
                    PathBuf::from(r"C:\일"),
                    Vec::new(),
                    HWND(std::ptr::null_mut()),
                    tx,
                    가짜_깨우기(),
                )
            };
            assert!(!시작, "옮기기={옮기기} 에서 워커가 떴다");
            assert!(rx.try_recv().is_err());
        }
    }

    #[test]
    fn 거부된_이름은_워커_없이_사유만_알린다() {
        let _guard =
            crate::i18n::LanguageGuard::lock(crate::app::settings::LanguageSetting::Korean);
        let (tx, rx) = channel();
        let started = rename_item(
            PathBuf::from(r"C:\work\app.js"),
            "a/b.js".to_owned(),
            HWND(std::ptr::null_mut()),
            tx,
            가짜_깨우기(),
        );
        assert!(started, "사유가 갈 것이므로 기다려야 한다");
        let outcome = rx.try_recv().expect("사유가 곧바로 온다");
        assert_eq!(outcome.requested, 1);
        assert!(!outcome.cancelled);
        assert_eq!(
            outcome.error.as_deref(),
            Some(r#"이름에 \ / : * ? " < > | 는 쓸 수 없습니다"#)
        );
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
    fn 대상이_원본과_같은_폴더여도_그대로_셸에_넘긴다() {
        // **이 계층은 걸러내지 않는다** — 무엇을 걸러낼지는 부르는 쪽이 정한다.
        // 앱 안 드롭은 `ui::list_common::local_copy_target`이 같은 폴더를 먼저 잘라내므로
        // (2026-08-21 — FR-60) 여기까지 오지 않고, 여기로 오는 같은 폴더 복사는
        // OS에서 끌어온 경로(FR-61 ⓐ)뿐이다. 그 경우 사본을 만들지 거부할지는 셸이 정한다
        let (tx, _rx) = channel();
        let started = copy_into(
            PathBuf::from(r"C:\work"),
            vec![PathBuf::from(r"C:\work\app.js")],
            HWND(std::ptr::null_mut()),
            tx,
            가짜_깨우기(),
        );
        assert!(started, "같은 폴더라는 이유로 걸러내지 않는다");
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
