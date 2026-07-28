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
use crate::ui::icon_tex::IconTextures;
use crate::ui::menu::{self, Command, MenuState};
use crate::ui::panel::PanelState;
use crate::ui::session::{self, PanelTabs, WorkspaceState};
use crate::ui::shell_host::ShellHost;
use crate::ui::sidebar::{SidebarAction, WorkspaceSidebar};
use crate::ui::splitter;
use crate::ui::theme;
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

/// 한글 폰트를 egui에 등록한다. 폰트 파일이 없으면 기본 폰트로 진행한다(반환 false)
pub fn install_korean_font(ctx: &egui::Context) -> bool {
    let Ok(bytes) = std::fs::read(KOREAN_FONT_PATH) else {
        return false;
    };
    let mut fonts = egui::FontDefinitions::default();
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
    ctx.set_fonts(fonts);
    true
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

    /// 활성 패널을 네 방향 중 한쪽으로 나눈다 — 메뉴·단축키의 대상은 언제나 활성 패널이다
    fn split_active(&mut self, dir: SplitDir, place: SplitPlace, area: LayoutRect) {
        self.split_panel(self.active, dir, place, area);
    }

    /// 지정한 패널을 나눈다. 새 패널은 원래 패널과 같은 폴더에서 시작한다.
    ///
    /// 패널마다 있는 분할 버튼은 **자기 패널**을 대상으로 하므로 활성 패널과 다를 수 있다 (D3)
    fn split_panel(&mut self, target: PanelId, dir: SplitDir, place: SplitPlace, area: LayoutRect) {
        let start = self
            .panels
            .get(&target)
            .map(|p| p.dir().to_path_buf())
            .unwrap_or_else(start_dir);
        // 공간이 부족하면 나눌 수 없다 — 조용히 무시한다(사용자는 창을 키우면 된다)
        if let Ok(new_id) = self.layout.split(target, dir, place, area) {
            self.panels.insert(new_id, PanelState::new(start));
            self.active = new_id;
        }
    }

    /// 활성 패널을 닫는다. 마지막 하나는 닫히지 않는다 (FR-2).
    ///
    /// 닫은 **자리를 흡수한 패널**을 다음 활성으로 삼는다 — 트리 순서상 첫 패널을 고르면
    /// 포커스가 화면 반대편으로 튀어 방금 조작한 위치와 멀어진다
    fn close_active(&mut self, area: LayoutRect) {
        let closed = self
            .layout
            .compute_rects(area)
            .panes
            .iter()
            .find(|(id, _)| *id == self.active)
            .map(|(_, rect)| *rect);
        if self.layout.close(self.active).is_err() {
            return;
        }
        self.panels.remove(&self.active);
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
            let panel = PanelState::from_tabs(tabs.tabs.clone(), tabs.active_tab)
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
                },
                // 트리에는 있는데 상태가 없는 리프 — 분할 직후가 아니면 생기지 않는다
                None => PanelTabs {
                    tabs: vec![start_dir()],
                    active_tab: 0,
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
}

impl ExplorerApp {
    /// eframe 창 생성 직후 호출된다 — 폰트·팔레트·셸 호스트를 이 시점에 준비한다.
    /// `session`이 있으면 지난 실행의 워크스페이스·사이드바·창 상태를 되살린다 (FR-11·FR-20)
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        com: ComStatus,
        session: Option<Session>,
    ) -> ExplorerApp {
        let korean_font = install_korean_font(&cc.egui_ctx);
        theme::apply_dark(&cc.egui_ctx);
        // HWND 획득·서브클래스 설치는 창이 만들어진 이 시점에만 가능하다
        let shell = ShellHost::new(cc);
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
    fn handle_sidebar(&mut self, action: SidebarAction) {
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
            SidebarAction::ToggleCollapse => {
                self.sidebar_collapsed = !self.sidebar_collapsed;
            }
        }
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

    /// 활성 워크스페이스의 패널 수 — 아직 열지 않았으면 기본 1개 구성이 될 자리다
    fn active_panel_count(&self) -> usize {
        self.views
            .get(&self.workspaces.active().id)
            .map(|v| v.layout.panel_count())
            .unwrap_or(1)
    }

    /// 명령이 향하는 패널 — 활성 워크스페이스의 활성 패널
    fn active_panel_mut(&mut self) -> Option<&mut PanelState> {
        let view = self.ensure_active_view();
        view.panels.get_mut(&view.active)
    }

    /// 메뉴·단축키 명령 실행 (FR-12).
    /// `area`는 분할에 쓰이며, 메뉴 줄을 그린 **뒤에** 확정된 영역이어야 한다
    fn apply_command(&mut self, command: Command, area: LayoutRect, ctx: &egui::Context) {
        match command {
            // 분할·닫기는 활성 워크스페이스의 뷰를 대상으로 한다(없으면 여기서 만들어진다)
            Command::Split(to) => {
                let (dir, place) = to.to_layout();
                self.ensure_active_view().split_active(dir, place, area);
            }
            Command::ClosePanel => self.ensure_active_view().close_active(area),
            Command::ToggleSidebar => self.sidebar_collapsed = !self.sidebar_collapsed,
            Command::NewWorkspace => {
                self.workspaces.add();
                // 접혀 있으면 편집칸이 보이지 않는다 — 사이드바가 그려져야 편집이 시작된다
                self.sidebar_collapsed = false;
                // 사이드바의 `+`와 같은 흐름으로 잇는다 — 추가 직후 이름을 고칠 수 있어야 한다
                self.sidebar.edit_after_add();
            }
            Command::RenameWorkspace => {
                // 접혀 있으면 편집칸이 보이지 않으므로 함께 펼친다
                self.sidebar_collapsed = false;
                self.sidebar
                    .start_rename(self.workspaces.active_index(), &self.workspaces);
            }
            Command::RemoveWorkspace => {
                // 사이드바 컨텍스트 메뉴와 같은 경로 — 확인 대화를 거친다
                self.pending_remove = Some(self.workspaces.active().id);
            }
            Command::NewTab
            | Command::CloseTab
            | Command::Back
            | Command::Forward
            | Command::Up
            | Command::Refresh
            | Command::ToggleTree => {
                let Some(panel) = self.active_panel_mut() else {
                    return;
                };
                match command {
                    Command::NewTab => panel.new_tab(ctx),
                    Command::CloseTab => panel.close_tab(ctx),
                    Command::Back => panel.go_back(ctx),
                    Command::Forward => panel.go_forward(ctx),
                    Command::Up => panel.go_up(ctx),
                    Command::Refresh => panel.refresh(ctx),
                    Command::ToggleTree => panel.toggle_tree(),
                    // 위 분기에서 걸러진 명령들 — 여기 오지 않는다
                    _ => {}
                }
            }
        }
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
        save_session(&self.collect_session());
    }

    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.track_window(ctx);
        self.textures.begin_frame();
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
            // 메뉴 줄·구분선을 먼저 그린 **뒤** 남는 영역이 실제 분할 대상이다.
            // 그리기 전 영역으로 판정하면 최소 패널 크기 검사가 메뉴 줄 높이만큼 느슨해진다
            let menu_command = menu::show_menu_bar(
                ui,
                MenuState {
                    can_close_panel: self.active_panel_count() > 1,
                    can_remove_workspace: self.workspaces.len() > 1,
                },
            );
            ui.separator();

            if !self.sidebar_collapsed {
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
                        self.sidebar
                            .show(ui, &self.workspaces, &mut self.icons, &mut self.textures)
                    });
                self.sidebar_width = panel.response.rect.width();
                for action in panel.inner {
                    self.handle_sidebar(action);
                }
            }

            let area = splitter::to_layout_rect(ui.available_rect_before_wrap());
            // 단축키는 프레임당 한 번만 소비한다(`consume_shortcut`이 입력을 소모한다).
            // 메뉴와 단축키가 같은 프레임에 겹쳐도 둘 다 실행한다
            // 삭제를 묻는 동안에는 단축키를 받지 않는다 — 모달은 입력을 막는다는 뜻이다
            let shortcut_command = if self.pending_remove.is_some() {
                None
            } else {
                menu::poll_shortcuts(&ctx)
            };
            for command in menu_command.into_iter().chain(shortcut_command) {
                self.apply_command(command, area, &ctx);
            }
            // 확보와 사용을 나눈다 — 아래 호출이 `views`와 `icons`·`textures`를 동시에 빌린다
            let id = self.workspaces.active().id;
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
                );
                menu = outcome.menu;
                // 분할은 그리기가 끝난 뒤에 한다 — 트리를 바꾸면 이번 프레임의 배치와 어긋난다.
                // 대상은 버튼이 속한 패널이지 활성 패널이 아니다 (D3)
                if let Some((target, to)) = outcome.split {
                    let (dir, place) = to.to_layout();
                    view.split_panel(target, dir, place, area);
                }
            }
        });

        // 삭제 확인은 egui 모달이라 `CentralPanel` 밖에서 그려도 된다(자체 레이어를 쓴다)
        self.show_remove_confirm(&ctx);

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
        // 분할 버튼은 패널마다 있어 활성 패널이 아닌 곳에서도 눌린다 (D3).
        // `split_active`를 그대로 쓰면 엉뚱한 패널이 나뉘므로 이 테스트가 그것을 막는다
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
    fn 워크스페이스_뷰는_패널_하나로_시작한다() {
        let view = WorkspaceView::new(PathBuf::from(r"C:\"));
        assert_eq!(view.layout.panel_count(), 1);
        assert_eq!(view.panels.len(), 1);
        assert_eq!(view.active_dir(), Some(PathBuf::from(r"C:\")));
    }
}
