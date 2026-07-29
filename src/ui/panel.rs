//! 패널 — 탭 스트립 / 주소창 / 파일 목록을 담는 탐색 단위 (FR-3).
//!
//! 패널은 자기 탐색 상태(탭·히스토리·목록·열거)를 온전히 소유하며 서로를 모른다.
//! 아이콘 캐시·셸 호스트처럼 앱 전역에서 하나면 충분한 것은 `show`가 인자로 받는다.
//!
//! 탐색은 **pending-커밋** 모델이다: 열거가 성공했을 때만 경로·히스토리를 커밋한다.
//! 실패(삭제·권한)하면 사유만 표시하고 현 위치·목록을 그대로 둔다.
use crate::fs::create;
use crate::fs::enumerate::{EnumOutcome, enumerate_dir};
use crate::fs::icons::IconCache;
use crate::fs::watcher::DirWatcher;
use crate::panel::tabs::{CloseOutcome, TabState, TabsModel};
use crate::ui::address_bar::{AddressBar, NavAction};
use crate::ui::file_list::{FileListAction, FileListView};
use crate::ui::icon_tex::IconTextures;
use crate::ui::menu::{Command, PanelMenuState};
use crate::ui::shell_host;
use crate::ui::tabs::TabAction;
use crate::ui::theme;
use crate::ui::tree::{FolderTreeView, TREE_WIDTH};
use eframe::egui;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, TryRecvError, channel};

/// 트리와 목록을 가르는 세로 선 두께 — 현행 판 트리의 테두리(`WS_EX_CLIENTEDGE`)를 대신한다
const TREE_BORDER: f32 = 1.0;

/// 트리 영역 안쪽 여백 — 항목이 패널 가장자리에 붙지 않게 한다
const TREE_PAD: f32 = 4.0;

/// 백그라운드 폴더 열거 상태.
///
/// 기존 `fs::enumerate::spawn_enumerate`는 완료를 `PostMessageW`로 **HWND에 통지**해
/// egui에서는 쓸 수 없다. 동기 `enumerate_dir`을 자체 워커로 감싸고 채널로 받는다.
/// UI 스레드에서 직접 열거하면 10만 파일 폴더에서 창이 멈춘다(AGENTS: UI 스레드 블로킹 금지)
struct DirLoad {
    /// 늦게 도착한 이전 폴더의 결과를 버리기 위한 세대 번호
    generation: u64,
    pending: Option<Receiver<(u64, EnumOutcome)>>,
}

impl DirLoad {
    fn new() -> DirLoad {
        DirLoad {
            generation: 0,
            pending: None,
        }
    }

    /// 워커 스레드에서 열거를 시작한다. 이전 요청의 결과는 세대 불일치로 폐기된다
    fn start(&mut self, path: PathBuf, ctx: &egui::Context) {
        self.generation += 1;
        let generation = self.generation;
        let (tx, rx) = channel();
        self.pending = Some(rx);
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let outcome = enumerate_dir(&path);
            // 수신부가 이미 버려졌으면(앱 종료·폴더 재이동) 전송 실패는 무해하다
            let _ = tx.send((generation, outcome));
            ctx.request_repaint();
        });
    }

    /// 완료된 결과를 꺼낸다. 아직이면 `None`
    fn poll(&mut self) -> Option<EnumOutcome> {
        let rx = self.pending.as_ref()?;
        match rx.try_recv() {
            Ok((generation, outcome)) => {
                self.pending = None;
                // 폴더를 연달아 이동하면 이전 결과가 나중에 도착할 수 있다
                (generation == self.generation).then_some(outcome)
            }
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => {
                self.pending = None;
                None
            }
        }
    }

    fn is_loading(&self) -> bool {
        self.pending.is_some()
    }
}

/// 새 폴더·새 파일 생성 상태 (FR-25).
///
/// 생성도 열거와 같이 **워커 스레드에서** 한다 — `CreateDirectoryW`·`CreateFileW`는 로컬
/// 디스크에서는 순식간이지만 네트워크 드라이브에서는 수 초가 걸릴 수 있고, 이름이 겹치면
/// 그만큼 재시도가 이어진다. UI 스레드에서 부르면 그동안 창이 멈춘다
/// (AGENTS: UI 스레드 블로킹 I/O 금지 — `DirLoad`와 같은 규칙)
struct CreateOp {
    /// (무엇을 만들었는지, 결과) — 실패 문구에 종류를 넣기 위해 함께 보낸다
    pending: Option<Receiver<(&'static str, std::io::Result<PathBuf>)>>,
}

impl CreateOp {
    fn new() -> CreateOp {
        CreateOp { pending: None }
    }

    /// 워커에서 생성을 시작한다. 이미 진행 중이면 무시한다 —
    /// 메뉴를 연달아 눌러도 한 번에 하나만 만든다
    fn start(
        &mut self,
        dir: PathBuf,
        kind: &'static str,
        make: fn(&Path) -> std::io::Result<PathBuf>,
        ctx: &egui::Context,
    ) {
        if self.pending.is_some() {
            return;
        }
        let (tx, rx) = channel();
        self.pending = Some(rx);
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            // 수신부가 이미 버려졌으면(패널 닫힘·앱 종료) 전송 실패는 무해하다
            let _ = tx.send((kind, make(&dir)));
            ctx.request_repaint();
        });
    }

    /// 완료된 결과를 꺼낸다. 아직이면 `None`
    fn poll(&mut self) -> Option<(&'static str, std::io::Result<PathBuf>)> {
        let rx = self.pending.as_ref()?;
        match rx.try_recv() {
            Ok(done) => {
                self.pending = None;
                Some(done)
            }
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => {
                self.pending = None;
                None
            }
        }
    }
}

/// 열거 성공 시 히스토리에 적용할 동작
#[derive(Clone, Copy, PartialEq, Eq)]
enum PendingNav {
    /// 폴더만 다시 읽는다 — 첫 로드·탭 전환·새로고침
    None,
    /// 새 이동 — 커서 뒤를 자르고 추가
    Push,
    Back,
    Forward,
}

/// 패널이 상위(레이아웃)에 올려보내는 요청.
/// 둘 다 이 패널을 그리는 도중에는 실행할 수 없어 값으로 돌려준다
pub struct PanelOutcome {
    pub menu: Option<MenuRequest>,
    /// 패널 메뉴에서 고른 명령 — 대상은 **이 패널**이다 (plan D16)
    pub command: Option<Command>,
}

/// 셸 컨텍스트 메뉴 요청 — 그리기가 모두 끝난 뒤 앱이 실행한다.
/// `items`가 비면 폴더 배경 메뉴다
pub struct MenuRequest {
    pub folder: PathBuf,
    pub items: Vec<PathBuf>,
    /// egui 논리 좌표 — 화면 좌표 변환은 실행 시점에 한다
    pub pos: egui::Pos2,
}

/// 패널 하나의 탐색 상태
pub struct PanelState {
    /// 탭 목록 — 탭마다 커밋된 경로와 독립 히스토리를 갖는다 (FR-3)
    tabs: TabsModel,
    list: FileListView,
    address: AddressBar,
    load: DirLoad,
    /// 열거 중인 대상 — 성공해야 활성 탭의 경로로 커밋된다
    pending_dir: PathBuf,
    /// 열거가 성공하면 히스토리에 무엇을 할지.
    /// 커서 이동도 성공 후로 미뤄야 한다 — 먼저 옮기면 실패했을 때 화면과 히스토리가 어긋난다
    pending_nav: PendingNav,
    /// 열거 실패 사유 — 성공 시 빈 문자열
    status: String,
    /// 첫 프레임을 그린 뒤 열거를 시작하기 위한 대기 경로.
    /// 생성자에서 바로 열거하면 창이 늦게 뜬다
    deferred_start: Option<PathBuf>,
    /// 폴더 트리 — 패널마다 독립이다 (FR-9)
    tree: FolderTreeView,
    /// 트리 표시 여부. 현행 판과 같이 숨김으로 시작한다
    tree_visible: bool,
    /// 표시 중인 폴더의 변경 감시 (FR-10). 폴더가 바뀌면 통째로 교체된다
    watch: Option<DirWatch>,
    /// 진행 중인 새 폴더·새 파일 생성 (FR-25)
    create: CreateOp,
}

/// 감시자와 그 통지 채널 — 둘의 수명이 같아야 해서 함께 둔다
struct DirWatch {
    watcher: DirWatcher,
    /// 변경 통지 (내용 없음 — 받으면 폴더를 통째로 다시 읽는다)
    rx: Receiver<()>,
}

impl PanelState {
    pub fn new(start: PathBuf) -> PanelState {
        PanelState {
            tabs: TabsModel::new(TabState::new(start.clone())),
            list: FileListView::new(),
            address: AddressBar::new(),
            load: DirLoad::new(),
            pending_dir: PathBuf::new(),
            pending_nav: PendingNav::None,
            status: String::new(),
            deferred_start: Some(start),
            tree: FolderTreeView::new(),
            tree_visible: false,
            watch: None,
            create: CreateOp::new(),
        }
    }

    /// 저장된 탭 구성과 열 폭으로 패널을 되살린다 (FR-11). 탭 목록이 비면 `None`.
    ///
    /// 히스토리는 복원하지 않는다 — 세션에는 경로만 저장한다(현행과 같은 규칙).
    /// `columns`가 비면 기본 폭으로 시작한다
    pub fn from_tabs(tabs: Vec<PathBuf>, active_tab: usize, columns: &[f32]) -> Option<PanelState> {
        let states: Vec<TabState> = tabs.into_iter().map(TabState::new).collect();
        let model = TabsModel::from_tabs(states, active_tab)?;
        let start = model.active().committed.clone();
        let mut panel = PanelState::new(start);
        panel.tabs = model;
        if !columns.is_empty() {
            panel.list.set_columns(columns);
        }
        Some(panel)
    }

    /// 현재 표시 중인 폴더 — 활성 탭이 커밋한 경로가 정본이다
    pub fn dir(&self) -> &Path {
        &self.tabs.active().committed
    }

    /// 세션 저장용 — 탭들의 폴더 경로(탭 순서)와 활성 탭
    pub fn tab_paths(&self) -> Vec<PathBuf> {
        self.tabs.paths()
    }

    pub fn active_tab(&self) -> usize {
        self.tabs.active_index()
    }

    /// 세션 저장용 — 자세히 보기 열 폭 (패널마다 독립)
    pub fn columns(&self) -> Vec<f32> {
        self.list.columns()
    }

    /// 프레임마다 호출 — 지연 시작·열거 완료·변경 감시를 처리하고, 열거 중이면 다시 그리게 한다
    pub fn poll(&mut self, ctx: &egui::Context, icons: &mut IconCache) {
        if let Some(path) = self.deferred_start.take() {
            // 시작 경로는 이미 히스토리에 들어 있으므로 다시 쌓지 않는다
            self.start_load(path, PendingNav::None, ctx);
        }
        self.poll_load(icons);
        self.poll_watch(ctx);
        self.poll_create(ctx);
        if self.load.is_loading() {
            ctx.request_repaint();
        }
    }

    /// 표시 중인 폴더를 감시한다 (FR-10). 이전 폴더의 감시는 여기서 끝난다 —
    /// 교체하면 옛 `DirWatcher`가 drop되며 그 스레드가 정지·회수된다
    fn watch(&mut self, path: &Path) {
        if self
            .watch
            .as_ref()
            .is_some_and(|watch| watch.watcher.path() == path)
        {
            return;
        }
        let (tx, rx) = channel();
        // 감시는 창 메시지를 쓰지 않는다 — 채널만 받아 프레임마다 확인한다 (D7)
        let watcher = DirWatcher::start(path.to_path_buf(), tx, None);
        self.watch = Some(DirWatch { watcher, rx });
    }

    /// 감시 통지를 확인해 변경이 있으면 폴더를 다시 읽는다.
    /// 폴더가 열리지 않아 감시가 즉시 끝난 경우에도 통지 1회가 오며, 재열거가 사유를 표시한다
    fn poll_watch(&mut self, ctx: &egui::Context) {
        let Some(watch) = self.watch.as_ref() else {
            return;
        };
        // 쌓인 통지가 여러 개여도 다시 읽는 것은 한 번이면 된다
        let mut changed = false;
        while watch.rx.try_recv().is_ok() {
            changed = true;
        }
        if changed {
            let dir = self.dir().to_path_buf();
            self.start_load(dir, PendingNav::None, ctx);
            ctx.request_repaint();
        }
    }

    /// 폴더 이동 (사용자 조작) — 성공하면 히스토리에 남는다
    fn navigate(&mut self, path: PathBuf, ctx: &egui::Context) {
        self.start_load(path, PendingNav::Push, ctx);
    }

    fn start_load(&mut self, path: PathBuf, nav: PendingNav, ctx: &egui::Context) {
        self.pending_dir = path.clone();
        self.pending_nav = nav;
        self.status.clear();
        self.load.start(path, ctx);
    }

    /// 워커 결과를 목록에 반영한다
    fn poll_load(&mut self, icons: &mut IconCache) {
        let Some(outcome) = self.load.poll() else {
            return;
        };
        match outcome {
            EnumOutcome::Ok(entries) => {
                // 여기서 비로소 커밋한다 — 이 지점 전에는 화면이 이전 폴더를 유지한다
                let dir = std::mem::take(&mut self.pending_dir);
                let tab = self.tabs.active_mut();
                tab.committed = dir.clone();
                match self.pending_nav {
                    PendingNav::None => {}
                    PendingNav::Push => tab.history.push(dir.clone()),
                    PendingNav::Back => {
                        tab.history.back();
                    }
                    PendingNav::Forward => {
                        tab.history.forward();
                    }
                }
                // 감시 대상도 이 시점에 맞춘다 — 커밋된 폴더만 감시한다(열거 실패한 곳은 아니다)
                self.watch(&dir);
                self.list.set_entries(dir, entries, icons);
            }
            // 실패해도 목록·경로·히스토리를 그대로 둔다 — 사유만 알린다(pending-커밋)
            EnumOutcome::AccessDenied => {
                self.status = format!("'{}' 폴더를 열 권한이 없습니다", self.pending_name());
            }
            EnumOutcome::NotFound => {
                self.status = format!("'{}' 폴더를 찾을 수 없습니다", self.pending_name());
            }
            EnumOutcome::Error => {
                self.status = format!(
                    "'{}' 폴더를 여는 중 문제가 발생했습니다",
                    self.pending_name()
                );
            }
        }
        self.pending_nav = PendingNav::None;
    }

    /// 실패 문구에 쓸 대상 폴더 이름 — 전체 경로는 길어 끝 이름만 보여준다
    fn pending_name(&self) -> String {
        self.pending_dir
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| self.pending_dir.to_string_lossy().into_owned())
    }

    /// 메뉴·단축키에서 온 탐색 명령 (FR-12).
    /// 탭 스트립·주소창 클릭과 **같은 경로**를 타므로 동작이 갈리지 않는다
    pub fn new_tab(&mut self, ctx: &egui::Context) {
        self.handle_tab(TabAction::New, ctx);
    }

    pub fn close_tab(&mut self, ctx: &egui::Context) {
        self.handle_tab(TabAction::Close(self.tabs.active_index()), ctx);
    }

    pub fn go_back(&mut self, ctx: &egui::Context) {
        self.handle_nav(NavAction::Back, ctx);
    }

    pub fn go_forward(&mut self, ctx: &egui::Context) {
        self.handle_nav(NavAction::Forward, ctx);
    }

    pub fn go_up(&mut self, ctx: &egui::Context) {
        self.handle_nav(NavAction::Up, ctx);
    }

    /// 보고 있는 폴더를 다시 읽는다 — 경로도 히스토리도 그대로다
    pub fn refresh(&mut self, ctx: &egui::Context) {
        let dir = self.dir().to_path_buf();
        self.start_load(dir, PendingNav::None, ctx);
    }

    /// 폴더 트리 표시 토글 (FR-9) — 패널마다 독립이다
    pub fn toggle_tree(&mut self) {
        self.tree_visible = !self.tree_visible;
    }

    /// 표시 중인 폴더에 새 폴더를 만든다 (FR-25)
    pub fn new_folder(&mut self, ctx: &egui::Context) {
        let dir = self.dir().to_path_buf();
        self.create.start(dir, "폴더", create::new_folder, ctx);
    }

    /// 표시 중인 폴더에 빈 텍스트 문서를 만든다 (FR-25)
    pub fn new_file(&mut self, ctx: &egui::Context) {
        let dir = self.dir().to_path_buf();
        self.create.start(dir, "파일", create::new_text_file, ctx);
    }

    /// 생성 결과를 반영한다 — 실패하면 사유만 상태 줄에 보인다 (plan D12).
    ///
    /// 성공하면 **감시 여부와 무관하게** 폴더를 다시 읽는다. 감시(FR-10)가 살아 있으면 통지로도
    /// 갱신되지만, `DirWatcher`는 폴더 열기에 실패해도 조용히 끝나 그 실패가 밖으로 드러나지
    /// 않는다 — 감시 객체가 있다는 것만으로 통지를 믿으면, 감시가 죽은 위치에서 방금 만든
    /// 항목이 목록에 나타나지 않는다. 재열거 한 번이 그 침묵보다 싸다
    fn poll_create(&mut self, ctx: &egui::Context) {
        let Some((kind, result)) = self.create.poll() else {
            return;
        };
        match result {
            Ok(_) => {
                let dir = self.dir().to_path_buf();
                self.start_load(dir, PendingNav::None, ctx);
            }
            Err(error) => {
                self.status = format!("새 {kind}을(를) 만들지 못했습니다 — {error}");
            }
        }
    }

    /// 탭 스트립에서 올라온 조작 처리
    fn handle_tab(&mut self, action: TabAction, ctx: &egui::Context) {
        match action {
            TabAction::Switch(index) => {
                if self.tabs.switch(index) {
                    // 탭마다 폴더가 다르므로 전환하면 그 탭의 폴더를 다시 읽는다.
                    // 히스토리는 이미 그 탭의 것이라 손대지 않는다
                    let path = self.tabs.active().committed.clone();
                    self.start_load(path, PendingNav::None, ctx);
                }
            }
            TabAction::Close(index) => {
                // 보고 있던 탭을 닫을 때만 화면이 바뀐다 — 배경 탭을 닫으면 그대로 유지된다
                let was_active = index == self.tabs.active_index();
                if let CloseOutcome::Removed(_) = self.tabs.close(index)
                    && was_active
                {
                    let path = self.tabs.active().committed.clone();
                    self.start_load(path, PendingNav::None, ctx);
                }
                // LastTab이면 아무것도 하지 않는다 — 패널의 마지막 탭은 남는다
            }
            TabAction::New => {
                // 새 탭은 현재 폴더를 복제해 연다 (탐색기 관례)
                let path = self.dir().to_path_buf();
                self.tabs.add(TabState::new(path.clone()));
                self.start_load(path, PendingNav::None, ctx);
            }
        }
    }

    /// 주소창에서 올라온 탐색 요청 처리
    fn handle_nav(&mut self, action: NavAction, ctx: &egui::Context) {
        match action {
            NavAction::Back => {
                if let Some(path) = self.tabs.active().history.peek_back().map(PathBuf::from) {
                    self.start_load(path, PendingNav::Back, ctx);
                }
            }
            NavAction::Forward => {
                if let Some(path) = self.tabs.active().history.peek_forward().map(PathBuf::from) {
                    self.start_load(path, PendingNav::Forward, ctx);
                }
            }
            NavAction::Up => {
                if let Some(parent) = self.dir().parent().map(PathBuf::from) {
                    self.navigate(parent, ctx);
                }
            }
            NavAction::Goto(path) => self.navigate(path, ctx),
        }
    }

    /// 목록에서 올라온 조작 처리. 셸 메뉴 요청은 **실행하지 않고 값으로 돌려준다**.
    ///
    /// `TrackPopupMenuEx`는 자체 메시지 루프를 돌려 winit 이벤트 루프를 재진입시킨다 —
    /// egui 위젯 트리가 절반만 구성된 상태에서 그 루프에 들어가면 안 되므로,
    /// 그리기 클로저를 전부 빠져나온 뒤에 띄워야 한다(`ExplorerApp::ui`가 처리)
    fn handle_list_action(
        &mut self,
        action: FileListAction,
        ctx: &egui::Context,
    ) -> Option<MenuRequest> {
        match action {
            FileListAction::None => None,
            FileListAction::Open(index) => {
                let entry = self.list.entry_at(index)?;
                let target = self.dir().join(entry.name_string());
                if entry.is_dir {
                    self.navigate(target, ctx);
                } else {
                    shell_host::execute(&target);
                }
                None
            }
            FileListAction::Context { index, pos } => {
                // 행 메뉴는 선택 전체가 대상이다 — 여러 항목을 골라 한 번에 복사·삭제할 수 있다.
                // 빈 영역이면 항목 없이 요청해 폴더 배경 메뉴("새로 만들기")를 띄운다
                let items = if index.is_some() {
                    self.list.selected_paths()
                } else {
                    Vec::new()
                };
                Some(MenuRequest {
                    folder: self.dir().to_path_buf(),
                    items,
                    pos,
                })
            }
        }
    }

    /// 패널 하나를 그리고 이번 프레임의 조작을 처리한다.
    /// 세로 구성은 탭 스트립 / 주소창 / (폴더 트리 | 상태 · 파일 목록)이며,
    /// 트리를 숨기면 목록이 그 폭까지 차지한다 (FR-9).
    ///
    /// 셸 메뉴 요청과 분할 요청은 실행하지 않고 반환한다 — 셸 메뉴는 모달이라 그리기가 끝난 뒤에
    /// 띄워야 하고, 분할은 트리를 바꾸므로 이 패널을 그리는 도중에 할 수 없다
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        icons: &mut IconCache,
        textures: &mut IconTextures,
        menu_state: PanelMenuState,
    ) -> PanelOutcome {
        let strip = crate::ui::tabs::show_tab_strip(ui, &self.tabs, menu_state);
        let tab = self.tabs.active();
        let nav = self.address.show(ui, &tab.committed, &tab.history);

        // 트리는 주소창 아래 좌측 고정폭을 차지한다 — 현행 Win32 판의 배치와 같다
        let area = ui.available_rect_before_wrap();
        let mut tree_choice = None;
        let content = if self.tree_visible {
            let split_x = area.left() + TREE_WIDTH.min(area.width());
            let tree_rect = egui::Rect::from_min_max(area.min, egui::pos2(split_x, area.bottom()));
            ui.painter().rect_filled(tree_rect, 0.0, theme::SURFACE_BG);
            ui.painter().vline(
                split_x,
                area.y_range(),
                egui::Stroke::new(TREE_BORDER, theme::TREE_LINE),
            );
            tree_choice = ui
                .scope_builder(
                    // 이름을 붙이지 않으면 형제 영역과 같은 id를 갖게 되고, 그 안의 위젯 id까지
                    // 함께 겹친다(egui는 이름 없는 하위 영역에 전부 같은 이름을 준다)
                    egui::UiBuilder::new()
                        .id_salt("tree")
                        .max_rect(tree_rect.shrink(TREE_PAD)),
                    |ui| {
                        ui.set_clip_rect(tree_rect);
                        self.tree.show(ui, ctx)
                    },
                )
                .inner;
            egui::Rect::from_min_max(egui::pos2(split_x + TREE_BORDER, area.top()), area.max)
        } else {
            area
        };
        let action = ui
            .scope_builder(
                egui::UiBuilder::new().id_salt("content").max_rect(content),
                |ui| {
                    ui.set_clip_rect(content);
                    self.show_content(ui, icons, textures)
                },
            )
            .inner;

        // 탭·탐색은 여기서 바로 처리해도 된다(모달이 없다). 셸 메뉴만 호출부로 올려보낸다
        if let Some(path) = tree_choice {
            self.navigate(path, ctx);
        }
        if let Some(tab_action) = strip.tab {
            self.handle_tab(tab_action, ctx);
        }
        if let Some(nav) = nav {
            self.handle_nav(nav, ctx);
        }
        PanelOutcome {
            menu: self.handle_list_action(action, ctx),
            command: strip.command,
        }
    }

    /// 트리를 뺀 나머지 — 트리 토글·상태 줄과 파일 목록
    fn show_content(
        &mut self,
        ui: &mut egui::Ui,
        icons: &mut IconCache,
        textures: &mut IconTextures,
    ) -> FileListAction {
        // 왼쪽에 트리 토글·진행 상황, 오른쪽 끝에 항목 수를 둔다 (사용자 요청 7).
        // `Sides`는 오른쪽 것을 먼저 자리잡게 하므로, 오류 문구가 길어져도 항목 수가 밀리지 않는다
        egui::Sides::new().show(
            ui,
            |ui| {
                if ui
                    .selectable_label(self.tree_visible, "폴더 트리")
                    .clicked()
                {
                    self.tree_visible = !self.tree_visible;
                }
                if self.load.is_loading() {
                    ui.spinner();
                    ui.colored_label(theme::TEXT_DIM, "읽는 중…");
                }
                if !self.status.is_empty() {
                    ui.colored_label(theme::TEXT_DIM, &self.status);
                }
            },
            |ui| {
                // 읽는 중에는 세지 않는다 — 이전 폴더의 수가 남아 새 폴더의 것처럼 보인다
                if !self.load.is_loading() {
                    let (dirs, files) = self.list.counts();
                    ui.colored_label(theme::TEXT_DIM, format!("폴더 {dirs} 파일 {files}"));
                }
            },
        );
        ui.separator();
        self.list.show(ui, icons, textures)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// egui는 같은 ID가 한 프레임에 두 번 쓰이면 화면에 경고 텍스트를 그린다.
    /// 그 텍스트를 그려진 도형에서 찾아 ID 충돌 여부를 판정한다
    fn id_clash_warnings(output: &eframe::egui::FullOutput) -> Vec<String> {
        fn collect(shape: &egui::Shape, found: &mut Vec<String>) {
            match shape {
                egui::Shape::Text(text) => {
                    let body = text.galley.text();
                    if body.contains("use of") {
                        found.push(body.to_owned());
                    }
                }
                egui::Shape::Vec(shapes) => {
                    for shape in shapes {
                        collect(shape, found);
                    }
                }
                _ => {}
            }
        }
        let mut found = Vec::new();
        for clipped in &output.shapes {
            collect(&clipped.shape, &mut found);
        }
        found
    }

    /// 패널을 한 프레임 그리고 ID 충돌 경고를 모은다
    fn draw_panel(tree_visible: bool) -> Vec<String> {
        let ctx = egui::Context::default();
        let mut panel = PanelState::new(std::path::PathBuf::from(r"C:\"));
        if panel.tree_visible != tree_visible {
            panel.toggle_tree();
        }
        let mut icons = crate::fs::icons::IconCache::new();
        let mut textures = crate::ui::icon_tex::IconTextures::new();
        let output = ctx.run_ui(Default::default(), |ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                let ctx = ui.ctx().clone();
                panel.show(
                    ui,
                    &ctx,
                    &mut icons,
                    &mut textures,
                    PanelMenuState {
                        can_close_panel: false,
                    },
                );
            });
        });
        id_clash_warnings(&output)
    }

    #[test]
    fn 패널_안에서_같은_위젯_id가_두_번_쓰이지_않는다() {
        // 탭 스트립·폴더 트리·파일 목록이 각자 스크롤 영역을 갖는데, 이들이 같은 id를 쓰면
        // 스크롤 위치가 서로 섞인다(화면에는 빨간 경고로 드러난다)
        assert!(
            draw_panel(false).is_empty(),
            "위젯 ID 충돌(트리 숨김): {:?}",
            draw_panel(false)
        );
        assert!(
            draw_panel(true).is_empty(),
            "위젯 ID 충돌(트리 표시): {:?}",
            draw_panel(true)
        );
    }
}
