//! 중복 실행 방지 (FR-51).
//!
//! 이미 실행 중이면 새 프로세스는 **창을 만들지 않고** 기존 창을 띄운 뒤 끝난다.
//! 트레이에 숨어 있는 앱을 잊고 다시 실행하는 것은 흔한 일이고, 그때 창이 둘이 되면
//! 두 프로세스가 같은 `settings.json`에 서로 덮어써 세션이 깨진다.
//!
//! **기존 창을 찾지 않는다** — 창 제목("MOA")은 다른 앱과 겹칠 수 있고 클래스 이름은
//! winit 내부값이라 안정적이지 않다. 대신 시스템 전역에서 고유한 메시지를 등록해
//! 뿌리고, 우리 창 프로시저가 그것을 받아 스스로 올라온다 (D6).
use windows::Win32::Foundation::{ERROR_ALREADY_EXISTS, HANDLE, LPARAM, WPARAM};
use windows::Win32::System::Threading::CreateMutexW;
use windows::Win32::UI::WindowsAndMessaging::{
    HWND_BROADCAST, PostMessageW, RegisterWindowMessageW,
};
#[cfg(test)]
use windows::core::HSTRING;
use windows::core::{PCWSTR, w};

/// 뮤텍스 이름 — `Local\` 접두로 **로그온 세션 단위**로 가른다.
/// 그러지 않으면 다른 사용자로 로그온한 두 번째 세션에서 앱이 아예 뜨지 않는다
const MUTEX_NAME: PCWSTR = w!(r"Local\MOA-single-instance");
/// 기존 창을 깨우는 메시지 이름 — 시스템이 이 이름에 고유 번호를 배정한다
const WAKE_NAME: PCWSTR = w!("MOA.WakeMainWindow");

/// 이 프로세스가 처음인가.
///
/// **가드를 살려 둬야 한다** — 떨어뜨리면 뮤텍스가 풀려 다음 실행이 자기를 처음으로 본다
#[must_use = "가드를 떨어뜨리면 중복 실행 방지가 풀린다"]
pub enum Instance {
    /// 이 프로세스가 처음이다 — 창을 만들어도 된다. 뮤텍스를 프로세스 수명 동안 쥔다
    First(Guard),
    /// 이미 다른 프로세스가 돌고 있다 — 창을 만들지 말고 끝내야 한다
    AlreadyRunning,
}

impl Instance {
    pub fn is_first(&self) -> bool {
        matches!(self, Instance::First(_))
    }
}

/// 뮤텍스를 쥐고 있는 동안만 "내가 그 하나"임이 보장된다
pub struct Guard {
    _handle: HANDLE,
}

/// 중복 실행을 판정한다. **창을 만들기 전에** 부른다.
///
/// COM 초기화·세션 로드보다 먼저 부르는 이유: 두 번째 프로세스라면 그 준비가 전부 헛일이고,
/// 세션 파일을 읽는 동안 첫 프로세스가 그것을 쓰고 있을 수도 있다
pub fn acquire() -> Instance {
    acquire_named(MUTEX_NAME)
}

/// 이름을 받아 판정한다 — **시험이 프로덕션과 다른 이름을 쓰기 위해** 갈라 두었다.
///
/// 갈라 두지 않으면 시험이 실제 앱과 같은 뮤텍스를 다투게 된다: 개발 중 트레이에
/// MOA를 띄워 둔 채(이 기능이 바로 그것을 권한다) `cargo test`를 돌리면 첫 획득부터
/// `AlreadyRunning`이 나와 시험이 머신 상태에 따라 깨진다
fn acquire_named(name: PCWSTR) -> Instance {
    // 안전성: 이름 있는 뮤텍스를 만든다. 이미 있으면 그 핸들을 받고 `GetLastError`가 알려 준다.
    // 핸들은 `Guard`가 들고 있다가 프로세스가 끝날 때 OS가 닫는다
    unsafe {
        let Ok(handle) = CreateMutexW(None, true, name) else {
            // 뮤텍스를 만들지 못하면 판정할 수 없다 — 막기보다 여는 쪽을 택한다
            // (앱이 아예 안 뜨는 것보다 창이 둘인 편이 낫다)
            return Instance::First(Guard {
                _handle: HANDLE::default(),
            });
        };
        if windows::Win32::Foundation::GetLastError() == ERROR_ALREADY_EXISTS {
            // 핸들은 곧 프로세스와 함께 닫힌다 — 여기서 닫아도 그만이지만,
            // 닫는 순간 뮤텍스 소유가 흔들릴 여지를 남기지 않으려 그대로 둔다
            return Instance::AlreadyRunning;
        }
        Instance::First(Guard { _handle: handle })
    }
}

/// 이미 떠 있는 앱에게 "창을 보여 달라"고 알린다 (두 번째 프로세스가 끝나기 전에 부른다).
///
/// 받는 쪽이 없어도 실패하지 않는다 — 그 사이 첫 프로세스가 끝났을 뿐이다.
///
/// **아무것도 실어 보내지 않는다**(`WPARAM(0)`·`LPARAM(0)`) — 두 번째 프로세스가 받은
/// 명령줄을 첫 프로세스에 넘기는 IPC는 만들지 않는다. 이번 요구는 "창을 띄운다"뿐이고,
/// 값을 실으려면 프로세스 사이로 문자열을 나르는 별도 통로가 필요하다
pub fn wake_existing() {
    // 안전성: 시스템 전역 메시지를 최상위 창들에 뿌린다. 우리 메시지 번호는 이름으로
    // 배정돼 고유하므로 남의 창은 이것을 무시한다
    unsafe {
        let message = wake_message();
        if message != 0 {
            let _ = PostMessageW(Some(HWND_BROADCAST), message, WPARAM(0), LPARAM(0));
        }
    }
}

/// 기존 창을 깨우는 메시지 번호 — 창 프로시저도 이 값으로 견준다.
///
/// 이름이 같으면 시스템이 같은 번호를 준다. 그래서 두 프로세스가 서로를 알아본다
pub fn wake_message() -> u32 {
    use std::sync::OnceLock;
    static ID: OnceLock<u32> = OnceLock::new();
    // 안전성: 문자열 상수로 메시지를 등록한다
    *ID.get_or_init(|| unsafe { RegisterWindowMessageW(WAKE_NAME) })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 시험 전용 이름 — **프로덕션 이름을 쓰지 않는다**(위 `acquire_named` 주석 참조).
    /// 프로세스 번호를 붙여 시험끼리도 서로 다투지 않게 한다
    fn test_mutex_name() -> HSTRING {
        HSTRING::from(format!(
            r"Local\MOA-single-instance-test-{}",
            std::process::id()
        ))
    }

    #[test]
    fn 두_번째_획득은_이미_실행_중으로_본다() {
        let name = test_mutex_name();
        let first = acquire_named(PCWSTR(name.as_ptr()));
        assert!(first.is_first(), "처음인데 이미 실행 중이라고 한다");
        // 같은 이름을 다시 청하면 뮤텍스가 이미 있다 — 두 번째 프로세스와 같은 상황이다
        let second = acquire_named(PCWSTR(name.as_ptr()));
        assert!(!second.is_first(), "두 번째인데 처음이라고 한다");
        drop(first);
        drop(second);
    }

    #[test]
    fn 깨우기_메시지_번호를_얻는다() {
        let id = wake_message();
        assert_ne!(id, 0, "메시지를 등록하지 못했다");
        // 같은 이름은 언제나 같은 번호 — 두 프로세스가 서로를 알아보는 근거다
        assert_eq!(id, wake_message());
    }
}
