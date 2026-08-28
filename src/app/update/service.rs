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
    /// 받는 중 — `received`는 지금까지 받은 바이트, `total`은 서버가 알려 준 전체 크기다.
    /// **전체 크기는 없을 수 있다**(chunked 전송) — 그때는 백분율을 보이지 않는다(D1)
    Downloading { received: u64, total: Option<u64> },
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
    /// 받는 도중의 중간 보고 — 아래 `ProgressGate`가 거른 것만 온다
    Progress {
        received: u64,
        total: Option<u64>,
    },
    Downloaded(Result<PathBuf, UpdateError>),
}

/// 받은 양을 백분율로 — 전체 크기를 모르면 `None`.
///
/// 서버가 실제보다 작은 크기를 알려 주는 경우가 있어 **100에서 자른다**
pub fn download_percent(received: u64, total: Option<u64>) -> Option<u8> {
    let total = total?;
    if total == 0 {
        return None;
    }
    Some((received.saturating_mul(100) / total).min(100) as u8)
}

/// 진행 보고를 걸러 내는 자리 — **백분율이 바뀌었을 때만 통과시킨다**.
///
/// 조각 하나가 64KB라 100MB짜리 설치 파일이면 조각이 1,600번 온다. 그대로 보내면
/// 채널과 화면 다시 그리기가 그 수만큼 도는데, 화면에 보이는 것은 정수 백분율이라
/// 그 가운데 실제로 달라지는 것은 100번뿐이다
struct ProgressGate {
    last_percent: Option<u8>,
}

impl ProgressGate {
    fn new() -> ProgressGate {
        ProgressGate { last_percent: None }
    }

    /// 이번 값을 보내야 하는가
    fn should_send(&mut self, received: u64, total: Option<u64>) -> bool {
        // 전체 크기를 모르면 보일 백분율이 없다 — 보고 자체가 뜻이 없다(D1)
        let Some(percent) = download_percent(received, total) else {
            return false;
        };
        if self.last_percent == Some(percent) {
            return false;
        }
        self.last_percent = Some(percent);
        true
    }
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
        self.spawn(wake, move |tx, wake| {
            send_and_wake(tx, Message::Checked(fetch()), wake);
        });
    }

    /// 새 판을 받는다 — `Available` 상태에서만 뜻이 있다
    pub fn start_download(&mut self, wake: Wake) {
        let UpdateStatus::Available(info) = self.status.clone() else {
            return;
        };
        self.start_download_with(
            move |progress| install::download_and_verify(&info, progress),
            wake,
        );
    }

    /// 내려받기 작업을 받아서 돌린다 — 위 `start_check_with`와 같은 이유로 갈라 둔다.
    ///
    /// 받는 쪽은 진행을 알리는 통로를 인자로 받는다 — `(받은 누적 바이트, 전체 크기)`
    pub fn start_download_with(
        &mut self,
        download: impl FnOnce(&mut dyn FnMut(u64, Option<u64>)) -> Result<PathBuf, UpdateError>
        + Send
        + 'static,
        wake: Wake,
    ) {
        if !self.enabled || self.busy() {
            return;
        }
        // 아직 한 조각도 받지 않았고 전체 크기도 모른다 — 첫 보고가 오면 채워진다
        self.status = UpdateStatus::Downloading {
            received: 0,
            total: None,
        };
        self.spawn(wake, move |tx, wake| {
            let mut gate = ProgressGate::new();
            let result = download(&mut |received, total| {
                if gate.should_send(received, total) {
                    send_and_wake(tx, Message::Progress { received, total }, wake);
                }
            });
            send_and_wake(tx, Message::Downloaded(result), wake);
        });
    }

    /// 워커 하나를 띄운다 — 일하는 쪽이 보내는 통로를 직접 쥔다.
    ///
    /// 내려받기가 결과 하나가 아니라 **진행 보고를 여러 번** 보내야 해서, 메시지를 돌려받아
    /// 대신 보내 주는 대신 통로를 넘긴다
    fn spawn(&mut self, wake: Wake, work: impl FnOnce(&Sender<Message>, &Wake) + Send + 'static) {
        let (tx, rx) = channel();
        self.rx = Some(rx);
        std::thread::spawn(move || work(&tx, &wake));
    }

    /// 워커가 보내온 것을 거둬 상태를 옮긴다 — 매 프레임 불러도 된다.
    ///
    /// **한 프레임에 쌓인 것을 모두 거둔다** — 진행 보고는 여러 번 오는데 한 번에 하나씩만
    /// 거두면 빠른 회선에서 보고가 쌓인 만큼 설치 시작이 뒤로 밀린다
    pub fn pump(&mut self) {
        loop {
            // 끝을 알리는 메시지가 `rx`를 비우므로, 그것이 이 반복의 종료 조건이기도 하다
            let Some(rx) = self.rx.as_ref() else {
                return;
            };
            match rx.try_recv() {
                Ok(Message::Progress { received, total }) => {
                    self.status = UpdateStatus::Downloading { received, total };
                }
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
                Err(TryRecvError::Empty) => return,
                Err(TryRecvError::Disconnected) => {
                    // 워커가 결과 없이 사라졌다 — 다시 눌러 볼 수 있게 비워 둔다
                    self.rx = None;
                    self.status = UpdateStatus::Failed(UpdateError::Network);
                }
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
                move |_| {
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
    fn 백분율은_전체_크기를_알_때만_난다() {
        assert_eq!(download_percent(42, Some(100)), Some(42));
        assert_eq!(download_percent(0, Some(100)), Some(0));
        assert_eq!(
            download_percent(50, None),
            None,
            "chunked 전송이면 전체 크기가 없다"
        );
        assert_eq!(download_percent(50, Some(0)), None, "0으로 나눌 수 없다");
    }

    #[test]
    fn 백분율은_100을_넘지_않는다() {
        // 서버가 실제보다 작은 크기를 알려 주는 경우가 있다
        assert_eq!(download_percent(150, Some(100)), Some(100));
    }

    #[test]
    fn 진행_보고는_백분율이_바뀔_때만_나간다() {
        let total = Some(10_000u64);
        let mut gate = ProgressGate::new();
        assert!(gate.should_send(100, total), "0%에서 1%로 올랐다");
        assert!(!gate.should_send(150, total), "아직 1%다");
        assert!(!gate.should_send(199, total), "아직 1%다");
        assert!(gate.should_send(200, total), "2%가 됐다");

        // 조각이 몇 번을 오든 나가는 것은 백분율이 바뀐 횟수뿐이다
        let mut gate = ProgressGate::new();
        let sent = (0..=10_000u64)
            .filter(|received| gate.should_send(*received, total))
            .count();
        assert_eq!(sent, 101, "0%부터 100%까지 101번");
    }

    #[test]
    fn 전체_크기를_모르면_진행_보고를_보내지_않는다() {
        // 보일 백분율이 없으니 화면이 달라질 것도 없다 (D1)
        let mut gate = ProgressGate::new();
        assert!(!gate.should_send(1024, None));
        assert!(!gate.should_send(2048, None));
    }

    #[test]
    fn 받는_중의_진행이_상태에_실린다() {
        // 워커를 붙잡아 두고 그 사이의 상태를 본다 — 붙잡지 않으면 한 번의 `pump`가
        // 진행 보고와 끝맺음을 함께 거둬 중간 상태를 볼 수 없다
        let (release_tx, release_rx) = channel::<()>();
        let mut service = UpdateService::new(true);
        service.start_download_with(
            move |progress| {
                progress(42, Some(100));
                let _ = release_rx.recv();
                Ok(PathBuf::from("setup.exe"))
            },
            wake(),
        );

        let downloading = UpdateStatus::Downloading {
            received: 42,
            total: Some(100),
        };
        for _ in 0..200 {
            service.pump();
            if service.status() == &downloading {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert_eq!(service.status(), &downloading);

        let _ = release_tx.send(());
        pump_until_settled(&mut service);
        assert_eq!(
            service.status(),
            &UpdateStatus::Ready(PathBuf::from("setup.exe"))
        );
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
