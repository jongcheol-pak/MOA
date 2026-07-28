//! 폴더 트리 — 계층 표시와 지연 확장 (FR-9).
//!
//! 드라이브 루트만 미리 나열하고, 노드를 **처음 펼칠 때** 그 폴더의 하위 1단계만
//! 워커 스레드로 열거한다 (part2 D5 — UI 무정지). egui `CollapsingHeader`의 본문 클로저는
//! 접힌 상태에서 호출되지 않으므로, 그 클로저가 곧 "처음 펼쳐진 순간"의 신호가 된다.
//!
//! 목록과의 동기화는 단방향이다 — 트리에서 폴더를 고르면 목록이 이동하지만,
//! 목록이 이동해도 트리를 펼치지 않는다 (D6, 현행 Win32 판과 동일).
use crate::fs::enumerate::{EnumOutcome, FileEntry, enumerate_dir};
use crate::panel::file_list::{SortKey, compare_entries};
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
    /// 트리에서 마지막으로 고른 폴더 — 강조 표시용
    selected: Option<PathBuf>,
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

    /// 트리를 그리고, 이번 프레임에 고른 폴더가 있으면 돌려준다.
    /// 이동은 호출부(패널)의 몫이다 — 트리는 목록을 모른다
    pub fn show(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) -> Option<PathBuf> {
        self.poll();
        let mut chosen = None;
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let roots = Rc::clone(&self.roots);
                for root in roots.iter() {
                    self.show_node(ui, ctx, root, &mut chosen);
                }
            });
        chosen
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
    fn show_node(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        path: &Path,
        chosen: &mut Option<PathBuf>,
    ) {
        let label = display_name(path);
        let is_selected = self.selected.as_deref() == Some(path);

        // 하위 폴더가 없다고 확인된 노드는 펼침 화살표를 그리지 않는다
        // (현행 Win32 판의 `set_no_children`과 같은 동작). 화살표 자리만큼 들여쓴다
        if matches!(self.nodes.get(path), Some(Node::Loaded(children)) if children.is_empty()) {
            ui.horizontal(|ui| {
                ui.add_space(ui.spacing().indent);
                if ui.selectable_label(is_selected, label).clicked() {
                    self.select(path, chosen);
                }
            });
            return;
        }

        let id = ui.make_persistent_id(path);
        let header =
            CollapsingState::load_with_default_open(ui.ctx(), id, false).show_header(ui, |ui| {
                if ui.selectable_label(is_selected, label).clicked() {
                    self.select(path, chosen);
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
                self.start_load(path.to_path_buf(), ctx);
            }
            match children {
                Some(children) => {
                    for child in children.iter() {
                        self.show_node(ui, ctx, child, chosen);
                    }
                }
                None => {
                    ui.colored_label(theme::TEXT_DIM, "읽는 중…");
                }
            }
        });
    }

    fn select(&mut self, path: &Path, chosen: &mut Option<PathBuf>) {
        self.selected = Some(path.to_path_buf());
        *chosen = Some(path.to_path_buf());
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
}
