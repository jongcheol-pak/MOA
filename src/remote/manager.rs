//! 연결 전체를 소유하는 관리자 (FR-45·D4·NFR-11).
//!
//! 화면(`ui`)은 이 타입만 만진다 — 워커도 세션도 여기 안에 있다. 연결 하나가 막혀도
//! 다른 연결의 명령은 자기 워커에서 그대로 처리되므로, 관리자는 **소유와 배정**만 맡는다.
//!
//! **채널 배정 규칙(D4)은 이 안에만 있다.** 사이트 설정의 `최대 동시 연결 수(M)`가 1이면
//! TCP 연결이 하나뿐이라 탐색 채널이 전송을 겸하는데, 그 사실을 전송 큐가 알면
//! "전송 슬롯 0"으로 읽어 전송이 영영 시작되지 않는다. 그래서 **큐가 보는 값은 언제나
//! `max(1, 전송 채널 수)`**이고, 겸용 여부는 여기서만 안다.
use std::collections::HashMap;
use std::sync::Arc;

use crate::remote::connection::{
    ConnCommand, ConnEvent, Connection, ConnectionId, RetryPolicy, TransferId, Wake,
};
use crate::remote::log::LogKind;
use crate::remote::types::{RemoteSession, SiteRecord};

/// 제한을 켜지 않았을 때 여는 연결 수 — 탐색 1 + 전송 2 (사용자 확정, FR-45)
const DEFAULT_TOTAL: u8 = 3;

/// 한 사이트에 배정한 채널 구성
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChannelPlan {
    /// 실제로 여는 TCP 연결 수
    pub connections: u8,
    /// **전송 큐에 보고하는 슬롯 수** — 겸용이어도 최소 1이다
    pub transfer_slots: u8,
    /// 탐색 채널이 전송을 겸하는가. 이 사실은 관리자 밖으로 나가지 않는다
    pub shared: bool,
}

/// 사이트 설정을 채널 구성으로 옮긴다 (D4).
///
/// `limit`는 사이트 관리자의 `동시 연결 수 제한(L)`이 켜졌을 때의 `M`이며 **상한**이다 —
/// 기본 정책(탐색 1 + 전송 2)보다 크게 잡아도 그 이상 열지 않는다.
pub fn plan_channels(limit: Option<u8>) -> ChannelPlan {
    let total = limit.map_or(DEFAULT_TOTAL, |m| m.clamp(1, DEFAULT_TOTAL));
    if total <= 1 {
        // 연결이 하나뿐이라 탐색 채널이 전송을 겸한다. 큐에는 슬롯 1로 보고한다
        ChannelPlan {
            connections: 1,
            transfer_slots: 1,
            shared: true,
        }
    } else {
        ChannelPlan {
            connections: total,
            transfer_slots: total - 1,
            shared: false,
        }
    }
}

/// 열린 연결 전부를 쥔다
pub struct ConnectionManager {
    connections: HashMap<ConnectionId, Connection>,
    /// 연결이 열린 순서 — 화면이 목록을 흔들리지 않게 그리는 데 쓴다
    order: Vec<ConnectionId>,
    next_id: u32,
    wake: Wake,
    retry: RetryPolicy,
}

impl ConnectionManager {
    pub fn new(wake: Wake) -> ConnectionManager {
        ConnectionManager::with_retry(wake, RetryPolicy::DEFAULT)
    }

    /// 재시도 규칙을 지정해 만든다 — 테스트가 초 단위를 기다리지 않게 하는 통로다
    pub fn with_retry(wake: Wake, retry: RetryPolicy) -> ConnectionManager {
        ConnectionManager {
            connections: HashMap::new(),
            order: Vec::new(),
            next_id: 1,
            wake,
            retry,
        }
    }

    /// 연결을 연다. `session`은 프로토콜 구현이며 워커 스레드로 소유가 넘어간다.
    ///
    /// **세션을 여기서 만들지 않는 이유**: SFTP는 호스트 키 확인 통로가 필요하고 그것은
    /// 화면(T10)이 쥔다 — 관리자가 세션을 조립하면 `remote`가 화면을 알아야 한다.
    pub fn open(
        &mut self,
        site: &SiteRecord,
        password: String,
        session: Box<dyn RemoteSession>,
    ) -> ConnectionId {
        let id = ConnectionId(self.next_id);
        self.next_id += 1;
        let connection = Connection::spawn(
            id,
            site.clone(),
            password,
            session,
            Arc::clone(&self.wake),
            self.retry,
        );
        connection.send(ConnCommand::Connect);
        self.connections.insert(id, connection);
        self.order.push(id);
        id
    }

    /// 이 사이트가 쓸 채널 구성 (D4)
    pub fn plan_for(&self, site: &SiteRecord) -> ChannelPlan {
        plan_channels(site.connection_limit)
    }

    /// 전송 큐가 보는 슬롯 수 — 겸용이어도 1 이상이다
    pub fn transfer_slots(&self, site: &SiteRecord) -> u8 {
        self.plan_for(site).transfer_slots
    }

    pub fn get(&self, id: ConnectionId) -> Option<&Connection> {
        self.connections.get(&id)
    }

    /// 그 연결의 로그에 한 줄 남긴다 (FR-39) — 연결이 이미 접혔으면 아무 일도 하지 않는다
    pub fn note(&mut self, id: ConnectionId, kind: LogKind, text: String) {
        if let Some(connection) = self.connections.get_mut(&id) {
            connection.push_log(kind, text);
        }
    }

    /// 명령을 보낸다. 없는 연결이거나 워커가 죽었으면 `false`
    pub fn send(&self, id: ConnectionId, command: ConnCommand) -> bool {
        self.connections
            .get(&id)
            .is_some_and(|connection| connection.send(command))
    }

    /// 진행 중인 전송을 멈추라고 알린다 — **명령 채널을 타지 않는다**.
    ///
    /// 워커는 명령을 하나씩 처리하므로 전송 중에는 채널을 읽지 않는다. 취소를 명령으로 보내면
    /// 그 전송이 끝난 뒤에야 읽혀, 취소도 늦고 그 뒤에 넣은 명령(같은 이름 확인 조회 등)까지
    /// 함께 밀린다. 그래서 취소만은 채널 밖 신호로 전한다
    pub fn cancel_transfer(&self, id: ConnectionId, transfer: TransferId) {
        if let Some(connection) = self.connections.get(&id) {
            connection.cancel_transfer(transfer);
        }
    }

    /// 모든 연결의 이벤트를 모은다 — 화면은 이것 하나만 매 프레임 부르면 된다
    pub fn poll(&mut self) -> Vec<(ConnectionId, ConnEvent)> {
        let mut events = Vec::new();
        for id in &self.order {
            if let Some(connection) = self.connections.get_mut(id) {
                events.extend(connection.poll().into_iter().map(|event| (*id, event)));
            }
        }
        events
    }

    /// 연결을 닫는다 — 워커도 함께 끝난다(`Connection`의 `Drop`)
    pub fn close(&mut self, id: ConnectionId) {
        self.connections.remove(&id);
        self.order.retain(|open| *open != id);
    }

    /// 열린 연결 식별자들 (열린 순서)
    pub fn ids(&self) -> &[ConnectionId] {
        &self.order
    }

    pub fn is_empty(&self) -> bool {
        self.connections.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::*;
    use crate::remote::connection::{ConnPhase, RetryPolicy};
    use crate::remote::testing::{FakeServer, FakeSession, fake_entry};
    use crate::remote::types::{RemotePath, SiteId};

    fn silent_wake() -> Wake {
        Arc::new(|| {})
    }

    fn fast_retry() -> RetryPolicy {
        RetryPolicy {
            base: Duration::from_millis(1),
            max: Duration::from_millis(4),
            attempts: 5,
        }
    }

    fn site(limit: Option<u8>) -> SiteRecord {
        let mut record = SiteRecord::new(SiteId(1), "가짜".to_owned());
        record.connection_limit = limit;
        record
    }

    #[test]
    fn 제한을_켜지_않으면_탐색_하나에_전송_둘이다() {
        let plan = plan_channels(None);
        assert_eq!(plan.connections, 3);
        assert_eq!(plan.transfer_slots, 2);
        assert!(!plan.shared);
    }

    #[test]
    fn 제한_값이_전체_상한이_된다() {
        // M=3이면 기본 정책과 같다
        assert_eq!(plan_channels(Some(3)), plan_channels(None));
        // M=2면 탐색 1 + 전송 1
        let two = plan_channels(Some(2));
        assert_eq!(
            (two.connections, two.transfer_slots, two.shared),
            (2, 1, false)
        );
        // 기본 정책보다 큰 값을 넣어도 그 이상 열지 않는다
        assert_eq!(plan_channels(Some(10)), plan_channels(None));
    }

    #[test]
    fn 제한이_하나면_겸용이되_큐에는_슬롯_하나로_보고한다() {
        // 큐가 0을 보면 전송이 영영 시작되지 않는다 (D4)
        let one = plan_channels(Some(1));
        assert_eq!(one.connections, 1);
        assert_eq!(one.transfer_slots, 1);
        assert!(one.shared, "탐색 채널이 전송을 겸한다");
    }

    #[test]
    fn 범위를_벗어난_설정값도_안전하게_다룬다() {
        // 사이트 관리자 스피너는 1~10이지만 저장 파일이 손상될 수 있다
        let zero = plan_channels(Some(0));
        assert_eq!(zero.connections, 1);
        assert_eq!(zero.transfer_slots, 1);
    }

    #[test]
    fn 관리자가_연결을_열고_이벤트를_모은다() {
        let server = FakeServer::new();
        server.set_entries("/", vec![fake_entry("a.txt", false)]);
        let mut manager = ConnectionManager::with_retry(silent_wake(), fast_retry());
        let record = site(None);
        let id = manager.open(
            &record,
            "비밀".to_owned(),
            Box::new(FakeSession::new(Arc::clone(&server))),
        );
        // `open`이 연결 명령까지 보낸다
        manager.send(
            id,
            ConnCommand::List {
                generation: 1,
                path: RemotePath::root(),
            },
        );

        let deadline = Instant::now() + Duration::from_secs(2);
        let mut events = Vec::new();
        while Instant::now() < deadline {
            events.extend(manager.poll());
            if events
                .iter()
                .any(|(_, event)| matches!(event, ConnEvent::Listed { .. }))
            {
                break;
            }
            std::thread::sleep(Duration::from_millis(2));
        }

        assert!(events.iter().all(|(event_id, _)| *event_id == id));
        assert!(
            events
                .iter()
                .any(|(_, event)| matches!(event, ConnEvent::Phase(ConnPhase::Ready)))
        );
        assert!(
            events
                .iter()
                .any(|(_, event)| matches!(event, ConnEvent::Listed { .. }))
        );
    }

    #[test]
    fn 같은_사이트에_두_번_연결하면_별개_연결이_된다() {
        let server = FakeServer::new();
        let mut manager = ConnectionManager::with_retry(silent_wake(), fast_retry());
        let record = site(None);
        let first = manager.open(
            &record,
            "비밀".to_owned(),
            Box::new(FakeSession::new(Arc::clone(&server))),
        );
        let second = manager.open(
            &record,
            "비밀".to_owned(),
            Box::new(FakeSession::new(Arc::clone(&server))),
        );
        assert_ne!(first, second);
        assert_eq!(manager.ids(), &[first, second]);
        assert_eq!(server.live_sessions(), 2);
    }

    #[test]
    fn 연결을_닫으면_워커도_함께_사라진다() {
        let server = FakeServer::new();
        let mut manager = ConnectionManager::with_retry(silent_wake(), fast_retry());
        let id = manager.open(
            &site(None),
            "비밀".to_owned(),
            Box::new(FakeSession::new(Arc::clone(&server))),
        );
        manager.close(id);
        assert!(manager.is_empty());

        let deadline = Instant::now() + Duration::from_secs(2);
        while server.live_sessions() > 0 && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(5));
        }
        assert_eq!(server.live_sessions(), 0);
        // 없는 연결에 보낸 명령은 조용히 무시된다
        assert!(!manager.send(id, ConnCommand::Disconnect));
        // 없는 연결의 취소도 마찬가지다 — 아무 일도 일어나지 않는다
        manager.cancel_transfer(id, TransferId(0));
    }

    #[test]
    fn 전송_슬롯은_사이트_설정을_따른다() {
        let manager = ConnectionManager::with_retry(silent_wake(), fast_retry());
        assert_eq!(manager.transfer_slots(&site(None)), 2);
        assert_eq!(manager.transfer_slots(&site(Some(3))), 2);
        assert_eq!(manager.transfer_slots(&site(Some(2))), 1);
        assert_eq!(manager.transfer_slots(&site(Some(1))), 1);
    }
}
