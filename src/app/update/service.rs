//! 업데이트 상태와 워커 (FR-62).
//!
//! 확인·내려받기는 **워커 스레드가** 하고 결과만 채널로 받는다 — 앱이 뜨는 길과
//! 화면을 그리는 길이 네트워크를 기다리지 않는다(AGENTS UI 스레드 원칙).
//!
//! **상태 기계 얼개를 만들지 않는다** — `enum` 하나와 채널 하나로 족하다.
use super::install;
use super::release::{self, ReleaseInfo, UpdateError};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender, TryRecvError, channel};

/// 워커를 깨우고 나서 화면을 다시 그리게 하는 통로 — `ui`가 넘겨준다
pub type Wake = Arc<dyn Fn() + Send + Sync>;

/// 확인 한 번의 결과
pub type CheckResult = Result<Option<ReleaseInfo>, UpdateError>;

/// 지금 업데이트가 어느 자리에 있는가
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateStatus {
    /// 아직 아무것도 하지 않았다
    Idle,
    /// 확인하는 중
    Checking,
    /// 새 판이 있다
    Available(ReleaseInfo),
    /// 받는 중
    Downloading,
    /// 다 받아 대조까지 마쳤다 — 이제 설치할 수 있다
    Ready(PathBuf),
    /// 확인해 보니 지금이 최신이다
    UpToDate,
    /// 확인·내려받기가 실패했다
    Failed(UpdateError),
}

/// 워커가 보내오는 것
enum Message {
    Checked(CheckResult),
    Downloaded(Result<PathBuf, UpdateError>),
}

/// 업데이트 상태를 쥐고 워커를 부리는 자리.
///
/// `ui::app`이 하나를 들고 프레임마다 `pump`한다
pub struct UpdateService {
    /// 이 실행에서 업데이트 기능을 쓰는가 — **설치본 판정을 스스로 하지 않고 받는다**.
    /// 시험은 `target\debug\deps`에서 돌아 판정이 언제나 거짓이 되므로, 여기서 물으면
    /// 「켜진 상태」의 흐름을 한 줄도 시험할 수 없다
    enabled: bool,
    status: UpdateStatus,
    rx: Option<Receiver<Message>>,
}

impl UpdateService {
    /// `enabled`는 부르는 쪽이 정한다 — 앱은 `install::is_installed_build()`를 넘긴다
    pub fn new(enabled: bool) -> UpdateService {
        UpdateService {
            enabled,
            status: UpdateStatus::Idle,
            rx: None,
        }
    }

    pub fn status(&self) -> &UpdateStatus {
        &self.status
    }

    pub fn enabled(&self) -> bool {
        self.enabled
    }

    /// 일하는 중인가 — 겹쳐 시키지 않으려고 본다
    fn busy(&self) -> bool {
        self.rx.is_some()
    }

    /// 최신 릴리즈를 확인한다 (앱이 뜰 때 한 번, 그리고 메뉴에서 누를 때마다).
    ///
    /// 꺼져 있거나 이미 무언가 하는 중이면 아무 일도 하지 않는다
    pub fn start_check(&mut self, wake: Wake) {
        #[cfg(debug_assertions)]
        if install::dev_override().as_deref() == Some("fake") {
            // 저장소에 릴리즈가 하나도 없어도 배지·내려받기 화면을 볼 수 있게 한다 (D14).
            // 시험용 주입 자리를 그대로 쓴다 — 개발 확인 전용 길을 따로 내지 않는다
            self.start_check_with(|| Ok(Some(fake_release())), wake);
            return;
        }
        self.start_check_with(release::fetch_latest, wake);
    }

    /// 확인 작업을 **받아서** 돌린다 — 시험은 즉시 값을 돌려주는 클로저를 넘긴다.
    ///
    /// 게이트(`enabled`)만 주입하고 작업을 박아 두면 시험이 진짜 GitHub을 두드리게 된다
    pub fn start_check_with(
        &mut self,
        fetch: impl FnOnce() -> CheckResult + Send + 'static,
        wake: Wake,
    ) {
        if !self.enabled || self.busy() {
            return;
        }
        self.status = UpdateStatus::Checking;
        self.spawn(wake, move || Message::Checked(fetch()));
    }

    /// 새 판을 받는다 — `Available` 상태에서만 뜻이 있다
    pub fn start_download(&mut self, wake: Wake) {
        let UpdateStatus::Available(info) = self.status.clone() else {
            return;
        };
        self.start_download_with(move || install::download_and_verify(&info), wake);
    }

    /// 내려받기 작업을 받아서 돌린다 — 위 `start_check_with`와 같은 이유로 갈라 둔다
    pub fn start_download_with(
        &mut self,
        download: impl FnOnce() -> Result<PathBuf, UpdateError> + Send + 'static,
        wake: Wake,
    ) {
        if !self.enabled || self.busy() {
            return;
        }
        self.status = UpdateStatus::Downloading;
        self.spawn(wake, move || Message::Downloaded(download()));
    }

    /// 워커 하나를 띄운다 — 결과를 보내고 화면을 깨운다
    fn spawn(&mut self, wake: Wake, work: impl FnOnce() -> Message + Send + 'static) {
        let (tx, rx) = channel();
        self.rx = Some(rx);
        std::thread::spawn(move || {
            let message = work();
            send_and_wake(&tx, message, &wake);
        });
    }

    /// 워커가 보내온 것을 거둬 상태를 옮긴다 — 매 프레임 불러도 된다
    pub fn pump(&mut self) {
        let Some(rx) = self.rx.as_ref() else {
            return;
        };
        match rx.try_recv() {
            Ok(Message::Checked(Ok(Some(info)))) => {
                self.rx = None;
                self.status = UpdateStatus::Available(info);
            }
            Ok(Message::Checked(Ok(None))) => {
                self.rx = None;
                self.status = UpdateStatus::UpToDate;
            }
            Ok(Message::Checked(Err(error))) | Ok(Message::Downloaded(Err(error))) => {
                self.rx = None;
                self.status = UpdateStatus::Failed(error);
            }
            Ok(Message::Downloaded(Ok(path))) => {
                self.rx = None;
                self.status = UpdateStatus::Ready(path);
            }
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => {
                // 워커가 결과 없이 사라졌다 — 다시 눌러 볼 수 있게 비워 둔다
                self.rx = None;
                self.status = UpdateStatus::Failed(UpdateError::Network);
            }
        }
    }
}

/// 결과를 보내고 화면을 깨운다. 받는 쪽이 이미 사라졌으면 깨울 것도 없다
fn send_and_wake(tx: &Sender<Message>, message: Message, wake: &Wake) {
    if tx.send(message).is_ok() {
        wake();
    }
}

/// 개발 확인용 가짜 릴리즈 (D14) — 버전을 아주 높게 잡아 언제나 새 판으로 보인다.
///
/// 주소는 실제로 받으러 가면 실패하는 자리다 — 그 실패 표시도 함께 확인할 수 있다
#[cfg(debug_assertions)]
fn fake_release() -> ReleaseInfo {
    ReleaseInfo {
        version: "99.0.0".to_owned(),
        asset_name: "MOA-Setup-99.0.0.exe".to_owned(),
        asset_url: "https://example.invalid/MOA-Setup-99.0.0.exe".to_owned(),
        sha256: "0".repeat(64),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// 아무것도 하지 않는 깨우기 — 시험에는 그릴 화면이 없다
    fn wake() -> Wake {
        Arc::new(|| {})
    }

    fn info() -> ReleaseInfo {
        ReleaseInfo {
            version: "0.2.0".to_owned(),
            asset_name: "MOA-Setup-0.2.0.exe".to_owned(),
            asset_url: "https://example.com/MOA-Setup-0.2.0.exe".to_owned(),
            sha256: "0".repeat(64),
        }
    }

    /// 워커가 끝날 때까지 `pump`를 돌린다 — 스레드라 첫 `pump`에 값이 없을 수 있다
    fn pump_until_settled(service: &mut UpdateService) {
        for _ in 0..200 {
            service.pump();
            if !service.busy() {
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        panic!("워커가 끝나지 않았다: {:?}", service.status());
    }

    #[test]
    fn 처음에는_아무것도_하지_않은_상태다() {
        let service = UpdateService::new(true);
        assert_eq!(service.status(), &UpdateStatus::Idle);
    }

    #[test]
    fn 확인_결과가_새_판이면_있음으로_옮긴다() {
        let mut service = UpdateService::new(true);
        service.start_check_with(|| Ok(Some(info())), wake());
        pump_until_settled(&mut service);
        assert_eq!(service.status(), &UpdateStatus::Available(info()));
    }

    #[test]
    fn 확인_결과가_없음이면_최신으로_옮긴다() {
        let mut service = UpdateService::new(true);
        service.start_check_with(|| Ok(None), wake());
        pump_until_settled(&mut service);
        assert_eq!(service.status(), &UpdateStatus::UpToDate);
    }

    #[test]
    fn 확인이_실패하면_실패로_옮긴다() {
        let mut service = UpdateService::new(true);
        service.start_check_with(|| Err(UpdateError::Network), wake());
        pump_until_settled(&mut service);
        assert_eq!(
            service.status(),
            &UpdateStatus::Failed(UpdateError::Network)
        );
    }

    #[test]
    fn 확인하는_중에_또_불러도_워커가_늘지_않는다() {
        // 겹쳐 띄우면 뒤엣것이 앞엣것의 결과를 덮어 상태가 튄다
        let count = Arc::new(AtomicUsize::new(0));
        let mut service = UpdateService::new(true);
        for _ in 0..3 {
            let count = count.clone();
            service.start_check_with(
                move || {
                    count.fetch_add(1, Ordering::SeqCst);
                    Ok(None)
                },
                wake(),
            );
        }
        pump_until_settled(&mut service);
        assert_eq!(count.load(Ordering::SeqCst), 1, "일은 한 번만 돌아야 한다");
    }

    #[test]
    fn 받는_중에_또_불러도_워커가_늘지_않는다() {
        let count = Arc::new(AtomicUsize::new(0));
        let mut service = UpdateService::new(true);
        for _ in 0..3 {
            let count = count.clone();
            service.start_download_with(
                move || {
                    count.fetch_add(1, Ordering::SeqCst);
                    Ok(PathBuf::from("setup.exe"))
                },
                wake(),
            );
        }
        pump_until_settled(&mut service);
        assert_eq!(count.load(Ordering::SeqCst), 1, "일은 한 번만 돌아야 한다");
        assert_eq!(
            service.status(),
            &UpdateStatus::Ready(PathBuf::from("setup.exe"))
        );
    }

    #[test]
    fn 꺼져_있으면_워커를_띄우지_않는다() {
        // 설치본이 아닌 실행에서는 확인조차 하지 않는다 (D4)
        let count = Arc::new(AtomicUsize::new(0));
        let mut service = UpdateService::new(false);
        let moved = count.clone();
        service.start_check_with(
            move || {
                moved.fetch_add(1, Ordering::SeqCst);
                Ok(None)
            },
            wake(),
        );
        service.pump();
        assert_eq!(count.load(Ordering::SeqCst), 0, "일이 돌면 안 된다");
        assert_eq!(service.status(), &UpdateStatus::Idle);
        assert!(!service.busy(), "워커가 없어야 한다");
    }

    #[test]
    fn 받기는_새_판이_있을_때만_시작한다() {
        // `Available`이 아닌 자리에서 누르는 길은 화면에 없지만, 상태가 어긋나도
        // 엉뚱한 것을 받으러 가지 않는 것이 이 가드의 뜻이다
        let mut service = UpdateService::new(true);
        service.start_download(wake());
        assert_eq!(service.status(), &UpdateStatus::Idle);
        assert!(!service.busy());
    }

    #[test]
    fn 실패한_뒤에도_다시_확인할_수_있다() {
        let mut service = UpdateService::new(true);
        service.start_check_with(|| Err(UpdateError::Network), wake());
        pump_until_settled(&mut service);
        assert_eq!(
            service.status(),
            &UpdateStatus::Failed(UpdateError::Network)
        );

        service.start_check_with(|| Ok(None), wake());
        pump_until_settled(&mut service);
        assert_eq!(service.status(), &UpdateStatus::UpToDate);
    }
}
