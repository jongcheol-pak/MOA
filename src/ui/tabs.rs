//! 탭 스트립 — 패널별 독립 탭 (FR-3).
//!
//! 상태는 `panel::tabs::TabsModel`(순수 모델)이 갖고, 이 파일은 그리기와 입력만 한다.
use crate::panel::tabs::{TabsModel, tab_title};
use crate::ui::theme;
use eframe::egui;

/// 탭 스트립이 상위(패널)에 돌려주는 조작
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TabAction {
    Switch(usize),
    Close(usize),
    New,
}

/// 탭 스트립을 그리고 이번 프레임의 조작을 반환한다.
/// 탭이 많으면 가로로 스크롤된다
pub fn show_tab_strip(ui: &mut egui::Ui, model: &TabsModel) -> Option<TabAction> {
    let mut action = None;
    egui::ScrollArea::horizontal()
        .auto_shrink([false, true])
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                for (index, path) in model.paths().iter().enumerate() {
                    let active = index == model.active_index();
                    let title = tab_title(path);
                    // 활성 탭만 밝은 배경으로 구분한다
                    let fill = if active {
                        theme::CONTROL_ACTIVE
                    } else {
                        theme::CONTROL_BG
                    };
                    let button = egui::Button::new(elide(&title))
                        .fill(fill)
                        .min_size(egui::vec2(0.0, 22.0));
                    let resp = ui.add(button).on_hover_text(path.to_string_lossy());
                    if resp.clicked() {
                        action = Some(TabAction::Switch(index));
                    }
                    // 가운데 버튼 클릭으로도 닫는다 (브라우저 관례)
                    if resp.middle_clicked() {
                        action = Some(TabAction::Close(index));
                    }
                    if ui.small_button("×").on_hover_text("탭 닫기").clicked() {
                        action = Some(TabAction::Close(index));
                    }
                    ui.add_space(2.0);
                }
                if ui.small_button("+").on_hover_text("새 탭").clicked() {
                    action = Some(TabAction::New);
                }
            });
        });
    action
}

/// 탭 제목 말줄임 — 긴 폴더명이 스트립을 다 차지하지 않게 한다
fn elide(title: &str) -> String {
    // 문자 수 기준 — 한글은 폭이 넓어 대략 절반으로 잡는다
    const MAX_CHARS: usize = 16;
    let count = title.chars().count();
    if count <= MAX_CHARS {
        return title.to_owned();
    }
    let kept: String = title.chars().take(MAX_CHARS - 1).collect();
    format!("{kept}…")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 짧은_제목은_그대로_둔다() {
        assert_eq!(elide("문서"), "문서");
        assert_eq!(elide("Downloads"), "Downloads");
    }

    #[test]
    fn 긴_제목은_말줄임한다() {
        let long = "아주아주아주아주아주아주긴폴더이름입니다";
        let out = elide(long);
        assert!(out.ends_with('…'));
        assert_eq!(out.chars().count(), 16);
    }

    #[test]
    fn 경계_길이는_자르지_않는다() {
        let exact: String = "가".repeat(16);
        assert_eq!(elide(&exact), exact);
    }
}
