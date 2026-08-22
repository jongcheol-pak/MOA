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
use crate::ui::icon_tex::bgra_to_color_image;
use crate::ui::theme;
use crate::ui::widgets;
use crate::ui::widgets::MenuRowIcon;

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
    icons: &MenuIcons,
    max_height: f32,
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

    // **항목이 화면 높이를 넘으면 그 부분만 세로로 굴린다** — 아이콘 줄과 `추가 옵션 표시`는
    // 제자리에 남는다. 확장이 많이 깔린 PC에서는 목록이 화면보다 길어진다
    // 아이콘 줄·구분선 둘·`추가 옵션 표시`가 쓰는 만큼을 빼고 남는 자리가 목록 몫이다.
    // 아무리 좁아도 한 줄은 보인다 — 0이 되면 무엇이 있는지조차 알 수 없다
    let 고정_높이 = ACTION_ROW_HEIGHT + SEPARATOR_HEIGHT * 2.0 + theme::MENU_ITEM_HEIGHT;
    let 목록_높이 = (max_height - 고정_높이).max(theme::MENU_ITEM_HEIGHT);
    egui::ScrollArea::vertical()
        .max_height(목록_높이)
        .show(ui, |ui| {
            // 스크롤 영역은 별도 `Ui`라 스타일을 다시 세운다 (AGENTS 「팝업 메뉴 한 줄」)
            theme::menu_style(ui);
            ui.set_width(MENU_WIDTH);
            for (index, item) in items.iter().enumerate() {
                if item.separator {
                    ui.separator();
                    continue;
                }
                if widgets::menu_row_rich(
                    ui,
                    icons.row(index),
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
        });

    ui.separator();
    if widgets::menu_row_rich(
        ui,
        MenuRowIcon::Glyph(egui_phosphor::regular::ARROW_SQUARE_OUT),
        crate::i18n::menu_show_more(),
        "",
        false,
        true,
    ) {
        picked = Some(ShellMenuPick::ShowMore);
    }
    picked
}

/// 셸이 준 아이콘을 egui 텍스처로 올려 두는 자리 (FR-8).
///
/// **메뉴가 닫히면 함께 버린다** — 그 메뉴 한 판에서만 쓰는 그림이라 앱 수명 내내 들고 있을
/// 이유가 없다(목록 아이콘을 캐시하는 `ui::icon_tex`와 그 점이 다르다. 그쪽은 같은 확장자가
/// 수천 줄이라 캐시가 곧 성능이다).
///
/// 차례는 `items`와 같다 — 줄 번호로 찾는다
pub struct MenuIcons {
    textures: Vec<Option<egui::TextureHandle>>,
}

impl MenuIcons {
    /// 줄들의 아이콘을 텍스처로 올린다.
    ///
    /// 셸이 아이콘을 주지 않았거나 올리지 못한 자리는 `None`이고, 그 줄은 아이콘 없이
    /// 그려진다(열은 그대로 자리를 지킨다)
    pub fn build(ctx: &egui::Context, items: &[ShellMenuItem]) -> MenuIcons {
        let textures = items
            .iter()
            .enumerate()
            .map(|(index, item)| {
                let (width, height, bgra) = item.icon.clone()?;
                if width <= 0 || height <= 0 {
                    return None;
                }
                let image = bgra_to_color_image(width as usize, height as usize, bgra);
                // 이름에 줄 번호를 넣어 같은 판의 아이콘끼리 덮어쓰지 않게 한다
                Some(ctx.load_texture(
                    format!("shell_menu_icon_{index}"),
                    image,
                    egui::TextureOptions::LINEAR,
                ))
            })
            .collect();
        MenuIcons { textures }
    }

    /// 그 줄에 그릴 아이콘
    fn row(&self, index: usize) -> MenuRowIcon<'_> {
        match self.textures.get(index).and_then(|slot| slot.as_ref()) {
            Some(texture) => MenuRowIcon::Texture(texture),
            None => MenuRowIcon::None,
        }
    }
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
/// 프레임 여백은 더하지 않는다 — 그 몫은 `ui::menu::menu_frame_pad`가 따로 잰다(T7).
///
/// **`ui`를 받는 이유는 위젯 사이 간격 때문이다** — egui는 위젯을 하나 놓을 때마다
/// `item_spacing.y`를 더하는데, 그 값이 스타일에 있어 상수로 적어 두면 스타일이 바뀔 때
/// 조용히 어긋난다. 실제로 그것을 빼고 세었더니 줄마다 3px씩 모자랐다
pub fn menu_size(ui: &egui::Ui, items: &[ShellMenuItem]) -> egui::Vec2 {
    let (rows, separators) = count_rows(items);
    egui::vec2(
        MENU_WIDTH,
        menu_height(rows, separators, ui.spacing().item_spacing.y),
    )
}

/// 셸이 준 줄을 `(항목 수, 구분선 수)`로 센다
fn count_rows(items: &[ShellMenuItem]) -> (usize, usize) {
    let separators = items.iter().filter(|item| item.separator).count();
    (items.len() - separators, separators)
}

/// 메뉴 높이 — 위젯 하나하나와 그 사이 간격을 더한다.
///
/// 놓이는 위젯은 **아이콘 줄 1 + 구분선 1 + 항목 `rows` + 셸 구분선 `separators` +
/// 구분선 1 + `추가 옵션 표시` 1**이고, 간격은 그 사이사이(개수 − 1)에 들어간다.
/// Win32 호출이 없어 이 계산만 따로 시험할 수 있게 떼어 두었다
fn menu_height(rows: usize, separators: usize, gap: f32) -> f32 {
    let widgets = rows + separators + 4;
    ACTION_ROW_HEIGHT
        + SEPARATOR_HEIGHT * 2.0
        + theme::MENU_ITEM_HEIGHT * (rows + 1) as f32
        + SEPARATOR_HEIGHT * separators as f32
        + gap * widgets.saturating_sub(1) as f32
}

/// 구분선 한 줄이 차지하는 높이 — egui `Separator`의 기본 `spacing`이다
/// (`egui::widgets::Separator`가 `style().separator_style(..).spacing`을 읽고, 그 기본이 6.0).
/// **위젯 사이 간격은 여기 포함되지 않는다** — 그것은 `menu_height`가 따로 더한다
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
    fn 줄과_구분선을_따로_센다() {
        let items = vec![줄("열기"), 구분선(), 줄("복사")];
        assert_eq!(count_rows(&items), (2, 1));
        assert_eq!(count_rows(&[]), (0, 0));
    }

    #[test]
    fn 높이는_아이콘_줄과_항목과_구분선과_그_사이_간격을_더한_값이다() {
        // 이 값이 틀리면 화면 가장자리 보정이 어긋나 메뉴가 잘린다
        let 기대 = ACTION_ROW_HEIGHT
            + SEPARATOR_HEIGHT * 2.0
            + theme::MENU_ITEM_HEIGHT * 3.0
            + SEPARATOR_HEIGHT
            // 놓이는 위젯 7개(아이콘 줄·구분선·항목 2·셸 구분선·구분선·추가 옵션) 사이 6칸
            + 3.0 * 6.0;
        assert_eq!(menu_height(2, 1, 3.0), 기대, "항목 2 + 추가 옵션 1 = 세 줄");
    }

    #[test]
    fn 위젯_사이_간격을_빠뜨리지_않는다() {
        // 간격을 0으로 두면 그만큼 낮게 재어 메뉴 아래가 화면 밖으로 나간다
        let 간격_없음 = menu_height(5, 2, 0.0);
        let 간격_있음 = menu_height(5, 2, 3.0);
        assert!(간격_있음 > 간격_없음);
        assert_eq!(간격_있음 - 간격_없음, 3.0 * 10.0, "위젯 11개 사이 10칸");
    }

    #[test]
    fn 항목이_없어도_아이콘_줄과_추가_옵션은_남는다() {
        // 셸이 아무것도 주지 못한 경우다 — 그래도 표준 메뉴로 가는 길은 있어야 한다
        let 기대 = ACTION_ROW_HEIGHT
            + SEPARATOR_HEIGHT * 2.0
            + theme::MENU_ITEM_HEIGHT
            // 위젯 4개(아이콘 줄·구분선·구분선·추가 옵션) 사이 3칸
            + 3.0 * 3.0;
        assert_eq!(menu_height(0, 0, 3.0), 기대);
    }
}
