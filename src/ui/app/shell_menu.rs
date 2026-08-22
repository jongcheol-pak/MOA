//! Windows 11 모양 컨텍스트 메뉴의 상태와 배선 (FR-8).
//!
//! **`ExplorerApp`의 자식으로 둔 이유**는 이 흐름이 그 private 필드(`shell`·`shell_menu`·
//! `pending_show_more`·`file_op_tx`)를 직접 만지기 때문이다 — 자식이면 가시성을 그대로 두고
//! 나눌 수 있다(`ui::app::transfer_conflict`와 같은 판단).
//!
//! **그리기는 `ui::shell_context_menu`가 한다** — 이 모듈은 언제 열고 닫을지, 고른 것으로
//! 무엇을 할지만 정한다.
use eframe::egui;

use super::ExplorerApp;
use crate::ui::menu;
use crate::ui::panel;
use crate::ui::shell_context_menu;
use crate::ui::theme;

/// 셸이 `공유` 항목에 붙이는 verb 이름 — 아이콘 줄의 그 칸이 이것을 찾는다.
///
/// **화면 문구가 아니라 셸이 정한 식별자다** — 앱 언어를 따르면 그 항목을 못 찾는다
/// (AGENTS 「화면 문구」의 그 예외)
const SHARE_VERB: &str = "Windows.Share";

/// 지금 열려 있는 Win11 모양 컨텍스트 메뉴 한 판 (FR-8).
///
/// **셸 인터페이스와 그 판의 아이콘·항목을 함께 든다** — `ShellMenu`가 살아 있어야 고른 것을
/// 실행할 수 있고(`invoke`), 아이콘 텍스처는 이 판에서만 쓰는 그림이라 함께 버려야 한다.
///
/// 하위 메뉴는 **펼친 하나만** 든다 — 셸 메뉴는 두 단계를 넘지 않고, 여러 개를 동시에 펼치는
/// 것은 어느 것이 열려 있는지 화면에서 읽기 어렵다
pub(super) struct OpenShellMenu {
    menu: crate::fs::shell_menu::ShellMenu,
    items: Vec<crate::fs::shell_menu::ShellMenuItem>,
    icons: crate::ui::shell_context_menu::MenuIcons,
    /// 펼쳐 둔 하위 메뉴 — `(손잡이, 그 줄들, 그 아이콘들)`
    submenu: Option<(
        crate::fs::shell_menu::SubmenuHandle,
        Vec<crate::fs::shell_menu::ShellMenuItem>,
        crate::ui::shell_context_menu::MenuIcons,
    )>,
    /// 메뉴가 뜬 자리 (논리 pt)
    pos: egui::Pos2,
    /// 이 메뉴가 대상으로 삼은 폴더와 항목들 — `추가 옵션 표시`가 그대로 다시 쓴다
    folder: std::path::PathBuf,
    items_paths: Vec<std::path::PathBuf>,
    /// 아이콘 줄 판정에 쓰는 상태
    state: crate::ui::shell_context_menu::MenuState,
    /// `공유` verb를 가진 항목의 명령 번호 — 아이콘 줄의 그 칸이 이것을 셸에 넘긴다.
    /// 셸이 그 항목을 주지 않았으면 `None`이고 그 칸은 비활성이다
    share_id: Option<u32>,
}

impl ExplorerApp {
    /// 아이콘 줄에서 고른 것을 수행한다 (FR-8·FR-64).
    ///
    /// **`공유`만 셸에 넘긴다**(D2) — 나머지 넷은 앱이 자체 기능으로 한다. 셸의 `rename`은
    /// 탐색기 자신의 목록 뷰가 처리하는 것이라 여기서 불러도 아무 일이 없고, 잘라내기·복사도
    /// verb로 부르면 셸이 자기 클립보드 상태를 쥐어 우리 화면의 잘라내기 표시와 어긋난다.
    ///
    /// **`이름 바꾸기`는 목록의 인라인 편집을 연다** (FR-64) — 여기서 셸에 거는 것이 아니라
    /// 편집을 열어 두고, 사용자가 `Enter`로 확정한 뒤에 걸린다
    fn apply_menu_action(
        &mut self,
        action: shell_context_menu::MenuAction,
        open: &OpenShellMenu,
        owner: windows::Win32::Foundation::HWND,
    ) {
        use shell_context_menu::MenuAction;
        let targets = &open.items_paths;
        match action {
            MenuAction::Copy => {
                // 담기지 못했으면 클립보드에는 **종전 것이 그대로** 남아 있다 —
                // 그때 잘라내기 표시를 풀면 화면과 클립보드가 어긋난다
                if crate::fs::clipboard::put(targets, false) {
                    self.clear_cut_marks();
                }
            }
            MenuAction::Cut => {
                if crate::fs::clipboard::put(targets, true) {
                    self.set_cut_marks(targets);
                }
            }
            MenuAction::Delete => {
                // **휴지통으로 보낸다** — 메뉴에는 영구 삭제를 가를 자리가 없다(탐색기와 같다).
                // 곧바로 지우는 것은 `Shift+Delete`뿐이다(FR-64)
                crate::fs::file_op::delete_items(
                    targets.clone(),
                    false,
                    owner,
                    self.file_op_tx.clone(),
                    self.repaint.clone(),
                );
            }
            MenuAction::Share => {
                // 이 칸만 셸에 넘긴다 — 대응하는 자체 기능이 없다(D2).
                // `share_id`가 없으면 그 칸이 비활성이라 여기까지 오지 않는다
                if let Some(id) = open.share_id {
                    open.menu.invoke(id, owner);
                }
            }
            MenuAction::Rename => {
                // 메뉴를 연 패널이 곧 활성 패널이다 — 우클릭이 그 패널을 활성으로 만든다.
                // 고른 것이 하나일 때만 이 칸이 열리므로(`MenuState::allows`) 첫 항목이
                // 곧 그 항목이다
                if let Some(panel) = self.command_panel_mut(None) {
                    panel.begin_rename_selected();
                }
            }
        }
    }

    /// 우클릭 요청을 받아 Win11 모양 메뉴를 연다 (FR-8).
    ///
    /// 셸이 메뉴를 주지 못하면(COM 실패·다룰 수 없는 경로) **아무것도 열지 않는다** — 종전
    /// 경로도 그런 경우 조용히 지나갔고, 빈 메뉴를 띄우면 고장으로 보인다
    pub(super) fn open_shell_menu(&mut self, ctx: &egui::Context, request: panel::MenuRequest) {
        let Some(shell) = self.shell.as_ref() else {
            return;
        };
        let Some(menu) = shell.open_menu(&request.folder, &request.items) else {
            return;
        };
        let items = menu.model();
        let icons = shell_context_menu::MenuIcons::build(ctx, &items);
        // `공유`는 셸이 그 verb를 준 항목이 있을 때만 열린다 — 없는데 눌리면 아무 일도
        // 일어나지 않고 사용자는 그 까닭을 알 수 없다
        let share_id = items
            .iter()
            .find(|item| menu.verb(item.id).is_some_and(|verb| verb == SHARE_VERB))
            .map(|item| item.id);
        // **선택 항목의 폴더 여부는 들지 않는다** — 이 메뉴가 그것으로 가르는 것이 없다
        // (아이콘 줄은 고른 개수와 `공유` verb 유무만 본다). 쓰지 않는 값을 실어 두면 다음
        // 사람이 그것으로 무엇을 가르는지 찾게 된다
        self.shell_menu = Some(OpenShellMenu {
            menu,
            items,
            icons,
            submenu: None,
            pos: request.pos,
            folder: request.folder,
            state: shell_context_menu::MenuState {
                selected: request.items.len(),
                can_share: share_id.is_some(),
                // 목록의 인라인 편집이 받는다 (FR-64) — 로컬 목록에서만 열리는데
                // 이 메뉴 자체가 로컬 전용이라(D21) 여기서 더 가릴 것이 없다
                can_rename: true,
            },
            share_id,
            items_paths: request.items,
        });
    }

    /// 열려 있는 메뉴를 그리고 고른 것을 실행한다 (FR-8).
    ///
    /// 바깥을 누르거나 `Esc`면 닫는다 — 메뉴가 화면에 눌어붙지 않게 한다(원격 메뉴와 같은 규칙)
    pub(super) fn show_shell_menu(&mut self, ctx: &egui::Context) {
        let Some(open) = self.shell_menu.as_ref() else {
            return;
        };
        // **보고 있는 폴더가 바뀌면 닫는다** — 메뉴가 가리키는 곳과 화면이 어긋난 채 남으면
        // 엉뚱한 폴더의 항목을 실행하게 된다. 같은 폴더 안에서 파일이 지워지는 경우는 닫지
        // 않는다 — 그때는 셸이 실행 시점에 자기 대화로 알린다(종전과 같은 규칙)
        let 보고_있는_폴더 = self
            .views
            .get(&self.workspaces.active().id)
            .and_then(|view| view.active_dir());
        if 보고_있는_폴더.is_some_and(|dir| dir != open.folder) {
            self.shell_menu = None;
            return;
        }
        let viewport = ctx.input(|input| input.viewport_rect());
        let size = shell_context_menu::menu_size_at(ctx, &open.items);
        let at = menu::clamp_menu_pos(viewport, open.pos, size);
        // 아래로 뻗을 수 있는 만큼이 목록이 쓸 수 있는 최대 높이다
        let max_height = (viewport.bottom() - at.y).max(theme::MENU_ITEM_HEIGHT);

        let (mut picked, rect) = shell_context_menu::show_popup(
            ctx,
            egui::Id::new("shell_context_menu"),
            at,
            open.state,
            &open.items,
            &open.icons,
            max_height,
        );

        // 펼쳐 둔 하위 메뉴는 부모 오른쪽에 붙인다
        let mut submenu_rect = None;
        if let Some((_, rows, icons)) = open.submenu.as_ref() {
            let sub_at = menu::clamp_menu_pos(
                viewport,
                egui::pos2(at.x + size.x, at.y),
                shell_context_menu::menu_size_at(ctx, rows),
            );
            let (sub_picked, sub_rect) = shell_context_menu::show_submenu_popup(
                ctx,
                egui::Id::new("shell_context_submenu"),
                sub_at,
                rows,
                icons,
            );
            if sub_picked.is_some() {
                picked = sub_picked;
            }
            submenu_rect = Some(sub_rect);
        }

        let inside = |pos: egui::Pos2| {
            rect.contains(pos) || submenu_rect.is_some_and(|sub| sub.contains(pos))
        };
        let outside = ctx.input(|input| {
            input.pointer.any_click() && input.pointer.interact_pos().is_none_or(|pos| !inside(pos))
        });
        let escape = ctx.input(|input| input.key_pressed(egui::Key::Escape));
        if let Some(pick) = picked {
            self.apply_shell_menu_pick(ctx, pick);
        } else if outside || escape {
            self.shell_menu = None;
        }
    }

    /// 메뉴에서 고른 것을 수행한다 (FR-8).
    ///
    /// **하위 메뉴 펼치기만 메뉴를 열어 둔다** — 나머지는 무엇을 하든 메뉴를 먼저 닫는다.
    /// 셸 항목은 새 창을 띄우기도 해서, 닫지 않으면 그 창 뒤에 메뉴가 남는다
    fn apply_shell_menu_pick(
        &mut self,
        ctx: &egui::Context,
        pick: shell_context_menu::ShellMenuPick,
    ) {
        let Some(open) = self.shell_menu.as_mut() else {
            return;
        };
        match pick {
            shell_context_menu::ShellMenuPick::Expand(handle) => {
                // 같은 것을 다시 누르면 접는다
                if open
                    .submenu
                    .as_ref()
                    .is_some_and(|(had, ..)| *had == handle)
                {
                    open.submenu = None;
                    return;
                }
                let rows = open.menu.expand(handle);
                let icons = shell_context_menu::MenuIcons::build(ctx, &rows);
                open.submenu = Some((handle, rows, icons));
            }
            shell_context_menu::ShellMenuPick::Command(id) => {
                let owner = self
                    .shell
                    .as_ref()
                    .map(|shell| shell.hwnd())
                    .unwrap_or_default();
                // **닫고 나서 실행한다** — 셸 확장의 `InvokeCommand`는 새 창을 띄우거나 자기
                // 메시지 펌프를 돌기도 해서, 그 사이에 다시 그려지면 이미 고른 메뉴가 화면에
                // 남는다. 나머지 두 분기(`ShowMore`·`Action`)도 같은 순서다.
                // 메뉴를 지우기 전에 인터페이스를 옮겨 잡는다 — 실행은 그것이 살아 있어야 한다
                let Some(open) = self.shell_menu.take() else {
                    return;
                };
                open.menu.invoke(id, owner);
            }
            shell_context_menu::ShellMenuPick::ShowMore => {
                // 우리 메뉴를 먼저 닫고, 표준 메뉴는 그리기가 끝난 뒤에 띄운다.
                // 같은 값을 옮겨 담는 것이라 `take`가 준 것을 그대로 쓴다
                if let Some(open) = self.shell_menu.take() {
                    self.pending_show_more = Some((open.folder, open.items_paths, open.pos));
                }
            }
            shell_context_menu::ShellMenuPick::Action(action) => {
                let owner = self
                    .shell
                    .as_ref()
                    .map(|shell| shell.hwnd())
                    .unwrap_or_default();
                // 무엇을 하든 메뉴를 **먼저 닫는다** — 셸 대화가 뜨는 갈래가 있어서다
                let Some(open) = self.shell_menu.take() else {
                    return;
                };
                self.apply_menu_action(action, &open, owner);
            }
        }
    }
}
