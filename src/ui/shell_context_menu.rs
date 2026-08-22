//! Windows 11 탐색기 모양의 컨텍스트 메뉴 (FR-8 개정) — **우리가 그린다**.
//!
//! Win11의 모던 메뉴는 탐색기 프로세스 안에서만 사는 표면이라 다른 앱이 띄울 길이 없다.
//! 그래서 **모양만 같게 그리고 항목은 셸에서 읽어 온다**(`fs::shell_menu`) — 그 PC에 깔린
//! 확장(압축·이름 바꾸기 도구 등)이 그대로 목록에 들어온다.
//!
//! 구성은 위에서부터 **아이콘 줄**(잘라내기·복사·이름 바꾸기·공유·삭제) → 항목 줄들 →
//! **`추가 옵션 표시`**다. 마지막 줄이 종전 Windows 표준 메뉴를 그대로 연다 — 글자 없이
//! 스스로 그리는 확장은 이쪽에서만 제대로 보이므로 그 길을 남겨 둔다.
//!
//! **실행하지 않는다** — 고른 것을 값으로 돌려주고 무엇을 할지는 `ui::app`이 정한다
//! (`ui::remote_menu`와 같은 규칙).
use eframe::egui;

use crate::fs::shell_menu::{ShellMenuItem, SubmenuHandle};
use crate::ui::theme;
use crate::ui::widgets;

/// 메뉴 폭 — 우측 단축키 열이 있어 원격 메뉴(180)보다 넓다
pub const MENU_WIDTH: f32 = 260.0;

/// 아이콘 줄의 높이 — 일반 줄보다 조금 높다(기준 이미지가 그렇다)
const ACTION_ROW_HEIGHT: f32 = theme::MENU_ITEM_HEIGHT + 8.0;

/// 아이콘 줄에서 아이콘 글자 크기
const ACTION_ICON_PX: f32 = 16.0;

/// 사용자가 이 메뉴에서 고른 것 — 실행은 `ui::app`이 한다
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShellMenuPick {
    /// 위쪽 아이콘 줄에서 골랐다
    Action(MenuAction),
    /// 셸이 준 항목을 골랐다 — 그 명령 번호를 `ShellMenu::invoke`에 넘긴다
    Command(u32),
    /// 하위 메뉴를 펼친다
    Expand(SubmenuHandle),
    /// 종전 Windows 표준 메뉴를 연다
    ShowMore,
}

/// 아이콘 줄의 다섯 가지 (FR-8·FR-64).
///
/// **`Share`만 셸 verb로 간다** — 나머지 넷은 앱이 자체 기능으로 수행한다(D2). 셸의
/// `rename` verb는 탐색기 자신의 목록 뷰가 처리하는 것이라 다른 호스트에서는 동작하지 않고,
/// 잘라내기·복사도 verb로 부르면 셸이 자기 클립보드 상태를 쥐어 우리 화면의 잘라내기 표시와
/// 어긋난다
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuAction {
    Cut,
    Copy,
    Rename,
    Share,
    Delete,
}

impl MenuAction {
    /// 왼쪽부터의 차례 — 기준 이미지의 순서다
    pub const ALL: [MenuAction; 5] = [
        MenuAction::Cut,
        MenuAction::Copy,
        MenuAction::Rename,
        MenuAction::Share,
        MenuAction::Delete,
    ];

    /// 그 자리에 그릴 아이콘 (phosphor — 프로젝트 아이콘 규약)
    fn glyph(self) -> &'static str {
        match self {
            MenuAction::Cut => egui_phosphor::regular::SCISSORS,
            MenuAction::Copy => egui_phosphor::regular::COPY,
            MenuAction::Rename => egui_phosphor::regular::PENCIL,
            MenuAction::Share => egui_phosphor::regular::SHARE,
            MenuAction::Delete => egui_phosphor::regular::TRASH,
        }
    }

    /// 마우스를 얹으면 뜨는 이름 — 라벨이 없는 줄이라 이것이 유일한 설명이다
    fn tooltip(self) -> &'static str {
        match self {
            MenuAction::Cut => crate::i18n::menu_cut(),
            MenuAction::Copy => crate::i18n::menu_copy(),
            MenuAction::Rename => crate::i18n::rename(),
            MenuAction::Share => crate::i18n::menu_share(),
            MenuAction::Delete => crate::i18n::delete(),
        }
    }
}

/// 이번에 그릴 메뉴의 상태 — 그리기 전에 정해지는 것들
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MenuState {
    /// 고른 항목 수 — 0이면 폴더 배경 메뉴라 아이콘 줄이 통째로 비활성이다
    pub selected: usize,
    /// 셸이 `공유` verb를 주었는가 — 없으면 그 칸만 비활성이다
    pub can_share: bool,
}

impl MenuState {
    /// 그 자리가 눌릴 수 있는가.
    ///
    /// 고른 것이 없으면 다섯 다 뜻이 없고, `이름 바꾸기`는 **하나일 때만** 열린다
    /// (새 이름은 하나뿐이라 여럿에 줄 수 없다 — 원격 메뉴의 같은 규칙)
    pub fn enabled(&self, action: MenuAction) -> bool {
        if self.selected == 0 {
            return false;
        }
        match action {
            MenuAction::Rename => self.selected == 1,
            MenuAction::Share => self.can_share,
            _ => true,
        }
    }
}

/// 메뉴 한 판을 그린다 — 고른 것을 값으로 돌려준다 (FR-8).
///
/// `items`는 `fs::shell_menu`가 읽어 준 줄들이고, `icons`는 그 아이콘을 텍스처로 올려 둔
/// 것이다(줄 차례와 같은 순서, 아이콘이 없으면 `None`).
pub fn show(
    ui: &mut egui::Ui,
    state: MenuState,
    items: &[ShellMenuItem],
    icons: &[Option<egui::TextureHandle>],
) -> Option<ShellMenuPick> {
    // 팝업을 여는 자리에서 공통 항목 스타일을 세운다 (AGENTS 「팝업 메뉴 한 줄」).
    // 하위 메뉴는 부모 스타일을 잇지 않는 별도 `Area`라 거기서도 따로 부른다
    theme::menu_style(ui);
    ui.set_width(MENU_WIDTH);

    let mut picked = None;
    if let Some(action) = action_row(ui, state) {
        picked = Some(ShellMenuPick::Action(action));
    }
    ui.separator();

    for (index, item) in items.iter().enumerate() {
        if item.separator {
            ui.separator();
            continue;
        }
        let icon = icons.get(index).and_then(|slot| slot.as_ref());
        if widgets::menu_row_rich(
            ui,
            icon,
            &item.label,
            &item.shortcut,
            item.submenu.is_some(),
            item.enabled,
        ) {
            picked = Some(match item.submenu {
                Some(handle) => ShellMenuPick::Expand(handle),
                None => ShellMenuPick::Command(item.id),
            });
        }
    }

    ui.separator();
    if widgets::menu_row_rich(ui, None, crate::i18n::menu_show_more(), "", false, true) {
        picked = Some(ShellMenuPick::ShowMore);
    }
    picked
}

/// 위쪽 아이콘 줄 — 다섯 칸을 균등하게 나눈다
fn action_row(ui: &mut egui::Ui, state: MenuState) -> Option<MenuAction> {
    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), ACTION_ROW_HEIGHT),
        egui::Sense::hover(),
    );
    let width = rect.width() / MenuAction::ALL.len() as f32;
    let mut picked = None;
    for (index, action) in MenuAction::ALL.iter().enumerate() {
        let 칸 = egui::Rect::from_min_size(
            egui::pos2(rect.left() + width * index as f32, rect.top()),
            egui::vec2(width, rect.height()),
        );
        let enabled = state.enabled(*action);
        let response = ui.interact(
            칸,
            // 위젯 열쇠는 **화면 언어를 따르지 않는다** — 언어를 바꾸면 상태가 끊긴다
            // (AGENTS 「화면 문구」의 그 예외)
            ui.id().with(("shell_menu_action", index)),
            if enabled {
                egui::Sense::click()
            } else {
                egui::Sense::hover()
            },
        );
        if enabled && response.hovered() {
            // 삭제만 파괴색으로 얹는다 — 되돌리기 어려운 것이 눈에 띄어야 한다
            let 채움 = if *action == MenuAction::Delete {
                theme::MENU_HOT_DANGER
            } else {
                theme::MENU_HOT
            };
            ui.painter()
                .rect_filled(칸.shrink(2.0), theme::MENU_ITEM_CORNER_RADIUS, 채움);
        }
        ui.painter().text(
            칸.center(),
            egui::Align2::CENTER_CENTER,
            action.glyph(),
            egui::FontId::proportional(ACTION_ICON_PX),
            if enabled {
                theme::TEXT
            } else {
                theme::TEXT_DIM
            },
        );
        // 라벨이 없는 줄이라 이름은 툴팁으로만 드러난다
        let response = response.on_hover_text(action.tooltip());
        if enabled && response.clicked() {
            picked = Some(*action);
        }
    }
    picked
}

/// 이 메뉴가 차지할 크기 — 화면 밖으로 나가지 않게 미리 재는 데 쓴다.
///
/// 여백은 더하지 않는다 — 프레임 몫은 `ui::menu::menu_frame_pad`가 따로 잰다(T7)
pub fn menu_size(items: &[ShellMenuItem]) -> egui::Vec2 {
    let 구분선 = items.iter().filter(|item| item.separator).count();
    let 항목 = items.len() - 구분선;
    // 아이콘 줄 + 그 아래 구분선 + 항목들 + 셸이 준 구분선들 + 마지막 구분선 + `추가 옵션 표시`
    let height = ACTION_ROW_HEIGHT
        + SEPARATOR_HEIGHT * 2.0
        + theme::MENU_ITEM_HEIGHT * (항목 + 1) as f32
        + SEPARATOR_HEIGHT * 구분선 as f32;
    egui::vec2(MENU_WIDTH, height)
}

/// 구분선 한 줄이 차지하는 높이 — egui `separator`의 기본값(선 + 위아래 여백)
const SEPARATOR_HEIGHT: f32 = 6.0;

#[cfg(test)]
mod tests {
    use super::*;

    fn 줄(label: &str) -> ShellMenuItem {
        ShellMenuItem {
            id: 1,
            label: label.to_owned(),
            shortcut: String::new(),
            icon: None,
            enabled: true,
            checked: false,
            separator: false,
            submenu: None,
        }
    }

    fn 구분선() -> ShellMenuItem {
        ShellMenuItem {
            separator: true,
            ..줄("")
        }
    }

    #[test]
    fn 고른_것이_없으면_아이콘_줄이_통째로_비활성이다() {
        // 폴더 배경 메뉴다 — 잘라낼 것도 지울 것도 없다
        let state = MenuState {
            selected: 0,
            can_share: true,
        };
        for action in MenuAction::ALL {
            assert!(!state.enabled(action), "{action:?} 는 열리면 안 된다");
        }
    }

    #[test]
    fn 이름_바꾸기는_하나를_골랐을_때만_열린다() {
        // 새 이름은 하나뿐이라 여럿에 줄 수 없다 (원격 메뉴의 같은 규칙)
        let 하나 = MenuState {
            selected: 1,
            can_share: true,
        };
        let 여럿 = MenuState {
            selected: 3,
            can_share: true,
        };
        assert!(하나.enabled(MenuAction::Rename));
        assert!(!여럿.enabled(MenuAction::Rename));
        assert!(여럿.enabled(MenuAction::Copy), "복사는 여럿이어도 된다");
    }

    #[test]
    fn 공유_verb가_없으면_그_칸만_비활성이다() {
        let state = MenuState {
            selected: 1,
            can_share: false,
        };
        assert!(!state.enabled(MenuAction::Share));
        assert!(state.enabled(MenuAction::Cut), "나머지는 그대로 열린다");
    }

    #[test]
    fn 높이는_아이콘_줄과_항목과_구분선을_더한_값이다() {
        // 이 값이 틀리면 화면 가장자리 보정이 어긋나 메뉴가 잘린다
        let items = vec![줄("열기"), 구분선(), 줄("복사")];
        let size = menu_size(&items);
        assert_eq!(size.x, MENU_WIDTH);
        let 기대 = ACTION_ROW_HEIGHT
            + SEPARATOR_HEIGHT * 2.0
            + theme::MENU_ITEM_HEIGHT * 3.0
            + SEPARATOR_HEIGHT;
        assert_eq!(size.y, 기대, "항목 2 + 추가 옵션 1 = 세 줄");
    }

    #[test]
    fn 항목이_없어도_아이콘_줄과_추가_옵션은_남는다() {
        // 셸이 아무것도 주지 못한 경우다 — 그래도 표준 메뉴로 가는 길은 있어야 한다
        let size = menu_size(&[]);
        let 기대 = ACTION_ROW_HEIGHT + SEPARATOR_HEIGHT * 2.0 + theme::MENU_ITEM_HEIGHT;
        assert_eq!(size.y, 기대);
    }
}
