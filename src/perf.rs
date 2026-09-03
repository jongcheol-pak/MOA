//! 임시 성능 계측 — 어느 단계가 실제로 느린지 실측으로 가르기 위한 것이며 **기본은 꺼져 있다**.
//!
//! `MOA_PERF_LOG=1`로 켜면 **실행 파일 옆** `moa_perf.log`에 한 줄씩 덧붙인다. 켜지 않으면
//! 형식 문자열조차 만들지 않는다 — `log`가 클로저를 받아 `enabled()`가 거짓이면 부르지 않는다.
//!
//! **경로·파일 이름을 적지 않는다** — 이 파일은 사용자가 그대로 보내 오는 것이라 개인 폴더
//! 구조가 실려 나가면 안 된다. 남기는 것은 소요 시간과 개수뿐이다.
//!
//! **켰을 때는 줄마다 파일을 열고 닫는다** — UI 스레드에서 도는 블로킹 I/O지만, 계측을 켠
//! 동안만 그렇고 재는 구간 **밖**에서 기록하므로 측정값 자체는 왜곡되지 않는다. 원인을
//! 가른 뒤에는 이 모듈째 걷어낸다.
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::Instant;

static ENABLED: OnceLock<bool> = OnceLock::new();
static START: OnceLock<Instant> = OnceLock::new();

/// 계측이 켜져 있는가 — 환경변수를 한 번만 읽는다
pub fn enabled() -> bool {
    *ENABLED.get_or_init(|| {
        std::env::var_os("MOA_PERF_LOG").is_some_and(|value| value != "0" && !value.is_empty())
    })
}

/// 한 줄 남긴다 — 꺼져 있으면 `make`를 부르지 않는다.
///
/// 실패(경로를 얻지 못함·열지 못함)는 조용히 지나간다. 계측이 실행을 막아서는 안 된다
pub fn log(make: impl FnOnce() -> String) {
    if !enabled() {
        return;
    }
    let since = START.get_or_init(Instant::now).elapsed().as_secs_f32();
    let Some(path) = log_path() else {
        return;
    };
    let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) else {
        return;
    };
    let _ = writeln!(file, "[{since:9.3}] {}", make());
}

/// 기록 파일 자리 — 실행 파일 옆(`settings.json`과 같은 규칙)
fn log_path() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    Some(exe.parent()?.join("moa_perf.log"))
}
