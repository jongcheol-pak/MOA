//! 주소창 입력 정규화 (FR-6) — 따옴표·공백을 벗기고 상대 경로를 절대화한다.
//!
//! 화면은 `ui::address_bar`가 그린다. 여기 남은 것은 그 입력 처리 규칙 하나다.
use std::path::{Path, PathBuf};

/// 입력 문자열 정규화 — 따옴표·공백 제거 후, 상대 경로면 현재 경로 기준 절대화 (T5 Edge)
pub fn normalize_input(current: &Path, input: &str) -> Option<PathBuf> {
    let trimmed = input.trim().trim_matches('"').trim();
    if trimmed.is_empty() {
        return None;
    }
    let p = Path::new(trimmed);
    if p.is_absolute() || trimmed.starts_with(r"\\") {
        Some(p.to_path_buf())
    } else {
        Some(current.join(p))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 절대_경로는_그대로() {
        let cur = Path::new("C:\\base");
        assert_eq!(
            normalize_input(cur, r"D:\data").unwrap(),
            PathBuf::from(r"D:\data")
        );
    }

    #[test]
    fn 따옴표와_공백을_벗긴다() {
        let cur = Path::new("C:\\base");
        assert_eq!(
            normalize_input(cur, "  \"D:\\my folder\"  ").unwrap(),
            PathBuf::from(r"D:\my folder")
        );
    }

    #[test]
    fn 상대_경로는_현재_기준_절대화() {
        let cur = Path::new(r"C:\base");
        assert_eq!(
            normalize_input(cur, "sub\\dir").unwrap(),
            PathBuf::from(r"C:\base\sub\dir")
        );
    }

    #[test]
    fn 빈_입력은_none() {
        assert!(normalize_input(Path::new("C:\\"), "   ").is_none());
    }

    #[test]
    fn unc_경로_지원() {
        assert!(
            normalize_input(Path::new("C:\\"), r"\\server\share")
                .unwrap()
                .to_string_lossy()
                .starts_with(r"\\server")
        );
    }
}
