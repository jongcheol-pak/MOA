//! egui(eframe) 이식 가능성 PoC — 진입점 (plan: docs/plans/2026-07-25-egui-poc.md)
//!
//! 기존 Win32 앱(`src/main.rs`)과 별개인 **실험 바이너리**다. 세 가지를 실측 검증한다:
//! ① 대량 항목 가상 스크롤 성능 ② 시스템 아이콘 표시 ③ Win32 셸 컨텍스트 메뉴 연동.
//! `cargo run --bin egui_poc --features egui-poc`로만 빌드된다(기본 빌드에는 포함되지 않음).
#![windows_subsystem = "windows"]

// egui는 eframe이 재수출한 것을 쓴다 — 별도 의존성으로 넣으면 버전이 어긋날 수 있다
use eframe::egui;
use std::sync::Arc;
use windows::Win32::Foundation::RPC_E_CHANGED_MODE;
use windows::Win32::System::Com::{COINIT_APARTMENTTHREADED, CoInitializeEx, CoUninitialize};

/// 맑은 고딕 — egui 기본 폰트에는 한글 글리프가 없어 파일명이 두부(□)로 보인다
const KOREAN_FONT_PATH: &str = r"C:\Windows\Fonts\malgun.ttf";

/// UI 스레드의 COM 아파트먼트 상태 (plan D5).
/// 셸 컨텍스트 메뉴(IContextMenu)는 STA를 요구하므로 T4의 가용 여부가 여기서 갈린다
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ComStatus {
    /// STA 확보 — 이 프로세스가 초기화했거나(S_OK) 이미 초기화돼 있었다(S_FALSE)
    Sta { owned: bool },
    /// 다른 아파트먼트로 이미 초기화됨 — 셸 메뉴 검증 불가
    WrongApartment,
    /// 그 외 실패
    Failed,
}

impl ComStatus {
    fn label(self) -> &'static str {
        match self {
            ComStatus::Sta { .. } => "COM: STA 확보",
            ComStatus::WrongApartment => "COM: STA 확보 실패 (다른 아파트먼트) — 셸 메뉴 사용 불가",
            ComStatus::Failed => "COM: 초기화 실패 — 셸 메뉴 사용 불가",
        }
    }
}

/// COM을 STA로 초기화한다. 반환을 세 갈래로 처리한다 (plan D5) —
/// `S_OK`(이번에 초기화)·`S_FALSE`(이미 초기화됨) 모두 STA 확보로 보고 진행한다.
fn init_com() -> ComStatus {
    // 안전성: UI 스레드에서 1회 호출. 인자는 정적 상수이며 반환 HRESULT로만 분기한다
    let hr = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
    if hr.is_ok() {
        // S_FALSE면 이미 초기화된 상태라 이 바이너리가 해제 책임을 지지 않는다
        ComStatus::Sta {
            owned: hr == windows::Win32::Foundation::S_OK,
        }
    } else if hr == RPC_E_CHANGED_MODE {
        ComStatus::WrongApartment
    } else {
        ComStatus::Failed
    }
}

/// 한글 폰트를 egui에 등록한다. 폰트 파일이 없으면 기본 폰트로 진행한다(반환 false)
fn install_korean_font(ctx: &egui::Context) -> bool {
    let Ok(bytes) = std::fs::read(KOREAN_FONT_PATH) else {
        return false;
    };
    let mut fonts = egui::FontDefinitions::default();
    fonts
        .font_data
        .insert("malgun".to_owned(), Arc::new(egui::FontData::from_owned(bytes)));
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

/// PoC 앱 상태
struct PocApp {
    com: ComStatus,
    korean_font: bool,
}

impl PocApp {
    fn new(com: ComStatus, korean_font: bool) -> PocApp {
        PocApp { com, korean_font }
    }
}

impl eframe::App for PocApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        ui.heading("egui 이식 PoC — 파일 목록");
        ui.separator();
        ui.label(self.com.label());
        if !self.korean_font {
            ui.colored_label(
                egui::Color32::from_rgb(0xE0, 0x80, 0x40),
                "한글 폰트를 찾지 못해 기본 폰트로 표시합니다",
            );
        }
        ui.label("가나다라마바사 ABC 0123 — 한글 표시 확인용");
    }
}

fn main() -> eframe::Result {
    let com = init_com();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1100.0, 700.0])
            .with_title("egui PoC — 파일 목록"),
        ..Default::default()
    };
    let result = eframe::run_native(
        "egui_poc",
        options,
        Box::new(move |cc| {
            let korean_font = install_korean_font(&cc.egui_ctx);
            cc.egui_ctx.set_visuals(egui::Visuals::dark());
            Ok(Box::new(PocApp::new(com, korean_font)))
        }),
    );
    if let ComStatus::Sta { owned: true } = com {
        // 이 바이너리가 초기화한 경우에만 해제한다
        // 안전성: init_com이 S_OK를 받은 같은 스레드에서 1회 호출
        unsafe { CoUninitialize() };
    }
    result
}
