//! 고를 수 있는 글꼴 목록을 워커 스레드에서 만든다 (FR-48).
//!
//! **UI 스레드에서 만들 수 없다** — 설치된 한글 글꼴을 전수로 읽는 데 1.5초쯤 걸리고
//! (2026-08-13 실측: 이름 열거 1ms + 93개 읽기 1,525ms), 그 위에 아래의 등록 검증이 더 붙는다.
//! 설정 대화를 여는 순간 그만큼 창이 멈추면 안 되므로 `ui::panel`의 `DirLoad`와 같은
//! 방식(워커 + 1회용 채널 + 다시 그리기 요청)으로 감싼다 (AGENTS: UI 스레드 블로킹 I/O 금지).
//!
//! **읽히는 것과 쓸 수 있는 것은 다르다** — `D2Coding`은 바이트가 정상으로 돌아오는데
//! egui에 등록하면 한글 폭이 0이 된다(글자가 통째로 사라진다). 그래서 목록을 만들 때
//! 각 글꼴을 실제로 등록해 보고 **한글이 그려지는 것만** 남긴다. 목록에 있는데 고르면
//! 깨지는 항목이 있으면 사용자에게는 그저 고장이다
use crate::app::fonts;
use eframe::egui;
use std::sync::mpsc::{Receiver, TryRecvError, channel};

/// 등록 검증에 쓰는 글자 — 한글이 실제로 그려지는지만 보면 되므로 짧을수록 좋다
const PROBE_TEXT: &str = "한글";
const PROBE_FONT_PX: f32 = 14.0;
/// 검증용으로 잠깐 등록할 때 쓰는 이름 (화면 글꼴 이름과 겹치지 않으면 된다)
const PROBE_SLOT: &str = "글꼴 검증";

/// 글꼴 목록 상태 — 아직 없음 / 만드는 중 / 준비됨.
///
/// 한 번 만든 목록은 다시 만들지 않는다. 앱이 도는 동안 설치 글꼴이 바뀌는 일은 드물고,
/// 그때마다 1.5초를 다시 쓰면 대화를 여닫을 때마다 목록이 사라졌다 나타난다
#[derive(Default)]
pub struct FontScan {
    pending: Option<Receiver<Vec<String>>>,
    ready: Option<Vec<String>>,
}

impl FontScan {
    pub fn new() -> FontScan {
        FontScan::default()
    }

    /// 아직 목록이 없고 만드는 중도 아니면 워커를 띄운다 (있으면 아무 일도 하지 않는다)
    pub fn ensure_started(&mut self, ctx: &egui::Context) {
        if self.ready.is_some() || self.pending.is_some() {
            return;
        }
        let (tx, rx) = channel();
        self.pending = Some(rx);
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let names = usable_fonts();
            // 수신부가 이미 버려졌으면(대화를 닫았거나 앱이 끝났으면) 전송 실패는 무해하다
            let _ = tx.send(names);
            ctx.request_repaint();
        });
    }

    /// 워커가 끝냈으면 결과를 거둔다 — 매 프레임 불러도 된다
    pub fn poll(&mut self) {
        let Some(rx) = self.pending.as_ref() else {
            return;
        };
        match rx.try_recv() {
            Ok(names) => {
                self.pending = None;
                self.ready = Some(names);
            }
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => {
                // 워커가 결과 없이 사라졌다 — 다시 시도할 수 있게 비워 둔다
                self.pending = None;
            }
        }
    }

    /// 준비된 목록. 아직이면 `None`(대화는 그동안 안내 문구를 보인다)
    pub fn names(&self) -> Option<&[String]> {
        self.ready.as_deref()
    }

    pub fn is_scanning(&self) -> bool {
        self.pending.is_some()
    }
}

/// 설치된 한글 글꼴 중 **이 앱이 실제로 그릴 수 있는 것**만 (워커 스레드에서 돈다).
///
/// 등록 검증에 쓰는 `egui::Context`는 화면 것과 **별개**다 — 여기서 `set_fonts`를 아무리
/// 불러도 사용자가 보는 글꼴은 바뀌지 않는다
fn usable_fonts() -> Vec<String> {
    let probe = egui::Context::default();
    fonts::installed_korean_fonts()
        .into_iter()
        .filter(|name| draws_hangul(&probe, name))
        .collect()
}

/// 그 글꼴을 등록했을 때 한글이 실제로 그려지는가.
///
/// 폭이 0이면 글리프를 찾지 못한 것이다 — 바이트는 멀쩡한데 egui의 글꼴 파서가
/// 읽지 못하는 경우가 실재한다(실측: `D2Coding`)
fn draws_hangul(ctx: &egui::Context, name: &str) -> bool {
    let Some(bytes) = fonts::load_font(name) else {
        return false;
    };
    let mut definitions = egui::FontDefinitions::empty();
    definitions.font_data.insert(
        PROBE_SLOT.to_owned(),
        std::sync::Arc::new(egui::FontData::from_owned(bytes)),
    );
    definitions
        .families
        .entry(egui::FontFamily::Proportional)
        .or_default()
        .push(PROBE_SLOT.to_owned());
    ctx.set_fonts(definitions);

    // 글꼴은 다음 pass부터 적용된다 — 한 프레임 흘려보내고 나서 재야 한다
    let _ = ctx.run_ui(Default::default(), |_ui| {});
    let mut width = 0.0;
    let _ = ctx.run_ui(Default::default(), |ui| {
        width = ui
            .painter()
            .layout_no_wrap(
                PROBE_TEXT.to_owned(),
                egui::FontId::proportional(PROBE_FONT_PX),
                egui::Color32::WHITE,
            )
            .size()
            .x;
    });
    width > 0.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 처음에는_목록도_작업도_없다() {
        let scan = FontScan::new();
        assert!(scan.names().is_none());
        assert!(!scan.is_scanning());
    }

    #[test]
    fn 워커가_끝나면_목록이_준비된다() {
        let ctx = egui::Context::default();
        let mut scan = FontScan::new();
        scan.ensure_started(&ctx);
        assert!(scan.is_scanning(), "워커가 시작되지 않았다");

        // 두 번 불러도 워커는 하나다 — 매 프레임 부르는 자리이므로 이것이 중요하다
        scan.ensure_started(&ctx);

        // 워커는 글꼴을 전수로 읽어 오래 걸린다(실측 1.5초+). 넉넉히 기다린다
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(120);
        while scan.names().is_none() && std::time::Instant::now() < deadline {
            scan.poll();
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        let names = scan.names().expect("목록을 받지 못했다");
        assert!(!names.is_empty(), "쓸 수 있는 글꼴이 하나도 없다");
        assert!(!scan.is_scanning(), "끝났는데 아직 도는 중이라고 한다");
    }

    #[test]
    fn 목록의_모든_글꼴은_한글을_그린다() {
        // T5 Acceptance — 목록에 있는데 고르면 두부(□)가 되는 글꼴이 없어야 한다.
        // `app::fonts`의 목록은 "읽히는 것"까지이고, 여기서 "그려지는 것"으로 좁힌다
        let probe = egui::Context::default();
        let usable = usable_fonts();
        assert!(!usable.is_empty(), "쓸 수 있는 글꼴이 하나도 없다");
        for name in &usable {
            assert!(
                draws_hangul(&probe, name),
                "{name}으로 한글이 그려지지 않는다"
            );
        }
        // 읽히지만 그려지지 않는 글꼴은 걸러졌어야 한다
        let readable = fonts::installed_korean_fonts();
        assert!(
            usable.len() <= readable.len(),
            "거르고 났더니 오히려 늘었다"
        );
    }

    /// `install_fonts`로 등록한 뒤 그 컨텍스트에서 한글 폭을 잰다.
    /// `draws_hangul`을 쓰지 않는 이유: 그 함수가 검증용으로 글꼴을 **다시** 등록해
    /// 방금 등록한 것을 덮어써, 무엇을 재는지가 흐려진다
    fn hangul_width_after_install(family: Option<&str>) -> f32 {
        let ctx = egui::Context::default();
        assert!(
            crate::ui::app::install_fonts(&ctx, family),
            "글꼴을 하나도 등록하지 못했다"
        );
        // 글꼴은 다음 pass부터 적용된다
        let _ = ctx.run_ui(Default::default(), |_ui| {});
        let mut width = 0.0;
        let _ = ctx.run_ui(Default::default(), |ui| {
            width = ui
                .painter()
                .layout_no_wrap(
                    PROBE_TEXT.to_owned(),
                    egui::FontId::proportional(PROBE_FONT_PX),
                    egui::Color32::WHITE,
                )
                .size()
                .x;
        });
        width
    }

    #[test]
    fn 고른_글꼴이_실제로_등록된다() {
        // T5 Acceptance — 고른 글꼴로 화면이 그려져야 한다.
        // 등록 경로가 끊기면 값만 저장되고 화면은 그대로인 채로 남는다
        assert!(
            hangul_width_after_install(Some("굴림")) > 0.0,
            "고른 글꼴로 한글이 그려지지 않는다"
        );
    }

    #[test]
    fn 없는_글꼴을_고르면_기본_글꼴로_돌아간다() {
        // 사용자가 고른 글꼴이 나중에 지워진 경우 — 화면이 두부(□)로 덮이면 안 된다.
        // **설정 값은 건드리지 않는다**(글꼴을 다시 설치하면 되살아나야 한다)는 것은
        // `install_fonts`가 `AppSettings`를 아예 받지 않는 구조로 지켜진다
        assert!(
            hangul_width_after_install(Some("없는글꼴이름XYZ")) > 0.0,
            "폴백(맑은 고딕) 뒤에도 한글이 그려지지 않는다"
        );
    }

    #[test]
    fn 목록을_받은_뒤에는_다시_읽지_않는다() {
        let ctx = egui::Context::default();
        let mut scan = FontScan::new();
        scan.ready = Some(vec!["맑은 고딕".to_owned()]);
        scan.ensure_started(&ctx);
        assert!(!scan.is_scanning(), "이미 목록이 있는데 다시 읽는다");
    }
}
