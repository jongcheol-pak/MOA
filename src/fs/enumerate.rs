//! 디렉터리 열거 — 워커 스레드에서 수행, UI 스레드는 절대 블로킹하지 않는다 (plan D5)
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;
use windows::Win32::Foundation::{
    ERROR_ACCESS_DENIED, ERROR_FILE_NOT_FOUND, ERROR_NO_MORE_FILES, ERROR_PATH_NOT_FOUND, HWND,
    LPARAM, WPARAM,
};
use windows::Win32::Storage::FileSystem::{
    FILE_ATTRIBUTE_DIRECTORY, FIND_FIRST_EX_LARGE_FETCH, FindClose, FindExInfoBasic,
    FindExSearchNameMatch, FindFirstFileExW, FindNextFileW, WIN32_FIND_DATAW,
};
use windows::Win32::UI::WindowsAndMessaging::PostMessageW;
use windows::core::HSTRING;

/// 열거 완료를 패널 창에 알리는 메시지 (lparam·wparam 미사용 — 데이터는 채널로)
pub const WM_APP_ENUM_DONE: u32 = windows::Win32::UI::WindowsAndMessaging::WM_APP + 1;

/// 목록 항목 — 이름은 UTF-16(널 종단) 원본 유지 (정렬 API·표시 공용, 변환 손실 없음)
#[derive(Clone, Debug)]
pub struct FileEntry {
    /// 널 종단 포함 UTF-16 이름
    pub name: Vec<u16>,
    pub is_dir: bool,
    pub size: u64,
    /// FILETIME 원시값 (100ns 단위) — 정렬·표시 시 변환
    pub modified: u64,
}

impl FileEntry {
    /// 표시용 문자열 (널 종단 제외)
    pub fn name_string(&self) -> String {
        String::from_utf16_lossy(&self.name[..self.name.len().saturating_sub(1)])
    }

    /// 소문자 확장자 ("" = 없음/폴더)
    pub fn extension(&self) -> String {
        if self.is_dir {
            return String::new();
        }
        let s = self.name_string();
        match s.rsplit_once('.') {
            Some((stem, ext)) if !stem.is_empty() => ext.to_ascii_lowercase(),
            _ => String::new(),
        }
    }
}

/// 열거 결과
#[derive(Debug)]
pub enum EnumOutcome {
    Ok(Vec<FileEntry>),
    /// 접근 권한 없음
    AccessDenied,
    /// 경로 없음/삭제됨
    NotFound,
    /// 기타 오류
    Error,
}

pub struct EnumResult {
    pub generation: u64,
    pub outcome: EnumOutcome,
}

/// HWND를 워커 스레드로 넘기기 위한 래퍼.
/// 안전성: HWND는 값 타입 핸들이며 PostMessageW는 어느 스레드에서도 호출 가능하다
struct HwndSend(isize);
unsafe impl Send for HwndSend {}

/// 백그라운드 열거 시작 — 완료 시 채널로 결과 전송 후 WM_APP_ENUM_DONE 통지
pub fn spawn_enumerate(path: PathBuf, generation: u64, tx: Sender<EnumResult>, notify: HWND) {
    let notify = HwndSend(notify.0 as isize);
    std::thread::spawn(move || {
        let outcome = enumerate_dir(&path);
        // 수신 측(패널)이 먼저 파괴됐으면 send 실패 — 무해하게 종료
        if tx
            .send(EnumResult {
                generation,
                outcome,
            })
            .is_ok()
        {
            // 안전성: PostMessageW는 스레드 간 안전. 창이 이미 파괴됐으면 실패만 반환
            unsafe {
                let _ = PostMessageW(
                    Some(HWND(notify.0 as *mut core::ffi::c_void)),
                    WM_APP_ENUM_DONE,
                    WPARAM(0),
                    LPARAM(0),
                );
            }
        }
    });
}

/// 디렉터리 1단계 열거 (동기 — 워커 스레드에서 호출).
/// 긴 경로(260+) 지원을 위해 `\\?\` 접두를 사용한다 (NFR-5)
pub fn enumerate_dir(path: &Path) -> EnumOutcome {
    let pattern = to_extended_pattern(path);
    let pattern_h = HSTRING::from(pattern.as_str());
    let mut data = WIN32_FIND_DATAW::default();

    // 안전성: data는 스택 소유, 핸들은 FindClose로 반드시 해제된다
    unsafe {
        let handle = match FindFirstFileExW(
            &pattern_h,
            FindExInfoBasic,
            &mut data as *mut _ as *mut core::ffi::c_void,
            FindExSearchNameMatch,
            None,
            FIND_FIRST_EX_LARGE_FETCH,
        ) {
            Ok(h) => h,
            Err(e) => {
                return match windows::Win32::Foundation::WIN32_ERROR(e.code().0 as u32 & 0xffff) {
                    ERROR_ACCESS_DENIED => EnumOutcome::AccessDenied,
                    ERROR_FILE_NOT_FOUND | ERROR_PATH_NOT_FOUND => EnumOutcome::NotFound,
                    _ => EnumOutcome::Error,
                };
            }
        };

        let mut entries = Vec::new();
        loop {
            push_entry(&data, &mut entries);
            data = WIN32_FIND_DATAW::default();
            if let Err(e) = FindNextFileW(handle, &mut data) {
                let _ = FindClose(handle);
                if windows::Win32::Foundation::WIN32_ERROR(e.code().0 as u32 & 0xffff)
                    == ERROR_NO_MORE_FILES
                {
                    return EnumOutcome::Ok(entries);
                }
                return EnumOutcome::Error;
            }
        }
    }
}

/// `.`·`..`을 제외하고 항목 추가
fn push_entry(data: &WIN32_FIND_DATAW, out: &mut Vec<FileEntry>) {
    let len = data
        .cFileName
        .iter()
        .position(|&c| c == 0)
        .unwrap_or(data.cFileName.len());
    let name_slice = &data.cFileName[..len];
    if name_slice == [b'.' as u16] || name_slice == [b'.' as u16, b'.' as u16] {
        return;
    }
    let mut name = name_slice.to_vec();
    name.push(0);
    out.push(FileEntry {
        name,
        is_dir: (data.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY.0) != 0,
        size: ((data.nFileSizeHigh as u64) << 32) | data.nFileSizeLow as u64,
        modified: ((data.ftLastWriteTime.dwHighDateTime as u64) << 32)
            | data.ftLastWriteTime.dwLowDateTime as u64,
    });
}

/// 검색 패턴 `\\?\<절대경로>\*` 생성 (이미 확장 접두가 있으면 유지)
fn to_extended_pattern(path: &Path) -> String {
    // `\\?\` 접두사는 Win32의 경로 정규화를 **건너뛰게** 한다 — 그래서 이 접두사가 붙은 경로에서는
    // `/`가 디렉터리 구분자로 인식되지 않는다. 주소창에 `C:/Users`처럼 입력하면 열거가 실패하므로
    // 접두사를 붙이기 전에 구분자를 통일한다
    let s = path.to_string_lossy().replace('/', r"\");
    let base = if s.starts_with(r"\\?\") {
        s
    } else if s.starts_with(r"\\") {
        // UNC 경로: \\server\share → \\?\UNC\server\share
        format!(r"\\?\UNC{}", &s[1..])
    } else {
        format!(r"\\?\{s}")
    };
    if base.ends_with('\\') {
        format!("{base}*")
    } else {
        format!("{base}\\*")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("fe_enum_test_{name}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn 확장_패턴은_슬래시를_백슬래시로_바꾼다() {
        // `\\?\` 접두사는 정규화를 건너뛰므로 슬래시가 남으면 열거가 실패한다
        assert_eq!(
            to_extended_pattern(Path::new("C:/Windows/System32")),
            r"\\?\C:\Windows\System32\*"
        );
    }

    #[test]
    fn 확장_패턴은_경로_형태별로_접두사를_붙인다() {
        assert_eq!(
            to_extended_pattern(Path::new(r"C:\Users")),
            r"\\?\C:\Users\*"
        );
        // 끝에 구분자가 있으면 중복해서 붙이지 않는다
        assert_eq!(to_extended_pattern(Path::new(r"C:\")), r"\\?\C:\*");
        // UNC는 \\?\UNC\ 형태로
        assert_eq!(
            to_extended_pattern(Path::new(r"\\server\share")),
            r"\\?\UNC\server\share\*"
        );
        // 이미 확장 접두사가 붙었으면 그대로 둔다
        assert_eq!(
            to_extended_pattern(Path::new(r"\\?\D:\data")),
            r"\\?\D:\data\*"
        );
    }

    #[test]
    fn 슬래시_경로도_실제로_열거된다() {
        let dir = make_temp_dir("slash");
        std::fs::write(dir.join("a.txt"), b"x").unwrap();
        // 백슬래시 경로를 슬래시로 바꿔도 같은 결과가 나와야 한다
        let slash = PathBuf::from(dir.to_string_lossy().replace('\\', "/"));
        let outcome = enumerate_dir(&slash);
        let EnumOutcome::Ok(entries) = outcome else {
            panic!("슬래시 경로 열거 실패");
        };
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name_string(), "a.txt");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn 파일과_폴더를_열거한다() {
        let dir = make_temp_dir("basic");
        std::fs::write(dir.join("a.txt"), b"hello").unwrap();
        std::fs::create_dir(dir.join("sub")).unwrap();

        let EnumOutcome::Ok(entries) = enumerate_dir(&dir) else {
            panic!("열거 실패");
        };
        assert_eq!(entries.len(), 2);
        let a = entries.iter().find(|e| e.name_string() == "a.txt").unwrap();
        assert!(!a.is_dir);
        assert_eq!(a.size, 5);
        let sub = entries.iter().find(|e| e.name_string() == "sub").unwrap();
        assert!(sub.is_dir);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn 없는_경로는_notfound() {
        let ghost = std::env::temp_dir().join("fe_enum_test_ghost_없는폴더_12345");
        assert!(matches!(enumerate_dir(&ghost), EnumOutcome::NotFound));
    }

    #[test]
    fn 유니코드_이름을_보존한다() {
        let dir = make_temp_dir("uni");
        std::fs::write(dir.join("한글 파일 😀.txt"), b"x").unwrap();

        let EnumOutcome::Ok(entries) = enumerate_dir(&dir) else {
            panic!("열거 실패");
        };
        assert_eq!(entries[0].name_string(), "한글 파일 😀.txt");
        assert_eq!(entries[0].extension(), "txt");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn 확장자_추출_규칙() {
        let mk = |name: &str, is_dir: bool| {
            let mut v: Vec<u16> = name.encode_utf16().collect();
            v.push(0);
            FileEntry {
                name: v,
                is_dir,
                size: 0,
                modified: 0,
            }
        };
        assert_eq!(mk("A.TXT", false).extension(), "txt");
        assert_eq!(mk("no_ext", false).extension(), "");
        assert_eq!(mk(".gitignore", false).extension(), "");
        assert_eq!(mk("dir.name", true).extension(), "");
    }
}
