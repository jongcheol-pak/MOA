//! 폴더 트리 — 계층 표시와 지연 확장 (FR-9).
//!
//! 드라이브 루트만 미리 나열하고, 노드를 **처음 펼칠 때** 그 폴더의 하위 1단계만
//! 워커 스레드로 열거한다 (part2 D5 — UI 무정지). egui `CollapsingHeader`의 본문 클로저는
//! 접힌 상태에서 호출되지 않으므로, 그 클로저가 곧 "처음 펼쳐진 순간"의 신호가 된다.
//!
//! 목록과의 동기화는 단방향이다 — 트리에서 폴더를 고르면 목록이 이동하지만,
//! 목록이 이동해도 트리를 펼치지 않는다 (D6, 현행 Win32 판과 동일).
//!
//! **로컬과 원격이 같은 트리를 쓴다**(T24) — 다른 것은 "무엇을 뿌리로 삼고 하위를 어디서
//! 읽는가"뿐이라, 화면 코드를 두 벌로 두면 들여쓰기·선택 강조·`읽는 중…` 같은 것이 곧
//! 어긋난다. 로컬은 이 모듈이 워커 스레드로 직접 읽고, 원격은 **읽어 달라는 요청을 값으로
//! 올려보낸다**(연결을 아는 것은 앱이다 — `remote::tree_cache`가 받아 둔다).
use crate::fs::enumerate::{EnumOutcome, FileEntry, enumerate_dir};
use crate::panel::file_list::{SortKey, compare_entries};
use crate::remote::connection::ConnectionId;
use crate::remote::tree_cache::{TreeCache, TreeNode};
use crate::remote::types::RemotePath;
use crate::ui::theme;
use eframe::egui;
use eframe::egui::collapsing_header::CollapsingState;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::mpsc::{Receiver, Sender, TryRecvError, channel};
use windows::Win32::Storage::FileSystem::GetLogicalDrives;

/// 트리 고정 폭 — 현행 Win32 판(`panel::folder_tree::TREE_WIDTH`)과 같은 값.
/// 폭 조절은 요구에 없다(FR-9는 표시 토글까지)
pub const TREE_WIDTH: f32 = 200.0;

/// 이 트리가 보여 주는 것 (T24).
///
/// 원격일 때 하위 목록은 **앱이 가진 캐시**에서 읽는다 — 트리는 연결을 모르고, 캐시를
/// 채우는 것도 앱이다(트리는 "이 폴더를 읽어 달라"고 청하기만 한다)
pub enum TreeSource<'a> {
    Local,
    Remote {
        conn: ConnectionId,
        /// 트리의 뿌리 — 지금 보고 있는 원격 경로를 부모로 거슬러 올라간 최상단이다.
        /// 루트가 `/`가 아닌 서버도 있어 `/`로 못 박지 않는다 (plan Edge Case)
        root: RemotePath,
        cache: &'a TreeCache,
    },
}

/// 트리에서 고른 폴더 — 이동은 호출부(패널)가 한다
#[derive(Debug, Clone, PartialEq)]
pub enum TreeChoice {
    Local(PathBuf),
    Remote(RemotePath),
}

/// 하위를 읽어 달라는 청 (plan 신규 심볼 `TreeRequest`).
///
/// 로컬도 값으로 올려보낸다 — 실행은 패널이 곧바로 `start_local_load`로 되돌려주지만,
/// "펼침 → 요청"이라는 흐름이 두 소스에서 같은 모양이라야 한쪽만 고쳐지는 일이 없다
#[derive(Debug, Clone, PartialEq)]
pub enum TreeRequest {
    Local(PathBuf),
    Remote {
        conn: ConnectionId,
        path: RemotePath,
    },
}

/// 이번 프레임에 트리가 낸 것
#[derive(Debug, Default)]
pub struct TreeOutcome {
    pub chosen: Option<TreeChoice>,
    /// 이번 프레임에 처음 펼쳐진 폴더들 — 보통 0~1개다
    pub requests: Vec<TreeRequest>,
}

/// 노드의 하위 폴더 상태. 열거 실패(접근 거부·삭제)도 빈 `Loaded`로 수렴한다 —
/// 트리에서 사용자에게 알릴 것이 없고, 화살표만 사라지면 충분하다
enum Node {
    /// 워커가 열거 중
    Loading,
    /// 하위 폴더 목록 (이름 자연 정렬).
    /// `Rc`인 이유: 그리기 중 자식을 순회하면서 자기 자신을 재귀 호출해야 해
    /// 매 프레임 목록을 복제하는 대신 참조만 늘린다
    Loaded(Rc<Vec<PathBuf>>),
}

/// 패널 하나가 소유하는 폴더 트리.
///
/// 확장 상태(열림/닫힘)는 egui가 위젯 id로 보관하고, 이 구조체는 **무엇을 읽었는지**만 갖는다
pub struct FolderTreeView {
    roots: Rc<Vec<PathBuf>>,
    /// 펼친 적이 있는 폴더의 하위 목록. 한 번 읽으면 다시 읽지 않는다
    nodes: HashMap<PathBuf, Node>,
    /// 트리에서 마지막으로 고른 폴더 — 강조 표시용. 원격 트리도 이 자리를 쓴다
    selected: Option<TreeChoice>,
    tx: Sender<(PathBuf, Vec<PathBuf>)>,
    rx: Receiver<(PathBuf, Vec<PathBuf>)>,
}

impl Default for FolderTreeView {
    fn default() -> FolderTreeView {
        FolderTreeView::new()
    }
}

impl FolderTreeView {
    pub fn new() -> FolderTreeView {
        let (tx, rx) = channel();
        FolderTreeView {
            roots: Rc::new(drive_roots()),
            nodes: HashMap::new(),
            selected: None,
            tx,
            rx,
        }
    }

    /// 트리를 그리고, 고른 폴더와 새로 펼쳐진 폴더의 조회 요청을 돌려준다.
    /// 이동도 조회도 호출부의 몫이다 — 트리는 목록도 연결도 모른다
    pub fn show(&mut self, ui: &mut egui::Ui, source: TreeSource<'_>) -> TreeOutcome {
        self.poll();
        let mut outcome = TreeOutcome::default();
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| match source {
                TreeSource::Local => {
                    let roots = Rc::clone(&self.roots);
                    for root in roots.iter() {
                        self.show_node(ui, root, &mut outcome);
                    }
                }
                TreeSource::Remote { conn, root, cache } => {
                    self.show_remote_node(ui, conn, &root, 0, cache, &mut outcome);
                }
            });
        outcome
    }

    /// 원격 노드 하나와 (펼쳐져 있으면) 그 하위를 그린다.
    ///
    /// 하위 목록은 캐시에서만 읽는다 — 없으면 요청을 올려보내고 이번 프레임에는 `읽는 중…`을
    /// 보인다. 조회가 도는 동안에도 이 트리는 계속 그려지므로 목록·다른 패널이 멈추지 않는다
    /// (Acceptance ③)
    fn show_remote_node(
        &mut self,
        ui: &mut egui::Ui,
        conn: ConnectionId,
        path: &RemotePath,
        depth: usize,
        cache: &TreeCache,
        outcome: &mut TreeOutcome,
    ) {
        let choice = TreeChoice::Remote(path.clone());
        let is_selected = self.selected.as_ref() == Some(&choice);
        let label = remote_display_name(path);

        // 하위 폴더가 없다고 확인된 노드, 그리고 **상한까지 내려온 노드**는 펼침 화살표를
        // 그리지 않는다 — 후자는 순환 링크를 끊는 자리다 (plan Edge Case)
        if !can_expand(depth)
            || matches!(cache.node(conn, path), Some(TreeNode::Loaded(children)) if children.is_empty())
        {
            ui.horizontal(|ui| {
                ui.add_space(ui.spacing().indent);
                if ui.selectable_label(is_selected, label).clicked() {
                    self.select(choice, outcome);
                }
            });
            return;
        }

        let id = ui.make_persistent_id(path.as_str());
        // 뿌리는 펼친 채로 시작한다 — 원격 트리를 켜자마자 접힌 줄 하나만 보이면
        // 한 번 더 눌러야 무엇이 있는지 알 수 있다(로컬은 드라이브가 여럿이라 접어 둔다)
        let header = CollapsingState::load_with_default_open(ui.ctx(), id, depth == 0).show_header(
            ui,
            |ui| {
                if ui.selectable_label(is_selected, label).clicked() {
                    self.select(choice, outcome);
                }
            },
        );
        header.body(|ui| {
            match cache.node(conn, path) {
                Some(TreeNode::Loaded(children)) => {
                    // 자식 경로를 먼저 뜬다 — 아래 재귀가 `cache`를 다시 빌린다
                    let paths: Vec<RemotePath> = children
                        .iter()
                        .map(|entry| path.join(&entry.name))
                        .collect();
                    for child in paths {
                        self.show_remote_node(ui, conn, &child, depth + 1, cache, outcome);
                    }
                }
                Some(TreeNode::Failed(detail)) => {
                    // 그 노드만 사유를 보이고 트리는 그대로 둔다 (plan Edge Case).
                    // 사유는 이미 완결된 문장이라 앞에 말을 덧붙이지 않는다(quality 리뷰 m1)
                    ui.colored_label(theme::ERROR_TEXT, detail);
                }
                Some(TreeNode::Loading) => {
                    ui.colored_label(theme::TEXT_DIM, LOADING);
                }
                None => {
                    ui.colored_label(theme::TEXT_DIM, LOADING);
                    if needs_children(cache, conn, path) {
                        outcome.requests.push(TreeRequest::Remote {
                            conn,
                            path: path.clone(),
                        });
                    }
                }
            }
        });
    }

    /// 하위 1단계를 워커 스레드로 읽는다 — 패널이 `TreeRequest::Local`을 받아 되돌려준다
    pub fn start_local_load(&mut self, path: PathBuf, ctx: &egui::Context) {
        self.start_load(path, ctx);
    }

    /// 워커가 끝낸 열거 결과를 모두 반영한다
    fn poll(&mut self) {
        loop {
            match self.rx.try_recv() {
                Ok((path, children)) => {
                    self.nodes.insert(path, Node::Loaded(Rc::new(children)));
                }
                Err(TryRecvError::Empty | TryRecvError::Disconnected) => return,
            }
        }
    }

    /// 노드 하나와 (펼쳐져 있으면) 그 하위를 그린다
    fn show_node(&mut self, ui: &mut egui::Ui, path: &Path, outcome: &mut TreeOutcome) {
        let label = display_name(path);
        let choice = TreeChoice::Local(path.to_path_buf());
        let is_selected = self.selected.as_ref() == Some(&choice);

        // 하위 폴더가 없다고 확인된 노드는 펼침 화살표를 그리지 않는다
        // (현행 Win32 판의 `set_no_children`과 같은 동작). 화살표 자리만큼 들여쓴다
        if matches!(self.nodes.get(path), Some(Node::Loaded(children)) if children.is_empty()) {
            ui.horizontal(|ui| {
                ui.add_space(ui.spacing().indent);
                if ui.selectable_label(is_selected, label).clicked() {
                    self.select(choice, outcome);
                }
            });
            return;
        }

        let id = ui.make_persistent_id(path);
        let header =
            CollapsingState::load_with_default_open(ui.ctx(), id, false).show_header(ui, |ui| {
                if ui.selectable_label(is_selected, label).clicked() {
                    self.select(choice, outcome);
                }
            });
        header.body(|ui| {
            // 이 클로저는 펼쳐졌을 때만 호출된다 — 여기가 지연 열거의 시작점이다
            let started = self.nodes.contains_key(path);
            let children = match self.nodes.get(path) {
                Some(Node::Loaded(children)) => Some(Rc::clone(children)),
                _ => None,
            };
            if !started {
                outcome
                    .requests
                    .push(TreeRequest::Local(path.to_path_buf()));
            }
            match children {
                Some(children) => {
                    for child in children.iter() {
                        self.show_node(ui, child, outcome);
                    }
                }
                None => {
                    ui.colored_label(theme::TEXT_DIM, LOADING);
                }
            }
        });
    }

    fn select(&mut self, choice: TreeChoice, outcome: &mut TreeOutcome) {
        self.selected = Some(choice.clone());
        outcome.chosen = Some(choice);
    }

    /// 하위 1단계를 워커 스레드로 읽는다.
    /// UI 스레드에서 직접 읽으면 항목이 많은 폴더나 응답이 느린 네트워크 드라이브에서 창이 멈춘다
    fn start_load(&mut self, path: PathBuf, ctx: &egui::Context) {
        self.nodes.insert(path.clone(), Node::Loading);
        let tx = self.tx.clone();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let children = match enumerate_dir(&path) {
                EnumOutcome::Ok(entries) => child_dirs(&path, entries),
                // 접근 거부·삭제·오류 — 하위 없음으로 다룬다
                _ => Vec::new(),
            };
            // 수신부가 이미 사라졌으면(앱 종료) 전송 실패는 무해하다
            let _ = tx.send((path, children));
            ctx.request_repaint();
        });
    }
}

/// 아직 읽는 중인 노드의 자리 표시 — 로컬·원격이 같은 문구를 쓴다
const LOADING: &str = "읽는 중…";

/// 원격 트리가 내려갈 수 있는 최대 깊이.
///
/// **끊지 않으면 순환 심볼릭 링크에서 끝없이 깊어진다**(plan Edge Case) — 서버 쪽 링크는
/// 목록만으로는 순환인지 알 수 없어(가리키는 곳을 따라가 봐야 안다) 깊이로 끊는다.
/// 훑기(`ConnCommand::ListTree`)가 쓰는 상한과 같은 자리의 판단이다
const MAX_TREE_DEPTH: usize = 40;

/// 이 깊이에서 더 내려가도 되는가 (plan Edge Case — 심볼릭 링크 순환)
fn can_expand(depth: usize) -> bool {
    depth < MAX_TREE_DEPTH
}

/// 이 폴더의 하위를 서버에 청해야 하는가 — 캐시가 아무것도 모를 때만이다.
///
/// 그리기에서 떼어 둔 이유는 **판정만 따로 시험하기 위해서**다 (egui의 접힘 상태는
/// 위젯 id에 묶여 있어 시험이 원하는 노드를 펼쳐 둘 수 없다)
fn needs_children(cache: &TreeCache, conn: ConnectionId, path: &RemotePath) -> bool {
    cache.node(conn, path).is_none()
}

/// 원격 노드에 보일 이름 — 뿌리(`/`)는 이름이 없어 경로 자체를 쓴다
fn remote_display_name(path: &RemotePath) -> String {
    path.file_name()
        .map(str::to_owned)
        .unwrap_or_else(|| path.as_str().to_owned())
}

/// 드라이브 루트 (`C:\`, `D:\` …).
/// 비트마스크의 비트 순서가 곧 알파벳 순이라 따로 정렬하지 않는다
pub fn drive_roots() -> Vec<PathBuf> {
    // 안전성: 인자 없는 조회 — 현재 드라이브 비트마스크만 반환한다
    let mask = unsafe { GetLogicalDrives() };
    (0..26u32)
        .filter(|i| mask & (1 << i) != 0)
        .map(|i| PathBuf::from(format!("{}:\\", (b'A' + i as u8) as char)))
        .collect()
}

/// 열거 결과에서 하위 폴더만 골라 이름 자연 정렬로 돌려준다.
/// 정렬은 목록과 같은 규칙을 쓴다 — 종류 인자는 이름 정렬에서 쓰이지 않아 빈 문자열을 넘긴다
fn child_dirs(parent: &Path, mut entries: Vec<FileEntry>) -> Vec<PathBuf> {
    entries.retain(|e| e.is_dir);
    entries.sort_by(|a, b| compare_entries(a, "", b, "", SortKey::Name));
    entries
        .iter()
        .map(|e| parent.join(e.name_string()))
        .collect()
}

/// 트리에 보일 이름 — 드라이브 루트는 이름이 없어 경로 자체(`C:\`)를 쓴다
fn display_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_string_lossy().into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::remote::types::RemoteEntry;

    fn entry(name: &str, is_dir: bool) -> FileEntry {
        let mut n: Vec<u16> = name.encode_utf16().collect();
        n.push(0);
        FileEntry {
            name: n,
            is_dir,
            size: 0,
            modified: 0,
        }
    }

    #[test]
    fn 하위_폴더만_남기고_파일은_거른다() {
        let entries = vec![
            entry("문서.txt", false),
            entry("사진", true),
            entry("설치.exe", false),
        ];
        let children = child_dirs(Path::new(r"C:\Users"), entries);
        assert_eq!(children, vec![PathBuf::from(r"C:\Users\사진")]);
    }

    #[test]
    fn 하위_폴더는_자연_정렬된다() {
        // "폴더10"이 "폴더2"보다 뒤에 와야 한다 (사전식이면 앞에 온다)
        let entries = vec![entry("폴더10", true), entry("폴더2", true)];
        let children = child_dirs(Path::new(r"D:\"), entries);
        assert_eq!(
            children,
            vec![PathBuf::from(r"D:\폴더2"), PathBuf::from(r"D:\폴더10")]
        );
    }

    #[test]
    fn 표시_이름은_폴더명이고_드라이브는_경로다() {
        assert_eq!(display_name(Path::new(r"C:\Users\jongc")), "jongc");
        assert_eq!(display_name(Path::new(r"C:\")), r"C:\");
    }

    #[test]
    fn 드라이브_루트는_루트_경로_형태다() {
        // 실제 구성은 PC마다 다르므로 형태만 검증한다 (C: 드라이브는 항상 있다)
        let roots = drive_roots();
        assert!(roots.iter().any(|r| r == Path::new(r"C:\")));
        assert!(roots.iter().all(|r| r.parent().is_none()));
    }

    /// 원격 트리를 한 프레임 그리고 결과를 돌려준다
    fn draw_remote(cache: &TreeCache, root: &str) -> TreeOutcome {
        let ctx = egui::Context::default();
        let mut view = FolderTreeView::new();
        let mut outcome = TreeOutcome::default();
        let _ = ctx.run_ui(Default::default(), |ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                outcome = view.show(
                    ui,
                    TreeSource::Remote {
                        conn: ConnectionId(1),
                        root: RemotePath::new(root),
                        cache,
                    },
                );
            });
        });
        outcome
    }

    #[test]
    fn 처음_펼친_원격_폴더만_조회를_청한다() {
        // Acceptance ②의 화면 절반 — 캐시가 답을 들고 있으면 청하지 않는다.
        // (같은 폴더를 두 번 펼쳐도 서버에 한 번만 가는 것은 `TreeCache::begin`이 막는다)
        let mut cache = TreeCache::new();
        let conn = ConnectionId(1);
        let root = RemotePath::new("/");

        // 뿌리는 펼친 채로 시작하므로 첫 프레임에 곧바로 하위를 청한다
        let first = draw_remote(&cache, "/");
        assert_eq!(
            first.requests,
            vec![TreeRequest::Remote {
                conn,
                path: root.clone()
            }],
            "펼쳐진 뿌리가 하위를 청하지 않았다"
        );

        // 답이 담기면 화면은 그것을 그린다 — 이때는 새 요청이 없다
        cache.begin(conn, &root);
        cache.fill(
            conn,
            0,
            &root,
            vec![RemoteEntry {
                name: "var".to_owned(),
                is_dir: true,
                is_symlink: false,
                link_target: None,
                size: 0,
                modified: None,
                mode: None,
                owner: None,
            }],
        );
        let loaded = draw_remote(&cache, "/");
        assert!(loaded.requests.is_empty(), "이미 읽은 것을 또 청했다");
        assert!(loaded.chosen.is_none());
    }

    #[test]
    fn 원격_노드_이름은_마지막_조각이고_뿌리는_경로다() {
        assert_eq!(remote_display_name(&RemotePath::new("/var/www")), "www");
        assert_eq!(remote_display_name(&RemotePath::new("/")), "/");
    }

    #[test]
    fn 형제_노드가_함께_펼쳐지면_각자_조회를_청한다() {
        // quality 리뷰 M1 — 한 프레임에 여럿이 펼쳐지면 요청도 여럿이다. 하나로 압착하면
        // 나머지가 그 프레임에서 버려져 노드마다 프레임이 하나씩 밀린다.
        // (egui의 접힘 상태는 위젯 id에 묶여 있어 시험이 임의 노드를 펼칠 수 없다 —
        //  그래서 그리기에서 떼어 둔 판정을 직접 확인한다)
        let mut cache = TreeCache::new();
        let conn = ConnectionId(1);
        let siblings = [RemotePath::new("/var"), RemotePath::new("/etc")];
        for path in &siblings {
            assert!(needs_children(&cache, conn, path), "{path:?}");
        }
        // 청하기 시작한 것은 다시 청하지 않는다
        cache.begin(conn, &siblings[0]);
        assert!(!needs_children(&cache, conn, &siblings[0]));
        assert!(
            needs_children(&cache, conn, &siblings[1]),
            "형제가 함께 막혔다"
        );
    }

    #[test]
    fn 상한까지_내려가면_더_펼치지_않는다() {
        // plan Edge Case — 순환 심볼릭 링크는 목록만으로 알 수 없어 깊이로 끊는다
        assert!(can_expand(0));
        assert!(can_expand(MAX_TREE_DEPTH - 1));
        assert!(!can_expand(MAX_TREE_DEPTH));
        assert!(!can_expand(MAX_TREE_DEPTH + 1));
    }
}
