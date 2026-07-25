//! egui(eframe) 이식 가능성 PoC — 진입점 (plan: docs/plans/2026-07-25-egui-poc.md)
//!
//! 기존 Win32 앱(`src/main.rs`)과 별개인 **실험 바이너리**다. 세 가지를 실측 검증한다:
//! ① 대량 항목 가상 스크롤 성능 ② 시스템 아이콘 표시 ③ Win32 셸 컨텍스트 메뉴 연동.
//! `cargo run --bin egui_poc --features egui-poc`로만 빌드된다(기본 빌드에는 포함되지 않음).
//!
//! 화면에 표시하는 COM 상태·프레임 시간 등은 **측정을 위한 진단 정보**다 —
//! 일반 사용자용 화면이 아니라 이식 판단 근거를 눈으로 읽기 위한 것이다.
#![windows_subsystem = "windows"]

mod icon_tex;
mod shell_host;

// egui는 eframe이 재수출한 것을 쓴다 — 별도 의존성으로 넣으면 버전이 어긋날 수 있다
use eframe::egui;
use file_explorer::fs::enumerate::{EnumOutcome, FileEntry, enumerate_dir};
use file_explorer::fs::icons::IconCache;
use file_explorer::panel::file_list::{format_filetime, format_size_kb};
use icon_tex::IconTextures;
use shell_host::ShellHost;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, TryRecvError, channel};
use std::time::Instant;
use windows::Win32::Foundation::RPC_E_CHANGED_MODE;
use windows::Win32::System::Com::{COINIT_APARTMENTTHREADED, CoInitializeEx, CoUninitialize};
use windows::Win32::UI::Shell::StrCmpLogicalW;
use windows::core::PCWSTR;

/// 맑은 고딕 — egui 기본 폰트에는 한글 글리프가 없어 파일명이 두부(□)로 보인다
const KOREAN_FONT_PATH: &str = r"C:\Windows\Fonts\malgun.ttf";
/// 목록 행 높이 — 16px 시스템 아이콘(T3)이 들어갈 여유를 둔다
const ROW_HEIGHT: f32 = 20.0;
/// 열 폭 (머리글과 행이 같은 x를 쓰도록 상수로 공유). 마지막 열은 남는 폭 전부
const COL_NAME_W: f32 = 320.0;
const COL_SIZE_W: f32 = 90.0;
const COL_TYPE_W: f32 = 150.0;

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

/// 폴더 우선 + 이름 오름차순 (plan D7). 기존 앱과 같은 자연 정렬을 쓴다
fn sort_entries(entries: &mut [FileEntry]) {
    use std::cmp::Ordering;
    entries.sort_by(|a, b| match (a.is_dir, b.is_dir) {
        (true, false) => Ordering::Less,
        (false, true) => Ordering::Greater,
        // 안전성: 두 이름 모두 널 종단 UTF-16 (FileEntry 불변식)
        _ => unsafe { StrCmpLogicalW(PCWSTR(a.name.as_ptr()), PCWSTR(b.name.as_ptr())) }.cmp(&0),
    });
}

/// 스크롤 벤치 상태 (`--bench` 인자로 활성).
/// 사람이 휠을 돌리는 것으로는 측정이 재현되지 않아, 매 프레임 스크롤 위치를 강제로 옮기며
/// 렌더 시간을 표본으로 모은다 — 가상 스크롤이 실제로 보이는 행만 그리는지 판정하는 근거다.
struct Bench {
    active: bool,
    offset: f32,
    /// +1 아래로 / -1 위로 (끝에 닿으면 뒤집어 왕복)
    direction: f32,
    /// 로드 직후 몇 프레임은 초기 레이아웃 비용이 섞이므로 표본에서 뺀다
    warmup: u32,
    samples: Vec<f32>,
}

impl Bench {
    fn new(active: bool) -> Bench {
        Bench {
            active,
            offset: 0.0,
            direction: 1.0,
            warmup: 30,
            samples: Vec::new(),
        }
    }

    /// 표본 요약 (평균, p95, 최대)
    fn summary(&self) -> Option<(f32, f32, f32)> {
        if self.samples.is_empty() {
            return None;
        }
        let mut sorted = self.samples.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let sum: f32 = sorted.iter().sum();
        let p95 = sorted[(sorted.len() * 95 / 100).min(sorted.len() - 1)];
        let max = sorted[sorted.len() - 1];
        Some((sum / sorted.len() as f32, p95, max))
    }
}

/// PoC 앱 상태
struct PocApp {
    com: ComStatus,
    korean_font: bool,
    /// 현재 폴더
    dir: PathBuf,
    entries: Vec<FileEntry>,
    /// 종류 열 문자열 (entries와 같은 인덱스) — 렌더 중 셸 조회를 하지 않기 위해 미리 만든다
    type_names: Vec<String>,
    /// 아이콘 인덱스 (entries와 같은 인덱스). `None` = 아직 조회하지 않음 —
    /// 보이는 행만 렌더 시점에 채운다(exe·lnk의 개별 조회 비용을 로드에서 빼기 위함)
    icon_indices: Vec<Option<i32>>,
    icons: IconCache,
    icon_textures: IconTextures,
    /// 열거 상태 문구 (오류·진행 표시)
    status: String,
    /// 진행 중인 열거의 세대 — 늦게 도착한 이전 폴더 결과를 폐기한다
    generation: u64,
    pending: Option<Receiver<(u64, EnumOutcome)>>,
    /// 열거 시작 시각·소요·그동안 그린 프레임 수 —
    /// "워커 스레드 열거 중에도 UI가 계속 그려진다"(plan D9)를 수치로 남기기 위한 계측이다
    load_start: Option<Instant>,
    load_ms: f32,
    frames_during_load: u32,
    /// 이번 프레임 UI 구성이 시작된 시각 — 유휴 대기를 빼고 **렌더 소요만** 재기 위한 것
    /// (egui는 입력이 없으면 프레임을 그리지 않아, 프레임 간 간격은 성능 지표가 못 된다)
    frame_start: Instant,
    frame_ms: f32,
    /// 관측된 최대 렌더 시간 — 스크롤 중 스파이크 판정용
    frame_ms_max: f32,
    bench: Bench,
    /// 첫 프레임을 그린 뒤에 열거를 시작하기 위한 대기 경로.
    /// 생성자에서 바로 열거하면 창이 뜨기 전에 열거가 진행돼 ① 창 표시가 늦고
    /// ② "열거 중 몇 프레임을 그렸는가"가 측정되지 않는다
    deferred_start: Option<PathBuf>,
    /// 셸 컨텍스트 메뉴 호스트 — HWND를 얻지 못하면 None(메뉴 비활성)
    shell: Option<ShellHost>,
    /// 마지막으로 띄운 셸 메뉴 결과 — 화면에서 동작 여부를 확인하기 위한 진단
    shell_note: String,
    /// 행 Response 진단 계측 — hover는 잡히는데 click만 안 잡히는지 가른다
    hover_hits: u32,
    click_hits: u32,
    /// 행에 실제로 할당된 폭 — 클릭 영역이 좁아 입력을 못 받는지 판별한다
    row_width: f32,
}

impl PocApp {
    fn new(com: ComStatus, korean_font: bool, shell: Option<ShellHost>) -> PocApp {
        let bench_on = std::env::args().any(|a| a == "--bench");
        let app = PocApp {
            com,
            korean_font,
            dir: PathBuf::new(),
            entries: Vec::new(),
            type_names: Vec::new(),
            icon_indices: Vec::new(),
            icons: IconCache::new(),
            icon_textures: IconTextures::new(),
            status: String::new(),
            generation: 0,
            pending: None,
            load_start: None,
            load_ms: 0.0,
            frames_during_load: 0,
            frame_start: Instant::now(),
            frame_ms: 0.0,
            frame_ms_max: 0.0,
            bench: Bench::new(bench_on),
            deferred_start: None,
            shell,
            shell_note: String::new(),
            hover_hits: 0,
            click_hits: 0,
            row_width: 0.0,
        };
        // 시작 폴더를 인자로 받는다 — 대량 폴더 성능 측정을 자동화하기 위한 것
        // (주소 입력 UI는 PoC 범위 밖이라 인자로 대신한다)
        let start = std::env::args()
            .nth(1)
            .filter(|a| !a.starts_with("--"))
            .unwrap_or_else(|| std::env::var("USERPROFILE").unwrap_or_else(|_| "C:\\".to_owned()));
        PocApp {
            deferred_start: Some(PathBuf::from(start)),
            ..app
        }
    }

    /// 폴더 열거를 워커 스레드에서 시작한다 (plan D9 — UI 스레드 블로킹 금지)
    fn load_dir(&mut self, path: PathBuf, ctx: &egui::Context) {
        self.generation += 1;
        let generation = self.generation;
        let (tx, rx) = channel();
        self.pending = Some(rx);
        self.dir = path.clone();
        self.status = "열거 중…".to_owned();
        self.load_start = Some(Instant::now());
        self.frames_during_load = 0;
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let outcome = enumerate_dir(&path);
            // 수신 측이 이미 교체·파괴됐으면 send 실패 — 무해하게 종료
            if tx.send((generation, outcome)).is_ok() {
                ctx.request_repaint();
            }
        });
    }

    /// 워커 결과 수신 — 매 프레임 폴링
    fn poll_enum(&mut self) {
        let Some(rx) = &self.pending else {
            return;
        };
        let (generation, outcome) = match rx.try_recv() {
            Ok(v) => v,
            Err(TryRecvError::Empty) => return,
            Err(TryRecvError::Disconnected) => {
                self.pending = None;
                self.status = "열거 스레드가 결과 없이 종료됨".to_owned();
                return;
            }
        };
        self.pending = None;
        if let Some(start) = self.load_start.take() {
            self.load_ms = start.elapsed().as_secs_f32() * 1000.0;
        }
        // 다른 폴더로 이동한 뒤 도착한 결과는 버린다
        if generation != self.generation {
            return;
        }
        match outcome {
            EnumOutcome::Ok(entries) => {
                self.entries = entries;
                sort_entries(&mut self.entries);
                self.rebuild_row_meta();
                self.status.clear();
            }
            EnumOutcome::AccessDenied => self.fail("이 폴더에 접근할 권한이 없습니다"),
            EnumOutcome::NotFound => self.fail("폴더를 찾을 수 없습니다"),
            EnumOutcome::Error => self.fail("폴더를 읽는 중 오류가 발생했습니다"),
        }
    }

    fn fail(&mut self, message: &str) {
        self.entries.clear();
        self.type_names.clear();
        self.icon_indices.clear();
        self.status = message.to_owned();
    }

    /// 종류 문자열만 미리 계산한다 — 확장자별 캐시라 대량 폴더에서도 조회는 몇 종류뿐이다.
    ///
    /// 아이콘 인덱스는 **여기서 계산하지 않는다**: exe·lnk는 파일마다 개별 조회(디스크 접근)라
    /// 전량 미리 계산하면 그런 파일이 많은 폴더에서 로드가 길어진다. 기존 앱처럼 **보이는 행만**
    /// 렌더 시점에 조회한다(`None` = 아직 조회 안 함)
    fn rebuild_row_meta(&mut self) {
        let meta: Vec<(String, bool)> = self
            .entries
            .iter()
            .map(|e| (e.extension(), e.is_dir))
            .collect();
        self.type_names = meta
            .iter()
            .map(|(ext, is_dir)| self.icons.type_name(ext, *is_dir))
            .collect();
        self.icon_indices = vec![None; self.entries.len()];
    }

    /// 셸 컨텍스트 메뉴를 띄운다. `index`가 없으면 폴더 배경 메뉴.
    /// 메뉴가 닫힐 때까지 반환하지 않는다(모달) — egui 프레임도 그동안 멈춘다
    fn show_shell_menu(&mut self, ctx: &egui::Context, index: Option<usize>, pos: egui::Pos2) {
        if !matches!(self.com, ComStatus::Sta { .. }) {
            self.shell_note = "COM STA가 아니어서 셸 메뉴를 띄울 수 없습니다".to_owned();
            return;
        }
        let Some(shell) = &self.shell else {
            self.shell_note = "창 핸들(HWND)을 얻지 못해 셸 메뉴를 띄울 수 없습니다".to_owned();
            return;
        };
        // egui 좌표는 논리 포인트라 물리 픽셀로 되돌린 뒤 화면 좌표로 바꾼다
        let scale = ctx.pixels_per_point();
        let (screen_x, screen_y) = shell.to_screen((pos.x * scale) as i32, (pos.y * scale) as i32);
        let items: Vec<PathBuf> = match index {
            Some(i) => match self.entries.get(i) {
                Some(entry) => vec![self.dir.join(entry.name_string())],
                None => Vec::new(),
            },
            None => Vec::new(),
        };
        let target = match items.first() {
            Some(path) => path.display().to_string(),
            None => format!("{} (배경)", self.dir.display()),
        };
        shell.popup(&self.dir, &items, screen_x, screen_y);
        self.shell_note = format!("셸 메뉴 표시: {target}");
    }

    fn go_parent(&mut self, ctx: &egui::Context) {
        if let Some(parent) = self.dir.parent().map(PathBuf::from) {
            self.load_dir(parent, ctx);
        }
    }

    /// 상단 진단·탐색 바
    fn top_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if ui.button("↑ 상위").clicked() {
                let ctx = ui.ctx().clone();
                self.go_parent(&ctx);
            }
            // 경로 라벨 우클릭 = 폴더 배경 메뉴("새로 만들기" 등).
            // 목록 아래 여백에 배경 응답을 깔면 행 클릭을 가로채므로 여기로 옮겼다
            let path_label = ui
                .add(egui::Label::new(self.dir.display().to_string()).sense(egui::Sense::click()));
            if path_label.secondary_clicked()
                && let Some(pos) = path_label.interact_pointer_pos()
            {
                let ctx = ui.ctx().clone();
                self.show_shell_menu(&ctx, None, pos);
            }
        });
        ui.horizontal(|ui| {
            // 스피너는 애니메이션이라 매 프레임 다시 그리게 만든다 —
            // 열거 중에도 UI가 실제로 돌고 있음을 눈으로 확인할 수 있다
            if self.pending.is_some() {
                ui.spinner();
                ui.label("열거 중");
                ui.separator();
            }
            ui.label(format!("{}개 항목", self.entries.len()));
            ui.separator();
            ui.label(format!(
                "렌더 {:.2} ms (최대 {:.2})",
                self.frame_ms, self.frame_ms_max
            ));
            ui.separator();
            ui.label(format!(
                "로드 {:.0} ms (그동안 {}프레임 렌더)",
                self.load_ms, self.frames_during_load
            ));
            ui.separator();
            ui.label(format!("아이콘 텍스처 {}개", self.icon_textures.created()));
            ui.separator();
            ui.label(format!("아이콘 {}", self.icon_textures.created()));
            ui.separator();
            ui.label(self.com.label());
            if !self.korean_font {
                ui.separator();
                ui.colored_label(
                    egui::Color32::from_rgb(0xE0, 0x80, 0x40),
                    "한글 폰트 없음 — 기본 폰트 사용",
                );
            }
        });
        // 진단 줄 — 입력이 어디까지 도달하는지 한눈에 본다
        let pointer = ui.ctx().input(|i| i.pointer.latest_pos());
        ui.label(format!(
            "진단: 행 hover {} · click {} · 포인터 {} · 행폭 {:.0}",
            self.hover_hits,
            self.click_hits,
            match pointer {
                Some(p) => format!("({:.0},{:.0})", p.x, p.y),
                None => "없음".to_owned(),
            },
            self.row_width
        ));
        if let Some((avg, p95, max)) = self.bench.summary() {
            ui.label(format!(
                "벤치(스크롤 {}프레임): 평균 {avg:.2} ms · p95 {p95:.2} ms · 최대 {max:.2} ms",
                self.bench.samples.len()
            ));
        }
        if !self.shell_note.is_empty() {
            ui.label(&self.shell_note);
        }
        if !self.status.is_empty() {
            ui.colored_label(egui::Color32::from_rgb(0xE0, 0x80, 0x40), &self.status);
        }
    }

    /// 열 머리글 — 목록과 같은 x 좌표를 쓰도록 상수로 폭을 맞춘다
    fn header_row(&self, ui: &mut egui::Ui) {
        let (rect, _) =
            ui.allocate_exact_size(egui::vec2(ui.available_width(), 22.0), egui::Sense::hover());
        let painter = ui.painter();
        let font = egui::TextStyle::Body.resolve(ui.style());
        let color = ui.visuals().strong_text_color();
        let y = rect.center().y;
        for (title, x) in [
            ("이름", rect.left() + 6.0),
            ("크기", rect.left() + COL_NAME_W + 6.0),
            ("종류", rect.left() + COL_NAME_W + COL_SIZE_W + 6.0),
            (
                "수정한 날짜",
                rect.left() + COL_NAME_W + COL_SIZE_W + COL_TYPE_W + 6.0,
            ),
        ] {
            painter.text(
                egui::pos2(x, y),
                egui::Align2::LEFT_CENTER,
                title,
                font.clone(),
                color,
            );
        }
        painter.hline(
            rect.x_range(),
            rect.bottom(),
            ui.visuals().widgets.noninteractive.bg_stroke,
        );
    }

    /// 파일 목록 — `ScrollArea::show_rows`가 보이는 행만 렌더한다(가상 스크롤).
    ///
    /// `egui_extras::TableBuilder`를 쓰지 않는 이유: 셀 Response가 `sense(click)`을 줘도
    /// hover·click을 전혀 감지하지 못해(실측: hover 0·click 0) 더블클릭 진입과 우클릭 메뉴가
    /// 불가능했다. 행 rect를 직접 할당하면 입력이 확실히 잡히고, 그리기도 painter 한 번으로 끝난다.
    ///
    /// 반환값은 "이 프레임에 셸 메뉴를 띄웠는가" — 배경 메뉴가 연달아 뜨는 것을 막는다
    fn table(&mut self, ui: &mut egui::Ui) -> bool {
        // 클로저가 self를 통째로 빌리지 않도록 필요한 것만 미리 참조로 분리한다
        let entries = &self.entries;
        let type_names = &self.type_names;
        let icon_indices = &mut self.icon_indices;
        let icon_textures = &mut self.icon_textures;
        let icons = &mut self.icons;
        let dir = &self.dir;
        let himl = icons.himl();
        let ctx = ui.ctx().clone();
        let mut open_index = None;
        // 우클릭은 모달 메뉴를 띄우므로 스크롤 클로저 안에서 바로 실행하지 않고 요청만 모은다
        let mut menu_request: Option<(Option<usize>, egui::Pos2)> = None;
        let hover_hits = &mut self.hover_hits;
        let click_hits = &mut self.click_hits;
        let row_width = &mut self.row_width;

        let mut scroll = egui::ScrollArea::vertical().auto_shrink([false, false]);
        // 벤치 모드에서는 스크롤 위치를 강제로 지정해 렌더 부하를 재현한다
        if self.bench.active {
            scroll = scroll.vertical_scroll_offset(self.bench.offset);
        }
        scroll.show_rows(ui, ROW_HEIGHT, entries.len(), |ui, range| {
            let font = egui::TextStyle::Body.resolve(ui.style());
            let text_color = ui.visuals().text_color();
            let stripe = ui.visuals().faint_bg_color;
            let hover_bg = ui.visuals().widgets.hovered.bg_fill;
            for index in range {
                let entry = &entries[index];
                // 행 전체를 하나의 클릭 대상으로 할당한다 — 어느 열을 눌러도 같은 행으로 잡힌다.
                // `allocate_space`로 자리와 id를 얻고 `interact`로 명시 감지한다
                // (`allocate_exact_size(.., Sense::click())`는 이 스크롤 영역 안에서 hover조차 잡히지 않았다)
                let (id, rect) = ui.allocate_space(egui::vec2(ui.available_width(), ROW_HEIGHT));
                let resp = ui.interact(rect, id, egui::Sense::click());
                *row_width = rect.width();
                if resp.hovered() {
                    *hover_hits += 1;
                }
                if resp.clicked() || resp.secondary_clicked() {
                    *click_hits += 1;
                }
                if resp.double_clicked() && entry.is_dir {
                    open_index = Some(index);
                }
                if resp.secondary_clicked()
                    && let Some(pos) = resp.interact_pointer_pos()
                {
                    menu_request = Some((Some(index), pos));
                }
                if !ui.is_rect_visible(rect) {
                    continue;
                }

                let painter = ui.painter();
                if index % 2 == 1 {
                    painter.rect_filled(rect, 0.0, stripe);
                }
                if resp.hovered() {
                    painter.rect_filled(rect, 0.0, hover_bg);
                }
                let y = rect.center().y;
                // 보이는 행에 한해 아이콘 인덱스를 이때 조회한다 (지연 조회)
                let icon_index = match icon_indices.get(index).copied().flatten() {
                    Some(cached) => cached,
                    None => {
                        let full = dir.join(entry.name_string()).to_string_lossy().into_owned();
                        let looked_up =
                            icons.icon_index(&entry.extension(), entry.is_dir, Some(&full));
                        if let Some(slot) = icon_indices.get_mut(index) {
                            *slot = Some(looked_up);
                        }
                        looked_up
                    }
                };
                if let Some(tex) = icon_textures.get(&ctx, himl, icon_index) {
                    let icon_rect = egui::Rect::from_min_size(
                        egui::pos2(rect.left() + 4.0, y - 8.0),
                        egui::vec2(16.0, 16.0),
                    );
                    // painter를 다시 얻는다 — icon_textures가 ui를 빌리는 사이 painter가 무효화된다
                    ui.painter().image(
                        tex.id(),
                        icon_rect,
                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                        egui::Color32::WHITE,
                    );
                }

                let size_text = if entry.is_dir {
                    String::new()
                } else {
                    format_size_kb(entry.size)
                };
                let cells = [
                    (entry.name_string(), rect.left() + 24.0, COL_NAME_W - 28.0),
                    (size_text, rect.left() + COL_NAME_W + 6.0, COL_SIZE_W - 10.0),
                    (
                        type_names.get(index).cloned().unwrap_or_default(),
                        rect.left() + COL_NAME_W + COL_SIZE_W + 6.0,
                        COL_TYPE_W - 10.0,
                    ),
                    (
                        format_filetime(entry.modified),
                        rect.left() + COL_NAME_W + COL_SIZE_W + COL_TYPE_W + 6.0,
                        rect.width() - (COL_NAME_W + COL_SIZE_W + COL_TYPE_W) - 10.0,
                    ),
                ];
                for (text, x, width) in cells {
                    if text.is_empty() {
                        continue;
                    }
                    // 열 밖으로 넘치는 긴 이름은 셀 경계에서 자른다
                    let clip = egui::Rect::from_min_size(
                        egui::pos2(x, rect.top()),
                        egui::vec2(width.max(0.0), rect.height()),
                    );
                    ui.painter().with_clip_rect(clip).text(
                        egui::pos2(x, y),
                        egui::Align2::LEFT_CENTER,
                        text,
                        font.clone(),
                        text_color,
                    );
                }
            }
        });

        // 스크롤 클로저가 끝나 self 차용이 풀린 뒤에 폴더를 이동한다
        if let Some(index) = open_index {
            let path = self.dir.join(self.entries[index].name_string());
            let ctx = ui.ctx().clone();
            self.load_dir(path, &ctx);
        }
        if let Some((index, pos)) = menu_request {
            let ctx = ui.ctx().clone();
            self.show_shell_menu(&ctx, index, pos);
            return true;
        }
        false
    }
}

impl eframe::App for PocApp {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.frame_start = Instant::now();
        self.icon_textures.begin_frame();
        // 첫 프레임을 그린 뒤 열거를 시작한다 (창이 먼저 뜨고, 열거 중 응답성이 측정된다)
        if let Some(path) = self.deferred_start.take() {
            self.load_dir(path, ctx);
        }
        self.poll_enum();
        // 열거 중에는 매 프레임 다시 그린다 — 진행 표시가 갱신되고, UI가 살아 있음이 수치로 남는다
        if self.pending.is_some() {
            ctx.request_repaint();
        }
        if self.bench.active && !self.entries.is_empty() {
            // 스크롤 가능 범위를 행 높이로 근사해 위아래로 왕복시킨다
            let span = self.entries.len() as f32 * (ROW_HEIGHT + 6.0);
            self.bench.offset += self.bench.direction * 400.0;
            if self.bench.offset <= 0.0 || self.bench.offset >= span {
                self.bench.direction = -self.bench.direction;
                self.bench.offset = self.bench.offset.clamp(0.0, span);
            }
            // 연속 렌더로 만들어야 표본이 쌓인다 (egui는 입력이 없으면 그리지 않는다)
            ctx.request_repaint();
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // F5 새로고침 — 창이 이미 떠 있는 상태에서의 열거 응답성을 측정하는 수단이기도 하다
        if ui.input(|i| i.key_pressed(egui::Key::F5)) {
            let (path, ctx) = (self.dir.clone(), ui.ctx().clone());
            self.load_dir(path, &ctx);
        }
        self.top_bar(ui);
        ui.separator();
        self.header_row(ui);
        self.table(ui);
        // UI 구성이 끝난 시점에 소요를 확정한다 (다음 프레임의 표시에 쓰인다)
        self.frame_ms = self.frame_start.elapsed().as_secs_f32() * 1000.0;
        self.frame_ms_max = self.frame_ms_max.max(self.frame_ms);
        if self.load_start.is_some() {
            self.frames_during_load += 1;
        }
        if self.bench.active && !self.entries.is_empty() {
            if self.bench.warmup > 0 {
                self.bench.warmup -= 1;
            } else {
                self.bench.samples.push(self.frame_ms);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(name: &str, is_dir: bool) -> FileEntry {
        let mut wide: Vec<u16> = name.encode_utf16().collect();
        wide.push(0);
        FileEntry {
            name: wide,
            is_dir,
            size: 0,
            modified: 0,
        }
    }

    fn names(entries: &[FileEntry]) -> Vec<String> {
        entries.iter().map(|e| e.name_string()).collect()
    }

    #[test]
    fn 폴더가_파일보다_앞에_온다() {
        let mut list = vec![entry("zeta.txt", false), entry("alpha", true)];
        sort_entries(&mut list);
        assert_eq!(names(&list), vec!["alpha", "zeta.txt"]);
    }

    #[test]
    fn 이름은_숫자_인지_정렬이다() {
        let mut list = vec![entry("파일10.txt", false), entry("파일2.txt", false)];
        sort_entries(&mut list);
        assert_eq!(names(&list), vec!["파일2.txt", "파일10.txt"]);
    }

    #[test]
    fn 빈_목록도_안전하다() {
        let mut list: Vec<FileEntry> = Vec::new();
        sort_entries(&mut list);
        assert!(list.is_empty());
    }
}

fn main() -> eframe::Result {
    let com = init_com();
    // 기본은 wgpu. `--glow`를 주면 OpenGL 백엔드로 띄워 메모리를 비교 측정한다
    let renderer = if std::env::args().any(|a| a == "--glow") {
        eframe::Renderer::Glow
    } else {
        eframe::Renderer::Wgpu
    };
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1100.0, 700.0])
            .with_title("egui PoC — 파일 목록"),
        renderer,
        ..Default::default()
    };
    let result = eframe::run_native(
        "egui_poc",
        options,
        Box::new(move |cc| {
            // `--no-font`: 한글 폰트(맑은 고딕 13MB)가 메모리에 얼마나 기여하는지 재기 위한 측정 스위치
            let korean_font = if std::env::args().any(|a| a == "--no-font") {
                false
            } else {
                install_korean_font(&cc.egui_ctx)
            };
            cc.egui_ctx.set_visuals(egui::Visuals::dark());
            // HWND 획득·서브클래스 설치는 창이 만들어진 이 시점에만 가능하다
            let shell = ShellHost::new(cc);
            Ok(Box::new(PocApp::new(com, korean_font, shell)))
        }),
    );
    if let ComStatus::Sta { owned: true } = com {
        // 이 바이너리가 초기화한 경우에만 해제한다
        // 안전성: init_com이 S_OK를 받은 같은 스레드에서 1회 호출
        unsafe { CoUninitialize() };
    }
    result
}
