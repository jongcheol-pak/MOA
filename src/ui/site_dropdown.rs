//! 탭 스트립의 사이트 드롭다운 — 등록된 사이트를 **새 탭으로** 여는 진입점 (FR-33).
//!
//! 이 팝업의 색·치수는 다른 메뉴와 **다르다**(plan 전제 18) — 배경 `#252525`(다른 팝업은
//! `#1E1E1E`), hover `#333333`(다른 메뉴는 `#383838`), 캡션 11px(다른 캡션은 12px).
//! 공용 팝업 부품으로 뭉뚱그리면 이 차이가 조용히 사라지므로 여기서 직접 그린다.
//!
//! 고른 사이트를 값으로 돌려주고 탭 생성·연결은 `ExplorerApp`이 한다 (기존 규약).
use crate::remote::types::SiteId;
use crate::ui::remote_states::RemoteView;
use crate::ui::theme;
use eframe::egui;

// ── 시각 토큰 (원본 `ExplorerPane.dc.html:158-165`, plan 시각 속성 표) ──
/// `▾` 버튼 폭 — 탭 스트립의 `+` 바로 오른쪽에 붙는다
pub const CARET_WIDTH: f32 = 18.0;
/// `▾` 글리프 크기
const CARET_FONT_PX: f32 = 11.0;
/// 팝업 폭
const MENU_WIDTH: f32 = 250.0;
/// 캡션 (인벤토리 #92)
const MENU_CAPTION: &str = "연결 사이트를 새 탭으로";
/// 캡션 글자 크기 — **11px이다**(다른 팝업 캡션은 12px, 전제 18)
const CAPTION_FONT_PX: f32 = 11.0;
/// 행 높이·글자 크기
const ROW_HEIGHT: f32 = 28.0;
const ROW_FONT_PX: f32 = 13.0;
/// 행 안 상태 점 지름 — 사이드바(7px)보다 한 단계 작다
const ROW_DOT: f32 = 6.0;
/// 프로토콜 글자 크기
const PROTO_FONT_PX: f32 = 12.0;
/// 팝업 배경·테두리 — **다른 팝업과 다른 값**이다 (전제 18)
const MENU_BG: egui::Color32 = egui::Color32::from_rgb(0x25, 0x25, 0x25);
/// 행 hover — 다른 메뉴(`#383838`)보다 어둡다 (전제 18)
const ROW_HOT: egui::Color32 = egui::Color32::from_rgb(0x33, 0x33, 0x33);

/// `▾` 버튼과 그 팝업을 그리고, 고른 사이트를 돌려준다.
///
/// **등록된 사이트가 하나도 없으면 버튼 자체를 그리지 않는다** (Acceptance ①) — 눌러도
/// 빈 목록만 나오는 버튼은 자리만 차지한다. 로컬 패널에도 보인다(README §3).
pub fn show_site_dropdown(
    ui: &mut egui::Ui,
    remote: RemoteView<'_>,
    height: f32,
) -> Option<SiteId> {
    // 등록된 사이트가 하나도 없으면 버튼 자체를 그리지 않는다
    remote.sites.visible().next()?;
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(CARET_WIDTH, height), egui::Sense::click());
    let open = egui::Popup::is_id_open(ui.ctx(), response.id);
    // 열려 있는 동안에도 배경이 유지된다 — 어느 버튼에서 나온 팝업인지 보이게 한다
    if open || response.hovered() {
        ui.painter().rect_filled(rect, 0.0, theme::MENU_HOT);
    }
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        "▾",
        egui::FontId::proportional(CARET_FONT_PX),
        if open || response.hovered() {
            theme::TEXT
        } else {
            theme::TEXT_MUTED
        },
    );
    response.clone().on_hover_text("다른 사이트로 새 탭 열기");

    let mut chosen = None;
    egui::Popup::menu(&response)
        .frame(
            egui::Frame::menu(ui.style())
                .fill(MENU_BG)
                .stroke(egui::Stroke::new(1.0, theme::BORDER_CONTROL))
                .corner_radius(0),
        )
        .show(|ui| {
            ui.set_width(MENU_WIDTH);
            ui.label(
                egui::RichText::new(MENU_CAPTION)
                    .size(CAPTION_FONT_PX)
                    .color(theme::TEXT_DIM),
            );
            for record in remote.sites.visible() {
                if show_row(
                    ui,
                    &record.name,
                    record.protocol.label(),
                    remote.is_connected(record.id),
                ) {
                    chosen = Some(record.id);
                    ui.close();
                }
            }
        });
    chosen
}

/// 사이트 한 줄 — 상태 점 · 이름 · 프로토콜 (인벤토리 #93). 눌렸으면 `true`
fn show_row(ui: &mut egui::Ui, name: &str, protocol: &str, connected: bool) -> bool {
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), ROW_HEIGHT),
        egui::Sense::click(),
    );
    if response.hovered() {
        // 이 메뉴만 hover가 `#333338`이 아니라 `#333333`이다 (전제 18)
        ui.painter().rect_filled(rect, 0.0, ROW_HOT);
    }
    let painter = ui.painter();
    let dot_center = egui::pos2(rect.left() + ROW_PAD_X + ROW_DOT / 2.0, rect.center().y);
    painter.circle_filled(
        dot_center,
        ROW_DOT / 2.0,
        if connected {
            theme::OK_DOT
        } else {
            theme::TEXT_DIM
        },
    );

    // 프로토콜을 먼저 오른쪽에 붙여 이름이 길어도 밀려나지 않게 한다
    let proto = painter.layout_no_wrap(
        protocol.to_owned(),
        egui::FontId::proportional(PROTO_FONT_PX),
        theme::TEXT_DIM,
    );
    let proto_left = rect.right() - ROW_PAD_X - proto.size().x;
    painter.galley(
        egui::pos2(proto_left, rect.center().y - proto.size().y / 2.0),
        proto,
        theme::TEXT_DIM,
    );

    let name_left = dot_center.x + ROW_DOT / 2.0 + ROW_GAP;
    let name = painter.layout(
        name.to_owned(),
        egui::FontId::proportional(ROW_FONT_PX),
        theme::TEXT,
        (proto_left - ROW_GAP - name_left).max(0.0),
    );
    painter.galley(
        egui::pos2(name_left, rect.center().y - name.size().y / 2.0),
        name,
        theme::TEXT,
    );
    response.clicked()
}

/// 행 좌우 여백·요소 간격 — 다른 메뉴(`0 12px`)보다 좁다 (전제 18)
const ROW_PAD_X: f32 = 10.0;
const ROW_GAP: f32 = 8.0;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::remote::sites::SiteStore;

    #[test]
    fn 드롭다운_치수는_원본과_같다() {
        // 전제 18 — 이 팝업만 다른 값을 쓴다. 공용 부품으로 합치면 조용히 어긋난다
        assert_eq!(CARET_WIDTH, 18.0);
        assert_eq!(MENU_WIDTH, 250.0);
        assert_eq!(ROW_HEIGHT, 28.0);
        assert_eq!(ROW_DOT, 6.0);
        assert_eq!(ROW_PAD_X, 10.0);
        // 캡션은 11px — 다른 팝업 캡션(12px)과 다르다
        assert_eq!(CAPTION_FONT_PX, 11.0);
        // 배경·hover도 다른 팝업과 다르다
        assert_eq!(MENU_BG, egui::Color32::from_rgb(0x25, 0x25, 0x25));
        assert_eq!(ROW_HOT, egui::Color32::from_rgb(0x33, 0x33, 0x33));
        assert_ne!(ROW_HOT, theme::MENU_HOT, "일반 메뉴 hover와 같아졌다");
    }

    #[test]
    fn 캡션은_인벤토리_원문_그대로다() {
        // 인벤토리 #92
        assert_eq!(MENU_CAPTION, "연결 사이트를 새 탭으로");
    }

    /// 탭 스트립과 같은 가로 배치에서 `▾`가 차지한 폭을 잰다.
    /// 세로 배치로 재면 커서가 y로만 움직여 "그리지 않았다"와 구분되지 않는다
    fn caret_width_in_strip(sites: &SiteStore) -> f32 {
        let ctx = egui::Context::default();
        let mut width = 0.0;
        let _ = ctx.run_ui(Default::default(), |ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                ui.horizontal(|ui| {
                    // 항목 사이 기본 간격이 폭에 섞이지 않게 한다
                    ui.spacing_mut().item_spacing.x = 0.0;
                    let before = ui.cursor().min.x;
                    show_site_dropdown(
                        ui,
                        RemoteView {
                            sites,
                            connected: &[],
                        },
                        28.0,
                    );
                    width = ui.cursor().min.x - before;
                });
            });
        });
        width
    }

    #[test]
    fn 사이트가_없으면_버튼을_그리지_않는다() {
        // Acceptance ① — 눌러도 빈 목록만 나오는 버튼은 자리만 차지한다
        assert_eq!(
            caret_width_in_strip(&SiteStore::new()),
            0.0,
            "사이트가 없는데 `▾` 자리를 잡았다"
        );
    }

    #[test]
    fn 사이트가_있으면_버튼이_자리를_잡는다() {
        let mut sites = SiteStore::new();
        sites.add("배포 서버");
        assert_eq!(
            caret_width_in_strip(&sites),
            CARET_WIDTH,
            "`▾` 버튼 폭이 원본과 다르다"
        );

        // 숨긴 사이트만 남으면 다시 사라진다 — 목록이 비는 것과 같다
        let mut hidden = SiteStore::new();
        let id = hidden.add("배포 서버");
        hidden.hide(id);
        assert_eq!(caret_width_in_strip(&hidden), 0.0);
    }
}
