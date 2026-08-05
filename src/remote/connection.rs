//! 연결 하나 = 워커 스레드 하나 (NFR-10·NFR-11).
//!
//! UI 스레드는 **명령을 채널로 보내고 이벤트를 채널로 받을 뿐** 원격 I/O를 직접 하지 않는다.
//! 그래서 서버가 느리거나 아예 응답하지 않아도 창이 멈추지 않고, 한 연결이 막혀도 다른 연결과
//! 로컬 탐색은 그대로 돈다 — 연결마다 스레드·채널·상태가 따로이기 때문이다.
//!
//! `remote`는 `ui`(egui)를 모르므로 화면을 직접 깨우지 못한다. 대신 **깨우기 콜백**(`Wake`)을
//! 주입받아 이벤트를 보낸 뒤 부른다 (D6 — 썸네일 워커가 겪은 "화면이 50ms마다 스스로 확인"을
//! 되풀이하지 않기 위함).
use std::fs::{File, OpenOptions};
use std::io::{Seek, SeekFrom};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use crate::remote::log::{LogBuffer, LogKind, mask_secrets};
use crate::remote::types::{
    Progress, RemoteEntry, RemoteError, RemotePath, RemoteResult, RemoteSession, SiteId, SiteRecord,
};

/// 화면을 다시 그리게 하는 콜백 (D6). `remote`가 egui를 모른 채 화면을 깨우는 유일한 통로다
pub type Wake = Arc<dyn Fn() + Send + Sync>;

/// 연결 식별자 — 같은 사이트에 두 번 연결하면 서로 다른 값을 받는다
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ConnectionId(pub u32);

/// 전송 하나의 식별자 — 큐(T17)가 발급한다
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TransferId(pub u64);

/// 진행률 이벤트를 보내는 최소 간격.
///
/// 64KB마다(= 초당 수백 번) 이벤트를 보내면 채널과 화면이 그 갱신에 잠식된다 —
/// 사람이 알아볼 수 있는 간격으로 묶어 보낸다. 전송 자체는 그대로 64KB 단위다
const PROGRESS_INTERVAL: Duration = Duration::from_millis(100);

/// `Drop`에서 워커가 끝나기를 기다리는 시간.
///
/// 워커가 서버 응답을 기다리는 중일 수 있어 무한정 기다리지 않는다 — 이만큼 기다려 보고
/// 안 끝나면 스레드를 떼어 놓는다(프로세스 종료가 정리한다)
const STOP_GRACE: Duration = Duration::from_millis(200);

/// 연결이 다시 시도할 규칙.
///
/// 서버가 "지금은 안 된다"(FTP 421 등)고 답하거나 네트워크가 잠깐 끊긴 것은 다시 걸면 되는
/// 실패다. 반대로 **인증 실패·호스트 키 거부는 다시 걸어도 같은 답**이라 재시도하지 않는다.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetryPolicy {
    pub base: Duration,
    pub max: Duration,
    /// 재시도 횟수 상한 — 이만큼 실패하면 포기한다
    pub attempts: u32,
}

impl RetryPolicy {
    /// 1초에서 시작해 두 배씩, 30초를 넘지 않게, 5회까지
    pub const DEFAULT: RetryPolicy = RetryPolicy {
        base: Duration::from_secs(1),
        max: Duration::from_secs(30),
        attempts: 5,
    };

    /// `attempt`(0부터)번째 재시도 전에 기다릴 시간. 상한을 넘으면 `None`(포기)
    pub fn delay(&self, attempt: u32) -> Option<Duration> {
        if attempt >= self.attempts {
            return None;
        }
        // 2의 거듭제곱이 커져도 넘치지 않게 상한에서 자른다
        let factor = 1u32.checked_shl(attempt).unwrap_or(u32::MAX);
        Some(self.base.saturating_mul(factor).min(self.max))
    }
}

/// 전송 방향
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferDirection {
    Upload,
    Download,
}

/// 전송 한 건의 지시
#[derive(Debug, Clone, PartialEq)]
pub struct TransferRequest {
    pub id: TransferId,
    pub direction: TransferDirection,
    pub remote: RemotePath,
    pub local: PathBuf,
    /// 이어받기 시작점 — 0이면 처음부터.
    ///
    /// **받기는 이 값을 쓰지 않는다** — 받다 만 파일의 크기를 재는 것은 파일시스템 호출이라
    /// 화면 쪽에서 하면 프레임이 멈춘다(AGENTS: UI 스레드 블로킹 I/O 금지). 그래서 **워커가**
    /// 아래 `remote_size`와 실제 파일 크기로 직접 정한다. 보내기는 호출부가 정한 값을 쓴다
    pub offset: u64,
    /// 서버가 가진 파일 크기 — 받기에서 이어받기 지점을 정하는 데 쓴다(0이면 모른다)
    pub remote_size: u64,
}

/// 워커에게 보내는 명령
#[derive(Debug, Clone, PartialEq)]
pub enum ConnCommand {
    Connect,
    /// 목록 조회. `generation`은 **늦게 도착한 이전 요청의 결과를 버리기 위한** 번호다
    List {
        generation: u64,
        path: RemotePath,
    },
    Cwd(RemotePath),
    Mkdir(RemotePath),
    Remove(RemotePath),
    Rmdir(RemotePath),
    Rename {
        from: RemotePath,
        to: RemotePath,
    },
    Chmod {
        path: RemotePath,
        mode: u32,
    },
    Transfer(TransferRequest),
    /// 진행 중인 전송을 그 자리에서 멈춘다
    Cancel,
    Disconnect,
    /// 워커를 끝낸다
    Stop,
}

/// 파일 작업 종류 — 결과 이벤트가 무엇에 대한 답인지 알리는 데 쓴다
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpKind {
    Cwd,
    Mkdir,
    Remove,
    Rmdir,
    Rename,
    Chmod,
    Disconnect,
}

/// 연결의 현재 단계 — 화면 표시(T10)가 이것을 투영한다
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnPhase {
    /// 아직 연결을 걸지 않았다
    Idle,
    Connecting,
    Ready,
    Failed {
        detail: String,
    },
    Closed,
}

/// 워커가 돌려주는 소식
#[derive(Debug, Clone, PartialEq)]
pub enum ConnEvent {
    Phase(ConnPhase),
    Listed {
        generation: u64,
        path: RemotePath,
        entries: Vec<RemoteEntry>,
    },
    /// 서버와 주고받은 줄 — `Connection`이 자기 로그 버퍼에 쌓고 화면(T20)이 읽는다.
    ///
    /// **이 변형은 `log_event`로만 만든다** — 거기서 비밀번호를 가리기 때문이다(D14).
    /// 직접 만들면 평문이 채널을 타고 나간다(열거형 필드에는 가시성을 걸 수 없어 규약으로 지킨다)
    Log {
        kind: LogKind,
        text: String,
    },
    OpDone {
        op: OpKind,
        result: Result<(), RemoteError>,
    },
    TransferProgress {
        id: TransferId,
        transferred: u64,
    },
    TransferDone {
        id: TransferId,
        result: Result<u64, RemoteError>,
    },
}

/// 연결 하나 — 워커 스레드의 손잡이다.
///
/// 이 값이 사라지면 워커도 끝난다(`Drop`).
pub struct Connection {
    pub id: ConnectionId,
    pub site: SiteId,
    tx: Sender<ConnCommand>,
    rx: Receiver<ConnEvent>,
    handle: Option<JoinHandle<()>>,
    /// 진행 중인 전송을 멈추라는 신호 — 워커의 진행 통지가 매 64KB마다 본다
    cancel: Arc<AtomicBool>,
    /// 이 연결을 접는다는 신호. **명령 채널을 보지 않는 구간**(재시도 백오프 대기)에서도
    /// 워커가 이것을 살펴 곧바로 빠져나온다
    shutdown: Arc<AtomicBool>,
    /// 워커가 끝나면 켜진다 — `Drop`이 이것으로 회수를 확인한다
    finished: Arc<AtomicBool>,
    phase: ConnPhase,
    /// 이 연결의 서버 로그 (FR-40). 워커가 올린 줄을 `poll`이 여기 쌓고 화면은 읽기만 한다
    log: LogBuffer,
}

impl Connection {
    /// 워커를 띄운다. `session`은 프로토콜 구현(`FtpSession`·`SftpSession`)이거나
    /// 테스트의 가짜 세션이며, **이 스레드로 소유가 넘어간다**
    pub fn spawn(
        id: ConnectionId,
        site: SiteRecord,
        password: String,
        session: Box<dyn RemoteSession>,
        wake: Wake,
        retry: RetryPolicy,
    ) -> Connection {
        let site_id = site.id;
        let (command_tx, command_rx) = channel::<ConnCommand>();
        let (event_tx, event_rx) = channel::<ConnEvent>();
        let cancel = Arc::new(AtomicBool::new(false));
        let finished = Arc::new(AtomicBool::new(false));
        let shutdown = Arc::new(AtomicBool::new(false));

        let worker_cancel = Arc::clone(&cancel);
        let worker_shutdown = Arc::clone(&shutdown);
        let worker_finished = Arc::clone(&finished);
        let handle = std::thread::spawn(move || {
            worker(Worker {
                site,
                password,
                session,
                rx: command_rx,
                tx: event_tx,
                wake,
                cancel: worker_cancel,
                shutdown: worker_shutdown,
                retry,
            });
            worker_finished.store(true, Ordering::SeqCst);
        });

        Connection {
            id,
            site: site_id,
            tx: command_tx,
            rx: event_rx,
            handle: Some(handle),
            cancel,
            shutdown,
            finished,
            phase: ConnPhase::Idle,
            log: LogBuffer::new(),
        }
    }

    /// 명령을 보낸다. 워커가 이미 죽었으면 `false` — **조용히 무시하는 것이 맞다**
    /// (연결이 끊긴 뒤 남은 화면 조작이 오류 대화를 띄우면 사용자만 성가시다)
    pub fn send(&self, command: ConnCommand) -> bool {
        self.tx.send(command).is_ok()
    }

    /// 진행 중인 전송을 멈추라고 알린다. 워커가 다음 64KB 경계에서 멈춘다
    pub fn cancel_transfer(&self) {
        self.cancel.store(true, Ordering::SeqCst);
    }

    /// 취소 신호를 내린다 — 다음 전송이 곧바로 취소되지 않게
    pub fn clear_cancel(&self) {
        self.cancel.store(false, Ordering::SeqCst);
    }

    /// 도착한 이벤트를 모두 가져온다. 단계 변화는 여기서 반영된다
    pub fn poll(&mut self) -> Vec<ConnEvent> {
        let mut events = Vec::new();
        // 비었거나 워커가 사라지면 `try_recv`가 실패하고 그 자리에서 끝난다
        while let Ok(event) = self.rx.try_recv() {
            match &event {
                ConnEvent::Phase(phase) => self.phase = phase.clone(),
                ConnEvent::Log { kind, text } => self.log.push(*kind, text.clone()),
                _ => {}
            }
            events.push(event);
        }
        events
    }

    pub fn phase(&self) -> &ConnPhase {
        &self.phase
    }

    /// 이 연결의 서버 로그 — 화면은 읽기만 한다 (FR-40)
    pub fn log(&self) -> &LogBuffer {
        &self.log
    }

    /// 워커가 끝났는가 — 회수 확인용
    pub fn is_finished(&self) -> bool {
        self.finished.load(Ordering::SeqCst)
    }
}

impl Drop for Connection {
    fn drop(&mut self) {
        // 전송 중이거나 재시도를 기다리는 중이라면 먼저 깨워 놓아야 `Stop`을 볼 수 있다
        self.cancel.store(true, Ordering::SeqCst);
        self.shutdown.store(true, Ordering::SeqCst);
        let _ = self.tx.send(ConnCommand::Stop);

        let deadline = Instant::now() + STOP_GRACE;
        while !self.finished.load(Ordering::SeqCst) && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(5));
        }
        // 제때 끝난 워커만 거둔다. 서버 응답을 기다리며 붙잡힌 스레드까지 기다리면
        // 창을 닫는 순간 앱이 멈춘 것처럼 보인다
        if self.finished.load(Ordering::SeqCst)
            && let Some(handle) = self.handle.take()
        {
            let _ = handle.join();
        }
    }
}

/// 워커가 들고 도는 것들
struct Worker {
    site: SiteRecord,
    password: String,
    session: Box<dyn RemoteSession>,
    rx: Receiver<ConnCommand>,
    tx: Sender<ConnEvent>,
    wake: Wake,
    cancel: Arc<AtomicBool>,
    shutdown: Arc<AtomicBool>,
    retry: RetryPolicy,
}

impl Worker {
    /// 이벤트를 보내고 화면을 깨운다. 수신부가 사라졌으면 `false` — 워커가 스스로 끝낼 신호다
    fn emit(&self, event: ConnEvent) -> bool {
        if self.tx.send(event).is_err() {
            return false;
        }
        (self.wake)();
        true
    }

    fn log(&self, kind: LogKind, text: String) -> bool {
        self.emit(log_event(kind, text))
    }
}

/// 워커 본체 — 명령을 하나씩 순서대로 처리한다.
///
/// **한 번에 하나만 처리한다**는 것이 이 구조의 핵심이다: 탐색과 전송을 같은 연결에 태우면
/// 서로 끼어들지 않고 차례로 처리되며(D4의 겸용 상태), 다른 연결은 자기 워커에서 따로 돈다.
fn worker(mut worker: Worker) {
    // 명령 송신부가 사라지면 `recv`가 실패한다 — 이 연결은 더 쓰이지 않으므로 끝낸다
    while let Ok(command) = worker.rx.recv() {
        match command {
            ConnCommand::Stop => break,
            ConnCommand::Connect => {
                if !connect_with_retry(&mut worker) {
                    break;
                }
            }
            ConnCommand::List { generation, path } => {
                let result = worker.session.list(&path);
                let alive = match result {
                    Ok(entries) => worker.emit(ConnEvent::Listed {
                        generation,
                        path,
                        entries,
                    }),
                    Err(err) => worker.log(LogKind::Error, err.to_string()),
                };
                if !alive {
                    break;
                }
            }
            ConnCommand::Cwd(path) => {
                let result = worker.session.cwd(&path);
                if !finish_op(&worker, OpKind::Cwd, result) {
                    break;
                }
            }
            ConnCommand::Mkdir(path) => {
                let result = worker.session.mkdir(&path);
                if !finish_op(&worker, OpKind::Mkdir, result) {
                    break;
                }
            }
            ConnCommand::Remove(path) => {
                let result = worker.session.remove(&path);
                if !finish_op(&worker, OpKind::Remove, result) {
                    break;
                }
            }
            ConnCommand::Rmdir(path) => {
                let result = worker.session.rmdir(&path);
                if !finish_op(&worker, OpKind::Rmdir, result) {
                    break;
                }
            }
            ConnCommand::Rename { from, to } => {
                let result = worker.session.rename(&from, &to);
                if !finish_op(&worker, OpKind::Rename, result) {
                    break;
                }
            }
            ConnCommand::Chmod { path, mode } => {
                let result = worker.session.chmod(&path, mode);
                if !finish_op(&worker, OpKind::Chmod, result) {
                    break;
                }
            }
            ConnCommand::Transfer(request) => {
                if !run_transfer(&mut worker, request) {
                    break;
                }
            }
            // 전송 중이 아닐 때의 취소는 할 일이 없다 — 신호는 `Connection::cancel_transfer`가 든다
            ConnCommand::Cancel => worker.cancel.store(true, Ordering::SeqCst),
            ConnCommand::Disconnect => {
                let result = worker.session.quit();
                let alive = worker.emit(ConnEvent::Phase(ConnPhase::Closed))
                    && finish_op(&worker, OpKind::Disconnect, result);
                if !alive {
                    break;
                }
            }
        }
    }
    // 끝내기 전에 인사만 하고 결과는 보지 않는다 — 소켓은 어차피 닫힌다
    let _ = worker.session.quit();
}

/// 연결·로그인을 시도하고, 다시 걸어 볼 만한 실패면 지수 백오프로 되풀이한다.
///
/// 돌아오는 값은 **워커를 계속 돌릴지**다(수신부가 사라졌으면 `false`).
fn connect_with_retry(worker: &mut Worker) -> bool {
    let mut attempt = 0;
    loop {
        if !worker.emit(ConnEvent::Phase(ConnPhase::Connecting)) {
            return false;
        }
        let outcome = worker
            .session
            .connect(&worker.site)
            .and_then(|()| worker.session.login(&worker.site, &worker.password));

        let Err(err) = outcome else {
            return worker.emit(ConnEvent::Phase(ConnPhase::Ready));
        };

        // 인증·호스트 키 실패는 다시 걸어도 같은 답이다
        let Some(delay) = (is_retryable(&err))
            .then(|| worker.retry.delay(attempt))
            .flatten()
        else {
            return worker.emit(ConnEvent::Phase(ConnPhase::Failed {
                detail: err.to_string(),
            }));
        };

        if !worker.log(
            LogKind::Status,
            format!(
                "연결에 실패해 {}초 뒤 다시 시도합니다 — {err}",
                delay.as_secs_f32()
            ),
        ) {
            return false;
        }
        // 기다리는 동안은 명령 채널을 보지 않는다 — 그 사이 연결이 닫히면 곧바로 접는다
        if !sleep_until_shutdown(&worker.shutdown, delay) {
            return false;
        }
        attempt += 1;
    }
}

/// 로그 이벤트를 만든다 — **비밀번호는 여기서 가려진다** (D14).
///
/// 버퍼에 쌓을 때만 가리면, 이벤트를 버퍼 대신 직접 소비하는 쪽(화면·상위 계층)에는
/// 평문이 그대로 흘러간다. 채널에 실리기 전에 한 번 가려 그 경로를 막는다.
fn log_event(kind: LogKind, text: String) -> ConnEvent {
    ConnEvent::Log {
        kind,
        text: mask_secrets(&text),
    }
}

/// 종료 신호를 살피며 잔다. 신호가 오면 그 자리에서 `false`를 돌려준다.
///
/// 재시도 대기는 최대 30초라 한 번에 자 버리면, 그동안 앱을 닫아도 워커가 소켓을 쥔 채
/// 그만큼 남는다. 잘게 나눠 자며 살펴 종료가 `STOP_GRACE` 안에 끝나게 한다.
fn sleep_until_shutdown(shutdown: &AtomicBool, total: Duration) -> bool {
    /// 살피는 간격 — `Drop`의 대기(200ms)보다 짧아야 그 안에 빠져나온다
    const SLICE: Duration = Duration::from_millis(20);
    let deadline = Instant::now() + total;
    loop {
        if shutdown.load(Ordering::SeqCst) {
            return false;
        }
        let left = deadline.saturating_duration_since(Instant::now());
        if left.is_zero() {
            return true;
        }
        std::thread::sleep(left.min(SLICE));
    }
}

/// 다시 걸어 볼 만한 실패인가.
///
/// 연결 계열(FTP 421 "지금은 안 된다", 일시적 네트워크 오류)만 다시 시도한다.
/// 인증·호스트 키·경로·권한 실패는 상황이 바뀌지 않으면 같은 답이 돌아온다
fn is_retryable(err: &RemoteError) -> bool {
    matches!(err, RemoteError::Connect { .. })
}

fn finish_op(worker: &Worker, op: OpKind, result: RemoteResult<()>) -> bool {
    worker.emit(ConnEvent::OpDone { op, result })
}

/// 전송 한 건 — 로컬 파일을 열고 세션에 스트림을 넘긴다 (NFR-12).
///
/// 진행률은 `PROGRESS_INTERVAL`마다 묶어 보내고, 취소 신호는 64KB 경계마다 본다.
fn run_transfer(worker: &mut Worker, request: TransferRequest) -> bool {
    worker.cancel.store(false, Ordering::SeqCst);
    let id = request.id;
    let result = transfer(worker, &request);
    let alive = worker.emit(ConnEvent::TransferDone { id, result });
    worker.cancel.store(false, Ordering::SeqCst);
    alive
}

fn transfer(worker: &mut Worker, request: &TransferRequest) -> RemoteResult<u64> {
    // 받기의 이어받기 지점은 **여기서** 정한다 — 받다 만 파일의 크기를 재는 것은
    // 파일시스템 호출이고, 그것을 화면 쪽에서 하면 프레임이 멈춘다 (AGENTS)
    let offset = match request.direction {
        TransferDirection::Download => {
            let done = std::fs::metadata(&request.local)
                .map(|meta| meta.len())
                .unwrap_or(0);
            crate::remote::transfer::resume_offset(done, request.remote_size)
        }
        TransferDirection::Upload => request.offset,
    };
    let mut progress = ThrottledProgress {
        tx: worker.tx.clone(),
        wake: Arc::clone(&worker.wake),
        id: request.id,
        cancel: Arc::clone(&worker.cancel),
        last_sent: Instant::now(),
        // 이어받는 중이면 화면이 보는 값은 **파일 전체 기준**이어야 한다 —
        // 이번에 옮긴 바이트만 올리면 진행률이 뒤로 튄다
        base: offset,
    };

    match request.direction {
        TransferDirection::Download => {
            let mut file = open_for_download(&request.local, offset)?;
            worker
                .session
                .download(&request.remote, &mut file, offset, &mut progress)
                .map(|moved| moved + offset)
        }
        TransferDirection::Upload => {
            let mut file = File::open(&request.local).map_err(local_error)?;
            if offset > 0 {
                file.seek(SeekFrom::Start(offset)).map_err(local_error)?;
            }
            worker
                .session
                .upload(&request.remote, &mut file, offset, &mut progress)
        }
    }
}

/// 받을 파일을 연다. 이어받기면 있던 파일을 자르지 않고 그 지점부터 이어 쓴다
fn open_for_download(local: &PathBuf, offset: u64) -> RemoteResult<File> {
    if offset == 0 {
        return File::create(local).map_err(local_error);
    }
    let mut file = OpenOptions::new()
        .write(true)
        .open(local)
        .map_err(local_error)?;
    file.seek(SeekFrom::Start(offset)).map_err(local_error)?;
    Ok(file)
}

/// 로컬 파일 쪽 실패도 전송 실패로 알린다 — 사용자에게는 같은 한 가지 일이다
fn local_error(err: std::io::Error) -> RemoteError {
    RemoteError::Transfer {
        detail: err.to_string(),
        transferred: 0,
    }
}

/// 진행률을 묶어 보내고 취소 신호를 전하는 통지
struct ThrottledProgress {
    tx: Sender<ConnEvent>,
    wake: Wake,
    id: TransferId,
    cancel: Arc<AtomicBool>,
    last_sent: Instant,
    /// 이미 받아 둔 만큼 — 보고 값에 더해 **파일 전체 기준**으로 올린다
    base: u64,
}

impl Progress for ThrottledProgress {
    fn report(&mut self, transferred: u64) -> bool {
        if self.last_sent.elapsed() >= PROGRESS_INTERVAL {
            self.last_sent = Instant::now();
            if self
                .tx
                .send(ConnEvent::TransferProgress {
                    id: self.id,
                    transferred: self.base + transferred,
                })
                .is_ok()
            {
                (self.wake)();
            }
        }
        !self.cancel.load(Ordering::SeqCst)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::remote::testing::{FakeServer, FakeSession, fake_entry};
    use crate::remote::types::SiteId;

    fn silent_wake() -> Wake {
        Arc::new(|| {})
    }

    fn fast_retry() -> RetryPolicy {
        // 테스트가 실제로 초 단위를 기다리지 않도록 눈금만 줄인다 — 규칙은 같다
        RetryPolicy {
            base: Duration::from_millis(1),
            max: Duration::from_millis(4),
            attempts: 5,
        }
    }

    fn site() -> SiteRecord {
        SiteRecord::new(SiteId(9), "가짜".to_owned())
    }

    fn spawn(server: &Arc<FakeServer>, retry: RetryPolicy) -> Connection {
        Connection::spawn(
            ConnectionId(1),
            site(),
            "비밀".to_owned(),
            Box::new(FakeSession::new(Arc::clone(server))),
            silent_wake(),
            retry,
        )
    }

    /// 테스트가 쓰는 임시 파일 — 프로세스 번호를 넣어 동시에 도는 다른 실행과 겹치지 않게 한다
    fn temp_path(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!("fe_t4_{label}_{}.bin", std::process::id()))
    }

    /// 조건이 참이 될 때까지 짧게 기다린다. 시간이 아니라 **관측된 상태**로 판정하기 위한 것이다
    fn wait_until(limit: Duration, mut done: impl FnMut() -> bool) -> bool {
        let deadline = Instant::now() + limit;
        while Instant::now() < deadline {
            if done() {
                return true;
            }
            std::thread::sleep(Duration::from_millis(2));
        }
        done()
    }

    /// 이벤트가 올 때까지 짧게 기다린다 — 워커가 다른 스레드라 즉시 오지 않는다
    fn wait_events(connection: &mut Connection, want: usize, limit: Duration) -> Vec<ConnEvent> {
        let deadline = Instant::now() + limit;
        let mut events = Vec::new();
        while events.len() < want && Instant::now() < deadline {
            events.extend(connection.poll());
            if events.len() < want {
                std::thread::sleep(Duration::from_millis(2));
            }
        }
        events
    }

    #[test]
    fn 로그_이벤트는_채널에_실리기_전에_가려진다() {
        // 버퍼에 쌓을 때만 가리면, 이벤트를 직접 소비하는 쪽에 평문이 흘러간다 (D14)
        let ConnEvent::Log { text, .. } =
            log_event(LogKind::Command, "PASS 진짜비밀번호".to_owned())
        else {
            panic!("로그 이벤트가 아니다");
        };
        assert_eq!(text, "PASS ******");

        let ConnEvent::Log { text, .. } = log_event(
            LogKind::Status,
            "sftp://deploy:p@ss@example.test:22 에 연결".to_owned(),
        ) else {
            panic!("로그 이벤트가 아니다");
        };
        assert!(!text.contains("p@ss"), "비밀번호가 남았다: {text}");
    }

    #[test]
    fn 백오프는_두_배씩_늘고_상한에서_멈추며_횟수를_넘기면_포기한다() {
        let policy = RetryPolicy::DEFAULT;
        assert_eq!(policy.delay(0), Some(Duration::from_secs(1)));
        assert_eq!(policy.delay(1), Some(Duration::from_secs(2)));
        assert_eq!(policy.delay(2), Some(Duration::from_secs(4)));
        assert_eq!(policy.delay(3), Some(Duration::from_secs(8)));
        assert_eq!(policy.delay(4), Some(Duration::from_secs(16)));
        // 5회를 넘기면 포기한다
        assert_eq!(policy.delay(5), None);

        // 상한을 넘는 지연은 30초에서 잘린다
        let long = RetryPolicy {
            attempts: 10,
            ..RetryPolicy::DEFAULT
        };
        assert_eq!(long.delay(5), Some(Duration::from_secs(30)));
        assert_eq!(long.delay(9), Some(Duration::from_secs(30)));
    }

    #[test]
    fn 워커는_명령을_처리하고_이벤트를_돌려준다() {
        let server = FakeServer::new();
        server.set_entries("/pub", vec![fake_entry("a.txt", false)]);
        let mut connection = spawn(&server, fast_retry());

        connection.send(ConnCommand::Connect);
        connection.send(ConnCommand::List {
            generation: 1,
            path: RemotePath::new("/pub"),
        });
        let events = wait_events(&mut connection, 3, Duration::from_secs(2));

        assert!(events.contains(&ConnEvent::Phase(ConnPhase::Connecting)));
        assert!(events.contains(&ConnEvent::Phase(ConnPhase::Ready)));
        let listed = events
            .iter()
            .find_map(|event| match event {
                ConnEvent::Listed {
                    generation,
                    entries,
                    ..
                } => Some((*generation, entries.len())),
                _ => None,
            })
            .expect("목록 이벤트가 없다");
        assert_eq!(listed, (1, 1));
        assert_eq!(*connection.phase(), ConnPhase::Ready);
    }

    #[test]
    fn 연결이_사라지면_워커_스레드가_회수된다() {
        let server = FakeServer::new();
        {
            let mut connection = spawn(&server, fast_retry());
            connection.send(ConnCommand::Connect);
            wait_events(&mut connection, 2, Duration::from_secs(2));
            assert_eq!(server.live_sessions(), 1);
        }
        // `Drop`이 `Stop`을 보내고 짧게 기다린다 — 세션도 함께 사라진다
        let deadline = Instant::now() + Duration::from_secs(2);
        while server.live_sessions() > 0 && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(5));
        }
        assert_eq!(server.live_sessions(), 0, "워커가 회수되지 않았다");
    }

    #[test]
    fn 늦게_도착한_이전_세대의_목록은_버려진다() {
        // 세대 번호는 워커가 실어 돌려주고, 화면 쪽이 지금 세대와 견줘 버린다
        let server = FakeServer::new();
        server.set_entries("/old", vec![fake_entry("old.txt", false)]);
        server.set_entries("/new", vec![fake_entry("new.txt", false)]);
        let mut connection = spawn(&server, fast_retry());
        connection.send(ConnCommand::Connect);
        connection.send(ConnCommand::List {
            generation: 1,
            path: RemotePath::new("/old"),
        });
        connection.send(ConnCommand::List {
            generation: 2,
            path: RemotePath::new("/new"),
        });
        let events = wait_events(&mut connection, 4, Duration::from_secs(2));

        let current = 2;
        let kept: Vec<&RemotePath> = events
            .iter()
            .filter_map(|event| match event {
                ConnEvent::Listed {
                    generation, path, ..
                } if *generation == current => Some(path),
                _ => None,
            })
            .collect();
        let dropped = events
            .iter()
            .filter(|event| {
                matches!(event, ConnEvent::Listed { generation, .. } if *generation != current)
            })
            .count();
        assert_eq!(kept.len(), 1, "지금 세대의 결과만 남아야 한다");
        assert_eq!(kept[0].as_str(), "/new");
        assert_eq!(
            dropped, 1,
            "이전 세대의 결과가 함께 돌아와야 한다(버릴 대상)"
        );
    }

    #[test]
    fn 한_연결이_막혀도_다른_연결은_계속_처리된다() {
        // NFR-11의 핵심 근거 — 연결마다 스레드가 따로라 서로를 막지 않는다
        let blocked_server = FakeServer::new();
        let live_server = FakeServer::new();
        live_server.set_entries("/", vec![fake_entry("살아있음", false)]);

        let mut blocked = spawn(&blocked_server, fast_retry());
        let mut live = Connection::spawn(
            ConnectionId(2),
            site(),
            "비밀".to_owned(),
            Box::new(FakeSession::new(Arc::clone(&live_server))),
            silent_wake(),
            fast_retry(),
        );
        live.send(ConnCommand::Connect);
        wait_events(&mut live, 2, Duration::from_secs(2));

        // 한쪽 서버를 응답 없는 상태로 만들고 명령을 밀어 넣는다
        blocked_server.set_hang(true);
        blocked.send(ConnCommand::Connect);
        blocked.send(ConnCommand::List {
            generation: 1,
            path: RemotePath::root(),
        });
        std::thread::sleep(Duration::from_millis(30));
        assert!(
            !blocked
                .poll()
                .iter()
                .any(|event| matches!(event, ConnEvent::Listed { .. })),
            "막힌 연결이 목록을 내면 안 된다"
        );

        // 그 사이에도 다른 연결은 정상 처리된다
        live.send(ConnCommand::List {
            generation: 1,
            path: RemotePath::root(),
        });
        let events = wait_events(&mut live, 1, Duration::from_secs(2));
        assert!(
            events
                .iter()
                .any(|event| matches!(event, ConnEvent::Listed { .. })),
            "막히지 않은 연결은 계속 답해야 한다"
        );

        // 테스트가 끝나도록 풀어 준다
        blocked_server.set_hang(false);
    }

    #[test]
    fn 다시_걸어_볼_만한_실패만_재시도한다() {
        assert!(is_retryable(&RemoteError::Connect {
            detail: "421 Too many connections".to_owned()
        }));
        // 비밀번호가 틀린 것은 다시 걸어도 같다
        assert!(!is_retryable(&RemoteError::Auth {
            detail: "530".to_owned()
        }));
        assert!(!is_retryable(&RemoteError::HostKey {
            detail: "지문 불일치".to_owned()
        }));
    }

    #[test]
    fn 거절당한_연결은_백오프로_다시_시도한다() {
        let server = FakeServer::new();
        server.fail_connects(2);
        let mut connection = spawn(&server, fast_retry());
        connection.send(ConnCommand::Connect);
        let events = wait_events(&mut connection, 5, Duration::from_secs(3));

        assert!(
            events.contains(&ConnEvent::Phase(ConnPhase::Ready)),
            "두 번 거절당한 뒤 연결되어야 한다: {events:?}"
        );
        let attempts = server
            .calls()
            .iter()
            .filter(|name| name.as_str() == "connect")
            .count();
        assert_eq!(attempts, 3);
    }

    #[test]
    fn 상한까지_실패하면_포기하고_실패_단계로_간다() {
        let server = FakeServer::new();
        server.fail_connects(99);
        let mut connection = spawn(&server, fast_retry());
        connection.send(ConnCommand::Connect);
        let events = wait_events(&mut connection, 12, Duration::from_secs(3));

        assert!(
            events
                .iter()
                .any(|event| matches!(event, ConnEvent::Phase(ConnPhase::Failed { .. }))),
            "포기하고 실패 단계로 가야 한다: {events:?}"
        );
        // 처음 1회 + 재시도 5회
        let attempts = server
            .calls()
            .iter()
            .filter(|name| name.as_str() == "connect")
            .count();
        assert_eq!(attempts, 6);
    }

    #[test]
    fn 탐색과_전송이_한_연결에서_차례로_처리된다() {
        // D4의 겸용 상태(M=1) — 한 워커가 둘을 직렬로 처리하고 어느 쪽도 잃지 않는다
        let server = FakeServer::new();
        server.set_entries("/", vec![fake_entry("a.txt", false)]);
        server.set_download_size(4 * 1024);
        let mut connection = spawn(&server, fast_retry());
        let local = temp_path("직렬_확인");

        connection.send(ConnCommand::Connect);
        connection.send(ConnCommand::List {
            generation: 1,
            path: RemotePath::root(),
        });
        connection.send(ConnCommand::Transfer(TransferRequest {
            id: TransferId(7),
            direction: TransferDirection::Download,
            remote: RemotePath::new("/a.txt"),
            local: local.clone(),
            offset: 0,
            remote_size: 0,
        }));
        connection.send(ConnCommand::List {
            generation: 2,
            path: RemotePath::root(),
        });
        let events = wait_events(&mut connection, 5, Duration::from_secs(3));

        let listed = events
            .iter()
            .filter(|event| matches!(event, ConnEvent::Listed { .. }))
            .count();
        let done = events.iter().any(
            |event| matches!(event, ConnEvent::TransferDone { id, result } if *id == TransferId(7) && result.is_ok()),
        );
        assert_eq!(listed, 2, "탐색 두 건이 모두 살아 있어야 한다: {events:?}");
        assert!(done, "전송도 끝나야 한다: {events:?}");
        // 명령이 뒤섞이지 않고 보낸 순서대로 처리됐다
        let order: Vec<String> = server
            .calls()
            .into_iter()
            .filter(|name| name == "list" || name == "download")
            .collect();
        assert_eq!(order, vec!["list", "download", "list"]);

        drop(connection);
        let _ = std::fs::remove_file(local);
    }

    #[test]
    fn 취소하면_전송이_그_자리에서_멈춘다() {
        let server = FakeServer::new();
        server.set_download_size(8 * 1024 * 1024);
        let mut connection = spawn(&server, fast_retry());
        let local = temp_path("취소_확인");

        connection.send(ConnCommand::Connect);
        wait_events(&mut connection, 2, Duration::from_secs(2));

        // **기계 속도에 기대지 않는다** — 서버를 응답 없는 상태로 두어 전송을 들어간 자리에
        // 세워 놓고, 취소 신호를 준 뒤에 풀어 준다. 그래야 "얼마나 빨리 옮기느냐"와 무관하게
        // 취소가 반드시 전송 도중에 도착한다
        server.set_hang(true);
        connection.send(ConnCommand::Transfer(TransferRequest {
            id: TransferId(1),
            direction: TransferDirection::Download,
            remote: RemotePath::new("/big.bin"),
            local: local.clone(),
            offset: 0,
            remote_size: 0,
        }));
        wait_until(Duration::from_secs(2), || {
            server.calls().iter().any(|name| name == "download")
        });
        connection.cancel_transfer();
        server.set_hang(false);

        let events = wait_events(&mut connection, 1, Duration::from_secs(3));
        let cancelled = events.iter().any(|event| {
            matches!(
                event,
                ConnEvent::TransferDone {
                    result: Err(RemoteError::Cancelled),
                    ..
                }
            )
        });
        assert!(cancelled, "취소로 끝나야 한다: {events:?}");

        drop(connection);
        let _ = std::fs::remove_file(local);
    }

    #[test]
    fn 재시도를_기다리는_중에_닫아도_곧바로_회수된다() {
        // 백오프 대기는 기본값이 최대 30초다 — 그 사이 앱을 닫았는데 워커가 소켓을 쥔 채
        // 그만큼 남으면, 사용자에게는 종료가 걸린 것으로 보인다
        let server = FakeServer::new();
        server.fail_connects(99);
        let slow_retry = RetryPolicy {
            base: Duration::from_secs(5),
            max: Duration::from_secs(5),
            attempts: 5,
        };
        let started = Instant::now();
        {
            let connection = spawn(&server, slow_retry);
            connection.send(ConnCommand::Connect);
            // 첫 시도가 실패해 대기에 들어갈 때까지 기다린다
            wait_until(Duration::from_secs(2), || {
                server.calls().iter().any(|name| name == "connect")
            });
        }
        wait_until(Duration::from_secs(3), || server.live_sessions() == 0);
        assert_eq!(server.live_sessions(), 0, "워커가 회수되지 않았다");
        assert!(
            started.elapsed() < Duration::from_secs(4),
            "백오프 대기(5초)를 다 기다린 뒤에야 끝났다: {:?}",
            started.elapsed()
        );
    }

    #[test]
    fn 워커가_죽은_뒤의_명령은_조용히_무시된다() {
        let server = FakeServer::new();
        let mut connection = spawn(&server, fast_retry());
        connection.send(ConnCommand::Stop);
        let deadline = Instant::now() + Duration::from_secs(2);
        while !connection.is_finished() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(5));
        }
        assert!(connection.is_finished(), "워커가 끝나야 한다");
        // 워커가 사라지면 수신부도 함께 사라져 `send`가 실패한다 —
        // 호출부는 이 `false`를 보고 조용히 넘어간다(오류 대화를 띄우지 않는다)
        assert!(!connection.send(ConnCommand::Cancel));
        assert!(connection.poll().is_empty());
    }
}
