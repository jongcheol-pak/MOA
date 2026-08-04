//! 원격 탭의 단계별 화면과 호스트 키 확인 대화 (FR-30·FR-32·D15).
//!
//! 원격 탭은 연결 상태에 따라 **본문이 통째로 달라진다** — 아직 연결하지 않았으면 안내 문구,
//! 연결 중이면 자리 표시 막대, 실패했으면 사유와 조치 버튼, 연결됐으면 보통의 파일 목록이다.
//!
//! 넷을 하나의 상태 머신 컴포넌트로 묶지 않는다 — 레이아웃이 전혀 달라 묶으면 분기가 함수 안으로
//! 옮겨질 뿐이고, 어느 화면을 고치는지 한눈에 보이지 않게 된다 (plan T10 비추상화 선언).
//!
//! **조작은 값으로 돌려주고 여기서 실행하지 않는다** — 기존 패널 규약과 같다.
use eframe::egui;

use crate::panel::tabs::TabPhase;
use crate::remote::hostkey::{HostKeyCheck, HostKeyDecision};
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
/// 실패 화면 버튼 높이
const FAIL_BUTTON_HEIGHT: f32 = 28.0;
/// 사유 문구가 길 때 보일 최대 줄 수 — 그보다 길면 말줄임한다
const FAIL_REASON_MAX_ROWS: usize = 3;

/// 연결 중 취소 버튼 높이 (HTML:228)
const CANCEL_BUTTON_HEIGHT: f32 = 22.0;

/// 미연결 원격 패널의 항목 수 표기 (인벤토리 #95)
pub const UNKNOWN_COUNT: &str = "—";

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

/// 배지의 글자·테두리·채움 색
fn badge_colors(phase: &TabPhase) -> (egui::Color32, egui::Color32, egui::Color32) {
    match phase {
        TabPhase::Ok => (theme::OK_TEXT, theme::OK_BORDER, theme::OK_FILL),
        TabPhase::Connecting => (theme::WARN, theme::WARN_BORDER, theme::WARN_FILL),
        TabPhase::Error { .. } => (theme::ERROR_TEXT, theme::ERROR_BORDER, theme::ERROR_FILL),
        TabPhase::New => (theme::TEXT_MUTED, theme::BORDER_CONTROL, theme::HEADER_BG),
    }
}

/// 탭 이름 오른쪽에 붙는 배지 — 차지할 폭을 돌려준다.
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
    let (text_color, border, fill) = badge_colors(phase);
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
    painter.circle_filled(dot_center, BADGE_DOT / 2.0, text_color);
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

/// 연결 중 취소 버튼 — 눌렸으면 `true` (인벤토리 #21)
pub fn show_cancel(ui: &mut egui::Ui) -> bool {
    ui.add_sized(
        egui::vec2(60.0, CANCEL_BUTTON_HEIGHT),
        egui::Button::new(egui::RichText::new("취소").color(theme::TEXT_BUTTON)),
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
            ui.label(egui::RichText::new("주소창에 ").color(theme::TEXT_MUTED));
            ui.label(egui::RichText::new("sftp://호스트").color(theme::OK_TEXT));
            ui.label(egui::RichText::new(" 를 입력해 연결하세요").color(theme::TEXT_MUTED));
        });
        ui.add_space(6.0);
        ui.label(
            egui::RichText::new("사이드바의 사이트를 이 탭으로 끌어다 놓아도 됩니다")
                .color(theme::TEXT_DIM),
        );
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
            egui::RichText::new("연결하지 못했습니다")
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
            // 가운데 정렬 — `vertical_centered` 안이라도 가로 묶음은 스스로 맞춰야 한다
            let buttons = 2.0 * 96.0 + ui.spacing().item_spacing.x;
            ui.add_space(((ui.available_width() - buttons) / 2.0).max(0.0));
            if ui
                .add_sized(
                    egui::vec2(96.0, FAIL_BUTTON_HEIGHT),
                    egui::Button::new(egui::RichText::new("재시도").color(theme::TEXT_BUTTON)),
                )
                .clicked()
            {
                action = Some(FailedAction::Retry);
            }
            if ui
                .add_sized(
                    egui::vec2(96.0, FAIL_BUTTON_HEIGHT),
                    egui::Button::new(egui::RichText::new("설정 열기").color(theme::TEXT_BUTTON)),
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
                    egui::RichText::new("서버 로그 보기")
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
        assert_eq!(ok.0, theme::OK_TEXT);
        assert_eq!(connecting.0, theme::WARN);
        assert_eq!(new.0, theme::TEXT_MUTED);
        assert_ne!(ok, connecting);
        assert_ne!(ok, new);
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

    #[test]
    fn 자리표시_막대는_여덟_개다() {
        // README §4의 수치를 상수로 고정한다 — 그리기 코드에서 숫자를 바꾸면 이 테스트가 잡는다
        assert_eq!(SKELETON_BARS, 8);
        assert_eq!(SKELETON_BAR_HEIGHT, 12.0);
        assert_eq!(SKELETON_GAP, 6.0);
    }
}
