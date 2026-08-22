//! 앱이 **자동 업데이트를 다루는 배선** (FR-62) — `ui::app`의 자식 모듈.
//!
//! 새 판을 확인하고, 받고, 설치 프로그램을 띄우고 앱을 닫는 흐름이 여기 있다.
//! 확인·내려받기 자체는 `crate::app::update`가 맡고 이 파일은 그것을 **화면에 잇는 층**이다 —
//! 배지에 무엇을 보일지, 전송이 도는 중에 물을지, 결과를 언제 알릴지가 이쪽 관심사다.
//!
//! **부모(`ui::app`)의 자식으로 둔 이유**: 이 배선은 `ExplorerApp`의 private 필드
//! (`update`·`update_confirm`·`update_asked_by_hand`·`queue`·`toast`)를 직접 만진다.
//! 형제 모듈로 두면 그 필드를 `pub(crate)`로 넓혀야 하지만, 자식이면 가시성을 그대로 두고
//! 나눌 수 있다(`app::remote`·`app::transfer_conflict`와 같은 판단).
use super::ExplorerApp;
use crate::ui::dialog;
use crate::ui::titlebar;
use eframe::egui;

/// 업데이트 확인 대화 본문 폭 — 두 줄짜리 문장이 들어가 삭제 확인(360)보다 넓다
const UPDATE_DIALOG_WIDTH: f32 = 420.0;

impl ExplorerApp {
    /// 설정 메뉴의 `업데이트` — 지금 다시 확인하고 **결과를 알린다**.
    ///
    /// 앱이 뜰 때 도는 확인은 조용하지만(있으면 배지가 서고 없으면 아무 일도 없다),
    /// 손으로 누른 것은 눌렸다는 것 자체를 알려야 한다
    pub(super) fn check_update_by_hand(&mut self) {
        self.update_asked_by_hand = true;
        self.update.start_check(self.repaint.clone());
    }

    /// 업데이트 배지를 눌렀을 때 — 아직 안 받았으면 받고, 다 받았으면 설치한다.
    ///
    /// **전송이 도는 중이면 먼저 묻는다**(D5) — 설치는 앱을 닫으므로 올리던 파일이 끊긴다
    pub(super) fn start_update(&mut self, ctx: &egui::Context) {
        let active = self.pending_transfer_count();
        if active > 0 {
            self.update_confirm = Some(active);
            return;
        }
        self.proceed_update(ctx);
    }

    /// 확인을 마친 뒤의 실제 진행 — 받기 또는 설치
    fn proceed_update(&mut self, ctx: &egui::Context) {
        use crate::app::update::UpdateStatus;
        match self.update.status() {
            UpdateStatus::Available(_) => self.update.start_download(self.repaint.clone()),
            UpdateStatus::Ready(path) => {
                let path = path.clone();
                self.install_update(&path, ctx);
            }
            _ => {}
        }
    }

    /// 설치 프로그램을 띄우고 앱을 닫는다.
    ///
    /// **띄우기에 성공했을 때만 닫는다** — 닫고 나서 실패하면 사용자는 앱도 업데이트도
    /// 잃는다. 닫는 길은 트레이 `종료`와 같은 것을 쓴다(그 길에 세션 저장이 있다)
    fn install_update(&mut self, installer: &std::path::Path, ctx: &egui::Context) {
        if crate::app::update::install::launch_installer(installer) {
            // 트레이 `종료`와 같은 길로 닫는다 — 그 길에 세션 저장이 있다.
            // 설치 프로그램도 우리를 닫으려 하지만(`taskkill`), 스스로 정상 종료하는 편이
            // 설정을 적을 틈을 확실히 얻는다
            self.quitting = true;
            self.hidden = false;
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }
        let now = ctx.input(|input| input.time);
        self.toast.show(crate::i18n::update_launch_failed(), now);
    }

    /// 아직 끝나지 않은 전송 건수 — 대기·진행을 함께 센다
    fn pending_transfer_count(&self) -> usize {
        self.queue
            .items()
            .iter()
            .filter(|item| item.state.is_pending())
            .count()
    }

    /// 워커가 보내온 것을 거두고, 손으로 물은 확인이면 결과를 알린다
    pub(super) fn pump_update(&mut self, ctx: &egui::Context) {
        use crate::app::update::{UpdateError, UpdateStatus};

        let before = self.update.status().clone();
        self.update.pump();
        let after = self.update.status().clone();
        if before == after {
            return;
        }

        // 다 받았으면 곧바로 설치로 넘어간다 — 사용자는 이미 한 번 눌렀다
        if let UpdateStatus::Ready(path) = &after {
            let path = path.clone();
            self.install_update(&path, ctx);
            return;
        }

        let text = match &after {
            // 손으로 물었을 때만 「최신입니다」를 알린다 — 저절로 도는 확인까지 알리면
            // 앱을 켤 때마다 알림이 뜬다
            UpdateStatus::UpToDate if self.update_asked_by_hand => {
                Some(crate::i18n::update_latest())
            }
            UpdateStatus::Failed(error) => Some(match error {
                UpdateError::ChecksumMismatch => crate::i18n::update_verify_failed(),
                UpdateError::Download => crate::i18n::update_download_failed(),
                // 그 밖의 사유는 「확인하지 못했다」로 묶는다 — 사용자가 할 수 있는 일이
                // 같고(연결 확인·나중에 다시), 응답 형식 같은 말은 뜻이 닿지 않는다
                _ => crate::i18n::update_check_failed(),
            }),
            _ => None,
        };
        if !matches!(after, UpdateStatus::Checking | UpdateStatus::Downloading) {
            self.update_asked_by_hand = false;
        }
        if let Some(text) = text {
            let now = ctx.input(|input| input.time);
            self.toast.show(text, now);
        }
    }

    /// 타이틀바에 넘길 배지 상태 — 상태 기계를 화면 값 둘로 옮긴다
    pub(super) fn update_badge(&self) -> titlebar::UpdateBadge {
        use crate::app::update::UpdateStatus;
        let enabled = self.update.enabled();
        match self.update.status() {
            UpdateStatus::Available(_) => titlebar::UpdateBadge {
                visible: true,
                downloading: false,
                update_enabled: enabled,
            },
            UpdateStatus::Downloading => titlebar::UpdateBadge {
                visible: true,
                downloading: true,
                update_enabled: enabled,
            },
            // **다 받아 둔 채로 머무는 자리** — 설치 프로그램을 띄우지 못했을 때 여기 남는다.
            // 배지를 거두면 다시 눌러 볼 자리가 사라져, 「잠시 후 다시 시도해 주세요」라고
            // 알려 놓고 정작 그 길을 없애는 꼴이 된다(설정 메뉴로 다시 확인하면 이미 받아
            // 대조까지 마친 파일을 버리고 처음부터 받는다)
            UpdateStatus::Ready(_) => titlebar::UpdateBadge {
                visible: true,
                downloading: false,
                update_enabled: enabled,
            },
            _ => titlebar::UpdateBadge {
                update_enabled: enabled,
                ..titlebar::UpdateBadge::default()
            },
        }
    }

    /// 전송이 도는 중에 업데이트를 누르면 뜨는 확인 대화 (D5)
    pub(super) fn show_update_confirm(&mut self, ctx: &egui::Context) {
        let Some(active) = self.update_confirm else {
            return;
        };
        let buttons = [
            dialog::ButtonSpec::strong(crate::i18n::update_confirm_ok()),
            dialog::ButtonSpec::plain(crate::i18n::cancel()),
        ];
        let shell = dialog::show(
            ctx,
            egui::Id::new("update_confirm"),
            UPDATE_DIALOG_WIDTH,
            &buttons,
            |ui| {
                ui.heading(crate::i18n::update_confirm_title());
                ui.add_space(8.0);
                ui.label(crate::i18n::dynamic::update_confirm_body(active));
            },
        );
        let mut decided = match shell.clicked {
            Some(0) => Some(true),
            Some(_) => Some(false),
            None => None,
        };
        // 배경 클릭·`Esc`는 셸이 판정한다 — 되돌릴 수 없는 쪽으로 기울지 않는다
        if shell.should_close {
            decided = Some(false);
        }
        match decided {
            Some(true) => {
                self.update_confirm = None;
                self.proceed_update(ctx);
            }
            Some(false) => self.update_confirm = None,
            None => {}
        }
    }
}
