//! 주소 스트립 — [←][→][↑] 탐색 버튼 + 경로 입력 (FR-6).
//!
//! 입력 정규화는 `panel::address_bar::normalize_input`을 그대로 쓴다(따옴표·상대 경로 처리).
use crate::panel::address_bar::normalize_input;
use crate::panel::history::History;
use crate::ui::theme;
use eframe::egui;
use std::path::{Path, PathBuf};

/// 주소창이 상위(패널)에 돌려주는 탐색 요청
#[derive(Clone, PartialEq, Debug)]
pub enum NavAction {
    Back,
    Forward,
    Up,
    Goto(PathBuf),
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
        ui.horizontal(|ui| {
            // 갈 수 없는 방향은 눌리지 않게 한다 — 버튼이 회색으로 보인다
            if ui
                .add_enabled(history.can_back(), egui::Button::new("←"))
                .on_hover_text("뒤로")
                .clicked()
            {
                action = Some(NavAction::Back);
            }
            if ui
                .add_enabled(history.can_forward(), egui::Button::new("→"))
                .on_hover_text("앞으로")
                .clicked()
            {
                action = Some(NavAction::Forward);
            }
            if ui
                .add_enabled(current.parent().is_some(), egui::Button::new("↑"))
                .on_hover_text("상위 폴더")
                .clicked()
            {
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
                if ui.input(|i| i.key_pressed(egui::Key::Enter))
                    && let Some(path) = normalize_input(current, &self.buffer)
                {
                    action = Some(NavAction::Goto(path));
                }
                self.editing = false;
            }
            if resp.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                self.editing = false;
                resp.surrender_focus();
            }
        });
        action
    }
}
