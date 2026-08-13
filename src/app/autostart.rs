//! 윈도우 시작 시 자동 실행 (FR-49).
//!
//! `HKCU`의 `Run` 키에 이 앱을 등록·해제한다. **`HKLM`이 아니라 `HKCU`를 쓰는 이유**는
//! 관리자 권한이 필요 없기 때문이다 — 이 앱은 개인용이고, 권한 상승을 요구하면
//! 설정 토글 하나에 UAC 창이 뜬다.
//!
//! **정본은 레지스트리다.** 설정 파일에도 값을 담지만(`AppSettings.auto_start`) 그것은 사본이며,
//! 화면에 보일 때는 레지스트리를 다시 읽는다 — 다른 도구(작업 관리자의 시작 프로그램 탭 등)가
//! 그 값을 지웠을 수 있고, 그때 화면만 `켜짐`으로 남으면 사용자는 켜져 있다고 믿게 된다.
use std::io;
use windows::Win32::Foundation::{ERROR_FILE_NOT_FOUND, ERROR_SUCCESS};
use windows::Win32::System::Registry::{
    HKEY, HKEY_CURRENT_USER, KEY_READ, KEY_WRITE, REG_OPTION_NON_VOLATILE, REG_SZ, RegCloseKey,
    RegCreateKeyExW, RegDeleteValueW, RegQueryValueExW, RegSetValueExW,
};
use windows::core::HSTRING;

/// 시작 프로그램이 등록되는 자리 (사용자 단위)
const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
/// 값 이름 — 앱 이름과 같게 둔다. 작업 관리자의 시작 프로그램 목록에 이 이름으로 보인다
const VALUE_NAME: &str = "MOA";
/// 자동 실행으로 시작했음을 앱이 알아보는 표식 (D9) — 그때는 트레이로만 올라온다 (FR-49)
pub const TRAY_ARG: &str = "--tray";

/// 지금 자동 실행이 켜져 있는가 — **레지스트리를 직접 읽는다**(설정 파일이 아니라).
///
/// 값이 있기만 하면 켜진 것으로 본다. 경로가 옛것이어도 마찬가지다 —
/// 그 경우 `set_enabled(true)`가 현재 경로로 덮어쓴다
pub fn is_enabled() -> bool {
    // 안전성: 키 핸들은 이 함수 안에서 열고 닫는다. 읽기만 하므로 값을 바꾸지 않는다
    unsafe {
        let Some(key) = open_run_key(KEY_READ.0, false) else {
            return false;
        };
        let mut size = 0u32;
        let status = RegQueryValueExW(
            key,
            &HSTRING::from(VALUE_NAME),
            None,
            None,
            None,
            Some(&mut size),
        );
        let _ = RegCloseKey(key);
        status == ERROR_SUCCESS
    }
}

/// 자동 실행을 켜거나 끈다.
///
/// 켤 때는 **현재 exe 경로로 덮어쓴다** — 앱을 다른 폴더로 옮긴 뒤에도 그대로 동작해야 한다.
/// 끌 때 값이 이미 없으면 성공으로 본다(지우려던 상태가 이미 이뤄져 있다).
///
/// 실패를 삼키지 않고 돌려주는 이유: 정책으로 쓰기가 막힌 환경에서 조용히 무시하면
/// 화면의 토글과 실제 상태가 어긋난다
pub fn set_enabled(on: bool) -> io::Result<()> {
    let Some(command) = command_line() else {
        return Err(io::Error::other("실행 파일 경로를 알 수 없다"));
    };
    // 안전성: 키 핸들은 이 함수 안에서 열고 닫는다
    unsafe {
        let Some(key) = open_run_key(KEY_WRITE.0 | KEY_READ.0, true) else {
            return Err(io::Error::other("시작 프로그램 설정을 열지 못했다"));
        };
        let name = HSTRING::from(VALUE_NAME);
        let status = if on {
            let wide: Vec<u16> = command.encode_utf16().chain(std::iter::once(0)).collect();
            // 널 종단을 포함한 **바이트** 길이를 넘긴다 — 문자 수가 아니다
            let bytes = std::slice::from_raw_parts(wide.as_ptr().cast::<u8>(), wide.len() * 2);
            RegSetValueExW(key, &name, None, REG_SZ, Some(bytes))
        } else {
            let status = RegDeleteValueW(key, &name);
            // 이미 없으면 지우려던 상태가 이뤄진 것이다
            if status == ERROR_FILE_NOT_FOUND {
                ERROR_SUCCESS
            } else {
                status
            }
        };
        let _ = RegCloseKey(key);
        if status == ERROR_SUCCESS {
            Ok(())
        } else {
            Err(io::Error::from_raw_os_error(status.0 as i32))
        }
    }
}

/// 레지스트리에 적을 명령줄 — `"<exe 경로>" --tray`.
///
/// 경로를 따옴표로 감싸는 것은 **공백이 든 경로**(`C:\Program Files\...`) 때문이다.
/// 감싸지 않으면 Windows가 첫 공백에서 끊어 엉뚱한 파일을 찾는다
fn command_line() -> Option<String> {
    let exe = std::env::current_exe().ok()?;
    Some(format!("\"{}\" {TRAY_ARG}", exe.display()))
}

/// `Run` 키를 연다. `create`면 없을 때 만든다(처음 등록하는 경우).
///
/// # 안전성
/// 돌려준 핸들은 호출자가 `RegCloseKey`로 닫아야 한다
unsafe fn open_run_key(access: u32, create: bool) -> Option<HKEY> {
    unsafe {
        let mut key = HKEY::default();
        let status = if create {
            RegCreateKeyExW(
                HKEY_CURRENT_USER,
                &HSTRING::from(RUN_KEY),
                None,
                None,
                REG_OPTION_NON_VOLATILE,
                windows::Win32::System::Registry::REG_SAM_FLAGS(access),
                None,
                &mut key,
                None,
            )
        } else {
            windows::Win32::System::Registry::RegOpenKeyExW(
                HKEY_CURRENT_USER,
                &HSTRING::from(RUN_KEY),
                None,
                windows::Win32::System::Registry::REG_SAM_FLAGS(access),
                &mut key,
            )
        };
        (status == ERROR_SUCCESS).then_some(key)
    }
}

/// 이 실행이 **자동 실행으로 시작된 것인가** (D9).
///
/// 레지스트리에 적은 `--tray` 인자로 판정한다 — 사용자가 직접 실행하면 그 인자가 없다.
/// 부팅 후 경과 시간 같은 추정을 쓰지 않는 이유는 그것이 틀릴 수 있어서다
pub fn started_by_autostart() -> bool {
    std::env::args().any(|arg| arg == TRAY_ARG)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 시험이 사용자의 실제 시작 프로그램 설정을 건드리므로 **반드시 원래대로 되돌린다**.
    ///
    /// 되돌리기를 `Drop`에 두는 이유: 단언이 실패해 패닉이 나도 복구가 돈다
    struct Restore {
        was_enabled: bool,
    }

    impl Drop for Restore {
        fn drop(&mut self) {
            let _ = set_enabled(self.was_enabled);
        }
    }

    #[test]
    fn 켜고_끄면_레지스트리에_그대로_반영된다() {
        let _restore = Restore {
            was_enabled: is_enabled(),
        };

        set_enabled(true).expect("켜지 못했다");
        assert!(is_enabled(), "켰는데 꺼진 것으로 읽힌다");

        set_enabled(false).expect("끄지 못했다");
        assert!(!is_enabled(), "껐는데 켜진 것으로 읽힌다");

        // 이미 꺼진 것을 또 끄는 것은 성공이다 — 지우려던 상태가 이미 이뤄져 있다
        set_enabled(false).expect("이미 꺼진 것을 끄는데 실패했다");
    }

    #[test]
    fn 명령줄은_경로를_따옴표로_감싼다() {
        let command = command_line().expect("실행 파일 경로를 얻지 못했다");
        assert!(command.starts_with('"'), "경로가 따옴표로 시작하지 않는다");
        assert!(
            command.ends_with(&format!("\" {TRAY_ARG}")),
            "따옴표를 닫고 인자를 붙이지 않았다: {command}"
        );
        // 공백이 든 경로에서도 실행 파일이 한 덩어리로 읽혀야 한다
        let quoted_end = command.rfind('"').expect("닫는 따옴표가 없다");
        assert!(quoted_end > 0, "따옴표가 하나뿐이다");
    }

    #[test]
    fn 인자가_없으면_자동_실행이_아니다() {
        // 시험은 `--tray` 없이 돌아간다
        assert!(!started_by_autostart());
    }
}
