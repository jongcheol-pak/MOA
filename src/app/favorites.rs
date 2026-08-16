//! 폴더 트리 즐겨찾기 (FR-56) — 자주 쓰는 로컬 폴더의 바로가기 목록.
//!
//! **앱에 하나뿐이다** — 모든 워크스페이스·패널·탭이 같은 목록을 보고, 세션 파일에 담겨
//! 재시작해도 남는다(사용자 결정 2026-08-16). 원격 폴더는 담지 않는다.
//!
//! 이 모듈은 화면을 모른다 — 트리는 `FavoriteAction`을 만들어 올리기만 하고, 그것을
//! 목록에 반영하는 규칙은 여기 `apply` 하나에 있다. **적용 규칙을 화면 쪽(`ExplorerApp`)에
//! 두면 시험으로 덮을 수 없기 때문이다** — 그 타입은 `eframe::CreationContext`가 있어야
//! 만들어져 단위 시험에서 세울 수 없다(plan D8).
use std::path::{Path, PathBuf};

/// 트리에서 올라온 즐겨찾기 조작 — 실행은 `FavoriteStore::apply`가 한다
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FavoriteAction {
    /// 이 폴더를 목록 맨 아래에 더한다
    Add(PathBuf),
    /// 이 폴더를 목록에서 뺀다
    Remove(PathBuf),
}

/// 즐겨찾기 목록 — **더한 차례를 그대로 지킨다**(사용자 결정: 이름순이 아니라 추가순).
///
/// 정렬 옵션·개수 상한·변경 통지를 두지 않는다(plan 비추상화 선언) — 사용자가 손으로
/// 몇 개 넣는 목록이라 매 프레임 그대로 읽으면 충분하다
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FavoriteStore {
    paths: Vec<PathBuf>,
}

impl FavoriteStore {
    pub fn new() -> FavoriteStore {
        FavoriteStore::default()
    }

    /// 세션에서 되살린다 — 저장은 문자열이므로 경로로 바꿔 받는다.
    /// **손으로 편집된 파일에 중복이 들어 있을 수 있어** 여기서도 한 번 걸러 낸다
    pub fn from_paths(paths: impl IntoIterator<Item = PathBuf>) -> FavoriteStore {
        let mut store = FavoriteStore::new();
        for path in paths {
            store.add(path);
        }
        store
    }

    /// 저장할 목록 — 순서 그대로다
    pub fn paths(&self) -> &[PathBuf] {
        &self.paths
    }

    /// 이미 담긴 폴더인가 — 메뉴의 `즐겨찾기` 줄을 비활성으로 할지 정한다
    pub fn contains(&self, path: &Path) -> bool {
        self.paths.iter().any(|known| known == path)
    }

    /// 맨 아래에 더한다. **이미 있으면 아무 일도 하지 않는다** —
    /// 같은 폴더가 두 줄로 보이면 어느 것을 지워야 할지 알 수 없다
    pub fn add(&mut self, path: PathBuf) {
        if self.contains(&path) {
            return;
        }
        self.paths.push(path);
    }

    /// 목록에서 뺀다 — 없는 폴더면 아무 일도 하지 않는다
    pub fn remove(&mut self, path: &Path) {
        self.paths.retain(|known| known != path);
    }

    /// 트리가 올린 조작을 반영한다 (plan D8).
    ///
    /// 화면 쪽은 이 함수를 부르기만 한다 — 무엇이 늘고 주는지의 규칙이 여기 있어야
    /// 시험이 그것을 직접 확인할 수 있다
    pub fn apply(&mut self, action: FavoriteAction) {
        match action {
            FavoriteAction::Add(path) => self.add(path),
            FavoriteAction::Remove(path) => self.remove(&path),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn path(text: &str) -> PathBuf {
        PathBuf::from(text)
    }

    #[test]
    fn 더한_차례를_지킨다() {
        let mut store = FavoriteStore::new();
        store.add(path(r"D:\작업"));
        store.add(path(r"C:\Users"));
        store.add(path(r"E:\사진"));

        assert_eq!(
            store.paths(),
            [path(r"D:\작업"), path(r"C:\Users"), path(r"E:\사진")],
            "이름순으로 뒤바뀌었다"
        );
    }

    #[test]
    fn 같은_폴더를_두_번_더해도_한_줄이다() {
        let mut store = FavoriteStore::new();
        store.add(path(r"D:\작업"));
        store.add(path(r"D:\작업"));

        assert_eq!(store.paths().len(), 1, "같은 폴더가 두 줄로 담겼다");
        assert!(store.contains(&path(r"D:\작업")));
    }

    #[test]
    fn 빼도_나머지_순서는_그대로다() {
        let mut store =
            FavoriteStore::from_paths([path(r"D:\작업"), path(r"C:\Users"), path(r"E:\사진")]);

        store.remove(&path(r"D:\작업"));

        assert_eq!(
            store.paths(),
            [path(r"C:\Users"), path(r"E:\사진")],
            "앞의 것을 뺐더니 나머지 차례가 흐트러졌다"
        );
        assert!(!store.contains(&path(r"D:\작업")));
    }

    #[test]
    fn 없는_폴더를_빼도_아무_일이_없다() {
        let mut store = FavoriteStore::from_paths([path(r"D:\작업")]);
        store.remove(&path(r"Z:\없음"));
        assert_eq!(store.paths().len(), 1);
    }

    #[test]
    fn 되살릴_때_중복은_걸러진다() {
        // 세션 파일은 손으로 편집될 수 있다
        let store =
            FavoriteStore::from_paths([path(r"D:\작업"), path(r"D:\작업"), path(r"C:\Users")]);
        assert_eq!(store.paths(), [path(r"D:\작업"), path(r"C:\Users")]);
    }

    #[test]
    fn 트리가_올린_조작이_목록에_반영된다() {
        // plan D8 — 적용 규칙이 이 계층에 있어야 시험으로 덮인다
        let mut store = FavoriteStore::from_paths([path(r"D:\작업")]);

        store.apply(FavoriteAction::Add(path(r"C:\Users")));
        assert_eq!(store.paths(), [path(r"D:\작업"), path(r"C:\Users")]);

        store.apply(FavoriteAction::Remove(path(r"D:\작업")));
        assert_eq!(store.paths(), [path(r"C:\Users")]);
    }
}
