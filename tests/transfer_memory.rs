//! 전송 스트리밍의 메모리 실측 (NFR-12 · plan T18 Acceptance ② · 임계 D27).
//!
//! **별도 통합 테스트로 둔 이유**: 작업 집합은 프로세스 전체를 재는 값이라, 같은 바이너리의
//! 다른 단위 테스트가 스레드 병렬로 함께 돌면 그들의 할당이 이 측정에 섞인다(`cargo test`의
//! 기본값이 병렬이다). 통합 테스트는 파일마다 **자기 프로세스**로 실행되므로, 이 파일에
//! 이 측정 하나만 두면 재는 대상이 우리 전송 경로뿐이 된다.
//!
//! 측정은 **올리기**로 한다 — 받기로 재면 4GB를 디스크에 쓰게 되어 측정 대상이 우리 버퍼가
//! 아니라 파일 캐시가 된다. 가짜 세션의 싱크는 길이만 세고 버린다 (D25-a).
use std::io::Read;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration;

use file_explorer::remote::testing::{FakeServer, FakeSession};
use file_explorer::remote::types::{NoProgress, RemotePath, RemoteSession, SiteId, SiteRecord};

/// 한 건의 크기 — 계획이 정한 값 (Acceptance ②)
const GIB: u64 = 1024 * 1024 * 1024;
/// 동시 건수
const STREAMS: usize = 4;
/// 유휴 대비 허용 증가분 (D27 — 버퍼 64KB×4 + 큐·프로토콜 상태 몫)
const LIMIT: u64 = 50 * 1024 * 1024;

/// 이 프로세스의 작업 집합(바이트).
///
/// # 안전성
/// 구조체를 0으로 채우고 그 크기를 함께 넘긴다(Win32가 요구하는 규약). 실패하면 0을 돌려주며,
/// 호출부는 그것을 **통과가 아니라 측정 실패**로 다룬다
fn working_set() -> u64 {
    use windows::Win32::System::ProcessStatus::{GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS};
    use windows::Win32::System::Threading::GetCurrentProcess;
    let mut counters = PROCESS_MEMORY_COUNTERS {
        cb: std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32,
        ..Default::default()
    };
    let ok = unsafe {
        GetProcessMemoryInfo(
            GetCurrentProcess(),
            &mut counters,
            std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32,
        )
    };
    if ok.is_err() {
        return 0;
    }
    counters.WorkingSetSize as u64
}

/// 요청한 만큼 같은 바이트를 만들어 내주는 원본 — 1GB를 메모리에 쌓지 않기 위함이다
struct Endless {
    remaining: u64,
}

impl Read for Endless {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if self.remaining == 0 {
            return Ok(0);
        }
        let take = buf.len().min(self.remaining as usize);
        buf[..take].fill(0xAB);
        self.remaining -= take as u64;
        Ok(take)
    }
}

#[test]
fn 일기가바이트_동시_네_건을_옮겨도_메모리가_버퍼_몫에_머문다() {
    let baseline = working_set();
    assert!(
        baseline > 0,
        "작업 집합을 재지 못했다 — NFR-12를 판정할 수 없다(측정 실패를 통과로 삼지 않는다)"
    );

    let server = FakeServer::new();
    let peak = Arc::new(AtomicU64::new(baseline));
    let stop = Arc::new(AtomicBool::new(false));
    let watcher = {
        let peak = Arc::clone(&peak);
        let stop = Arc::clone(&stop);
        std::thread::spawn(move || {
            while !stop.load(Ordering::SeqCst) {
                peak.fetch_max(working_set(), Ordering::SeqCst);
                std::thread::sleep(Duration::from_millis(20));
            }
        })
    };

    let mut workers = Vec::new();
    for _ in 0..STREAMS {
        let server = Arc::clone(&server);
        workers.push(std::thread::spawn(move || {
            let mut session = FakeSession::new(server);
            let record = SiteRecord::new(SiteId(0), "가짜".to_owned());
            session.connect(&record).expect("연결");
            let mut source = Endless { remaining: GIB };
            let mut progress = NoProgress;
            session
                .upload(&RemotePath::new("/big.bin"), &mut source, 0, &mut progress)
                .expect("전송")
        }));
    }
    let moved: u64 = workers
        .into_iter()
        .map(|worker| worker.join().expect("전송 스레드"))
        .sum();
    stop.store(true, Ordering::SeqCst);
    let _ = watcher.join();

    assert_eq!(moved, STREAMS as u64 * GIB, "네 건이 다 옮겨지지 않았다");
    let growth = peak.load(Ordering::SeqCst).saturating_sub(baseline);
    assert!(
        growth < LIMIT,
        "전송 중 작업 집합이 {}MB 늘었다 — 임계는 {}MB다 (파일을 통째로 버퍼에 올리고 있지 않은지 보라)",
        growth / (1024 * 1024),
        LIMIT / (1024 * 1024)
    );
}
