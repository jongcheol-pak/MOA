//! 원격 폴더 트리가 읽어 둔 하위 폴더들 (FR-9의 원격 판·T24).
//!
//! 트리는 한 번 펼친 폴더를 **다시 조회하지 않는다** — 서버 왕복은 로컬 열거보다 훨씬
//! 비싸고, 펼쳤다 접었다 하는 것만으로 명령이 쌓이면 목록 조회까지 밀린다.
//!
//! **LRU로 만들지 않는다**(plan 비추상화 선언) — 연결이 끊기면 그 연결의 것을 통째로
//! 버리므로 상한이 저절로 생긴다. 살아 있는 연결 하나가 담는 양은 사용자가 실제로 펼친
//! 폴더 수만큼이다.
use crate::remote::connection::ConnectionId;
use crate::remote::types::{RemoteEntry, RemotePath};
use std::collections::HashMap;

/// 폴더 하나의 상태
#[derive(Debug, Clone, PartialEq)]
pub enum TreeNode {
    /// 조회를 보냈고 답을 기다린다
    Loading,
    /// 하위 **폴더**들 (파일은 담지 않는다 — 트리는 폴더만 보여 준다)
    Loaded(Vec<RemoteEntry>),
    /// 읽지 못했다 — 그 노드에만 사유를 보이고 트리는 그대로 둔다 (plan Edge Case)
    Failed(String),
}

/// 연결 하나가 읽어 둔 것들
#[derive(Debug, Default)]
struct ConnTree {
    /// 지금 세대. 새로 고침·재연결 때 올라간다 — **늦게 온 지난 세대의 답을 버리기 위한 것**이다
    generation: u64,
    nodes: HashMap<RemotePath, TreeNode>,
}

/// 연결별 하위 폴더 캐시
#[derive(Debug, Default)]
pub struct TreeCache {
    conns: HashMap<ConnectionId, ConnTree>,
}

impl TreeCache {
    pub fn new() -> TreeCache {
        TreeCache::default()
    }

    /// 그 폴더를 조회해야 하는가. 조회가 필요하면 **그 요청에 실을 세대**를 돌려주고
    /// 노드를 `Loading`으로 표시한다.
    ///
    /// 이미 읽었거나 읽는 중이면 `None`이다 — 두 번째 펼침이 서버에 닿지 않는 이유가 여기다
    /// (Acceptance ②)
    pub fn begin(&mut self, conn: ConnectionId, path: &RemotePath) -> Option<u64> {
        let tree = self.conns.entry(conn).or_default();
        if tree.nodes.contains_key(path) {
            return None;
        }
        tree.nodes.insert(path.clone(), TreeNode::Loading);
        Some(tree.generation)
    }

    /// 조회 결과를 담는다 — **폴더만** 남긴다(트리는 파일을 보여 주지 않는다).
    /// 지난 세대의 답이면 버린다.
    ///
    /// **정렬은 호출부가 맞춰 넘긴다** — 목록과 같은 규칙(`panel::file_list::compare_rows`)을
    /// 쓰려면 그쪽을 알아야 하는데, `remote`는 `panel`·`ui`를 모른다(AGENTS 계층 방향)
    pub fn fill(
        &mut self,
        conn: ConnectionId,
        generation: u64,
        path: &RemotePath,
        entries: Vec<RemoteEntry>,
    ) {
        let Some(tree) = self.conns.get_mut(&conn) else {
            return;
        };
        if tree.generation != generation {
            return;
        }
        let dirs: Vec<RemoteEntry> = entries
            .into_iter()
            // `..`는 트리에서 위로 가는 길이 아니다 — 부모는 이미 위에 그려져 있다
            .filter(|entry| entry.is_dir && entry.name != "..")
            .collect();
        tree.nodes.insert(path.clone(), TreeNode::Loaded(dirs));
    }

    /// 조회가 실패했음을 담는다 (plan Edge Case — 그 노드만 오류, 트리는 유지)
    pub fn fail(&mut self, conn: ConnectionId, generation: u64, path: &RemotePath, detail: String) {
        let Some(tree) = self.conns.get_mut(&conn) else {
            return;
        };
        if tree.generation != generation {
            return;
        }
        tree.nodes.insert(path.clone(), TreeNode::Failed(detail));
    }

    /// 그 폴더의 상태 — 아직 펼친 적이 없으면 `None`
    pub fn node(&self, conn: ConnectionId, path: &RemotePath) -> Option<&TreeNode> {
        self.conns.get(&conn)?.nodes.get(path)
    }

    /// 그 연결이 읽어 둔 것을 통째로 버린다 (Acceptance ④ — 연결이 끊기면).
    ///
    /// 세대는 함께 올린다 — 버린 뒤에 도착한 지난 답이 빈 캐시를 다시 채우면
    /// "끊긴 연결의 트리"가 남는다
    pub fn forget(&mut self, conn: ConnectionId) {
        if let Some(tree) = self.conns.get_mut(&conn) {
            tree.generation += 1;
            tree.nodes.clear();
        }
    }

    /// 다시 읽게 만든다 — 담아 둔 것을 버리고 세대를 올린다.
    /// 다음에 펼치는 폴더부터 서버에 새로 묻는다
    pub fn refresh(&mut self, conn: ConnectionId) {
        self.forget(conn);
    }

    /// 그 연결이 담고 있는 폴더 수 — 시험과 진단용
    pub fn len(&self, conn: ConnectionId) -> usize {
        self.conns
            .get(&conn)
            .map(|tree| tree.nodes.len())
            .unwrap_or(0)
    }

    pub fn is_empty(&self, conn: ConnectionId) -> bool {
        self.len(conn) == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(name: &str, is_dir: bool) -> RemoteEntry {
        RemoteEntry {
            name: name.to_owned(),
            is_dir,
            is_symlink: false,
            link_target: None,
            size: 0,
            modified: None,
            mode: None,
            owner: None,
        }
    }

    #[test]
    fn 한_번_읽은_폴더는_다시_묻지_않는다() {
        // Acceptance ② — 두 번째 펼침은 캐시를 쓴다
        let mut cache = TreeCache::new();
        let conn = ConnectionId(1);
        let path = RemotePath::new("/var");
        assert_eq!(cache.begin(conn, &path), Some(0), "첫 펼침은 조회한다");
        assert_eq!(
            cache.begin(conn, &path),
            None,
            "답을 기다리는 중에 또 물었다"
        );
        cache.fill(conn, 0, &path, vec![entry("www", true)]);
        assert_eq!(cache.begin(conn, &path), None, "이미 읽은 것을 또 물었다");
    }

    #[test]
    fn 파일과_상위_이동은_담지_않는다() {
        let mut cache = TreeCache::new();
        let conn = ConnectionId(1);
        let path = RemotePath::new("/");
        cache.begin(conn, &path);
        cache.fill(
            conn,
            0,
            &path,
            vec![
                entry("폴더2", true),
                entry("읽기.txt", false),
                entry("폴더10", true),
                entry("..", true),
            ],
        );
        let Some(TreeNode::Loaded(dirs)) = cache.node(conn, &path) else {
            panic!("담기지 않았다");
        };
        let names: Vec<&str> = dirs.iter().map(|entry| entry.name.as_str()).collect();
        // 파일과 `..`는 빠지고, 넘어온 차례는 그대로다(정렬은 호출부가 맞춰 넘긴다)
        assert_eq!(names, vec!["폴더2", "폴더10"]);
    }

    #[test]
    fn 연결이_끊기면_그_연결의_것만_비운다() {
        // Acceptance ④ — 다른 연결의 트리는 그대로 남는다
        let mut cache = TreeCache::new();
        let (gone, alive) = (ConnectionId(1), ConnectionId(2));
        let path = RemotePath::new("/pub");
        for conn in [gone, alive] {
            cache.begin(conn, &path);
            cache.fill(conn, 0, &path, vec![entry("안쪽", true)]);
        }
        cache.forget(gone);
        assert!(cache.is_empty(gone));
        assert_eq!(cache.len(alive), 1);
    }

    #[test]
    fn 버린_뒤에_온_지난_답은_받지_않는다() {
        // 세대 무효화 — 끊긴 뒤 도착한 답이 빈 캐시를 다시 채우면 유령 트리가 남는다
        let mut cache = TreeCache::new();
        let conn = ConnectionId(1);
        let path = RemotePath::new("/pub");
        let generation = cache.begin(conn, &path).expect("첫 펼침");
        cache.forget(conn);
        cache.fill(conn, generation, &path, vec![entry("안쪽", true)]);
        assert!(cache.is_empty(conn), "지난 세대의 답이 담겼다");
        cache.fail(conn, generation, &path, "550".to_owned());
        assert!(cache.is_empty(conn), "지난 세대의 실패가 담겼다");
    }

    #[test]
    fn 읽지_못한_폴더는_사유를_남긴다() {
        // plan Edge Case — 그 노드만 오류를 보이고 트리는 유지된다
        let mut cache = TreeCache::new();
        let conn = ConnectionId(1);
        let path = RemotePath::new("/root");
        let generation = cache.begin(conn, &path).expect("첫 펼침");
        cache.fail(conn, generation, &path, "550 Permission denied".to_owned());
        assert_eq!(
            cache.node(conn, &path),
            Some(&TreeNode::Failed("550 Permission denied".to_owned()))
        );
        // 실패도 "읽은 것"이라 다시 묻지 않는다 — 다시 보려면 새로 고침이다
        assert_eq!(cache.begin(conn, &path), None);
    }
}
