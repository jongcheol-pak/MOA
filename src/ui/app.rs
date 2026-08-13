//! egui 앱 골격 — 창·폰트·팔레트·COM·셸 호스트와 전역 공유 자원을 보유한다.
//!
//! 실제 탐색은 `ui::panel::PanelState`가 담당하고, 그 패널들을 담은 분할 화면 한 벌이
//! `WorkspaceView`다. 이 구조체는 워크스페이스 목록(사이드바)과 뷰들을 잇는 그릇이다.
use crate::app::layout::TreeShape;
use crate::app::layout::{LayoutTree, PanelId, Rect as LayoutRect, SplitDir, SplitPlace};
use crate::app::settings::{
    AppSettings, SIDEBAR_DEFAULT_WIDTH, SIDEBAR_MAX_WIDTH, SIDEBAR_MIN_WIDTH, Session,
    SidebarSession, WindowState, save_session,
};
use crate::app::workspace::{WorkspaceId, WorkspaceList};
use crate::fs::icons::IconCache;
use crate::panel::tabs::TabPhase;
use crate::remote::connection::{
    ConnCommand, ConnEvent, ConnPhase, ConnectionId, OpKind, TransferDirection,
};
use crate::remote::ftp::FtpSession;
use crate::remote::log::{LogBuffer, LogKind};
use crate::remote::manager::ConnectionManager;
use crate::remote::queue::TransferQueue;
use crate::remote::sftp::SftpSession;
use crate::remote::sites::SiteStore;
use crate::remote::transfer::{self, TransferRunner};
use crate::remote::tree_cache::TreeCache;
use crate::remote::types::{LogonType, RemoteError, RemotePath, RemoteSession, SiteId};
use crate::remote::url::RemoteUrl;
use crate::ui::app_icon;
use crate::ui::dock::{self, DockAction, DockPanel, DockState, DockView};
use crate::ui::font_scan::FontScan;
use crate::ui::icon_tex::IconTextures;
use crate::ui::list_common::{self, DragItem, DropOutcome, DropTarget};
use crate::ui::log_panel;
use crate::ui::menu::{self, Command};
use crate::ui::panel::{PanelState, RemoteAction};
use crate::ui::queue_panel::{self, QueueAction};
use crate::ui::remote_menu::{self, DialogOutcome, Permissions, RemoteMenuAction, RemoteTarget};
use crate::ui::remote_states::{HostKeyGate, RemoteView};
use crate::ui::session::{self, PanelTabs, RemoteSnapshot, TabSpec, WorkspaceState};
use crate::ui::settings_dialog::{FontChoices, SettingsDialog};
use crate::ui::shell_host::ShellHost;
use crate::ui::sidebar::{SidebarAction, WorkspaceSidebar};
use crate::ui::site_manager::{SiteManager, SiteManagerOutcome};
use crate::ui::splitter;
use crate::ui::status_bar::{self, StatusAction, StatusView};
use crate::ui::theme;
use crate::ui::titlebar::{self, WindowRequest};
use crate::ui::toast::{self, Toast};
use crate::ui::tray::{Tray, TrayEvent};
use crate::ui::tree::TreeRequest;
use eframe::egui;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use windows::Win32::Foundation::RPC_E_CHANGED_MODE;
use windows::Win32::System::Com::{COINIT_APARTMENTTHREADED, CoInitializeEx, CoUninitialize};

/// 맑은 고딕 — egui 기본 폰트에는 한글 글리프가 없어 파일명이 두부(□)로 보인다
const KOREAN_FONT_PATH: &str = r"C:\Windows\Fonts\malgun.ttf";
/// 타이틀바 앱 아이콘으로 올릴 원본 크기(px) — 20px 자리에 그리므로 고배율 화면에서도
/// 늘어나지 않게 한 단계 큰 항목을 쓴다
const APP_ICON_TEXTURE_PX: u32 = 48;

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

/// 세션의 최대화 상태를 되살릴 때 명령을 다시 걸어 볼 프레임 수.
///
/// 보통 한두 프레임이면 걸린다. 그럼에도 세는 것은 명령이 먹지 않는 환경에서 **영영
/// 창 크기를 저장하지 못하는 상태**로 남지 않기 위함이다 — 다 쓰면 평소 추적으로 돌아간다
const MAXIMIZE_RETRY_FRAMES: u8 = 30;

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

/// 앱이 쓰는 글꼴을 등록한다 — 한글 본문 글꼴과 아이콘 글꼴(Phosphor).
///
/// `family`가 있으면 그 글꼴을, 없거나 읽지 못하면 **맑은 고딕**을 쓴다 (FR-48).
/// 사용자가 고른 글꼴이 나중에 지워졌을 때 화면이 두부(□)로 덮이지 않게 하는 폴백이며,
/// **설정 값은 건드리지 않는다** — 글꼴을 다시 설치하면 되살아나야 한다.
///
/// 반환값은 **한글 글꼴 적용 여부**다. 어느 것도 읽지 못해도 아이콘 글꼴은 등록되므로
/// 타이틀바 버튼은 그대로 보인다(파일명만 기본 글꼴로 표시된다).
/// 등록은 한 번에 끝낸다 — `set_fonts`를 두 번 부르면 뒤엣것이 앞엣것을 덮어쓴다.
///
/// **반영은 다음 pass부터다**(egui `Context::set_fonts` 문서) — 부르는 쪽이 그 프레임을
/// 보장하려면 `ctx.request_repaint()`를 함께 부른다
pub fn install_fonts(ctx: &egui::Context, family: Option<&str>) -> bool {
    let mut fonts = egui::FontDefinitions::default();
    // 고른 글꼴은 face 인덱스까지 함께 온다 — 모음 글꼴(굴림·바탕 등)은 파일 하나에
    // 여러 글꼴이 들어 있어 인덱스가 없으면 언제나 첫 번째만 나온다
    let picked = family.and_then(crate::app::fonts::load_font);
    let loaded = match picked {
        Some(font) => Some((font.bytes, font.index)),
        // 기본 글꼴(맑은 고딕)은 단일 파일이라 인덱스가 0이다
        None => std::fs::read(KOREAN_FONT_PATH).ok().map(|bytes| (bytes, 0)),
    };
    let korean = match loaded {
        Some((bytes, index)) => {
            fonts.font_data.insert(
                "malgun".to_owned(),
                Arc::new(egui::FontData {
                    font: bytes.into(),
                    index,
                    tweak: Default::default(),
                }),
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
        None => false,
    };
    // 아이콘 글꼴은 exe에 정적으로 담겨 있어 실패 경로가 없다 — 한글 글꼴 성공 여부와 무관하게 등록한다
    egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);
    ctx.set_fonts(fonts);
    korean
}

/// 닫기 요청을 창 숨김으로 바꿔야 하는가 (FR-50).
///
/// 셋 다 참일 때만 숨긴다:
/// - 사용자가 `종료` 토글을 켰다
/// - 트레이 아이콘이 **실제로** 올라가 있다 — 없는데 숨기면 창을 되부를 방법이 사라진다
/// - 트레이 메뉴 `종료`로 끝내는 중이 아니다 (그건 진짜 종료다)
///
/// 판정만 떼어 둔 이유: `ViewportCommand` 전송은 시험할 수 없지만 이 판정은 할 수 있다
fn should_hide_on_close(tray_on_close: bool, tray_present: bool, quitting: bool) -> bool {
    tray_on_close && tray_present && !quitting
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
                    tabs: panel.tab_specs(),
                    active_tab: panel.active_tab(),
                    columns: panel.columns(),
                    view_mode: panel.view_mode().as_key().to_owned(),
                },
                // 트리에는 있는데 상태가 없는 리프 — 분할 직후가 아니면 생기지 않는다
                None => PanelTabs {
                    tabs: vec![TabSpec::Local(start_dir())],
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
    /// 타이틀바 왼쪽에 그릴 앱 아이콘 — 시작할 때 한 번 올린다.
    /// 읽지 못하면 `None`(그 자리를 비워 두고 나머지는 그대로 그린다)
    app_icon: Option<egui::TextureHandle>,
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
    /// 세션이 최대화였는데 아직 그 상태로 만들지 못했다면 **남은 시도 프레임 수**.
    ///
    /// **최대화는 창을 만들 때가 아니라 첫 프레임을 그린 뒤에 건다** — 창 생성 단계에서 걸면
    /// winit가 `ShowWindow(SW_MAXIMIZE)`로 아직 그리지 않은 창을 드러내 흰 사각형이 번쩍인다
    /// (`ui::window_start` 참조). 최대화가 실제로 걸릴 때까지는 관측한 사각형을 저장하지
    /// 않는다 — 그러지 않으면 되돌릴 "일반 크기"가 작업 영역 크기로 덮인다.
    /// 세어 두는 이유는 명령이 먹지 않는 환경에서도 **언젠가는 평소 추적으로 돌아가기** 위함이다
    restoring_maximized: u8,
    /// 삭제 확인을 기다리는 워크스페이스 (FR-18).
    /// 인덱스가 아니라 id로 잡는다 — 확인 대화는 프레임을 넘겨 살아 있는데,
    /// 그 사이 순서가 바뀌면 인덱스는 다른 워크스페이스를 가리킨다 (D12 ①과 같은 이유)
    pending_remove: Option<WorkspaceId>,
    /// 앱 전역 설정 (FR-47) — 설정 대화가 바꾸고 각 기능이 읽는다
    settings: AppSettings,
    /// 앱 설정 대화 (FR-47) — 타이틀바 설정 메뉴의 `설정`이 연다
    settings_dialog: SettingsDialog,
    /// 고를 수 있는 글꼴 목록 (FR-48) — 워커가 만든다. 대화를 처음 열 때 한 번만 읽는다
    font_scan: FontScan,
    /// 알림 영역 아이콘 (FR-50) — `종료` 토글이 켜져 있을 때만 있다. 없애면 아이콘이 사라진다
    tray: Option<Tray>,
    /// 트레이 조작이 오는 통로 — 창 프로시저가 보낸다
    tray_rx: std::sync::mpsc::Receiver<TrayEvent>,
    /// 창을 숨긴 상태인가 (FR-50) — 닫기를 눌렀지만 앱은 살아 있다.
    ///
    /// **`ctx.input(|i| i.viewport().visible())`로 파생시키지 않는다** — Windows에서 그 값은
    /// 늘 `None`이라(winit이 `occluded`를 채우지 않는다) 숨김과 표시를 구분하지 못한다
    hidden: bool,
    /// 트레이 메뉴 `종료`로 끝내는 중인가 — 그때는 닫기를 가로채지 않는다
    quitting: bool,
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
    /// 훑어 달라고 청한 원격 폴더들 — 답이 오면 그 아래 파일을 큐에 넣는다 (FR-38)
    pending_trees: HashMap<u64, (SiteId, RemotePath, PathBuf)>,
    /// 훑기 요청 번호 — 목록 조회의 세대 번호와 섞이지 않게 따로 센다
    next_tree: u64,
    /// 원격 파일 작업 대화의 상태 (FR-39)
    remote_ops: RemoteOps,
    /// 상태 줄에 잠깐 띄울 실패 사유와 그 만료 시각 (FR-39)
    notice: Option<(String, f64)>,
    /// 지금 펼치고 있는 로컬 폴더 수 (T22 Edge Case) — 상태 줄이 이것을 알린다.
    /// 펼치기는 별도 스레드에서 돌아 화면이 멈추지는 않지만, 아무 표시가 없으면
    /// 사용자는 아무 일도 안 일어난 줄 안다 (F-7 리뷰 M1)
    expanding: usize,
    /// 원격 트리가 읽어 둔 하위 폴더들 (T24)
    tree_cache: TreeCache,
    /// 트리가 청한 조회의 답을 기다리는 자리 — 세대 → (연결, 경로, 캐시 세대)
    pending_tree_lists: HashMap<u64, (ConnectionId, RemotePath, u64)>,
    /// 트리 조회의 세대 번호
    next_tree_list: u64,
    /// 로컬 폴더를 펼친 결과가 오는 통로 (FR-38) — 펼치기는 워커 스레드가 한다
    expand_tx: std::sync::mpsc::Sender<ExpandResult>,
    expand_rx: std::sync::mpsc::Receiver<ExpandResult>,
    /// 워커가 일을 마쳤을 때 화면을 깨우는 통로 — 연결 관리자에 준 것과 같다
    repaint: Arc<dyn Fn() + Send + Sync>,
}

impl ExplorerApp {
    /// eframe 창 생성 직후 호출된다 — 폰트·팔레트·셸 호스트를 이 시점에 준비한다.
    /// `session`이 있으면 지난 실행의 워크스페이스·사이드바·창 상태를 되살린다 (FR-11·FR-20)
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        com: ComStatus,
        session: Option<Session>,
    ) -> ExplorerApp {
        // **저장된 글꼴을 첫 프레임부터 적용한다** (FR-48) — 여기서 기본값으로 등록해 두고
        // 세션을 읽은 뒤에 다시 등록하면, 시작할 때마다 맑은 고딕으로 한 번 그려졌다가
        // 고른 글꼴로 바뀌는 것이 눈에 보인다
        let korean_font = install_fonts(
            &cc.egui_ctx,
            session
                .as_ref()
                .and_then(|session| session.settings.selected_font()),
        );
        theme::apply_dark(&cc.egui_ctx);
        // HWND 획득·서브클래스 설치는 창이 만들어진 이 시점에만 가능하다
        let shell = ShellHost::new(cc);
        // 최대화·복원 때 OS가 옛 화면과 새 화면을 겹쳐 페이드하면 글자가 이중으로 보인다 (FR-22)
        if let Some(shell) = &shell {
            crate::app::theme::disable_window_transitions(shell.hwnd());
            // 아직 그리지 않은 자리가 흰색으로 번쩍이지 않게 한다 (창 표시·최대화 순간)
            crate::app::theme::paint_unpainted_as_window_bg(shell.hwnd());
        }
        let (expand_tx, expand_rx) = std::sync::mpsc::channel();
        let repaint: Arc<dyn Fn() + Send + Sync> = {
            let ctx = cc.egui_ctx.clone();
            Arc::new(move || ctx.request_repaint())
        };
        // 트레이 조작은 창 프로시저가 보낸다 — 창이 숨은 동안에도 오는 유일한 경로다 (FR-50)
        let (tray_tx, tray_rx) = std::sync::mpsc::channel();
        crate::ui::tray::install_channel(tray_tx, cc.egui_ctx.clone());

        let mut app = ExplorerApp {
            com,
            shell,
            korean_font,
            icons: IconCache::new(),
            textures: IconTextures::new(),
            app_icon: app_icon::load_texture(&cc.egui_ctx, APP_ICON_TEXTURE_PX),
            workspaces: WorkspaceList::new(),
            views: HashMap::new(),
            restored: HashMap::new(),
            sidebar: WorkspaceSidebar::new(),
            sidebar_width: SIDEBAR_DEFAULT_WIDTH as f32,
            sidebar_collapsed: false,
            window: DEFAULT_WINDOW,
            restore_window: None,
            restoring_maximized: 0,
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
            settings: AppSettings::default(),
            settings_dialog: SettingsDialog::new(),
            font_scan: FontScan::new(),
            tray: None,
            tray_rx,
            hidden: false,
            quitting: false,
            pending_clipboard: None,
            pending_trees: HashMap::new(),
            next_tree: 0,
            remote_ops: RemoteOps::default(),
            notice: None,
            expanding: 0,
            tree_cache: TreeCache::new(),
            pending_tree_lists: HashMap::new(),
            next_tree_list: 0,
            expand_tx,
            expand_rx,
            repaint,
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
        // 사이트·큐·도크를 되살린다 (FR-44) — **연결은 열지 않고 전송도 시작하지 않는다**.
        // 원격 탭은 `연결 없음`으로 서 있고, 큐는 대기·실패인 채로 기다린다
        self.sites = session.sites.clone();
        self.queue = session::restore_queue(&session);
        self.dock = DockState::from_session(&session.dock);
        self.settings = session.settings.clone();
        self.sidebar_width = session.sidebar.width as f32;
        self.sidebar_collapsed = session.sidebar.collapsed;
        self.restoring_maximized = if session.window.maximized {
            MAXIMIZE_RETRY_FRAMES
        } else {
            0
        };
        self.window = session.window.clone();
        self.restore_window = Some(session.window);
    }

    /// 아직 열지 않은 워크스페이스의 활성 폴더 — 사이드바 부제를 복원 직후에도 채우기 위함
    fn restored_active_dir(&self, index: usize) -> Option<PathBuf> {
        let id = self.workspaces.items().get(index)?.id;
        let state = self.restored.get(&id)?;
        let panel = state.panels.get(state.active_panel)?;
        // 원격 탭이 활성이면 부제로 쓸 로컬 경로가 없다 — 그때는 부제를 비운다
        panel.tabs.get(panel.active_tab)?.local_path().cloned()
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
                            tabs: vec![TabSpec::Local(start_dir())],
                            active_tab: 0,
                            ..Default::default()
                        }],
                        active_panel: 0,
                    },
                },
            })
            .collect();
        // 앱 설정은 `to_session`이 아니라 여기서 싣는다 — `to_session`은 창·워크스페이스를
        // 옮기는 자리라 인자를 하나 더 받으면 그 책임이 흐려지고, 호출부(테스트 7곳)도 함께 는다
        Session {
            settings: self.settings.clone(),
            ..session::to_session(
                self.window.clone(),
                SidebarSession {
                    width: self.sidebar_width as i32,
                    collapsed: self.sidebar_collapsed,
                },
                self.workspaces.active_index(),
                &workspaces,
                RemoteSnapshot {
                    sites: &self.sites,
                    queue: &self.queue,
                    dock: self.dock.to_session(),
                },
            )
        }
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
    /// 복원 위치가 화면 밖이면(모니터 구성 변경) 첫 프레임에 화면 안으로 옮긴다.
    ///
    /// 세션이 최대화였다면 **여기서 비로소 최대화를 건다** — 창을 만들 때 걸면 아직 그리지
    /// 않은 창이 드러나 흰 사각형이 번쩍인다(`ui::window_start` 참조)
    fn track_window(&mut self, ctx: &egui::Context) {
        // **숨긴 동안에는 관측하지 않는다** — 숨은 창의 viewport 정보로 위치·크기를 덮으면
        // 숨기기 직전에 저장해 둔 좋은 값이 그것으로 밀리고, 종료 시 그 밀린 값이 저장된다
        if self.hidden {
            return;
        }
        let (rect, maximized, monitor) = ctx.input(|input| {
            let viewport = input.viewport();
            (
                viewport.outer_rect,
                viewport.maximized.unwrap_or(false),
                viewport.monitor_size,
            )
        });
        // 최대화가 실제로 걸릴 때까지는 관측한 사각형도, 최대화 여부도 반영하지 않는다 —
        // 그 사이의 창은 "작업 영역만 한 일반 창"이라, 그대로 받으면 되돌릴 일반 크기가 덮인다
        if self.restoring_maximized > 0 {
            if maximized {
                self.restoring_maximized = 0;
                self.window.maximized = true;
            } else if ctx.cumulative_pass_nr() > 0 {
                // **첫 프레임에는 걸지 않는다** — eframe은 첫 프레임을 그린 뒤에야 창을 보이게
                // 하는데(glow_integration `post_rendering`), 그 전에 최대화를 걸면
                // `ShowWindow(SW_MAXIMIZE)`가 아직 아무것도 그려지지 않은 창을 먼저 드러내
                // 흰 사각형이 번쩍인다(2026-08-13 실측 — 이것이 이 결함의 두 번째 경로다)
                self.restoring_maximized -= 1;
                ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(true));
            }
            return;
        }
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
            .show(ui, |ui| {
                titlebar::show_titlebar(ui, &title, state, self.app_icon.as_ref().map(|t| t.id()))
            })
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

    /// 상태 표시줄 — **언제나 보인다** (FR-41·D18). 도크보다 먼저 붙어 창 맨 아래에 남는다.
    ///
    /// 여기 캐럿이 도크를 여는 문이다(README §8) — 열고 나면 그 안의 탭에서 큐·로그를 고른다
    fn show_status_bar(&mut self, ui: &mut egui::Ui) {
        // 지난 사유는 지운다 — 남겨 두면 이미 끝난 실패가 계속 떠 있다(로그에는 그대로 남는다)
        let now = ui.input(|input| input.time);
        if self.notice.as_ref().is_some_and(|(_, until)| now >= *until) {
            self.notice = None;
        }
        let action = egui::Panel::bottom(egui::Id::new("status_bar"))
            .resizable(false)
            .default_size(status_bar::HEIGHT)
            .size_range(egui::Rangef::new(status_bar::HEIGHT, status_bar::HEIGHT))
            .frame(egui::Frame::NONE)
            .show(ui, |ui| {
                let rect = ui.max_rect();
                let view = StatusView {
                    queue: &self.queue,
                    expanding: self.expanding,
                    notice: self.notice.as_ref().map(|(text, _)| text.as_str()),
                };
                status_bar::show_status_bar(ui, rect, &self.dock, &view)
            })
            .inner;
        match action {
            Some(StatusAction::ToggleQueue) => self.dock.toggle(DockPanel::Queue),
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
            let connected = self.connected_conn_sites();
            let view = DockView {
                queue: &self.queue,
                failed,
                connected: &connected,
            };
            let dock_action = dock::show_strip(&mut dock_ui, strip, &mut self.dock, &view);
            let queue_action = match self.dock.panel {
                Some(DockPanel::Queue) => {
                    queue_panel::show_queue(&mut dock_ui, body, &mut self.dock, &view, &self.sites)
                }
                Some(DockPanel::Log) => {
                    // 큐와 **같은 연결별 탭 줄**을 먼저 그린다 — 도크에 줄은 하나다(디자인 `:272`).
                    // 여기서 고른 사이트가 곧 "어느 서버의 로그를 볼지"가 된다
                    let site_row = egui::Rect::from_min_size(
                        body.min,
                        egui::vec2(body.width(), queue_panel::SITE_ROW_HEIGHT),
                    );
                    queue_panel::show_site_tabs(
                        &mut dock_ui,
                        site_row,
                        &mut self.dock,
                        &view,
                        &self.sites,
                    );
                    // 지금 보고 있는 연결의 로그를 그린다 — 연결이 없으면 빈 화면이다
                    let body = egui::Rect::from_min_max(
                        egui::pos2(body.left(), site_row.bottom() + log_panel::BODY_PAD_Y),
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

    /// 끌어다 놓은 것을 전송 큐에 넣는다 (FR-38).
    ///
    /// **로컬 → 원격은 올리기, 원격 → 로컬은 받기**다. 로컬끼리·원격끼리는 아무 일도 하지
    /// 않는다(PRD Out of Scope) — 항목의 종류와 놓은 자리의 종류가 같으면 걸러진다.
    /// 폴더는 파일 단위로 펼쳐 넣는다(T17 규약): 로컬은 그 자리에서, 원격은 워커에 훑기를 맡긴다
    fn apply_drop(&mut self, drop: DropOutcome) {
        match &drop.target {
            DropTarget::Remote { site, dir } => {
                // 폴더를 펼치는 것은 디렉터리를 재귀로 읽는 일이라 **워커 스레드**가 한다
                // (AGENTS: UI 스레드 블로킹 I/O 금지 — 큰 폴더면 프레임이 멈춘다).
                // 결과는 채널로 받아 다음 프레임에 큐로 옮긴다 (`DirLoad`와 같은 방식)
                let roots: Vec<PathBuf> = drop
                    .items
                    .iter()
                    .filter(|item| list_common::drop_direction(item, &drop.target).is_some())
                    .filter_map(|item| match item {
                        DragItem::Local { path, .. } => Some(path.clone()),
                        DragItem::Remote { .. } => None,
                    })
                    .collect();
                if roots.is_empty() {
                    return;
                }
                let tx = self.expand_tx.clone();
                self.expanding += 1;
                let (site, dir) = (*site, dir.clone());
                let wake = self.repaint.clone();
                std::thread::spawn(move || {
                    let mut files = Vec::new();
                    let mut skipped = 0;
                    for root in roots {
                        let expanded = transfer::expand_for_transfer(&root);
                        skipped += expanded.skipped;
                        for (path, relative) in expanded.files {
                            let size = std::fs::metadata(&path).map(|meta| meta.len()).unwrap_or(0);
                            files.push((path, dir.join(&relative), size));
                        }
                    }
                    if tx.send((site, files, skipped)).is_ok() {
                        wake();
                    }
                });
            }
            DropTarget::Local(local_dir) => {
                let Some(site) = drop.source_site else {
                    return;
                };
                for item in &drop.items {
                    // 종류가 같으면(로컬 → 로컬) 아무 일도 하지 않는다
                    if list_common::drop_direction(item, &drop.target).is_none() {
                        continue;
                    }
                    let DragItem::Remote { path, is_dir, size } = item else {
                        continue;
                    };
                    if *is_dir {
                        self.request_tree(site, path.clone(), local_dir.clone());
                        continue;
                    }
                    self.queue.enqueue(
                        site,
                        TransferDirection::Download,
                        local_dir.join(item.name()),
                        path.clone(),
                        *size,
                    );
                }
            }
        }
    }

    /// 원격 메뉴에서 고른 것을 실행한다 (FR-39).
    ///
    /// 대화가 필요한 것(이름 바꾸기·새 폴더·권한·삭제)은 여기서 **열기만** 하고, 실제 명령은
    /// 사용자가 확인한 뒤에 나간다 — 특히 삭제는 확인 없이 도는 경로를 만들지 않는다
    /// (plan Halt Forecast)
    fn apply_remote_menu(
        &mut self,
        panel: PanelId,
        action: RemoteMenuAction,
        targets: Vec<RemoteTarget>,
    ) {
        let Some(conn) = self.panel_conn(panel) else {
            return;
        };
        let site = self.manager.get(conn).map(|connection| connection.site);
        self.remote_ops.conn = Some(conn);
        self.remote_ops.targets = targets.iter().map(|item| item.path.clone()).collect();
        self.remote_ops.error = None;
        match action {
            // 받기·올리기는 **끌어다 놓기와 같은 길**로 보낸다 (FR-38) — 폴더를 훑는 것도,
            // 큐에 넣는 것도 이미 그쪽에 있다. 메뉴만 따로 두면 두 길이 곧 어긋난다
            RemoteMenuAction::Download => {
                let (Some(site), Some(local)) = (site, self.other_panel_local(panel)) else {
                    return;
                };
                let items = targets
                    .into_iter()
                    .map(|item| DragItem::Remote {
                        path: item.path,
                        is_dir: item.is_dir,
                        size: item.size,
                    })
                    .collect();
                self.apply_drop(DropOutcome {
                    items,
                    source_site: Some(site),
                    target: DropTarget::Local(local.dir),
                });
            }
            RemoteMenuAction::Upload => {
                let (Some(site), Some(local), Some(dir)) =
                    (site, self.other_panel_local(panel), self.remote_dir(conn))
                else {
                    return;
                };
                if local.selected.is_empty() {
                    return;
                }
                let items = local
                    .selected
                    .into_iter()
                    .map(|(path, is_dir)| DragItem::Local { path, is_dir })
                    .collect();
                self.apply_drop(DropOutcome {
                    items,
                    source_site: None,
                    target: DropTarget::Remote { site, dir },
                });
            }
            RemoteMenuAction::Refresh => self.request_remote_list(conn),
            RemoteMenuAction::Rename => {
                self.remote_ops.name = self
                    .remote_ops
                    .targets
                    .first()
                    .and_then(|path| path.file_name())
                    .unwrap_or_default()
                    .to_owned();
                self.remote_ops.dialog = Some(RemoteDialog::Rename);
            }
            RemoteMenuAction::NewFolder => {
                self.remote_ops.name = String::new();
                self.remote_ops.dialog = Some(RemoteDialog::NewFolder);
            }
            RemoteMenuAction::Chmod => {
                // 서버가 알려 준 권한에서 시작한다 — 엉뚱한 기본값에서 출발하면 사용자가
                // 만지지 않은 비트까지 함께 바뀐다(spec 리뷰 N4). 안 알려 주는 서버에서만
                // 흔한 기본값을 쓴다
                let mode = targets.first().and_then(|item| item.mode);
                self.remote_ops.permissions = Permissions::from_mode(mode.unwrap_or(0o644));
                self.remote_ops.octal = self.remote_ops.permissions.to_octal_text();
                self.remote_ops.dialog = Some(RemoteDialog::Chmod);
            }
            RemoteMenuAction::Delete => {
                self.remote_ops.recursive = false;
                self.remote_ops.dialog = Some(RemoteDialog::Delete);
            }
        }
    }

    /// 그 패널이 쓰는 연결
    fn panel_conn(&self, panel: PanelId) -> Option<ConnectionId> {
        self.views
            .get(&self.workspaces.active().id)
            .and_then(|view| view.panels.get(&panel))
            .and_then(|panel| panel.active_conn())
    }

    /// 원격 메뉴의 `받기`·`올리기`가 짝으로 삼는 **다른 패널의 로컬 쪽 상태**.
    ///
    /// 끌어다 놓기에는 "어디로"가 손끝에 있지만 메뉴에는 없다 — 두 칸 탐색기의 반대편이
    /// 그 자리를 대신한다. 로컬 패널이 하나도 없으면 `None`이라 아무 일도 일어나지 않는다
    fn other_panel_local(&self, from: PanelId) -> Option<LocalSide> {
        let view = self.views.get(&self.workspaces.active().id)?;
        view.panels
            .iter()
            .filter(|(id, _)| **id != from)
            .find_map(|(_, panel)| {
                panel.local_dir().map(|dir| LocalSide {
                    dir,
                    selected: panel.selected_local(),
                })
            })
    }

    /// 원격 대화들을 그리고 확인된 명령을 연결에 보낸다 (FR-39).
    ///
    /// **확인이든 취소든 대화는 그 자리에서 닫힌다** — 취소를 "아직 안 골랐다"와 같이 다루면
    /// 다음 프레임에 같은 대화가 다시 떠 빠져나올 수 없다 (spec 리뷰 M1)
    fn show_remote_dialogs(&mut self, ctx: &egui::Context) {
        let Some(dialog) = self.remote_ops.dialog else {
            return;
        };
        let Some(conn) = self.remote_ops.conn else {
            self.remote_ops.dialog = None;
            return;
        };
        match dialog {
            RemoteDialog::Rename | RemoteDialog::NewFolder => {
                let rename = dialog == RemoteDialog::Rename;
                let title = if rename {
                    "이름 바꾸기"
                } else {
                    "새 폴더"
                };
                let outcome = remote_menu::show_name_dialog(
                    ctx,
                    title,
                    &mut self.remote_ops.name,
                    &mut self.remote_ops.error,
                );
                let Some(name) = settle_dialog(outcome, &mut self.remote_ops.dialog) else {
                    return;
                };
                let command = if rename {
                    self.remote_ops.targets.first().and_then(|path| {
                        Some(ConnCommand::Rename {
                            from: path.clone(),
                            to: path.parent()?.join(&name),
                        })
                    })
                } else {
                    self.remote_dir(conn)
                        .map(|dir| ConnCommand::Mkdir(dir.join(&name)))
                };
                if let Some(command) = command {
                    self.manager.send(conn, command);
                }
            }
            RemoteDialog::Chmod => {
                let outcome = remote_menu::show_chmod_dialog(
                    ctx,
                    &mut self.remote_ops.permissions,
                    &mut self.remote_ops.octal,
                );
                let Some(mode) = settle_dialog(outcome, &mut self.remote_ops.dialog) else {
                    return;
                };
                for path in std::mem::take(&mut self.remote_ops.targets) {
                    self.manager.send(conn, ConnCommand::Chmod { path, mode });
                }
            }
            RemoteDialog::Delete => {
                let outcome = remote_menu::show_delete_confirm(
                    ctx,
                    &self.remote_ops.targets,
                    &mut self.remote_ops.recursive,
                );
                let Some(recursive) = settle_dialog(outcome, &mut self.remote_ops.dialog) else {
                    return;
                };
                for path in std::mem::take(&mut self.remote_ops.targets) {
                    // 폴더인지는 서버가 안다 — 재귀를 켜지 않았으면 파일 삭제를 보내고,
                    // 폴더라 거부되면 그 사유가 로그와 상태 줄에 남는다 (D22와 같은 방식)
                    self.manager.send(conn, delete_command(path, recursive));
                }
            }
        }
    }

    /// 그 연결을 보고 있는 패널의 현재 원격 폴더
    fn remote_dir(&self, conn: ConnectionId) -> Option<RemotePath> {
        self.views
            .values()
            .flat_map(|view| view.panels.values())
            .find(|panel| panel.active_conn() == Some(conn))
            .and_then(|panel| panel.remote_dir())
    }

    /// 트리가 청한 하위 조회를 연결에 보낸다 (T24).
    ///
    /// 캐시가 "아직 안 읽었다"고 할 때만 실제로 나간다 — 펼침이 반복돼도 서버에는 한 번만
    /// 묻는다 (Acceptance ②)
    fn request_tree_children(&mut self, conn: ConnectionId, path: RemotePath) {
        let Some(cache_generation) = self.tree_cache.begin(conn, &path) else {
            return;
        };
        // 목록 조회(패널)와 **번호 공간을 나눈다** — 같은 번호가 겹치면 한쪽의 답을 다른 쪽이
        // 가져가 서로 영영 기다린다. 트리 쪽은 높은 자리에서 센다
        self.next_tree_list += 1;
        let generation = TREE_LIST_BASE + self.next_tree_list;
        self.pending_tree_lists
            .insert(generation, (conn, path.clone(), cache_generation));
        self.manager
            .send(conn, ConnCommand::List { generation, path });
    }

    /// 원격 폴더를 훑어 달라고 워커에 청한다 (FR-38).
    ///
    /// 화면이 한 겹씩 요청해 가며 훑지 않는 이유: 목록 응답 라우팅과 뒤섞이고 프레임마다
    /// 상태를 이어 붙여야 한다. 워커는 어차피 블로킹이라 한 번에 끝내는 편이 단순하다
    fn request_tree(&mut self, site: SiteId, root: RemotePath, local_dir: PathBuf) {
        let Some(conn) = self.site_connection(site) else {
            return;
        };
        let generation = self.next_tree;
        self.next_tree += 1;
        self.pending_trees
            .insert(generation, (site, root.clone(), local_dir));
        self.manager
            .send(conn, ConnCommand::ListTree { generation, root });
    }

    /// 그 사이트의 연결 하나 — 여럿이면 먼저 연 것을 쓴다
    fn site_connection(&self, site: SiteId) -> Option<ConnectionId> {
        self.manager
            .ids()
            .iter()
            .copied()
            .find(|id| self.manager.get(*id).is_some_and(|conn| conn.site == site))
    }

    /// 로그 화면이 보여 줄 연결 — **지금 보고 있는 원격 탭의 것**이 먼저다.
    ///
    /// 그 탭이 로컬이면 마지막으로 연 연결을 보인다: 로그를 여는 까닭은 대개 방금 무슨 일이
    /// 있었는지 보려는 것이라, 아무것도 안 보이는 것보다 최근 연결을 보이는 편이 쓸모 있다
    fn log_connection(&self) -> Option<ConnectionId> {
        // 연결별 탭에서 고른 사이트가 있으면 그 사이트의 연결이 먼저다 — 사용자가 고른 것이다.
        // 그 사이트의 연결이 이미 접혔으면 아래의 자동 선택으로 돌아간다
        if let Some(site) = self.dock.site
            && let Some(conn) = self.site_connection(site)
        {
            return Some(conn);
        }
        let active = self
            .views
            .get(&self.workspaces.active().id)
            .and_then(|view| view.panels.get(&view.active))
            .and_then(|panel| panel.active_conn());
        active.or_else(|| self.manager.ids().last().copied())
    }

    /// 지금 **연결이 열려 있는** 사이트들 — 연결별 탭이 큐가 비어 있어도 이들을 세운다
    fn connected_conn_sites(&self) -> Vec<SiteId> {
        let mut sites: Vec<SiteId> = self
            .manager
            .ids()
            .iter()
            .filter_map(|id| self.manager.get(*id).map(|connection| connection.site))
            .collect();
        sites.dedup();
        sites
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

    /// 앱 설정 대화 (FR-47) — 바뀐 값은 그 자리에서 저장한다.
    ///
    /// 즉시 저장인 이유는 이 화면에 `취소`가 없기 때문이다(사용자 결정) — 닫기만 있는
    /// 화면에서 저장을 닫을 때로 미루면, 앱이 그 사이에 죽었을 때 바꾼 값이 사라진다
    /// 닫기 요청을 가로채 창만 숨긴다 (FR-50).
    ///
    /// 타이틀바 `✕`뿐 아니라 `Alt+F4`·작업 표시줄 닫기·시스템 메뉴까지 **모든 종료 경로가
    /// 이 한 지점으로 모인다** — 버튼 핸들러에서 막으면 나머지 길로 들어온 종료를 놓친다 (D4)
    fn intercept_close(&mut self, ctx: &egui::Context) {
        if !ctx.input(|input| input.viewport().close_requested()) {
            return;
        }
        if !should_hide_on_close(
            self.settings.tray_on_close,
            self.tray.is_some(),
            self.quitting,
        ) {
            return;
        }
        // **숨기기 전에 저장한다** — 종료 때 도는 `on_exit`가 이번에는 오지 않는다
        self.persist_session();
        self.hidden = true;
        ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
    }

    /// `종료` 토글에 맞춰 트레이 아이콘을 올리거나 내린다 (FR-50).
    ///
    /// 매 프레임 확인하는 이유: 토글이 바뀌는 자리가 설정 대화 한 곳이지만, 그 값과
    /// 실제 아이콘 유무가 어긋나면 화면에서 그것을 알 길이 없다
    fn sync_tray(&mut self, now: f64) {
        let want = self.settings.tray_on_close;
        if want == self.tray.is_some() {
            return;
        }
        if !want {
            // `Drop`이 아이콘을 거둔다
            self.tray = None;
            return;
        }
        // 창 핸들이 없으면(다른 백엔드·headless) 아이콘을 올릴 수 없다 —
        // 조용히 넘기면 토글은 켜져 있는데 아이콘이 영영 뜨지 않는다
        self.tray = self
            .shell
            .as_ref()
            .and_then(|shell| Tray::add(shell.hwnd()));
        if self.tray.is_none() {
            // 아이콘을 못 올렸으면 토글을 되돌린다 — 켜져 있다고 보이는데 아이콘이
            // 없으면 닫기를 눌렀을 때 앱을 되살릴 방법이 사라진다
            self.settings.tray_on_close = false;
            self.notice = Some((
                "트레이 아이콘을 만들지 못했습니다".to_owned(),
                now + NOTICE_SECS,
            ));
        }
    }

    /// 트레이에서 온 통지를 처리한다 — 창을 띄우는 일은 프로시저가 이미 끝냈다
    fn poll_tray(&mut self, ctx: &egui::Context) {
        while let Ok(event) = self.tray_rx.try_recv() {
            match event {
                // 창은 프로시저가 이미 띄웠다. 여기서는 ⓐ 숨김 상태를 내리고
                // ⓑ winit의 가시성 캐시를 맞춘다 — 그러지 않으면 이후 최대화 등
                // 창 플래그가 바뀔 때 `apply_diff`가 `SW_HIDE`를 다시 적용해 창이 사라진다
                TrayEvent::Shown => {
                    if self.hidden {
                        self.hidden = false;
                        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                    }
                }
                // 탐색기가 되살아나 아이콘이 사라졌다 — 비워 두면 다음 `sync_tray`가 다시 올린다
                TrayEvent::Recreated => self.tray = None,
                // 메뉴 `종료` — 평소 닫기와 같은 길로 보낸다(세션 저장이 그 길에 있다)
                TrayEvent::Quit => {
                    // 이 닫기는 가로채지 않는다 — 트레이로 보내는 것이 아니라 실제 종료다
                    self.quitting = true;
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            }
        }
    }

    fn show_settings_dialog(&mut self, ctx: &egui::Context) {
        if !self.settings_dialog.is_open() {
            return;
        }
        // 목록 만들기는 1.5초쯤 걸려 워커에 맡긴다 — 대화를 여는 순간 창이 멈추면 안 된다.
        // 이미 받아 둔 목록이 있으면 `ensure_started`가 아무 일도 하지 않는다
        self.font_scan.ensure_started(ctx);
        self.font_scan.poll();

        let outcome = self.settings_dialog.show(
            ctx,
            &mut self.settings,
            FontChoices {
                names: self.font_scan.names(),
            },
        );
        if outcome.font_changed {
            // 글꼴은 다음 pass부터 적용된다 — 그 프레임이 오도록 다시 그리기를 요청한다
            self.korean_font = install_fonts(ctx, self.settings.selected_font());
            ctx.request_repaint();
        }
        // 자동 실행 등록이 막힌 환경 등 — 조용히 넘기면 토글이 왜 안 움직였는지 알 수 없다
        if let Some(notice) = outcome.notice {
            self.toast
                .show(notice.to_owned(), ctx.input(|input| input.time));
        }
        if outcome.changed {
            self.persist_session();
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
            SiteManagerOutcome::None => {}
            // 대화 안에서 이름 바꾸기·삭제·복제로 목록이 바뀌었을 수 있다 — 닫을 때 함께 적는다
            SiteManagerOutcome::Close => self.persist_session(),
            // 등록만 했으면 그 사실을 짧게 알린다 (인벤토리 #89·#91)
            SiteManagerOutcome::Register(site) => {
                self.persist_session();
                let host = self
                    .sites
                    .get(site)
                    .map(|record| record.host.clone())
                    .unwrap_or_default();
                let now = ctx.input(|input| input.time);
                self.toast.show(toast::registered_text(&host), now);
            }
            SiteManagerOutcome::RegisterAndConnect(site) => {
                self.persist_session();
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
                // 패널이 사라지면 그 탭들이 쓰던 연결도 쓸 곳이 없어진다 — 닫기 전에 모아 둔다
                let conns = view
                    .panels
                    .get(&panel)
                    .map(PanelState::conns)
                    .unwrap_or_default();
                view.close_panel(panel, area);
                // 마지막 패널은 닫히지 않는다 (FR-2) — 그때는 연결도 그대로 둔다
                let closed = !view.panels.contains_key(&panel);
                if closed {
                    for conn in conns {
                        // 다른 패널이 아직 그 연결을 쓰고 있으면 접지 않는다 (FR-32)
                        if !self.conn_in_use(conn) {
                            self.release_conn(conn);
                        }
                    }
                }
            }
            Command::ToggleSidebar => self.sidebar_collapsed = !self.sidebar_collapsed,
            Command::OpenAppSettings => self.settings_dialog.open(),
            // 이 셋은 연결(`manager`)에 닿아야 해서 패널만 빌리는 아래 묶음에 들어갈 수 없다
            Command::OpenSiteTab(site) => self.open_site_tab_here(site, target),
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

    /// 이 연결을 아직 쓰는 탭이 있는가 — 지금 보이지 않는 워크스페이스까지 본다.
    /// 한 연결을 여러 패널이 나눠 쓸 수 있어, 한 곳이 놓았다고 접으면 나머지가 끊긴다
    fn conn_in_use(&self, conn: ConnectionId) -> bool {
        self.views
            .values()
            .any(|view| view.panels.values().any(|panel| panel.uses_conn(conn)))
    }

    /// 연결 하나를 접고 그에 딸린 대기 자리를 함께 지운다 (FR-32).
    ///
    /// 워커와 소켓이 여기서 회수된다. 그 연결에 청해 둔 훑기는 답이 오지 않으므로
    /// 기다리는 자리도 함께 지운다 (T24 Acceptance ④) — 남기면 영영 기다린다
    fn release_conn(&mut self, conn: ConnectionId) {
        let site = self.manager.get(conn).map(|connection| connection.site);
        self.manager.close(conn);
        self.tree_cache.forget(conn);
        self.pending_tree_lists
            .retain(|_, (waiting, _, _)| *waiting != conn);
        if let Some(site) = site
            && self.site_connection(site).is_none()
        {
            self.pending_trees
                .retain(|_, (waiting, _, _)| *waiting != site);
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
                    // 트리가 청한 답이면 캐시로 간다 — 목록 화면은 이것을 모른다
                    if let Some((conn, path, cache_generation)) =
                        self.pending_tree_lists.remove(&generation)
                    {
                        let mut entries = entries;
                        sort_tree_children(&mut entries);
                        self.tree_cache.fill(conn, cache_generation, &path, entries);
                        continue;
                    }
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
                // 훑기 결과 — 찾은 파일을 통째로 큐에 넣는다 (FR-38)
                ConnEvent::TreeListed {
                    generation,
                    root,
                    files,
                } => {
                    if let Some((site, _, local_dir)) = self.pending_trees.remove(&generation) {
                        for (path, size) in files {
                            // 서버 쪽 구조를 로컬에도 그대로 만든다 — 뿌리 폴더 이름부터 붙인다
                            let relative = path
                                .as_str()
                                .strip_prefix(root.as_str())
                                .unwrap_or(path.file_name().unwrap_or_default())
                                .trim_start_matches('/');
                            let root_name = root.file_name().unwrap_or_default();
                            let local = local_dir.join(root_name).join(relative.replace('/', "\\"));
                            self.queue.enqueue(
                                site,
                                TransferDirection::Download,
                                local,
                                path,
                                size,
                            );
                        }
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
                // 조회 실패 — 트리가 청한 것이면 그 노드에만 사유를 남기고(T24 Edge Case),
                // 패널이 청한 것이면 **옮기기를 무르고** 사유를 상태 줄에 남긴다 (F-7 리뷰 B2).
                // 무르지 않으면 주소창은 새 폴더를, 목록은 이전 폴더를 가리킨 채 갈라진다
                ConnEvent::ListFailed { generation, detail } => {
                    match self.pending_tree_lists.remove(&generation) {
                        Some((conn, path, cache_generation)) => {
                            self.tree_cache.fail(conn, cache_generation, &path, detail);
                        }
                        None => self.revert_remote_move(conn, generation, detail, now),
                    }
                }
                // 파일 작업의 답 — 성공하면 목록을 다시 읽고, 실패하면 사유를 남긴다 (FR-39)
                ConnEvent::OpDone { op, result } => self.on_op_done(conn, op, result, now),
                // 서버 로그는 `Connection`이 자기 버퍼에 이미 쌓는다(화면은 T20이 만든다)
                _ => {}
            }
        }
    }

    /// 파일 작업의 결과를 반영한다 (FR-39).
    ///
    /// 성공하면 **목록을 다시 읽는다** — 서버가 바뀐 것을 앱이 짐작해 그리면 실제와 어긋난다.
    /// 실패는 상태 줄과 로그 양쪽에 남긴다: 상태 줄은 곧 사라지므로 되짚을 자리가 필요하다.
    /// 서버가 `SITE CHMOD`를 모르는 것은 흔한 일이라 이때도 앱은 그대로 돈다 (D22)
    fn on_op_done(
        &mut self,
        conn: ConnectionId,
        op: OpKind,
        result: Result<(), RemoteError>,
        now: f64,
    ) {
        match op_outcome(op, result) {
            OpOutcome::Relist => self.request_remote_list(conn),
            OpOutcome::Notice(text) => {
                self.manager.note(conn, LogKind::Error, text.clone());
                self.notice = Some((text, now + NOTICE_SECS));
            }
            OpOutcome::Ignore => {}
        }
    }

    /// 조회가 실패한 패널의 옮기기를 무르고 사유를 알린다 (F-7 리뷰 B2).
    ///
    /// **보이는 목록과 경로가 어긋나지 않게** 하는 것이 목적이다 — 어긋난 채로 두면 그 위에서
    /// 연 원격 메뉴가 사용자가 보는 것과 다른 경로에 삭제·권한 변경을 건다
    fn revert_remote_move(
        &mut self,
        conn: ConnectionId,
        generation: u64,
        detail: String,
        now: f64,
    ) {
        // `any`로 쓰지 않는다 — 짧게 끊기면 같은 연결을 보는 다른 패널이 어긋난 채 남는다.
        // 되돌릴지는 패널이 스스로 판정한다(그 세대의 이동이었고 아직 그 자리에 있는가)
        let mut reverted = false;
        for panel in self
            .views
            .values_mut()
            .flat_map(|view| view.panels.values_mut())
        {
            if panel.active_conn() == Some(conn) && panel.revert_remote_path(generation) {
                reverted = true;
            }
        }
        let text = if reverted {
            format!("폴더를 열지 못했습니다 — {detail}")
        } else {
            format!("목록을 읽지 못했습니다 — {detail}")
        };
        self.manager.note(conn, LogKind::Error, text.clone());
        self.notice = Some((text, now + NOTICE_SECS));
    }

    /// 원격 위치가 바뀐 패널들이 새 위치의 목록을 청한다 (T24 Acceptance ⑤).
    ///
    /// 옮긴 쪽(트리·상위 이동)은 연결을 모르고 명령을 보낼 수단도 없다 — 깃발만 세워 두고
    /// 여기서 거둔다
    fn list_moved_panels(&mut self) {
        let ExplorerApp { views, manager, .. } = self;
        for view in views.values_mut() {
            for panel in view.panels.values_mut() {
                if panel.take_remote_dirty() {
                    panel.request_remote_list(manager);
                }
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
        // `다시 연결`만은 **연결이 없을 때** 누르는 것이라 아래 연결 가드보다 앞에 둔다
        if action == RemoteAction::Reconnect {
            self.reconnect_panel(target);
            return;
        }
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
            // 위에서 이미 처리하고 돌아갔다
            RemoteAction::Reconnect => {}
        }
    }

    /// 세션에서 되살아난 원격 탭을 그 사이트로 다시 연결한다 (사용자 보고 2026-08-13).
    ///
    /// 사이트를 새로 여는 길(`connect_site`)을 그대로 탄다 — 재시작 뒤에는 워커가 없어
    /// `Retry`처럼 명령만 보낼 상대가 없다. 연결이 서면 그 다음은 사이드바에서 사이트를 열
    /// 때와 같은 흐름이다(단계가 `Ok`가 되면 앱이 목록을 청한다)
    fn reconnect_panel(&mut self, target: PanelId) {
        let Some(view) = self.views.get_mut(&self.workspaces.active().id) else {
            return;
        };
        let Some(site) = view
            .panels
            .get(&target)
            .and_then(|panel| panel.active_site())
        else {
            return;
        };
        // `connect_site`는 **활성 패널의 활성 탭**에 연결을 붙인다 — 버튼을 누른 패널이
        // 활성이 아니면 엉뚱한 탭이 붙으므로 먼저 맞춘다(누른 패널이 활성이 되는 것이 자연스럽다)
        view.active = target;
        self.connect_site(site);
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
                self.persist_session();
                site
            }
        };
        self.open_site_tab_at(site, Some(target), url.path, area);
    }

    /// 지금 상태를 곧바로 세션 파일에 적는다 (FR-44).
    ///
    /// **사이트 목록이 바뀌면 그 자리에서 적는다** — 종료 때만 적으면 그 사이에 앱이
    /// 비정상 종료됐을 때(패닉·강제 종료·전원 차단) 등록한 사이트가 통째로 사라진다.
    /// 파일이 작고 사이트 등록은 드문 일이라 그때마다 적어도 부담이 없다
    fn persist_session(&self) {
        save_session(&self.collect_session());
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

    /// 사이트를 **그 패널의 새 탭**으로 연다 — 나누지 않는다.
    ///
    /// 탭 스트립의 `연결 사이트를 새 탭으로` 드롭다운과 스트립에 끌어다 놓기가 이 길을 쓴다.
    /// 이름 그대로 새 탭이며, `+`(새 탭)와 같은 자리에 열려야 한다 (사용자 보고) — 사이드바·
    /// 사이트 관리자에서 여는 길만 좌우로 나눠 연다 (`open_site_tab` — FR-35)
    fn open_site_tab_here(&mut self, site: SiteId, target: Option<PanelId>) {
        if self.sites.get(site).is_none() {
            return;
        }
        let view = self.ensure_active_view();
        let opened = target.unwrap_or(view.active);
        let Some(panel) = view.panels.get_mut(&opened) else {
            return;
        };
        panel.open_remote_tab(site, RemotePath::root());
        // 연결은 **활성 패널의 활성 탭**에 붙는다 — 다른 패널에서 연 경우까지 맞춰 둔다
        view.active = opened;
        self.connect_site(site);
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
        let created = view.split_panel(source, SplitDir::Horizontal, SplitPlace::After, area);
        let opened = created.unwrap_or(source);
        let Some(panel) = view.panels.get_mut(&opened) else {
            return;
        };
        if created.is_some() {
            // 새로 나온 패널에는 연결만 남긴다 — 시작 폴더 탭은 사용자가 연 적이 없다
            panel.open_remote_tab_only(site, path);
        } else {
            // 나눌 자리가 없어 현재 패널로 물러선 길 — 쓰던 탭은 그대로 두고 하나 더 연다
            panel.open_remote_tab(site, path);
        }
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
        self.intercept_close(ctx);
        self.track_window(ctx);
        self.textures.begin_frame();
        // 연결 소식은 **워크스페이스와 무관하게** 받는다 — 채널에 쌓인 것을 건너뛰면
        // 보이지 않는 워크스페이스의 원격 탭이 옛 단계로 굳는다
        let now = ctx.input(|input| input.time);
        self.poll_remote(now);
        // 펼쳐진 로컬 폴더를 큐로 옮긴다 (FR-38)
        while let Ok((site, files, skipped)) = self.expand_rx.try_recv() {
            // 이 펼치기가 끝났다 — 상태 줄의 `펼치는 중`이 그만큼 줄어든다
            self.expanding = self.expanding.saturating_sub(1);
            for (local, remote, size) in files {
                self.queue
                    .enqueue(site, TransferDirection::Upload, local, remote, size);
            }
            // 읽지 못한 폴더가 있으면 그 사실을 알린다 — 조용히 빼면 사용자는
            // 그 파일들이 왜 큐에 없는지 알 길이 없다 (plan Edge Case)
            if skipped > 0 {
                self.toast.show(
                    format!("읽을 수 없는 폴더 {skipped}개는 건너뛰었습니다"),
                    now,
                );
            }
        }
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
        // 목록에 끌어다 놓은 것 (FR-38) — 큐에 넣는 것은 그리기가 끝난 뒤다
        let mut dropped = None;
        // 원격 목록에서 고른 메뉴 항목 (FR-39)
        let mut remote_menu = None;
        // 원격 트리가 청한 하위 조회 (T24)
        let mut tree_requests = Vec::new();
        // 사이트 관리자의 `연결(C)`이 쓸 분할 영역 — 모달은 CentralPanel 밖에서 그리므로
        // 안에서 정해지는 이 값을 밖으로 들고 나온다
        let mut layout_area = None;
        // 타이틀바를 먼저 그린다 — 남는 영역이 아래 CentralPanel의 몫이 된다 (FR-22)
        let titlebar_command = self.show_titlebar(ui, &ctx);
        // eframe이 주는 Ui는 여백·배경이 없다 — CentralPanel로 감싸야 바탕이 칠해진다.
        // **기본 여백은 끈다**(`Frame::NONE` + 바탕색): egui의 중앙 패널은 사방에 여백을 두는데,
        // 그만큼 패널 줄이 타이틀바 선에서 떨어져 뜬다(사용자 보고). 이 앱의 화면은 창
        // 가장자리까지 꽉 차는 탐색기 배치다 — 여백이 필요한 곳은 각 패널이 스스로 둔다
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(theme::WINDOW_BG))
            .show(ui, |ui| {
                if !self.korean_font {
                    ui.colored_label(
                        theme::TEXT_DIM,
                        "한글 글꼴을 불러오지 못해 기본 글꼴로 표시합니다",
                    );
                }
                if !self.shell_available() {
                    ui.colored_label(theme::TEXT_DIM, SHELL_UNAVAILABLE);
                }
                // 하단 상태 표시줄·도크를 사이드바보다 **먼저** 뗀다 — egui 패널은 먼저 그린 쪽이
                // 넓은 자리를 가져가므로, 순서를 뒤집으면 둘 다 사이드바를 뺀 폭에만 그려진다.
                // 창 폭 전체를 가로지르는 것이 디자인이다 (FR-36·FR-40)
                self.show_status_bar(ui);
                self.show_dock(ui);

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
                            tree: &self.tree_cache,
                        },
                    );
                    menu = outcome.menu;
                    panel_command = outcome.command;
                    remote_action = outcome.remote;
                    remote_url = outcome.remote_url;
                    closed_conns = outcome.closed_conns;
                    dropped = outcome.drop;
                    remote_menu = outcome.remote_menu;
                    tree_requests = outcome.tree_requests;
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
                // 끌어다 놓은 것을 큐에 넣는다 — 어느 패널에 놓였는지는 쓰지 않는다(항목의
                // 종류와 놓은 자리의 종류만으로 방향이 정해진다)
                if let Some((_, drop)) = dropped.take() {
                    self.apply_drop(drop);
                }
                // 원격 메뉴가 고른 것 — 대화가 필요한 것은 여기서 열리기만 한다
                if let Some((target, (action, targets))) = remote_menu.take() {
                    self.apply_remote_menu(target, action, targets);
                }
                // 원격 위치가 바뀐 패널은 목록을 다시 읽는다 — 트리 선택(T24 Acceptance ⑤)과
                // 상위 이동이 이 길을 함께 쓴다
                self.list_moved_panels();
                // 트리가 펼쳐진 폴더의 하위를 청한다 (T24)
                for (_, request) in tree_requests.drain(..) {
                    if let TreeRequest::Remote { conn, path } = request {
                        self.request_tree_children(conn, path);
                    }
                }
                // 마지막 원격 탭이 닫힌 연결을 접는다 — 워커와 소켓이 여기서 회수된다 (FR-32)
                for conn in closed_conns {
                    self.release_conn(conn);
                }
            });

        // 삭제 확인은 egui 모달이라 `CentralPanel` 밖에서 그려도 된다(자체 레이어를 쓴다)
        self.show_remove_confirm(&ctx);
        // 서버 지문 확인도 같다. 사용자가 고를 때까지 그 연결의 워커는 기다리고 있다 (D15)
        self.hostkey.show(&ctx);
        // 사이트 관리자도 자체 레이어를 쓴다 (FR-27)
        self.show_site_manager(&ctx, layout_area);
        self.sync_tray(ctx.input(|input| input.time));
        self.poll_tray(&ctx);
        self.show_settings_dialog(&ctx);
        // 원격 파일 작업 대화 (FR-39)
        self.show_remote_dialogs(&ctx);
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

/// 원격 파일 작업이 띄운 대화의 상태 (FR-39).
///
/// **대상 경로를 대화가 뜰 때 붙잡아 둔다** — 대화가 떠 있는 동안 목록이 다시 읽히거나
/// 선택이 바뀔 수 있는데, 그때 다시 읽으면 사용자가 고른 것과 **다른 항목**에 명령이 간다
#[derive(Debug, Default)]
struct RemoteOps {
    /// 어느 연결에 보낼 것인가
    conn: Option<ConnectionId>,
    /// 지금 뜬 대화
    dialog: Option<RemoteDialog>,
    /// 대화가 다루는 대상들
    targets: Vec<RemotePath>,
    /// 이름 입력값과 그 오류
    name: String,
    error: Option<String>,
    /// 권한 대화의 상태
    permissions: Permissions,
    octal: String,
    /// 삭제 대화의 재귀 여부
    recursive: bool,
}

/// 트리 조회의 세대 번호가 시작하는 자리.
///
/// 패널의 목록 조회는 0부터 하나씩 올라간다 — 두 번호가 겹치면 한쪽의 답을 다른 쪽이
/// 가져가 서로 영영 기다린다. 실제로 부딪히려면 패널이 이 값만큼 폴더를 옮겨야 한다
const TREE_LIST_BASE: u64 = 1 << 40;

/// 트리에 보일 차례로 줄을 세운다 — **목록과 같은 규칙**이라야 화면이 두 벌로 갈리지 않는다.
/// (`remote::tree_cache`는 `panel`을 모르므로 정렬은 이쪽에서 맞춰 넘긴다)
fn sort_tree_children(entries: &mut [crate::remote::types::RemoteEntry]) {
    entries.sort_by(|a, b| {
        crate::panel::file_list::compare_rows(a, "", b, "", crate::panel::file_list::SortKey::Name)
    });
}

/// 파일 작업 실패 사유가 상태 줄에 머무는 시간(초) — 알림(FR-43)보다 조금 길게 둔다
const NOTICE_SECS: f64 = 6.0;

/// 실패 사유 앞에 붙일 작업 이름 — 사용자가 시키지 않은 작업(`Cwd`·`Disconnect`)은 알리지 않는다
fn op_label(op: OpKind) -> Option<&'static str> {
    match op {
        OpKind::Mkdir => Some("새 폴더"),
        OpKind::Remove | OpKind::Rmdir => Some("삭제"),
        OpKind::Rename => Some("이름 바꾸기"),
        OpKind::Chmod => Some("권한 바꾸기"),
        OpKind::Cwd | OpKind::Disconnect => None,
    }
}

/// 파일 작업의 답을 어떻게 다룰지 (FR-39) — 화면·연결을 건드리지 않고 판정만 한다
#[derive(Debug, Clone, PartialEq, Eq)]
enum OpOutcome {
    /// 서버 쪽이 바뀌었다 — 목록을 다시 읽는다
    Relist,
    /// 실패 사유를 상태 줄과 로그에 남긴다
    Notice(String),
    /// 사용자가 시킨 작업이 아니라 알리지 않는다
    Ignore,
}

/// 작업 결과의 처리 방법을 정한다.
///
/// 실패해도 **사유만 남기고 앱은 그대로 돈다** — `SITE CHMOD`를 모르는 FTP 서버가 흔한데
/// 그때마다 앱이 멈추거나 연결이 끊기면 쓸 수 없다 (D22)
fn op_outcome(op: OpKind, result: Result<(), RemoteError>) -> OpOutcome {
    let Some(label) = op_label(op) else {
        return OpOutcome::Ignore;
    };
    match result {
        Ok(()) => OpOutcome::Relist,
        Err(err) => OpOutcome::Notice(format!("{label} 실패 — {err}")),
    }
}

/// 대화의 결론을 상태에 반영한다 (FR-39).
///
/// 확인이면 그 값을 내주고 대화를 닫는다. **취소도 똑같이 닫는다** — 이 한 줄이 빠져서
/// 취소 단추가 아무 일도 하지 않았다(spec 리뷰 M1). 아직 고르지 않았으면 그대로 둔다
fn settle_dialog<T>(outcome: DialogOutcome<T>, dialog: &mut Option<RemoteDialog>) -> Option<T> {
    match outcome {
        DialogOutcome::Pending => None,
        DialogOutcome::Confirmed(value) => {
            *dialog = None;
            Some(value)
        }
        DialogOutcome::Cancelled => {
            *dialog = None;
            None
        }
    }
}

/// 확인을 마친 삭제가 보낼 명령 (FR-39).
///
/// **이 함수를 부르는 곳은 확인 대화가 `Some`을 돌려준 자리 하나뿐이다** — 메뉴에서 곧바로
/// 삭제로 가는 길은 없다(plan Halt Forecast)
fn delete_command(path: RemotePath, recursive: bool) -> ConnCommand {
    if recursive {
        ConnCommand::Rmdir(path)
    } else {
        ConnCommand::Remove(path)
    }
}

/// 원격 메뉴가 짝으로 삼는 로컬 패널의 상태 — 보고 있는 폴더와 그 안에서 고른 것들
struct LocalSide {
    dir: PathBuf,
    selected: Vec<(PathBuf, bool)>,
}

/// 지금 뜬 원격 대화의 종류
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RemoteDialog {
    Rename,
    NewFolder,
    Chmod,
    Delete,
}

/// 로컬 폴더를 펼친 결과 — 올릴 파일들과 **읽지 못해 건너뛴 폴더 수**.
///
/// 이름을 `transfer::Expanded`와 달리 두는 이유: 그쪽은 뿌리 기준 상대 경로를 들고
/// 이쪽은 사이트와 서버 경로까지 든다 — 같은 이름이면 오가며 읽을 때 헷갈린다
///
/// 건너뛴 것을 함께 나르는 이유: 권한 없는 폴더 하나 때문에 나머지를 버리지는 않지만,
/// 조용히 빼면 사용자는 그 파일들이 왜 큐에 없는지 알 길이 없다 (plan Edge Case)
type ExpandResult = (SiteId, Vec<(PathBuf, RemotePath, u64)>, usize);

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
            vec![TabSpec::Local(PathBuf::from(r"D:\"))],
            "walk 순서상 앞에 온 새 패널이 첫 번째로 저장된다"
        );
        assert_eq!(
            state.panels[1].tabs,
            vec![TabSpec::Local(PathBuf::from(r"C:\"))]
        );

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

    #[test]
    fn 작업이_성공하면_목록을_다시_읽고_실패하면_사유가_남는다() {
        // Acceptance ② — 성공 응답 뒤에는 서버를 다시 읽는다. 앱이 짐작해 그리면 실제와 어긋난다
        for op in [OpKind::Mkdir, OpKind::Rename, OpKind::Remove, OpKind::Rmdir] {
            assert_eq!(op_outcome(op, Ok(())), OpOutcome::Relist, "{op:?}");
        }
        // Acceptance ④ — SITE CHMOD를 모르는 서버(D22)의 답은 사유로 남고, 그것으로 끝이다
        let unsupported = RemoteError::Unsupported {
            operation: "SITE CHMOD".to_owned(),
            detail: "500 Unknown command".to_owned(),
        };
        let OpOutcome::Notice(text) = op_outcome(OpKind::Chmod, Err(unsupported)) else {
            panic!("실패가 사유로 남지 않았다");
        };
        assert!(text.starts_with("권한 바꾸기 실패"), "{text}");
        assert!(text.contains("SITE CHMOD"), "서버 원문이 빠졌다: {text}");
        // 사용자가 시키지 않은 작업까지 알리면 상태 줄이 잡음으로 찬다
        assert_eq!(
            op_outcome(
                OpKind::Cwd,
                Err(RemoteError::Protocol {
                    detail: "x".to_owned()
                })
            ),
            OpOutcome::Ignore
        );
    }

    #[test]
    fn 삭제는_재귀_여부에_따라_다른_명령이_된다() {
        // Acceptance ① — 확인 대화가 돌려준 값만 이 함수에 들어온다
        let path = RemotePath::new("/var/www/old");
        assert_eq!(
            delete_command(path.clone(), false),
            ConnCommand::Remove(path.clone())
        );
        assert_eq!(delete_command(path.clone(), true), ConnCommand::Rmdir(path));
    }

    #[test]
    fn 취소한_대화는_그_자리에서_닫힌다() {
        // spec 리뷰 M1의 회귀 방지선 — 취소를 "아직 안 골랐다"와 같이 다루면 다음 프레임에
        // 같은 대화가 다시 떠 빠져나올 수 없다
        let mut dialog = Some(RemoteDialog::Rename);
        assert_eq!(
            settle_dialog(DialogOutcome::<String>::Pending, &mut dialog),
            None
        );
        assert_eq!(dialog, Some(RemoteDialog::Rename), "고르기 전에 닫혔다");

        assert_eq!(
            settle_dialog(DialogOutcome::<String>::Cancelled, &mut dialog),
            None
        );
        assert_eq!(dialog, None, "취소했는데 대화가 남았다");

        let mut dialog = Some(RemoteDialog::Chmod);
        assert_eq!(
            settle_dialog(DialogOutcome::Confirmed(0o755), &mut dialog),
            Some(0o755)
        );
        assert_eq!(dialog, None, "확인 뒤에도 대화가 남았다");
    }

    #[test]
    fn 트레이가_켜져_있을_때만_닫기를_숨김으로_바꾼다() {
        // Acceptance — 토글이 꺼져 있으면 `✕`는 종전대로 종료한다
        assert!(
            should_hide_on_close(true, true, false),
            "숨겨야 하는데 종료한다"
        );
        assert!(
            !should_hide_on_close(false, true, false),
            "토글이 꺼졌는데 숨긴다"
        );
    }

    #[test]
    fn 아이콘이_없으면_숨기지_않는다() {
        // 아이콘 없이 숨기면 창을 되부를 방법이 사라진다 — 작업 관리자로 죽여야 한다
        assert!(
            !should_hide_on_close(true, false, false),
            "트레이 아이콘이 없는데 창을 숨긴다"
        );
    }

    #[test]
    fn 트레이_메뉴_종료는_가로채지_않는다() {
        // 그 닫기는 트레이로 보내는 것이 아니라 진짜 종료다 — 가로채면 앱을 끌 수 없다
        assert!(
            !should_hide_on_close(true, true, true),
            "종료 중인데 창만 숨긴다"
        );
    }
}
