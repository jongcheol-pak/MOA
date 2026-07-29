//! 새 폴더·새 파일 만들기 (FR-25).
//!
//! 이름이 겹치면 `(2)`, `(3)`… 번호를 붙인다. **존재 여부를 먼저 확인하지 않고 바로 만들어 본다** —
//! 확인과 생성 사이에 다른 앱이 같은 이름을 만들 수 있고(TOCTOU), 그 틈에 덮어쓰면 남의 파일이
//! 사라진다. `create_dir`·`File::create_new`는 이미 있으면 실패하므로 그 실패를 다음 후보로
//! 넘어가는 신호로 쓴다.
use std::io;
use std::path::{Path, PathBuf};

/// 새 폴더의 기본 이름
const FOLDER_BASE: &str = "새 폴더";
/// 새 파일의 기본 이름과 확장자 — Windows 탐색기의 "새로 만들기 > 텍스트 문서"와 같다 (사용자 확정)
const FILE_BASE: &str = "새 텍스트 문서";
const FILE_EXT: &str = "txt";

/// 번호를 붙여 볼 최대 횟수. 이 한도가 없으면 쓰기 권한이 없는 폴더처럼
/// **매번 같은 이유로 실패하는** 상황에서 무한히 돈다
const MAX_ATTEMPTS: usize = 1000;

/// 표시 중인 폴더에 새 폴더를 만든다. 성공하면 만들어진 경로
pub fn new_folder(dir: &Path) -> io::Result<PathBuf> {
    create_unique(dir, FOLDER_BASE, None, |path| std::fs::create_dir(path))
}

/// 표시 중인 폴더에 빈 텍스트 문서를 만든다. 성공하면 만들어진 경로
pub fn new_text_file(dir: &Path) -> io::Result<PathBuf> {
    create_unique(dir, FILE_BASE, Some(FILE_EXT), |path| {
        // `create_new`는 이미 있으면 실패한다 — 기존 파일을 잘라내는 `create`와 다르다
        std::fs::File::create_new(path).map(|_| ())
    })
}

/// 겹치지 않는 이름을 찾을 때까지 `make`를 반복한다.
///
/// `AlreadyExists`만 다음 후보로 넘어가는 신호로 본다 — 권한 없음·경로 없음 같은 실패는
/// 번호를 바꿔도 해결되지 않으므로 그대로 돌려준다(사유를 상태 줄에 보여야 한다)
fn create_unique(
    dir: &Path,
    base: &str,
    ext: Option<&str>,
    make: impl Fn(&Path) -> io::Result<()>,
) -> io::Result<PathBuf> {
    let dir = to_extended(dir);
    let mut last = None;
    for number in 1..=MAX_ATTEMPTS {
        let path = dir.join(candidate_name(base, ext, number));
        match make(&path) {
            Ok(()) => return Ok(path),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => last = Some(error),
            Err(error) => return Err(error),
        }
    }
    Err(last.unwrap_or_else(|| {
        io::Error::new(
            io::ErrorKind::AlreadyExists,
            "쓸 수 있는 이름을 찾지 못했습니다",
        )
    }))
}

/// `number`번째 후보 이름 — 1이면 번호 없이, 그 뒤로는 ` (n)`을 붙인다.
/// 확장자는 번호 **뒤에** 온다("새 텍스트 문서 (2).txt") — 탐색기와 같은 규칙이다
fn candidate_name(base: &str, ext: Option<&str>, number: usize) -> String {
    let stem = if number == 1 {
        base.to_owned()
    } else {
        format!("{base} ({number})")
    };
    match ext {
        Some(ext) => format!("{stem}.{ext}"),
        None => stem,
    }
}

/// 260자가 넘는 경로에서도 만들 수 있도록 `\\?\` 접두를 붙인다 (NFR-5, plan D13).
///
/// `fs::enumerate`의 같은 규칙과 짝을 이루지만 그쪽은 검색 패턴(`\*`)까지 붙여 용도가 다르다.
/// 접두가 붙은 경로에서는 `/`가 구분자로 인식되지 않으므로 먼저 `\`로 통일한다
fn to_extended(path: &Path) -> PathBuf {
    let text = path.to_string_lossy().replace('/', r"\");
    if text.starts_with(r"\\?\") {
        PathBuf::from(text)
    } else if let Some(rest) = text.strip_prefix(r"\\") {
        // UNC 경로: \\server\share → \\?\UNC\server\share
        PathBuf::from(format!(r"\\?\UNC\{rest}"))
    } else {
        PathBuf::from(format!(r"\\?\{text}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("fe_create_{name}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn 첫_이름에는_번호가_없다() {
        assert_eq!(candidate_name("새 폴더", None, 1), "새 폴더");
        assert_eq!(
            candidate_name("새 텍스트 문서", Some("txt"), 1),
            "새 텍스트 문서.txt"
        );
    }

    #[test]
    fn 번호는_확장자_앞에_붙는다() {
        // "새 텍스트 문서.txt (2)"가 되면 확장자가 사라져 연결 프로그램으로 열 수 없다
        assert_eq!(
            candidate_name("새 텍스트 문서", Some("txt"), 2),
            "새 텍스트 문서 (2).txt"
        );
        assert_eq!(candidate_name("새 폴더", None, 3), "새 폴더 (3)");
    }

    #[test]
    fn 같은_폴더에_거듭_만들면_번호가_늘어난다() {
        let dir = temp_dir("folder_seq");
        let first = new_folder(&dir).unwrap();
        let second = new_folder(&dir).unwrap();
        let third = new_folder(&dir).unwrap();
        assert!(first.ends_with("새 폴더"));
        assert!(second.ends_with("새 폴더 (2)"));
        assert!(third.ends_with("새 폴더 (3)"));
        // 셋 다 실재해야 한다 — 덮어썼다면 개수가 모자란다
        for path in [&first, &second, &third] {
            assert!(path.is_dir(), "{path:?}가 만들어지지 않았다");
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn 새_파일은_빈_텍스트_문서다() {
        let dir = temp_dir("file_new");
        let path = new_text_file(&dir).unwrap();
        assert!(path.ends_with("새 텍스트 문서.txt"));
        assert_eq!(std::fs::metadata(&path).unwrap().len(), 0);
        let second = new_text_file(&dir).unwrap();
        assert!(second.ends_with("새 텍스트 문서 (2).txt"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn 기존_파일을_덮어쓰지_않는다() {
        // 같은 이름이 이미 있으면 번호를 붙여 비켜간다 — 내용이 보존돼야 한다
        let dir = temp_dir("no_overwrite");
        let existing = dir.join("새 텍스트 문서.txt");
        std::fs::write(&existing, b"keep me").unwrap();
        let made = new_text_file(&dir).unwrap();
        assert!(made.ends_with("새 텍스트 문서 (2).txt"));
        assert_eq!(std::fs::read(&existing).unwrap(), b"keep me");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn 이름이_모두_겹치면_상한에서_멈춘다() {
        // 상한이 없으면 이름이 끝없이 겹치는 상황에서 무한히 돈다.
        // 실제 파일을 1000개 만들지 않고, 항상 "이미 있음"을 돌려주는 생성기로 상한만 확인한다
        let tried = std::cell::Cell::new(0usize);
        let result = create_unique(Path::new(r"C:\"), "무엇이든", None, |_| {
            tried.set(tried.get() + 1);
            Err(io::Error::new(io::ErrorKind::AlreadyExists, "이미 있음"))
        });
        assert!(result.is_err(), "상한을 넘겼는데 성공으로 보고됐다");
        assert_eq!(tried.get(), MAX_ATTEMPTS, "상한만큼만 시도해야 한다");
        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::AlreadyExists);
    }

    #[test]
    fn 겹치지_않으면_한_번만_시도한다() {
        // 첫 이름이 비어 있으면 곧장 성공해야 한다 — 상한 루프가 매번 도는 것이 아니다
        let tried = std::cell::Cell::new(0usize);
        let made = create_unique(Path::new(r"C:\"), "무엇이든", None, |_| {
            tried.set(tried.get() + 1);
            Ok(())
        })
        .unwrap();
        assert_eq!(tried.get(), 1);
        assert!(made.ends_with("무엇이든"));
    }

    #[test]
    fn 없는_폴더에서는_사유를_그대로_돌려준다() {
        // 번호를 바꿔도 해결되지 않는 실패는 즉시 돌려줘야 한다(무한 재시도 금지)
        let missing = std::env::temp_dir().join("fe_create_absent_dir_xyz");
        let _ = std::fs::remove_dir_all(&missing);
        let error = new_folder(&missing).unwrap_err();
        assert_ne!(
            error.kind(),
            io::ErrorKind::AlreadyExists,
            "존재하지 않는 폴더인데 이름 충돌로 보고됐다"
        );
    }

    #[test]
    fn 확장_접두를_두_번_붙이지_않는다() {
        let already = PathBuf::from(r"\\?\C:\Users");
        assert_eq!(to_extended(&already), already);
    }

    #[test]
    fn 일반_경로에는_접두를_붙인다() {
        assert_eq!(
            to_extended(Path::new(r"C:\Users")),
            PathBuf::from(r"\\?\C:\Users")
        );
    }

    #[test]
    fn unc_경로는_unc_형식으로_바꾼다() {
        assert_eq!(
            to_extended(Path::new(r"\\server\share")),
            PathBuf::from(r"\\?\UNC\server\share")
        );
    }

    #[test]
    fn 슬래시는_역슬래시로_통일된다() {
        // 접두가 붙은 경로에서는 `/`가 구분자로 인식되지 않는다
        assert_eq!(
            to_extended(Path::new("C:/Users/test")),
            PathBuf::from(r"\\?\C:\Users\test")
        );
    }
}
