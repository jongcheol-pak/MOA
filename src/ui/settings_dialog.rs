//! 앱 설정 대화 (FR-47).
//!
//! 타이틀바 설정 메뉴의 `설정`이 연다. 항목이 일곱 개뿐이라 좌측 카테고리 목록을 두지 않고
//! **한 면에 그룹을 세로로 나열**한다(사용자 결정) — 카테고리를 두면 목록 하나에 항목이
//! 하나씩 매달려 클릭만 늘고 빈 공간이 많아진다.
//!
//! 바꾼 값은 **즉시 반영·저장**된다(사용자 결정). 일곱 항목이 모두 독립적인 토글·선택이라
//! 서로 엮어 검증할 것이 없고, 글꼴·언어는 바뀌는 결과를 보며 고르는 편이 확실하다.
//! 그래서 바닥 버튼은 `닫기` 하나이고 `취소`가 없다.
use crate::app::autostart;
use crate::app::settings::{
    AppSettings, LanguageSetting, REMOTE_REFRESH_CHOICES, TRANSFER_RETRY_RANGE,
    remote_refresh_secs, transfer_retry_count,
};
use crate::i18n;
use crate::ui::dialog;
use crate::ui::theme;
use crate::ui::widgets;
use eframe::egui;

/// 대화 크기 — 사이트 관리자(1080×680)보다 작지만 **여섯 그룹이 다 들어갈 만큼**은 된다.
///
/// 처음 잡은 420×400은 `언어` 그룹이 잘리고 바닥의 `닫기`와 겹쳤다(2026-08-14 화면 확인).
/// 그래도 창이 작으면 넘칠 수 있어 본문은 스크롤한다
const DIALOG_WIDTH: f32 = 480.0;
/// **2026-08-27에 560 → 660으로 올렸다** — `원격` 그룹(FR-67)이 늘며 여섯 그룹이 됐고,
/// 560에서는 본문이 바닥 버튼 자리를 넘어 마지막 그룹을 스크롤해야만 볼 수 있었다.
/// 스크롤은 넘쳤을 때의 안전망이지 평상시 배치가 아니다 (아래 두 시험이 그 둘을 따로 잰다).
///
/// **2026-08-28에 660 → 700으로 다시 올렸다** — `원격` 그룹에 전송 재시도 횟수(FR-37)가
/// 한 줄 붙어 같은 일이 생겼다(본문 582px > 남은 자리 564px)
const DIALOG_HEIGHT: f32 = 700.0;

/// 헤더 높이 — 사이트 관리자와 같은 값이다. 같은 창 안의 대화 둘이 제목 줄 높이를
/// 다르게 쓰면 번갈아 열었을 때 판이 흔들려 보인다.
/// 바닥 줄 높이는 `dialog::FOOTER_HEIGHT`가 정본이다
const HEADER_HEIGHT: f32 = 40.0;
/// 그룹을 가르는 선 — 제목 위에 그어 앞 그룹과 끊는다 (첫 그룹 위에는 긋지 않는다)
const DIVIDER_THICKNESS: f32 = 1.0;
/// 구분선과 그 아래 그룹 제목 사이 여백
const DIVIDER_GAP: f32 = 10.0;
/// 본문 좌우 여백
const BODY_PAD_X: f32 = 20.0;
/// 헤더 제목의 왼쪽 여백 — 본문과 같은 선에서 시작한다
const HEADER_PAD_LEFT: f32 = 20.0;
const TITLE_FONT_PX: f32 = 14.0;
/// 그룹 제목 — 본문 글자보다 작고 흐리다(항목이 아니라 묶음 이름임을 보인다)
const GROUP_FONT_PX: f32 = 12.0;
/// 그룹 제목 위 여백 — 앞 그룹과 떨어뜨린다
const GROUP_GAP_TOP: f32 = 14.0;
/// 그룹 제목과 첫 항목 사이
const GROUP_GAP_BOTTOM: f32 = 6.0;
/// 글꼴 드롭다운 폭 — 라벨(96px) 뒤 남는 자리에 맞춘다
const FONT_FIELD_WIDTH: f32 = 240.0;
/// 언어 드롭다운 폭 — 항목이 짧아 글꼴 목록만큼 넓을 이유가 없다
const LANGUAGE_FIELD_WIDTH: f32 = 160.0;
/// 갱신 주기 드롭다운 폭 — 항목이 `10초`~`10 minutes`라 언어 폭보다 좁아도 된다
const REFRESH_FIELD_WIDTH: f32 = 120.0;

// ── 문구 ──

/// 대화가 한 프레임에 만들어 낸 결과.
///
/// **부수 효과가 필요한 항목마다 필드를 따로 둔다** — 값을 저장하는 것과, 그 값을 바깥
/// 세계(글꼴 등록·레지스트리·트레이 아이콘)에 반영하는 것은 서로 다른 일이라
/// `changed` 하나로는 무엇을 해야 하는지 알 수 없다. 필드는 그 일이 생기는 task가 더한다
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SettingsOutcome {
    /// 값이 하나라도 바뀌었다 — 세션을 저장해야 한다
    pub changed: bool,
    /// 글꼴이 바뀌었다 — `install_fonts`를 다시 불러야 화면에 반영된다 (FR-48)
    pub font_changed: bool,
    /// 언어가 바뀌었다 — `i18n::set_language`를 다시 불러야 화면에 반영된다 (FR-53)
    pub language_changed: bool,
    /// 사용자에게 알릴 것이 생겼다 — 지금은 자동 실행 등록 실패뿐이다 (FR-49)
    pub notice: Option<&'static str>,
}

/// 대화가 그릴 글꼴 목록 — 워커가 아직 읽는 중이면 `None`.
///
/// 목록을 대화가 직접 만들지 않는 이유: 만드는 데 1.5초가 걸려 UI 스레드에서 할 수 없다
/// (`ui::font_scan`). 대화는 받은 것을 그리기만 한다
#[derive(Debug, Clone, Copy)]
pub struct FontChoices<'a> {
    pub names: Option<&'a [String]>,
}

/// 설정 대화 (FR-47) — 열림 상태만 들고 값은 빌려 쓴다.
///
/// 값을 자기 안에 복사해 두지 않는 이유: 즉시 반영이라 초안이 필요 없고, 사본을 두면
/// 화면과 실제 설정이 어긋날 수 있는 자리가 하나 더 생긴다(사이트 관리자의 `Draft`는
/// `확인`을 눌러야 반영되는 구조라 사본이 필요했다 — 여기는 그렇지 않다)
#[derive(Debug, Default)]
pub struct SettingsDialog {
    open: bool,
}

impl SettingsDialog {
    pub fn new() -> SettingsDialog {
        SettingsDialog::default()
    }

    pub fn open(&mut self) {
        self.open = true;
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn close(&mut self) {
        self.open = false;
    }

    /// 대화를 그린다. 닫혀 있으면 아무것도 그리지 않는다
    pub fn show(
        &mut self,
        ctx: &egui::Context,
        settings: &mut AppSettings,
        fonts: FontChoices<'_>,
    ) -> SettingsOutcome {
        if !self.open {
            return SettingsOutcome::default();
        }
        let mut outcome = SettingsOutcome::default();
        // 바닥 버튼이 하나뿐이라 그것이 곧 이 대화의 주 동작이다
        let buttons = [dialog::ButtonSpec::strong(i18n::close())];
        let shell = dialog::show_fixed(
            ctx,
            egui::Id::new("앱 설정"),
            egui::vec2(DIALOG_WIDTH, DIALOG_HEIGHT),
            &buttons,
            |ui, rect| {
                // `rect`는 바닥 버튼 줄을 뺀 나머지다 — 헤더와 본문이 그 안을 나눠 쓴다
                let header =
                    egui::Rect::from_min_size(rect.min, egui::vec2(rect.width(), HEADER_HEIGHT));
                let body = egui::Rect::from_min_max(
                    egui::pos2(rect.left() + BODY_PAD_X, header.bottom()),
                    egui::pos2(rect.right() - BODY_PAD_X, rect.bottom()),
                );
                show_header(ui, header);
                // **본문만 스크롤한다** — 제목과 `닫기`는 늘 제자리에 있어야 한다.
                // 스크롤 영역이 자기 자리(`body`) 밖으로 그리지 않으므로 바닥 버튼과
                // 겹치지도 않는다(겹침이 바로 이 영역을 두지 않아 생겼다)
                let mut body_ui = ui.new_child(egui::UiBuilder::new().max_rect(body));
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(&mut body_ui, |ui| {
                        let inner = ui.max_rect();
                        outcome = show_body(ui, inner, settings, fonts);
                    });
            },
        );
        // 배경 클릭·`Esc`도 닫기다 — 셸이 그 판정을 해 준다
        if shell.clicked.is_some() || shell.should_close {
            self.close();
        }
        outcome
    }
}

/// 헤더 — 제목만 둔다. 닫기 버튼을 겹쳐 두지 않는 것은 바닥에 `닫기`가 이미 있어서다
fn show_header(ui: &egui::Ui, rect: egui::Rect) {
    ui.painter().text(
        egui::pos2(rect.left() + HEADER_PAD_LEFT, rect.center().y),
        egui::Align2::LEFT_CENTER,
        i18n::settings_title(),
        egui::FontId::proportional(TITLE_FONT_PX),
        theme::TEXT,
    );
}

/// 본문 — 그룹을 세로로 쌓는다.
///
/// **항목을 배열+반복으로 묶지 않는다**(plan 비추상화 선언) — 그룹마다 컨트롤 종류와
/// 부수 효과(글꼴 재등록·레지스트리 쓰기·트레이 아이콘)가 달라, 묶으면 채우는 순간 다시 풀어야 한다
fn show_body(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    settings: &mut AppSettings,
    fonts: FontChoices<'_>,
) -> SettingsOutcome {
    let mut body = ui.new_child(egui::UiBuilder::new().max_rect(rect));

    // 모양 — 글꼴
    group_title(&mut body, i18n::settings_group_appearance(), Divider::Skip);
    let font = show_font_group(&mut body, settings, fonts);

    // 시작 — 자동 실행
    group_title(&mut body, i18n::settings_group_startup(), Divider::Draw);
    let startup = show_startup_group(&mut body, settings);

    // 종료 — 트레이 전환
    group_title(&mut body, i18n::settings_group_exit(), Divider::Draw);
    let exit = show_exit_group(&mut body, settings);

    // 파일 보기 — 확장명·숨김·시스템 파일
    group_title(&mut body, i18n::settings_group_files(), Divider::Draw);
    let files = show_file_group(&mut body, settings);

    // 원격 — 목록 자동 갱신 (FR-67)
    group_title(&mut body, i18n::settings_group_remote(), Divider::Draw);
    let remote = show_remote_group(&mut body, settings);

    // 언어 — 앱 문구 전환
    group_title(&mut body, i18n::settings_group_language(), Divider::Draw);
    let language = show_language_group(&mut body, settings);

    // **쓴 자리를 부모에게 알린다** — `new_child`는 부모의 `min_rect`를 넓히지 않아,
    // 이 함수를 감싸는 `ScrollArea`가 내용 높이를 0으로 재고 만다. 그러면 스크롤바도 뜨지 않고
    // 휠도 먹지 않은 채 넘친 부분이 잘리기만 한다 (2026-08-14 리뷰 지적)
    ui.advance_cursor_after_rect(body.min_rect());

    SettingsOutcome {
        changed: font.changed
            || startup.changed
            || exit.changed
            || files.changed
            || remote.changed
            || language.changed,
        font_changed: font.font_changed,
        language_changed: language.language_changed,
        notice: startup.notice,
    }
}

/// `시작` 그룹 — 윈도우 시작 시 자동 실행 (FR-49).
///
/// **화면에 보이는 값은 레지스트리에서 읽는다**(설정 파일이 아니라) — 다른 도구가 그 등록을
/// 지웠을 수 있고, 그때 설정 파일만 믿으면 켜져 있다고 잘못 알린다
fn show_startup_group(ui: &mut egui::Ui, settings: &mut AppSettings) -> SettingsOutcome {
    let mut outcome = SettingsOutcome::default();
    let enabled = autostart::is_enabled();
    if widgets::toggle_row(ui, i18n::settings_auto_start(), enabled) {
        match autostart::set_enabled(!enabled) {
            Ok(()) => {
                // 설정 파일의 값은 사본이다 — 정본(레지스트리)과 맞춰 둔다
                settings.auto_start = !enabled;
                outcome.changed = true;
            }
            // 실패를 조용히 삼키면 토글은 움직였는데 실제로는 안 바뀐 상태가 된다
            Err(_) => outcome.notice = Some(i18n::settings_auto_start_failed()),
        }
    }
    outcome
}

/// `모양` 그룹 — 글꼴 고르기 (FR-48).
///
/// 목록이 아직 준비되지 않았으면(워커가 읽는 중) 드롭다운 대신 안내를 보인다 —
/// 빈 목록을 열 수 있게 두면 고를 것이 없는 상자를 여닫게 된다
fn show_font_group(
    ui: &mut egui::Ui,
    settings: &mut AppSettings,
    fonts: FontChoices<'_>,
) -> SettingsOutcome {
    let mut outcome = SettingsOutcome::default();
    // 표시할 현재 값을 먼저 복사해 둔다 — 아래 클로저가 `settings`를 통째로 빌려야 해서
    // 빌린 문자열을 그 안으로 들고 갈 수 없다
    let current = settings
        .selected_font()
        .unwrap_or(i18n::settings_font_default())
        .to_owned();

    let Some(names) = fonts.names else {
        widgets::form_label(ui, i18n::settings_font(), true);
        ui.painter().text(
            egui::pos2(ui.cursor().left(), ui.cursor().top()),
            egui::Align2::LEFT_TOP,
            i18n::settings_font_scanning(),
            egui::FontId::proportional(widgets::FORM_FONT_PX),
            theme::TEXT_MUTED,
        );
        ui.add_space(widgets::FORM_FIELD_HEIGHT);
        return outcome;
    };

    // 맨 앞에 기본 글꼴을 둔다 — 고른 글꼴을 되돌릴 길이 목록 안에 있어야 한다
    let mut options: Vec<&str> = Vec::with_capacity(names.len() + 1);
    options.push(i18n::settings_font_default());
    options.extend(names.iter().map(String::as_str));

    ui.horizontal(|ui| {
        widgets::form_label(ui, i18n::settings_font(), true);
        if let Some(index) =
            widgets::dropdown_field(ui, "설정 글꼴", &current, FONT_FIELD_WIDTH, true, &options)
        {
            let picked = (index > 0).then(|| options[index].to_owned());
            if picked.as_deref() != settings.selected_font() {
                settings.font_family = picked;
                outcome.changed = true;
                outcome.font_changed = true;
            }
        }
    });
    outcome
}

/// `종료` 그룹 — 닫기를 눌렀을 때 트레이로 보낼지 (FR-50).
///
/// 켜면 트레이 아이콘이 **창이 떠 있는 동안에도** 올라온다 — 닫기를 누르기 전에
/// "이 앱은 트레이로 간다"는 것을 알 수 있어야 한다
fn show_exit_group(ui: &mut egui::Ui, settings: &mut AppSettings) -> SettingsOutcome {
    let mut outcome = SettingsOutcome::default();
    if widgets::toggle_row(ui, i18n::settings_tray_on_close(), settings.tray_on_close) {
        settings.tray_on_close = !settings.tray_on_close;
        outcome.changed = true;
    }
    outcome
}

/// 드롭다운에 실리는 순서 — **번호가 곧 이 배열의 자리**다.
///
/// 항목 이름과 따로 두는 이유: 이름은 언어에 따라 바뀌지만 순서는 고정이다
const LANGUAGE_CHOICES: [LanguageSetting; 3] = [
    LanguageSetting::System,
    LanguageSetting::Korean,
    LanguageSetting::English,
];

/// 지금 언어로 쓴 항목 이름 — `LANGUAGE_CHOICES`와 같은 순서다
fn language_names() -> [&'static str; 3] {
    [
        i18n::settings_language_system(),
        i18n::settings_language_korean(),
        i18n::settings_language_english(),
    ]
}

/// `언어` 그룹 — 앱 문구를 한국어·영어로 바꾼다 (FR-53).
///
/// 항목 이름은 **지금 언어를 따른다** — 영어로 두면 `System default`·`Korean`·`English`가
/// 된다. `English`만 두 언어에서 같은 글자다
fn show_language_group(ui: &mut egui::Ui, settings: &mut AppSettings) -> SettingsOutcome {
    let mut outcome = SettingsOutcome::default();
    let choices = LANGUAGE_CHOICES;
    let names = language_names();
    let current = names[choices
        .iter()
        .position(|choice| *choice == settings.language)
        .unwrap_or(0)];

    ui.horizontal(|ui| {
        widgets::form_label(ui, i18n::settings_language_label(), true);
        if let Some(index) =
            widgets::dropdown_field(ui, "설정 언어", current, LANGUAGE_FIELD_WIDTH, true, &names)
        {
            // 같은 항목을 다시 고르면 아무 일도 하지 않는다 — 저장·다시 그리기를 부를 이유가 없다
            if choices[index] != settings.language {
                settings.language = choices[index];
                outcome.changed = true;
                outcome.language_changed = true;
            }
        }
    });
    outcome
}

/// 갱신 주기 항목 이름 — `REMOTE_REFRESH_CHOICES`와 같은 순서다
fn refresh_names() -> [&'static str; 5] {
    [
        i18n::settings_refresh_10s(),
        i18n::settings_refresh_30s(),
        i18n::settings_refresh_1m(),
        i18n::settings_refresh_5m(),
        i18n::settings_refresh_10m(),
    ]
}

/// `원격` 그룹 — 목록 자동 갱신 on/off와 주기(FR-67), 전송 재시도 횟수(FR-37).
///
/// **끄면 주기 항목이 흐려진다** — 끈 채로 주기를 고르는 것은 뜻이 없다.
/// **재시도 횟수는 언제나 활성**이다 — 자동 갱신과 무관한 값이라 그 토글에 매이지 않는다
fn show_remote_group(ui: &mut egui::Ui, settings: &mut AppSettings) -> SettingsOutcome {
    let mut outcome = SettingsOutcome::default();
    if widgets::toggle_row(
        ui,
        i18n::settings_remote_auto_refresh(),
        settings.remote_auto_refresh,
    ) {
        settings.remote_auto_refresh = !settings.remote_auto_refresh;
        outcome.changed = true;
    }

    let enabled = settings.remote_auto_refresh;
    let choices = REMOTE_REFRESH_CHOICES;
    let names = refresh_names();
    let current = names[choices
        .iter()
        .position(|secs| *secs == remote_refresh_secs(settings.remote_refresh_secs))
        .unwrap_or(0)];
    ui.horizontal(|ui| {
        widgets::form_label(ui, i18n::settings_remote_refresh_label(), enabled);
        if let Some(index) = widgets::dropdown_field(
            ui,
            "설정 원격 갱신 주기",
            current,
            REFRESH_FIELD_WIDTH,
            enabled,
            &names,
        ) && choices[index] != settings.remote_refresh_secs
        {
            settings.remote_refresh_secs = choices[index];
            outcome.changed = true;
        }
    });

    // 전송 재시도 횟수 (FR-37) — 자동 갱신 토글과 무관하므로 언제나 활성이다
    let retry_names = retry_names();
    let current = transfer_retry_count(settings.transfer_retry_count);
    let picked_index = usize::try_from(current.saturating_sub(*TRANSFER_RETRY_RANGE.start()))
        .unwrap_or_default()
        .min(retry_names.len().saturating_sub(1));
    ui.horizontal(|ui| {
        widgets::form_label(ui, i18n::settings_transfer_retry_label(), true);
        if let Some(index) = widgets::dropdown_field(
            ui,
            "설정 전송 재시도 횟수",
            retry_names[picked_index],
            REFRESH_FIELD_WIDTH,
            true,
            &retry_names,
        ) {
            let chosen = *TRANSFER_RETRY_RANGE.start() + index as u32;
            if chosen != settings.transfer_retry_count {
                settings.transfer_retry_count = chosen;
                outcome.changed = true;
            }
        }
    });
    outcome
}

/// 재시도 횟수 선택지 — **숫자만 적는다**.
///
/// `갱신 주기`가 `10초`처럼 단위를 붙이는 것과 달리 횟수에는 단위가 없다.
/// **`i18n` 카탈로그를 거치지 않는 것은 숫자가 언어를 타지 않기 때문**이다 — 화면 문구
/// 규약이 겨냥하는 것은 언어마다 달라지는 낱말이고, 소스 훑기 시험도 한글만 본다.
///
/// 반환형이 배열인 것은 바로 위 `refresh_names`와 같다 — 고정 목록이라 힙에 담을 이유가 없다
fn retry_names() -> [&'static str; 10] {
    ["1", "2", "3", "4", "5", "6", "7", "8", "9", "10"]
}

/// `파일 보기` 그룹의 세 토글 — 값을 그대로 뒤집기만 하면 되는 자리라 여기서 배선한다
/// (`시작`·`모양` 그룹은 레지스트리·글꼴이라는 부수 효과가 있어 각자 따로 있다).
///
/// 그룹 하나를 따로 뗀 이유: 이 부분만 그려 시험할 수 있어야 **앞 그룹들의 줄 수가 바뀔 때**
/// 좌표가 밀려 시험이 엉뚱한 자리를 누르는 일이 없다
fn show_file_group(ui: &mut egui::Ui, settings: &mut AppSettings) -> SettingsOutcome {
    let mut outcome = SettingsOutcome::default();
    if widgets::toggle_row(
        ui,
        i18n::settings_show_extensions(),
        settings.show_extensions,
    ) {
        settings.show_extensions = !settings.show_extensions;
        outcome.changed = true;
    }
    if widgets::toggle_row(ui, i18n::settings_show_hidden(), settings.show_hidden) {
        settings.show_hidden = !settings.show_hidden;
        outcome.changed = true;
    }
    if widgets::toggle_row(ui, i18n::settings_show_system(), settings.show_system) {
        settings.show_system = !settings.show_system;
        outcome.changed = true;
    }
    outcome
}

/// 그룹 제목 위에 구분선을 그을지 — 첫 그룹은 위가 헤더라 긋지 않는다
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Divider {
    Draw,
    Skip,
}

/// 그룹 제목 한 줄 — 앞 그룹과 구분선·여백으로 끊는다
fn group_title(ui: &mut egui::Ui, text: &str, divider: Divider) {
    ui.add_space(GROUP_GAP_TOP);
    if divider == Divider::Draw {
        let y = ui.cursor().top();
        ui.painter().hline(
            ui.max_rect().x_range(),
            y,
            egui::Stroke::new(DIVIDER_THICKNESS, theme::BORDER_CONTROL),
        );
        ui.add_space(DIVIDER_THICKNESS + DIVIDER_GAP);
    }
    ui.painter().text(
        egui::pos2(ui.cursor().left(), ui.cursor().top()),
        egui::Align2::LEFT_TOP,
        text,
        egui::FontId::proportional(GROUP_FONT_PX),
        theme::TEXT_MUTED,
    );
    ui.add_space(GROUP_FONT_PX + GROUP_GAP_BOTTOM);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 목록이 아직 준비되지 않은 상태 — 대부분의 시험은 글꼴 목록과 무관하다
    fn no_fonts() -> FontChoices<'static> {
        FontChoices { names: None }
    }

    #[test]
    fn 닫힌_대화는_아무것도_그리지_않는다() {
        let ctx = egui::Context::default();
        let mut dialog = SettingsDialog::new();
        let mut settings = AppSettings::default();
        assert!(!dialog.is_open());
        let _ = ctx.run_ui(Default::default(), |_ui| {});
        assert_eq!(
            dialog.show(&ctx, &mut settings, no_fonts()),
            SettingsOutcome::default(),
            "닫혀 있는데 결과가 나왔다"
        );
    }

    #[test]
    fn 열고_닫는_상태가_바뀐다() {
        let mut dialog = SettingsDialog::new();
        assert!(!dialog.is_open(), "처음에는 닫혀 있어야 한다");
        dialog.open();
        assert!(dialog.is_open());
        dialog.close();
        assert!(!dialog.is_open());
    }

    #[test]
    fn 그룹_사이에만_구분선을_긋는다() {
        // Acceptance — "구분선과 그룹 제목으로 나뉘어". 첫 그룹 위에는 헤더가 있어 긋지 않는다.
        // 그린 도형을 세어 확인한다: 구분선은 `hline`(=`Shape::LineSegment`)으로 그려진다
        fn line_count(divider: Divider) -> usize {
            let ctx = egui::Context::default();
            let mut lines = 0;
            let output = ctx.run_ui(Default::default(), |ui| {
                group_title(ui, "묶음", divider);
            });
            for shape in output.shapes {
                if matches!(shape.shape, egui::Shape::LineSegment { .. }) {
                    lines += 1;
                }
            }
            lines
        }
        assert_eq!(line_count(Divider::Skip), 0, "첫 그룹 위에 선을 그었다");
        assert_eq!(line_count(Divider::Draw), 1, "그룹 사이 구분선이 없다");
    }

    #[test]
    fn 본문이_바닥_버튼_자리를_넘지_않는다() {
        // 2026-08-14 화면 확인 — 처음 잡은 420×400에서는 `언어` 그룹이 잘리고
        // 바닥의 `닫기`와 겹쳤다. **여섯 그룹이 스크롤 없이** 들어가는지 재 둔다
        // (2026-08-27에 `원격` 그룹이 늘어 `DIALOG_HEIGHT`를 660으로 올렸다)
        let ctx = egui::Context::default();
        let mut settings = AppSettings::default();
        let 자리 = egui::Rect::from_min_size(
            egui::pos2(0.0, 0.0),
            egui::vec2(DIALOG_WIDTH - BODY_PAD_X * 2.0, 10_000.0),
        );
        let output = ctx.run_ui(Default::default(), |ui| {
            show_body(ui, 자리, &mut settings, no_fonts());
        });
        // 그린 것 중 가장 아래가 본문이 실제로 쓴 높이다
        let 쓴_높이 = output
            .shapes
            .iter()
            .map(|clipped| clipped.shape.visual_bounding_rect().max.y)
            .filter(|y| y.is_finite())
            .fold(0.0_f32, f32::max);
        // 셸이 버튼 줄 위로 `BODY_GAP_BOTTOM`을 더 띄우므로 본문이 쓸 수 있는 자리도 그만큼 줄었다
        let 남은_자리 =
            DIALOG_HEIGHT - HEADER_HEIGHT - dialog::FOOTER_HEIGHT - dialog::BODY_GAP_BOTTOM;
        assert!(
            쓴_높이 <= 남은_자리,
            "본문이 {쓴_높이}px를 써 남은 자리 {남은_자리}px를 넘는다 — 바닥 버튼과 겹친다"
        );
    }

    #[test]
    fn 본문이_넘치면_스크롤할_수_있다() {
        // 2026-08-14 리뷰 지적 — `show_body`가 부모의 `min_rect`를 넓히지 않으면
        // `ScrollArea`는 내용 높이를 0으로 재고, 넘친 부분이 스크롤 없이 잘리기만 한다.
        // 위 `본문이_바닥_버튼_자리를_넘지_않는다`는 고정 배치만 재므로 이 결함을 잡지 못한다
        let ctx = egui::Context::default();
        let mut settings = AppSettings::default();
        let mut 내용_높이 = 0.0;
        let _ = ctx.run_ui(Default::default(), |ui| {
            내용_높이 = egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    let inner = ui.max_rect();
                    show_body(ui, inner, &mut settings, no_fonts());
                })
                .content_size
                .y;
        });
        assert!(
            내용_높이 > 0.0,
            "스크롤 영역이 본문 높이를 {내용_높이}px로 재고 있다 — 넘쳐도 스크롤되지 않는다"
        );
    }

    #[test]
    fn 본문은_여섯_그룹을_쌓고_구분선_다섯을_긋는다() {
        // 그룹이 여섯이므로 그 사이 구분선은 다섯이다 — 그룹이 늘거나 줄면 여기서 드러난다
        let ctx = egui::Context::default();
        let mut settings = AppSettings::default();
        let output = ctx.run_ui(Default::default(), |ui| {
            let rect = ui.max_rect();
            show_body(ui, rect, &mut settings, no_fonts());
        });
        let lines = output
            .shapes
            .iter()
            .filter(|shape| matches!(shape.shape, egui::Shape::LineSegment { .. }))
            .count();
        assert_eq!(lines, 5, "여섯 그룹 사이 구분선은 다섯이어야 한다");
    }

    #[test]
    fn 원격_그룹은_세_줄이다() {
        // 자동 갱신 토글 · 갱신 주기 · **전송 재시도 횟수**(FR-37이 더한 줄)
        let _guard =
            crate::i18n::LanguageGuard::lock(crate::app::settings::LanguageSetting::Korean);
        let ctx = egui::Context::default();
        let mut settings = AppSettings::default();
        let output = ctx.run_ui(Default::default(), |ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                show_remote_group(ui, &mut settings);
            });
        });
        let 글 = 그린_글자(&output.shapes);
        for 문구 in ["원격 목록 자동 갱신", "갱신 주기", "전송 재시도 횟수"] {
            assert!(
                글.iter().any(|t| t.contains(문구)),
                "`{문구}` 줄이 없다: {글:?}"
            );
        }
        // 기본값 3이 지금 값으로 보여야 한다
        assert!(
            글.iter().any(|t| t == "3"),
            "고른 값이 보이지 않는다: {글:?}"
        );
    }

    #[test]
    fn 재시도_횟수는_자동_갱신을_꺼도_활성이다() {
        // 자동 갱신과 무관한 값이라 그 토글에 매이지 않는다 — `갱신 주기`와 다른 점이다
        let ctx = egui::Context::default();
        let mut 켬 = AppSettings::default();
        let mut 끔 = AppSettings {
            remote_auto_refresh: false,
            ..AppSettings::default()
        };
        let 글자수 = |settings: &mut AppSettings| {
            let output = ctx.run_ui(Default::default(), |ui| {
                egui::CentralPanel::default().show(ui, |ui| {
                    show_remote_group(ui, settings);
                });
            });
            그린_글자(&output.shapes).len()
        };
        // 줄 수는 토글과 무관하게 같다(흐려질 뿐 사라지지 않는다)
        assert_eq!(글자수(&mut 켬), 글자수(&mut 끔));
    }

    #[test]
    fn 재시도_횟수를_고르면_값이_바뀌고_저장을_알린다() {
        // 즉시 반영이 이 화면의 계약이다 (FR-47) — 고른 그 프레임에 값이 바뀌고 저장 신호가 선다.
        //
        // 드롭다운은 팝업을 거쳐야 해 좌표로 누르기 까다롭다 — 대신 **선택지 번호에서 값으로
        // 가는 셈**을 직접 검증한다(화면 배선은 위 두 시험과 HUMAN-VERIFY가 맡는다)
        let names = retry_names();
        assert_eq!(names.len(), 10, "1~10 열 갈래여야 한다");
        assert_eq!(names.first().copied(), Some("1"));
        assert_eq!(names.last().copied(), Some("10"));

        // 번호 → 값: 첫 항목이 1, 마지막이 10이다
        for (index, name) in names.iter().enumerate() {
            let chosen = *TRANSFER_RETRY_RANGE.start() + index as u32;
            assert_eq!(
                name.parse::<u32>().ok(),
                Some(chosen),
                "선택지와 값이 어긋난다"
            );
        }

        // 값 → 번호: 지금 값이 그 자리에 표시된다(범위 밖 값도 죄어서 찾는다)
        let 번호 = |saved: u32| {
            let current = transfer_retry_count(saved);
            usize::try_from(current.saturating_sub(*TRANSFER_RETRY_RANGE.start()))
                .unwrap_or_default()
                .min(names.len() - 1)
        };
        assert_eq!(names[번호(3)], "3", "기본값이 3으로 보여야 한다");
        assert_eq!(names[번호(10)], "10");
        assert_eq!(names[번호(0)], "1", "0은 1로 죈다");
        assert_eq!(names[번호(99)], "10", "99는 10으로 죈다");
    }

    #[test]
    fn 원격_그룹은_고르기_전까지_아무것도_바꾸지_않는다() {
        // 그리기만 하고 값을 건드리면 설정을 열어 보는 것만으로 저장이 돈다.
        // 팝업 항목까지 눌러 보는 시험은 이 레포의 다른 드롭다운에도 없다
        // (`Popup::menu`는 두 단계 상호작용이라 좌표 흉내가 성립하지 않는다 —
        //  `언어_그룹은_고르기_전까지_아무것도_바꾸지_않는다`와 같은 제약)
        let ctx = egui::Context::default();
        let mut settings = AppSettings::default();
        let mut outcome = SettingsOutcome::default();
        let _ = ctx.run_ui(Default::default(), |ui| {
            outcome = show_remote_group(ui, &mut settings);
        });
        assert!(!outcome.changed, "고르지도 않았는데 바뀌었다고 한다");
        assert_eq!(
            settings.transfer_retry_count,
            crate::app::settings::TRANSFER_RETRY_DEFAULT,
            "그리기만 했는데 재시도 횟수가 바뀌었다"
        );
        assert_eq!(
            settings.remote_refresh_secs,
            crate::app::settings::REMOTE_REFRESH_DEFAULT_SECS,
            "옆 줄의 값까지 흔들렸다"
        );
    }

    #[test]
    fn 범위_밖_값이_담겨_있어도_드롭다운이_그것을_보인다() {
        // Edge Case — 손으로 고친 설정 파일이 0이나 99를 담고 있으면 지금 값을 찾지 못해
        // 첫 항목으로 보인다. 죄어서 찾으므로 그 일이 없어야 한다
        let _guard = i18n::LanguageGuard::lock(LanguageSetting::Korean);
        let ctx = egui::Context::default();
        for (담긴_값, 보일_값) in [(0u32, "1"), (99, "10"), (7, "7")] {
            let mut settings = AppSettings {
                transfer_retry_count: 담긴_값,
                ..AppSettings::default()
            };
            let output = ctx.run_ui(Default::default(), |ui| {
                egui::CentralPanel::default().show(ui, |ui| {
                    show_remote_group(ui, &mut settings);
                });
            });
            let 글 = 그린_글자(&output.shapes);
            assert!(
                글.iter().any(|t| t == 보일_값),
                "{담긴_값}이 담겼는데 `{보일_값}`이 보이지 않는다: {글:?}"
            );
        }
    }

    /// 그린 셰이프에서 글자만 거둔다
    fn 그린_글자(shapes: &[egui::epaint::ClippedShape]) -> Vec<String> {
        fn 모은다(shape: &egui::Shape, out: &mut Vec<String>) {
            match shape {
                egui::Shape::Text(text) => out.push(text.galley.text().to_owned()),
                egui::Shape::Vec(list) => {
                    for inner in list {
                        모은다(inner, out);
                    }
                }
                _ => {}
            }
        }
        let mut out = Vec::new();
        for clipped in shapes {
            모은다(&clipped.shape, &mut out);
        }
        out
    }

    #[test]
    fn 파일_보기_토글을_누르면_값이_뒤집히고_저장을_알린다() {
        // 즉시 반영이 이 화면의 계약이다 — 누른 그 프레임에 값이 바뀌고 저장 신호가 서야 한다
        fn press(pos: egui::Pos2, pressed: bool) -> egui::Event {
            egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: egui::Modifiers::NONE,
            }
        }

        // **그룹 하나만 떼어 그린다** — `show_body` 전체를 그리면 앞 그룹 셋의 높이에
        // 좌표가 딸려, 그 그룹의 줄 수가 바뀔 때마다 이 시험이 엉뚱한 자리를 누르게 된다
        let ctx = egui::Context::default();
        let mut settings = AppSettings::default();
        assert!(settings.show_extensions, "기본값이 바뀌었다");

        let mut outcome = SettingsOutcome::default();
        let _ = ctx.run_ui(Default::default(), |ui| {
            outcome = show_file_group(ui, &mut settings);
        });
        assert!(!outcome.changed, "누르지도 않았는데 바뀌었다고 한다");

        // 그룹의 첫 줄이 `파일 확장명` 토글이다
        let spot = egui::pos2(40.0, widgets::FORM_FIELD_HEIGHT / 2.0);
        for (time, event) in [(0.05, press(spot, true)), (0.10, press(spot, false))] {
            let input = egui::RawInput {
                time: Some(time),
                events: vec![event],
                ..Default::default()
            };
            let _ = ctx.run_ui(input, |ui| {
                outcome = show_file_group(ui, &mut settings);
            });
        }
        assert!(outcome.changed, "토글을 눌렀는데 저장 신호가 서지 않았다");
        assert!(
            !settings.show_extensions,
            "토글을 눌렀는데 값이 뒤집히지 않았다"
        );
        assert!(settings.show_hidden, "누르지 않은 토글까지 바뀌었다");
        assert!(!settings.show_system, "누르지 않은 토글까지 바뀌었다");

        // 셋째 줄이 `시스템 파일 표시` 토글이다 — 줄 사이 간격을 빼고 세면 줄 틈을 눌러
        // 아무 일도 일어나지 않는다(그때 이 시험은 "값이 안 바뀐다"로 실패한다)
        let mut 셋째줄 = egui::pos2(40.0, 0.0);
        let _ = ctx.run_ui(Default::default(), |ui| {
            let 줄높이 = widgets::FORM_FIELD_HEIGHT + ui.spacing().item_spacing.y;
            셋째줄.y = 2.0 * 줄높이 + widgets::FORM_FIELD_HEIGHT / 2.0;
            outcome = show_file_group(ui, &mut settings);
        });
        for (time, event) in [(0.15, press(셋째줄, true)), (0.20, press(셋째줄, false))] {
            let input = egui::RawInput {
                time: Some(time),
                events: vec![event],
                ..Default::default()
            };
            let _ = ctx.run_ui(input, |ui| {
                outcome = show_file_group(ui, &mut settings);
            });
        }
        assert!(settings.show_system, "시스템 토글을 눌렀는데 켜지지 않았다");
        assert!(settings.show_hidden, "숨김 토글까지 함께 바뀌었다");
        assert!(!settings.show_extensions, "확장명 토글까지 함께 바뀌었다");
    }

    #[test]
    fn 파일_보기_토글의_문구가_카탈로그에_있다() {
        // 값은 원문 리터럴로 단언한다 — 카탈로그를 불러 견주면 무엇으로 바뀌어도 통과한다.
        // **두 잠금을 겹치지 않게 스코프로 끊는다** — `LanguageGuard`가 쥐는 것은 재진입을
        // 허용하지 않는 `Mutex`라, 앞 guard가 살아 있는 채로 다시 잠그면 그 자리에서 멎는다
        {
            let _guard = i18n::LanguageGuard::lock(LanguageSetting::Korean);
            assert_eq!(i18n::settings_show_hidden(), "숨김 파일 및 폴더 표시");
            assert_eq!(i18n::settings_show_system(), "시스템 파일 표시");
        }
        {
            let _guard = i18n::LanguageGuard::lock(LanguageSetting::English);
            assert_eq!(
                i18n::settings_show_hidden(),
                "Show hidden files and folders"
            );
            assert_eq!(i18n::settings_show_system(), "Show system files");
        }
    }

    #[test]
    fn 드롭다운_번호가_언어_설정에_그대로_대응한다() {
        // 번호와 설정 값이 어긋나면 `한국어`를 골랐는데 영어가 되는 식으로 조용히 틀어진다
        assert_eq!(LANGUAGE_CHOICES[0], LanguageSetting::System);
        assert_eq!(LANGUAGE_CHOICES[1], LanguageSetting::Korean);
        assert_eq!(LANGUAGE_CHOICES[2], LanguageSetting::English);
        // 이름도 같은 순서여야 한다 — 하나만 어긋나도 화면과 값이 갈린다
        assert_eq!(LANGUAGE_CHOICES.len(), language_names().len());
    }

    #[test]
    fn 언어_그룹은_고르기_전까지_아무것도_바꾸지_않는다() {
        // 그리기만 하고 값을 건드리면 설정을 열어 보는 것만으로 저장이 돈다.
        // 팝업 항목까지 눌러 보는 시험은 이 레포의 다른 드롭다운에도 없다
        // (`Popup::menu`는 두 단계 상호작용이라 좌표 흉내가 성립하지 않는다)
        let _guard = i18n::LanguageGuard::lock(LanguageSetting::Korean);
        let ctx = egui::Context::default();
        let mut settings = AppSettings::default();
        let mut outcome = SettingsOutcome::default();
        let _ = ctx.run_ui(Default::default(), |ui| {
            outcome = show_language_group(ui, &mut settings);
        });
        assert!(!outcome.changed, "고르지도 않았는데 바뀌었다고 한다");
        assert!(!outcome.language_changed, "언어 반영 신호가 먼저 섰다");
        assert_eq!(settings.language, LanguageSetting::System, "값이 바뀌었다");
    }

    #[test]
    fn 항목_이름은_지금_언어를_따른다() {
        // FR-53 — 영어로 두면 드롭다운 자신도 영어로 보여야 한다.
        // 가드가 다른 시험과 겹치지 않게 막고, 떨어질 때 언어를 되돌린다
        let _guard = i18n::LanguageGuard::lock(LanguageSetting::Korean);
        assert_eq!(language_names(), ["시스템 기본", "한국어", "English"]);
        i18n::set_language(LanguageSetting::English);
        assert_eq!(language_names(), ["System default", "Korean", "English"]);
    }

    #[test]
    fn 트레이_토글을_누르면_값이_뒤집힌다() {
        // FR-50 — 이 토글이 없으면 트레이 기능을 켤 방법이 없다
        fn press(pos: egui::Pos2, pressed: bool) -> egui::Event {
            egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: egui::Modifiers::NONE,
            }
        }

        let ctx = egui::Context::default();
        let mut settings = AppSettings::default();
        assert!(!settings.tray_on_close, "기본값이 바뀌었다");

        // 첫 프레임에 토글을 등록해 둔다 — egui는 지난 프레임의 배치로 눌린 곳을 가린다
        let mut outcome = SettingsOutcome::default();
        let _ = ctx.run_ui(Default::default(), |ui| {
            outcome = show_exit_group(ui, &mut settings);
        });
        assert!(!outcome.changed, "누르지도 않았는데 바뀌었다고 한다");

        let spot = egui::pos2(40.0, widgets::FORM_FIELD_HEIGHT / 2.0);
        for (time, event) in [(0.05, press(spot, true)), (0.10, press(spot, false))] {
            let input = egui::RawInput {
                time: Some(time),
                events: vec![event],
                ..Default::default()
            };
            let _ = ctx.run_ui(input, |ui| {
                outcome = show_exit_group(ui, &mut settings);
            });
        }
        assert!(outcome.changed, "토글을 눌렀는데 저장 신호가 서지 않았다");
        assert!(
            settings.tray_on_close,
            "토글을 눌렀는데 값이 뒤집히지 않았다"
        );
    }
}
