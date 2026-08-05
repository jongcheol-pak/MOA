//! 원격 목록의 우클릭 메뉴와 그 대화들 (FR-39).
//!
//! **로컬 셸 메뉴와 공통 부품으로 묶지 않는다**(plan 비추상화 선언) — 한쪽은 OS가 그리고
//! (`IContextMenu`, 로컬 PIDL 전용 — D21) 한쪽은 우리가 그린다. 겉모습이 비슷하다는 이유로
//! 한 인터페이스에 넣으면 양쪽 제약이 서로를 갉는다.
//!
//! **실행하지 않는다** — 고른 것을 값으로 돌려주고 연결에 명령을 보내는 것은 `ExplorerApp`이다.
use crate::remote::types::RemotePath;
use crate::ui::theme;
use eframe::egui;

// ── 문구 (디자인 미제공 — 이 구현이 정한 신규 문구다) ──
const MENU_DOWNLOAD: &str = "받기";
const MENU_UPLOAD: &str = "올리기";
const MENU_RENAME: &str = "이름 바꾸기…";
const MENU_NEW_FOLDER: &str = "새 폴더…";
const MENU_CHMOD: &str = "권한 변경…";
const MENU_DELETE: &str = "삭제…";
const MENU_REFRESH: &str = "새로 고침";
/// 이름에 쓸 수 없는 글자를 적었을 때
const ERROR_SLASH: &str = "이름에 / 는 쓸 수 없습니다.";
const ERROR_EMPTY: &str = "이름을 입력해 주세요.";

/// 메뉴 폭 — 일반 메뉴와 같은 값 (`FileExplorer-FTP.dc.html:355` 계열)
const MENU_WIDTH: f32 = 180.0;
const ROW_HEIGHT: f32 = 28.0;

/// 메뉴가 다룰 원격 항목 하나.
///
/// 폴더 여부와 크기까지 함께 든다 — `받기`가 폴더를 만나면 그 아래를 훑어야 하고(FR-38),
/// 큐는 크기를 알아야 진행률을 그린다. 메뉴가 뜨는 **그 순간의 선택**을 담아 두는 값이다
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteTarget {
    pub path: RemotePath,
    pub is_dir: bool,
    pub size: u64,
}

/// 사용자가 원격 메뉴에서 고른 것. 실행은 앱이 한다
#[derive(Debug, Clone, PartialEq)]
pub enum RemoteMenuAction {
    /// 고른 항목을 로컬로 받는다 (전송 큐로 들어간다)
    Download,
    /// **다른 패널에서 고른 로컬 항목**을 이 폴더로 올린다.
    ///
    /// 올릴 것을 파일 대화로 고르게 하지 않는다 — 두 칸 탐색기에서 반대편 패널이 이미
    /// "고른 것"을 들고 있고, 끌어다 놓기(FR-38)도 같은 짝을 쓴다
    Upload,
    /// 이름 바꾸기 대화를 연다
    Rename,
    /// 새 폴더 대화를 연다
    NewFolder,
    /// 권한 변경 대화를 연다
    Chmod,
    /// 삭제 확인 대화를 연다
    Delete,
    /// 목록을 다시 읽는다
    Refresh,
}

/// 원격 목록 우클릭 메뉴를 띄운다 (FR-39).
///
/// `has_selection`이 거짓이면 **항목이 있어야 뜻이 있는 것**은 비활성이다 —
/// 빈 자리를 눌렀을 때 `새 폴더`·`새로 고침`만 남는다.
/// `connected`가 거짓이면 서버에 닿는 것이 전부 비활성이다 (plan Edge Case)
pub fn show_remote_menu(
    ui: &mut egui::Ui,
    has_selection: bool,
    connected: bool,
) -> Option<RemoteMenuAction> {
    ui.set_width(MENU_WIDTH);
    let mut chosen = None;
    for row in menu_rows(has_selection, connected) {
        // 구분선 앞뒤로 "고른 것에 하는 일"과 "이 폴더에 하는 일"이 나뉜다
        if row.separator_before {
            ui.separator();
        }
        if menu_row(ui, row.label, row.enabled) {
            chosen = Some(row.action);
        }
    }
    chosen
}

/// 메뉴 한 줄의 구성 — 그리기 전에 정해지는 것들
#[derive(Debug, Clone, PartialEq)]
pub struct MenuRow {
    pub label: &'static str,
    pub action: RemoteMenuAction,
    pub enabled: bool,
    pub separator_before: bool,
}

/// 이번에 보일 메뉴 줄들과 각각의 활성 여부 (plan Edge Case).
///
/// 끊긴 연결에서는 **서버에 닿는 것이 전부** 비활성이다 — 눌러도 되지 않는 것을 눌리게 두면
/// 사용자는 눌렀다가 아무 일도 안 일어나는 것을 보게 된다.
/// `올리기`·`새 폴더`·`새로 고침`은 원격 선택이 없어도 뜻이 있다(각각 반대편 패널의 선택,
/// 지금 폴더가 대상이다)
pub fn menu_rows(has_selection: bool, connected: bool) -> Vec<MenuRow> {
    [
        (MENU_DOWNLOAD, RemoteMenuAction::Download, true, false),
        (MENU_UPLOAD, RemoteMenuAction::Upload, false, false),
        (MENU_RENAME, RemoteMenuAction::Rename, true, false),
        (MENU_CHMOD, RemoteMenuAction::Chmod, true, false),
        (MENU_DELETE, RemoteMenuAction::Delete, true, false),
        (MENU_NEW_FOLDER, RemoteMenuAction::NewFolder, false, true),
        (MENU_REFRESH, RemoteMenuAction::Refresh, false, false),
    ]
    .into_iter()
    .map(|(label, action, needs_item, separator_before)| MenuRow {
        label,
        action,
        enabled: connected && (!needs_item || has_selection),
        separator_before,
    })
    .collect()
}

/// 메뉴 한 줄 — 비활성이면 눌리지 않고 글자가 흐려진다
fn menu_row(ui: &mut egui::Ui, label: &str, enabled: bool) -> bool {
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), ROW_HEIGHT),
        if enabled {
            egui::Sense::click()
        } else {
            egui::Sense::hover()
        },
    );
    if enabled && response.hovered() {
        ui.painter().rect_filled(rect, 0.0, theme::MENU_HOT);
    }
    ui.painter().text(
        egui::pos2(rect.left() + 12.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        label,
        egui::FontId::proportional(13.0),
        if enabled {
            theme::TEXT
        } else {
            theme::TEXT_DIM
        },
    );
    enabled && response.clicked()
}

/// 원격 이름으로 쓸 수 있는가 (plan Edge Case).
///
/// `/`를 막는 이유: 원격 경로의 구분자라 이름에 들어가면 **다른 폴더를 가리키게** 된다.
/// 서버가 거부해 주기를 기대하지 않는다 — 어떤 서버는 그대로 만들어 버린다
pub fn validate_name(name: &str) -> Result<&str, &'static str> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(ERROR_EMPTY);
    }
    if trimmed.contains('/') {
        return Err(ERROR_SLASH);
    }
    Ok(trimmed)
}

/// 권한 비트 아홉 개 ↔ 8진 세 자리 (FR-39 chmod).
///
/// 화면은 체크박스 아홉 개로 보이고 서버에는 숫자로 간다 — **한 곳에서 옮겨** 두 표현이
/// 어긋나지 않게 한다
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Permissions {
    /// `[소유자, 그룹, 기타] × [읽기, 쓰기, 실행]`
    pub bits: [[bool; 3]; 3],
}

impl Permissions {
    /// POSIX 비트에서 만든다 — 아래 세 자리(0o777)만 본다
    pub fn from_mode(mode: u32) -> Permissions {
        let mut bits = [[false; 3]; 3];
        for (group, row) in bits.iter_mut().enumerate() {
            for (bit, slot) in row.iter_mut().enumerate() {
                let shift = (2 - group) * 3 + (2 - bit);
                *slot = mode & (1 << shift) != 0;
            }
        }
        Permissions { bits }
    }

    /// 서버에 보낼 8진 값
    pub fn to_mode(self) -> u32 {
        let mut mode = 0;
        for (group, row) in self.bits.iter().enumerate() {
            for (bit, on) in row.iter().enumerate() {
                if *on {
                    mode |= 1 << ((2 - group) * 3 + (2 - bit));
                }
            }
        }
        mode
    }

    /// 화면에 보이는 세 자리 — `755`
    pub fn to_octal_text(self) -> String {
        format!("{:03o}", self.to_mode())
    }

    /// 적어 넣은 세 자리를 되읽는다. 숫자가 아니거나 범위를 넘으면 `None`
    pub fn from_octal_text(text: &str) -> Option<Permissions> {
        let mode = u32::from_str_radix(text.trim(), 8).ok()?;
        (mode <= 0o777).then(|| Permissions::from_mode(mode))
    }
}

/// 이름 입력 대화 — 이름 바꾸기와 새 폴더가 같은 모양이라 함께 쓴다.
///
/// 확인을 누르면 `Some(이름)`, 취소·닫기면 `Some(빈 문자열)`이 아니라 대화를 닫는 신호로
/// `None`을 돌려주고 `open`을 끈다
pub fn show_name_dialog(
    ctx: &egui::Context,
    title: &str,
    name: &mut String,
    error: &mut Option<String>,
) -> Option<String> {
    let mut confirmed = None;
    let mut closed = false;
    let response = egui::Modal::new(egui::Id::new(("원격 이름 대화", title))).show(ctx, |ui| {
        ui.set_width(360.0);
        ui.label(egui::RichText::new(title).size(16.0).color(theme::TEXT));
        ui.add_space(10.0);
        ui.add(egui::TextEdit::singleline(name).desired_width(f32::INFINITY));
        if let Some(message) = error.as_ref() {
            ui.add_space(6.0);
            ui.label(egui::RichText::new(message).color(theme::ERROR_TEXT));
        }
        ui.add_space(12.0);
        ui.horizontal(|ui| {
            if ui.button("확인").clicked() {
                match validate_name(name) {
                    Ok(valid) => confirmed = Some(valid.to_owned()),
                    Err(message) => *error = Some(message.to_owned()),
                }
            }
            if ui.button("취소").clicked() {
                closed = true;
            }
        });
    });
    if response.should_close() {
        closed = true;
    }
    if closed {
        *error = None;
    }
    confirmed
}

/// 권한 변경 대화 — 8진 세 자리와 체크박스 아홉 개가 서로 따라간다 (Acceptance ③)
pub fn show_chmod_dialog(
    ctx: &egui::Context,
    permissions: &mut Permissions,
    octal: &mut String,
) -> Option<u32> {
    const GROUPS: [&str; 3] = ["소유자", "그룹", "기타"];
    const BITS: [&str; 3] = ["읽기", "쓰기", "실행"];
    let mut confirmed = None;
    egui::Modal::new(egui::Id::new("원격 권한 변경")).show(ctx, |ui| {
        ui.set_width(360.0);
        ui.label(
            egui::RichText::new("권한 변경")
                .size(16.0)
                .color(theme::TEXT),
        );
        ui.add_space(10.0);
        for (group, label) in GROUPS.iter().enumerate() {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(*label).color(theme::HEADER_TEXT));
                for (bit, name) in BITS.iter().enumerate() {
                    if ui
                        .checkbox(&mut permissions.bits[group][bit], *name)
                        .changed()
                    {
                        // 체크를 바꾸면 숫자가 따라간다
                        *octal = permissions.to_octal_text();
                    }
                }
            });
        }
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("숫자(8진):").color(theme::HEADER_TEXT));
            if ui
                .add(egui::TextEdit::singleline(octal).desired_width(80.0))
                .changed()
                && let Some(parsed) = Permissions::from_octal_text(octal)
            {
                // 숫자를 고치면 체크가 따라간다 — 잘못 적은 동안에는 그대로 둔다
                *permissions = parsed;
            }
        });
        ui.add_space(12.0);
        ui.horizontal(|ui| {
            if ui.button("적용").clicked() {
                confirmed = Some(permissions.to_mode());
            }
            if ui.button("취소").clicked() {
                confirmed = None;
            }
        });
    });
    confirmed
}

/// 삭제 확인 대화 (Acceptance ①·Halt Forecast).
///
/// **자동으로 지우는 경로는 없다** — 폴더가 섞여 있으면 `안에 든 것까지 지웁니다`를
/// 따로 켜야 재귀 삭제가 나간다. 되돌릴 수 없는 일이라 두 번 묻는 셈이다.
///
/// 돌려주는 값: `Some(recursive)`면 지운다, `None`이면 아직 고르지 않았거나 취소다
pub fn show_delete_confirm(
    ctx: &egui::Context,
    targets: &[RemotePath],
    recursive: &mut bool,
    cancelled: &mut bool,
) -> Option<bool> {
    let mut confirmed = None;
    let response = egui::Modal::new(egui::Id::new("원격 삭제 확인")).show(ctx, |ui| {
        ui.set_width(420.0);
        ui.label(
            egui::RichText::new("원격 항목 삭제")
                .size(16.0)
                .color(theme::TEXT),
        );
        ui.add_space(8.0);
        ui.label(format!("{}개 항목을 서버에서 지웁니다.", targets.len()));
        for path in targets.iter().take(5) {
            ui.label(egui::RichText::new(path.as_str()).color(theme::TEXT_MUTED));
        }
        if targets.len() > 5 {
            ui.label(egui::RichText::new("…").color(theme::TEXT_MUTED));
        }
        ui.add_space(6.0);
        ui.label(egui::RichText::new("되돌릴 수 없습니다.").color(theme::ERROR_TEXT));
        ui.add_space(8.0);
        ui.checkbox(recursive, "폴더 안에 든 것까지 지웁니다");
        ui.add_space(12.0);
        ui.horizontal(|ui| {
            if ui.button("삭제").clicked() {
                confirmed = Some(*recursive);
            }
            if ui.button("취소").clicked() {
                *cancelled = true;
            }
        });
    });
    if response.should_close() {
        *cancelled = true;
    }
    confirmed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 권한_비트와_팔진_값이_서로_따라간다() {
        // Acceptance ③ — 화면은 체크박스, 서버는 숫자다. 한 곳에서 옮겨야 어긋나지 않는다
        let permissions = Permissions::from_mode(0o755);
        assert_eq!(permissions.to_mode(), 0o755);
        assert_eq!(permissions.to_octal_text(), "755");
        // 소유자 rwx / 그룹 r-x / 기타 r-x
        assert_eq!(permissions.bits[0], [true, true, true]);
        assert_eq!(permissions.bits[1], [true, false, true]);
        assert_eq!(permissions.bits[2], [true, false, true]);

        // 숫자를 고치면 체크가 따라온다
        let parsed = Permissions::from_octal_text("640").expect("8진");
        assert_eq!(parsed.bits[0], [true, true, false]);
        assert_eq!(parsed.bits[1], [true, false, false]);
        assert_eq!(parsed.bits[2], [false, false, false]);
        assert_eq!(parsed.to_mode(), 0o640);

        // 왕복해도 같다
        for mode in [0o000, 0o600, 0o644, 0o700, 0o777] {
            assert_eq!(Permissions::from_mode(mode).to_mode(), mode);
        }
    }

    #[test]
    fn 잘못_적은_팔진_값은_받아들이지_않는다() {
        // 적는 도중의 값(빈 문자열·`8`)에 체크가 흔들리면 안 된다
        assert!(Permissions::from_octal_text("").is_none());
        assert!(Permissions::from_octal_text("8").is_none());
        assert!(
            Permissions::from_octal_text("1000").is_none(),
            "0o777을 넘는다"
        );
        assert!(Permissions::from_octal_text(" 644 ").is_some());
    }

    #[test]
    fn 이름에_구분자를_넣으면_거부한다() {
        // plan Edge Case — `/`가 들어가면 다른 폴더를 가리키게 된다
        assert_eq!(validate_name("보고서.txt"), Ok("보고서.txt"));
        assert_eq!(validate_name("  여백  "), Ok("여백"));
        assert_eq!(validate_name(""), Err(ERROR_EMPTY));
        assert_eq!(validate_name("   "), Err(ERROR_EMPTY));
        assert_eq!(validate_name("위/아래"), Err(ERROR_SLASH));
    }

    #[test]
    fn 메뉴가_한_프레임을_그린다() {
        // 선택 유무·연결 유무 네 조합이 모두 패닉 없이 도는지 본다
        let ctx = egui::Context::default();
        for has_selection in [false, true] {
            for connected in [false, true] {
                let _ = ctx.run_ui(Default::default(), |ui| {
                    egui::CentralPanel::default().show(ui, |ui| {
                        show_remote_menu(ui, has_selection, connected);
                    });
                });
            }
        }
    }

    #[test]
    fn 끊긴_연결에서는_서버에_닿는_줄이_모두_비활성이다() {
        // plan Edge Case — 연결이 끊긴 상태 → 항목 비활성
        for has_selection in [false, true] {
            assert!(
                menu_rows(has_selection, false)
                    .iter()
                    .all(|row| !row.enabled),
                "끊긴 연결에서 눌리는 줄이 남았다"
            );
        }
    }

    #[test]
    fn 고른_것이_없어도_할_수_있는_일은_남는다() {
        let rows = menu_rows(false, true);
        let enabled: Vec<_> = rows
            .iter()
            .filter(|row| row.enabled)
            .map(|row| row.action.clone())
            .collect();
        assert_eq!(
            enabled,
            vec![
                // 올리기는 **반대편 패널의 선택**이 대상이라 원격 선택과 무관하다
                RemoteMenuAction::Upload,
                RemoteMenuAction::NewFolder,
                RemoteMenuAction::Refresh,
            ]
        );
        // 고른 것이 있으면 나머지도 열린다
        assert!(menu_rows(true, true).iter().all(|row| row.enabled));
        // 문구는 구현이 정한 신규 문구 그대로다 (디자인 미제공)
        let labels: Vec<_> = rows.iter().map(|row| row.label).collect();
        assert_eq!(
            labels,
            vec![
                "받기",
                "올리기",
                "이름 바꾸기…",
                "권한 변경…",
                "삭제…",
                "새 폴더…",
                "새로 고침"
            ]
        );
    }
}
