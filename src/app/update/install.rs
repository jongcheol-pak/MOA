//! 내려받기·무결성 대조·설치 실행과 뒷정리 (FR-62).
//!
//! **설치본에서만 도는 기능이다** — 개발 실행(`target\debug`)에서는 확인조차 하지 않는다.
//! 설치 파일은 실행 파일 옆 `update\` 폴더에 받고, 설치가 끝나 새 판으로 다시 뜬 앱이
//! 그 폴더를 지운다(설치가 끝나는 시점에는 이 앱이 이미 죽어 있어 스스로 치울 수 없다).
//!
//! **설치 방식을 갈래로 나누지 않는다** — 우리가 내는 것은 NSIS 설치 파일 하나다.
use super::release::{ReleaseInfo, UpdateError};
use super::{http, sha256};
use std::path::{Path, PathBuf};

/// 설치 파일을 받아 두는 폴더 이름 (사용자 요청 문면 그대로)
const UPDATE_DIR: &str = "update";

/// 설치본이면 실행 파일 옆에 있는 것 — 이것으로 설치본과 개발 실행을 가른다
/// (`installer/moa.nsi`의 `WriteUninstaller`)
const UNINSTALLER: &str = "uninstall.exe";

/// 설치 프로그램에 업데이트 모드를 알리는 인자 (`installer/moa.nsi`가 읽는다)
const UPDATE_FLAG: &str = "/UPDATE";

/// 이 실행 파일이 설치본인가 — 옆에 제거 프로그램이 있으면 설치본이다.
///
/// 개발 빌드에서는 **`MOA_UPDATE_DEV`로 이 판정을 건너뛸 수 있다**(값 `1` 또는 `fake`) —
/// 그러지 않으면 배지·다운로드 화면을 릴리즈가 나가기 전까지 한 번도 볼 수 없다.
/// 이 갈래는 `debug_assertions`에서만 컴파일되므로 배포판에는 존재하지 않는다
pub fn is_installed_build() -> bool {
    #[cfg(debug_assertions)]
    if dev_override().is_some() {
        return true;
    }
    std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(is_installed_at))
        .unwrap_or(false)
}

/// 판정 알맹이만 뗀 것 — 폴더를 인자로 받아 시험할 수 있다.
///
/// `is_installed_build`가 `current_exe`를 넘기는데, 시험은 `target\debug\deps`에서 돌아
/// 그 경로가 언제나 「설치본 아님」이 되므로 이 자리가 없으면 두 갈래를 다 볼 수 없다
fn is_installed_at(dir: &Path) -> bool {
    dir.join(UNINSTALLER).is_file()
}

/// 개발용 우회 값 — `1`이면 설치본인 척만 하고, `fake`면 확인 결과까지 가짜로 준다
#[cfg(debug_assertions)]
pub fn dev_override() -> Option<String> {
    match std::env::var("MOA_UPDATE_DEV") {
        Ok(value) if value == "1" || value == "fake" => Some(value),
        _ => None,
    }
}

/// 설치 파일을 받아 둘 폴더 — 실행 파일 옆의 `update\`.
///
/// 실행 파일 자리를 알 수 없는 비정상 환경이면 `None`이고, 그때는 업데이트를 하지 않는다
pub fn update_dir() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    Some(exe.parent()?.join(UPDATE_DIR))
}

/// 받아 둔 것을 폴더째 치운다 — 앱이 뜰 때와 새로 받기 전에 부른다.
///
/// 폴더가 없어도 조용히 넘어간다. **제거할 때 설치 폴더가 남지 않게 하는 것도 이 청소의
/// 몫이다** — `moa.nsi`의 `RMDir "$INSTDIR"`은 재귀가 아니라 빈 폴더만 지운다
pub fn clear_update_dir() {
    if let Some(dir) = update_dir() {
        clear_update_dir_at(&dir);
    }
}

/// 치우는 알맹이만 뗀 것 — 폴더를 인자로 받아 시험할 수 있다.
///
/// `is_installed_at`과 같은 이유로 가른다: 공개 함수는 `current_exe()`에 묶여 있어
/// **시험이 실제로 지우는 것을 볼 수 없고**, 그러면 파일을 지우는 이 경로에 자동 검증이
/// 하나도 없게 된다
fn clear_update_dir_at(dir: &Path) {
    let _ = std::fs::remove_dir_all(dir);
}

/// 릴리즈가 준 자산 이름을 파일 이름으로 써도 되는가 — **경로가 섞여 있으면 안 된다**.
///
/// 이름은 서버에서 온 값이라 `..\`나 절대 경로가 들어 있으면 `update\` 바깥에 쓰거나
/// 지우게 된다(`PathBuf::join`은 `..`를 풀어 주지 않는다). 마디가 **평범한 이름 하나**일
/// 때만 받는다 — 릴리즈를 낼 권한이 있어야 성립하는 공격이지만, 서버가 준 값을 그대로
/// 경로에 붙이지 않는 것이 이쪽의 몫이다
fn is_safe_file_name(name: &str) -> bool {
    let mut components = Path::new(name).components();
    matches!(components.next(), Some(std::path::Component::Normal(_)))
        && components.next().is_none()
}

/// 받은 파일이 기대 체크섬과 같은가 — **어긋나면 그 파일을 지운다**.
///
/// 내려받기와 갈라 둔 이유는 이 대조 규칙을 네트워크 없이 시험하기 위해서다
pub fn verify_downloaded(path: &Path, expected: &str) -> Result<(), UpdateError> {
    let actual = sha256::file_sha256(path).ok_or(UpdateError::Download)?;
    if sha256::matches(expected, &actual) {
        return Ok(());
    }
    // 손상됐거나 바꿔치기된 파일을 남겨 두면 다음 실행이 그것을 온전한 것으로 오인한다
    let _ = std::fs::remove_file(path);
    Err(UpdateError::ChecksumMismatch)
}

/// 설치 파일을 받아 대조까지 마치고 그 경로를 돌려준다.
///
/// 받기 전에 `update\`를 비운다 — 지난번에 받다 만 것이 남아 있을 수 있다
pub fn download_and_verify(info: &ReleaseInfo) -> Result<PathBuf, UpdateError> {
    if !is_safe_file_name(&info.asset_name) {
        return Err(UpdateError::BadResponse);
    }
    clear_update_dir();
    let dir = update_dir().ok_or(UpdateError::Download)?;
    let dest = dir.join(&info.asset_name);
    http::download_to_file(&info.asset_url, &dest)?;
    verify_downloaded(&dest, &info.sha256)?;
    Ok(dest)
}

/// 설치 프로그램을 업데이트 모드로 띄운다. 띄우지 못하면 `false`.
///
/// **띄우기에 성공한 것을 확인한 뒤에야 앱을 닫는다**(부르는 쪽의 책임) — 닫고 나서
/// 실패하면 사용자는 앱도 업데이트도 잃는다
pub fn launch_installer(installer: &Path) -> bool {
    std::process::Command::new(installer)
        .arg(UPDATE_FLAG)
        .spawn()
        .is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 시험용 임시 폴더 — 이름에 프로세스 번호를 넣어 병렬 실행이 서로를 밟지 않게 한다
    fn temp_dir(label: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("moa_update_{label}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("임시 폴더 만들기");
        dir
    }

    #[test]
    fn 받는_자리는_실행_파일_옆의_update_폴더다() {
        let dir = update_dir().expect("실행 파일 자리를 알아야 한다");
        let exe = std::env::current_exe().expect("실행 파일 경로");
        assert_eq!(dir.parent(), exe.parent());
        assert_eq!(
            dir.file_name().and_then(|name| name.to_str()),
            Some("update")
        );
    }

    #[test]
    fn 제거_프로그램_유무로_설치본을_가린다() {
        let dir = temp_dir("설치본판정");
        assert!(
            !is_installed_at(&dir),
            "제거 프로그램이 없으면 개발 실행이다"
        );

        std::fs::write(dir.join("uninstall.exe"), b"stub").expect("제거 프로그램 흉내");
        assert!(is_installed_at(&dir), "있으면 설치본이다");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn 파일이_든_폴더를_통째로_치운다() {
        let dir = temp_dir("청소");
        std::fs::write(dir.join("받다만.exe"), b"x").expect("파일 만들기");
        std::fs::create_dir_all(dir.join("하위")).expect("하위 폴더");
        std::fs::write(dir.join("하위").join("것.bin"), b"y").expect("하위 파일");

        clear_update_dir_at(&dir);

        assert!(!dir.exists(), "폴더째 사라져야 한다");
    }

    #[test]
    fn 폴더가_없어도_치우기는_조용히_끝난다() {
        // 앱이 뜰 때마다 부르므로 「없음」이 정상 상태다
        let dir = temp_dir("청소없음");
        std::fs::remove_dir_all(&dir).expect("먼저 비운다");
        assert!(!dir.exists());

        clear_update_dir_at(&dir); // 없는 자리를 치워도 패닉하지 않는다

        assert!(!dir.exists());
    }

    #[test]
    fn 경로가_섞인_자산_이름은_받지_않는다() {
        // 이름은 서버에서 온 값이다 — `update\` 밖에 쓰거나 지우게 두지 않는다
        assert!(is_safe_file_name("MOA-Setup-0.2.0.exe"));
        assert!(!is_safe_file_name("..\\..\\Windows\\System32\\evil.exe"));
        assert!(!is_safe_file_name("../../evil.exe"));
        assert!(!is_safe_file_name("sub\\dir\\setup.exe"));
        assert!(!is_safe_file_name("C:\\Windows\\evil.exe"));
        assert!(!is_safe_file_name(".."));
        assert!(!is_safe_file_name(""));
    }

    #[test]
    fn 경로가_섞인_자산은_받으러_가지도_않는다() {
        let info = ReleaseInfo {
            version: "9.9.9".to_owned(),
            asset_name: "..\\evil.exe".to_owned(),
            asset_url: "https://example.invalid/x".to_owned(),
            sha256: "0".repeat(64),
        };
        // 네트워크에 닿기 전에 걸러야 한다 — 이 시험이 도는 동안 어떤 연결도 열리지 않는다
        assert_eq!(download_and_verify(&info), Err(UpdateError::BadResponse));
    }

    #[test]
    fn 체크섬이_맞으면_파일을_남긴다() {
        let dir = temp_dir("대조성공");
        let file = dir.join("setup.exe");
        std::fs::write(&file, b"abc").expect("파일 만들기");
        let expected = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";

        assert_eq!(verify_downloaded(&file, expected), Ok(()));
        assert!(file.exists(), "통과한 파일은 그대로 있어야 한다");
        // 릴리즈 노트에 대문자로 적혀 있어도 같은 값으로 본다
        assert_eq!(
            verify_downloaded(&file, &expected.to_ascii_uppercase()),
            Ok(())
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn 체크섬이_어긋나면_받은_파일이_남지_않는다() {
        let dir = temp_dir("대조실패");
        let file = dir.join("setup.exe");
        std::fs::write(&file, b"abc").expect("파일 만들기");
        let wrong = "0000000000000000000000000000000000000000000000000000000000000000";

        assert_eq!(
            verify_downloaded(&file, wrong),
            Err(UpdateError::ChecksumMismatch)
        );
        assert!(
            !file.exists(),
            "손상·변조된 파일을 남기면 다음 실행이 그것을 온전한 것으로 오인한다"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn 없는_파일은_대조에_실패한다() {
        let dir = temp_dir("대조없음");
        let file = dir.join("없다.exe");
        assert_eq!(
            verify_downloaded(&file, "0".repeat(64).as_str()),
            Err(UpdateError::Download)
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
