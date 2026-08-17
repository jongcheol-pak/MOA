//! 윈도우가 등록해 둔 특별 폴더 조회 — 즐겨찾기 기본 항목의 출처 (FR-56).
//!
//! **경로를 문자열로 조립하지 않는다** — 사용자가 바탕 화면·다운로드를 다른 드라이브로 옮길 수
//! 있어 `%USERPROFILE%\Desktop` 같은 조합은 틀린 곳을 가리킬 수 있다. 셸에 물으면 옮긴 자리를
//! 그대로 알려 준다.
use crate::fs::icons::IconCache;
use std::path::PathBuf;
use windows::Win32::UI::Shell::{
    FOLDERID_Desktop, FOLDERID_Downloads, KF_FLAG_DEFAULT, SHGetKnownFolderPath,
};

/// 즐겨찾기 맨 위에 서는 기본 항목들 — `(경로, 화면에 보일 이름)`.
///
/// **실재하는 것만 돌려준다**(사용자 결정) — 다운로드 폴더를 지웠거나 옮겨 못 찾으면 그 항목은
/// 빠진다. 누를 수 없는 줄이 목록에 고정으로 남는 것보다 낫다.
///
/// 이름은 셸 표시 이름이라 `바탕 화면`·`다운로드`처럼 화면 언어를 따른다 — 폴더명 그대로면
/// `Desktop`·`Downloads`가 되어 탐색기와 달라 보인다
pub fn default_favorites(icons: &mut IconCache) -> Vec<(PathBuf, String)> {
    [FOLDERID_Desktop, FOLDERID_Downloads]
        .into_iter()
        .filter_map(|id| known_folder(&id))
        .filter(|path| path.is_dir())
        .map(|path| {
            let label = icons
                .shell_display_name(&path.to_string_lossy())
                .unwrap_or_else(|| display_fallback(&path));
            (path, label)
        })
        .collect()
}

/// 등록된 특별 폴더의 경로 — 얻지 못하면 `None`
fn known_folder(id: &windows::core::GUID) -> Option<PathBuf> {
    // 안전성: 셸이 할당한 문자열을 받아 즉시 복사하고 `CoTaskMemFree`로 돌려준다.
    // 실패는 `Result`로 오므로 널 포인터를 역참조할 일이 없다
    unsafe {
        let raw = SHGetKnownFolderPath(id, KF_FLAG_DEFAULT, None).ok()?;
        let path = raw.to_string().ok().map(PathBuf::from);
        windows::Win32::System::Com::CoTaskMemFree(Some(raw.0 as *const _));
        path
    }
}

/// 셸이 이름을 주지 않을 때의 폴백 — 폴더명, 그것도 없으면 경로 전체
fn display_fallback(path: &std::path::Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_string_lossy().into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::icons::shell_test_guard;

    #[test]
    fn 기본_즐겨찾기는_실재하는_폴더만_돌려준다() {
        // 이 PC의 실제 셸을 부르는 시험이다 — 값 자체는 환경마다 다르므로 성질만 본다.
        // 셸 전역 상태를 건드리므로 `icons` 쪽 시험과 같은 잠금을 잡는다
        let _shell = shell_test_guard();
        let mut icons = IconCache::new();
        let items = default_favorites(&mut icons);

        assert!(
            items.len() <= 2,
            "바탕 화면·다운로드 둘뿐인데 더 왔다: {items:?}"
        );
        for (path, label) in &items {
            assert!(path.is_dir(), "실재하지 않는 폴더가 왔다: {path:?}");
            assert!(!label.is_empty(), "이름이 비었다: {path:?}");
            // 라벨이 경로 문자열 그대로면 셸 표시 이름을 거치지 않은 것이다
            assert_ne!(
                label.as_str(),
                path.to_string_lossy().as_ref(),
                "경로가 그대로 이름이 됐다 — 셸 표시 이름을 거치지 않았다"
            );
        }
    }

    #[test]
    fn 바탕_화면이_있으면_맨_앞이다() {
        // 순서가 뒤바뀌면 화면에서 다운로드가 위에 선다 (사용자 결정: 바탕 화면 · 다운로드 차례)
        let _shell = shell_test_guard();
        let mut icons = IconCache::new();
        let items = default_favorites(&mut icons);
        let desktop = known_folder(&FOLDERID_Desktop).filter(|p| p.is_dir());
        if let Some(desktop) = desktop {
            assert_eq!(
                items.first().map(|(path, _)| path),
                Some(&desktop),
                "바탕 화면이 맨 앞이 아니다"
            );
        }
    }
}
