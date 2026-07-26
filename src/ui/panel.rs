//! 패널 — 탭 스트립 / 주소창 / 파일 목록을 담는 탐색 단위 (FR-3).
//!
//! 패널은 자기 탐색 상태(탭·히스토리·목록·열거)를 온전히 소유하며 서로를 모른다.
//! 아이콘 캐시·셸 호스트처럼 앱 전역에서 하나면 충분한 것은 `show`가 인자로 받는다.
//!
//! 탐색은 **pending-커밋** 모델이다: 열거가 성공했을 때만 경로·히스토리를 커밋한다.
//! 실패(삭제·권한)하면 사유만 표시하고 현 위치·목록을 그대로 둔다.
use crate::fs::enumerate::{EnumOutcome, enumerate_dir};
use crate::fs::icons::IconCache;
use crate::panel::tabs::{CloseOutcome, TabState, TabsModel};
use crate::ui::address_bar::{AddressBar, NavAction};
use crate::ui::file_list::{FileListAction, FileListView};
use crate::ui::icon_tex::IconTextures;
use crate::ui::shell_host;
use crate::ui::tabs::TabAction;
use crate::ui::theme;
use eframe::egui;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, TryRecvError, channel};

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
        }
    }

    /// 현재 표시 중인 폴더 — 활성 탭이 커밋한 경로가 정본이다
    pub fn dir(&self) -> &Path {
        &self.tabs.active().committed
    }

    /// 프레임마다 호출 — 지연 시작·열거 완료를 처리하고, 열거 중이면 다시 그리게 한다
    pub fn poll(&mut self, ctx: &egui::Context, icons: &mut IconCache) {
        if let Some(path) = self.deferred_start.take() {
            // 시작 경로는 이미 히스토리에 들어 있으므로 다시 쌓지 않는다
            self.start_load(path, PendingNav::None, ctx);
        }
        self.poll_load(icons);
        if self.load.is_loading() {
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
    /// 세로 구성은 탭 스트립 / 주소창 / 상태 / 파일 목록이며, 목록이 남는 공간을 채운다.
    ///
    /// 셸 메뉴 요청만은 실행하지 않고 반환한다 — 모달이라 그리기가 끝난 뒤에 띄워야 한다
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        icons: &mut IconCache,
        textures: &mut IconTextures,
    ) -> Option<MenuRequest> {
        let tab_action = crate::ui::tabs::show_tab_strip(ui, &self.tabs);
        let tab = self.tabs.active();
        let nav = self.address.show(ui, &tab.committed, &tab.history);
        ui.horizontal(|ui| {
            if self.load.is_loading() {
                ui.spinner();
                ui.colored_label(theme::TEXT_DIM, "읽는 중…");
            } else {
                ui.colored_label(theme::TEXT_DIM, format!("{}개 항목", self.list.len()));
            }
            if !self.status.is_empty() {
                ui.colored_label(theme::TEXT_DIM, &self.status);
            }
        });
        ui.separator();
        let action = self.list.show(ui, icons, textures);

        // 탭·탐색은 여기서 바로 처리해도 된다(모달이 없다). 셸 메뉴만 호출부로 올려보낸다
        if let Some(tab_action) = tab_action {
            self.handle_tab(tab_action, ctx);
        }
        if let Some(nav) = nav {
            self.handle_nav(nav, ctx);
        }
        self.handle_list_action(action, ctx)
    }
}
