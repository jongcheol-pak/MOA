//! egui 앱 진입 상태 — 창 골격·폰트·COM·셸 호스트·파일 목록 보유.
//!
//! 분할·탭·주소창은 이후 task에서 이 구조체에 붙는다.
use crate::fs::enumerate::{EnumOutcome, enumerate_dir};
use crate::fs::icons::IconCache;
use crate::panel::history::History;
use crate::ui::address_bar::{AddressBar, NavAction};
use crate::ui::file_list::{FileListAction, FileListView};
use crate::ui::icon_tex::IconTextures;
use crate::ui::shell_host::{self, ShellHost};
use crate::ui::theme;
use eframe::egui;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, TryRecvError, channel};
use windows::Win32::Foundation::RPC_E_CHANGED_MODE;
use windows::Win32::System::Com::{COINIT_APARTMENTTHREADED, CoInitializeEx, CoUninitialize};

/// 맑은 고딕 — egui 기본 폰트에는 한글 글리프가 없어 파일명이 두부(□)로 보인다
const KOREAN_FONT_PATH: &str = r"C:\Windows\Fonts\malgun.ttf";

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

/// 셸 메뉴를 쓸 수 없을 때 화면에 보일 문구.
/// 원인이 무엇이든 사용자가 할 수 있는 일은 재시작뿐이라 한 문구로 통일한다
const SHELL_UNAVAILABLE: &str =
    "마우스 오른쪽 버튼 메뉴를 사용할 수 없습니다 (앱을 다시 시작해 주세요)";

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
    /// 첫 로드 — 히스토리에 이미 시작 경로가 들어 있다
    None,
    /// 새 이동 — 커서 뒤를 자르고 추가
    Push,
    Back,
    Forward,
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
    list: FileListView,
    load: DirLoad,
    address: AddressBar,
    history: History,
    /// 현재 **표시 중인** 폴더 (열거가 성공해 커밋된 것)
    dir: PathBuf,
    /// 열거 중인 대상 — 성공해야 `dir`로 커밋된다.
    /// 실패해도 현 위치를 유지하려는 것이다(삭제된 폴더로 이동을 시도해도 화면이 비지 않는다)
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

impl ExplorerApp {
    /// eframe 창 생성 직후 호출된다 — 폰트·팔레트·셸 호스트를 이 시점에 준비한다
    pub fn new(cc: &eframe::CreationContext<'_>, com: ComStatus) -> ExplorerApp {
        let korean_font = install_korean_font(&cc.egui_ctx);
        theme::apply_dark(&cc.egui_ctx);
        // HWND 획득·서브클래스 설치는 창이 만들어진 이 시점에만 가능하다
        let shell = ShellHost::new(cc);
        let start = start_dir();
        ExplorerApp {
            com,
            shell,
            korean_font,
            icons: IconCache::new(),
            textures: IconTextures::new(),
            list: FileListView::new(),
            load: DirLoad::new(),
            address: AddressBar::new(),
            history: History::new(start.clone()),
            dir: start.clone(),
            pending_dir: PathBuf::new(),
            pending_nav: PendingNav::None,
            status: String::new(),
            deferred_start: Some(start),
        }
    }

    /// 셸 메뉴를 쓸 수 있는가 — COM STA와 창 핸들이 모두 있어야 한다
    fn shell_available(&self) -> bool {
        self.com.is_available() && self.shell.is_some()
    }

    /// 폴더 이동 (사용자 조작) — 성공하면 히스토리에 남는다.
    ///
    /// 경로·히스토리는 열거가 **성공한 뒤에만** 커밋한다(pending-커밋).
    /// 실패해도 현 위치·목록이 그대로 남아 사용자가 길을 잃지 않는다
    fn navigate(&mut self, path: PathBuf, ctx: &egui::Context) {
        self.start_load(path, PendingNav::Push, ctx);
    }

    fn start_load(&mut self, path: PathBuf, nav: PendingNav, ctx: &egui::Context) {
        self.pending_dir = path.clone();
        self.pending_nav = nav;
        self.status.clear();
        self.load.start(path, ctx);
    }

    /// 주소창에서 올라온 탐색 요청 처리
    fn handle_nav(&mut self, action: NavAction, ctx: &egui::Context) {
        match action {
            NavAction::Back => {
                if let Some(path) = self.history.peek_back().map(PathBuf::from) {
                    self.start_load(path, PendingNav::Back, ctx);
                }
            }
            NavAction::Forward => {
                if let Some(path) = self.history.peek_forward().map(PathBuf::from) {
                    self.start_load(path, PendingNav::Forward, ctx);
                }
            }
            NavAction::Up => {
                if let Some(parent) = self.dir.parent().map(PathBuf::from) {
                    self.navigate(parent, ctx);
                }
            }
            NavAction::Goto(path) => self.navigate(path, ctx),
        }
    }

    /// 목록에서 올라온 조작을 처리한다.
    ///
    /// 셸 메뉴는 `TrackPopupMenuEx` 모달이라 프레임이 그 안에서 멈춘다 —
    /// 그리기 클로저 안이 아니라 프레임 구성이 끝난 뒤에 호출해야 한다
    fn handle_list_action(&mut self, action: FileListAction, ctx: &egui::Context) {
        match action {
            FileListAction::None => {}
            FileListAction::Open(index) => {
                let Some(entry) = self.list.entry_at(index) else {
                    return;
                };
                let target = self.dir.join(entry.name_string());
                if entry.is_dir {
                    self.navigate(target, ctx);
                } else {
                    shell_host::execute(&target);
                }
            }
            FileListAction::Context { index, pos } => {
                let Some(shell) = self.shell.as_ref() else {
                    return;
                };
                // 행 메뉴는 선택 전체가 대상이다 — 여러 항목을 골라 한 번에 복사·삭제할 수 있다.
                // 빈 영역이면 항목 없이 호출해 폴더 배경 메뉴("새로 만들기")를 띄운다
                let items = if index.is_some() {
                    self.list.selected_paths()
                } else {
                    Vec::new()
                };
                // egui 좌표는 논리 포인트라 물리 픽셀로 되돌린 뒤 화면 좌표로 바꾼다
                let scale = ctx.pixels_per_point();
                let (x, y) = shell.to_screen((pos.x * scale) as i32, (pos.y * scale) as i32);
                shell.popup(&self.dir, &items, x, y);
            }
        }
    }

    /// 워커 결과를 목록에 반영한다
    fn poll_load(&mut self) {
        let Some(outcome) = self.load.poll() else {
            return;
        };
        match outcome {
            EnumOutcome::Ok(entries) => {
                // 여기서 비로소 커밋한다 — 이 지점 전에는 화면이 이전 폴더를 유지한다
                self.dir = std::mem::take(&mut self.pending_dir);
                match self.pending_nav {
                    PendingNav::None => {}
                    PendingNav::Push => self.history.push(self.dir.clone()),
                    PendingNav::Back => {
                        self.history.back();
                    }
                    PendingNav::Forward => {
                        self.history.forward();
                    }
                }
                self.list
                    .set_entries(self.dir.clone(), entries, &mut self.icons);
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

impl eframe::App for ExplorerApp {
    /// 창 클리어 색 — eframe 기본값은 하드코딩된 회색이라 팔레트와 어긋난다.
    /// 이것을 덮어써야 크기 조절 중 노출되는 여백까지 창 배경색으로 칠해진다
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        theme::WINDOW_BG.to_normalized_gamma_f32()
    }

    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.textures.begin_frame();
        // 첫 프레임을 그린 뒤 열거를 시작한다 — 창이 먼저 뜨고 열거 중에도 응답한다.
        // 시작 경로는 이미 히스토리에 들어 있으므로 다시 쌓지 않는다
        if let Some(path) = self.deferred_start.take() {
            self.start_load(path, PendingNav::None, ctx);
        }
        self.poll_load();
        // 열거 중에는 계속 다시 그린다(진행 표시가 갱신되고 완료를 즉시 반영한다)
        if self.load.is_loading() {
            ctx.request_repaint();
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        let mut action = FileListAction::None;
        let mut nav: Option<NavAction> = None;
        // eframe이 주는 Ui는 여백·배경이 없다 — CentralPanel로 감싸야 panel_fill이 칠해진다
        egui::CentralPanel::default().show(ui, |ui| {
            nav = self.address.show(ui, &self.dir, &self.history);
            ui.horizontal(|ui| {
                if self.load.is_loading() {
                    ui.spinner();
                    ui.colored_label(theme::TEXT_DIM, "읽는 중…");
                } else {
                    ui.colored_label(theme::TEXT_DIM, format!("{}개 항목", self.list.len()));
                }
            });
            if !self.korean_font {
                ui.colored_label(
                    theme::TEXT_DIM,
                    "한글 글꼴을 불러오지 못해 기본 글꼴로 표시합니다",
                );
            }
            if !self.shell_available() {
                ui.colored_label(theme::TEXT_DIM, SHELL_UNAVAILABLE);
            }
            if !self.status.is_empty() {
                ui.colored_label(theme::TEXT_DIM, &self.status);
            }
            ui.separator();
            action = self.list.show(ui, &mut self.icons, &mut self.textures);
        });
        if let Some(nav) = nav {
            self.handle_nav(nav, &ctx);
        }
        // 셸 메뉴가 모달이라 그리기가 끝난 뒤에 처리한다 (메뉴가 뜬 동안 프레임이 멈춘다)
        self.handle_list_action(action, &ctx);
    }
}
