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
    /// 사용자가 담은 항목의 차례를 바꾼다 (FR-56).
    ///
    /// **자리는 `paths()`의 자리다** — 화면에는 기본 두 줄이 앞에 서 있어 그 자리와
    /// 어긋난다. 그 변환은 트리가 한 곳에서 하고 여기까지 오는 값은 사용자 항목 기준이다
    Reorder { from: usize, to: usize },
}

/// 화면에 설 즐겨찾기 한 줄 (FR-56).
///
/// **라벨은 기본 항목만 갖는다** — 바탕 화면·다운로드는 셸 표시 이름을 쓰기 때문이다
/// (`바탕 화면`처럼 화면 언어를 따른다). 사용자가 담은 항목은 `None`이고 화면이
/// 폴더명으로 그린다 — 폴더명을 뽑는 규칙을 이 계층에 복제하지 않으려는 것이다
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FavoriteEntry {
    pub path: PathBuf,
    pub label: Option<String>,
    /// 우클릭으로 뺄 수 있는가 — 기본 항목은 `false`다(사용자 결정: 해제할 수 없음)
    pub removable: bool,
    /// 가리키는 폴더가 지금 없는가 (FR-56) — 흐린 글씨로 보이고 눌러도 이동하지 않는다.
    ///
    /// **없다고 목록에서 지우지는 않는다**(사용자 결정 2026-08-27) — 잠시 끊긴
    /// 네트워크 드라이브나 뽑아 둔 USB의 폴더가 조용히 사라지면 되돌릴 길이 없다.
    ///
    /// 「있음·없음·확인중」 세 값을 담는 열거형을 만들지 않는다 — **확인 전은 정상으로
    /// 그리기로 정해** 화면에서 갈리는 것이 둘뿐이다(흐렸다가 되돌리면 깜빡임이 된다)
    pub missing: bool,
}

/// 즐겨찾기 목록 — **더한 차례를 그대로 지킨다**(사용자 결정: 이름순이 아니라 추가순).
///
/// 기본 항목(바탕 화면·다운로드)이 맨 위에 서고 사용자가 담은 것이 그 아래 온다.
/// **둘은 저장 경계가 다르다** — `paths()`는 사용자 항목만 주고(세션에 담기는 것),
/// `entries()`는 기본까지 합쳐 준다(화면에 그리는 것). 기본을 저장하면 그 폴더가
/// 옮겨졌을 때 옛 경로가 파일에 굳는다
///
/// 정렬 옵션·개수 상한·변경 통지를 두지 않는다(plan 비추상화 선언) — 사용자가 손으로
/// 몇 개 넣는 목록이라 매 프레임 그대로 읽으면 충분하다
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FavoriteStore {
    /// 사용자가 담은 것 — 이것만 세션에 저장된다
    paths: Vec<PathBuf>,
    /// 윈도우가 정해 준 기본 항목 `(경로, 셸 표시 이름)` — 저장하지 않는다
    defaults: Vec<(PathBuf, String)>,
    /// 확인 결과 **지금 없는** 경로들 — 저장하지 않는다(다음 실행에 다시 확인한다).
    ///
    /// 비어 있으면 「아직 한 번도 확인하지 않았다」와 「전부 있다」 둘 다이며, 두 경우 다
    /// 정상으로 그린다 — 앱을 막 켠 직후를 흐리게 그렸다가 되돌리면 깜빡임이 된다
    missing: Vec<PathBuf>,
}

impl FavoriteStore {
    pub fn new() -> FavoriteStore {
        FavoriteStore::default()
    }

    /// 사용자 항목만으로 만든다 — 저장은 문자열이므로 경로로 바꿔 받는다.
    /// **손으로 편집된 파일에 중복이 들어 있을 수 있어** 여기서도 한 번 걸러 낸다.
    ///
    /// **모듈 밖으로 열지 않는다** — 이 길로 복원하면 기본 항목이 통째로 사라진다.
    /// 바깥은 언제나 `with_defaults`를 쓴다
    fn from_paths(paths: impl IntoIterator<Item = PathBuf>) -> FavoriteStore {
        let mut store = FavoriteStore::new();
        for path in paths {
            store.add(path);
        }
        store
    }

    /// 기본 항목을 얹어 되살린다 — **복원도 이 길을 쓴다**.
    ///
    /// 세션에는 사용자 항목만 담기므로, 복원할 때마다 기본을 다시 실어야 한다.
    /// `from_paths`로 통째로 갈아치우면 settings.json이 있는 모든 사용자에게서
    /// 기본 둘이 사라진다
    pub fn with_defaults(
        defaults: impl IntoIterator<Item = (PathBuf, String)>,
        paths: impl IntoIterator<Item = PathBuf>,
    ) -> FavoriteStore {
        let mut store = FavoriteStore::from_paths(paths);
        store.defaults = defaults.into_iter().collect();
        store
    }

    /// **저장할 목록** — 사용자가 담은 것만이다(기본 항목은 빠진다).
    ///
    /// 기본은 시작할 때 셸에 다시 물으므로 파일에 남길 이유가 없고, 남기면 그 폴더를
    /// 옮겼을 때 옛 경로가 굳는다
    pub fn paths(&self) -> &[PathBuf] {
        &self.paths
    }

    /// **화면에 그릴 목록** — 기본이 앞, 사용자가 뒤다(사용자 결정: 한 목록).
    ///
    /// 사용자 항목에 기본과 같은 경로가 들어 있으면(이 기능 전에 담았을 수 있다)
    /// 기본 쪽만 남긴다 — 같은 폴더가 두 줄로 서면 어느 것이 해제되는지 알 수 없다
    pub fn entries(&self) -> Vec<FavoriteEntry> {
        let mut entries: Vec<FavoriteEntry> = self
            .defaults
            .iter()
            .map(|(path, label)| FavoriteEntry {
                path: path.clone(),
                label: Some(label.clone()),
                removable: false,
                missing: self.is_missing(path),
            })
            .collect();
        for path in &self.paths {
            if self.is_default(path) {
                continue;
            }
            entries.push(FavoriteEntry {
                path: path.clone(),
                label: None,
                removable: true,
                missing: self.is_missing(path),
            });
        }
        entries
    }

    /// 윈도우가 정해 준 기본 항목인가 — 목록 합치기·해제 판정이 같은 규칙을 본다
    fn is_default(&self, path: &Path) -> bool {
        self.defaults.iter().any(|(known, _)| known == path)
    }

    /// 실재를 확인할 경로 전부 — **기본 두 줄까지 담는다** (FR-56 · FR-67).
    ///
    /// 기본 항목도 사라질 수 있다(옮겼거나 지웠거나) — `entries()`가 둘을 같은 규칙으로
    /// 그리므로 확인도 같은 목록을 본다. 확인은 워커가 하고 이 함수는 목록만 내준다(D5)
    pub fn watched_paths(&self) -> Vec<PathBuf> {
        self.entries().into_iter().map(|entry| entry.path).collect()
    }

    /// 마지막 확인에서 없던 경로인가 — 기본 항목도 같은 규칙을 따른다 (FR-56)
    fn is_missing(&self, path: &Path) -> bool {
        self.missing.iter().any(|gone| gone == path)
    }

    /// 실재 확인 결과를 반영한다 — 없는 경로 목록으로 통째로 갈아 끼운다 (FR-56).
    ///
    /// **이 모듈은 파일시스템을 직접 묻지 않는다**(D5) — `path.is_dir()`은 끊긴 네트워크
    /// 경로에서 수십 초 걸리는 호출이라 그리기 경로에서 부르면 화면이 그만큼 멈춘다.
    /// 확인은 워커가 하고 여기는 그 결과를 값으로 받기만 한다
    pub fn set_missing(&mut self, missing: &[PathBuf]) {
        self.missing = missing.to_vec();
    }

    /// 이미 담긴 폴더인가 — 메뉴의 `즐겨찾기에 담기` 줄을 비활성으로 할지 정한다.
    ///
    /// **기본 항목도 담긴 것으로 본다** — 바탕 화면이 이미 목록에 서 있는데 그 폴더의
    /// 트리 노드에서 다시 담을 수 있으면 같은 줄이 둘이 된다
    pub fn contains(&self, path: &Path) -> bool {
        self.paths.iter().any(|known| known == path) || self.is_default(path)
    }

    /// 맨 아래에 더한다. **이미 있으면 아무 일도 하지 않는다** —
    /// 같은 폴더가 두 줄로 보이면 어느 것을 지워야 할지 알 수 없다
    pub fn add(&mut self, path: PathBuf) {
        if self.contains(&path) {
            return;
        }
        self.paths.push(path);
    }

    /// 목록에서 뺀다 — 없는 폴더면 아무 일도 하지 않는다.
    ///
    /// **기본 항목은 빠지지 않는다**(사용자 결정: 해제할 수 없음). 화면이 그 메뉴를
    /// 아예 보이지 않지만, 규칙은 순수 계층에도 둔다 — 화면만 막으면 다른 경로로
    /// 요청이 올라왔을 때 조용히 지워진다
    pub fn remove(&mut self, path: &Path) {
        if self.is_default(path) {
            return;
        }
        self.paths.retain(|known| known != path);
    }

    /// 사용자가 담은 항목의 차례를 바꾼다 — `from` 자리의 것을 꺼내 `to` 자리에 넣는다 (FR-56).
    ///
    /// **기본 두 줄은 이 목록에 없다** — 그 둘은 언제나 맨 위이고 끌 수 없다.
    /// 범위 밖이거나 제자리면 아무 일도 하지 않는다
    pub fn reorder(&mut self, from: usize, to: usize) {
        if from >= self.paths.len() || to >= self.paths.len() || from == to {
            return;
        }
        let moved = self.paths.remove(from);
        self.paths.insert(to, moved);
    }

    /// 트리가 올린 조작을 반영한다 (plan D8).
    ///
    /// 화면 쪽은 이 함수를 부르기만 한다 — 무엇이 늘고 주는지의 규칙이 여기 있어야
    /// 시험이 그것을 직접 확인할 수 있다
    pub fn apply(&mut self, action: FavoriteAction) {
        match action {
            FavoriteAction::Add(path) => self.add(path),
            FavoriteAction::Remove(path) => self.remove(&path),
            FavoriteAction::Reorder { from, to } => self.reorder(from, to),
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
    fn 차례를_바꾸면_저장_목록의_차례도_바뀐다() {
        // Acceptance ⓐ — 세션에 담기는 것은 `paths()`이므로 이 차례가 곧 재시작 뒤의 차례다
        let mut store = FavoriteStore::new();
        for name in [r"D:\하나", r"D:\둘", r"D:\셋"] {
            store.add(path(name));
        }
        store.apply(FavoriteAction::Reorder { from: 2, to: 0 });
        assert_eq!(
            store.paths(),
            [path(r"D:\셋"), path(r"D:\하나"), path(r"D:\둘")]
        );
    }

    #[test]
    fn 범위_밖이나_제자리로_옮기면_아무_일도_없다() {
        let mut store = FavoriteStore::new();
        for name in [r"D:\하나", r"D:\둘"] {
            store.add(path(name));
        }
        let before = store.paths().to_vec();
        store.reorder(0, 0);
        store.reorder(0, 5);
        store.reorder(5, 0);
        assert_eq!(store.paths(), before.as_slice());
    }

    #[test]
    fn 기본_항목은_차례_바꾸기의_대상이_아니다() {
        // Acceptance ⓑ — 기본 두 줄은 언제나 맨 위다. `reorder`가 보는 목록에 아예 없다
        let 바탕 = path(r"C:\Users\누구\Desktop");
        let mut store = FavoriteStore::with_defaults(
            [(바탕.clone(), "바탕 화면".to_owned())],
            [path(r"D:\하나"), path(r"D:\둘")],
        );
        store.apply(FavoriteAction::Reorder { from: 1, to: 0 });
        assert_eq!(store.paths(), [path(r"D:\둘"), path(r"D:\하나")]);
        assert_eq!(
            store.entries()[0].path,
            바탕,
            "기본 항목이 여전히 맨 위에 선다"
        );
    }

    #[test]
    fn 확인할_경로에_기본_항목도_든다() {
        // FR-67 — 바탕 화면·다운로드도 사라질 수 있다(옮겼거나 지웠거나)
        let 바탕 = path(r"C:\Users\누구\Desktop");
        let store = FavoriteStore::with_defaults(
            [(바탕.clone(), "바탕 화면".to_owned())],
            [path(r"D:\작업")],
        );
        assert_eq!(store.watched_paths(), vec![바탕, path(r"D:\작업")]);
    }

    #[test]
    fn 확인하기_전에는_모두_있는_것으로_본다() {
        // Acceptance — 앱을 막 켠 직후를 흐리게 그렸다가 되돌리면 깜빡임이 된다
        let mut store = FavoriteStore::new();
        store.add(path(r"D:\작업"));
        assert!(store.entries().iter().all(|entry| !entry.missing));
    }

    #[test]
    fn 없다고_알려_준_경로만_흐려진다() {
        // Acceptance ⓐ — 확인은 워커가 하고 저장소는 그 결과를 받기만 한다 (D5)
        let mut store = FavoriteStore::with_defaults(
            [(path(r"C:\Users\누구\Desktop"), "바탕 화면".to_owned())],
            [path(r"D:\작업"), path(r"E:\보관")],
        );
        store.set_missing(&[path(r"D:\작업")]);

        let entries = store.entries();
        let 흐린것: Vec<&PathBuf> = entries
            .iter()
            .filter(|entry| entry.missing)
            .map(|entry| &entry.path)
            .collect();
        assert_eq!(흐린것, vec![&path(r"D:\작업")]);
    }

    #[test]
    fn 기본_항목도_같은_규칙을_따른다() {
        // Acceptance ⓔ — 바탕 화면·다운로드도 사라질 수 있다(옮겨졌거나 지워졌거나)
        let 바탕 = path(r"C:\Users\누구\Desktop");
        let mut store = FavoriteStore::with_defaults([(바탕.clone(), "바탕 화면".to_owned())], []);
        store.set_missing(std::slice::from_ref(&바탕));
        assert!(store.entries()[0].missing);
    }

    #[test]
    fn 다시_생기면_정상으로_돌아온다() {
        // Acceptance ⓓ — 자동으로 지우지 않으므로 되돌아올 자리가 남아 있다
        let mut store = FavoriteStore::new();
        store.add(path(r"D:\작업"));
        store.set_missing(&[path(r"D:\작업")]);
        assert!(store.entries()[0].missing);

        store.set_missing(&[]);
        assert!(!store.entries()[0].missing);
    }

    #[test]
    fn 사라져도_목록에서_빠지지_않는다() {
        // 사용자 결정 2026-08-27 — 잠시 끊긴 네트워크·USB 폴더가 조용히 사라지면 안 된다.
        // 저장되는 목록(`paths`)도 그대로여야 다음 실행에 되살아난다
        let mut store = FavoriteStore::new();
        store.add(path(r"D:\작업"));
        store.set_missing(&[path(r"D:\작업")]);
        assert_eq!(store.paths(), [path(r"D:\작업")]);
        assert_eq!(store.entries().len(), 1);
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

    fn 기본() -> Vec<(PathBuf, String)> {
        vec![
            (path(r"C:\Users\누구\Desktop"), "바탕 화면".to_owned()),
            (path(r"C:\Users\누구\Downloads"), "다운로드".to_owned()),
        ]
    }

    #[test]
    fn 기본_항목이_사용자_항목보다_앞에_선다() {
        // 사용자 결정 — 한 목록이고 기본이 위다
        let store = FavoriteStore::with_defaults(기본(), [path(r"D:\작업")]);
        let entries = store.entries();

        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].label.as_deref(), Some("바탕 화면"));
        assert_eq!(entries[1].label.as_deref(), Some("다운로드"));
        assert_eq!(entries[2].path, path(r"D:\작업"));
        assert_eq!(entries[2].label, None, "사용자 항목은 라벨을 갖지 않는다");
    }

    #[test]
    fn 기본_항목은_해제할_수_없다() {
        // 사용자 결정 — 화면이 메뉴를 감추지만 규칙은 여기에도 둔다
        let mut store = FavoriteStore::with_defaults(기본(), []);
        let desktop = path(r"C:\Users\누구\Desktop");

        store.apply(FavoriteAction::Remove(desktop.clone()));

        assert!(store.contains(&desktop), "기본 항목이 빠졌다");
        assert_eq!(store.entries().len(), 2);
        assert!(
            !store.entries()[0].removable,
            "기본 항목이 해제 가능으로 표시됐다"
        );
    }

    #[test]
    fn 사용자가_담은_기본_경로는_한_줄로_합쳐진다() {
        // 이 기능 전에 바탕 화면을 손으로 담아 둔 세션 — 같은 폴더가 두 줄이 되면 안 된다
        let desktop = path(r"C:\Users\누구\Desktop");
        let store = FavoriteStore::with_defaults(기본(), [desktop.clone(), path(r"D:\작업")]);
        let entries = store.entries();

        assert_eq!(entries.len(), 3, "바탕 화면이 두 줄이 됐다");
        assert_eq!(entries.iter().filter(|e| e.path == desktop).count(), 1);
        assert!(!entries[0].removable, "합쳐진 줄은 기본 쪽 규칙을 따른다");
    }

    #[test]
    fn 저장_목록에는_기본_항목이_실리지_않는다() {
        // `paths()`(저장)와 `entries()`(화면)의 경계 — 기본을 저장하면 그 폴더가
        // 옮겨졌을 때 옛 경로가 파일에 굳는다
        let store = FavoriteStore::with_defaults(기본(), [path(r"D:\작업")]);

        assert_eq!(store.paths(), [path(r"D:\작업")]);
        assert_eq!(store.entries().len(), 3);
    }

    #[test]
    fn 복원해도_기본_항목이_남는다() {
        // 저장 → 복원 왕복. `from_paths`로 통째로 갈아치우면 기본이 사라지던 자리다
        let store = FavoriteStore::with_defaults(기본(), [path(r"D:\작업")]);
        let 저장된: Vec<PathBuf> = store.paths().to_vec();

        let 복원 = FavoriteStore::with_defaults(기본(), 저장된);

        assert_eq!(복원.entries().len(), 3, "기본 항목이 복원에서 사라졌다");
        assert_eq!(복원.paths().len(), 1, "기본 항목이 저장 목록에 섞였다");
    }

    #[test]
    fn 기본_항목은_이미_담긴_것으로_본다() {
        // 트리 메뉴의 `즐겨찾기에 담기`가 비활성이어야 한다 — 아니면 같은 줄이 둘이 된다
        let store = FavoriteStore::with_defaults(기본(), []);
        assert!(store.contains(&path(r"C:\Users\누구\Desktop")));
    }
}
