//! 변경 감시 통합 테스트 (T3 Acceptance) — HWND 비의존, 채널 수신으로 검증
use moa::fs::watcher::DirWatcher;
use std::sync::mpsc::channel;
use std::time::Duration;

fn make_temp_dir(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("fe_watch_test_{name}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn 생성과_삭제가_디바운스된_통지로_수신된다() {
    let dir = make_temp_dir("basic");
    let (tx, rx) = channel();
    let watcher = DirWatcher::start(dir.clone(), tx, None);

    // 감시 시동 대기 — 첫 ReadDirectoryChangesW 발행 전의 변경은 OS가 추적하지 않으므로
    // 통지가 올 때까지 마커 변경을 반복한다 (시동 레이스 흡수)
    let mut armed = false;
    for i in 0..20 {
        std::fs::write(dir.join(format!("arm{i}.txt")), b"x").unwrap();
        if rx.recv_timeout(Duration::from_millis(500)).is_ok() {
            armed = true;
            break;
        }
    }
    assert!(armed, "감시가 시동되지 않았다 (시동 통지 없음)");
    // 시동 중 쌓인 잔여 통지를 비워 이후 단정과 분리한다
    while rx.try_recv().is_ok() {}
    std::thread::sleep(Duration::from_millis(500));
    while rx.try_recv().is_ok() {}

    // 연속 생성 — 디바운스 창(300ms) 안의 변경은 통지 1회로 묶인다
    for i in 0..3 {
        std::fs::write(dir.join(format!("f{i}.txt")), b"x").unwrap();
    }
    assert!(
        rx.recv_timeout(Duration::from_secs(3)).is_ok(),
        "생성 변경 통지를 받지 못했다"
    );
    assert!(
        rx.recv_timeout(Duration::from_millis(700)).is_err(),
        "디바운스 창 안의 변경이 여러 통지로 쪼개졌다"
    );

    // 삭제 → 감시가 계속 살아있어 새 통지 수신
    std::fs::remove_file(dir.join("f0.txt")).unwrap();
    assert!(
        rx.recv_timeout(Duration::from_secs(3)).is_ok(),
        "삭제 변경 통지를 받지 못했다"
    );

    // 정지 — Drop이 정지 신호 후 회수 스레드에 join을 위임한다 (UI 무정지 설계)
    drop(watcher);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn 감시_경로를_보관한다() {
    let dir = make_temp_dir("path");
    let (tx, _rx) = channel();
    let watcher = DirWatcher::start(dir.clone(), tx, None);
    assert_eq!(watcher.path(), dir.as_path());
    drop(watcher);
    let _ = std::fs::remove_dir_all(&dir);
}
