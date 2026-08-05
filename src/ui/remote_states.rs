//! 원격 탭의 단계별 화면과 호스트 키 확인 대화 (FR-30·FR-32·D15).
//!
//! 원격 탭은 연결 상태에 따라 **본문이 통째로 달라진다** — 아직 연결하지 않았으면 안내 문구,
//! 연결 중이면 자리 표시 막대, 실패했으면 사유와 조치 버튼, 연결됐으면 보통의 파일 목록이다.
//!
//! 넷을 하나의 상태 머신 컴포넌트로 묶지 않는다 — 레이아웃이 전혀 달라 묶으면 분기가 함수 안으로
//! 옮겨질 뿐이고, 어느 화면을 고치는지 한눈에 보이지 않게 된다 (plan T10 비추상화 선언).
//!
//! **조작은 값으로 돌려주고 여기서 실행하지 않는다** — 기존 패널 규약과 같다.
use std::sync::mpsc::{Receiver, Sender, channel};

use eframe::egui;

use crate::panel::tabs::TabPhase;
use crate::remote::hostkey::{HostKeyCheck, HostKeyDecision};
use crate::remote::sftp::HostKeyPrompt;
use crate::remote::types::Protocol;
use crate::ui::theme;

// ── 시각 토큰 (plan `## 시각 요소 분해` 1:1, 96DPI 기준 고정 px) ──
/// 배지 높이 (README §4)
const BADGE_HEIGHT: f32 = 15.0;
/// 배지 좌우 여백
const BADGE_PAD_X: f32 = 5.0;
/// 배지 글자 크기
const BADGE_TEXT_PX: f32 = 11.0;
/// 배지와 그 앞 이름 사이 간격
const BADGE_GAP: f32 = 4.0;
/// 배지 안 상태 점 지름
const BADGE_DOT: f32 = 5.0;

/// 연결 중 자리 표시 막대 수 (README §4)
const SKELETON_BARS: usize = 8;
/// 막대 하나의 높이
const SKELETON_BAR_HEIGHT: f32 = 12.0;
/// 막대 사이 간격
const SKELETON_GAP: f32 = 6.0;
/// 막대 색
const SKELETON_FILL: egui::Color32 = egui::Color32::from_rgb(0x26, 0x26, 0x26);
/// 막대 묶음 위 여백
const SKELETON_TOP: f32 = 14.0;

/// 실패 화면의 아이콘 원 지름 (HTML:245)
const FAIL_ICON_SIZE: f32 = 34.0;
/// 실패 화면 요소 사이 간격
const FAIL_GAP: f32 = 14.0;
/// 실패 화면 좌우 여백
const FAIL_PAD_X: f32 = 28.0;
/// 실패 화면 버튼 높이·좌우 여백 (HTML:249)
const FAIL_BUTTON_HEIGHT: f32 = 28.0;
const FAIL_BUTTON_PAD_X: f32 = 16.0;
/// 사유 문구가 길 때 보일 최대 줄 수 — 그보다 길면 말줄임한다
const FAIL_REASON_MAX_ROWS: usize = 3;

/// 연결 중 취소 버튼 높이·좌우 여백 (HTML:228)
const CANCEL_BUTTON_HEIGHT: f32 = 22.0;
const CANCEL_PAD_X: f32 = 10.0;

/// 미연결 원격 패널의 항목 수 표기 (인벤토리 #95)
pub const UNKNOWN_COUNT: &str = "—";

// ── 화면 문구 (인벤토리 원문 그대로 — 여기서 다듬으면 화면과 명세가 갈린다) ──
/// 미연결 탭 안내 첫 줄 (인벤토리 #14). `sftp://호스트`만 다른 색이라 셋으로 나눠 든다
const EMPTY_HINT_HEAD: &str = "주소창에 ";
const EMPTY_HINT_SCHEME: &str = "sftp://호스트";
const EMPTY_HINT_TAIL: &str = " 를 입력해 연결하세요";
/// 미연결 탭 안내 둘째 줄 (인벤토리 #15)
const EMPTY_HINT_DRAG: &str = "사이드바의 사이트를 이 탭으로 끌어다 놓아도 됩니다";
/// 실패 화면 제목 (인벤토리 #16)
const FAIL_TITLE: &str = "연결하지 못했습니다";
/// 실패 화면 버튼·링크 (인벤토리 #18~20)
const FAIL_RETRY: &str = "재시도";
const FAIL_SETTINGS: &str = "설정 열기";
const FAIL_VIEW_LOG: &str = "서버 로그 보기";
/// 연결 중 취소 (인벤토리 #21)
const CANCEL_LABEL: &str = "취소";

/// 서버가 사유를 주지 않았을 때 보일 문구
const FAIL_REASON_FALLBACK: &str = "서버가 응답하지 않았습니다.";
/// 실패 사유 뒤에 늘 붙는 안내 (인벤토리 #17)
const FAIL_REASON_HINT: &str = "암호화 설정이 서버와 다를 수도 있습니다.";

/// 실패 화면에서 사용자가 고른 것
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailedAction {
    Retry,
    OpenSettings,
    ViewLog,
}

/// 탭 배지에 보일 문구 (인벤토리 #11~13).
///
/// 연결됐을 때는 **프로토콜 이름 그대로**다 — 사용자가 이 탭이 어느 방식으로 붙어 있는지를
/// 한눈에 알아야 한다(같은 서버에 ftp와 sftp로 각각 붙을 수 있다).
pub fn badge_label(phase: &TabPhase, protocol: Protocol) -> &'static str {
    match phase {
        TabPhase::Ok => protocol.label(),
        TabPhase::Connecting => "연결 중…",
        // 실패한 탭도 "연결이 없는" 상태다 — 사유는 본문이 보인다
        TabPhase::New | TabPhase::Error { .. } => "연결 없음",
    }
}

/// 배지의 점·글자·테두리·채움 색.
///
/// **점과 글자를 따로 두는 이유**: 디자인은 연결됨 상태에서 점을 `#4ADE80`, 글자를 `#7FD6A2`로
/// 나눈다(README `### Colors`) — 하나로 합치면 점이 흐려져 상태가 눈에 덜 띈다
fn badge_colors(phase: &TabPhase) -> (egui::Color32, egui::Color32, egui::Color32, egui::Color32) {
    match phase {
        TabPhase::Ok => (
            theme::OK_DOT,
            theme::OK_TEXT,
            theme::OK_BORDER,
            theme::OK_FILL,
        ),
        // 연결 중은 점과 글자가 같은 색이다 (README `### Colors`)
        TabPhase::Connecting => (
            theme::WARN,
            theme::WARN,
            theme::WARN_BORDER,
            theme::WARN_FILL,
        ),
        TabPhase::Error { .. } => (
            theme::ERROR,
            theme::ERROR_TEXT,
            theme::ERROR_BORDER,
            theme::ERROR_FILL,
        ),
        TabPhase::New => (
            theme::TEXT_MUTED,
            theme::TEXT_MUTED,
            theme::BORDER_CONTROL,
            theme::HEADER_BG,
        ),
    }
}

/// 탭 아이콘과 이름 사이에 놓이는 배지 — 차지할 폭을 돌려준다 (원본 `:99`의 배치).
///
/// **이름을 밀어내지 않는다** — 탭이 좁아지면 이름이 먼저 줄고 배지는 제 폭을 지킨다
/// (plan Edge Case). 그래서 호출부는 이 폭을 먼저 떼어 두고 남은 자리에 이름을 그린다.
pub fn badge_width(ui: &egui::Ui, phase: &TabPhase, protocol: Protocol) -> f32 {
    let label = badge_label(phase, protocol);
    let font = egui::FontId::proportional(BADGE_TEXT_PX);
    let text_width = ui
        .painter()
        .layout_no_wrap(label.to_owned(), font, theme::TEXT)
        .size()
        .x;
    BADGE_PAD_X * 2.0 + BADGE_DOT + BADGE_GAP + text_width
}

/// 배지를 그린다 (인벤토리 #11~13)
pub fn show_badge(ui: &egui::Ui, rect: egui::Rect, phase: &TabPhase, protocol: Protocol) {
    let (dot, text_color, border, fill) = badge_colors(phase);
    let painter = ui.painter();
    let badge = egui::Rect::from_center_size(
        rect.center(),
        egui::vec2(rect.width(), BADGE_HEIGHT.min(rect.height())),
    );
    // 모서리 반경 0 — 디자인 전역 규칙이다
    painter.rect_filled(badge, 0.0, fill);
    painter.rect_stroke(
        badge,
        0.0,
        egui::Stroke::new(1.0, border),
        egui::StrokeKind::Inside,
    );

    let dot_center = egui::pos2(
        badge.left() + BADGE_PAD_X + BADGE_DOT / 2.0,
        badge.center().y,
    );
    painter.circle_filled(dot_center, BADGE_DOT / 2.0, dot);
    painter.text(
        egui::pos2(dot_center.x + BADGE_DOT / 2.0 + BADGE_GAP, badge.center().y),
        egui::Align2::LEFT_CENTER,
        badge_label(phase, protocol),
        egui::FontId::proportional(BADGE_TEXT_PX),
        text_color,
    );
}

/// 연결 중 자리 표시 — 막대 8개 (README §4).
///
/// 진짜 목록이 오기 전까지 **자리만 잡아 둔다** — 빈 화면을 보이면 멈춘 것처럼 보이고,
/// 회전 표시만 두면 곧 무엇이 올지 알 수 없다
pub fn show_skeleton(ui: &mut egui::Ui) {
    let width = ui.available_width();
    ui.add_space(SKELETON_TOP);
    for index in 0..SKELETON_BARS {
        // 폭을 조금씩 달리해 진짜 목록처럼 보이게 한다
        let ratio = 0.55 + ((index % 4) as f32) * 0.12;
        let (rect, _) =
            ui.allocate_exact_size(egui::vec2(width, SKELETON_BAR_HEIGHT), egui::Sense::hover());
        let bar = egui::Rect::from_min_size(
            rect.min,
            egui::vec2(rect.width() * ratio, SKELETON_BAR_HEIGHT),
        );
        ui.painter().rect_filled(bar, 0.0, SKELETON_FILL);
        if index + 1 < SKELETON_BARS {
            ui.add_space(SKELETON_GAP);
        }
    }
}

/// 디자인의 보조 버튼 — 채움 `#252525` · 테두리 `#3A3A3A` · hover `#2E2E2E` · 모서리 반경 0.
///
/// egui 기본 버튼 색(`#2A2A2A`·기본 테두리·hover `#383838`)과 다르므로 스타일을 **국소로**
/// 덮는다. 전역 팔레트를 바꾸면 기존 화면의 버튼까지 함께 바뀐다.
/// 글자색은 자리마다 달라(취소 `#C8C8C8` · 실패 화면 `#D8D8D8`) 인자로 받는다
/// 폭은 **글자에 맞춘다** — 원본이 폭 대신 좌우 여백(`padding 0 Npx`)으로 정하기 때문이다
fn design_button(
    ui: &mut egui::Ui,
    label: &str,
    text_color: egui::Color32,
    pad_x: f32,
    height: f32,
) -> egui::Response {
    ui.scope(|ui| {
        ui.spacing_mut().button_padding = egui::vec2(pad_x, 0.0);
        let widgets = &mut ui.style_mut().visuals.widgets;
        for (state, fill) in [
            (&mut widgets.inactive, theme::HEADER_BG),
            (&mut widgets.hovered, theme::ROW_HOT),
            (&mut widgets.active, theme::ROW_HOT),
        ] {
            state.weak_bg_fill = fill;
            state.bg_fill = fill;
            state.bg_stroke = egui::Stroke::new(1.0, theme::BORDER_CONTROL);
            state.corner_radius = egui::CornerRadius::ZERO;
            // 눌렸을 때 커지지 않는다 — 디자인은 상태에 따라 크기가 변하지 않는다
            state.expansion = 0.0;
        }
        ui.add(
            egui::Button::new(egui::RichText::new(label).color(text_color))
                .min_size(egui::vec2(0.0, height)),
        )
    })
    .inner
}

/// `design_button`이 차지할 폭 — 가운데 정렬처럼 **그리기 전에** 폭을 알아야 하는 자리가 쓴다
fn design_button_width(ui: &egui::Ui, label: &str, pad_x: f32) -> f32 {
    let font = egui::TextStyle::Button.resolve(ui.style());
    ui.painter()
        .layout_no_wrap(label.to_owned(), font, theme::TEXT)
        .size()
        .x
        + pad_x * 2.0
}

/// 연결 중 취소 버튼 — 눌렸으면 `true` (인벤토리 #21)
pub fn show_cancel(ui: &mut egui::Ui) -> bool {
    design_button(
        ui,
        CANCEL_LABEL,
        theme::HEADER_TEXT,
        CANCEL_PAD_X,
        CANCEL_BUTTON_HEIGHT,
    )
    .clicked()
}

/// 아직 어디에도 연결하지 않은 탭의 안내 (인벤토리 #14·#15).
///
/// 문구는 디자인 원문 그대로다 — 여기서 다듬으면 화면과 명세가 갈린다
pub fn show_empty(ui: &mut egui::Ui) {
    ui.vertical_centered(|ui| {
        ui.add_space(FAIL_GAP * 2.0);
        // `sftp://` 부분만 초록으로 — 사용자가 무엇을 적어야 하는지 눈에 들어오게
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            ui.label(egui::RichText::new(EMPTY_HINT_HEAD).color(theme::TEXT_MUTED));
            ui.label(egui::RichText::new(EMPTY_HINT_SCHEME).color(theme::OK_TEXT));
            ui.label(egui::RichText::new(EMPTY_HINT_TAIL).color(theme::TEXT_MUTED));
        });
        ui.add_space(6.0);
        ui.label(egui::RichText::new(EMPTY_HINT_DRAG).color(theme::TEXT_DIM));
    });
}

/// 실패 화면의 사유 문구 — 서버가 준 것에 안내를 덧붙인다 (인벤토리 #17).
///
/// 서버가 아무 말도 하지 않았으면 빈 줄만 남으므로 일반 문구로 메운다 (plan Edge Case)
pub fn failure_reason(detail: &str) -> String {
    let detail = detail.trim();
    if detail.is_empty() {
        format!("{FAIL_REASON_FALLBACK} {FAIL_REASON_HINT}")
    } else {
        format!("{detail} {FAIL_REASON_HINT}")
    }
}

/// 연결 실패 화면 (인벤토리 #16~20). 사용자가 고른 조치를 돌려준다
pub fn show_failed(ui: &mut egui::Ui, detail: &str) -> Option<FailedAction> {
    let mut action = None;
    ui.vertical_centered(|ui| {
        ui.add_space(FAIL_GAP * 2.0);

        // 느낌표 원 — 글리프가 아니라 직접 그린다(글꼴에 없는 모양에 기대지 않는다)
        let (rect, _) = ui.allocate_exact_size(
            egui::vec2(FAIL_ICON_SIZE, FAIL_ICON_SIZE),
            egui::Sense::hover(),
        );
        let painter = ui.painter();
        painter.circle_stroke(
            rect.center(),
            FAIL_ICON_SIZE / 2.0,
            egui::Stroke::new(2.0, theme::ERROR),
        );
        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            "!",
            egui::FontId::proportional(19.0),
            theme::ERROR,
        );

        ui.add_space(FAIL_GAP);
        ui.label(
            egui::RichText::new(FAIL_TITLE)
                .size(14.0)
                .color(theme::TEXT),
        );

        ui.add_space(FAIL_GAP / 2.0);
        let available = (ui.available_width() - FAIL_PAD_X * 2.0).max(0.0);
        let reason = ui.painter().layout(
            failure_reason(detail),
            egui::FontId::proportional(13.0),
            theme::TEXT_MUTED,
            available,
        );
        // 아주 긴 사유는 세 줄까지만 보인다 — 화면을 사유가 통째로 덮으면 조치 버튼이 밀린다
        let rows = reason.rows.len().min(FAIL_REASON_MAX_ROWS);
        let height = reason.rows[..rows].iter().map(|row| row.height()).sum();
        let (rect, _) = ui.allocate_exact_size(egui::vec2(available, height), egui::Sense::hover());
        ui.painter()
            .with_clip_rect(rect)
            .galley(rect.min, reason, theme::TEXT_MUTED);

        ui.add_space(FAIL_GAP);
        ui.horizontal(|ui| {
            // 가운데 정렬 — `vertical_centered` 안이라도 가로 묶음은 스스로 맞춰야 한다.
            // 버튼 폭이 글자에 맞춰지므로 미리 재어 둔다
            let buttons: f32 = [FAIL_RETRY, FAIL_SETTINGS]
                .iter()
                .map(|label| design_button_width(ui, label, FAIL_BUTTON_PAD_X))
                .sum::<f32>()
                + ui.spacing().item_spacing.x;
            ui.add_space(((ui.available_width() - buttons) / 2.0).max(0.0));
            if design_button(
                ui,
                FAIL_RETRY,
                theme::TEXT_BUTTON,
                FAIL_BUTTON_PAD_X,
                FAIL_BUTTON_HEIGHT,
            )
            .clicked()
            {
                action = Some(FailedAction::Retry);
            }
            if design_button(
                ui,
                FAIL_SETTINGS,
                theme::TEXT_BUTTON,
                FAIL_BUTTON_PAD_X,
                FAIL_BUTTON_HEIGHT,
            )
            .clicked()
            {
                action = Some(FailedAction::OpenSettings);
            }
        });

        ui.add_space(6.0);
        if ui
            .add(
                egui::Label::new(
                    egui::RichText::new(FAIL_VIEW_LOG)
                        .size(12.0)
                        .color(theme::TEXT_DIM),
                )
                .sense(egui::Sense::click()),
            )
            .clicked()
        {
            action = Some(FailedAction::ViewLog);
        }
    });
    action
}

/// 호스트 키 확인 대화 (D15·인벤토리 #96).
///
/// **문구는 디자인에 없어 이 계획이 새로 정했다** — 서버 로그의 `호스트 키를 확인했습니다
/// (SHA256:…)` 줄에서 표기를 따왔다.
///
/// 사용자가 고르기 전에는 `None`이다 — 그동안 SFTP 연결은 진행되지 않는다.
pub fn show_hostkey_dialog(
    ctx: &egui::Context,
    host: &str,
    check: &HostKeyCheck,
) -> Option<HostKeyDecision> {
    let (title, fingerprint, warning) = match check {
        // 물어볼 것이 없다 — 호출부가 이 경우엔 대화를 띄우지 않는다
        HostKeyCheck::Match => return Some(HostKeyDecision::Accept),
        HostKeyCheck::Unknown { fingerprint } => {
            ("이 서버를 처음 연결합니다", fingerprint.as_str(), None)
        }
        HostKeyCheck::Changed { old, new } => (
            "서버 지문이 전과 다릅니다",
            new.as_str(),
            Some(format!(
                "전에 저장한 지문은 {old} 였습니다. 서버를 다시 설치했거나, \
                 중간에 다른 서버가 끼어든 것일 수 있습니다."
            )),
        ),
    };

    let mut decision = None;
    egui::Modal::new(egui::Id::new("원격 호스트 키 확인")).show(ctx, |ui| {
        ui.set_width(460.0);
        ui.label(egui::RichText::new(title).size(16.0).color(theme::TEXT));
        ui.add_space(10.0);
        ui.label(egui::RichText::new(host).color(theme::HEADER_TEXT));
        ui.add_space(6.0);
        // 지문은 사용자가 서버에서 뽑은 값과 눈으로 대조한다 — 고정폭으로 보여야 자릿수가 맞는다
        ui.label(
            egui::RichText::new(fingerprint)
                .monospace()
                .color(theme::OK_TEXT),
        );
        if let Some(warning) = &warning {
            ui.add_space(10.0);
            ui.label(egui::RichText::new(warning).color(theme::ERROR_TEXT));
        }

        ui.add_space(14.0);
        ui.horizontal(|ui| {
            if ui.button("수락하고 연결").clicked() {
                decision = Some(HostKeyDecision::Accept);
            }
            if ui.button("취소").clicked() {
                decision = Some(HostKeyDecision::Reject);
            }
        });
    });
    decision
}

/// 워커가 올린 지문 확인 요청 — 화면이 대화를 띄우고 답을 돌려준다
struct HostKeyRequest {
    /// 어느 서버인가 (`호스트:포트`)
    host: String,
    check: HostKeyCheck,
    /// 결정을 돌려보낼 통로. **이것이 버려지면 워커는 거절로 읽는다**
    reply: Sender<HostKeyDecision>,
}

/// 연결 워커와 확인 대화를 잇는 통로 (D15).
///
/// `remote`는 화면을 모르고, 화면은 워커 스레드에서 그릴 수 없다 — 그래서 워커가 요청을 채널로
/// 올리고 **그 자리에서 답을 기다린다**. 기다리는 것은 그 연결의 워커 하나뿐이라 다른 연결과
/// 로컬 탐색은 그대로 돈다 (NFR-11).
///
/// **자동 수락 경로가 없다**: 통로가 끊기거나(앱 종료) 화면이 사라지면 거절로 떨어진다.
pub struct HostKeyGate {
    tx: Sender<HostKeyRequest>,
    rx: Receiver<HostKeyRequest>,
    /// 지금 대화로 떠 있는 요청 — 사용자가 고를 때까지 여기 머문다
    pending: Option<HostKeyRequest>,
}

impl Default for HostKeyGate {
    fn default() -> HostKeyGate {
        HostKeyGate::new()
    }
}

impl HostKeyGate {
    pub fn new() -> HostKeyGate {
        let (tx, rx) = channel();
        HostKeyGate {
            tx,
            rx,
            pending: None,
        }
    }

    /// SFTP 세션에 넘길 확인 통로를 만든다. `host`는 대화에 그대로 보인다
    pub fn prompt(&self, host: String) -> HostKeyPrompt {
        let tx = self.tx.clone();
        Box::new(move |check: &HostKeyCheck| {
            let (reply, answer) = channel();
            let request = HostKeyRequest {
                host: host.clone(),
                check: check.clone(),
                reply,
            };
            if tx.send(request).is_err() {
                // 물을 곳이 없으면 거절이다 — 조용히 수락하는 경로를 두지 않는다 (D15)
                return HostKeyDecision::Reject;
            }
            // 사용자가 고를 때까지 이 연결의 워커만 기다린다.
            // 앱이 닫히면 회신 통로가 끊겨 거절이 된다 (plan Edge Case: 대화 중 앱 종료 → 연결 취소)
            answer.recv().unwrap_or(HostKeyDecision::Reject)
        })
    }

    /// 확인 대화가 떠 있는가 — 모달이 뜬 동안 단축키를 받지 않기 위해 호출부가 본다
    pub fn is_open(&self) -> bool {
        self.pending.is_some()
    }

    /// 대기 중인 요청이 있으면 대화를 띄우고, 사용자가 고르면 워커에 돌려준다.
    ///
    /// 매 프레임 호출한다. 요청이 없으면 아무것도 그리지 않는다
    pub fn show(&mut self, ctx: &egui::Context) -> Option<HostKeyDecision> {
        self.resolve(|host, check| show_hostkey_dialog(ctx, host, check))
    }

    /// 요청 수신 → 사용자 결정 → 워커 회신의 순서만 담는다.
    ///
    /// 묻는 방법을 인자로 받는 이유: 대화 그리기는 화면이 있어야 하지만 **이 순서 자체는
    /// 서버도 화면도 없이 검증돼야 한다** — 회신이 끊기면 워커가 영영 기다린다
    fn resolve(
        &mut self,
        ask: impl FnOnce(&str, &HostKeyCheck) -> Option<HostKeyDecision>,
    ) -> Option<HostKeyDecision> {
        if self.pending.is_none() {
            self.pending = self.rx.try_recv().ok();
        }
        let request = self.pending.as_ref()?;
        let decision = ask(&request.host, &request.check)?;
        let request = self.pending.take()?;
        // 워커가 그 사이 사라졌으면 보낼 곳이 없다 — 조용히 넘어간다
        let _ = request.reply.send(decision);
        Some(decision)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 배지_문구는_단계별로_정해져_있다() {
        // 인벤토리 #11~13 — 문구가 바뀌면 화면과 명세가 갈린다
        assert_eq!(badge_label(&TabPhase::Ok, Protocol::Sftp), "sftp");
        assert_eq!(badge_label(&TabPhase::Ok, Protocol::Ftps), "ftps");
        assert_eq!(badge_label(&TabPhase::Ok, Protocol::Ftp), "ftp");
        assert_eq!(
            badge_label(&TabPhase::Connecting, Protocol::Sftp),
            "연결 중…"
        );
        assert_eq!(badge_label(&TabPhase::New, Protocol::Sftp), "연결 없음");
        // 실패한 탭도 연결이 없는 상태다 — 사유는 본문이 보인다
        assert_eq!(
            badge_label(
                &TabPhase::Error {
                    message: "530".to_owned()
                },
                Protocol::Sftp
            ),
            "연결 없음"
        );
    }

    #[test]
    fn 배지_색은_단계마다_다르다() {
        let ok = badge_colors(&TabPhase::Ok);
        let connecting = badge_colors(&TabPhase::Connecting);
        let new = badge_colors(&TabPhase::New);
        // (점, 글자, 테두리, 채움) — 연결됨만 점과 글자가 다른 색이다 (README `### Colors`)
        assert_eq!((ok.0, ok.1), (theme::OK_DOT, theme::OK_TEXT));
        assert_ne!(ok.0, ok.1, "연결됨 배지의 점이 글자색으로 흐려졌다");
        assert_eq!((connecting.0, connecting.1), (theme::WARN, theme::WARN));
        assert_eq!(new.1, theme::TEXT_MUTED);
        assert_ne!(ok, connecting);
        assert_ne!(ok, new);
    }

    #[test]
    fn 미연결_안내_문구는_원문_그대로다() {
        // 인벤토리 #14·#15 — 색을 나누느라 셋으로 쪼갠 첫 줄도 이어 붙이면 원문과 같아야 한다
        let first = format!("{EMPTY_HINT_HEAD}{EMPTY_HINT_SCHEME}{EMPTY_HINT_TAIL}");
        assert_eq!(first, "주소창에 sftp://호스트 를 입력해 연결하세요");
        assert_eq!(
            EMPTY_HINT_DRAG,
            "사이드바의 사이트를 이 탭으로 끌어다 놓아도 됩니다"
        );
    }

    #[test]
    fn 실패_화면과_취소_문구는_원문_그대로다() {
        // 인벤토리 #16~21 — 다듬으면 화면과 명세가 갈린다
        assert_eq!(FAIL_TITLE, "연결하지 못했습니다");
        assert_eq!(FAIL_REASON_HINT, "암호화 설정이 서버와 다를 수도 있습니다.");
        assert_eq!(FAIL_RETRY, "재시도");
        assert_eq!(FAIL_SETTINGS, "설정 열기");
        assert_eq!(FAIL_VIEW_LOG, "서버 로그 보기");
        assert_eq!(CANCEL_LABEL, "취소");
    }

    #[test]
    fn 실패_사유가_비면_일반_문구로_메운다() {
        // 서버가 아무 말도 하지 않으면 빈 줄만 남는다 (plan Edge Case)
        let empty = failure_reason("   ");
        assert!(empty.starts_with(FAIL_REASON_FALLBACK), "{empty}");
        assert!(empty.ends_with(FAIL_REASON_HINT));

        // 사유가 있으면 그대로 두고 안내만 덧붙인다
        let given = failure_reason("530 Login incorrect");
        assert!(given.starts_with("530 Login incorrect"));
        assert!(given.ends_with(FAIL_REASON_HINT));
    }

    #[test]
    fn 미연결_항목수는_줄표다() {
        // 인벤토리 #95 — 연결되지 않은 원격 패널은 개수를 모른다
        assert_eq!(UNKNOWN_COUNT, "—");
    }

    fn 미등록() -> HostKeyCheck {
        HostKeyCheck::Unknown {
            fingerprint: "SHA256:47DEQpj8HBSa+/TImW+5JCeuQeRkm5NMpJWZG3hSuFU".to_owned(),
        }
    }

    #[test]
    fn 물을_곳이_없으면_거절한다() {
        // 화면이 사라진 뒤에도 워커는 답을 얻어야 한다 — 자동 수락으로 새면 D15가 무너진다
        let gate = HostKeyGate::new();
        let mut prompt = gate.prompt("example.test:22".to_owned());
        drop(gate);
        assert_eq!(prompt(&미등록()), HostKeyDecision::Reject);
    }

    #[test]
    fn 화면이_고른_결정이_워커에게_돌아간다() {
        // 워커는 답이 올 때까지 그 자리에서 기다린다 — 회신이 끊기면 연결이 영영 멈춘다
        let mut gate = HostKeyGate::new();
        let mut prompt = gate.prompt("example.test:22".to_owned());
        let worker = std::thread::spawn(move || prompt(&미등록()));

        // 요청이 올라오기 전에는 물을 것이 없다
        assert_eq!(gate.resolve(|_, _| Some(HostKeyDecision::Accept)), None);
        assert!(!gate.is_open());

        // 요청이 도착하면 대화를 띄우고(여기서는 "고르는 중"), 고르기 전에는 회신하지 않는다
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
        while !gate.is_open() && std::time::Instant::now() < deadline {
            assert_eq!(gate.resolve(|_, _| None), None);
        }
        assert!(gate.is_open(), "워커가 올린 요청을 받지 못했다");

        let answered = gate.resolve(|host, check| {
            assert_eq!(host, "example.test:22");
            assert_eq!(check, &미등록());
            Some(HostKeyDecision::Accept)
        });
        assert_eq!(answered, Some(HostKeyDecision::Accept));
        assert!(!gate.is_open(), "답한 요청이 남았다");
        assert_eq!(worker.join().expect("워커 종료"), HostKeyDecision::Accept);
    }

    #[test]
    fn 자리표시_막대는_여덟_개다() {
        // README §4의 수치를 상수로 고정한다 — 그리기 코드에서 숫자를 바꾸면 이 테스트가 잡는다
        assert_eq!(SKELETON_BARS, 8);
        assert_eq!(SKELETON_BAR_HEIGHT, 12.0);
        assert_eq!(SKELETON_GAP, 6.0);
    }
}
