//! 드라이브 줄 — 이 PC의 논리 드라이브와 그 연결 상태 (2026-08-17 사용자 요청).
//!
//! **트리가 아니라 앱이 이 목록을 소유한다** — 트리는 패널마다 있어(패널 셋이면 셋)
//! 각자 조회하면 셸·네트워크 왕복이 그만큼 되풀이되고, 연결 상태도 패널마다 갈려
//! 같은 드라이브에 X가 있는 트리와 없는 트리가 한 화면에 설 수 있다(즐겨찾기와 같은 구조).
//!
//! 조회를 **둘로 나눈 이유는 비용이 하늘과 땅 차이**라서다 — 목록 만들기(드라이브 열거 +
//! 셸 표시 이름·아이콘)는 수십 ms인데, 끊긴 네트워크 드라이브의 **접근 판정은 첫 시도가
//! 2.8초**다(실측). 한 함수로 묶으면 드라이브 줄이 화면에 서는 것 자체가 그만큼 늦고,
//! 시험도 끊긴 드라이브가 있는 PC에서 초 단위로 늘어진다.
use crate::fs::icons::IconCache;
use eframe::egui;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, Sender, channel};
use windows::Win32::Storage::FileSystem::{
    GetDriveTypeW, GetFileAttributesW, GetLogicalDrives, INVALID_FILE_ATTRIBUTES,
};
use windows::Win32::System::Com::{COINIT_APARTMENTTHREADED, CoInitializeEx, CoUninitialize};
use windows::Win32::System::WindowsProgramming::DRIVE_REMOTE;
use windows::core::HSTRING;

/// 트리에 설 드라이브 한 줄.
///
/// 화면이 그리는 데 필요한 것만 담는다 — 용량·파일시스템 종류는 이 앱이 보이지 않는다
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DriveRow {
    /// 뿌리 경로 (`C:\`)
    pub path: PathBuf,
    /// 셸 표시 이름 (`로컬 디스크 (C:)`) — 얻지 못하면 경로 문자열이다
    pub label: String,
    /// 시스템 이미지 리스트의 아이콘 인덱스.
    ///
    /// 인덱스는 프로세스 전역이라 **워커가 얻어 UI 스레드가 그려도 된다**
    pub icon: i32,
    /// 네트워크 드라이브인가 (`GetDriveTypeW == DRIVE_REMOTE`).
    ///
    /// 연결 끊김 표시는 네트워크 드라이브에만 붙는다 — 탐색기도 빈 광학 드라이브 같은
    /// 로컬 자리에는 그 표식을 두지 않는다
    pub network: bool,
    /// 닿지 못하는 상태인가 — 참이면 트리가 아이콘에 X 배지를 겹친다.
    ///
    /// `list_drives`는 이 값을 **언제나 `false`로 시작**한다(판정 전에는 배지를 두지 않는다).
    /// 접근 판정 결과가 오거나 사용자가 그 드라이브를 열어 본 뒤에 채워진다
    pub offline: bool,
}

/// 워커가 앱에 올려보내는 것 — 목록과 판정이 **따로 도착한다**.
///
/// 채널을 둘로 나누지 않는 이유는 받는 자리가 둘이 되기 때문이다
#[derive(Debug)]
pub enum DriveScan {
    /// 드라이브 목록이 준비됐다 (먼저 온다)
    Listed(Vec<DriveRow>),
    /// 네트워크 드라이브의 접근 판정 `(뿌리 경로, 닿았는가)` (뒤이어 온다)
    Reachability(Vec<(PathBuf, bool)>),
}

/// 드라이브 줄을 워커 스레드에서 만들고, 결과를 받을 채널을 돌려준다.
///
/// **두 번 보낸다** — 목록(`Listed`)을 먼저, 네트워크 드라이브의 접근 판정(`Reachability`)을
/// 뒤이어. 한 번에 묶으면 끊긴 드라이브 하나가 화면의 드라이브 줄 전체를 몇 초씩 붙든다.
///
/// 도착할 때마다 `request_repaint`로 화면을 깨운다 — 워커는 프레임 흐름을 모르므로
/// 이 신호가 없으면 사용자가 마우스를 움직일 때까지 결과가 화면에 오르지 않는다
pub fn spawn_scan(ctx: &egui::Context) -> Receiver<DriveScan> {
    let (tx, rx) = channel();
    let ctx = ctx.clone();
    std::thread::spawn(move || {
        // 셸 조회(`SHGetFileInfoW`)는 COM을 쓴다 — 스레드마다 초기화가 필요하다
        // (`fs::thumbnail`의 썸네일 워커와 같은 방식). 실패해도 조회만 폴백되고 앱은 계속 돈다
        // 안전성: 이 스레드에서 열고 끝에서 반드시 닫는다
        let com = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) }.is_ok();
        scan_into(&tx);
        if com {
            // 안전성: 위에서 성공한 초기화와 짝을 맞춘다
            unsafe { CoUninitialize() };
        }
        ctx.request_repaint();
    });
    rx
}

/// 워커 본체 — 목록을 보낸 뒤 판정을 보낸다. 받는 쪽이 사라지면 조용히 멎는다
fn scan_into(tx: &Sender<DriveScan>) {
    let mut icons = IconCache::new();
    let rows = list_drives(&mut icons);
    // 판정할 것을 먼저 챈다 — 목록은 곧 소유권을 넘긴다
    let network: Vec<PathBuf> = rows
        .iter()
        .filter(|row| row.network)
        .map(|row| row.path.clone())
        .collect();
    // 수신부가 이미 버려졌으면(앱 종료) 더 할 일이 없다 — 무거운 판정을 시작하지 않는다
    if tx.send(DriveScan::Listed(rows)).is_err() {
        return;
    }
    if network.is_empty() {
        return;
    }
    let judged = network
        .into_iter()
        .map(|root| {
            let reachable = is_reachable(&root);
            (root, reachable)
        })
        .collect();
    let _ = tx.send(DriveScan::Reachability(judged));
}

/// 이 PC의 논리 드라이브 목록 (`C:\`, `D:\` …).
///
/// 비트마스크의 비트 순서가 곧 알파벳 순이라 따로 정렬하지 않는다.
/// **접근 판정을 하지 않는다** — 끊긴 네트워크 드라이브에서도 곧 돌아온다(모듈 주석)
pub fn list_drives(icons: &mut IconCache) -> Vec<DriveRow> {
    // 안전성: 인자 없는 조회 — 현재 드라이브 비트마스크만 반환한다
    let mask = unsafe { GetLogicalDrives() };
    (0..26u32)
        .filter(|i| mask & (1 << i) != 0)
        .map(|i| PathBuf::from(format!("{}:\\", (b'A' + i as u8) as char)))
        .map(|path| {
            let text = path.to_string_lossy();
            // 탐색기처럼 `로컬 디스크 (C:)`로 보인다 — 이름을 우리가 조립하지 않는다
            let label = icons
                .shell_display_name(&text)
                .unwrap_or_else(|| text.clone().into_owned());
            DriveRow {
                icon: icons.icon_index_for_path(&text),
                network: is_network_drive(&path),
                label,
                path,
                offline: false,
            }
        })
        .collect()
}

/// 이 드라이브가 네트워크 드라이브인가.
///
/// 종류만 묻는 조회라 끊긴 드라이브에서도 즉시 돌아온다(실측 0ms) — 접근을 시도하는
/// `is_reachable`과 비용이 다르다
fn is_network_drive(root: &Path) -> bool {
    let root = HSTRING::from(root.to_string_lossy().as_ref());
    // 안전성: 널 종단 문자열을 넘기는 읽기 전용 조회 — 알 수 없는 드라이브는 0을 반환한다
    unsafe { GetDriveTypeW(&root) == DRIVE_REMOTE }
}

/// 이 드라이브에 지금 닿을 수 있는가.
///
/// **끊긴 네트워크 드라이브에서는 첫 시도가 2.8초까지 걸린다**(실측 — 윈도우가 연결을
/// 다시 맺어 보기 때문이다). 그래서 UI 스레드에서 부르면 안 되고, 워커에서만 쓴다.
///
/// 속성 조회로 판정하는 이유는 그것이 **가장 가벼운 접근**이라서다 — 목록을 읽으면
/// 파일이 많은 뿌리에서 값을 얻는 것과 무관한 비용을 문다
pub fn is_reachable(root: &Path) -> bool {
    let root = HSTRING::from(root.to_string_lossy().as_ref());
    // 안전성: 널 종단 문자열을 넘기는 읽기 전용 조회 — 실패는 INVALID_FILE_ATTRIBUTES로 온다
    unsafe { GetFileAttributesW(&root) != INVALID_FILE_ATTRIBUTES }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::icons::shell_test_guard;

    #[test]
    fn 드라이브_루트는_루트_경로_형태다() {
        // 실제 구성은 PC마다 다르므로 형태만 검증한다 (C: 드라이브는 항상 있다)
        let _shell = shell_test_guard();
        let mut icons = IconCache::new();
        let rows = list_drives(&mut icons);
        assert!(rows.iter().any(|row| row.path == Path::new(r"C:\")));
        assert!(rows.iter().all(|row| row.path.parent().is_none()));
    }

    #[test]
    fn 드라이브는_셸_표시_이름으로_보인다() {
        // `C:\`가 아니라 `로컬 디스크 (C:)` 같은 이름이다.
        // 값 자체는 OS 언어·볼륨 레이블을 따르므로 "경로와 다르다"만 본다
        let _shell = shell_test_guard();
        let mut icons = IconCache::new();
        let rows = list_drives(&mut icons);
        let row = rows
            .iter()
            .find(|row| row.path == Path::new(r"C:\"))
            .expect("C 드라이브");
        assert!(!row.label.is_empty(), "이름이 비었다");
        assert_ne!(
            row.label,
            row.path.to_string_lossy(),
            "경로가 그대로 왔다 — 셸 표시 이름을 거치지 않았다"
        );
    }

    #[test]
    fn 목록은_판정_전이라_아무것도_끊긴_것으로_두지_않는다() {
        // T3·T4 — `list_drives`는 접근을 시도하지 않는다. 배지는 판정이 온 뒤에만 붙는다
        let _shell = shell_test_guard();
        let mut icons = IconCache::new();
        let rows = list_drives(&mut icons);
        assert!(
            rows.iter().all(|row| !row.offline),
            "판정도 하지 않고 끊긴 것으로 표시했다"
        );
    }

    #[test]
    fn 로컬_드라이브는_네트워크가_아니고_닿는다() {
        // C:는 이 PC의 고정 디스크다 — 두 판정의 기준선이 된다
        let _shell = shell_test_guard();
        let root = Path::new(r"C:\");
        assert!(!is_network_drive(root), "고정 디스크를 네트워크로 봤다");
        assert!(is_reachable(root), "C 드라이브에 닿지 못했다");
    }

    #[test]
    fn 없는_드라이브에는_닿지_못한다() {
        // 판정이 늘 참을 돌려주면 배지가 영영 붙지 않는다 — 확실히 닿지 않는 자리로 견준다.
        // 드라이브 문자를 못 박지 않는 이유: PC마다 구성이 달라 `Q:`가 실재할 수 있다.
        // **비트마스크에 없는 문자**를 골라야 이 시험이 어느 PC에서나 같은 것을 본다
        // 안전성: 인자 없는 조회 — 현재 드라이브 비트마스크만 반환한다
        let mask = unsafe { GetLogicalDrives() };
        let unused = (0..26u32)
            .find(|i| mask & (1 << i) == 0)
            .map(|i| PathBuf::from(format!("{}:\\", (b'A' + i as u8) as char)));
        let Some(unused) = unused else {
            // A~Z가 전부 쓰이는 PC — 견줄 자리가 없어 이 시험은 할 일이 없다
            return;
        };
        assert!(
            !is_reachable(&unused),
            "쓰이지 않는 드라이브({})에 닿았다고 했다",
            unused.display()
        );
    }
}
