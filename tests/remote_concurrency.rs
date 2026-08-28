//! 원격 동시성 회귀 테스트 (NFR-10·NFR-11·NFR-13).
//!
//! **실서버가 필요 없다** — 가짜 서버(`remote::testing`)가 지연·무응답·대량 목록을 그대로
//! 흉내 낸다(D25). 실제 FTP/SFTP 서버로 확인하고 싶으면 `FE_TEST_FTP_URL`·`FE_TEST_SFTP_URL`을
//! 두고 수동으로 돌린다(AGENTS 참조) — 이 파일은 그 환경변수 없이도 항상 돈다.
//!
//! 여기서 지키려는 것은 하나다: **한 연결이 막혀도 앱은 계속 돈다.** UI 스레드는 채널만
//! 확인하므로(NFR-10), 응답 없는 서버는 그 연결의 워커 스레드 하나만 붙잡는다.
use moa::remote::connection::{ConnCommand, ConnEvent, ConnPhase, ConnectionId};
use moa::remote::manager::ConnectionManager;
use moa::remote::testing::{FakeServer, FakeSession, fake_entry};
use moa::remote::types::{RemoteEntry, RemotePath, SiteId, SiteRecord};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// 이벤트를 모으며 조건이 참이 될 때까지 기다린다 — 시간이 아니라 **관측된 상태**로 판정한다
fn pump_until(
    manager: &mut ConnectionManager,
    events: &mut Vec<(ConnectionId, ConnEvent)>,
    limit: Duration,
    mut done: impl FnMut(&[(ConnectionId, ConnEvent)]) -> bool,
) -> bool {
    let deadline = Instant::now() + limit;
    loop {
        events.extend(manager.poll());
        if done(events) {
            return true;
        }
        if Instant::now() >= deadline {
            return false;
        }
        std::thread::sleep(Duration::from_millis(2));
    }
}

fn site(id: u32, name: &str) -> SiteRecord {
    SiteRecord::new(SiteId(id), name.to_owned())
}

/// 그 연결이 돌려준 목록 항목 수
fn listed_len(events: &[(ConnectionId, ConnEvent)], target: ConnectionId) -> Option<usize> {
    events.iter().find_map(|(conn, event)| match event {
        ConnEvent::Listed { entries, .. } if *conn == target => Some(entries.len()),
        _ => None,
    })
}

#[test]
fn 응답_없는_연결이_있어도_나머지_연결은_계속_처리된다() {
    // NFR-11 — 서버 하나가 죽은 듯 굴어도 다른 사이트의 탐색은 그대로 된다.
    // 막힌 쪽은 **자기 워커 스레드 하나만** 붙잡는다(UI 스레드는 채널만 본다 — NFR-10)
    let 막힌_서버 = FakeServer::new();
    막힌_서버.set_entries("/", vec![fake_entry("안 보일 것", false)]);
    막힌_서버.set_hang(true);

    let 정상_서버 = FakeServer::new();
    정상_서버.set_entries("/pub", vec![fake_entry("보고서.txt", false)]);
    정상_서버.set_entries("/var", vec![fake_entry("로그", true)]);

    let mut manager = ConnectionManager::new(Arc::new(|| {}));
    let 막힌 = manager.open(
        &site(1, "막힌 서버"),
        String::new(),
        Box::new(FakeSession::new(Arc::clone(&막힌_서버))),
    );
    let 첫째 = manager.open(
        &site(2, "정상 서버 A"),
        String::new(),
        Box::new(FakeSession::new(Arc::clone(&정상_서버))),
    );
    let 둘째 = manager.open(
        &site(3, "정상 서버 B"),
        String::new(),
        Box::new(FakeSession::new(Arc::clone(&정상_서버))),
    );

    manager.send(
        막힌,
        ConnCommand::List {
            generation: 1,
            path: RemotePath::new("/"),
        },
    );
    manager.send(
        첫째,
        ConnCommand::List {
            generation: 1,
            path: RemotePath::new("/pub"),
        },
    );
    manager.send(
        둘째,
        ConnCommand::List {
            generation: 1,
            path: RemotePath::new("/var"),
        },
    );

    let mut events = Vec::new();
    let 도착 = pump_until(
        &mut manager,
        &mut events,
        Duration::from_secs(5),
        |events| listed_len(events, 첫째).is_some() && listed_len(events, 둘째).is_some(),
    );
    assert!(도착, "막힌 연결이 다른 연결의 목록까지 붙잡았다");
    assert_eq!(listed_len(&events, 첫째), Some(1));
    assert_eq!(listed_len(&events, 둘째), Some(1));
    assert_eq!(
        listed_len(&events, 막힌),
        None,
        "막힌 서버가 답을 줄 리 없다"
    );

    // 막힘을 풀면 그 연결도 제 갈 길을 간다 — 영영 잠긴 것이 아니다
    막힌_서버.set_hang(false);
    let 뒤늦게 = pump_until(
        &mut manager,
        &mut events,
        Duration::from_secs(5),
        |events| listed_len(events, 막힌).is_some(),
    );
    assert!(뒤늦게, "막힘이 풀렸는데도 답이 오지 않았다");

    // 세 워커 모두 회수된다 — 막혔던 것도 함께
    for conn in [막힌, 첫째, 둘째] {
        manager.close(conn);
    }
    let deadline = Instant::now() + Duration::from_secs(5);
    while (막힌_서버.live_sessions() > 0 || 정상_서버.live_sessions() > 0)
        && Instant::now() < deadline
    {
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(막힌_서버.live_sessions(), 0, "막혔던 워커가 남았다");
    assert_eq!(정상_서버.live_sessions(), 0, "정상 워커가 남았다");
}

#[test]
fn 만_항목_폴더를_조회해도_답이_돌아온다() {
    // NFR-13 — 큰 폴더 하나가 연결을 잠그면 그 사이트는 쓸 수 없게 된다.
    // 목록 자체는 워커가 통째로 받아 채널로 넘기고, 화면은 가상 스크롤로 그린다(T7)
    let server = FakeServer::new();
    let entries: Vec<RemoteEntry> = (0..10_000)
        .map(|index| fake_entry(&format!("파일{index:05}.bin"), false))
        .collect();
    server.set_entries("/big", entries);

    let mut manager = ConnectionManager::new(Arc::new(|| {}));
    let conn = manager.open(
        &site(1, "큰 폴더"),
        String::new(),
        Box::new(FakeSession::new(Arc::clone(&server))),
    );
    manager.send(
        conn,
        ConnCommand::List {
            generation: 7,
            path: RemotePath::new("/big"),
        },
    );

    let mut events = Vec::new();
    let 시작 = Instant::now();
    let 도착 = pump_until(
        &mut manager,
        &mut events,
        Duration::from_secs(10),
        |events| listed_len(events, conn).is_some(),
    );
    // 실측값을 남긴다 (T26 Acceptance ④ — `--nocapture`로 볼 수 있다)
    println!(
        "NFR-13 실측: 1만 항목 조회 왕복 {}ms",
        시작.elapsed().as_millis()
    );
    assert!(도착, "1만 항목 조회가 시간 안에 돌아오지 않았다");
    assert_eq!(listed_len(&events, conn), Some(10_000));

    // 그 뒤로도 같은 연결이 계속 일한다 — 큰 조회 하나로 굳지 않는다
    server.set_entries("/small", vec![fake_entry("작은.txt", false)]);
    manager.send(
        conn,
        ConnCommand::List {
            generation: 8,
            path: RemotePath::new("/small"),
        },
    );
    let 이어서 = pump_until(
        &mut manager,
        &mut events,
        Duration::from_secs(5),
        |events| {
            events.iter().any(|(id, event)| {
                *id == conn && matches!(event, ConnEvent::Listed { generation: 8, .. })
            })
        },
    );
    assert!(이어서, "큰 조회 뒤에 연결이 굳었다");
}

#[test]
fn 연결_단계는_사이트마다_따로_간다() {
    // NFR-10의 다른 얼굴 — 한 사이트의 연결 실패가 다른 사이트를 끌어내리지 않는다
    let 거절하는_서버 = FakeServer::new();
    거절하는_서버.fail_connects(u32::MAX);
    let 받아주는_서버 = FakeServer::new();
    받아주는_서버.set_entries("/", Vec::new());

    let mut manager = ConnectionManager::new(Arc::new(|| {}));
    let 실패 = manager.open(
        &site(1, "거절"),
        String::new(),
        Box::new(FakeSession::new(Arc::clone(&거절하는_서버))),
    );
    let 성공 = manager.open(
        &site(2, "정상"),
        String::new(),
        Box::new(FakeSession::new(Arc::clone(&받아주는_서버))),
    );

    let mut events = Vec::new();
    let 도착 = pump_until(
        &mut manager,
        &mut events,
        Duration::from_secs(10),
        |events| {
            events.iter().any(|(id, event)| {
                *id == 성공 && matches!(event, ConnEvent::Phase(ConnPhase::Ready))
            })
        },
    );
    assert!(도착, "옆 연결의 재시도가 정상 연결까지 붙잡았다");
    assert!(
        manager
            .get(성공)
            .is_some_and(|connection| matches!(connection.phase(), ConnPhase::Ready)),
        "정상 연결이 서지 않았다"
    );
    manager.close(실패);
    manager.close(성공);
}
