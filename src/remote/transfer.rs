//! 전송 실행기 — 큐(T17)와 연결 워커(T4)를 잇는 고리 (FR-37).
//!
//! 큐는 "무엇을 보낼지"만 알고 워커는 "어떻게 보낼지"만 안다. 그 사이에서 **누가 언제 어느
//! 연결로 보낼지**를 정하는 것이 여기다:
//!
//! - 사이트마다 배정된 전송 자리(D4)만큼만 동시에 시작한다
//! - 다운로드는 **`<이름>.part`로 받고 끝나면 이름을 바꾼다** — 받다 만 파일이 완성본처럼
//!   보이면 사용자가 그것을 열어 본다 (Acceptance ⑤)
//! - 끊긴 전송은 `.part` 크기에서 **이어받는다**. 사용자가 스스로 취소한 것은 `.part`를 지운다
//! - 속도는 여기서 잰다 — 워커는 누적 바이트만 올린다(시계를 두 곳에 두면 값이 갈린다)
//!
//! **I/O는 하지 않는다**(`.part` 이름 바꾸기·지우기 제외) — 실제 바이트는 워커 스레드가 옮긴다.
use std::collections::{HashMap, HashSet};
use std::ffi::OsString;
use std::path::{Path, PathBuf};

use crate::remote::connection::{
    ConnCommand, ConnectionId, TransferDirection, TransferId, TransferRequest,
};
use crate::remote::manager::ConnectionManager;
use crate::remote::queue::{TransferQueue, TransferState};
use crate::remote::sites::SiteStore;
use crate::remote::types::SiteId;

/// 받는 중인 파일에 붙는 꼬리 (Acceptance ⑤)
const PART_SUFFIX: &str = ".part";

/// 이어받기 시작점을 정한다.
///
/// - 받다 만 것이 **원격보다 크거나 같으면** 처음부터 받는다 — 서버 쪽 파일이 바뀐 것이라
///   이어 붙이면 뒤섞인 파일이 된다 (plan Edge Case)
/// - 원격 크기를 모르면(0) 이어받지 않는다 — 어디까지가 맞는지 확인할 길이 없다
pub fn resume_offset(local_size: u64, remote_size: u64) -> u64 {
    if remote_size == 0 || local_size >= remote_size {
        return 0;
    }
    local_size
}

/// 받는 중인 파일의 임시 이름 — `report.zip` → `report.zip.part`
pub fn part_path(local: &Path) -> PathBuf {
    let mut name = local.as_os_str().to_os_string();
    name.push(OsString::from(PART_SUFFIX));
    PathBuf::from(name)
}

/// 진행 보고를 속도로 바꾸는 계산기.
///
/// 워커는 누적 바이트만 올린다(100ms마다 — `connection.rs`의 `PROGRESS_INTERVAL`). 속도는
/// **직전 보고와의 차이 ÷ 걸린 시간**이며, 시간을 인자로 받아 시계를 하나로 둔다
/// (`Instant`를 쓰면 테스트가 실제로 기다려야 한다 — 토스트와 같은 판단).
#[derive(Debug, Clone, Copy, PartialEq)]
struct ProgressSink {
    last_bytes: u64,
    last_at: f64,
    /// 마지막으로 잰 속도 — 보고 간격이 너무 짧아 재지 못하면 이 값을 유지한다
    speed: u64,
}

impl ProgressSink {
    fn new(now: f64) -> ProgressSink {
        ProgressSink {
            last_bytes: 0,
            last_at: now,
            speed: 0,
        }
    }

    /// 새 누적 바이트를 받아 속도(바이트/초)를 갱신한다
    fn observe(&mut self, transferred: u64, now: f64) -> u64 {
        let elapsed = now - self.last_at;
        // 같은 프레임에 두 번 오면 나눌 수 없다 — 직전 속도를 유지한다
        if elapsed > 0.0 {
            let delta = transferred.saturating_sub(self.last_bytes);
            self.speed = (delta as f64 / elapsed) as u64;
            self.last_at = now;
            self.last_bytes = transferred;
        }
        self.speed
    }
}

/// 워커에 맡긴 전송 한 건의 기록 — 끝났을 때 무엇을 정리할지 여기에 있다
#[derive(Debug, Clone)]
struct Assignment {
    conn: ConnectionId,
    /// 완성됐을 때 놓일 자리
    final_path: PathBuf,
    /// 지금 쓰고 있는 자리 — 다운로드는 `.part`, 업로드는 원본 그대로다
    working_path: PathBuf,
    direction: TransferDirection,
    progress: ProgressSink,
}

/// 큐를 실제 전송으로 옮기는 실행기 (FR-37).
///
/// 상태를 큐와 나눠 갖지 않는다 — 항목의 진행·결과는 전부 큐에 쓰고, 여기에는 **워커에
/// 맡긴 것과 그 뒤처리 정보**만 둔다. 두 곳에 같은 상태를 두면 어긋난 순간이 화면에 보인다.
#[derive(Debug, Default)]
pub struct TransferRunner {
    assigned: HashMap<TransferId, Assignment>,
    /// 취소를 알렸고 **워커가 파일을 놓기를 기다리는** 것들 — 어느 연결에 맡겼는지와
    /// 받다 만 파일의 자리를 함께 든다.
    ///
    /// 그 자리에서 곧바로 지우지 않는 이유: 워커는 아직 그 파일을 열어 쓰고 있고(취소 신호는
    /// 64KB마다 살펴진다), Windows는 열려 있는 파일을 지우지 못한다. 지운 셈 치고 넘어가면
    /// 받다 만 파일이 그대로 남는다.
    ///
    /// **연결까지 드는 이유**: 취소 직후 그 연결이 닫히면 워커의 `TransferDone`이 채널째로
    /// 버려져 `on_done`이 영영 불리지 않는다 — 그때는 `forget_connection`이 이 자리를 넘겨받아
    /// 지운다(연결이 사라졌다는 것은 파일도 이미 놓였다는 뜻이다)
    cancelling: HashMap<TransferId, (ConnectionId, PathBuf)>,
    /// 지울 차례가 됐지만 아직 못 지운 `.part`들 — 백신 검사·핸들 지연으로 한 번에 실패할 수 있어
    /// 매 `start_ready`마다 다시 시도한다. 조용히 삼키면 받다 만 파일이 영영 남는다
    pending_delete: Vec<PathBuf>,
}

impl TransferRunner {
    pub fn new() -> TransferRunner {
        TransferRunner::default()
    }

    /// 지금 워커가 붙들고 있는 전송 수 — 화면·테스트가 진행 여부를 볼 때 쓴다
    pub fn in_flight(&self) -> usize {
        self.assigned.len()
    }

    /// 대기 중인 항목을 자리가 나는 대로 워커에 맡긴다.
    ///
    /// **연결이 없는 사이트는 건너뛴다** — 큐에 남아 있다가 다시 연결되면 그때 나간다.
    /// 한 연결은 한 번에 한 건만 옮긴다(워커가 명령을 차례로 처리하므로, 두 건을 한 연결에
    /// 밀어 넣으면 뒤엣것이 앞엣것을 기다리며 "진행 중"으로 잘못 보인다).
    pub fn start_ready(
        &mut self,
        queue: &mut TransferQueue,
        manager: &ConnectionManager,
        sites: &SiteStore,
        now: f64,
    ) {
        // 지우지 못한 `.part`가 있으면 먼저 다시 시도한다 — 매 프레임 불리는 자리라
        // 파일을 붙들고 있던 쪽이 놓는 순간 곧바로 정리된다
        self.sweep_pending_delete();
        if queue.is_paused() {
            return;
        }
        // 사이트별로 쓸 수 있는 연결을 모은다
        let mut idle: HashMap<SiteId, Vec<ConnectionId>> = HashMap::new();
        let busy: HashSet<ConnectionId> = self.assigned.values().map(|a| a.conn).collect();
        for id in manager.ids() {
            if busy.contains(id) {
                continue;
            }
            if let Some(connection) = manager.get(*id) {
                idle.entry(connection.site).or_default().push(*id);
            }
        }

        for (site, mut conns) in idle {
            let Some(record) = sites.get(site) else {
                continue;
            };
            // 자리 수와 실제로 노는 연결 수 중 **작은 쪽**이 이번에 시작할 수 있는 최대다
            let slots = manager.transfer_slots(record).max(1) as usize;
            let ready = queue.next_for(site, slots as u8);
            for id in ready {
                let Some(conn) = conns.pop() else {
                    break;
                };
                let Some(item) = queue.get(id) else {
                    continue;
                };
                let (working_path, offset) = match item.direction {
                    // 받다 만 것이 있으면 그 크기에서 이어받는다
                    TransferDirection::Download => {
                        let part = part_path(&item.local);
                        let done = std::fs::metadata(&part).map(|m| m.len()).unwrap_or(0);
                        (part, resume_offset(done, item.size))
                    }
                    // 올리기는 원본을 읽기만 한다 — 이어 올리기 지점은 서버가 가진 크기라
                    // 큐가 담아 둔 값(`size`)이 아니라 워커가 APPE로 처리한다 (T2 결정)
                    TransferDirection::Upload => (item.local.clone(), 0),
                };
                let request = TransferRequest {
                    id,
                    direction: item.direction,
                    remote: item.remote.clone(),
                    local: working_path.clone(),
                    offset,
                };
                if !manager.send(conn, ConnCommand::Transfer(request)) {
                    // 워커가 죽었다 — 대기로 두면 다음 연결에서 다시 나간다
                    continue;
                }
                self.assigned.insert(
                    id,
                    Assignment {
                        conn,
                        final_path: item.local.clone(),
                        working_path,
                        direction: item.direction,
                        progress: ProgressSink::new(now),
                    },
                );
                queue.update(
                    id,
                    TransferState::Active {
                        sent: offset,
                        speed: 0,
                    },
                );
            }
        }
    }

    /// 워커가 올린 진행 보고를 큐에 반영한다 — 속도는 여기서 잰다
    pub fn on_progress(&mut self, queue: &mut TransferQueue, id: TransferId, sent: u64, now: f64) {
        let Some(assignment) = self.assigned.get_mut(&id) else {
            return;
        };
        let speed = assignment.progress.observe(sent, now);
        queue.update(id, TransferState::Active { sent, speed });
    }

    /// 전송이 끝났다 — 성공이면 `.part`를 제 이름으로 바꾸고, 실패면 그대로 남겨 다음에 이어받는다.
    ///
    /// 실패한 `.part`를 지우지 않는 이유가 이어받기의 전부다(Acceptance ③) — 지우면
    /// 재시도가 언제나 처음부터가 된다
    pub fn on_done(
        &mut self,
        queue: &mut TransferQueue,
        id: TransferId,
        result: Result<u64, String>,
    ) {
        // 사용자가 그만둔 것이면 이제야 파일을 지울 수 있다 — 워커가 방금 놓았다
        if let Some((_, part)) = self.cancelling.remove(&id) {
            self.pending_delete.push(part);
            self.sweep_pending_delete();
            return;
        }
        let Some(assignment) = self.assigned.remove(&id) else {
            return;
        };
        match result {
            Ok(_) => {
                if assignment.direction == TransferDirection::Download
                    && let Err(err) =
                        std::fs::rename(&assignment.working_path, &assignment.final_path)
                {
                    // 옮기지 못하면 완료가 아니다 — `.part`가 남아 있어 다시 걸 수 있다
                    queue.update(
                        id,
                        TransferState::Error {
                            message: format!("받은 파일을 제자리에 두지 못했습니다: {err}"),
                        },
                    );
                    return;
                }
                queue.update(id, TransferState::Done);
            }
            Err(message) => queue.update(id, TransferState::Error { message }),
        }
    }

    /// 사용자가 그만뒀다 — 워커를 멈추고 **받다 만 파일을 지운다** (Acceptance ⑤).
    ///
    /// 큐에서 항목을 빼는 것은 `TransferQueue::cancel`이 한다. 여기서는 워커와 파일만 정리한다
    pub fn cancel(&mut self, manager: &ConnectionManager, id: TransferId) {
        let Some(assignment) = self.assigned.remove(&id) else {
            return;
        };
        manager.send(assignment.conn, ConnCommand::Cancel);
        if assignment.direction == TransferDirection::Download {
            // 워커가 놓을 때까지 기다렸다 지운다 (`cancelling` 주석 참조)
            self.cancelling
                .insert(id, (assignment.conn, assignment.working_path));
        }
    }

    /// 아직 정리하지 못한 `.part` 건수 — 기다리는 것과 지우기를 다시 시도할 것을 합친다
    pub fn pending_cleanup(&self) -> usize {
        self.cancelling.len() + self.pending_delete.len()
    }

    /// 지울 차례가 된 `.part`를 지운다. **못 지운 것은 목록에 남겨 다음에 다시 시도한다** —
    /// 이미 사라진 것은 성공으로 본다(다른 쪽에서 치웠거나 애초에 만들어지지 않았다)
    fn sweep_pending_delete(&mut self) {
        self.pending_delete
            .retain(|part| part.exists() && std::fs::remove_file(part).is_err());
    }

    /// `⏸` — 진행 중인 전송을 멈추고 **대기로 되돌린다** (Acceptance ④).
    ///
    /// 취소와 다르다: `.part`를 **남긴다.** 다시 누르면 그 크기에서 이어받으므로
    /// 사용자가 보기에 "멈췄다 이어간다"가 된다
    pub fn pause(&mut self, queue: &mut TransferQueue, manager: &ConnectionManager) {
        queue.set_paused(true);
        for (id, assignment) in self.assigned.drain() {
            manager.send(assignment.conn, ConnCommand::Cancel);
            queue.update(id, TransferState::Wait);
        }
    }

    /// `⏸`를 다시 눌렀다 — 다음 `start_ready`가 이어받는다
    pub fn resume(&mut self, queue: &mut TransferQueue) {
        queue.set_paused(false);
    }

    /// 앱이 닫힌다 — 진행 중이던 것을 **대기로 되돌리고** 취소분의 `.part`를 마지막으로 치운다
    /// (plan Edge Case: 전송 중 앱 종료 → `.part` 정리).
    ///
    /// **진행 중이던 `.part`는 지우지 않는다** — 큐는 저장돼 다음 실행에서 그대로 복원되고(T25),
    /// 사용자가 다시 걸면 그 조각에서 이어받는다. 여기서 지우면 받아 둔 것을 매번 버리게 된다.
    /// 대신 상태를 `Active`로 남기지 않는다: 저장된 큐가 "전송 중"이라고 주장하면 다음 실행의
    /// 화면이 실제로는 아무것도 돌지 않는데 진행 중으로 보인다.
    ///
    /// 호출부는 앱의 종료 처리(`ExplorerApp::on_exit`)다 — 실행기를 앱에 배선하는 T19가 잇는다
    pub fn shutdown(&mut self, queue: &mut TransferQueue) {
        for (id, _) in self.assigned.drain() {
            queue.update(id, TransferState::Wait);
        }
        // 취소해 두고 아직 못 치운 조각은 지금이 마지막 기회다 — 워커는 이미 멈췄다
        let leftovers: Vec<PathBuf> = self.cancelling.drain().map(|(_, (_, part))| part).collect();
        self.pending_delete.extend(leftovers);
        self.sweep_pending_delete();
    }

    /// 연결이 사라졌다 — 그 연결에 맡겼던 전송을 놓아준다.
    /// 큐 쪽 되돌리기는 `TransferQueue::requeue_site`가 한다.
    ///
    /// **취소 뒤 정리를 기다리던 것도 여기서 거둔다** — 연결이 닫히면 워커의 완료 통지가
    /// 채널째로 버려져 `on_done`이 불리지 않는다. 그대로 두면 받다 만 파일이 영영 남는다
    pub fn forget_connection(&mut self, conn: ConnectionId) {
        self.assigned
            .retain(|_, assignment| assignment.conn != conn);
        let orphaned: Vec<TransferId> = self
            .cancelling
            .iter()
            .filter(|(_, (owner, _))| *owner == conn)
            .map(|(id, _)| *id)
            .collect();
        for id in orphaned {
            if let Some((_, part)) = self.cancelling.remove(&id) {
                self.pending_delete.push(part);
            }
        }
        self.sweep_pending_delete();
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    use super::*;
    use crate::remote::connection::{ConnEvent, ConnPhase};
    use crate::remote::testing::{FakeServer, FakeSession, pattern_byte};
    use crate::remote::types::{RemotePath, RemoteSession, SiteRecord};

    fn temp_dir() -> PathBuf {
        let dir = std::env::temp_dir().join("file_explorer_transfer_tests");
        let _ = std::fs::create_dir_all(&dir);
        dir
    }

    /// 같은 이름이 겹치지 않게 번호를 붙인 임시 파일 경로
    fn temp_file(tag: &str) -> PathBuf {
        let unique = format!(
            "{tag}_{}_{:?}.bin",
            std::process::id(),
            std::thread::current().id()
        );
        temp_dir().join(unique)
    }

    fn manager_with_site(server: &Arc<FakeServer>) -> (ConnectionManager, SiteStore, SiteId) {
        let mut sites = SiteStore::new();
        let site = sites.add("가짜 서버");
        if let Some(record) = sites.get_mut(site) {
            record.host = "example.test".to_owned();
        }
        let record: SiteRecord = sites.get(site).expect("사이트").clone();
        let mut manager = ConnectionManager::new(Arc::new(|| {}));
        let session: Box<dyn RemoteSession> = Box::new(FakeSession::new(Arc::clone(server)));
        manager.open(&record, String::new(), session);
        (manager, sites, site)
    }

    /// 이벤트가 올 때까지 짧게 기다린다 — 워커가 다른 스레드라 즉시 오지 않는다
    fn drain_until<F>(
        manager: &mut ConnectionManager,
        limit: Duration,
        mut done: F,
    ) -> Vec<ConnEvent>
    where
        F: FnMut(&[ConnEvent]) -> bool,
    {
        let deadline = Instant::now() + limit;
        let mut events = Vec::new();
        while Instant::now() < deadline {
            events.extend(manager.poll().into_iter().map(|(_, event)| event));
            if done(&events) {
                break;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        events
    }

    #[test]
    fn 이어받기_시작점은_받다_만_크기다() {
        // Acceptance ③의 계산 부분
        assert_eq!(resume_offset(0, 1000), 0);
        assert_eq!(resume_offset(400, 1000), 400);
        // 받다 만 것이 더 크면 서버 쪽이 바뀐 것이다 — 이어 붙이면 뒤섞인 파일이 된다
        assert_eq!(resume_offset(1200, 1000), 0);
        assert_eq!(resume_offset(1000, 1000), 0);
        // 원격 크기를 모르면 이어받지 않는다
        assert_eq!(resume_offset(500, 0), 0);
    }

    #[test]
    fn 받는_중에는_part_이름을_쓴다() {
        // Acceptance ⑤ — 받다 만 파일이 완성본처럼 보이면 사용자가 그것을 열어 본다
        let path = PathBuf::from(r"C:\down\report.zip");
        assert_eq!(part_path(&path), PathBuf::from(r"C:\down\report.zip.part"));
        // 확장자가 없어도 뒤에 붙는다
        assert_eq!(
            part_path(&PathBuf::from(r"C:\down\LICENSE")),
            PathBuf::from(r"C:\down\LICENSE.part")
        );
    }

    #[test]
    fn 속도는_직전_보고와의_차이로_잰다() {
        let mut sink = ProgressSink::new(0.0);
        // 1초에 1000바이트
        assert_eq!(sink.observe(1000, 1.0), 1000);
        // 0.5초에 500바이트 → 여전히 초당 1000
        assert_eq!(sink.observe(1500, 1.5), 1000);
        // 시간이 흐르지 않았으면 직전 값을 유지한다(0으로 나누지 않는다)
        assert_eq!(sink.observe(2000, 1.5), 1000);
    }

    #[test]
    fn 큐의_대기_항목이_연결로_나가_전송된다() {
        // Acceptance ① — 200KB를 받으면 진행 보고가 오고 마지막이 100%다
        let server = FakeServer::new();
        server.set_download_size(200 * 1024);
        let (mut manager, sites, site) = manager_with_site(&server);
        let mut queue = TransferQueue::new();
        let mut runner = TransferRunner::new();
        let local = temp_file("download");
        let _ = std::fs::remove_file(&local);
        let _ = std::fs::remove_file(part_path(&local));

        let id = queue.enqueue(
            site,
            TransferDirection::Download,
            local.clone(),
            RemotePath::new("/big.bin"),
            200 * 1024,
        );
        // 연결이 설 때까지 기다린 뒤 배정한다
        let _ = drain_until(&mut manager, Duration::from_secs(3), |events| {
            events
                .iter()
                .any(|event| matches!(event, ConnEvent::Phase(ConnPhase::Ready)))
        });
        runner.start_ready(&mut queue, &manager, &sites, 0.0);
        assert_eq!(runner.in_flight(), 1, "전송이 워커로 나가지 않았다");
        assert!(queue.get(id).expect("항목").state.is_active());

        // 진행·완료 이벤트를 큐에 반영한다
        let mut now = 0.0;
        let deadline = Instant::now() + Duration::from_secs(10);
        while Instant::now() < deadline && runner.in_flight() > 0 {
            for (_, event) in manager.poll() {
                now += 0.1;
                match event {
                    ConnEvent::TransferProgress { id, transferred } => {
                        runner.on_progress(&mut queue, id, transferred, now);
                    }
                    ConnEvent::TransferDone { id, result } => {
                        runner.on_done(&mut queue, id, result.map_err(|e| e.to_string()));
                    }
                    _ => {}
                }
            }
            std::thread::sleep(Duration::from_millis(5));
        }

        assert!(
            queue.get(id).expect("항목").state.is_done(),
            "전송이 끝나지 않았다: {:?}",
            queue.get(id).map(|item| &item.state)
        );
        assert_eq!(queue.get(id).expect("항목").progress(), Some(1.0));
        // `.part`는 사라지고 제 이름의 파일이 남는다 (Acceptance ⑤)
        assert!(local.exists(), "받은 파일이 제자리에 없다");
        assert!(!part_path(&local).exists(), "`.part`가 남았다");
        assert_eq!(
            std::fs::metadata(&local).expect("파일").len(),
            200 * 1024,
            "받은 크기가 다르다"
        );
        let _ = std::fs::remove_file(&local);
    }

    #[test]
    fn 가짜_세션에서_200kb를_받으면_진행_보고가_네_번_이상_온다() {
        // Acceptance ① — 64KB마다 보고하므로 200KB면 네 번이다(D12).
        // 워커를 거치면 100ms 간격으로 묶여 나가므로(`PROGRESS_INTERVAL`) 그 아래층인
        // 세션에서 잰다 — 묶기는 화면을 위한 것이고, 이 단언은 스트리밍 자체를 본다
        struct Counting {
            calls: usize,
            last: u64,
        }
        impl crate::remote::types::Progress for Counting {
            fn report(&mut self, transferred: u64) -> bool {
                self.calls += 1;
                self.last = transferred;
                true
            }
        }

        let server = FakeServer::new();
        server.set_download_size(200 * 1024);
        let mut session = FakeSession::new(Arc::clone(&server));
        let record = SiteRecord::new(SiteId(0), "가짜".to_owned());
        session.connect(&record).expect("연결");
        let mut sink: Vec<u8> = Vec::new();
        let mut progress = Counting { calls: 0, last: 0 };

        let moved = session
            .download(&RemotePath::new("/big.bin"), &mut sink, 0, &mut progress)
            .expect("전송");
        assert_eq!(moved, 200 * 1024);
        assert!(
            progress.calls >= 4,
            "64KB마다 보고해야 한다 (실제 {}회)",
            progress.calls
        );
        assert_eq!(progress.last, 200 * 1024, "마지막 보고가 100%가 아니다");
    }

    #[test]
    fn 이어받은_파일이_원본과_바이트까지_같다() {
        // Acceptance ③ — plan이 "결과가 원본과 같다(해시 비교)"로 못 박은 것.
        //
        // 가짜 서버가 **자리마다 다른 바이트**를 주므로(`testing::pattern_byte`), 이어 붙이는
        // 지점이 한 바이트라도 겹치거나 빠지면 대조에서 드러난다. 균일한 값으로 채우면
        // 어긋난 이어받기도 "같아 보여" 이 단언이 아무것도 지키지 못한다
        const TOTAL: u64 = 200 * 1024;
        const ALREADY: u64 = 40 * 1024 + 7; // 블록 경계와 어긋난 지점에서 이어받는다

        let server = FakeServer::new();
        server.set_download_size(TOTAL);
        let (mut manager, sites, site) = manager_with_site(&server);
        let mut queue = TransferQueue::new();
        let mut runner = TransferRunner::new();
        let local = temp_file("resume_e2e");
        let part = part_path(&local);
        let _ = std::fs::remove_file(&local);

        // 앞부분만 받아 둔 상태를 만든다 — 실제로 끊겼을 때 디스크에 남는 것과 같은 모습이다
        let head: Vec<u8> = (0..ALREADY).map(pattern_byte).collect();
        std::fs::write(&part, &head).expect("부분 파일");

        let id = queue.enqueue(
            site,
            TransferDirection::Download,
            local.clone(),
            RemotePath::new("/big.bin"),
            TOTAL,
        );
        let _ = drain_until(&mut manager, Duration::from_secs(3), |events| {
            events
                .iter()
                .any(|event| matches!(event, ConnEvent::Phase(ConnPhase::Ready)))
        });
        runner.start_ready(&mut queue, &manager, &sites, 0.0);
        assert_eq!(runner.in_flight(), 1, "이어받기가 시작되지 않았다");
        assert_eq!(
            queue.get(id).expect("항목").state,
            TransferState::Active {
                sent: ALREADY,
                speed: 0
            },
            "받아 둔 만큼에서 이어가야 한다"
        );

        let mut now = 0.0;
        let deadline = Instant::now() + Duration::from_secs(10);
        while Instant::now() < deadline && runner.in_flight() > 0 {
            for (_, event) in manager.poll() {
                now += 0.1;
                match event {
                    ConnEvent::TransferProgress { id, transferred } => {
                        runner.on_progress(&mut queue, id, transferred, now)
                    }
                    ConnEvent::TransferDone { id, result } => {
                        runner.on_done(&mut queue, id, result.map_err(|e| e.to_string()))
                    }
                    _ => {}
                }
            }
            std::thread::sleep(Duration::from_millis(5));
        }

        assert!(
            queue.get(id).expect("항목").state.is_done(),
            "이어받기가 끝나지 않았다: {:?}",
            queue.get(id).map(|item| &item.state)
        );
        let got = std::fs::read(&local).expect("받은 파일");
        let expected: Vec<u8> = (0..TOTAL).map(pattern_byte).collect();
        assert_eq!(got.len(), expected.len(), "받은 크기가 원본과 다르다");
        assert!(
            got == expected,
            "이어 붙인 자리가 어긋났다 — 처음 다른 바이트: {:?}",
            got.iter()
                .zip(expected.iter())
                .position(|(a, b)| a != b)
                .map(|at| (at, got[at], expected[at]))
        );
        assert!(!part.exists(), "`.part`가 남았다");
        let _ = std::fs::remove_file(&local);
    }

    #[test]
    fn 일시정지_뒤_다시_누르면_이어받아_끝난다() {
        // Acceptance ④ 뒷문장 (spec 리뷰 M1) — 멈추는 것만이 아니라 **이어서 끝나는 것**까지 본다.
        // 멈춘 지점은 기계 속도에 따라 다르므로, 받아 둔 조각을 만들어 둔 상태에서
        // 멈춤 → 다시 누름 → 완료의 흐름을 확인한다
        const TOTAL: u64 = 128 * 1024;
        const ALREADY: u64 = 30 * 1024;

        let server = FakeServer::new();
        server.set_download_size(TOTAL);
        let (mut manager, sites, site) = manager_with_site(&server);
        let mut queue = TransferQueue::new();
        let mut runner = TransferRunner::new();
        let local = temp_file("pause_resume_e2e");
        let part = part_path(&local);
        let _ = std::fs::remove_file(&local);
        std::fs::write(&part, (0..ALREADY).map(pattern_byte).collect::<Vec<u8>>())
            .expect("부분 파일");

        let id = queue.enqueue(
            site,
            TransferDirection::Download,
            local.clone(),
            RemotePath::new("/big.bin"),
            TOTAL,
        );
        let _ = drain_until(&mut manager, Duration::from_secs(3), |events| {
            events
                .iter()
                .any(|event| matches!(event, ConnEvent::Phase(ConnPhase::Ready)))
        });

        // 멈춰 있는 동안에는 시작되지 않는다
        runner.pause(&mut queue, &manager);
        runner.start_ready(&mut queue, &manager, &sites, 0.0);
        assert_eq!(runner.in_flight(), 0);
        assert_eq!(queue.get(id).expect("항목").state, TransferState::Wait);

        // 다시 누르면 받아 둔 자리에서 이어간다
        runner.resume(&mut queue);
        runner.start_ready(&mut queue, &manager, &sites, 1.0);
        assert_eq!(runner.in_flight(), 1, "다시 눌렀는데 시작되지 않았다");

        let mut now = 1.0;
        let deadline = Instant::now() + Duration::from_secs(10);
        while Instant::now() < deadline && runner.in_flight() > 0 {
            for (_, event) in manager.poll() {
                now += 0.1;
                match event {
                    ConnEvent::TransferProgress { id, transferred } => {
                        runner.on_progress(&mut queue, id, transferred, now)
                    }
                    ConnEvent::TransferDone { id, result } => {
                        runner.on_done(&mut queue, id, result.map_err(|e| e.to_string()))
                    }
                    _ => {}
                }
            }
            std::thread::sleep(Duration::from_millis(5));
        }

        assert!(
            queue.get(id).expect("항목").state.is_done(),
            "끝나지 않았다"
        );
        let got = std::fs::read(&local).expect("받은 파일");
        assert_eq!(got, (0..TOTAL).map(pattern_byte).collect::<Vec<u8>>());
        let _ = std::fs::remove_file(&local);
    }

    #[test]
    fn 취소하면_받다_만_파일이_남지_않는다() {
        // Acceptance ⑤
        let server = FakeServer::new();
        server.set_download_size(64 * 1024 * 64);
        let (mut manager, sites, site) = manager_with_site(&server);
        let mut queue = TransferQueue::new();
        let mut runner = TransferRunner::new();
        let local = temp_file("cancel");
        let id = queue.enqueue(
            site,
            TransferDirection::Download,
            local.clone(),
            RemotePath::new("/big.bin"),
            64 * 1024 * 64,
        );
        let _ = drain_until(&mut manager, Duration::from_secs(3), |events| {
            events
                .iter()
                .any(|event| matches!(event, ConnEvent::Phase(ConnPhase::Ready)))
        });
        runner.start_ready(&mut queue, &manager, &sites, 0.0);
        assert_eq!(runner.in_flight(), 1);

        runner.cancel(&manager, id);
        assert!(queue.cancel(id), "큐에서도 빠진다");
        assert_eq!(runner.in_flight(), 0);
        assert_eq!(runner.pending_cleanup(), 1, "정리를 기다리는 중이어야 한다");

        // 워커가 파일을 놓았다고 알려 오면 그때 지운다 — 열려 있는 파일은 지울 수 없다
        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline && runner.pending_cleanup() > 0 {
            for (_, event) in manager.poll() {
                if let ConnEvent::TransferDone { id, result } = event {
                    runner.on_done(&mut queue, id, result.map_err(|e| e.to_string()));
                }
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        assert_eq!(runner.pending_cleanup(), 0, "정리가 끝나지 않았다");
        assert!(!part_path(&local).exists(), "받다 만 파일이 남았다");
        assert!(!local.exists(), "완성되지도 않은 파일이 생겼다");
    }

    #[test]
    fn 일시정지는_대기로_되돌리고_part를_남긴다() {
        // Acceptance ④ — 취소와 다르다. 다시 누르면 그 크기에서 이어받는다
        let server = FakeServer::new();
        server.set_download_size(64 * 1024 * 64);
        let (mut manager, sites, site) = manager_with_site(&server);
        let mut queue = TransferQueue::new();
        let mut runner = TransferRunner::new();
        let local = temp_file("pause");
        let id = queue.enqueue(
            site,
            TransferDirection::Download,
            local.clone(),
            RemotePath::new("/big.bin"),
            64 * 1024 * 64,
        );
        let _ = drain_until(&mut manager, Duration::from_secs(3), |events| {
            events
                .iter()
                .any(|event| matches!(event, ConnEvent::Phase(ConnPhase::Ready)))
        });
        runner.start_ready(&mut queue, &manager, &sites, 0.0);

        runner.pause(&mut queue, &manager);
        assert!(queue.is_paused());
        assert_eq!(queue.get(id).expect("항목").state, TransferState::Wait);
        assert_eq!(runner.in_flight(), 0);
        // 멈춘 동안에는 새로 시작하지 않는다
        runner.start_ready(&mut queue, &manager, &sites, 1.0);
        assert_eq!(runner.in_flight(), 0);

        runner.resume(&mut queue);
        assert!(!queue.is_paused());
        let _ = drain_until(&mut manager, Duration::from_secs(3), |events| {
            events
                .iter()
                .any(|event| matches!(event, ConnEvent::TransferDone { .. }))
        });
        let _ = std::fs::remove_file(part_path(&local));
        let _ = std::fs::remove_file(&local);
    }

    #[test]
    fn 취소_직후_연결이_닫혀도_받다_만_파일이_치워진다() {
        // quality 리뷰 M1 — 연결이 닫히면 워커의 완료 통지가 채널째로 버려져 `on_done`이
        // 불리지 않는다. 그때 정리를 넘겨받지 않으면 `.part`가 영영 남는다
        let mut runner = TransferRunner::new();
        let local = temp_file("orphan");
        let part = part_path(&local);
        std::fs::write(&part, vec![0u8; 32]).expect("부분 파일");

        let id = TransferId(1);
        let conn = ConnectionId(3);
        runner.cancelling.insert(id, (conn, part.clone()));
        assert_eq!(runner.pending_cleanup(), 1);

        runner.forget_connection(conn);
        assert_eq!(runner.pending_cleanup(), 0, "정리가 넘겨지지 않았다");
        assert!(!part.exists(), "받다 만 파일이 남았다");
    }

    #[test]
    fn 지우지_못한_조각은_다음_기회에_다시_치운다() {
        // quality 리뷰 M3 — 백신 검사·핸들 지연으로 한 번에 실패할 수 있다.
        // 조용히 삼키면 받다 만 파일이 영영 남는다
        let mut runner = TransferRunner::new();
        let mut queue = TransferQueue::new();
        let sites = SiteStore::new();
        let manager = ConnectionManager::new(Arc::new(|| {}));
        let local = temp_file("retry_cleanup");
        let part = part_path(&local);
        std::fs::write(&part, vec![0u8; 16]).expect("부분 파일");

        // 파일을 붙들고 있는 동안에는 지우지 못한다(Windows) — 목록에 남아야 한다
        let handle = std::fs::File::open(&part).expect("열기");
        runner.pending_delete.push(part.clone());
        runner.start_ready(&mut queue, &manager, &sites, 0.0);
        // 열려 있어도 지워지는 환경이 있어(공유 삭제 허용) 둘 다 정상으로 본다 —
        // 중요한 것은 **남았으면 다시 시도한다**는 것이다
        if part.exists() {
            assert_eq!(runner.pending_cleanup(), 1, "다시 시도할 목록에서 사라졌다");
        }
        drop(handle);

        runner.start_ready(&mut queue, &manager, &sites, 1.0);
        assert!(!part.exists(), "놓아준 뒤에도 치우지 못했다");
        assert_eq!(runner.pending_cleanup(), 0);
    }

    #[test]
    fn 앱이_닫히면_진행_중이던_것이_대기로_돌아가고_취소분만_치워진다() {
        // plan Edge Case — 저장된 큐가 "전송 중"이라고 주장하면 다음 실행 화면이 거짓말을 한다.
        // 반대로 받아 둔 조각을 지우면 이어받기가 매번 처음부터가 된다
        let mut runner = TransferRunner::new();
        let mut queue = TransferQueue::new();
        let running_local = temp_file("shutdown_running");
        let running_part = part_path(&running_local);
        std::fs::write(&running_part, vec![0u8; 64]).expect("부분 파일");
        let cancelled_part = part_path(&temp_file("shutdown_cancelled"));
        std::fs::write(&cancelled_part, vec![0u8; 32]).expect("부분 파일");

        let id = queue.enqueue(
            SiteId(1),
            TransferDirection::Download,
            running_local.clone(),
            RemotePath::new("/x.bin"),
            1000,
        );
        queue.update(id, TransferState::Active { sent: 64, speed: 1 });
        runner.assigned.insert(
            id,
            Assignment {
                conn: ConnectionId(0),
                final_path: running_local.clone(),
                working_path: running_part.clone(),
                direction: TransferDirection::Download,
                progress: ProgressSink::new(0.0),
            },
        );
        runner
            .cancelling
            .insert(TransferId(99), (ConnectionId(0), cancelled_part.clone()));

        runner.shutdown(&mut queue);

        assert_eq!(queue.get(id).expect("항목").state, TransferState::Wait);
        assert!(running_part.exists(), "이어받을 조각까지 지웠다");
        assert!(!cancelled_part.exists(), "취소분이 남았다");
        assert_eq!(runner.in_flight(), 0);
        assert_eq!(runner.pending_cleanup(), 0);
        let _ = std::fs::remove_file(&running_part);
    }

    #[test]
    fn 연결이_없는_사이트는_그대로_대기한다() {
        // 큐에 남아 있다가 다시 연결되면 그때 나간다
        let mut queue = TransferQueue::new();
        let mut runner = TransferRunner::new();
        let sites = SiteStore::new();
        let manager = ConnectionManager::new(Arc::new(|| {}));
        let id = queue.enqueue(
            SiteId(9),
            TransferDirection::Download,
            PathBuf::from(r"C:\x.bin"),
            RemotePath::new("/x.bin"),
            10,
        );
        runner.start_ready(&mut queue, &manager, &sites, 0.0);
        assert_eq!(runner.in_flight(), 0);
        assert_eq!(queue.get(id).expect("항목").state, TransferState::Wait);
    }

    #[test]
    fn 실패한_전송은_part를_남겨_다음에_이어받는다() {
        // Acceptance ③ — 실패한 `.part`를 지우면 재시도가 언제나 처음부터가 된다
        let mut queue = TransferQueue::new();
        let mut runner = TransferRunner::new();
        let local = temp_file("resume");
        let part = part_path(&local);
        std::fs::write(&part, vec![0u8; 400]).expect("부분 파일");

        let id = queue.enqueue(
            SiteId(1),
            TransferDirection::Download,
            local.clone(),
            RemotePath::new("/x.bin"),
            1000,
        );
        // 워커 없이 결과만 흘려 넣는다 — 뒤처리 규칙만 보는 자리다
        runner.assigned.insert(
            id,
            Assignment {
                conn: ConnectionId(0),
                final_path: local.clone(),
                working_path: part.clone(),
                direction: TransferDirection::Download,
                progress: ProgressSink::new(0.0),
            },
        );
        runner.on_done(&mut queue, id, Err("연결이 끊겼습니다".to_owned()));

        assert!(queue.get(id).expect("항목").state.is_error());
        assert!(part.exists(), "이어받을 조각이 사라졌다");
        // 다시 걸면 그 크기에서 이어받는다
        assert_eq!(
            resume_offset(std::fs::metadata(&part).expect("조각").len(), 1000),
            400
        );
        let _ = std::fs::remove_file(&part);
    }
}
