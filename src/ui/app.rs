//! egui 앱 골격 — 창·폰트·팔레트·COM·셸 호스트와 전역 공유 자원을 보유한다.
//!
//! 실제 탐색은 `ui::panel::PanelState`가 담당하고, 그 패널들을 담은 분할 화면 한 벌이
//! `WorkspaceView`다. 이 구조체는 워크스페이스 목록(사이드바)과 뷰들을 잇는 그릇이다.
use crate::app::layout::TreeShape;
use crate::app::layout::{LayoutTree, PanelId, Rect as LayoutRect, SplitDir, SplitPlace};
use crate::app::settings::{
    SIDEBAR_DEFAULT_WIDTH, SIDEBAR_MAX_WIDTH, SIDEBAR_MIN_WIDTH, Session, SidebarSession,
    WindowState, save_session,
};
use crate::app::workspace::{WorkspaceId, WorkspaceList};
use crate::fs::icons::IconCache;
use crate::panel::tabs::TabPhase;
use crate::remote::connection::{ConnCommand, ConnEvent, ConnPhase, ConnectionId};
use crate::remote::ftp::FtpSession;
use crate::remote::log::LogBuffer;
use crate::remote::manager::ConnectionManager;
use crate::remote::queue::TransferQueue;
use crate::remote::sftp::SftpSession;
use crate::remote::sites::SiteStore;
use crate::remote::transfer::TransferRunner;
use crate::remote::types::{LogonType, RemotePath, RemoteSession, SiteId};
use crate::remote::url::RemoteUrl;
use crate::ui::dock::{self, DockAction, DockPanel, DockState, DockView};
use crate::ui::icon_tex::IconTextures;
use crate::ui::log_panel;
use crate::ui::menu::{self, Command};
use crate::ui::panel::{PanelState, RemoteAction};
use crate::ui::queue_panel::{self, QueueAction};
use crate::ui::remote_states::{HostKeyGate, RemoteView};
use crate::ui::session::{self, PanelTabs, WorkspaceState};
use crate::ui::shell_host::ShellHost;
use crate::ui::sidebar::{SidebarAction, WorkspaceSidebar};
use crate::ui::site_manager::{SiteManager, SiteManagerOutcome};
use crate::ui::splitter;
use crate::ui::theme;
use crate::ui::titlebar::{self, WindowRequest};
use crate::ui::toast::{self, Toast};
use eframe::egui;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use windows::Win32::Foundation::RPC_E_CHANGED_MODE;
use windows::Win32::System::Com::{COINIT_APARTMENTTHREADED, CoInitializeEx, CoUninitialize};

/// 맑은 고딕 — egui 기본 폰트에는 한글 글리프가 없어 파일명이 두부(□)로 보인다
const KOREAN_FONT_PATH: &str = r"C:\Windows\Fonts\malgun.ttf";

/// 삭제 확인 대화 폭 — 문구 두 줄이 접히지 않을 만큼
const REMOVE_DIALOG_WIDTH: f32 = 360.0;

/// 세션이 없을 때의 창 크기·위치 (첫 실행)
const DEFAULT_WINDOW: WindowState = WindowState {
    x: 0,
    y: 0,
    w: 1100,
    h: 700,
    maximized: false,
};

/// 셸 메뉴를 쓸 수 없을 때 화면에 보일 문구.
/// 원인이 무엇이든 사용자가 할 수 있는 일은 재시작뿐이라 한 문구로 통일한다
const SHELL_UNAVAILABLE: &str =
    "마우스 오른쪽 버튼 메뉴를 사용할 수 없습니다 (앱을 다시 시작해 주세요)";

/// UI 스레드의 COM 아파트먼트 상태.
/// 셸 컨텍스트 메뉴(`IContextMenu`)는 STA를 요구하므로 셸 연동 가용 여부가 여기서 갈린다
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ComStatus {
    /// STA 확보 — 이 프로세스가 초기화했거나(S_OK) 이미 초기화돼 있었다(S_FALSE)
    Sta { owned: bool },
    /// 다른 아파트먼트로 이미 초기화됨 — 셸 메뉴 사용 불가
    WrongApartment,
    /// 그 외 실패
    Failed,
}

impl ComStatus {
    /// 셸 메뉴를 띄울 수 있는 상태인가.
    /// 실패 원인(`WrongApartment`/`Failed`)은 사용자가 취할 행동이 같아 화면에서는 구분하지 않는다
    pub fn is_available(self) -> bool {
        matches!(self, ComStatus::Sta { .. })
    }
}

/// COM을 STA로 초기화한다. 반환을 세 갈래로 처리한다 —
/// `S_OK`(이번에 초기화)·`S_FALSE`(이미 초기화됨) 모두 STA 확보로 보고 진행한다.
pub fn init_com() -> ComStatus {
    // 안전성: UI 스레드에서 1회 호출. 인자는 정적 상수이며 반환 HRESULT로만 분기한다
    let hr = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
    if hr.is_ok() {
        // S_FALSE면 이미 초기화된 상태라 이 프로세스가 해제 책임을 지지 않는다
        ComStatus::Sta {
            owned: hr == windows::Win32::Foundation::S_OK,
        }
    } else if hr == RPC_E_CHANGED_MODE {
        ComStatus::WrongApartment
    } else {
        ComStatus::Failed
    }
}

/// 이 프로세스가 초기화한 경우에만 COM을 해제한다.
///
/// # Safety
///
/// `init_com`이 `S_OK`를 받은 **같은 스레드에서 1회만** 호출해야 한다.
/// 다른 스레드에서 부르거나 중복 호출하면 COM 참조 계수가 어긋난다.
pub unsafe fn uninit_com(com: ComStatus) {
    if let ComStatus::Sta { owned: true } = com {
        unsafe { CoUninitialize() };
    }
}

/// 앱이 쓰는 글꼴을 등록한다 — 한글 본문 글꼴(맑은 고딕)과 아이콘 글꼴(Phosphor).
///
/// 반환값은 **한글 글꼴 적용 여부**다. 맑은 고딕을 읽지 못해도 아이콘 글꼴은 등록되므로
/// 타이틀바 버튼은 그대로 보인다(파일명만 기본 글꼴로 표시된다).
/// 등록은 한 번에 끝낸다 — `set_fonts`를 두 번 부르면 뒤엣것이 앞엣것을 덮어쓴다
pub fn install_fonts(ctx: &egui::Context) -> bool {
    let mut fonts = egui::FontDefinitions::default();
    let korean = match std::fs::read(KOREAN_FONT_PATH) {
        Ok(bytes) => {
            fonts.font_data.insert(
                "malgun".to_owned(),
                Arc::new(egui::FontData::from_owned(bytes)),
            );
            // 기본 폰트보다 앞에 두어 한글이 우선 매칭되게 한다
            fonts
                .families
                .entry(egui::FontFamily::Proportional)
                .or_default()
                .insert(0, "malgun".to_owned());
            fonts
                .families
                .entry(egui::FontFamily::Monospace)
                .or_default()
                .push("malgun".to_owned());
            true
        }
        Err(_) => false,
    };
    // 아이콘 글꼴은 exe에 정적으로 담겨 있어 실패 경로가 없다 — 한글 글꼴 성공 여부와 무관하게 등록한다
    egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);
    ctx.set_fonts(fonts);
    korean
}

/// 워크스페이스 한 벌의 탐색 상태 — 분할 트리와 그 패널들 (FR-17).
///
/// 워크스페이스마다 이것을 하나씩 갖고, **처음 선택될 때 비로소 만들어진다**(D1 지연 생성).
/// 한 번도 열지 않은 워크스페이스는 패널도 열거 스레드도 없다
pub struct WorkspaceView {
    /// 분할 트리 — 어느 패널이 화면 어디를 차지하는지 (FR-1)
    layout: LayoutTree,
    /// 패널 실체. 트리는 `PanelId`만 알고 상태는 여기에 있다
    panels: HashMap<PanelId, PanelState>,
    active: PanelId,
}

impl WorkspaceView {
    fn new(start: PathBuf) -> WorkspaceView {
        let (layout, first) = LayoutTree::new();
        let mut panels = HashMap::new();
        panels.insert(first, PanelState::new(start));
        WorkspaceView {
            layout,
            panels,
            active: first,
        }
    }

    /// 지정한 패널을 나눈다. 새 패널은 원래 패널과 같은 폴더에서 시작한다.
    ///
    /// 패널마다 있는 분할 버튼은 **자기 패널**을 대상으로 하므로 활성 패널과 다를 수 있다 (D3)
    /// 나뉘었으면 새 패널의 id — **공간이 부족하면 `None`**이다.
    ///
    /// 반환하는 이유: 연결 시 좌우 분할(FR-35)은 분할이 서지 못했을 때 현재 패널의 새 탭으로
    /// 물러서야 하는데, 성공 여부를 알 수 없으면 그 판단을 할 수 없다 (T14 Acceptance ④)
    fn split_panel(
        &mut self,
        target: PanelId,
        dir: SplitDir,
        place: SplitPlace,
        area: LayoutRect,
    ) -> Option<PanelId> {
        let start = self
            .panels
            .get(&target)
            .map(|p| p.dir().to_path_buf())
            .unwrap_or_else(start_dir);
        // 공간이 부족하면 나눌 수 없다 (사용자는 창을 키우면 된다)
        let new_id = self.layout.split(target, dir, place, area).ok()?;
        self.panels.insert(new_id, PanelState::new(start));
        self.active = new_id;
        Some(new_id)
    }

    /// 지정한 패널을 닫는다. 마지막 하나는 닫히지 않는다 (FR-2).
    ///
    /// 닫은 **자리를 흡수한 패널**을 다음 활성으로 삼는다 — 트리 순서상 첫 패널을 고르면
    /// 포커스가 화면 반대편으로 튀어 방금 조작한 위치와 멀어진다.
    ///
    /// 대상이 활성 패널과 다를 수 있다 — 패널 메뉴는 자기 패널을 가리키기 때문이다 (plan D16)
    fn close_panel(&mut self, target: PanelId, area: LayoutRect) {
        let closed = self
            .layout
            .compute_rects(area)
            .panes
            .iter()
            .find(|(id, _)| *id == target)
            .map(|(_, rect)| *rect);
        if self.layout.close(target).is_err() {
            return;
        }
        self.panels.remove(&target);
        let next = closed
            .and_then(|closed| {
                self.layout
                    .compute_rects(area)
                    .panes
                    .into_iter()
                    .max_by_key(|(_, rect)| overlap_area(*rect, closed))
                    .map(|(id, _)| id)
            })
            .or_else(|| self.layout.panel_ids().first().copied());
        if let Some(next) = next {
            self.active = next;
        }
    }

    /// 저장된 상태로 워크스페이스를 되살린다 (FR-11·FR-20).
    /// 패널은 분할 트리 리프의 walk 순서로 짝지어진다(`settings` 스키마 계약)
    fn from_state(state: &WorkspaceState) -> WorkspaceView {
        let (layout, ids) = LayoutTree::from_shape(&state.shape);
        let Some(&first) = ids.first() else {
            return WorkspaceView::new(start_dir());
        };
        let mut panels = HashMap::new();
        for (&id, tabs) in ids.iter().zip(&state.panels) {
            let panel = PanelState::from_tabs(
                tabs.tabs.clone(),
                tabs.active_tab,
                &tabs.columns,
                &tabs.view_mode,
            )
            .unwrap_or_else(|| PanelState::new(start_dir()));
            panels.insert(id, panel);
        }
        WorkspaceView {
            layout,
            panels,
            active: ids.get(state.active_panel).copied().unwrap_or(first),
        }
    }

    /// 세션 저장용 스냅숏 — 분할 구조와 패널별 탭을 리프 walk 순서로 담는다
    fn to_state(&self, name: String) -> WorkspaceState {
        let ids = self.layout.panel_ids();
        let panels = ids
            .iter()
            .map(|id| match self.panels.get(id) {
                Some(panel) => PanelTabs {
                    tabs: panel.tab_paths(),
                    active_tab: panel.active_tab(),
                    columns: panel.columns(),
                    view_mode: panel.view_mode().as_key().to_owned(),
                },
                // 트리에는 있는데 상태가 없는 리프 — 분할 직후가 아니면 생기지 않는다
                None => PanelTabs {
                    tabs: vec![start_dir()],
                    active_tab: 0,
                    ..Default::default()
                },
            })
            .collect();
        WorkspaceState {
            name,
            shape: self.layout.shape(),
            panels,
            active_panel: ids.iter().position(|id| *id == self.active).unwrap_or(0),
        }
    }

    /// 사이드바 부제에 쓸 현재 폴더 — 활성 패널의 활성 탭 경로
    fn active_dir(&self) -> Option<PathBuf> {
        self.panels.get(&self.active).map(|p| p.dir().to_path_buf())
    }
}

/// 탐색기 앱 상태.
pub struct ExplorerApp {
    com: ComStatus,
    /// 셸 메뉴용 창 핸들 — HWND를 얻지 못하면 `None`(셸 메뉴 비활성)
    shell: Option<ShellHost>,
    /// 한글 폰트 적용 여부 — 실패 시 화면에 알린다
    korean_font: bool,
    /// 아이콘 캐시 — 앱 전역 공유 (패널마다 두면 같은 아이콘을 중복 보관하게 된다)
    icons: IconCache,
    textures: IconTextures,
    /// 워크스페이스 목록(이름·부제·활성) — 표시 데이터의 정본 (FR-15)
    workspaces: WorkspaceList,
    /// 워크스페이스별 탐색 상태 — **방문한 것만** 들어 있다 (D1).
    /// 목록의 인덱스가 아니라 `WorkspaceId`로 잡는다: 순서 변경·삭제로 인덱스는 흔들린다
    views: HashMap<WorkspaceId, WorkspaceView>,
    /// 세션에서 불러왔지만 **아직 열지 않은** 워크스페이스의 저장 상태 (D1 지연 생성).
    /// 처음 선택될 때 이것으로 뷰를 만들고, 그 전에는 저장할 때 그대로 다시 내보낸다
    restored: HashMap<WorkspaceId, WorkspaceState>,
    sidebar: WorkspaceSidebar,
    /// 마지막으로 관측한 사이드바 폭 — 세션 저장용
    sidebar_width: f32,
    sidebar_collapsed: bool,
    /// 마지막으로 관측한 창 위치·크기 (최대화가 아닐 때만 갱신 — 최대화 상태를 저장하면
    /// 다음 실행에서 창을 되돌릴 "일반 크기"가 사라진다)
    window: WindowState,
    /// 첫 프레임에 화면 안으로 보정할 복원 위치 — 모니터 크기는 창이 뜬 뒤에야 알 수 있다
    restore_window: Option<WindowState>,
    /// 삭제 확인을 기다리는 워크스페이스 (FR-18).
    /// 인덱스가 아니라 id로 잡는다 — 확인 대화는 프레임을 넘겨 살아 있는데,
    /// 그 사이 순서가 바뀌면 인덱스는 다른 워크스페이스를 가리킨다 (D12 ①과 같은 이유)
    pending_remove: Option<WorkspaceId>,
    /// 열린 원격 연결 전부 — 워크스페이스가 아니라 앱이 쥔다.
    /// 연결은 탭보다 오래 살고 워크스페이스를 넘나들 수 있다 (FR-45·NFR-11)
    manager: ConnectionManager,
    /// 등록된 사이트 (FR-27). 탭·사이드바가 이름·프로토콜을 여기서 읽는다
    sites: SiteStore,
    /// SFTP 지문 확인 대화와 연결 워커를 잇는 통로 (D15)
    hostkey: HostKeyGate,
    /// 사이트 관리자 대화 (FR-27) — 연결 메뉴의 `새 사이트 추가…`와 실패 화면의 `설정 열기`가 연다
    site_manager: SiteManager,
    /// 짧게 떴다 사라지는 알림 (FR-43) — 창 전역이라 워크스페이스를 넘나든다
    toast: Toast,
    /// 전송 큐 (FR-36) — 연결과 같은 이유로 앱이 쥔다(워크스페이스를 넘나든다)
    queue: TransferQueue,
    /// 큐를 실제 전송으로 옮기는 실행기 (FR-37)
    runner: TransferRunner,
    /// 하단 도크의 화면 상태 (FR-36·FR-40)
    dock: DockState,
    /// 클립보드로 내보낼 것 — 그리기 도중에는 `ctx`를 빌릴 수 없어 프레임 끝에 보낸다
    pending_clipboard: Option<String>,
}

impl ExplorerApp {
    /// eframe 창 생성 직후 호출된다 — 폰트·팔레트·셸 호스트를 이 시점에 준비한다.
    /// `session`이 있으면 지난 실행의 워크스페이스·사이드바·창 상태를 되살린다 (FR-11·FR-20)
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        com: ComStatus,
        session: Option<Session>,
    ) -> ExplorerApp {
        let korean_font = install_fonts(&cc.egui_ctx);
        theme::apply_dark(&cc.egui_ctx);
        // HWND 획득·서브클래스 설치는 창이 만들어진 이 시점에만 가능하다
        let shell = ShellHost::new(cc);
        // 최대화·복원 때 OS가 옛 화면과 새 화면을 겹쳐 페이드하면 글자가 이중으로 보인다 (FR-22)
        if let Some(shell) = &shell {
            crate::app::theme::disable_window_transitions(shell.hwnd());
        }
        let mut app = ExplorerApp {
            com,
            shell,
            korean_font,
            icons: IconCache::new(),
            textures: IconTextures::new(),
            workspaces: WorkspaceList::new(),
            views: HashMap::new(),
            restored: HashMap::new(),
            sidebar: WorkspaceSidebar::new(),
            sidebar_width: SIDEBAR_DEFAULT_WIDTH as f32,
            sidebar_collapsed: false,
            window: DEFAULT_WINDOW,
            restore_window: None,
            pending_remove: None,
            // 워커가 소식을 올리면 창을 다시 그리게 한다 — 입력이 없으면 egui는 프레임을
            // 돌리지 않아, 이 신호가 없으면 목록이 사용자가 마우스를 움직일 때까지 안 나타난다
            manager: ConnectionManager::new({
                let ctx = cc.egui_ctx.clone();
                Arc::new(move || ctx.request_repaint())
            }),
            sites: SiteStore::new(),
            hostkey: HostKeyGate::new(),
            site_manager: SiteManager::new(),
            toast: Toast::new(),
            queue: TransferQueue::new(),
            runner: TransferRunner::new(),
            dock: DockState::default(),
            pending_clipboard: None,
        };
        if let Some(session) = session {
            app.apply_session(session);
        }
        app
    }

    /// 불러온 세션을 상태에 반영한다. 워크스페이스 **뷰는 만들지 않는다** —
    /// 활성 워크스페이스도 첫 프레임의 `ensure_active_view`에서 비로소 만들어진다(D1)
    fn apply_session(&mut self, session: Session) {
        let states = session::restore(&session);
        let names: Vec<String> = states.iter().map(|state| state.name.clone()).collect();
        let Some(workspaces) = WorkspaceList::from_names(names, session.active_workspace) else {
            return; // 빈 목록 — 기본 워크스페이스 1개로 시작한다
        };
        self.workspaces = workspaces;
        // `from_names`는 id를 0부터 순서대로 준다 — 그 순서가 곧 저장된 순서다
        self.restored = self
            .workspaces
            .items()
            .iter()
            .map(|item| item.id)
            .zip(states)
            .collect();
        for index in 0..self.workspaces.len() {
            if let Some(dir) = self.restored_active_dir(index) {
                self.workspaces.set_subtitle(index, &dir);
            }
        }
        self.sidebar_width = session.sidebar.width as f32;
        self.sidebar_collapsed = session.sidebar.collapsed;
        self.window = session.window.clone();
        self.restore_window = Some(session.window);
    }

    /// 아직 열지 않은 워크스페이스의 활성 폴더 — 사이드바 부제를 복원 직후에도 채우기 위함
    fn restored_active_dir(&self, index: usize) -> Option<PathBuf> {
        let id = self.workspaces.items().get(index)?.id;
        let state = self.restored.get(&id)?;
        let panel = state.panels.get(state.active_panel)?;
        panel.tabs.get(panel.active_tab).cloned()
    }

    /// 지금 상태를 세션으로 모은다 (종료 시 저장)
    fn collect_session(&self) -> Session {
        let workspaces: Vec<WorkspaceState> = self
            .workspaces
            .items()
            .iter()
            .map(|item| match self.views.get(&item.id) {
                Some(view) => view.to_state(item.name.clone()),
                // 한 번도 열지 않은 워크스페이스는 불러온 상태를 그대로 다시 내보낸다.
                // 이름만 최신값으로 — 열지 않고도 이름은 바꿀 수 있다
                None => match self.restored.get(&item.id) {
                    Some(state) => WorkspaceState {
                        name: item.name.clone(),
                        ..state.clone()
                    },
                    None => WorkspaceState {
                        name: item.name.clone(),
                        shape: TreeShape::Leaf,
                        panels: vec![PanelTabs {
                            tabs: vec![start_dir()],
                            active_tab: 0,
                            ..Default::default()
                        }],
                        active_panel: 0,
                    },
                },
            })
            .collect();
        session::to_session(
            self.window.clone(),
            SidebarSession {
                width: self.sidebar_width as i32,
                collapsed: self.sidebar_collapsed,
            },
            self.workspaces.active_index(),
            &workspaces,
        )
    }

    /// 셸 메뉴를 쓸 수 있는가 — COM STA와 창 핸들이 모두 있어야 한다
    fn shell_available(&self) -> bool {
        self.com.is_available() && self.shell.is_some()
    }

    /// 활성 워크스페이스의 탐색 상태를 확보한다 — 처음 열리는 워크스페이스면 여기서 만들어진다(D1).
    /// 세션에서 불러온 것이면 저장된 분할·탭으로 되살린다
    fn ensure_active_view(&mut self) -> &mut WorkspaceView {
        let id = self.workspaces.active().id;
        // 뷰가 이미 있으면 저장 상태를 꺼내지 않는다 — 꺼내 놓고 쓰지 않으면 그대로 버려진다
        let restored = if self.views.contains_key(&id) {
            None
        } else {
            self.restored.remove(&id)
        };
        self.views.entry(id).or_insert_with(|| match restored {
            Some(state) => WorkspaceView::from_state(&state),
            None => WorkspaceView::new(start_dir()),
        })
    }

    /// 사이드바 조작 반영 — 목록 변경은 전부 여기서만 일어난다
    fn handle_sidebar(&mut self, action: SidebarAction, area: LayoutRect) {
        match action {
            SidebarAction::Select(index) => {
                self.workspaces.set_active(index);
            }
            SidebarAction::Add => {
                self.workspaces.add();
            }
            SidebarAction::Rename(index, name) => {
                self.workspaces.rename(index, &name);
            }
            SidebarAction::Remove(index) => {
                // 바로 지우지 않고 한 번 묻는다 — 되돌릴 수 없다(현행 판과 같은 규칙)
                self.pending_remove = self.workspaces.items().get(index).map(|item| item.id);
            }
            SidebarAction::Reorder(from, to) => {
                self.workspaces.reorder(from, to);
            }
            // 어느 사이트를 골랐는지는 사이드바가 스스로 들고 그린다 — 앱이 따로 둘 상태가 없다
            SidebarAction::SelectSite(_) => {}
            SidebarAction::ConnectSite(site) => {
                // 진입점 셋과 **같은 경로**로 보낸다 — 사이드바만 분할 없이 연결하면
                // 여는 방법에 따라 배치가 달라진다
                self.open_site_tab(site, None, area);
            }
            // 목록에서 감출 뿐 사이트는 남는다 (README §1) — 사이트 관리자에 그대로 보인다
            SidebarAction::HideSite(site) => self.sites.hide(site),
            // 사이트 목록은 메모리에 있어 지금은 다시 읽을 것이 없다.
            // 파일로 오가는 저장이 붙는 T25(세션 v3)에서 이 자리가 다시 읽기가 된다
            SidebarAction::RefreshSites => {}
            // 연결 메뉴는 사이드바가 직접 띄운다 — 이 조작은 알림일 뿐이다
            SidebarAction::OpenConnectMenu => {}
            // `새 사이트 추가…`라 빈 초안으로 연다 (인벤토리 #8)
            SidebarAction::OpenSiteManager => self.site_manager.open_new(),
        }
    }

    /// 지금 연결이 열려 있는 사이트들 — 사이드바의 상태 점이 이것으로 갈린다.
    ///
    /// 사이드바에 `ConnectionManager`를 통째로 넘기지 않는 이유: 사이드바가 알아야 하는 것은
    /// "이 사이트에 연결이 있는가" 하나뿐이라, 연결 계층을 알게 하면 의존만 넓어진다
    fn connected_sites(&self) -> Vec<SiteId> {
        self.manager
            .ids()
            .iter()
            .filter_map(|id| self.manager.get(*id))
            .map(|connection| connection.site)
            .collect()
    }

    /// 연결이 **실패한 상태로 남아 있는** 사이트들 — 큐의 연결별 탭이 그 점을 빨강으로 그린다.
    ///
    /// `connected_sites`(연결 객체의 유무)와 다른 값이다: 실패한 연결도 탭을 닫기 전까지
    /// 매니저에 남아 있어, 유무로 가르면 실패한 사이트가 "연결됨"으로 보인다
    fn failed_sites(&self) -> Vec<SiteId> {
        self.manager
            .ids()
            .iter()
            .filter_map(|id| self.manager.get(*id))
            .filter(|connection| matches!(connection.phase(), ConnPhase::Failed { .. }))
            .map(|connection| connection.site)
            .collect()
    }

    /// 창 위치·크기를 따라간다. 최대화 중에는 갱신하지 않는다 —
    /// 그때의 사각형은 화면 전체라, 저장해 버리면 다음 실행에서 되돌릴 일반 크기가 사라진다.
    /// 복원 위치가 화면 밖이면(모니터 구성 변경) 첫 프레임에 화면 안으로 옮긴다
    fn track_window(&mut self, ctx: &egui::Context) {
        let (rect, maximized, monitor) = ctx.input(|input| {
            let viewport = input.viewport();
            (
                viewport.outer_rect,
                viewport.maximized.unwrap_or(false),
                viewport.monitor_size,
            )
        });
        self.window.maximized = maximized;
        if let Some(rect) = rect
            && !maximized
        {
            self.window.x = rect.left() as i32;
            self.window.y = rect.top() as i32;
            self.window.w = rect.width() as i32;
            self.window.h = rect.height() as i32;
        }
        // 모니터 크기를 아직 모르면 보정을 다음 프레임으로 미룬다 —
        // 여기서 소비해 버리면 값이 채워진 뒤에도 다시 시도하지 않는다
        if let Some(size) = monitor
            && let Some(saved) = self.restore_window.take()
        {
            let (monitor_w, monitor_h) = (size.x as i32, size.y as i32);
            let fixed = session::clamp_window(saved.clone(), monitor_w, monitor_h);
            if fixed != saved {
                ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(egui::pos2(
                    fixed.x as f32,
                    fixed.y as f32,
                )));
                ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(
                    fixed.w as f32,
                    fixed.h as f32,
                )));
                self.window = fixed;
            }
        }
    }

    /// 타이틀바를 그리고 앱 명령을 돌려준다 (FR-22).
    /// 창 조작(최소화·최대화·닫기·끌기·가장자리 크기 조절)은 여기서 바로 창에 전달하고,
    /// 앱 명령(사이드바 토글)만 호출부로 넘긴다 — 분할 영역이 확정된 뒤에 실행해야 하기 때문이다
    fn show_titlebar(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) -> Option<Command> {
        let state = titlebar::TitlebarState {
            maximized: self.window.maximized,
            sidebar_collapsed: self.sidebar_collapsed,
        };
        let title = self.workspaces.active().name.clone();
        let outcome = egui::Panel::top(egui::Id::new("titlebar"))
            .resizable(false)
            .exact_size(titlebar::TITLEBAR_HEIGHT)
            // 구분선은 `titlebar`가 앱 팔레트 색으로 직접 그린다 — egui 기본 구분선은 끈다
            .show_separator_line(false)
            .frame(egui::Frame::NONE.fill(theme::WINDOW_BG))
            .show(ui, |ui| titlebar::show_titlebar(ui, &title, state))
            .inner;
        // 창 가장자리 크기 조절 — 모서리는 크기 조절이 우선이고(버튼이 가져가면 대각선으로 창을
        // 잡을 자리가 사라진다), 버튼 위쪽 변은 버튼이 우선한다(`show_resize_handles`가 가른다)
        let resize = titlebar::show_resize_handles(ctx, self.window.maximized);
        if let Some(request) = resize.or(outcome.window) {
            let command = match request {
                WindowRequest::Minimize => egui::ViewportCommand::Minimized(true),
                WindowRequest::ToggleMaximize => {
                    egui::ViewportCommand::Maximized(!self.window.maximized)
                }
                // 닫기는 창 닫기 요청으로 보낸다 — eframe이 종료 절차를 거치며 `on_exit`를
                // 부르므로 세션이 저장된다(프로세스를 직접 끝내면 그 경로가 사라진다)
                WindowRequest::Close => egui::ViewportCommand::Close,
                WindowRequest::Drag => egui::ViewportCommand::StartDrag,
                WindowRequest::Resize(direction) => egui::ViewportCommand::BeginResize(direction),
            };
            ctx.send_viewport_cmd(command);
        }
        outcome.command
    }

    /// 삭제 확인 대화 (FR-18) — 워크스페이스 하나를 지우면 그 화면 구성과 탭이 함께 사라지고
    /// 되돌릴 수 없어 한 번 묻는다. 문구는 현행 Win32 판의 확인 대화와 같다
    fn show_remove_confirm(&mut self, ctx: &egui::Context) {
        let Some(id) = self.pending_remove else {
            return;
        };
        // 대화가 떠 있는 동안 순서가 바뀔 수 있으므로 자리를 매번 다시 찾는다
        let found = self
            .workspaces
            .items()
            .iter()
            .position(|item| item.id == id)
            .map(|index| (index, self.workspaces.items()[index].name.clone()));
        let Some((index, name)) = found else {
            // 목록이 그 사이 바뀌어 대상이 사라졌다 — 물을 것이 없다
            self.pending_remove = None;
            return;
        };
        let mut confirmed = None;
        egui::Modal::new(egui::Id::new("workspace_remove_confirm")).show(ctx, |ui| {
            ui.set_width(REMOVE_DIALOG_WIDTH);
            ui.heading("워크스페이스 삭제");
            ui.add_space(8.0);
            ui.label(format!("'{name}' 워크스페이스를 삭제할까요?"));
            ui.label("이 워크스페이스의 화면 구성과 탭이 함께 사라집니다.");
            ui.add_space(12.0);
            ui.horizontal(|ui| {
                if ui.button("삭제").clicked() {
                    confirmed = Some(true);
                }
                if ui.button("취소").clicked() || ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    confirmed = Some(false);
                }
            });
        });
        match confirmed {
            Some(true) => {
                self.pending_remove = None;
                self.remove_workspace(index);
            }
            Some(false) => self.pending_remove = None,
            None => {}
        }
    }

    /// 하단 도크 — 전송 큐·서버 로그가 번갈아 쓰는 자리 (FR-36·FR-40·D19).
    ///
    /// **egui의 아래쪽 패널로 뗀다** — 사이드바(`Panel::left`)와 같은 방식이다. 직접 사각형을
    /// 잡아 `allocate_rect`로 떼면 위→아래 배치에서 커서가 **화면 바닥 너머**로 밀려,
    /// 뒤에 오는 패널 그리드가 높이 0이 된다(egui 0.35 `advance_after_rects` — T19 quality
    /// 리뷰 B1에서 실측). 창이 낮으면 도크가 줄어든다(`dock::dock_height`)
    fn show_dock(&mut self, ui: &mut egui::Ui) {
        if !self.dock.is_open() {
            return;
        }
        let height = dock::dock_height(ui.available_height());
        if height <= 0.0 {
            return;
        }
        let failed = self.failed_sites();
        let log_conn = self.log_connection();
        let panel = egui::Panel::bottom(egui::Id::new("transfer_dock"))
            .resizable(false)
            .default_size(height)
            .size_range(egui::Rangef::new(height, height))
            .frame(egui::Frame::NONE)
            .show(ui, |ui| {
                let rect = ui.max_rect();
                self.show_dock_body(ui, rect, &failed, log_conn)
            });
        let (dock_action, queue_action) = panel.inner;
        if let Some(action) = dock_action {
            self.apply_dock_action(action);
        }
        if let Some(action) = queue_action {
            self.apply_queue_action(action);
        }
    }

    /// 도크 안쪽 — 탭 스트립과 본문. 조작은 값으로 돌려주고 호출부가 실행한다
    fn show_dock_body(
        &mut self,
        ui: &mut egui::Ui,
        rect: egui::Rect,
        failed: &[SiteId],
        log_conn: Option<ConnectionId>,
    ) -> (Option<DockAction>, Option<QueueAction>) {
        let mut dock_ui = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(rect)
                .layout(egui::Layout::top_down(egui::Align::Min)),
        );
        dock_ui.set_clip_rect(rect);
        dock_ui.painter().rect_filled(rect, 0.0, theme::SURFACE_BG);
        dock_ui.painter().line_segment(
            [
                egui::pos2(rect.left(), rect.top() + 0.5),
                egui::pos2(rect.right(), rect.top() + 0.5),
            ],
            egui::Stroke::new(1.0, theme::PANE_BORDER),
        );

        let strip =
            egui::Rect::from_min_size(rect.min, egui::vec2(rect.width(), dock::STRIP_HEIGHT));
        let body = egui::Rect::from_min_max(egui::pos2(rect.left(), strip.bottom()), rect.max);
        {
            let view = DockView {
                queue: &self.queue,
                failed,
            };
            let dock_action = dock::show_strip(&mut dock_ui, strip, &mut self.dock, &view);
            let queue_action = match self.dock.panel {
                Some(DockPanel::Queue) => {
                    queue_panel::show_queue(&mut dock_ui, body, &mut self.dock, &view, &self.sites)
                }
                Some(DockPanel::Log) => {
                    // 지금 보고 있는 연결의 로그를 그린다 — 연결이 없으면 빈 화면이다
                    let body = egui::Rect::from_min_max(
                        egui::pos2(body.left(), body.top() + log_panel::BODY_PAD_Y),
                        body.max,
                    );
                    match log_conn.and_then(|conn| self.manager.get(conn)) {
                        Some(connection) => {
                            log_panel::show_log(&mut dock_ui, body, connection.log())
                        }
                        None => log_panel::show_log(&mut dock_ui, body, &LogBuffer::new()),
                    }
                    None
                }
                None => None,
            };
            (dock_action, queue_action)
        }
    }

    /// 로그 화면이 보여 줄 연결 — **지금 보고 있는 원격 탭의 것**이 먼저다.
    ///
    /// 그 탭이 로컬이면 마지막으로 연 연결을 보인다: 로그를 여는 까닭은 대개 방금 무슨 일이
    /// 있었는지 보려는 것이라, 아무것도 안 보이는 것보다 최근 연결을 보이는 편이 쓸모 있다
    fn log_connection(&self) -> Option<ConnectionId> {
        let active = self
            .views
            .get(&self.workspaces.active().id)
            .and_then(|view| view.panels.get(&view.active))
            .and_then(|panel| panel.active_conn());
        active.or_else(|| self.manager.ids().last().copied())
    }

    /// 도크 탭 스트립의 조작 (인벤토리 #33·#34)
    fn apply_dock_action(&mut self, action: DockAction) {
        match action {
            DockAction::TogglePause => {
                if self.queue.is_paused() {
                    self.runner.resume(&mut self.queue);
                } else {
                    self.runner.pause(&mut self.queue, &self.manager);
                }
            }
            DockAction::ClearDone => self.queue.clear_done(),
            DockAction::CopyLog => self.copy_log(),
        }
    }

    /// `⧉` — 지금 보고 있는 연결의 로그를 클립보드로 (Acceptance ③).
    ///
    /// **복사본에도 비밀번호는 없다** — 가리기는 로그가 버퍼에 들어가기 전에 이미 끝났다
    /// (D14·T5). 여기서 다시 가리지 않는 것은 두 벌이 되면 한쪽만 고쳐질 수 있어서다
    fn copy_log(&mut self) {
        let Some(text) = self
            .log_connection()
            .and_then(|conn| self.manager.get(conn))
            .map(|connection| connection.log().to_text())
        else {
            return;
        };
        if text.is_empty() {
            return;
        }
        self.pending_clipboard = Some(text);
    }

    /// 큐 행에서 고른 조작 (T19 우클릭 메뉴)
    fn apply_queue_action(&mut self, action: QueueAction) {
        match action {
            // 실패한 것을 대기로 되돌리면 다음 `start_ready`가 다시 건다
            QueueAction::Retry(id) => self
                .queue
                .update(id, crate::remote::queue::TransferState::Wait),
            QueueAction::Cancel(id) => {
                self.runner.cancel(&self.manager, id);
                self.queue.cancel(id);
            }
        }
    }

    /// 사이트 관리자 대화 (FR-27) — 등록 결과를 받아 연결까지 잇는다.
    ///
    /// `area`는 `연결(C)`이 패널을 좌우로 나눌 때 쓴다(T14와 같은 착지점). 아직 배치를 모르는
    /// 첫 프레임에는 `None`이라, 그때는 연결 대신 등록만 하고 사용자가 다시 누르면 된다 —
    /// 임의의 영역을 지어내 엉뚱한 자리에 패널을 만들지 않는다
    fn show_site_manager(&mut self, ctx: &egui::Context, area: Option<LayoutRect>) {
        if !self.site_manager.is_open() {
            return;
        }
        let connected = self.connected_sites();
        let outcome = self.site_manager.show(ctx, &mut self.sites, &connected);
        match outcome {
            SiteManagerOutcome::None | SiteManagerOutcome::Close => {}
            // 등록만 했으면 그 사실을 짧게 알린다 (인벤토리 #89·#91)
            SiteManagerOutcome::Register(site) => {
                let host = self
                    .sites
                    .get(site)
                    .map(|record| record.host.clone())
                    .unwrap_or_default();
                let now = ctx.input(|input| input.time);
                self.toast.show(toast::registered_text(&host), now);
            }
            SiteManagerOutcome::RegisterAndConnect(site) => {
                if let Some(area) = area {
                    self.open_site_tab(site, None, area);
                }
            }
        }
    }

    /// 워크스페이스와 그 탐색 상태(패널·탭·열거 스레드)를 함께 버린다
    fn remove_workspace(&mut self, index: usize) {
        let removed_id = self.workspaces.items().get(index).map(|item| item.id);
        if self.workspaces.remove(index).is_ok()
            && let Some(id) = removed_id
        {
            self.views.remove(&id);
            // 한 번도 열지 않은 워크스페이스였다면 불러온 상태가 여기 남아 있다
            self.restored.remove(&id);
        }
    }

    /// 활성 워크스페이스의 부제를 현재 폴더로 맞춘다 (FR-15 2줄 카드)
    fn sync_subtitle(&mut self) {
        let index = self.workspaces.active_index();
        let id = self.workspaces.active().id;
        let Some(dir) = self.views.get(&id).and_then(|v| v.active_dir()) else {
            return;
        };
        self.workspaces.set_subtitle(index, &dir);
    }

    /// 메뉴·단축키 명령 실행 (FR-12·FR-26).
    ///
    /// `target`은 명령이 향할 패널이다 — 패널 메뉴에서 온 명령은 **메뉴를 연 패널**을 담아
    /// 오고(plan D16), 단축키·타이틀바에서 온 명령은 `None`이라 활성 패널이 대상이 된다.
    /// `area`는 분할·닫기에 쓰이며 레이아웃을 그리기 전에 확정된 영역이어야 한다
    fn apply_command(
        &mut self,
        command: Command,
        target: Option<PanelId>,
        area: LayoutRect,
        ctx: &egui::Context,
    ) {
        match command {
            // 분할·닫기는 활성 워크스페이스의 뷰를 대상으로 한다(없으면 여기서 만들어진다)
            Command::Split(to) => {
                let (dir, place) = to.to_layout();
                let view = self.ensure_active_view();
                let panel = target.unwrap_or(view.active);
                view.split_panel(panel, dir, place, area);
            }
            Command::ClosePanel => {
                let view = self.ensure_active_view();
                let panel = target.unwrap_or(view.active);
                view.close_panel(panel, area);
            }
            Command::ToggleSidebar => self.sidebar_collapsed = !self.sidebar_collapsed,
            // 이 셋은 연결(`manager`)에 닿아야 해서 패널만 빌리는 아래 묶음에 들어갈 수 없다
            Command::OpenSiteTab(site) => self.open_site_tab(site, target, area),
            Command::Refresh => self.refresh_panel(target, ctx),
            Command::CloseTab => self.close_tab(target, ctx),
            Command::NewTab
            | Command::Back
            | Command::Forward
            | Command::Up
            | Command::NewFile
            | Command::NewFolder
            | Command::SetViewMode(_) => {
                let Some(panel) = self.command_panel_mut(target) else {
                    return;
                };
                match command {
                    Command::NewTab => panel.new_tab(ctx),
                    Command::Back => panel.go_back(ctx),
                    Command::Forward => panel.go_forward(ctx),
                    Command::Up => panel.go_up(ctx),
                    Command::NewFile => panel.new_file(ctx),
                    Command::NewFolder => panel.new_folder(ctx),
                    Command::SetViewMode(mode) => panel.set_view_mode(mode),
                    // 위 분기에서 걸러진 명령들 — 여기 오지 않는다
                    _ => {}
                }
            }
        }
    }

    /// 명령이 향하는 패널 — 대상이 지정되면 그 패널, 아니면 활성 패널
    fn command_panel_mut(&mut self, target: Option<PanelId>) -> Option<&mut PanelState> {
        let view = self.ensure_active_view();
        let id = target.unwrap_or(view.active);
        view.panels.get_mut(&id)
    }

    /// 보고 있는 위치를 다시 읽는다 — 원격 탭은 서버에 다시 묻고, 로컬 탭은 폴더를 다시 연다.
    ///
    /// `apply_command`의 다른 명령들과 갈라 둔 이유: 원격 요청은 연결(`manager`)이 필요한데
    /// `command_panel_mut`이 `self`를 통째로 빌려 그 안에서는 연결에 닿을 수 없다
    fn refresh_panel(&mut self, target: Option<PanelId>, ctx: &egui::Context) {
        self.ensure_active_view();
        let id = self.workspaces.active().id;
        let ExplorerApp { views, manager, .. } = self;
        let Some(view) = views.get_mut(&id) else {
            return;
        };
        let panel_id = target.unwrap_or(view.active);
        let Some(panel) = view.panels.get_mut(&panel_id) else {
            return;
        };
        if panel.is_remote() {
            panel.request_remote_list(manager);
        } else {
            panel.refresh(ctx);
        }
    }

    /// 활성 탭을 닫는다 — 그 탭이 패널의 마지막 원격 탭이었으면 연결도 함께 접는다 (FR-32)
    fn close_tab(&mut self, target: Option<PanelId>, ctx: &egui::Context) {
        let Some(panel) = self.command_panel_mut(target) else {
            return;
        };
        if let Some(conn) = panel.close_tab(ctx) {
            self.manager.close(conn);
        }
    }

    /// 연결 워커가 올린 소식을 화면에 반영한다 (NFR-10 — UI 스레드는 채널만 확인한다).
    ///
    /// **모든 워크스페이스의 패널**에 뿌린다. 이벤트는 한 번만 오므로 지금 보이지 않는
    /// 워크스페이스를 건너뛰면 그쪽 탭이 영영 옛 단계로 남는다
    fn poll_remote(&mut self, now: f64) {
        for (conn, event) in self.manager.poll() {
            match event {
                ConnEvent::Phase(phase) => {
                    let tab_phase = to_tab_phase(&phase);
                    for view in self.views.values_mut() {
                        for panel in view.panels.values_mut() {
                            panel.set_phase_for(conn, &tab_phase);
                        }
                    }
                    // 연결되면 곧바로 첫 목록을 청한다 — 그러지 않으면 연결만 되고 화면이 빈 채 남는다
                    if matches!(phase, ConnPhase::Ready) {
                        self.request_remote_list(conn);
                    }
                }
                ConnEvent::Listed {
                    generation,
                    path,
                    entries,
                } => {
                    let ExplorerApp { views, icons, .. } = self;
                    // 요청 하나에 답 하나다 — 받을 패널을 먼저 고르고 목록은 **복사 없이** 한 번만 넘긴다
                    let target = views
                        .values_mut()
                        .flat_map(|view| view.panels.values_mut())
                        .find(|panel| {
                            panel.active_conn() == Some(conn)
                                && panel.awaits_remote_list(generation, &path)
                        });
                    if let Some(panel) = target {
                        panel.apply_remote_listed(generation, &path, entries, icons);
                    }
                }
                // 전송 소식은 실행기가 큐에 반영한다 (FR-37)
                ConnEvent::TransferProgress { id, transferred } => {
                    self.runner
                        .on_progress(&mut self.queue, id, transferred, now)
                }
                ConnEvent::TransferDone { id, result } => {
                    self.runner
                        .on_done(&mut self.queue, id, result.map_err(|err| err.to_string()))
                }
                // 서버 로그는 `Connection`이 자기 버퍼에 이미 쌓는다(화면은 T20이 만든다).
                // 파일 작업 결과는 T23이 받는다
                _ => {}
            }
        }
    }

    /// 그 연결을 활성 탭으로 쓰는 패널들이 목록을 다시 청한다
    fn request_remote_list(&mut self, conn: ConnectionId) {
        let ExplorerApp { views, manager, .. } = self;
        for view in views.values_mut() {
            for panel in view.panels.values_mut() {
                if panel.active_conn() == Some(conn) {
                    panel.request_remote_list(manager);
                }
            }
        }
    }

    /// 원격 단계 화면에서 고른 조치를 실행한다 (인벤토리 #18~21)
    fn apply_remote_action(&mut self, target: PanelId, action: RemoteAction) {
        let Some(conn) = self
            .views
            .get(&self.workspaces.active().id)
            .and_then(|view| view.panels.get(&target))
            .and_then(|panel| panel.active_conn())
        else {
            return;
        };
        match action {
            // 워커는 살아 있고 명령만 다시 받는다 — 새 연결을 열면 탭이 옛 연결을 가리킨 채 남는다
            RemoteAction::Retry => {
                self.manager.send(conn, ConnCommand::Connect);
            }
            RemoteAction::CancelConnect => {
                self.manager.send(conn, ConnCommand::Disconnect);
            }
            // 방금 실패한 그 사이트를 고른 채 연다 — 고치러 온 사용자가 목록에서 다시 찾지
            // 않게 한다 (인벤토리 #19)
            RemoteAction::OpenSettings => {
                let site = self.manager.get(conn).map(|connection| connection.site);
                self.site_manager.open(&self.sites, site);
            }
            // 서버 로그 패널은 T20이 만든다 — 그때 이 자리에서 연다
            RemoteAction::ViewLog => {}
        }
    }

    /// 주소창에 적은 원격 주소로 새 탭을 연다 (FR-34).
    ///
    /// **이미 등록된 서버면 그 사이트를 쓴다** — 프로토콜·호스트·포트가 같으면 같은 서버이고,
    /// 그때마다 사이트를 새로 만들면 목록이 같은 서버로 뒤덮인다.
    /// 처음 보는 주소는 **숨긴 사이트**로 들인다: 연결에 필요한 설정(사용자·포트)을 담을 곳이
    /// 있어야 하지만, 한 번 적어 본 주소가 사이드바에 눌러앉지는 않게 한다(사이트 관리자에는 보인다)
    fn open_remote_url(&mut self, target: PanelId, url: RemoteUrl, area: LayoutRect) {
        let port = url.effective_port();
        let site = match matching_site(&self.sites, &url) {
            Some(site) => site,
            None => {
                let site = self.sites.add(&url.host);
                if let Some(record) = self.sites.get_mut(site) {
                    record.protocol = url.protocol;
                    record.host = url.host.clone();
                    record.port = port;
                    if let Some(user) = &url.user {
                        record.logon = LogonType::Normal;
                        record.user = user.clone();
                    }
                }
                // 주소로 한 번 열어 본 서버가 사이드바에 눌러앉지 않게 한다
                self.sites.hide(site);
                site
            }
        };
        self.open_site_tab_at(site, Some(target), url.path, area);
    }

    /// 사이트를 그 패널의 **새 원격 탭**으로 열고 연결을 건다 (FR-33·FR-34·FR-38).
    ///
    /// 진입점 셋(탭 스트립 드롭다운·주소창 URL·사이드바 드래그)이 모두 여기로 착지한다 —
    /// 여는 방법마다 다른 경로를 두면 셋이 조금씩 다르게 동작하게 된다.
    ///
    /// 사이트가 그 사이 지워졌으면 아무 일도 하지 않는다 (plan Edge Case: 드래그 도중 삭제)
    fn open_site_tab(&mut self, site: SiteId, target: Option<PanelId>, area: LayoutRect) {
        // 서버가 정한 홈에서 시작한다 — 연결이 서면 워커가 실제 위치를 알려 준다
        self.open_site_tab_at(site, target, RemotePath::root(), area);
    }

    /// 위와 같되 시작 위치를 지정한다 — 주소에 경로가 함께 적힌 경우(`sftp://host/pub`).
    ///
    /// **연결은 활성 패널을 좌우로 나눠 오른쪽에 연다** (FR-35·README) — 로컬과 원격을 나란히
    /// 두고 주고받는 것이 이 기능의 쓰임이라, 같은 패널에서 열면 그 배치를 사용자가 매번 손으로
    /// 만들어야 한다. **나눌 자리가 없으면 현재 패널의 새 탭**으로 물러선다 (Acceptance ④) —
    /// 조용히 아무 일도 일어나지 않으면 사용자는 연결 자체가 실패한 줄 안다
    fn open_site_tab_at(
        &mut self,
        site: SiteId,
        target: Option<PanelId>,
        path: RemotePath,
        area: LayoutRect,
    ) {
        if self.sites.get(site).is_none() {
            return;
        }
        let view = self.ensure_active_view();
        let source = target.unwrap_or(view.active);
        // 기존 분할 구조는 그대로 두고 대상 패널만 나눈다 (Acceptance ②)
        let opened = view
            .split_panel(source, SplitDir::Horizontal, SplitPlace::After, area)
            .unwrap_or(source);
        let Some(panel) = view.panels.get_mut(&opened) else {
            return;
        };
        panel.open_remote_tab(site, path);
        // 방금 만든 탭이 활성이라 연결이 그 탭에 붙는다
        self.connect_site(site);
    }

    /// 사이트에 연결하고 활성 원격 탭을 그 연결에 붙인다 (FR-28).
    ///
    /// **세션 조립이 화면 쪽에 있는 이유**: SFTP는 지문 확인 통로가 필요하고 그 통로는 화면이
    /// 쥔다 — 연결 관리자가 세션을 만들면 `remote`가 화면을 알아야 한다 (T4 결정).
    ///
    /// 사이트를 새 탭으로 여는 진입점(사이드바·주소창·드롭다운)은 T12·T13이 붙인다
    pub fn connect_site(&mut self, site: SiteId) -> Option<ConnectionId> {
        let record = self.sites.get(site)?.clone();
        // 익명 로그온이면 비밀번호가 없다 — 서버가 관례대로 무시한다
        let password = self.sites.password(site).unwrap_or_default();
        // 세션은 **껍데기만** 만들어 넘긴다 — 소켓도 지문 표도 워커 스레드가 연결할 때 연다
        // (AGENTS: UI 스레드에서 블로킹 I/O 금지)
        let session: Box<dyn RemoteSession> = if record.protocol.is_ssh() {
            Box::new(SftpSession::new(Some(
                self.hostkey.prompt(record.address()),
            )))
        } else {
            Box::new(FtpSession::new())
        };
        let id = self.manager.open(&record, password, session);
        let view = self.ensure_active_view();
        let active = view.active;
        if let Some(panel) = view.panels.get_mut(&active) {
            panel.attach_conn(id);
        }
        Some(id)
    }
}

/// 이 주소와 **같은 서버**로 이미 등록된 사이트 — 프로토콜·호스트·포트가 모두 같아야 한다.
///
/// 호스트 대소문자는 구분하지 않는다(DNS가 그렇다). 사용자 이름은 견주지 않는다 —
/// 같은 서버에 다른 계정으로 붙는 것은 흔하고, 그때마다 사이트를 새로 만들면 목록이 뒤덮인다
fn matching_site(sites: &SiteStore, url: &RemoteUrl) -> Option<SiteId> {
    let port = url.effective_port();
    sites
        .sites()
        .iter()
        .find(|record| {
            record.protocol == url.protocol
                && record.host.eq_ignore_ascii_case(&url.host)
                && record.port == port
        })
        .map(|record| record.id)
}

/// 연결 단계를 탭이 보이는 단계로 옮긴다.
///
/// 둘을 따로 두는 이유는 탭이 **연결 없이도** 존재하기 때문이다(빈 탭·세션 복원 직후) —
/// `Idle`·`Closed`는 "이 탭에는 지금 연결이 없다"와 같은 뜻이라 `New`로 모은다
fn to_tab_phase(phase: &ConnPhase) -> TabPhase {
    match phase {
        ConnPhase::Idle | ConnPhase::Closed => TabPhase::New,
        ConnPhase::Connecting => TabPhase::Connecting,
        ConnPhase::Ready => TabPhase::Ok,
        ConnPhase::Failed { detail } => TabPhase::Error {
            message: detail.clone(),
        },
    }
}

/// 두 사각형이 겹치는 넓이 — 닫힌 자리를 누가 이어받았는지 고르는 데 쓴다
fn overlap_area(a: LayoutRect, b: LayoutRect) -> i64 {
    let w = (a.x + a.w).min(b.x + b.w) - a.x.max(b.x);
    let h = (a.y + a.h).min(b.y + b.h) - a.y.max(b.y);
    if w <= 0 || h <= 0 {
        return 0;
    }
    w as i64 * h as i64
}

impl eframe::App for ExplorerApp {
    /// 창 클리어 색 — eframe 기본값은 하드코딩된 회색이라 팔레트와 어긋난다.
    /// 이것을 덮어써야 크기 조절 중 노출되는 여백까지 창 배경색으로 칠해진다
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        theme::WINDOW_BG.to_normalized_gamma_f32()
    }

    /// 종료 직전 — 지금 상태를 `%APPDATA%\FileExplorer\settings.json`에 저장한다 (FR-11·NFR-7).
    /// 저장 실패(디스크 풀·권한)는 조용히 넘어간다 — 종료를 막을 이유가 없다
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        // 진행 중이던 전송을 대기로 되돌린다 — 저장된 큐가 "전송 중"이라 주장하면
        // 다음 실행의 화면이 실제로는 아무것도 돌지 않는데 진행 중으로 보인다 (T18)
        self.runner.shutdown(&mut self.queue);
        save_session(&self.collect_session());
    }

    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.track_window(ctx);
        self.textures.begin_frame();
        // 연결 소식은 **워크스페이스와 무관하게** 받는다 — 채널에 쌓인 것을 건너뛰면
        // 보이지 않는 워크스페이스의 원격 탭이 옛 단계로 굳는다
        let now = ctx.input(|input| input.time);
        self.poll_remote(now);
        // 자리가 나면 대기 중인 전송을 워커에 맡긴다 (FR-37)
        self.runner
            .start_ready(&mut self.queue, &self.manager, &self.sites, now);
        // 화면에 없는 워크스페이스는 폴링하지 않는다 — 전환하면 그때 밀린 결과가 반영된다
        let id = self.workspaces.active().id;
        if let Some(view) = self.views.get_mut(&id) {
            // 패널은 서로 독립이라 각자 자기 열거 결과만 처리한다
            for panel in view.panels.values_mut() {
                panel.poll(ctx, &mut self.icons);
            }
        }
        self.sync_subtitle();
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        let mut menu = None;
        let mut panel_command = None;
        let mut remote_action = None;
        let mut remote_url = None;
        let mut closed_conns = Vec::new();
        // 사이트 관리자의 `연결(C)`이 쓸 분할 영역 — 모달은 CentralPanel 밖에서 그리므로
        // 안에서 정해지는 이 값을 밖으로 들고 나온다
        let mut layout_area = None;
        // 타이틀바를 먼저 그린다 — 남는 영역이 아래 CentralPanel의 몫이 된다 (FR-22)
        let titlebar_command = self.show_titlebar(ui, &ctx);
        // eframe이 주는 Ui는 여백·배경이 없다 — CentralPanel로 감싸야 panel_fill이 칠해진다
        egui::CentralPanel::default().show(ui, |ui| {
            if !self.korean_font {
                ui.colored_label(
                    theme::TEXT_DIM,
                    "한글 글꼴을 불러오지 못해 기본 글꼴로 표시합니다",
                );
            }
            if !self.shell_available() {
                ui.colored_label(theme::TEXT_DIM, SHELL_UNAVAILABLE);
            }
            let mut sidebar_actions = Vec::new();
            if !self.sidebar_collapsed {
                // 연결 상태를 먼저 모은다 — 아래 클로저가 `self`를 통째로 빌린다
                let connected = self.connected_sites();
                // 사이드바가 자기 배경·여백을 직접 그리므로 egui 기본 프레임은 끈다
                let panel = egui::Panel::left(egui::Id::new("workspace_sidebar"))
                    .resizable(true)
                    .default_size(self.sidebar_width)
                    .size_range(egui::Rangef::new(
                        SIDEBAR_MIN_WIDTH as f32,
                        SIDEBAR_MAX_WIDTH as f32,
                    ))
                    .frame(egui::Frame::NONE)
                    .show(ui, |ui| {
                        self.sidebar.show(
                            ui,
                            &self.workspaces,
                            &self.sites,
                            &connected,
                            &mut self.icons,
                            &mut self.textures,
                        )
                    });
                self.sidebar_width = panel.response.rect.width();
                // 조작은 모아 두었다가 아래에서 처리한다 — 연결은 분할 영역을 알아야 하는데
                // 그 영역은 **사이드바를 뺀 나머지**라 여기서는 아직 정해지지 않았다
                sidebar_actions = panel.inner;
            }

            self.show_dock(ui);
            let area = splitter::to_layout_rect(ui.available_rect_before_wrap());
            layout_area = Some(area);
            for action in sidebar_actions {
                self.handle_sidebar(action, area);
            }
            // 단축키는 프레임당 한 번만 소비한다(`consume_shortcut`이 입력을 소모한다).
            // 메뉴와 단축키가 같은 프레임에 겹쳐도 둘 다 실행한다
            // 모달이 떠 있는 동안에는 단축키를 받지 않는다 — 모달은 입력을 막는다는 뜻이다
            // (워크스페이스 삭제 확인 · 서버 지문 확인)
            let shortcut_command = if self.pending_remove.is_some()
                || self.hostkey.is_open()
                || self.site_manager.is_open()
            {
                None
            } else {
                menu::poll_shortcuts(&ctx)
            };
            // 단축키·타이틀바 명령은 대상을 지정하지 않는다 — 활성 패널에 적용된다
            for command in shortcut_command.into_iter().chain(titlebar_command) {
                self.apply_command(command, None, area, &ctx);
            }
            // 확보와 사용을 나눈다 — 아래 호출이 `views`와 `icons`·`textures`를 동시에 빌린다
            let id = self.workspaces.active().id;
            // 탭 배지·드롭다운이 상태 점을 그리는 데 쓴다 — 빌림이 겹치지 않게 미리 모은다
            let connected = self.connected_sites();
            self.ensure_active_view();
            if let Some(view) = self.views.get_mut(&id) {
                let outcome = splitter::show_layout(
                    ui,
                    &ctx,
                    &mut view.layout,
                    &mut view.panels,
                    &mut view.active,
                    &mut self.icons,
                    &mut self.textures,
                    RemoteView {
                        sites: &self.sites,
                        connected: &connected,
                    },
                );
                menu = outcome.menu;
                panel_command = outcome.command;
                remote_action = outcome.remote;
                remote_url = outcome.remote_url;
                closed_conns = outcome.closed_conns;
            }
            // 패널 메뉴 명령은 그리기가 끝난 뒤에 실행한다 — 분할·닫기는 트리를 바꾸므로
            // 이번 프레임의 배치와 어긋나고, `apply_command`가 앱 전체를 빌려야 한다.
            // 대상은 메뉴를 연 패널이지 활성 패널이 아니다 (D16)
            if let Some((target, command)) = panel_command {
                self.apply_command(command, Some(target), area, &ctx);
            }
            if let Some((target, action)) = remote_action {
                self.apply_remote_action(target, action);
            }
            if let Some((target, url)) = remote_url {
                self.open_remote_url(target, url, area);
            }
            // 마지막 원격 탭이 닫힌 연결을 접는다 — 워커와 소켓이 여기서 회수된다 (FR-32)
            for conn in closed_conns {
                self.manager.close(conn);
            }
        });

        // 삭제 확인은 egui 모달이라 `CentralPanel` 밖에서 그려도 된다(자체 레이어를 쓴다)
        self.show_remove_confirm(&ctx);
        // 서버 지문 확인도 같다. 사용자가 고를 때까지 그 연결의 워커는 기다리고 있다 (D15)
        self.hostkey.show(&ctx);
        // 사이트 관리자도 자체 레이어를 쓴다 (FR-27)
        self.show_site_manager(&ctx, layout_area);
        // 알림은 모든 것 위에 뜬다 — 대화가 닫힌 뒤에도 남아 있어야 한다 (FR-43)
        self.toast.show_ui(&ctx);
        // 로그 복사는 그리기가 끝난 뒤에 보낸다 (`⧉` — FR-40)
        if let Some(text) = self.pending_clipboard.take() {
            ctx.copy_text(text);
        }

        // 셸 메뉴는 그리기가 **모두 끝난 뒤** 띄운다 — TrackPopupMenuEx가 자체 메시지 루프를
        // 돌려 이벤트 루프를 재진입시키므로, 위젯 트리가 절반만 구성된 상태로 들어가면 안 된다
        if let (Some(menu), Some(shell)) = (menu, self.shell.as_ref()) {
            // egui 좌표는 논리 포인트라 물리 픽셀로 되돌린 뒤 화면 좌표로 바꾼다
            let scale = ctx.pixels_per_point();
            let (x, y) = shell.to_screen((menu.pos.x * scale) as i32, (menu.pos.y * scale) as i32);
            shell.popup(&menu.folder, &menu.items, x, y);
        }
    }
}

/// 시작 폴더 — 인자로 폴더를 받으면 그곳에서, 없으면 홈 폴더에서 시작한다
/// (탐색기의 "여기서 열기"처럼 쓰이며, 대량 폴더 성능 측정에도 이 경로를 쓴다)
fn start_dir() -> PathBuf {
    let from_arg = std::env::args().nth(1).filter(|a| !a.starts_with("--"));
    if let Some(arg) = from_arg {
        let path = PathBuf::from(arg);
        if path.is_dir() {
            return path;
        }
    }
    std::env::var("USERPROFILE")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(r"C:\"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn rect(x: i32, y: i32, w: i32, h: i32) -> LayoutRect {
        LayoutRect { x, y, w, h }
    }

    #[test]
    fn 닫기는_전달받은_패널을_대상으로_한다() {
        // 패널 메뉴 팝업은 자기 패널 밖으로 뻗을 수 있고, 그 위에서 고르면 아래 깔린 패널이
        // 활성이 된다(활성 판정이 포인터 위치 기반). 그 상태로 활성 패널을 닫으면 사용자가
        // 누른 것과 다른 패널이 사라진다 — 이 테스트가 그것을 막는다 (D16)
        let area = rect(0, 0, 1200, 800);
        let mut view = WorkspaceView::new(PathBuf::from(r"C:\"));
        let left = view.active;
        let right = view
            .layout
            .split(left, SplitDir::Horizontal, SplitPlace::After, area)
            .unwrap();
        view.panels
            .insert(right, PanelState::new(PathBuf::from(r"D:\")));
        // 활성은 왼쪽인데, 닫으라고 지시받은 것은 오른쪽이다
        view.active = left;

        view.close_panel(right, area);

        assert_eq!(view.layout.panel_count(), 1);
        assert!(
            view.layout.panel_ids().contains(&left),
            "대상이 아닌 활성 패널이 닫혔다"
        );
        assert!(!view.panels.contains_key(&right), "닫힌 패널 상태가 남았다");
    }

    #[test]
    fn 겹치지_않으면_0이다() {
        assert_eq!(overlap_area(rect(0, 0, 10, 10), rect(20, 20, 10, 10)), 0);
        // 변끼리 맞닿기만 한 경우도 넓이는 0
        assert_eq!(overlap_area(rect(0, 0, 10, 10), rect(10, 0, 10, 10)), 0);
    }

    #[test]
    fn 부분_겹침은_교집합_넓이다() {
        assert_eq!(overlap_area(rect(0, 0, 10, 10), rect(5, 5, 10, 10)), 25);
    }

    #[test]
    fn 닫힌_자리를_더_많이_덮는_쪽이_크다() {
        // 닫힌 패널이 오른쪽 절반이었다면, 그 자리를 흡수한 패널의 겹침이 더 커야 한다
        let closed = rect(500, 0, 500, 600);
        let absorbed = rect(0, 0, 1000, 600);
        let far = rect(0, 0, 200, 600);
        assert!(overlap_area(absorbed, closed) > overlap_area(far, closed));
    }

    #[test]
    fn 분할은_전달받은_패널을_대상으로_한다() {
        // 패널 메뉴는 패널마다 있어 활성 패널이 아닌 곳에서도 열린다 (D3·D16).
        // 활성 패널을 대상으로 삼으면 엉뚱한 패널이 나뉘므로 이 테스트가 그것을 막는다
        let area = rect(0, 0, 1200, 800);
        let mut view = WorkspaceView::new(PathBuf::from(r"C:\"));
        let left = view.active;
        let right = view
            .layout
            .split(left, SplitDir::Horizontal, SplitPlace::After, area)
            .unwrap();
        view.panels
            .insert(right, PanelState::new(PathBuf::from(r"D:\")));
        view.active = left;

        let left_before = pane_rect(&view, left, area);
        view.split_panel(right, SplitDir::Vertical, SplitPlace::After, area);

        assert_eq!(view.layout.panel_count(), 3);
        assert_eq!(
            pane_rect(&view, left, area),
            left_before,
            "대상이 아닌 패널(활성 패널)의 자리는 그대로여야 한다"
        );
        // 분할하면 새로 생긴 패널이 활성이 된다 — 이어서 조작할 곳이 거기이기 때문
        let added = view
            .layout
            .panel_ids()
            .into_iter()
            .find(|id| *id != left && *id != right)
            .expect("새 패널이 트리에 있어야 한다");
        assert_eq!(view.active, added);
    }

    /// 지정 패널의 현재 사각형 — 어느 패널이 나뉘었는지 자리로 판별한다
    fn pane_rect(view: &WorkspaceView, id: PanelId, area: LayoutRect) -> LayoutRect {
        view.layout
            .compute_rects(area)
            .panes
            .iter()
            .find(|(pane_id, _)| *pane_id == id)
            .map(|(_, r)| *r)
            .expect("패널이 트리에 있어야 한다")
    }

    #[test]
    fn 앞에_둔_분할도_세션_왕복에서_탭이_제자리에_남는다() {
        // 세션은 패널 배열을 리프 walk 순서로 짝지으므로, 새 리프가 앞에 들어가도
        // 저장·복원에서 짝이 밀리면 안 된다 (왼쪽·위쪽 분할의 회귀 방지)
        let area = rect(0, 0, 1200, 800);
        let mut view = WorkspaceView::new(PathBuf::from(r"C:\"));
        let kept = view.active;
        let added = view
            .layout
            .split(kept, SplitDir::Horizontal, SplitPlace::Before, area)
            .unwrap();
        view.panels
            .insert(added, PanelState::new(PathBuf::from(r"D:\")));

        let state = view.to_state("워크스페이스 1".into());
        assert_eq!(
            state.panels[0].tabs,
            vec![PathBuf::from(r"D:\")],
            "walk 순서상 앞에 온 새 패널이 첫 번째로 저장된다"
        );
        assert_eq!(state.panels[1].tabs, vec![PathBuf::from(r"C:\")]);

        let restored = WorkspaceView::from_state(&state);
        let ids = restored.layout.panel_ids();
        assert_eq!(restored.panels[&ids[0]].dir(), Path::new(r"D:\"));
        assert_eq!(restored.panels[&ids[1]].dir(), Path::new(r"C:\"));
    }

    #[test]
    fn 연결하면_활성_패널이_오른쪽으로_나뉜다() {
        // Acceptance ①③ — 로컬과 원격을 나란히 두는 것이 이 기능의 쓰임이다 (FR-35)
        let area = rect(0, 0, 1200, 800);
        let mut view = WorkspaceView::new(PathBuf::from(r"C:\"));
        let source = view.active;
        let before = pane_rect(&view, source, area);

        let opened = view
            .split_panel(source, SplitDir::Horizontal, SplitPlace::After, area)
            .expect("공간이 넉넉하면 나뉜다");

        assert_eq!(view.layout.panel_count(), 2, "패널 수가 늘지 않았다");
        assert_eq!(view.active, opened, "새 패널이 활성이 아니다");
        // 새 패널은 **원래 패널의 오른쪽**에 온다
        let left = pane_rect(&view, source, area);
        let right = pane_rect(&view, opened, area);
        assert!(left.x < right.x, "새 패널이 왼쪽에 생겼다");
        assert_eq!(left.x, before.x, "원래 패널이 옮겨졌다");
    }

    #[test]
    fn 연결해도_기존_분할_구조는_그대로다() {
        // Acceptance ② — 4분할 상태에서 연결해도 나머지 패널의 자리가 흔들리면 안 된다 (FR-1 회귀)
        let area = rect(0, 0, 1600, 900);
        let mut view = WorkspaceView::new(PathBuf::from(r"C:\"));
        let first = view.active;
        let second = view
            .split_panel(first, SplitDir::Horizontal, SplitPlace::After, area)
            .expect("분할 1");
        let third = view
            .split_panel(second, SplitDir::Vertical, SplitPlace::After, area)
            .expect("분할 2");
        let fourth = view
            .split_panel(first, SplitDir::Vertical, SplitPlace::After, area)
            .expect("분할 3");
        assert_eq!(view.layout.panel_count(), 4, "4분할 상태를 만들지 못했다");
        let untouched: Vec<(PanelId, LayoutRect)> = [first, third, fourth]
            .into_iter()
            .map(|id| (id, pane_rect(&view, id, area)))
            .collect();

        // 이 4분할 상태에서 두 번째 패널을 대상으로 연결한다
        view.split_panel(second, SplitDir::Horizontal, SplitPlace::After, area)
            .expect("연결 분할");

        assert_eq!(view.layout.panel_count(), 5);
        for (id, before) in untouched {
            assert_eq!(
                pane_rect(&view, id, area),
                before,
                "대상이 아닌 패널의 자리가 바뀌었다"
            );
        }
    }

    #[test]
    fn 나눌_자리가_없으면_분할하지_않는다() {
        // Acceptance ④ — 이때 호출부는 현재 패널의 새 탭으로 물러선다
        let tiny = rect(0, 0, crate::app::layout::MIN_PANE_SIZE, 400);
        let mut view = WorkspaceView::new(PathBuf::from(r"C:\"));
        let source = view.active;

        assert_eq!(
            view.split_panel(source, SplitDir::Horizontal, SplitPlace::After, tiny),
            None,
            "좁은 화면에서 분할이 섰다"
        );
        assert_eq!(view.layout.panel_count(), 1, "패널이 늘었다");
        assert_eq!(view.active, source, "활성 패널이 바뀌었다");
    }

    #[test]
    fn 같은_서버의_주소는_사이트를_새로_만들지_않는다() {
        // 주소창으로 같은 서버를 여러 번 열어도 사이트 목록이 그 서버로 뒤덮이면 안 된다
        use crate::remote::types::Protocol;
        use crate::remote::url::parse_remote_url;

        let mut sites = SiteStore::new();
        let id = sites.add("배포 서버");
        if let Some(record) = sites.get_mut(id) {
            record.protocol = Protocol::Sftp;
            record.host = "example.test".to_owned();
            record.port = 22;
        }

        let 같은_서버 = parse_remote_url("sftp://example.test/pub").expect("파싱");
        assert_eq!(matching_site(&sites, &같은_서버), Some(id));
        // 호스트 대소문자는 구분하지 않는다
        let 대문자 = parse_remote_url("sftp://EXAMPLE.test").expect("파싱");
        assert_eq!(matching_site(&sites, &대문자), Some(id));
        // 계정이 달라도 같은 서버다
        let 다른_계정 = parse_remote_url("sftp://other@example.test").expect("파싱");
        assert_eq!(matching_site(&sites, &다른_계정), Some(id));

        // 포트·프로토콜·호스트가 다르면 다른 서버다
        for 다른 in [
            "sftp://example.test:2222",
            "ftp://example.test",
            "sftp://other.test",
        ] {
            let url = parse_remote_url(다른).expect("파싱");
            assert_eq!(matching_site(&sites, &url), None, "{다른}");
        }
    }

    #[test]
    fn 연결_단계는_탭_단계로_옮겨진다() {
        // 탭은 연결 없이도 존재하므로 둘을 따로 둔다 — `Idle`·`Closed`는 "이 탭에 연결이 없다"와
        // 같은 뜻이라 `New`로 모인다. 실패는 **사유를 잃지 않아야** 실패 화면이 그것을 보인다
        assert_eq!(to_tab_phase(&ConnPhase::Idle), TabPhase::New);
        assert_eq!(to_tab_phase(&ConnPhase::Closed), TabPhase::New);
        assert_eq!(to_tab_phase(&ConnPhase::Connecting), TabPhase::Connecting);
        assert_eq!(to_tab_phase(&ConnPhase::Ready), TabPhase::Ok);
        assert_eq!(
            to_tab_phase(&ConnPhase::Failed {
                detail: "530 Login incorrect".to_owned()
            }),
            TabPhase::Error {
                message: "530 Login incorrect".to_owned()
            }
        );
    }

    #[test]
    fn 워크스페이스_뷰는_패널_하나로_시작한다() {
        let view = WorkspaceView::new(PathBuf::from(r"C:\"));
        assert_eq!(view.layout.panel_count(), 1);
        assert_eq!(view.panels.len(), 1);
        assert_eq!(view.active_dir(), Some(PathBuf::from(r"C:\")));
    }
}
