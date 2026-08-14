//! 주소 스트립 — [←][→][↑] 탐색 버튼 + 경로 입력 (FR-6).
//!
//! 입력 정규화는 `panel::address_bar::normalize_input`을 그대로 쓴다(따옴표·상대 경로 처리).
use crate::panel::address_bar::normalize_input;
use crate::panel::history::History;
use crate::remote::url::{RemoteUrl, parse_remote_url};
use crate::ui::theme;
use crate::ui::widgets;
use eframe::egui;
use std::path::{Path, PathBuf};

// ── 시각 토큰 (plan `## 시각 요소 분해` 1:1, 96DPI 기준 고정 px) ──
/// 탐색 버튼 한 변
const NAV_BUTTON_SIZE: f32 = 24.0;
/// 탐색 버튼 아이콘 글꼴 — 타이틀바 버튼과 같은 크기다.
/// 활성·비활성이 이 한 값을 함께 쓴다(상태에 따라 아이콘 크기가 변하면 안 된다)
const NAV_ICON_PX: f32 = 16.0;

/// 주소창이 상위(패널)에 돌려주는 탐색 요청
#[derive(Clone, PartialEq, Debug)]
pub enum NavAction {
    Back,
    Forward,
    Up,
    Goto(PathBuf),
    /// 원격 주소를 입력했다 (FR-34) — 새 원격 탭으로 연다.
    ///
    /// 로컬 경로와 **같은 입력칸**에서 갈린다: `://`가 있고 아는 스킴이면 여기로,
    /// 아니면 위 `Goto`로 간다(`C:tp` 같은 폴더 이름이 원격으로 오해받지 않게 한다)
    GotoRemote(RemoteUrl),
}

/// 주소 스트립 상태 — 편집 중인 문자열만 보유한다(경로 정본은 패널이 갖는다)
pub struct AddressBar {
    /// 입력 버퍼. 편집 중이 아니면 현재 경로로 계속 덮어쓴다
    buffer: String,
    /// 사용자가 입력란을 건드린 뒤인가 — 편집 중에는 폴더가 바뀌어도 버퍼를 덮지 않는다
    editing: bool,
}

impl Default for AddressBar {
    fn default() -> AddressBar {
        AddressBar::new()
    }
}

impl AddressBar {
    pub fn new() -> AddressBar {
        AddressBar {
            buffer: String::new(),
            editing: false,
        }
    }

    /// 스트립을 그리고 이번 프레임의 탐색 요청을 반환한다
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        current: &Path,
        history: &History,
    ) -> Option<NavAction> {
        let mut action = None;
        if !self.editing {
            let shown = current.to_string_lossy();
            if self.buffer != shown {
                self.buffer = shown.into_owned();
            }
        }
        // 이 줄의 바탕은 **활성 탭과 같은 색**이다 — 고른 탭이 아래 내용과 이어져 보여야 한다
        // (Windows 11 탐색기). 자리를 먼저 잡아 두고 줄 높이가 정해진 뒤에 채운다:
        // 내용보다 나중에 그리면 버튼·입력칸을 덮고, 먼저 그리려면 높이를 알 수 없다
        let background = ui.painter().add(egui::Shape::Noop);
        let row = ui
            .horizontal(|ui| {
                if nav_button(
                    ui,
                    egui_phosphor::regular::ARROW_LEFT,
                    history.can_back(),
                    crate::i18n::address_back(),
                ) {
                    action = Some(NavAction::Back);
                }
                if nav_button(
                    ui,
                    egui_phosphor::regular::ARROW_RIGHT,
                    history.can_forward(),
                    crate::i18n::address_forward(),
                ) {
                    action = Some(NavAction::Forward);
                }
                if nav_button(
                    ui,
                    egui_phosphor::regular::ARROW_UP,
                    current.parent().is_some(),
                    crate::i18n::address_up(),
                ) {
                    action = Some(NavAction::Up);
                }

                let edit = egui::TextEdit::singleline(&mut self.buffer)
                    .desired_width(ui.available_width())
                    .text_color(theme::TEXT);
                let resp = ui.add(edit);
                if resp.changed() {
                    self.editing = true;
                }
                if resp.lost_focus() {
                    // 포커스를 잃으면 편집을 접고 현재 경로로 되돌린다(엔터로 확정하지 않은 입력은 버린다)
                    if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        // 원격 주소를 먼저 본다 — 로컬 정규화는 `sftp://`를 상대 경로로 오해한다
                        if let Some(url) = parse_remote_url(&self.buffer) {
                            action = Some(NavAction::GotoRemote(url));
                        } else if let Some(path) = normalize_input(current, &self.buffer) {
                            action = Some(NavAction::Goto(path));
                        }
                    }
                    self.editing = false;
                }
                if resp.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    self.editing = false;
                    resp.surrender_focus();
                }
            })
            .response
            .rect;
        // 바탕은 **패널 폭 전체**를 채운다 — 내용 폭에만 칠하면 오른쪽 끝이 잘려 탭과 이어지지 않는다
        let strip = egui::Rect::from_min_max(
            egui::pos2(ui.max_rect().left(), row.top()),
            egui::pos2(ui.max_rect().right(), row.bottom()),
        );
        ui.painter().set(
            background,
            egui::Shape::rect_filled(strip, 0.0, theme::CONTROL_BG),
        );
        action
    }
}

/// 탐색 버튼 하나 — 프레임 없이 아이콘만 그리고, 눌렸으면 참을 돌려준다.
///
/// 갈 수 없는 방향은 흐린 글자색으로 그리고 hover 배경·툴팁·클릭을 모두 막는다.
/// `add_enabled`를 쓰지 않는 이유: 그것은 버튼 프레임까지 함께 그려 사각 배경이 남는다.
/// 대신 그 함수가 주던 비활성 표현(흐린 색·클릭 차단·툴팁 억제)을 여기서 직접 재현한다
fn nav_button(ui: &mut egui::Ui, icon: &str, enabled: bool, hint: &str) -> bool {
    let size = egui::vec2(NAV_BUTTON_SIZE, NAV_BUTTON_SIZE);
    if !enabled {
        widgets::icon_button_styled(
            ui,
            icon,
            size,
            egui::Color32::TRANSPARENT,
            theme::TEXT_DIM,
            NAV_ICON_PX,
        );
        return false;
    }
    widgets::icon_button_styled(ui, icon, size, theme::CONTROL_HOT, theme::TEXT, NAV_ICON_PX)
        .on_hover_text(hint)
        .clicked()
}
