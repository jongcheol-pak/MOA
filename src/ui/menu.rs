//! 메뉴 바와 단축키 (FR-12·FR-21).
//!
//! 메뉴 구성·단축키는 현행 Win32 판(`app::menu`)의 것을 그대로 옮겼다.
//! 두 가지만 다르다 — ① `(&V)` 같은 니모닉은 egui에 대응 기능이 없어 표기하지 않는다
//! ② 팝업 배경이 시스템 메뉴가 아니라 앱 팔레트를 따른다(FR-21이 현행에서 못 지키던 부분).
//!
//! 이 모듈은 상태를 바꾸지 않는다 — 무엇을 하라는 **명령만 값으로 돌려주고**,
//! 실행은 `ui::app`이 한다(패널·워크스페이스 소유자가 거기이기 때문).
use eframe::egui;

/// 메뉴·단축키가 요청하는 동작
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    NewTab,
    CloseTab,
    Back,
    Forward,
    Up,
    Refresh,
    SplitH,
    SplitV,
    ClosePanel,
    NewWorkspace,
    RenameWorkspace,
    RemoveWorkspace,
    ToggleTree,
    ToggleSidebar,
}

/// 메뉴 항목의 활성/비활성을 가르는 현재 상태
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MenuState {
    /// 패널이 2개 이상인가 — 마지막 하나는 닫을 수 없다 (FR-2)
    pub can_close_panel: bool,
    /// 워크스페이스가 2개 이상인가 — 마지막 하나는 지울 수 없다 (FR-18)
    pub can_remove_workspace: bool,
}

/// 메뉴 바를 그리고 고른 항목을 돌려준다
pub fn show_menu_bar(ui: &mut egui::Ui, state: MenuState) -> Option<Command> {
    let mut command = None;
    egui::MenuBar::new().ui(ui, |ui| {
        ui.menu_button("보기", |ui| {
            item(
                ui,
                "좌우 분할",
                "Ctrl+\\",
                true,
                Command::SplitH,
                &mut command,
            );
            item(
                ui,
                "상하 분할",
                "Ctrl+Shift+\\",
                true,
                Command::SplitV,
                &mut command,
            );
            item(
                ui,
                "패널 닫기",
                "Ctrl+Shift+W",
                state.can_close_panel,
                Command::ClosePanel,
                &mut command,
            );
            ui.separator();
            item(ui, "폴더 트리", "", true, Command::ToggleTree, &mut command);
            item(ui, "새로 고침", "F5", true, Command::Refresh, &mut command);
            item(
                ui,
                "워크스페이스 사이드바",
                "Ctrl+B",
                true,
                Command::ToggleSidebar,
                &mut command,
            );
        });
        ui.menu_button("이동", |ui| {
            item(ui, "뒤로", "Alt+←", true, Command::Back, &mut command);
            item(ui, "앞으로", "Alt+→", true, Command::Forward, &mut command);
            item(ui, "상위 폴더", "Alt+↑", true, Command::Up, &mut command);
        });
        ui.menu_button("탭", |ui| {
            item(ui, "새 탭", "Ctrl+T", true, Command::NewTab, &mut command);
            item(
                ui,
                "탭 닫기",
                "Ctrl+W",
                true,
                Command::CloseTab,
                &mut command,
            );
        });
        ui.menu_button("워크스페이스", |ui| {
            item(
                ui,
                "새 워크스페이스",
                "",
                true,
                Command::NewWorkspace,
                &mut command,
            );
            item(
                ui,
                "이름 바꾸기",
                "F2",
                true,
                Command::RenameWorkspace,
                &mut command,
            );
            item(
                ui,
                "삭제",
                "",
                state.can_remove_workspace,
                Command::RemoveWorkspace,
                &mut command,
            );
        });
    });
    command
}

/// 메뉴 항목 하나 — 오른쪽에 단축키를 함께 보인다(`shortcut`이 비면 표기하지 않는다)
fn item(
    ui: &mut egui::Ui,
    label: &str,
    shortcut: &str,
    enabled: bool,
    command: Command,
    out: &mut Option<Command>,
) {
    let button = egui::Button::new(label).right_text(shortcut);
    if ui.add_enabled(enabled, button).clicked() {
        *out = Some(command);
        ui.close();
    }
}

/// 이번 프레임에 눌린 단축키를 명령으로 바꾼다.
///
/// 무수식 키는 F5 말고는 단축키로 두지 않는다 — 이름 편집 중 텍스트 입력을 가로챈다(현행 판의 같은 결정).
/// 메뉴에 적힌 F2는 사이드바가 직접 처리한다. 삭제는 키를 배정하지 않았다 —
/// 지금 구조에서는 사이드바가 키를 **전역으로** 보기 때문에, Delete를 받으면 파일 목록에서 누른
/// Delete까지 워크스페이스를 지운다. 카드에 포커스를 주고 `has_focus()`일 때만 받으면 해결되지만
/// 그 전환은 사이드바 입력 전반에 걸쳐 있어 별도 작업으로 미뤘다(Deferred)
pub fn poll_shortcuts(ctx: &egui::Context) -> Option<Command> {
    // 포커스를 가진 위젯이 있으면 단축키를 보지 않는다 — 주소창·이름 편집이 키를 먼저 가져간다.
    // 이 검사는 텍스트 입력뿐 아니라 포커스를 받은 위젯 전부를 덮는다(가로채기를 막는 쪽으로 넉넉하게)
    if ctx.egui_wants_keyboard_input() {
        return None;
    }
    ctx.input_mut(|input| {
        shortcut_table()
            .into_iter()
            .find_map(|(modifiers, key, command)| {
                let shortcut = egui::KeyboardShortcut::new(modifiers, key);
                input.consume_shortcut(&shortcut).then_some(command)
            })
    })
}

/// 단축키 → 명령 대응표 (현행 `app::menu::create_accels`와 같은 구성).
/// 수식 키가 많은 조합을 앞에 두어 `Ctrl+Shift+\`가 `Ctrl+\`로 오인되지 않게 한다
fn shortcut_table() -> [(egui::Modifiers, egui::Key, Command); 10] {
    let ctrl_shift = egui::Modifiers::CTRL | egui::Modifiers::SHIFT;
    [
        (ctrl_shift, egui::Key::Backslash, Command::SplitV),
        (ctrl_shift, egui::Key::W, Command::ClosePanel),
        (egui::Modifiers::CTRL, egui::Key::Backslash, Command::SplitH),
        (egui::Modifiers::CTRL, egui::Key::T, Command::NewTab),
        (egui::Modifiers::CTRL, egui::Key::W, Command::CloseTab),
        (egui::Modifiers::CTRL, egui::Key::B, Command::ToggleSidebar),
        (egui::Modifiers::ALT, egui::Key::ArrowLeft, Command::Back),
        (
            egui::Modifiers::ALT,
            egui::Key::ArrowRight,
            Command::Forward,
        ),
        (egui::Modifiers::ALT, egui::Key::ArrowUp, Command::Up),
        (egui::Modifiers::NONE, egui::Key::F5, Command::Refresh),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 같은_키_조합이_두_명령에_겹치지_않는다() {
        // 겹치면 표에서 앞선 것만 동작하고 뒤엣것은 조용히 죽는다
        let table = shortcut_table();
        for (index, (modifiers, key, _)) in table.iter().enumerate() {
            for (other_modifiers, other_key, command) in &table[index + 1..] {
                assert!(
                    !(modifiers == other_modifiers && key == other_key),
                    "{command:?}의 단축키가 앞선 항목과 겹친다"
                );
            }
        }
    }

    #[test]
    fn fr12_단축키_여섯_종이_모두_들어_있다() {
        let table = shortcut_table();
        let has = |modifiers, key| table.iter().any(|(m, k, _)| *m == modifiers && *k == key);
        assert!(has(egui::Modifiers::CTRL, egui::Key::T)); // 새 탭
        assert!(has(egui::Modifiers::CTRL, egui::Key::W)); // 탭 닫기
        assert!(has(egui::Modifiers::ALT, egui::Key::ArrowLeft)); // 뒤로
        assert!(has(egui::Modifiers::ALT, egui::Key::ArrowRight)); // 앞으로
        assert!(has(egui::Modifiers::NONE, egui::Key::F5)); // 새로 고침
        assert!(has(egui::Modifiers::CTRL, egui::Key::Backslash)); // 좌우 분할
        assert!(has(
            egui::Modifiers::CTRL | egui::Modifiers::SHIFT,
            egui::Key::Backslash
        )); // 상하 분할
    }

    #[test]
    fn 무수식_키는_f5_외에_두지_않는다() {
        // F2·Delete를 단축키로 두면 이름 편집 중 텍스트 입력을 가로챈다(현행과 같은 결정)
        for (modifiers, key, _) in shortcut_table() {
            if modifiers == egui::Modifiers::NONE {
                assert_eq!(key, egui::Key::F5);
            }
        }
    }
}
