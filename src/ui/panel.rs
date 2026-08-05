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
use crate::fs::thumbnail::ThumbnailCache;
use crate::fs::watcher::DirWatcher;
use crate::panel::tabs::{CloseOutcome, TabPhase, TabSource, TabState, TabsModel};
use crate::remote::connection::{ConnCommand, ConnectionId};
use crate::remote::manager::ConnectionManager;
use crate::remote::types::{RemoteEntry, RemotePath, SiteId};
use crate::remote::url::RemoteUrl;
use crate::ui::address_bar::{AddressBar, NavAction};
use crate::ui::file_list::{FileListAction, FileListView};
use crate::ui::icon_tex::{IconTextures, ThumbnailTextures};
use crate::ui::list_common::{DropOutcome, DropTarget, FileDrag};
use crate::ui::menu::{Command, PanelMenuState};
use crate::ui::remote_menu::{self, RemoteMenuAction, RemoteTarget};
use crate::ui::remote_states::{self, FailedAction, RemoteView};
use crate::ui::session::TabSpec;
use crate::ui::shell_host;
use crate::ui::tabs::TabAction;
use crate::ui::theme;
use crate::ui::tree::{FolderTreeView, TREE_WIDTH, TreeChoice, TreeRequest, TreeSource};
use crate::ui::view_mode::ViewMode;
use eframe::egui;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, TryRecvError, channel};
use std::time::Duration;

/// 트리와 목록을 가르는 세로 선 두께 — 현행 판 트리의 테두리(`WS_EX_CLIENTEDGE`)를 대신한다
/// 트리 토글 라벨 — 원격 패널에서는 갈린다 (인벤토리 #94)
const LOCAL_TREE_LABEL: &str = "폴더 트리";
const REMOTE_TREE_LABEL: &str = "원격 트리";

const TREE_BORDER: f32 = 1.0;

/// 트리 영역 안쪽 여백 — 항목이 패널 가장자리에 붙지 않게 한다
const TREE_PAD: f32 = 4.0;

/// 썸네일 도착을 확인하러 스스로 깨어나는 간격 (FR-24).
///
/// 매 프레임 깨우면 만드는 동안 앱이 쉬지 않고 그려 배터리를 먹고, 너무 길면 사진이
/// 뒤늦게 뜬다. 20장 남짓이 수백 ms 안에 만들어지므로 그 사이 몇 번 확인하는 값으로 잡았다
const THUMB_POLL_INTERVAL: Duration = Duration::from_millis(50);

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

/// 원격 목록 메뉴에서 고른 것과 그때의 대상들 (FR-39)
pub type RemoteMenuPick = (RemoteMenuAction, Vec<RemoteTarget>);

/// 팝업이 화면 밖으로 나가지 않게 시작점을 안으로 당긴다 (quality 리뷰 m1).
///
/// 화면보다 큰 팝업이면 왼쪽·위쪽 모서리를 우선한다 — 아래가 잘려도 첫 줄은 보인다
fn clamp_menu_pos(screen: egui::Rect, at: egui::Pos2, size: egui::Vec2) -> egui::Pos2 {
    egui::pos2(
        at.x.min(screen.right() - size.x).max(screen.left()),
        at.y.min(screen.bottom() - size.y).max(screen.top()),
    )
}

/// `show_content`가 한 프레임에서 거둔 것들 — 목록 조작·단계 화면의 조치·놓기·원격 메뉴
type ContentOutcome = (
    FileListAction,
    Option<RemoteAction>,
    Option<DropOutcome>,
    Option<RemoteMenuPick>,
);

/// 패널이 상위(레이아웃)에 올려보내는 요청.
/// 전부 이 패널을 그리는 도중에는 실행할 수 없어 값으로 돌려준다
pub struct PanelOutcome {
    pub menu: Option<MenuRequest>,
    /// 패널 메뉴에서 고른 명령 — 대상은 **이 패널**이다 (plan D16)
    pub command: Option<Command>,
    /// 원격 단계 화면에서 고른 조치 (FR-29·FR-32)
    pub remote: Option<RemoteAction>,
    /// 주소창에 적은 원격 주소 (FR-34) — 사이트로 해소해 새 탭을 여는 것은 앱의 몫이다.
    ///
    /// 패널이 직접 열지 못하는 이유: 주소에는 `SiteId`가 없고, 어느 사이트로 볼지(이미 등록된
    /// 서버인지 새로 만들지)는 사이트 목록을 쥔 앱만 판정할 수 있다
    pub remote_url: Option<RemoteUrl>,
    /// 마지막 원격 탭이 닫혀 이 패널이 더 쓰지 않게 된 연결 (FR-32).
    ///
    /// `remote`와 **따로 두는 이유**: 한 프레임에 둘 다 일어날 수 있는데 한 필드로 합치면
    /// 먼저 채워진 쪽에 밀려 연결이 닫히지 않은 채 워커와 소켓이 남는다
    pub closed_conn: Option<ConnectionId>,
    /// 이 패널의 목록에 끌어다 놓은 것 (FR-38) — 큐에 넣는 것은 앱의 몫이다
    pub drop: Option<DropOutcome>,
    /// 원격 목록 우클릭 메뉴에서 고른 것과 그 대상들 (FR-39)
    pub remote_menu: Option<RemoteMenuPick>,
    /// 원격 트리가 청한 하위 조회들 (FR-9 원격판) — 연결에 보내는 것은 앱이다.
    ///
    /// **여럿을 그대로 올린다** — 한 프레임에 형제 노드 여럿이 펼쳐져 있으면 요청도 여럿이다.
    /// 하나로 압착하면 나머지는 그 프레임에서 버려져 노드마다 프레임이 하나씩 밀린다
    /// (quality 리뷰 M1)
    pub tree_requests: Vec<TreeRequest>,
}

/// 원격 단계 화면에서 사용자가 고른 조치 — 실행은 앱이 한다(연결을 앱이 쥔다)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemoteAction {
    /// 실패한 연결을 다시 건다 (인벤토리 #18)
    Retry,
    /// 사이트 관리자를 연다 (인벤토리 #19)
    OpenSettings,
    /// 서버 로그를 보인다 (인벤토리 #20)
    ViewLog,
    /// 연결 중인 것을 그만둔다 (인벤토리 #21)
    CancelConnect,
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
    /// 주소창에 적힌 원격 주소 — 이번 프레임의 결과로 앱에 올려 보낸다
    pending_remote_url: Option<RemoteUrl>,
    /// 원격 목록 우클릭 메뉴가 뜰 자리 — `None`이면 닫혀 있다 (FR-39)
    remote_menu_at: Option<egui::Pos2>,
    /// 아직 조회를 청하지 않은 "직전에 보고 있던 곳" — `set_remote_path`가 세우고
    /// 곧이어 `request_remote_list`가 세대와 함께 `revert_at`으로 옮긴다
    pending_revert: Option<RemotePath>,
    /// 되돌릴 자리 — `(그 조회의 세대, 돌아갈 곳, 옮겨 간 곳)`.
    ///
    /// **요청 하나에 묶는다**(F-7 2라운드 B1·B2): 세대만 보고 되돌리면 ⓐ 이미 성공한 이동의
    /// 자리가 남아 나중의 새로 고침 실패가 옛 폴더로 되돌리고 ⓑ 같은 연결의 다른 패널·탭까지
    /// 함께 되돌아간다. 그래서 **성공하면 지우고**, 되돌릴 때는 지금 보고 있는 곳이
    /// `옮겨 간 곳` 그대로일 때만 손댄다
    revert_at: Option<(u64, RemotePath, RemotePath)>,
    /// 원격 위치가 바뀌어 목록을 다시 읽어야 한다 — 앱이 다음 프레임에 거둬 간다.
    ///
    /// **위치를 옮기는 것과 서버에 묻는 것은 다른 일이다** — 옮기는 쪽(트리 선택·상위 이동)은
    /// 연결을 모르고, 명령을 보내는 쪽(`ConnectionManager`)은 앱이 쥐고 있다. 그 사이를
    /// 이 깃발이 잇는다(spec 리뷰 B1: 옮기기만 하고 아무도 다시 읽지 않던 자리)
    remote_dirty: bool,
    /// 원격 목록 요청의 세대 — 늦게 도착한 이전 요청의 결과를 버린다 (D7).
    /// 로컬 열거의 `DirLoad`가 쓰는 것과 같은 기법이다
    remote_generation: u64,
    /// 썸네일 픽셀 캐시 (FR-24) — 상한이 패널당이라 패널이 소유한다 (NFR-9)
    thumbs: ThumbnailCache,
    /// 올라간 썸네일 텍스처 — 픽셀 캐시와 함께 비워진다
    thumb_textures: ThumbnailTextures,
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
            pending_remote_url: None,
            remote_menu_at: None,
            pending_revert: None,
            revert_at: None,
            remote_dirty: false,
            remote_generation: 0,
            thumbs: ThumbnailCache::new(),
            thumb_textures: ThumbnailTextures::new(),
        }
    }

    /// 저장된 탭 구성과 열 폭으로 패널을 되살린다 (FR-11). 탭 목록이 비면 `None`.
    ///
    /// 히스토리는 복원하지 않는다 — 세션에는 경로만 저장한다(현행과 같은 규칙).
    /// `columns`가 비면 기본 폭으로 시작한다
    pub fn from_tabs(
        tabs: Vec<TabSpec>,
        active_tab: usize,
        columns: &[f32],
        view_mode: &str,
    ) -> Option<PanelState> {
        // 원격 탭은 **연결 없이** 되살아난다 — 연결은 사용자가 연다 (FR-44)
        let states: Vec<TabState> = tabs
            .into_iter()
            .map(|tab| match tab {
                TabSpec::Local(path) => TabState::new(path),
                TabSpec::Remote { site, path } => TabState::remote(site, path),
            })
            .collect();
        let model = TabsModel::from_tabs(states, active_tab)?;
        let start = model.active().committed().to_path_buf();
        let mut panel = PanelState::new(start);
        panel.tabs = model;
        if !columns.is_empty() {
            panel.list.set_columns(columns);
        }
        if !view_mode.is_empty() {
            panel.set_view_mode(ViewMode::from_key(view_mode));
        }
        Some(panel)
    }

    /// 현재 표시 중인 폴더 — 활성 탭이 커밋한 경로가 정본이다
    pub fn dir(&self) -> &Path {
        self.tabs.active().committed()
    }

    /// 세션 저장용 — 탭들이 가리키는 곳(탭 순서). 원격 탭은 사이트와 원격 경로로 담긴다
    pub fn tab_specs(&self) -> Vec<TabSpec> {
        self.tabs
            .sources()
            .iter()
            .map(|source| match source {
                TabSource::Local(path) => TabSpec::Local(path.clone()),
                TabSource::Remote { site, path, .. } => TabSpec::Remote {
                    site: *site,
                    path: path.clone(),
                },
            })
            .collect()
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
        if let Some(delay) = self.poll_thumbnails(ctx) {
            ctx.request_repaint_after(delay);
        }
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
        // 원격 탭을 보는 동안에는 로컬 감시 통지를 처리하지 않는다 — 이전 폴더의 감시가
        // 아직 살아 있을 수 있는데, 그 통지로 로컬 열거를 걸면 원격 화면이 로컬 목록으로 덮인다
        if self.is_remote() {
            return;
        }
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
        self.apply_enumerated(outcome, icons);
    }

    /// 열거 결과 하나를 상태에 반영한다.
    ///
    /// `poll_load`에서 갈라낸 이유는 **테스트가 이 경로를 실제로 지나게** 하기 위해서다 —
    /// 판정 헬퍼만 직접 부르는 테스트는 호출부가 죽어도 통과한다(F-7 B1이 그렇게 새어나갔다)
    fn apply_enumerated(&mut self, outcome: EnumOutcome, icons: &mut IconCache) {
        match outcome {
            EnumOutcome::Ok(entries) => {
                // 여기서 비로소 커밋한다 — 이 지점 전에는 화면이 이전 폴더를 유지한다.
                // **이전 경로를 커밋 전에 잡아 둔다** — 커밋한 뒤에 비교하면 항상 같아져
                // "폴더가 바뀌었다"가 영영 성립하지 않는다
                let dir = std::mem::take(&mut self.pending_dir);
                let tab = self.tabs.active_mut();
                tab.set_committed(dir.clone());
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
                // 폴더가 바뀌면 이전 폴더의 썸네일을 즉시 놓는다 (NFR-9).
                // **판정은 캐시가 한다** — 탐색·탭 전환·탭 닫기가 각자 다른 순서로 폴더를
                // 바꾸므로, 여기서 비교하면 한 경로만 빠뜨려도 조용히 새어나간다(F-7 B1·m1).
                // 이 해제는 세대를 올리는 지점이기도 해서, 늦게 도착한 이전 폴더의 결과도 함께 걸러진다
                if self.thumbs.set_folder(&dir) {
                    self.thumb_textures.clear();
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

    /// 도착한 썸네일을 텍스처로 올린다 (FR-24).
    ///
    /// 채널을 비운 뒤 **픽셀 캐시 전체와 동기화**한다 — 방금 도착한 것만 올리면
    /// 프레임 상한에 걸린 경로가 영영 올라가지 못하고, 축출된 픽셀의 텍스처도 남는다
    /// 다시 그려야 하면 그 지연을 돌려준다. `None`이면 그릴 이유가 없다.
    ///
    /// `Duration::ZERO`는 **이번 프레임에 올린 것이 있다**는 뜻이고, 짧은 지연은
    /// **워커 결과를 기다린다**는 뜻이다. 열거(`DirLoad`)는 워커가 `ctx`를 들고 있어
    /// 직접 깨우지만, 썸네일 워커는 `fs` 계층이라 egui를 모른다(AGENTS: 의존 단방향) —
    /// 그래서 화면 쪽이 스스로 깨어나 채널을 확인한다.
    ///
    /// **판정을 값으로 돌려주는 이유**: 여기서 `ctx`에 직접 요청하면 `load_texture`가
    /// 스스로 일으키는 repaint와 섞여 테스트가 둘을 구분할 수 없다. 이 신호가 빠지면
    /// 워커가 늦게 준 썸네일이 사용자가 마우스를 움직일 때까지 형식 아이콘에 머문다
    /// (F-8 화면 확인에서 실제로 그랬다)
    fn poll_thumbnails(&mut self, ctx: &egui::Context) -> Option<Duration> {
        let arrived = self.thumbs.poll();
        let before = self.thumb_textures.len();
        self.thumb_textures.sync(ctx, &self.thumbs);
        // 프레임 상한(`MAX_NEW_THUMBS_PER_FRAME`)에 걸려 남은 것도 이 신호로 이어 올라간다
        if !arrived.is_empty() || self.thumb_textures.len() != before {
            return Some(Duration::ZERO);
        }
        self.thumbs.is_pending().then_some(THUMB_POLL_INTERVAL)
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
        // 새 탭은 연결을 접을 일이 없다
        self.handle_tab(TabAction::New, ctx);
    }

    /// 활성 탭을 닫는다. 그 탭이 이 패널의 마지막 원격 탭이었으면 접어야 할 연결을 돌려준다 (FR-32)
    pub fn close_tab(&mut self, ctx: &egui::Context) -> Option<ConnectionId> {
        self.handle_tab(TabAction::Close(self.tabs.active_index()), ctx)
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

    /// 보고 있는 폴더를 다시 읽는다 — 경로도 히스토리도 그대로다.
    ///
    /// **원격 탭에서는 로컬 열거를 걸지 않는다** — 원격 목록은 연결 워커에게 다시 물어야 하고
    /// (`request_remote_list`), 그 배선은 연결 화면과 함께 들어온다(T10)
    pub fn refresh(&mut self, ctx: &egui::Context) {
        let Some(dir) = self.tabs.active().source.local_path() else {
            return;
        };
        let dir = dir.to_path_buf();
        self.start_load(dir, PendingNav::None, ctx);
    }

    /// 지금 쓰는 보기 모드 — 메뉴가 현재 표시를 그리는 데 쓴다 (FR-23)
    pub fn view_mode(&self) -> ViewMode {
        self.list.view_mode()
    }

    /// 보기 모드를 바꾼다 (FR-23)
    pub fn set_view_mode(&mut self, mode: ViewMode) {
        self.list.set_view_mode(mode);
    }

    /// 활성 탭이 원격을 가리키는가 — 로컬에만 있는 일(열거·감시·썸네일·새 파일)이 이것으로 갈린다
    pub fn is_remote(&self) -> bool {
        self.tabs.active().source.is_remote()
    }

    /// 활성 탭이 쓰는 연결 — 로컬 탭이거나 아직 연결하지 않았으면 `None`
    pub fn active_conn(&self) -> Option<ConnectionId> {
        match &self.tabs.active().source {
            TabSource::Remote { conn, .. } => *conn,
            TabSource::Local(_) => None,
        }
    }

    /// 활성 탭이 보고 있는 로컬 폴더 — 원격 탭이면 `None`
    pub fn local_dir(&self) -> Option<PathBuf> {
        self.tabs
            .active()
            .source
            .local_path()
            .map(Path::to_path_buf)
    }

    /// 로컬 목록에서 고른 항목들 — 원격 탭이면 빈 벡터다 (FR-39 `올리기`)
    pub fn selected_local(&self) -> Vec<(PathBuf, bool)> {
        if self.is_remote() {
            return Vec::new();
        }
        self.list.selected_local()
    }

    /// 활성 탭이 보고 있는 원격 폴더 — 로컬 탭이면 `None`
    pub fn remote_dir(&self) -> Option<RemotePath> {
        self.tabs.active().source.remote_path().cloned()
    }

    /// 이 패널의 탭 중 그 연결을 쓰는 것이 있는가 — 연결을 접어도 되는지 판정한다 (FR-32)
    fn uses_conn(&self, target: ConnectionId) -> bool {
        self.tabs.sources().iter().any(|source| {
            matches!(source, TabSource::Remote { conn: Some(conn), .. } if *conn == target)
        })
    }

    /// 사이트를 가리키는 **새 원격 탭**을 열고 활성으로 만든다 (FR-33·FR-34·FR-38).
    ///
    /// 연결 붙이기는 이어서 `attach_conn`이 한다 — 탭을 먼저 만드는 이유는 연결이 서기 전에도
    /// 그 자리에 탭이 보여야 사용자가 "열리고 있다"는 것을 알기 때문이다
    pub fn open_remote_tab(&mut self, site: SiteId, path: RemotePath) {
        self.tabs.add(TabState::remote(site, path));
    }

    /// 활성 원격 탭에 연결을 붙이고 연결 중으로 표시한다 — 사이트를 막 열었을 때.
    /// 원격 탭이 아니면 아무 일도 하지 않고 `false`
    pub fn attach_conn(&mut self, id: ConnectionId) -> bool {
        let TabSource::Remote { conn, phase, .. } = &mut self.tabs.active_mut().source else {
            return false;
        };
        *conn = Some(id);
        *phase = TabPhase::Connecting;
        true
    }

    /// 그 연결을 쓰는 **모든 탭**의 단계를 바꾼다 — 워커의 단계 변화를 화면에 투영한다.
    ///
    /// 활성 탭만 바꾸지 않는 이유: 한 연결을 여러 탭이 나눠 쓸 수 있고(같은 서버의 다른 폴더),
    /// 배경 탭이 옛 단계로 남으면 그 탭으로 돌아갔을 때 화면이 실제와 어긋난다
    pub fn set_phase_for(&mut self, target: ConnectionId, next: &TabPhase) -> bool {
        let mut changed = false;
        for source in self.tabs.sources_mut() {
            if let TabSource::Remote {
                conn: Some(conn),
                phase,
                ..
            } = source
                && *conn == target
            {
                *phase = next.clone();
                changed = true;
            }
        }
        changed
    }

    /// 표시 중인 폴더에 새 폴더를 만든다 (FR-25).
    /// **원격 탭에서는 아무것도 하지 않는다** — 원격 폴더 만들기는 원격 파일 작업(T23)이 맡는다
    pub fn new_folder(&mut self, ctx: &egui::Context) {
        if self.is_remote() {
            return;
        }
        let dir = self.dir().to_path_buf();
        self.create.start(dir, "폴더", create::new_folder, ctx);
    }

    /// 표시 중인 폴더에 빈 텍스트 문서를 만든다 (FR-25). 원격 탭에서는 하지 않는다
    pub fn new_file(&mut self, ctx: &egui::Context) {
        if self.is_remote() {
            return;
        }
        let dir = self.dir().to_path_buf();
        self.create.start(dir, "파일", create::new_text_file, ctx);
    }

    /// 활성 원격 탭이 가리키는 위치를 옮긴다. 연결·단계는 그대로 둔다.
    ///
    /// 목록 다시 읽기는 **깃발을 세워** 앱에 맡긴다(`take_remote_dirty`) — 여기서 직접 보내지
    /// 않는 이유는 패널이 `ConnectionManager`를 쥐고 있지 않기 때문이다
    pub fn set_remote_path(&mut self, target: RemotePath) {
        if let TabSource::Remote { path, .. } = &mut self.tabs.active_mut().source {
            // 실패했을 때 돌아갈 자리를 남긴다 — 세대는 조회를 청할 때 붙는다 (F-7 리뷰 B2)
            self.pending_revert = Some(path.clone());
            *path = target;
            self.remote_dirty = true;
        }
    }

    /// 활성 탭이 바뀌었으니 그 탭이 보는 곳을 다시 읽는다 (F-7 3라운드 B1).
    ///
    /// **목록은 탭이 아니라 패널 하나가 든다** — 그래서 탭만 바꾸고 목록을 그대로 두면
    /// 주소창은 이 탭을, 목록은 저 탭의 폴더를 보이게 된다. 그 위에서 연 원격 메뉴는
    /// **화면에 없는 경로**에 삭제·권한 변경을 걸 수 있다(로컬은 종전부터 다시 읽어 왔고,
    /// 원격만 빠져 있었다)
    fn reload_active_tab(&mut self, ctx: &egui::Context) {
        match self.tabs.active().source.local_path() {
            Some(path) => {
                let path = path.to_path_buf();
                self.start_load(path, PendingNav::None, ctx);
            }
            None => {
                // 원격은 연결을 아는 앱이 청한다 — 여기서는 깃발만 세운다.
                // **답이 오기 전까지 목록을 비운다** — 옛 탭의 항목을 남겨 두면 그 몇 백 밀리초
                // 동안(또는 조회가 실패하면 계속) 주소창과 목록이 다른 곳을 가리키고,
                // 그 위에서 연 메뉴가 화면에 없는 경로에 삭제·권한 변경을 건다 (F-7 4라운드 M1)
                self.list.clear_entries();
                self.remote_dirty = true;
            }
        }
    }

    /// 그 세대의 조회가 실패해 옮기기를 무른다 — 자리가 없거나 **다른 요청의 것**이면 그대로 둔다.
    ///
    /// 지금 보고 있는 곳이 `옮겨 간 곳` 그대로일 때만 손댄다 — 그 사이 탭을 바꿨거나 다시
    /// 옮겼으면 이 되돌리기는 이미 과거의 것이다(F-7 2라운드 B2).
    /// 되돌린 뒤에는 **다시 읽지 않는다**(`remote_dirty`를 세우지 않는다) — 그 폴더의 목록은
    /// 이미 화면에 있고, 다시 청하면 실패한 조회와 성공한 조회가 번갈아 도는 고리가 된다
    pub fn revert_remote_path(&mut self, generation: u64) -> bool {
        let Some((waiting, previous, moved_to)) = self.revert_at.clone() else {
            return false;
        };
        if waiting != generation {
            return false;
        }
        let TabSource::Remote { path, .. } = &mut self.tabs.active_mut().source else {
            return false;
        };
        if *path != moved_to {
            return false;
        }
        *path = previous;
        self.revert_at = None;
        true
    }

    /// 옮긴 뒤 아직 다시 읽지 않았는가 — 앱이 프레임마다 거둬 `request_remote_list`로 잇는다
    pub fn take_remote_dirty(&mut self) -> bool {
        std::mem::take(&mut self.remote_dirty)
    }

    /// 활성 원격 탭의 목록을 요청한다. 돌려주는 값은 이번 요청의 세대다.
    ///
    /// **연결이 없으면 아무것도 보내지 않는다**(plan Edge Case) — 아직 연결하지 않은 탭에서
    /// 새로 고침을 눌러도 서버로 나가는 것이 없어야 한다
    pub fn request_remote_list(&mut self, manager: &ConnectionManager) -> Option<u64> {
        let TabSource::Remote {
            conn: Some(conn),
            path,
            ..
        } = &self.tabs.active().source
        else {
            return None;
        };
        let (conn, path) = (*conn, path.clone());
        self.remote_generation += 1;
        let generation = self.remote_generation;
        // 돌아갈 자리를 **이 요청에** 묶는다. 옮기기가 아닌 조회(새로 고침·작업 후 재조회)는
        // 돌아갈 곳이 없으므로 그 자리도 비운다 — 남겨 두면 그 실패가 엉뚱한 되돌리기를 부른다
        self.revert_at = self
            .pending_revert
            .take()
            .map(|previous| (generation, previous, path.clone()));
        manager
            .send(conn, ConnCommand::List { generation, path })
            .then_some(generation)
    }

    /// 이 패널이 그 목록 답을 기다리고 있는가 — 세대와 **보고 있는 위치**가 모두 맞아야 한다.
    ///
    /// 세대만 보지 않는 이유: 세대 번호는 패널마다 따로 세어지므로, 한 연결을 두 패널이
    /// 나눠 쓰면 우연히 겹쳐 남의 답을 제 목록으로 삼을 수 있다
    pub fn awaits_remote_list(&self, generation: u64, path: &RemotePath) -> bool {
        generation == self.remote_generation
            && self.tabs.active().source.remote_path() == Some(path)
    }

    /// 워커가 돌려준 원격 목록을 반영한다.
    ///
    /// 세대가 지금 것과 다르면 버린다(늦게 도착한 이전 폴더의 결과 — D7).
    /// **`..`는 언제나 첫 줄에 하나만 둔다** — 서버가 주기도 하고 안 주기도 해서 목록이
    /// 서버마다 달라 보이면 안 된다
    pub fn apply_remote_listed(
        &mut self,
        generation: u64,
        path: &RemotePath,
        entries: Vec<RemoteEntry>,
        icons: &mut IconCache,
    ) -> bool {
        if !self.awaits_remote_list(generation, path) {
            return false;
        }
        // 이 이동은 섰다 — 돌아갈 자리를 지운다. 남겨 두면 **나중의 무관한 조회 실패**가
        // 그 낡은 값으로 경로만 되돌린다(F-7 2라운드 B1)
        if matches!(&self.revert_at, Some((waiting, _, _)) if *waiting == generation) {
            self.revert_at = None;
        }
        self.list
            .set_remote_entries(with_parent_first(entries), icons);
        true
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
                // 만드는 동안 사용자가 원격 탭으로 옮겨 갔을 수 있다 — 그때 활성 탭 기준으로
                // 다시 읽으면 빈 경로를 열거하게 된다. 로컬 탭일 때만 다시 읽는다
                if let Some(dir) = self.tabs.active().source.local_path() {
                    let dir = dir.to_path_buf();
                    self.start_load(dir, PendingNav::None, ctx);
                }
            }
            Err(error) => {
                self.status = format!("새 {kind}을(를) 만들지 못했습니다 — {error}");
            }
        }
    }

    /// 탭 스트립에서 올라온 조작 처리.
    ///
    /// 닫힌 탭이 쓰던 연결을 **이 패널에서 아무도 쓰지 않게 되면** 그 연결 식별자를 돌려준다 —
    /// 실제로 접는 것은 연결을 쥔 앱의 몫이다 (FR-32)
    fn handle_tab(&mut self, action: TabAction, ctx: &egui::Context) -> Option<ConnectionId> {
        match action {
            TabAction::Switch(index) => {
                if self.tabs.switch(index) {
                    // 탭마다 소스가 다르므로 전환하면 그 탭의 것을 다시 읽는다.
                    // 히스토리는 이미 그 탭의 것이라 손대지 않는다
                    self.reload_active_tab(ctx);
                }
            }
            TabAction::Close(index) => {
                // 보고 있던 탭을 닫을 때만 화면이 바뀐다 — 배경 탭을 닫으면 그대로 유지된다
                let was_active = index == self.tabs.active_index();
                // 닫기 전에 잡아 둔다 — 닫은 뒤에는 그 탭이 무엇을 쓰던 탭인지 알 수 없다
                let closing = self
                    .tabs
                    .sources()
                    .get(index)
                    .and_then(|source| match source {
                        TabSource::Remote { conn, .. } => *conn,
                        TabSource::Local(_) => None,
                    });
                if let CloseOutcome::Removed(_) = self.tabs.close(index) {
                    if was_active {
                        self.reload_active_tab(ctx);
                    }
                    // 이 패널의 마지막 원격 탭이었으면 연결을 접는다 (FR-32·README §3)
                    if let Some(conn) = closing
                        && !self.uses_conn(conn)
                    {
                        return Some(conn);
                    }
                }
                // LastTab이면 아무것도 하지 않는다 — 패널의 마지막 탭은 남는다
            }
            TabAction::New => {
                // 새 탭은 지금 보고 있는 곳을 복제해 연다 (탐색기 관례).
                // 원격 탭이면 **같은 원격 위치**를 복제한다 — 로컬 폴더로 떨어지면 맥락이 끊긴다
                match self.tabs.active().source.clone() {
                    TabSource::Local(path) => {
                        self.tabs.add(TabState::new(path.clone()));
                        self.start_load(path, PendingNav::None, ctx);
                    }
                    TabSource::Remote { site, path, .. } => {
                        self.tabs.add(TabState::remote(site, path));
                    }
                }
            }
        }
        None
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
            NavAction::Up => match &self.tabs.active().source {
                TabSource::Local(path) => {
                    if let Some(parent) = path.parent().map(PathBuf::from) {
                        self.navigate(parent, ctx);
                    }
                }
                // 원격은 원격 경로로 올라간다. **루트에서는 그대로 머문다** — `parent()`가
                // `None`을 주므로 아무것도 하지 않는 것이 곧 "루트에 머문다"이다
                TabSource::Remote { path, .. } => {
                    if let Some(parent) = path.parent() {
                        self.set_remote_path(parent);
                    }
                }
            },
            NavAction::Goto(path) => self.navigate(path, ctx),
            // 주소로 여는 일은 앱이 한다 — 여기서는 값만 받아 둔다
            NavAction::GotoRemote(url) => self.pending_remote_url = Some(url),
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
                // 원격 항목은 로컬 경로가 없다 — 여는 일은 원격 탐색·전송(T13·T22)이 맡는다.
                // 여기서 빈 경로에 이름을 이어 붙이면 있지도 않은 로컬 파일을 셸에 넘기게 된다
                let entry = self.list.entry_at(index)?;
                let dir = self.tabs.active().source.local_path()?;
                let target = dir.join(entry.name_string());
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
                // 셸 메뉴는 로컬 전용이다 (D21) — 원격 목록의 우클릭은 자체 메뉴가 받는다
                let Some(folder) = self.tabs.active().source.local_path() else {
                    self.remote_menu_at = Some(pos);
                    return None;
                };
                let folder = folder.to_path_buf();
                Some(MenuRequest { folder, items, pos })
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
        remote: RemoteView<'_>,
        menu_state: PanelMenuState,
    ) -> PanelOutcome {
        let strip = crate::ui::tabs::show_tab_strip(ui, &self.tabs, remote, menu_state);
        let tab = self.tabs.active();
        let nav = self.address.show(ui, tab.committed(), &tab.history);

        // 트리는 주소창 아래 좌측 고정폭을 차지한다 — 현행 Win32 판의 배치와 같다
        let area = ui.available_rect_before_wrap();
        let mut tree_outcome = None;
        let content = if self.tree_visible {
            let split_x = area.left() + TREE_WIDTH.min(area.width());
            let tree_rect = egui::Rect::from_min_max(area.min, egui::pos2(split_x, area.bottom()));
            ui.painter().rect_filled(tree_rect, 0.0, theme::SURFACE_BG);
            ui.painter().vline(
                split_x,
                area.y_range(),
                egui::Stroke::new(TREE_BORDER, theme::TREE_LINE),
            );
            // 원격 탭이면 지금 보는 곳의 최상단을 뿌리로 삼는다 (#94 — 라벨도 함께 갈린다)
            let source = match self.remote_tree_root() {
                Some((conn, root)) => TreeSource::Remote {
                    conn,
                    root,
                    cache: remote.tree,
                },
                None => TreeSource::Local,
            };
            tree_outcome = ui
                .scope_builder(
                    // 이름을 붙이지 않으면 형제 영역과 같은 id를 갖게 되고, 그 안의 위젯 id까지
                    // 함께 겹친다(egui는 이름 없는 하위 영역에 전부 같은 이름을 준다)
                    egui::UiBuilder::new()
                        .id_salt("tree")
                        .max_rect(tree_rect.shrink(TREE_PAD)),
                    |ui| {
                        ui.set_clip_rect(tree_rect);
                        self.tree.show(ui, source)
                    },
                )
                .inner
                .into();
            egui::Rect::from_min_max(egui::pos2(split_x + TREE_BORDER, area.top()), area.max)
        } else {
            area
        };
        let (action, remote_action, drop, remote_menu) = ui
            .scope_builder(
                egui::UiBuilder::new().id_salt("content").max_rect(content),
                |ui| {
                    ui.set_clip_rect(content);
                    self.show_content(ui, icons, textures)
                },
            )
            .inner;

        // 탭·탐색은 여기서 바로 처리해도 된다(모달이 없다). 셸 메뉴만 호출부로 올려보낸다
        let mut tree_requests = Vec::new();
        if let Some(outcome) = tree_outcome {
            match outcome.chosen {
                // 트리에서 고른 폴더로 목록이 이동한다 (Acceptance ⑤)
                Some(TreeChoice::Local(path)) => self.navigate(path, ctx),
                Some(TreeChoice::Remote(path)) => self.navigate_remote(path),
                None => {}
            }
            for request in outcome.requests {
                match request {
                    // 로컬 열거는 트리가 직접 워커를 띄운다 — 연결이 필요 없다
                    TreeRequest::Local(path) => self.tree.start_local_load(path, ctx),
                    // 원격은 앱이 보낸다 — 연결을 아는 것은 앱이다
                    remote => tree_requests.push(remote),
                }
            }
        }
        let closed_conn = strip
            .tab
            .and_then(|tab_action| self.handle_tab(tab_action, ctx));
        if let Some(nav) = nav {
            self.handle_nav(nav, ctx);
        }
        PanelOutcome {
            menu: self.handle_list_action(action, ctx),
            // 드롭다운·드롭존에서 고른 사이트는 명령으로 올려 보낸다 —
            // 새 탭 생성·연결은 앱이 한다 (T13 착지 규약)
            command: strip.open_site.map(Command::OpenSiteTab).or(strip.command),
            remote: remote_action,
            remote_url: self.pending_remote_url.take(),
            closed_conn,
            drop,
            remote_menu,
            tree_requests,
        }
    }

    /// 원격 트리의 뿌리 — 활성 탭이 연결된 원격일 때만 있다.
    ///
    /// 지금 보는 경로를 부모로 계속 거슬러 올라간 최상단이다. `/`로 못 박지 않는 이유는
    /// 루트가 `/`가 아닌 서버가 있기 때문이다 (plan Edge Case)
    fn remote_tree_root(&self) -> Option<(ConnectionId, RemotePath)> {
        let conn = self.active_conn()?;
        let mut root = self.tabs.active().source.remote_path()?.clone();
        while let Some(parent) = root.parent() {
            root = parent;
        }
        Some((conn, root))
    }

    /// 트리에서 고른 원격 폴더로 목록을 옮긴다 (Acceptance ⑤).
    ///
    /// 상위 이동과 **같은 길**을 쓴다 — 옮기고 깃발을 세우면 앱이 다시 읽는다
    fn navigate_remote(&mut self, path: RemotePath) {
        self.set_remote_path(path);
    }

    /// 트리를 뺀 나머지 — 트리 토글·상태 줄과 본문.
    ///
    /// 원격 탭의 본문은 **연결 단계에 따라 통째로 달라진다**(README §4·§5) — 아직 연결하지
    /// 않았으면 안내 문구, 연결 중이면 자리 표시 막대, 실패했으면 사유와 조치, 연결됐으면
    /// 로컬과 같은 파일 목록이다
    fn show_content(
        &mut self,
        ui: &mut egui::Ui,
        icons: &mut IconCache,
        textures: &mut IconTextures,
    ) -> ContentOutcome {
        // 단계를 먼저 떼어 둔다 — 아래에서 목록(`&mut self.list`)을 그리는 동안 탭을 빌릴 수 없다
        let phase = match &self.tabs.active().source {
            TabSource::Remote { phase, .. } => Some(phase.clone()),
            TabSource::Local(_) => None,
        };
        let connected = !matches!(phase, Some(ref phase) if *phase != TabPhase::Ok);
        // 원격 패널에서는 트리 토글의 라벨이 갈린다 (인벤토리 #94).
        // 클로저 밖에서 정한다 — `Sides`의 클로저가 `self`를 통째로 빌린다
        let tree_label = if self.is_remote() {
            REMOTE_TREE_LABEL
        } else {
            LOCAL_TREE_LABEL
        };
        // 왼쪽에 트리 토글·진행 상황, 오른쪽 끝에 항목 수를 둔다 (사용자 요청 7).
        // `Sides`는 오른쪽 것을 먼저 자리잡게 하므로, 오류 문구가 길어져도 항목 수가 밀리지 않는다
        egui::Sides::new().show(
            ui,
            |ui| {
                if ui.selectable_label(self.tree_visible, tree_label).clicked() {
                    // `Sides` 클로저 안이라 `&mut self` 메서드를 부를 수 없어 필드를 직접 뒤집는다.
                    // 토글 진입점이 이 버튼 하나뿐이라 규칙이 흩어질 여지도 없다
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
                if !connected {
                    // 연결되지 않은 원격 패널은 항목 수를 모른다 — 0으로 보이면
                    // "빈 폴더"라는 없는 말을 하게 된다 (인벤토리 #95)
                    ui.colored_label(theme::TEXT_DIM, remote_states::UNKNOWN_COUNT);
                } else if !self.load.is_loading() {
                    // 읽는 중에는 세지 않는다 — 이전 폴더의 수가 남아 새 폴더의 것처럼 보인다
                    let (dirs, files) = self.list.counts();
                    ui.colored_label(theme::TEXT_DIM, format!("폴더 {dirs} 파일 {files}"));
                }
            },
        );
        ui.separator();
        match phase {
            Some(TabPhase::New) => {
                remote_states::show_empty(ui);
                (FileListAction::None, None, None, None)
            }
            Some(TabPhase::Connecting) => {
                // 취소는 자리 표시 막대 **위** 오른쪽에 둔다 (원본 `:223-228`의 상태 줄)
                let cancelled = ui
                    .with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        remote_states::show_cancel(ui)
                    })
                    .inner;
                remote_states::show_skeleton(ui);
                (
                    FileListAction::None,
                    cancelled.then_some(RemoteAction::CancelConnect),
                    None,
                    None,
                )
            }
            Some(TabPhase::Error { message }) => {
                let action = remote_states::show_failed(ui, &message).map(|chosen| match chosen {
                    FailedAction::Retry => RemoteAction::Retry,
                    FailedAction::OpenSettings => RemoteAction::OpenSettings,
                    FailedAction::ViewLog => RemoteAction::ViewLog,
                });
                (FileListAction::None, action, None, None)
            }
            // 연결된 원격 탭은 로컬과 **같은 목록 부품**으로 그린다 (T8)
            Some(TabPhase::Ok) | None => {
                let (action, drop) = self.show_list(ui, icons, textures);
                // 메뉴는 목록을 그린 **뒤에** 띄운다 — 먼저 그리면 목록이 그 위를 덮는다
                let menu = self.show_remote_menu(ui, phase == Some(TabPhase::Ok));
                (action, None, drop, menu)
            }
        }
    }

    /// 파일 목록 본문 — 로컬 탭과 연결된 원격 탭이 함께 쓴다.
    ///
    /// 목록 위에서 시작한 끌기와 이 목록에 놓인 드롭도 여기서 다룬다 (FR-38) —
    /// 두 쪽 다 "지금 이 탭이 어디를 보고 있는가"를 알아야 해서 목록 부품이 아니라 패널의 몫이다
    fn show_list(
        &mut self,
        ui: &mut egui::Ui,
        icons: &mut IconCache,
        textures: &mut IconTextures,
    ) -> (FileListAction, Option<DropOutcome>) {
        // 이번 프레임에 화면에 보인 파일들을 받는다. `request`는 아직 없으면 만들라고
        // 시키고, 이미 있으면 최근 사용으로 올린다 — 보이는 썸네일이 축출되지 않게 하는
        // 유일한 지점이다(그리기는 텍스처만 보고 픽셀 캐시를 건드리지 않는다)
        let mut visible = Vec::new();
        let list_rect = ui.available_rect_before_wrap();
        let interaction = self
            .list
            .show(ui, icons, textures, &self.thumb_textures, &mut visible);
        for path in visible {
            self.thumbs.request(&path);
        }

        // 끌기 시작 — 무엇을 싣는지는 지금 보고 있는 곳이 정한다
        if let Some(index) = interaction.drag_started {
            let source = &self.tabs.active().source;
            let remote_dir = source.remote_path().cloned();
            let source_site = match source {
                TabSource::Remote { site, .. } => Some(*site),
                TabSource::Local(_) => None,
            };
            let items = self.list.drag_items(index, remote_dir.as_ref());
            if !items.is_empty() {
                egui::DragAndDrop::set_payload(ui.ctx(), FileDrag { items, source_site });
            }
        }

        // 드롭 — 이 목록 위에서 손을 놓았는가
        let drop = self.take_drop(ui, list_rect);
        (interaction.action, drop)
    }

    /// 원격 목록의 우클릭 메뉴를 그린다 (FR-39).
    ///
    /// 고른 것과 **그때의 대상 경로들**을 함께 올린다 — 대화가 뜨는 동안 선택이 바뀔 수 있어,
    /// 나중에 다시 읽으면 사용자가 고른 것과 다른 항목에 명령이 갈 수 있다
    fn show_remote_menu(&mut self, ui: &mut egui::Ui, connected: bool) -> Option<RemoteMenuPick> {
        let at = self.remote_menu_at?;
        let Some(dir) = self.tabs.active().source.remote_path().cloned() else {
            self.remote_menu_at = None;
            return None;
        };
        let targets = self.list.selected_remote(&dir);
        let mut chosen = None;
        // 창 가장자리에서 눌러도 메뉴가 밖으로 넘어가지 않게 안으로 당긴다 — 셸 메뉴는
        // OS가 해 주는 일이라(D21) 우리가 그리는 이쪽에서는 직접 해야 한다 (quality 리뷰 m1)
        let viewport = ui.ctx().input(|input| input.viewport_rect());
        let at = clamp_menu_pos(viewport, at, remote_menu::menu_size());
        let response = egui::Area::new(ui.id().with("원격 메뉴"))
            .order(egui::Order::Foreground)
            .fixed_pos(at)
            .show(ui.ctx(), |ui| {
                egui::Frame::menu(ui.style())
                    .fill(theme::SURFACE_BG)
                    .stroke(egui::Stroke::new(1.0, theme::PANE_BORDER))
                    .corner_radius(0)
                    .show(ui, |ui| {
                        chosen = remote_menu::show_remote_menu(ui, targets.len(), connected);
                    });
            })
            .response;
        // 바깥을 누르거나 Esc면 닫는다 — 메뉴가 화면에 눌어붙지 않게 한다
        let outside = ui.input(|input| {
            input.pointer.any_click()
                && input
                    .pointer
                    .interact_pos()
                    .is_none_or(|pos| !response.rect.contains(pos))
        });
        let escape = ui.input(|input| input.key_pressed(egui::Key::Escape));
        if chosen.is_some() || outside || escape {
            self.remote_menu_at = None;
        }
        chosen.map(|action| (action, targets))
    }

    /// 이 목록 위에 놓인 드롭을 거둔다 (FR-38).
    ///
    /// 놓은 자리가 이 패널의 목록 밖이면 아무것도 가져가지 않는다 — 페이로드를 남겨 두어야
    /// 실제로 놓인 패널이 가져간다
    fn take_drop(&mut self, ui: &egui::Ui, list_rect: egui::Rect) -> Option<DropOutcome> {
        let released = ui.input(|input| input.pointer.any_released());
        if !released {
            return None;
        }
        let pointer = ui.input(|input| input.pointer.interact_pos())?;
        if !list_rect.contains(pointer) {
            return None;
        }
        let drag = egui::DragAndDrop::take_payload::<FileDrag>(ui.ctx())?;
        let target = match &self.tabs.active().source {
            TabSource::Local(dir) => DropTarget::Local(dir.clone()),
            TabSource::Remote { site, path, .. } => DropTarget::Remote {
                site: *site,
                dir: path.clone(),
            },
        };
        Some(DropOutcome {
            items: drag.items.clone(),
            source_site: drag.source_site,
            target,
        })
    }
}

/// 원격 목록의 첫 줄을 언제나 상위 이동(`..`)으로 맞춘다.
///
/// 서버·프로토콜에 따라 `..`를 주기도 하고 안 주기도 한다 — 그대로 두면 같은 조작이
/// 서버마다 다르게 보인다. 여기서 **한 번만, 맨 앞에** 오도록 정리한다
fn with_parent_first(entries: Vec<RemoteEntry>) -> Vec<RemoteEntry> {
    let mut out = Vec::with_capacity(entries.len() + 1);
    out.push(RemoteEntry {
        name: "..".to_owned(),
        is_dir: true,
        is_symlink: false,
        link_target: None,
        size: 0,
        modified: None,
        mode: None,
        owner: None,
    });
    out.extend(entries.into_iter().filter(|entry| entry.name != ".."));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::remote::sites::SiteStore;
    use crate::remote::types::{RemotePath, SiteId};

    /// 한 프레임에 그려진 글자를 전부 모은다 — 화면에 실제로 무엇이 보이는지 판정한다
    fn drawn_texts(output: &eframe::egui::FullOutput) -> Vec<String> {
        fn collect(shape: &egui::Shape, found: &mut Vec<String>) {
            match shape {
                egui::Shape::Text(text) => found.push(text.galley.text().to_owned()),
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

    /// egui는 같은 ID가 한 프레임에 두 번 쓰이면 화면에 경고 텍스트를 그린다.
    /// 그 텍스트를 그려진 글자에서 찾아 ID 충돌 여부를 판정한다
    fn id_clash_warnings(output: &eframe::egui::FullOutput) -> Vec<String> {
        drawn_texts(output)
            .into_iter()
            .filter(|body| body.contains("use of"))
            .collect()
    }

    /// 패널을 한 프레임 그린다 — 사이트 목록은 호출부가 준다
    fn draw_once(panel: &mut PanelState, sites: &SiteStore) -> eframe::egui::FullOutput {
        let tree = crate::remote::tree_cache::TreeCache::new();
        let remote = RemoteView {
            sites,
            connected: &[],
            tree: &tree,
        };
        let ctx = egui::Context::default();
        let mut icons = crate::fs::icons::IconCache::new();
        let mut textures = crate::ui::icon_tex::IconTextures::new();
        ctx.run_ui(Default::default(), |ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                let ctx = ui.ctx().clone();
                panel.show(
                    ui,
                    &ctx,
                    &mut icons,
                    &mut textures,
                    remote,
                    PanelMenuState::for_panes(1, ViewMode::Details),
                );
            });
        })
    }

    /// 패널을 한 프레임 그리고 ID 충돌 경고를 모은다
    fn draw_panel(tree_visible: bool) -> Vec<String> {
        let mut panel = PanelState::new(std::path::PathBuf::from(r"C:\"));
        panel.tree_visible = tree_visible;
        id_clash_warnings(&draw_once(&mut panel, &SiteStore::new()))
    }

    /// 사이트 하나를 등록하고 그 사이트의 원격 탭을 활성으로 둔 패널.
    /// 단계별 화면(README §4·§5)이 실제 렌더 경로를 지나게 하는 준비다
    fn remote_panel_in(phase: TabPhase) -> (PanelState, SiteStore) {
        let mut sites = SiteStore::new();
        let site = sites.add("배포 서버");
        let mut panel = PanelState::new(std::path::PathBuf::from(r"C:\테스트"));
        panel.tabs.add(crate::panel::tabs::TabState::remote(
            site,
            RemotePath::new("/var/www"),
        ));
        panel.attach_conn(ConnectionId(1));
        panel.set_phase_for(ConnectionId(1), &phase);
        (panel, sites)
    }

    /// 그 단계의 원격 패널을 한 프레임 그리고 화면 글자를 모은다
    fn remote_screen_texts(phase: TabPhase) -> Vec<String> {
        let (mut panel, sites) = remote_panel_in(phase);
        drawn_texts(&draw_once(&mut panel, &sites))
    }

    /// 열거 결과가 도착한 상황을 만들어 `poll_load`를 실제로 지나게 한다.
    /// **헬퍼만 직접 부르면 안 된다** — 호출부가 죽어 있어도 통과하기 때문이다(F-7 B1)
    fn commit_dir(panel: &mut PanelState, dir: &str, icons: &mut IconCache) {
        panel.pending_dir = std::path::PathBuf::from(dir);
        panel.pending_nav = PendingNav::None;
        panel.apply_enumerated(EnumOutcome::Ok(Vec::new()), icons);
    }

    #[test]
    fn 폴더를_옮기면_썸네일을_놓는다() {
        // 이 해제는 `ThumbnailCache`의 세대를 올리는 유일한 지점이기도 하다 —
        // 죽으면 떠난 폴더의 썸네일이 계속 남고(NFR-9), 늦게 도착한 결과도 못 거른다.
        // 커밋을 먼저 하고 비교하면 항상 같아져 이 경로가 통째로 죽는다(F-7 B1)
        let mut icons = IconCache::new();
        let mut panel = PanelState::new(std::path::PathBuf::from(r"C:\Users"));
        commit_dir(&mut panel, r"C:\Users", &mut icons);

        panel.thumbs.accept_for_test(
            std::path::PathBuf::from(r"C:\Users\사진.jpg"),
            Some(sample_thumb()),
        );
        assert_eq!(panel.thumbs.len(), 1, "사전 준비 실패");

        commit_dir(&mut panel, r"C:\Windows", &mut icons);
        assert_eq!(
            panel.thumbs.len(),
            0,
            "폴더를 옮겼는데 이전 폴더의 썸네일이 남았다"
        );
    }

    #[test]
    fn 탭을_바꿔_폴더가_달라져도_썸네일을_놓는다() {
        // 탭 전환은 `tabs.switch`로 **활성 탭을 먼저 바꾼 뒤** 그 경로를 읽는다 —
        // 커밋 직전 경로와 비교하는 방식이면 이 경로만 빠져나간다(F-7 m1).
        // 그래서 판정을 캐시(`set_folder`)로 옮겼고, 이 테스트가 그것을 지킨다
        let mut icons = IconCache::new();
        let mut panel = PanelState::new(std::path::PathBuf::from(r"C:\Users"));
        commit_dir(&mut panel, r"C:\Users", &mut icons);
        panel.thumbs.accept_for_test(
            std::path::PathBuf::from(r"C:\Users\사진.jpg"),
            Some(sample_thumb()),
        );

        // 다른 폴더를 보는 탭을 더한다 — `add`가 그 탭을 곧바로 활성으로 만든다
        panel
            .tabs
            .add(crate::panel::tabs::TabState::new(std::path::PathBuf::from(
                r"C:\Windows",
            )));
        commit_dir(&mut panel, r"C:\Windows", &mut icons);
        assert_eq!(
            panel.thumbs.len(),
            0,
            "탭을 바꿔 폴더가 달라졌는데 이전 폴더의 썸네일이 남았다"
        );

        // 되돌아가는 전환도 같아야 한다 — 새 폴더 썸네일을 담아 두고 원래 탭으로 돌아간다
        panel.thumbs.accept_for_test(
            std::path::PathBuf::from(r"C:\Windows\그림.png"),
            Some(sample_thumb()),
        );
        assert!(panel.tabs.switch(0), "첫 탭으로 되돌아가지 못했다");
        commit_dir(&mut panel, r"C:\Users", &mut icons);
        assert_eq!(panel.thumbs.len(), 0, "되돌아가는 전환에서 남았다");
    }

    #[test]
    fn 같은_폴더를_다시_읽으면_썸네일을_지킨다() {
        // 감시 갱신(FR-10)은 같은 폴더를 다시 읽는다 — 그때마다 버리면
        // 다른 앱이 파일 하나만 만들어도 폴더 전체를 다시 만들게 된다
        let mut icons = IconCache::new();
        let mut panel = PanelState::new(std::path::PathBuf::from(r"C:\Users"));
        commit_dir(&mut panel, r"C:\Users", &mut icons);
        panel.thumbs.accept_for_test(
            std::path::PathBuf::from(r"C:\Users\사진.jpg"),
            Some(sample_thumb()),
        );

        commit_dir(&mut panel, r"C:\Users", &mut icons); // 감시 갱신
        assert_eq!(panel.thumbs.len(), 1, "같은 폴더인데 썸네일을 버렸다");
    }

    fn sample_thumb() -> crate::fs::thumbnail::ThumbnailImage {
        crate::fs::thumbnail::ThumbnailImage {
            width: 2,
            height: 2,
            rgba: vec![255; 16],
        }
    }

    #[test]
    fn 썸네일을_올린_프레임은_곧바로_다시_그리라고_알린다() {
        // egui는 입력이 없으면 프레임을 돌리지 않는다 — 이 신호가 빠지면 워커가 늦게 준
        // 썸네일이 사용자가 마우스를 움직일 때까지 형식 아이콘에 머문다 (F-8에서 실제로 그랬다)
        let ctx = egui::Context::default();
        let mut panel = PanelState::new(std::path::PathBuf::from(r"C:\Users"));
        panel.thumbs.accept_for_test(
            std::path::PathBuf::from(r"C:\Users\사진.jpg"),
            Some(sample_thumb()),
        );

        assert_eq!(
            panel.poll_thumbnails(&ctx),
            Some(Duration::ZERO),
            "썸네일을 올린 프레임인데 곧바로 다시 그리라고 알리지 않았다"
        );
        assert_eq!(panel.thumb_textures.len(), 1, "텍스처가 올라가지 않았다");
        // 올릴 것도 기다릴 것도 없으면 알리지 않는다 — 늘 알리면 앱이 쉬지 않고 그린다
        assert_eq!(
            panel.poll_thumbnails(&ctx),
            None,
            "할 일이 없는데도 다시 그리라고 알렸다"
        );
    }

    #[test]
    fn 썸네일을_기다리는_동안은_스스로_깨어난다() {
        // 썸네일 워커는 `fs` 계층이라 egui를 모른다 — 결과가 채널에 들어와도 앱은 알 수 없다.
        // 이 신호가 없으면 사진이 사용자가 마우스를 움직일 때까지 안 나타난다(F-8 실측)
        let ctx = egui::Context::default();
        let mut panel = PanelState::new(std::path::PathBuf::from(r"C:\Users"));
        panel
            .thumbs
            .request(std::path::Path::new(r"C:\Users\아직없음.jpg"));

        assert_eq!(
            panel.poll_thumbnails(&ctx),
            Some(THUMB_POLL_INTERVAL),
            "결과를 기다리는데 다시 깨어날 시점을 알리지 않았다"
        );
    }

    #[test]
    fn 보기_모드는_패널을_거쳐_목록까지_전달된다() {
        // `Command::SetViewMode`가 닿는 지점이다 — 여기서 끊기면 메뉴에서 골라도
        // 목록은 이전 모드로 그려진다 (FR-23)
        let mut panel = PanelState::new(std::path::PathBuf::from(r"C:\"));
        assert_eq!(panel.view_mode(), ViewMode::Details);
        panel.set_view_mode(ViewMode::SmallIcons);
        assert_eq!(panel.view_mode(), ViewMode::SmallIcons);
    }

    #[test]
    fn 보기_모드는_패널마다_독립이다() {
        // 한 패널에서 바꾼 모드가 다른 패널에 번지면 "패널마다 독립"(FR-23)이 깨진다
        let mut first = PanelState::new(std::path::PathBuf::from(r"C:\"));
        let second = PanelState::new(std::path::PathBuf::from(r"D:\"));
        first.set_view_mode(ViewMode::Tiles);
        assert_eq!(first.view_mode(), ViewMode::Tiles);
        assert_eq!(
            second.view_mode(),
            ViewMode::Details,
            "다른 패널까지 바뀌었다"
        );
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

    #[test]
    fn 원격_목록의_첫_줄은_언제나_상위_이동이다() {
        // 서버가 `..`를 주기도 하고 안 주기도 한다 — 화면은 어느 쪽이든 같아야 한다 (plan T9 ③)
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

        let 없는_경우 = with_parent_first(vec![entry("public_html", true), entry("a.txt", false)]);
        let names: Vec<&str> = 없는_경우.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, vec!["..", "public_html", "a.txt"]);

        let 있는_경우 = with_parent_first(vec![
            entry("..", true),
            entry("public_html", true),
            entry("..", true),
        ]);
        let names: Vec<&str> = 있는_경우.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, vec!["..", "public_html"], "`..`가 둘이 되면 안 된다");

        // 빈 폴더에도 상위 이동은 남는다
        assert_eq!(with_parent_first(Vec::new()).len(), 1);
    }

    #[test]
    fn 원격_탭에서는_로컬_전용_작업이_일어나지_않는다() {
        // 열거·감시·썸네일·새 파일은 로컬에만 있는 일이다 (plan T9 ②)
        let ctx = egui::Context::default();
        let mut panel = PanelState::new(std::path::PathBuf::from(r"C:\테스트"));
        panel.tabs.add(crate::panel::tabs::TabState::remote(
            SiteId(1),
            RemotePath::new("/pub"),
        ));
        assert!(panel.is_remote(), "원격 탭이 활성이어야 한다");

        // 새 폴더·새 파일은 아무 일도 하지 않는다
        panel.new_folder(&ctx);
        panel.new_file(&ctx);
        assert!(
            panel.create.pending.is_none(),
            "원격 탭에서 로컬 생성이 시작됐다"
        );
        // 연결이 없는 원격 탭에서는 목록 요청도 나가지 않는다
        let manager = ConnectionManager::new(std::sync::Arc::new(|| {}));
        assert_eq!(panel.request_remote_list(&manager), None);
    }

    #[test]
    fn 한_패널에_로컬_탭과_원격_탭을_섞을_수_있다() {
        // 탭마다 자기 소스로 그려져야 한다 (plan T9 ⑤)
        let mut panel = PanelState::new(std::path::PathBuf::from(r"C:\테스트"));
        panel.tabs.add(crate::panel::tabs::TabState::remote(
            SiteId(3),
            RemotePath::new("/var/www"),
        ));

        let sources = panel.tabs.sources();
        assert_eq!(sources.len(), 2);
        assert!(!sources[0].is_remote(), "첫 탭은 로컬이어야 한다");
        assert!(sources[1].is_remote(), "둘째 탭은 원격이어야 한다");
        assert_eq!(sources[1].site(), Some(SiteId(3)));
        assert_eq!(
            sources[1].remote_path().map(|p| p.as_str()),
            Some("/var/www")
        );

        // 로컬 탭으로 돌아오면 다시 로컬 전용 일이 열린다
        assert!(panel.tabs.switch(0));
        assert!(!panel.is_remote());
        assert_eq!(panel.dir(), std::path::Path::new(r"C:\테스트"));
    }

    /// 원격 탭 하나를 더해 활성으로 만든 패널
    fn panel_with_remote_tab(path: &str) -> PanelState {
        let mut panel = PanelState::new(std::path::PathBuf::from(r"C:\테스트"));
        panel.tabs.add(crate::panel::tabs::TabState::remote(
            SiteId(1),
            RemotePath::new(path),
        ));
        assert!(panel.is_remote(), "원격 탭이 활성이어야 한다");
        panel
    }

    #[test]
    fn 원격_탭에서_새로_고침은_로컬_열거를_걸지_않는다() {
        let ctx = egui::Context::default();
        let mut panel = panel_with_remote_tab("/var/www");
        panel.refresh(&ctx);
        assert!(
            panel.load.pending.is_none(),
            "원격 탭에서 로컬 열거 워커가 떴다"
        );
    }

    #[test]
    fn 원격_탭의_상위_이동은_원격_경로로_가고_루트에서_머문다() {
        // plan T9 Edge Case — 루트를 넘어가지 않는다
        let ctx = egui::Context::default();
        let mut panel = panel_with_remote_tab("/var/www");

        panel.handle_nav(NavAction::Up, &ctx);
        assert_eq!(
            panel.tabs.active().source.remote_path().map(|p| p.as_str()),
            Some("/var")
        );
        panel.handle_nav(NavAction::Up, &ctx);
        assert_eq!(
            panel.tabs.active().source.remote_path().map(|p| p.as_str()),
            Some("/")
        );
        // 루트에서 한 번 더 눌러도 그대로다
        panel.handle_nav(NavAction::Up, &ctx);
        assert_eq!(
            panel.tabs.active().source.remote_path().map(|p| p.as_str()),
            Some("/")
        );
        // 로컬 열거 워커도 뜨지 않았다
        assert!(panel.load.pending.is_none());
    }

    #[test]
    fn 원격_탭에서는_셸_메뉴를_요청하지_않는다() {
        // 셸은 로컬 PIDL만 다룬다 (D21)
        let ctx = egui::Context::default();
        let mut panel = panel_with_remote_tab("/var/www");
        let request = panel.handle_list_action(
            FileListAction::Context {
                index: None,
                pos: egui::pos2(0.0, 0.0),
            },
            &ctx,
        );
        assert!(request.is_none(), "원격 탭에서 셸 메뉴가 요청됐다");

        // 항목 열기도 로컬 경로를 만들지 않는다
        let opened = panel.handle_list_action(FileListAction::Open(0), &ctx);
        assert!(opened.is_none());
        assert!(panel.load.pending.is_none());
    }

    #[test]
    fn 원격_탭을_보는_동안_로컬_감시_통지는_무시된다() {
        // 이전 폴더의 감시가 아직 살아 있어도 원격 화면이 로컬 목록으로 덮이면 안 된다
        let ctx = egui::Context::default();
        let mut panel = panel_with_remote_tab("/var/www");
        let (tx, rx) = std::sync::mpsc::channel();
        panel.watch = Some(DirWatch {
            watcher: crate::fs::watcher::DirWatcher::start(
                std::path::PathBuf::from(r"C:\테스트"),
                tx,
                None,
            ),
            rx,
        });

        panel.poll_watch(&ctx);
        assert!(
            panel.load.pending.is_none(),
            "감시 통지로 로컬 열거 워커가 떴다"
        );
    }

    #[test]
    fn 원격_탭에는_사이트_이름과_단계_배지가_함께_보인다() {
        // 인벤토리 #11~13 — 이름은 사이트 설정에서, 배지 문구는 단계에서 온다 (Acceptance ①).
        // 탭이 이름 사본을 들면 `이름 바꾸기(R)` 뒤에 탭만 옛 이름으로 남는다
        let 빈_탭 = remote_screen_texts(TabPhase::New);
        assert!(
            빈_탭.iter().any(|t| t == "배포 서버"),
            "사이트 이름이 탭에 없다: {빈_탭:?}"
        );
        assert!(
            빈_탭.iter().any(|t| t == "연결 없음"),
            "미연결 배지가 없다: {빈_탭:?}"
        );
        assert!(
            remote_screen_texts(TabPhase::Connecting)
                .iter()
                .any(|t| t == "연결 중…"),
            "연결 중 배지가 없다"
        );
        // 연결되면 배지가 프로토콜 이름으로 바뀐다 (새 사이트의 기본값은 FTP다)
        assert!(
            remote_screen_texts(TabPhase::Ok).iter().any(|t| t == "ftp"),
            "연결됨 배지가 프로토콜을 보이지 않는다"
        );
    }

    #[test]
    fn 단계마다_본문이_통째로_달라진다() {
        // README §4·§5 — 연결 전·중·실패에 목록 대신 그 단계의 화면이 보인다 (Acceptance ①③④)
        let 빈_탭 = remote_screen_texts(TabPhase::New);
        assert!(
            빈_탭.iter().any(|t| t.contains("sftp://호스트")),
            "미연결 안내가 없다: {빈_탭:?}"
        );
        assert!(
            빈_탭.iter().any(|t| t.contains("끌어다 놓아도 됩니다")),
            "드래그 안내가 없다: {빈_탭:?}"
        );

        let 연결_중 = remote_screen_texts(TabPhase::Connecting);
        assert!(
            연결_중.iter().any(|t| t == "취소"),
            "연결 중 취소 버튼이 없다: {연결_중:?}"
        );

        let 실패 = remote_screen_texts(TabPhase::Error {
            message: "530 Login incorrect".to_owned(),
        });
        for 문구 in [
            "연결하지 못했습니다",
            "재시도",
            "설정 열기",
            "서버 로그 보기",
        ] {
            assert!(
                실패.iter().any(|t| t.contains(문구)),
                "실패 화면에 '{문구}'가 없다: {실패:?}"
            );
        }
        assert!(
            실패.iter().any(|t| t.contains("530 Login incorrect")),
            "서버가 준 사유가 보이지 않는다: {실패:?}"
        );
    }

    #[test]
    fn 연결되지_않은_원격_패널은_항목_수를_모른다고_보인다() {
        // 인벤토리 #95 — `폴더 0 파일 0`으로 보이면 "빈 폴더"라는 없는 말을 하게 된다
        for phase in [
            TabPhase::New,
            TabPhase::Connecting,
            TabPhase::Error {
                message: "530".to_owned(),
            },
        ] {
            let texts = remote_screen_texts(phase.clone());
            assert!(
                texts
                    .iter()
                    .any(|t| t == crate::ui::remote_states::UNKNOWN_COUNT),
                "{phase:?}에서 `—`가 보이지 않는다: {texts:?}"
            );
            assert!(
                !texts.iter().any(|t| is_item_count(t)),
                "{phase:?}인데 항목 수를 세어 보였다: {texts:?}"
            );
        }
        // 연결되면 보통의 항목 수로 돌아온다
        let 연결됨 = remote_screen_texts(TabPhase::Ok);
        assert!(
            연결됨.iter().any(|t| is_item_count(t)),
            "연결됐는데 항목 수가 없다: {연결됨:?}"
        );
    }

    /// 상태 줄의 항목 수 표시인가 — 트리 토글(`폴더 트리`)과 구분한다
    fn is_item_count(text: &str) -> bool {
        text.starts_with("폴더 ") && text.contains("파일 ")
    }

    #[test]
    fn 원격_탭_화면에서도_위젯_id가_겹치지_않는다() {
        // Acceptance ⑧ — 단계별 화면이 목록 자리에 들어와도 id 공간이 섞이면 안 된다
        for phase in [
            TabPhase::New,
            TabPhase::Connecting,
            TabPhase::Error {
                message: "530".to_owned(),
            },
            TabPhase::Ok,
        ] {
            let (mut panel, sites) = remote_panel_in(phase.clone());
            let clashes = id_clash_warnings(&draw_once(&mut panel, &sites));
            assert!(
                clashes.is_empty(),
                "{phase:?}에서 위젯 ID 충돌: {clashes:?}"
            );
        }
    }

    #[test]
    fn 패널의_마지막_원격_탭을_닫으면_연결을_접는다() {
        // FR-32 — 같은 연결을 쓰는 탭이 남아 있으면 접지 않는다 (Acceptance ⑥)
        let ctx = egui::Context::default();
        let mut panel = PanelState::new(std::path::PathBuf::from(r"C:\테스트"));
        panel.tabs.add(crate::panel::tabs::TabState::remote(
            SiteId(1),
            RemotePath::new("/a"),
        ));
        panel.attach_conn(ConnectionId(5));
        panel.tabs.add(crate::panel::tabs::TabState::remote(
            SiteId(1),
            RemotePath::new("/b"),
        ));
        panel.attach_conn(ConnectionId(5));

        assert_eq!(
            panel.handle_tab(TabAction::Close(2), &ctx),
            None,
            "같은 연결을 쓰는 탭이 남았는데 연결을 접으려 했다"
        );
        assert_eq!(
            panel.handle_tab(TabAction::Close(1), &ctx),
            Some(ConnectionId(5)),
            "마지막 원격 탭을 닫았는데 연결이 남았다"
        );
        // 로컬 탭만 남았으니 더 접을 것이 없다
        assert!(!panel.is_remote());
    }

    #[test]
    fn 연결_단계는_그_연결을_쓰는_모든_탭에_퍼진다() {
        // 배경 탭이 옛 단계로 남으면 그 탭으로 돌아갔을 때 화면이 실제와 어긋난다
        let mut panel = PanelState::new(std::path::PathBuf::from(r"C:\테스트"));
        panel.tabs.add(crate::panel::tabs::TabState::remote(
            SiteId(1),
            RemotePath::new("/a"),
        ));
        panel.attach_conn(ConnectionId(5));
        panel.tabs.add(crate::panel::tabs::TabState::remote(
            SiteId(1),
            RemotePath::new("/b"),
        ));
        panel.attach_conn(ConnectionId(7)); // 다른 연결을 쓰는 탭

        assert!(panel.set_phase_for(ConnectionId(5), &TabPhase::Ok));
        let phases: Vec<TabPhase> = panel
            .tabs
            .sources()
            .iter()
            .filter_map(|source| match source {
                TabSource::Remote { phase, .. } => Some(phase.clone()),
                TabSource::Local(_) => None,
            })
            .collect();
        assert_eq!(
            phases,
            vec![TabPhase::Ok, TabPhase::Connecting],
            "다른 연결의 탭까지 바뀌었거나, 대상 탭이 바뀌지 않았다"
        );
        // 없는 연결에는 아무 일도 일어나지 않는다
        assert!(!panel.set_phase_for(ConnectionId(99), &TabPhase::Ok));
    }

    #[test]
    fn 남의_답이나_지난_위치의_목록은_받지_않는다() {
        // 세대만 보면 한 연결을 두 패널이 나눠 쓸 때 남의 답을 제 목록으로 삼는다
        let mut icons = IconCache::new();
        let mut panel = panel_with_remote_tab("/var/www");
        panel.attach_conn(ConnectionId(1));
        let manager = ConnectionManager::new(std::sync::Arc::new(|| {}));
        // 연결이 죽어 있어도 세대는 올라간다 — 여기서는 세대·위치 판정만 본다
        panel.request_remote_list(&manager);
        let generation = panel.remote_generation;

        assert!(!panel.awaits_remote_list(generation + 1, &RemotePath::new("/var/www")));
        assert!(!panel.awaits_remote_list(generation, &RemotePath::new("/etc")));
        assert!(panel.awaits_remote_list(generation, &RemotePath::new("/var/www")));
        assert!(!panel.apply_remote_listed(
            generation,
            &RemotePath::new("/etc"),
            Vec::new(),
            &mut icons
        ));
        assert!(panel.apply_remote_listed(
            generation,
            &RemotePath::new("/var/www"),
            Vec::new(),
            &mut icons
        ));
    }

    #[test]
    fn 만드는_중_원격_탭으로_옮겨_가면_로컬_열거를_걸지_않는다() {
        // 새 폴더를 만드는 워커가 도는 사이 원격 탭으로 옮겨 가면, 완료 시점의 활성 탭은
        // 원격이다 — 그때 활성 탭 기준으로 다시 읽으면 빈 경로를 열거하게 된다
        use std::time::{Duration, Instant};

        let ctx = egui::Context::default();
        let dir = std::env::temp_dir().join(format!("fe_t9_생성_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);

        let mut panel = PanelState::new(dir.clone());
        panel.new_folder(&ctx);
        assert!(panel.create.pending.is_some(), "생성이 시작되지 않았다");

        // 만드는 사이 원격 탭으로 옮겨 간다
        panel.tabs.add(crate::panel::tabs::TabState::remote(
            SiteId(1),
            RemotePath::new("/var/www"),
        ));
        assert!(panel.is_remote());

        let deadline = Instant::now() + Duration::from_secs(3);
        while panel.create.pending.is_some() && Instant::now() < deadline {
            panel.poll_create(&ctx);
            std::thread::sleep(Duration::from_millis(5));
        }
        assert!(panel.create.pending.is_none(), "생성이 끝나지 않았다");
        // **`load.pending`으로 본다** — `pending_dir`은 원격 탭에서 어차피 빈 경로라
        // 가드 유무를 가리지 못한다. 열거 워커가 떴는지가 유일하게 둘을 가르는 신호다
        assert!(
            panel.load.pending.is_none(),
            "원격 탭인데 로컬 열거 워커가 떴다"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn 원격_목록의_우클릭은_셸_메뉴를_띄우지_않는다() {
        // Acceptance ⑤ — 셸 메뉴는 로컬 경로가 있어야 뜬다(D21). 원격 탭에서는 자체 메뉴다
        let mut panel = PanelState::new(PathBuf::from(r"C:\"));
        let pos = egui::pos2(120.0, 80.0);

        let ctx = egui::Context::default();
        let request = panel.handle_list_action(
            FileListAction::Context {
                index: Some(0),
                pos,
            },
            &ctx,
        );
        assert!(request.is_some(), "로컬 탭에서는 셸 메뉴를 청해야 한다");
        assert!(panel.remote_menu_at.is_none(), "로컬 탭에 원격 메뉴가 떴다");

        panel.open_remote_tab(SiteId(1), RemotePath::new("/var/www"));
        let request = panel.handle_list_action(
            FileListAction::Context {
                index: Some(0),
                pos,
            },
            &ctx,
        );
        assert!(request.is_none(), "원격 탭에서 셸 메뉴를 청했다");
        assert_eq!(panel.remote_menu_at, Some(pos), "원격 메뉴가 뜨지 않았다");
    }

    #[test]
    fn 가장자리에서_연_메뉴는_화면_안으로_당겨진다() {
        // quality 리뷰 m1 — 셸 메뉴는 OS가 보정해 주지만(D21) 우리가 그리는 메뉴는 아니다
        let screen = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(1200.0, 800.0));
        let size = egui::vec2(200.0, 240.0);
        // 안쪽에서 열면 그 자리 그대로다
        assert_eq!(
            clamp_menu_pos(screen, egui::pos2(100.0, 100.0), size),
            egui::pos2(100.0, 100.0)
        );
        // 오른쪽·아래 가장자리에서 열면 안으로 당긴다
        assert_eq!(
            clamp_menu_pos(screen, egui::pos2(1150.0, 780.0), size),
            egui::pos2(1000.0, 560.0)
        );
        // 화면보다 큰 메뉴는 왼쪽 위를 맞춘다 — 아래가 잘려도 첫 줄은 보인다
        let huge = egui::vec2(2000.0, 2000.0);
        assert_eq!(
            clamp_menu_pos(screen, egui::pos2(600.0, 400.0), huge),
            egui::pos2(0.0, 0.0)
        );
    }

    #[test]
    fn 원격_패널의_트리_토글은_원격_트리다() {
        // Acceptance ① (인벤토리 #94) — 같은 자리의 라벨이 소스에 따라 갈린다
        let 로컬 = drawn_texts(&draw_once(
            &mut PanelState::new(std::path::PathBuf::from(r"C:\")),
            &SiteStore::new(),
        ));
        assert!(
            로컬.iter().any(|text| text == LOCAL_TREE_LABEL),
            "로컬 패널의 토글이 `폴더 트리`가 아니다: {로컬:?}"
        );
        let 원격 = remote_screen_texts(TabPhase::Ok);
        assert!(
            원격.iter().any(|text| text == REMOTE_TREE_LABEL),
            "원격 패널의 토글이 `원격 트리`가 아니다: {원격:?}"
        );
        assert!(
            !원격.iter().any(|text| text == LOCAL_TREE_LABEL),
            "원격 패널에 `폴더 트리`가 남아 있다"
        );
    }

    #[test]
    fn 트리에서_고른_원격_폴더로_목록이_옮겨간다() {
        // Acceptance ⑤ — 옮기는 것으로 끝나면 화면은 옛 목록 그대로다(spec 리뷰 B1).
        // 옮긴 뒤 **깃발이 서고**, 그 깃발을 거둔 쪽이 실제로 조회를 보내야 한다
        let (mut panel, _) = remote_panel_in(TabPhase::Ok);
        panel.take_remote_dirty();
        panel.navigate_remote(RemotePath::new("/var/www/html"));
        assert_eq!(
            panel.tabs.active().source.remote_path().map(|p| p.as_str()),
            Some("/var/www/html")
        );
        assert!(panel.take_remote_dirty(), "다시 읽어 달라는 표시가 없다");
        assert!(!panel.take_remote_dirty(), "깃발이 한 번에 거둬지지 않았다");

        // 거둔 쪽이 그 위치로 조회를 보낸다 — 세대와 위치가 함께 맞아야 답을 받는다
        let manager = ConnectionManager::new(std::sync::Arc::new(|| {}));
        panel.request_remote_list(&manager);
        assert!(
            panel.awaits_remote_list(panel.remote_generation, &RemotePath::new("/var/www/html"))
        );

        // 상위 이동도 같은 길을 쓴다 — 옮기고 나서 아무도 다시 읽지 않던 자리였다
        let ctx = egui::Context::default();
        panel.handle_nav(NavAction::Up, &ctx);
        assert_eq!(
            panel.tabs.active().source.remote_path().map(|p| p.as_str()),
            Some("/var/www")
        );
        assert!(panel.take_remote_dirty(), "상위 이동 뒤에 표시가 없다");
    }

    #[test]
    fn 원격_트리의_뿌리는_최상단까지_거슬러_올라간다() {
        // plan Edge Case — 루트가 `/`가 아닌 서버도 있어 `/`로 못 박지 않는다
        let (panel, _) = remote_panel_in(TabPhase::Ok);
        let (conn, root) = panel.remote_tree_root().expect("연결된 원격 탭");
        assert_eq!(conn, ConnectionId(1));
        assert_eq!(root.as_str(), "/");
        // 로컬 탭에는 원격 트리가 없다
        let 로컬 = PanelState::new(std::path::PathBuf::from(r"C:\"));
        assert!(로컬.remote_tree_root().is_none());
    }

    #[test]
    fn 성공한_이동은_되돌릴_자리를_남기지_않는다() {
        // F-7 2라운드 B1 — 자리가 남으면 **나중의 무관한 실패**(새로 고침·작업 후 재조회)가
        // 옛 폴더로 경로만 되돌린다
        let mut icons = IconCache::new();
        let manager = ConnectionManager::new(std::sync::Arc::new(|| {}));
        let (mut panel, _) = remote_panel_in(TabPhase::Ok);
        panel.set_remote_path(RemotePath::new("/var/www/html"));
        panel.request_remote_list(&manager);
        let moved = panel.remote_generation;
        // 답이 도착해 이동이 섰다
        assert!(panel.apply_remote_listed(
            moved,
            &RemotePath::new("/var/www/html"),
            Vec::new(),
            &mut icons
        ));

        // 그 뒤의 새로 고침이 실패해도 경로는 그대로여야 한다
        panel.request_remote_list(&manager);
        let refreshed = panel.remote_generation;
        assert!(
            !panel.revert_remote_path(refreshed),
            "성공한 이동이 되돌려졌다"
        );
        assert_eq!(
            panel.tabs.active().source.remote_path().map(|p| p.as_str()),
            Some("/var/www/html")
        );
    }

    #[test]
    fn 다른_요청의_실패는_경로를_건드리지_않는다() {
        // F-7 2라운드 B2 — 같은 연결을 두 패널이 나눠 쓰면 세대가 겹친다.
        // 되돌리기는 **그 요청의 세대**이면서 **아직 그 자리에 있을 때**만 일어나야 한다
        let manager = ConnectionManager::new(std::sync::Arc::new(|| {}));
        let (mut panel, _) = remote_panel_in(TabPhase::Ok);
        panel.set_remote_path(RemotePath::new("/root"));
        panel.request_remote_list(&manager);
        let generation = panel.remote_generation;

        // 남의 세대로는 되돌지 않는다
        assert!(!panel.revert_remote_path(generation + 1));
        assert_eq!(
            panel.tabs.active().source.remote_path().map(|p| p.as_str()),
            Some("/root")
        );

        // 그 사이 다른 곳으로 또 옮겼으면 지난 되돌리기는 무효다
        panel.set_remote_path(RemotePath::new("/etc"));
        assert!(
            !panel.revert_remote_path(generation),
            "지난 요청이 지금 위치를 되돌렸다"
        );
        assert_eq!(
            panel.tabs.active().source.remote_path().map(|p| p.as_str()),
            Some("/etc")
        );
    }

    #[test]
    fn 조회가_실패하면_옮기기를_무른다() {
        // F-7 리뷰 B2 — 주소창은 새 폴더를, 목록은 이전 폴더를 가리킨 채 갈라지면
        // 그 위에서 연 메뉴가 보이는 것과 다른 경로에 삭제·권한 변경을 건다
        let (mut panel, _) = remote_panel_in(TabPhase::Ok);
        assert_eq!(
            panel.tabs.active().source.remote_path().map(|p| p.as_str()),
            Some("/var/www")
        );
        let manager = ConnectionManager::new(std::sync::Arc::new(|| {}));
        panel.set_remote_path(RemotePath::new("/root"));
        panel.request_remote_list(&manager);
        let generation = panel.remote_generation;

        assert!(
            panel.revert_remote_path(generation),
            "되돌릴 자리가 없다고 했다"
        );
        assert_eq!(
            panel.tabs.active().source.remote_path().map(|p| p.as_str()),
            Some("/var/www"),
            "이전 폴더로 돌아오지 않았다"
        );
        // 되돌린 뒤에는 다시 청하지 않는다 — 실패·성공이 번갈아 도는 고리를 만들지 않는다
        panel.take_remote_dirty();
        assert!(!panel.take_remote_dirty());
        // 돌아갈 자리는 한 번만 쓴다
        assert!(!panel.revert_remote_path(generation));
    }

    #[test]
    fn 원격_탭을_바꾸면_그_탭의_목록을_다시_읽는다() {
        // F-7 3라운드 B1 — 목록은 탭이 아니라 패널 하나가 든다. 탭만 바꾸고 목록을 그대로 두면
        // 주소창은 이 탭을, 목록은 저 탭의 폴더를 보인다 — 그 위에서 연 원격 메뉴가
        // **화면에 없는 경로**에 삭제·권한 변경을 건다
        let ctx = egui::Context::default();
        let (mut panel, _) = remote_panel_in(TabPhase::Ok);
        // 원격 탭 하나를 더 연다 (`Ctrl+T`는 지금 보는 원격 위치를 복제한다)
        panel.handle_tab(TabAction::New, &ctx);
        panel.set_remote_path(RemotePath::new("/var/log"));
        panel.take_remote_dirty();

        // 첫 원격 탭으로 돌아간다 — 그 탭이 보는 곳을 다시 읽어야 한다
        let first = panel
            .tabs
            .sources()
            .iter()
            .position(|source| matches!(source, TabSource::Remote { .. }))
            .expect("원격 탭");
        panel.handle_tab(TabAction::Switch(first), &ctx);
        assert!(
            panel.take_remote_dirty(),
            "원격 탭으로 바꿨는데 목록을 다시 읽지 않는다"
        );
        // 답이 오기 전까지 목록은 비어 있어야 한다 — 옛 탭의 항목이 남으면 그 사이에
        // 연 메뉴가 화면에 없는 경로를 겨눈다 (F-7 4라운드 M1)
        assert_eq!(
            panel
                .list
                .selected_remote(&RemotePath::new("/var/www"))
                .len(),
            0,
            "전환 직후 옛 항목이 남아 있다"
        );

        // 로컬 탭으로 바꾸면 로컬 열거가 도므로 이 깃발은 서지 않는다
        let local = panel
            .tabs
            .sources()
            .iter()
            .position(|source| matches!(source, TabSource::Local(_)))
            .expect("로컬 탭");
        panel.handle_tab(TabAction::Switch(local), &ctx);
        assert!(!panel.take_remote_dirty(), "로컬 탭에 원격 조회를 청했다");
    }
}
