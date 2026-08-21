//! NSIS 설치 파일 생성기 — `cargo run --example gen_installer`.
//!
//! `cargo build --release`가 만든 `target/release/moa.exe`를 담아
//! `target/installer/MOA-Setup-<버전>.exe`를 만든다. 스크립트는 `installer/moa.nsi`이고
//! 이 예제는 그것을 **부르기만** 한다 — 설치 규칙은 사람이 읽고 고치는 소스라 생성하지 않는다.
//!
//! **NSIS는 이 저장소가 설치해 주지 않는다** — 없으면 `winget install NSIS.NSIS`를 안내하고
//! 실패로 끝낸다. 설치 파일을 만들라고 부른 명령이 아무것도 만들지 않고 성공으로 끝나는 것이
//! 더 나쁘기 때문이다.
//!
//! 예제 타깃이라 화면 출력과 `main -> Result`를 쓴다(GUI 프로덕션의 `println!` 금지는 콘솔
//! 창이 없는 exe를 겨냥한 것이고, 개발용 CLI에는 오류를 알릴 수단이 필요하다).
use std::path::{Path, PathBuf};
use std::process::Command;

/// 설치 스크립트가 있는 폴더 — **makensis의 작업 디렉터리로 지정한다**.
/// `.nsi`의 경로가 전부 이 폴더 기준이라, makensis가 스크립트 폴더로 옮기든 아니든 같은 자리를 가리킨다
const SCRIPT_DIR: &str = "installer";
const SCRIPT_NAME: &str = "moa.nsi";

fn main() -> Result<(), String> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let exe = root.join("target").join("release").join("moa.exe");
    if !exe.exists() {
        return Err(format!(
            "{}: 없다 — `cargo build --release`를 먼저 돌린다",
            exe.display()
        ));
    }

    let Some(makensis) = find_makensis() else {
        return Err(concat!(
            "makensis를 찾지 못했다 — NSIS가 설치돼 있지 않다.\n",
            "  설치: winget install NSIS.NSIS\n",
            "  설치 뒤 이 명령을 다시 돌린다"
        )
        .to_owned());
    };
    println!("makensis {}", makensis.display());

    // 산출 폴더는 `.nsi`의 `OutFile`이 가리키는 자리다 — 없으면 makensis가 쓰지 못한다
    let out_dir = root.join("target").join("installer");
    std::fs::create_dir_all(&out_dir)
        .map_err(|error| format!("{}: 만들지 못했다 — {error}", out_dir.display()))?;

    let version = env!("CARGO_PKG_VERSION");
    let script_dir = root.join(SCRIPT_DIR);
    run(&makensis, &script_dir, version)?;

    let out = out_dir.join(format!("MOA-Setup-{version}.exe"));
    let written = std::fs::metadata(&out)
        .map_err(|error| format!("{}: 만들어지지 않았다 — {error}", out.display()))?
        .len();
    println!("{} {written}B", out.display());
    Ok(())
}

/// `makensis` 실행 파일을 찾는다 — PATH → `%ProgramFiles%` → `%ProgramFiles(x86)%` 순서.
///
/// PATH를 먼저 보는 이유: 사용자가 다른 자리에 설치했거나 여러 판을 둔 경우 그가 고른 것이
/// PATH에 있다. 기본 설치는 64비트 Windows에서 `Program Files (x86)`에 놓이므로 그 자리도 본다
fn find_makensis() -> Option<PathBuf> {
    // PATH에 있으면 이름만으로 실행된다 — `/VERSION`으로 실제 실행되는지까지 확인한다
    // (여기서는 있고 없고만 가른다. 실행이 실패하는 경우는 아래 `run`이 종료 코드로 알린다)
    if Command::new("makensis").arg("/VERSION").output().is_ok() {
        return Some(PathBuf::from("makensis"));
    }
    for var in ["ProgramFiles", "ProgramFiles(x86)"] {
        let Some(base) = std::env::var_os(var) else {
            continue;
        };
        let candidate = PathBuf::from(base).join("NSIS").join("makensis.exe");
        if candidate.exists() {
            return Some(candidate);
        }
    }
    None
}

/// makensis를 부른다. **인자를 문자열로 조립하지 않는다** — 이 저장소 경로에 공백이 있어
/// (`D:\Personal Project\...`) 한 문자열로 넘기면 첫 공백에서 끊긴다.
///
/// `/INPUTCHARSET UTF8`을 주는 이유: `.nsi`가 BOM 없는 UTF-8인데(레포 규약) makensis는
/// BOM이 없으면 시스템 코드페이지로 읽어 그 안의 한글 문구가 깨진다
fn run(makensis: &Path, script_dir: &Path, version: &str) -> Result<(), String> {
    let status = Command::new(makensis)
        .current_dir(script_dir)
        .arg("/INPUTCHARSET")
        .arg("UTF8")
        .arg(format!("/DVERSION={version}"))
        .arg(SCRIPT_NAME)
        .status()
        .map_err(|error| format!("{}: 실행하지 못했다 — {error}", makensis.display()))?;
    if !status.success() {
        return Err(format!("makensis가 실패했다 — 종료 코드 {status}"));
    }
    Ok(())
}
