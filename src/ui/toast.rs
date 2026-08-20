//! 짧게 떴다 사라지는 알림 (FR-43·인벤토리 #91).
//!
//! 원본 `FileExplorer-FTP.dc.html:348-349`·README §10 — 오른쪽 아래에 34px 높이로 뜨고
//! **3200ms 뒤 스스로 사라진다.** 확인 버튼이 없다: 놓쳐도 되는 알림에만 쓴다
//! (되돌릴 수 없는 일·오류는 대화나 상태 줄이 맡는다).
//!
//! 시각은 egui가 프레임마다 주는 경과 초(`InputState::time`)로 잰다 — `Instant`를 쓰면
//! 테스트가 실제로 3.2초를 기다려야 한다.
use crate::ui::theme;
use eframe::egui;

// ── 시각 토큰 (원본 `:348-349`, README §10) ──
/// 창 오른쪽·아래에서 띄운 거리
const MARGIN_RIGHT: f32 = 16.0;
const MARGIN_BOTTOM: f32 = 44.0;
const HEIGHT: f32 = 34.0;
const PAD_X: f32 = 14.0;
/// 점과 글자 사이
const GAP: f32 = 10.0;
const DOT: f32 = 7.0;
const FONT_PX: f32 = 13.0;
/// 자동 소멸까지 (README §10 — 3200ms)
pub const LIFETIME_SECS: f64 = 3.2;

/// 등록 알림 문구 (인벤토리 #91) — `<host>` 자리에 사이트 호스트가 들어간다
pub fn registered_text(host: &str) -> String {
    crate::i18n::dynamic::site_registered(host)
}

/// 지금 떠 있는 알림 하나.
///
/// **여러 개를 쌓지 않는다** — 연달아 뜨면 마지막 것만 보이고 시계가 다시 간다
/// (plan Edge Case). 겹쳐 쌓으면 화면 오른쪽 아래가 알림으로 덮인다.
#[derive(Debug, Clone, Default)]
pub struct Toast {
    text: String,
    /// 이 시각(초)이 지나면 사라진다
    until: f64,
}

impl Toast {
    pub fn new() -> Toast {
        Toast::default()
    }

    /// 알림을 띄운다. 이미 떠 있으면 문구를 갈고 시계를 다시 센다
    pub fn show(&mut self, text: impl Into<String>, now: f64) {
        self.text = text.into();
        self.until = now + LIFETIME_SECS;
    }

    /// 지금 보이는가
    pub fn is_visible(&self, now: f64) -> bool {
        !self.text.is_empty() && now < self.until
    }

    /// 화면에 그린다. 떠 있는 동안에는 다시 그리도록 요청해 시간이 흘러도 화면이 갱신된다 —
    /// 입력이 없으면 egui는 프레임을 돌리지 않아, 이것이 없으면 알림이 사라지지 않은 채 굳는다
    pub fn show_ui(&self, ctx: &egui::Context) {
        let now = ctx.input(|input| input.time);
        if !self.is_visible(now) {
            return;
        }
        ctx.request_repaint_after(std::time::Duration::from_millis(100));

        let screen = ctx.content_rect();
        let font = egui::FontId::proportional(FONT_PX);
        let text =
            ctx.fonts_mut(|fonts| fonts.layout_no_wrap(self.text.clone(), font, theme::TEXT));
        let width = PAD_X * 2.0 + DOT + GAP + text.size().x;
        let rect = egui::Rect::from_min_size(
            egui::pos2(
                screen.right() - MARGIN_RIGHT - width,
                screen.bottom() - MARGIN_BOTTOM - HEIGHT,
            ),
            egui::vec2(width, HEIGHT),
        );

        // 알림은 다른 무엇보다 위에 뜬다 — 도크·상태 표시줄에 가리면 알릴 뜻이 없다
        egui::Area::new(egui::Id::new("원격 알림"))
            .order(egui::Order::Foreground)
            .fixed_pos(rect.min)
            .interactable(false)
            .show(ctx, |ui| {
                let painter = ui.painter();
                painter.rect(
                    rect,
                    0.0,
                    theme::HEADER_BG,
                    egui::Stroke::new(1.0, theme::BORDER_CONTROL),
                    egui::StrokeKind::Inside,
                );
                painter.circle_filled(
                    egui::pos2(rect.left() + PAD_X + DOT / 2.0, rect.center().y),
                    DOT / 2.0,
                    theme::TEXT_DIM,
                );
                painter.galley(
                    egui::pos2(
                        rect.left() + PAD_X + DOT + GAP,
                        rect.center().y - text.size().y / 2.0,
                    ),
                    text,
                    theme::TEXT,
                );
            });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 문구는_인벤토리_원문_그대로다() {
        // 인벤토리 #91 (`:582`·README §9·§10).
        // **언어를 잠그고 견준다** — 잠그지 않으면 병렬 실행에서 다른 시험이 영어로 바꾼 찰나에
        // 이 단언이 돌아 간헐 실패한다(2026-08-20 실측: 3회 중 1회). 카탈로그 값을 단언하는
        // 시험은 언제나 잠근다는 것이 AGENTS i18n 규약이다
        let _guard =
            crate::i18n::LanguageGuard::lock(crate::app::settings::LanguageSetting::Korean);
        assert_eq!(
            registered_text("example.test"),
            "example.test 등록됨 · 더블클릭하여 연결"
        );
    }

    #[test]
    fn 삼점이초_뒤에_사라진다() {
        // Acceptance ⑥ — README §10이 정한 3200ms
        assert_eq!(LIFETIME_SECS, 3.2);
        let mut toast = Toast::new();
        assert!(!toast.is_visible(0.0), "띄우지 않았는데 보인다");

        toast.show(registered_text("example.test"), 10.0);
        assert!(toast.is_visible(10.0));
        assert!(toast.is_visible(13.1));
        assert!(!toast.is_visible(13.2), "3.2초가 지났는데 남아 있다");
    }

    #[test]
    fn 연달아_띄우면_마지막_것만_보이고_시계가_다시_간다() {
        // plan Edge Case — 겹쳐 쌓으면 화면 오른쪽 아래가 알림으로 덮인다
        let mut toast = Toast::new();
        toast.show("첫 알림", 10.0);
        toast.show("둘째 알림", 12.0);
        assert_eq!(toast.text, "둘째 알림");
        // 첫 알림 기준(13.2)으로는 사라졌을 시각인데 시계를 다시 세어 살아 있다
        assert!(toast.is_visible(15.1));
        assert!(!toast.is_visible(15.2));
    }
}
