//! 변경 감시 통합 테스트 (T3 Acceptance) — HWND 비의존, 채널 수신으로 검증
use file_explorer::fs::watcher::DirWatcher;
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

    // 정지 — Drop이 이벤트 신호 후 join (블록되면 테스트가 끝나지 않아 실패로 드러남)
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
