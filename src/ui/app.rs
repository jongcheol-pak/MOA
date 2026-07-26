//! egui 앱 골격 — 창·폰트·팔레트·COM·셸 호스트와 전역 공유 자원을 보유한다.
//!
//! 실제 탐색은 `ui::panel::PanelState`가 담당하며, 이 구조체는 그것을 담는 그릇이다.
//! 분할 레이아웃(여러 패널)은 T7에서 이 자리에 붙는다.
use crate::fs::icons::IconCache;
use crate::ui::icon_tex::IconTextures;
use crate::ui::panel::PanelState;
use crate::ui::shell_host::ShellHost;
use crate::ui::theme;
use eframe::egui;
use std::path::PathBuf;
use std::sync::Arc;
use windows::Win32::Foundation::RPC_E_CHANGED_MODE;
use windows::Win32::System::Com::{COINIT_APARTMENTTHREADED, CoInitializeEx, CoUninitialize};

/// 맑은 고딕 — egui 기본 폰트에는 한글 글리프가 없어 파일명이 두부(□)로 보인다
const KOREAN_FONT_PATH: &str = r"C:\Windows\Fonts\malgun.ttf";

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
    /// 탐색 패널. T7에서 분할 레이아웃이 이 자리를 여러 패널로 확장한다
    panel: PanelState,
}

impl ExplorerApp {
    /// eframe 창 생성 직후 호출된다 — 폰트·팔레트·셸 호스트를 이 시점에 준비한다
    pub fn new(cc: &eframe::CreationContext<'_>, com: ComStatus) -> ExplorerApp {
        let korean_font = install_korean_font(&cc.egui_ctx);
        theme::apply_dark(&cc.egui_ctx);
        // HWND 획득·서브클래스 설치는 창이 만들어진 이 시점에만 가능하다
        let shell = ShellHost::new(cc);
        ExplorerApp {
            com,
            shell,
            korean_font,
            icons: IconCache::new(),
            textures: IconTextures::new(),
            panel: PanelState::new(start_dir()),
        }
    }

    /// 셸 메뉴를 쓸 수 있는가 — COM STA와 창 핸들이 모두 있어야 한다
    fn shell_available(&self) -> bool {
        self.com.is_available() && self.shell.is_some()
    }
}

impl eframe::App for ExplorerApp {
    /// 창 클리어 색 — eframe 기본값은 하드코딩된 회색이라 팔레트와 어긋난다.
    /// 이것을 덮어써야 크기 조절 중 노출되는 여백까지 창 배경색으로 칠해진다
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        theme::WINDOW_BG.to_normalized_gamma_f32()
    }

    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.textures.begin_frame();
        self.panel.poll(ctx, &mut self.icons);
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
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
            self.panel.show(
                ui,
                &ctx,
                &mut self.icons,
                &mut self.textures,
                self.shell.as_ref(),
            );
        });
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
