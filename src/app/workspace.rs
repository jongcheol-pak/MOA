//! 워크스페이스 목록 — 순수 로직 (HWND 비의존, 단위테스트 대상)
//!
//! 워크스페이스 하나가 "탐색기 화면 한 벌"(분할 레이아웃·패널·탭)을 가리키며,
//! 이 모듈은 그 **표시 데이터와 목록 연산**만 소유한다 (FR-15~FR-18).
//! 실제 탐색기 창(LayoutHost)의 소유·수명은 `app::window`가 관리한다.
use std::path::Path;

/// 이름 길이 상한(문자 수) — 사이드바 한 줄 표시와 저장 파일 오염 방지
const NAME_MAX_CHARS: usize = 128;
/// 자동 이름 접두 — "워크스페이스 3"처럼 뒤에 번호가 붙는다 (D7)
const AUTO_NAME_PREFIX: &str = "워크스페이스 ";

/// 워크스페이스 식별자 — 생성 순서 증가, 재사용 없음 (`PanelId` 관례)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WorkspaceId(pub u32);

/// 목록 연산 오류
#[derive(Debug, PartialEq, Eq)]
pub enum WorkspaceError {
    /// 마지막 1개는 삭제할 수 없다 (D8 — FR-2의 "마지막 패널 닫기 불가"와 같은 원칙)
    LastWorkspace,
    /// 대상 인덱스가 범위 밖
    NotFound,
}

/// 사이드바 항목 하나의 표시 데이터
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Workspace {
    pub id: WorkspaceId,
    /// 1줄 — 사용자가 정하는 이름
    pub name: String,
    /// 2줄 — 활성 패널의 활성 탭 폴더 경로 (자동 갱신, D6)
    pub subtitle: String,
}

/// 워크스페이스 목록 순수 모델 — UI 비의존 (단위테스트 대상)
pub struct WorkspaceList {
    items: Vec<Workspace>,
    active: usize,
    next_id: u32,
}

impl WorkspaceList {
    /// 워크스페이스 1개("워크스페이스 1")로 시작
    pub fn new() -> WorkspaceList {
        let mut list = WorkspaceList {
            items: Vec::new(),
            active: 0,
            next_id: 0,
        };
        list.add();
        list
    }

    /// 세션 복원용 재구성 — 빈 목록이면 None, 활성 인덱스는 범위로 클램프
    /// (`TabsModel::from_tabs`와 같은 계약)
    pub fn from_names(names: Vec<String>, active: usize) -> Option<WorkspaceList> {
        if names.is_empty() {
            return None;
        }
        let items: Vec<Workspace> = names
            .into_iter()
            .enumerate()
            .map(|(i, name)| Workspace {
                id: WorkspaceId(i as u32),
                name,
                subtitle: String::new(),
            })
            .collect();
        let next_id = items.len() as u32;
        let active = active.min(items.len() - 1);
        Some(WorkspaceList {
            items,
            active,
            next_id,
        })
    }

    /// 표시용 항목 목록 (사이드바가 그대로 그린다)
    pub fn items(&self) -> &[Workspace] {
        &self.items
    }

    /// 항목 수 — is_empty는 제공하지 않음: 목록은 항상 1개 이상(불변식)이라
    /// 항상 false로 오해만 낳는다 (`TabsModel::len`과 같은 이유)
    #[expect(clippy::len_without_is_empty)]
    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn active_index(&self) -> usize {
        self.active
    }

    pub fn active(&self) -> &Workspace {
        &self.items[self.active]
    }

    /// 활성 전환 (범위 밖·동일 인덱스는 무시하고 false)
    pub fn set_active(&mut self, index: usize) -> bool {
        if index < self.items.len() && index != self.active {
            self.active = index;
            true
        } else {
            false
        }
    }

    /// 새 워크스페이스를 목록 끝에 추가하고 활성화한다. 반환값은 새 항목의 인덱스.
    /// 이름은 사용 중이지 않은 최소 번호로 자동 부여 (D7)
    pub fn add(&mut self) -> usize {
        let id = WorkspaceId(self.next_id);
        self.next_id += 1;
        self.items.push(Workspace {
            id,
            name: self.auto_name(),
            subtitle: String::new(),
        });
        self.active = self.items.len() - 1;
        self.active
    }

    /// 이름 변경 — 개행 제거·앞뒤 공백 제거·128자 컷 후 빈 문자열이면 거부(false, 이전 이름 유지)
    pub fn rename(&mut self, index: usize, name: &str) -> bool {
        let Some(item) = self.items.get_mut(index) else {
            return false;
        };
        let normalized = normalize_name(name);
        if normalized.is_empty() {
            return false;
        }
        item.name = normalized;
        true
    }

    /// 부제(경로) 갱신 — 표시 길이를 넘는 경로는 말줄임해 보관한다
    pub fn set_subtitle(&mut self, index: usize, path: &Path) {
        if let Some(item) = self.items.get_mut(index) {
            item.subtitle = elide_path(path, SUBTITLE_MAX_CHARS);
        }
    }

    /// 워크스페이스 삭제 — 마지막 1개면 `LastWorkspace`. 반환값은 삭제 후 활성 인덱스.
    /// 활성 항목이 지워지면 인접 항목(뒤가 없으면 앞)이 활성이 된다
    pub fn remove(&mut self, index: usize) -> Result<usize, WorkspaceError> {
        if index >= self.items.len() {
            return Err(WorkspaceError::NotFound);
        }
        if self.items.len() <= 1 {
            return Err(WorkspaceError::LastWorkspace);
        }
        self.items.remove(index);
        if self.active > index || self.active >= self.items.len() {
            self.active = self.active.saturating_sub(1);
        }
        Ok(self.active)
    }

    /// 순서 변경 — `from` 항목을 결과 목록의 `to` 위치로 옮긴다.
    /// 활성 항목은 인덱스가 아니라 **항목 자체**가 유지된다(D12 드래그 정렬)
    pub fn reorder(&mut self, from: usize, to: usize) -> bool {
        if from >= self.items.len() || to >= self.items.len() || from == to {
            return false;
        }
        let active_id = self.items[self.active].id;
        let item = self.items.remove(from);
        self.items.insert(to, item);
        // 활성 항목의 새 위치를 되찾는다 (id는 재사용되지 않으므로 유일)
        if let Some(pos) = self.items.iter().position(|w| w.id == active_id) {
            self.active = pos;
        }
        true
    }

    /// 사용 중이지 않은 최소 번호로 자동 이름 생성 (D7 — 삭제 후 재생성 시 중복 방지)
    fn auto_name(&self) -> String {
        let used: Vec<u32> = self
            .items
            .iter()
            .filter_map(|w| w.name.strip_prefix(AUTO_NAME_PREFIX))
            .filter_map(|n| n.parse::<u32>().ok())
            .collect();
        let mut candidate = 1;
        while used.contains(&candidate) {
            candidate += 1;
        }
        format!("{AUTO_NAME_PREFIX}{candidate}")
    }
}

impl Default for WorkspaceList {
    fn default() -> WorkspaceList {
        WorkspaceList::new()
    }
}

/// 부제로 보관하는 경로의 최대 문자 수 — 사이드바 폭(최대 480px)에서도 넘치는 길이라
/// 그리기 전에 잘라 GDI 말줄임(DT_END_ELLIPSIS)이 다룰 양을 제한한다
const SUBTITLE_MAX_CHARS: usize = 120;

/// 이름 정규화 — 개행·탭을 공백으로 바꾸고 앞뒤 공백 제거 후 상한 문자 수로 컷.
/// 개행을 없애는 이유: 사이드바는 한 줄로 그리고 세션 파일에도 그대로 저장된다
fn normalize_name(name: &str) -> String {
    let flattened: String = name
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect();
    flattened.trim().chars().take(NAME_MAX_CHARS).collect()
}

/// 긴 경로를 `max_chars` 이하로 말줄임한다 (초과분은 끝에서 자르고 '…' 부착)
pub fn elide_path(path: &Path, max_chars: usize) -> String {
    let text = path.to_string_lossy();
    if text.chars().count() <= max_chars {
        return text.into_owned();
    }
    let head: String = text.chars().take(max_chars.saturating_sub(1)).collect();
    format!("{head}…")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn 새_목록은_워크스페이스_1개로_시작한다() {
        let list = WorkspaceList::new();
        assert_eq!(list.len(), 1);
        assert_eq!(list.active_index(), 0);
        assert_eq!(list.active().name, "워크스페이스 1");
    }

    #[test]
    fn 자동_이름은_사용중이지_않은_최소_번호다() {
        let mut list = WorkspaceList::new();
        list.add(); // 워크스페이스 2
        list.add(); // 워크스페이스 3
        assert_eq!(list.len(), 3);

        list.remove(1).unwrap(); // "워크스페이스 2" 제거
        let index = list.add();
        assert_eq!(list.items()[index].name, "워크스페이스 2");
    }

    #[test]
    fn 마지막_하나는_삭제할_수_없다() {
        let mut list = WorkspaceList::new();
        assert_eq!(list.remove(0), Err(WorkspaceError::LastWorkspace));
        assert_eq!(list.len(), 1);
    }

    #[test]
    fn 범위_밖_삭제는_찾을_수_없음_오류다() {
        let mut list = WorkspaceList::new();
        list.add();
        assert_eq!(list.remove(9), Err(WorkspaceError::NotFound));
        assert_eq!(list.len(), 2);
    }

    #[test]
    fn 활성_항목이_삭제되면_인접_항목이_활성이_된다() {
        let mut list = WorkspaceList::new();
        list.add();
        list.add(); // 활성 = 2
        assert_eq!(list.active_index(), 2);

        let active = list.remove(2).unwrap();
        assert_eq!(active, 1);

        // 활성(1)보다 앞을 지우면 활성 항목 자체는 유지되고 인덱스만 당겨진다
        let id_before = list.active().id;
        let active = list.remove(0).unwrap();
        assert_eq!(active, 0);
        assert_eq!(list.active().id, id_before);
    }

    #[test]
    fn 순서를_바꿔도_활성_항목은_유지된다() {
        let mut list = WorkspaceList::new();
        list.add();
        list.add();
        list.set_active(0);
        let active_id = list.active().id;

        assert!(list.reorder(0, 2));
        assert_eq!(list.active_index(), 2);
        assert_eq!(list.active().id, active_id);
    }

    #[test]
    fn 범위_밖이거나_같은_자리_이동은_무시된다() {
        let mut list = WorkspaceList::new();
        list.add();
        let names: Vec<String> = list.items().iter().map(|w| w.name.clone()).collect();

        assert!(!list.reorder(0, 0));
        assert!(!list.reorder(0, 9));
        assert!(!list.reorder(9, 0));
        let after: Vec<String> = list.items().iter().map(|w| w.name.clone()).collect();
        assert_eq!(names, after);
    }

    #[test]
    fn 빈_이름이나_공백만으로는_이름을_바꿀_수_없다() {
        let mut list = WorkspaceList::new();
        assert!(!list.rename(0, ""));
        assert!(!list.rename(0, "   "));
        assert!(!list.rename(0, "\n\t"));
        assert_eq!(list.active().name, "워크스페이스 1");

        assert!(list.rename(0, "  자료 정리  "));
        assert_eq!(list.active().name, "자료 정리");
    }

    #[test]
    fn 이름의_개행은_공백이_되고_길이는_상한으로_잘린다() {
        let mut list = WorkspaceList::new();
        assert!(list.rename(0, "앞\n뒤"));
        assert_eq!(list.active().name, "앞 뒤");

        let long = "가".repeat(NAME_MAX_CHARS + 50);
        assert!(list.rename(0, &long));
        assert_eq!(list.active().name.chars().count(), NAME_MAX_CHARS);
    }

    #[test]
    fn 범위_밖_활성_전환은_무시된다() {
        let mut list = WorkspaceList::new();
        list.add();
        assert!(!list.set_active(9));
        assert_eq!(list.active_index(), 1);
        assert!(!list.set_active(1)); // 동일 인덱스
        assert!(list.set_active(0));
    }

    #[test]
    fn 긴_경로는_말줄임된다() {
        let short = PathBuf::from("C:\\Users");
        assert_eq!(elide_path(&short, 20), "C:\\Users");

        let long = PathBuf::from(format!("C:\\{}", "가".repeat(300)));
        let elided = elide_path(&long, 120);
        assert_eq!(elided.chars().count(), 120);
        assert!(elided.ends_with('…'));
    }

    #[test]
    fn 부제는_경로에서_갱신된다() {
        let mut list = WorkspaceList::new();
        list.set_subtitle(0, Path::new("D:\\작업\\프로젝트"));
        assert_eq!(list.active().subtitle, "D:\\작업\\프로젝트");

        list.set_subtitle(9, Path::new("C:\\")); // 범위 밖은 무시
        assert_eq!(list.active().subtitle, "D:\\작업\\프로젝트");
    }

    #[test]
    fn 세션_복원은_빈_목록을_거부하고_활성을_클램프한다() {
        assert!(WorkspaceList::from_names(Vec::new(), 0).is_none());

        let list = WorkspaceList::from_names(vec!["가".into(), "나".into()], 9).unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!(list.active_index(), 1);
        assert_eq!(list.active().name, "나");
    }
}
