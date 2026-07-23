//! 디렉터리 변경 감시 — 전용 스레드 + 디바운스 (FR-10, plan D16)
//!
//! ReadDirectoryChangesW(overlapped)를 전용 스레드에서 대기하고, 변경이 오면
//! 300ms 디바운스 후 채널 송신 + `WM_APP_DIR_CHANGED` 게시로 알린다.
//! 변경 항목 단위 부분 갱신은 하지 않는다 — 통지 수신 측이 전체 재열거한다 (D16).
//! 정지는 이벤트 신호로 결정적이다 (탭 고속 전환 시 스레드 누수 금지 — T3 Edge).
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;
use windows::Win32::Foundation::{CloseHandle, HANDLE, LPARAM, WAIT_OBJECT_0, WPARAM};
use windows::Win32::Storage::FileSystem::{
    CreateFileW, FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OVERLAPPED, FILE_LIST_DIRECTORY,
    FILE_NOTIFY_CHANGE_ATTRIBUTES, FILE_NOTIFY_CHANGE_DIR_NAME, FILE_NOTIFY_CHANGE_FILE_NAME,
    FILE_NOTIFY_CHANGE_LAST_WRITE, FILE_NOTIFY_CHANGE_SIZE, FILE_SHARE_DELETE, FILE_SHARE_READ,
    FILE_SHARE_WRITE, OPEN_EXISTING, ReadDirectoryChangesW,
};
use windows::Win32::System::IO::{CancelIo, GetOverlappedResult, OVERLAPPED};
use windows::Win32::System::Threading::{
    CreateEventW, INFINITE, ResetEvent, SetEvent, WaitForMultipleObjects,
};
use windows::Win32::UI::WindowsAndMessaging::{PostMessageW, WM_APP};
use windows::core::HSTRING;

/// 감시 폴더에 변경이 있었음을 알리는 메시지 (데이터 없음 — 수신 측이 전체 재열거)
pub const WM_APP_DIR_CHANGED: u32 = WM_APP + 9;

/// 첫 변경 후 통지까지 묶어 기다리는 시간 — 대량 변경 폭주 억제 (plan D16)
const DEBOUNCE_MS: u32 = 300;

/// 변경 통지 버퍼 (64KB). u32 배열인 이유: FILE_NOTIFY_INFORMATION은 DWORD 정렬 요구.
/// 내용은 파싱하지 않으므로(전체 재열거) 크기·정렬만 의미 있다
const BUF_LEN_U32: usize = 16 * 1024;

/// HANDLE을 감시 스레드로 넘기기 위한 래퍼.
/// 안전성: HANDLE은 값 타입 핸들이며 이벤트 신호·메시지 게시는 스레드 간 안전하다
struct HandleSend(isize);
unsafe impl Send for HandleSend {}

/// 폴더 1개를 감시하는 워커. Drop 시 스레드를 정지·회수한다
pub struct DirWatcher {
    path: PathBuf,
    /// 정지 신호용 수동 리셋 이벤트 — 스레드의 모든 대기 지점이 함께 듣는다
    stop_event: HANDLE,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl DirWatcher {
    /// 감시 시작. 변경이 디바운스된 뒤 tx로 ()를 보내고 notify 창에 WM_APP_DIR_CHANGED를 게시한다.
    /// 폴더 열기 실패(네트워크 드라이브 등)는 스레드가 조용히 종료 — 감시 없이 동작 (T3 Edge)
    pub fn start(
        path: PathBuf,
        tx: Sender<()>,
        notify: Option<windows::Win32::Foundation::HWND>,
    ) -> DirWatcher {
        // 안전성: 이벤트 생성 실패는 극히 예외적 — 실패 시 무효 핸들로 두면
        // 스레드가 대기 오류로 즉시 종료해 "감시 없이 동작"으로 저하된다
        let stop_event = unsafe { CreateEventW(None, true, false, None) }.unwrap_or_default();
        let stop = HandleSend(stop_event.0 as isize);
        let hwnd = HandleSend(notify.map_or(0, |h| h.0 as isize));
        let watch_path = path.clone();
        let thread = std::thread::spawn(move || {
            watch_loop(
                &watch_path,
                HANDLE(stop.0 as *mut core::ffi::c_void),
                tx,
                hwnd,
            );
        });
        DirWatcher {
            path,
            stop_event,
            thread: Some(thread),
        }
    }

    /// 감시 중 경로 — 활성 탭 경로와 대조해 재시작 여부 판정용
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for DirWatcher {
    fn drop(&mut self) {
        // 안전성: 유효한 이벤트에 정지 신호 → 스레드는 모든 대기 지점에서 즉시 깨어나 종료
        unsafe {
            let _ = SetEvent(self.stop_event);
        }
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
        // 안전성: 스레드 종료 후이므로 이벤트 핸들을 참조하는 곳이 없다
        unsafe {
            let _ = CloseHandle(self.stop_event);
        }
    }
}

/// 감시 본체 (워커 스레드). 오류(폴더 삭제 등)는 마지막 통지 1회 후 종료 —
/// 수신 측 재열거가 접근 불가 문구를 표시한다 (T3 Edge, D6)
fn watch_loop(path: &Path, stop: HANDLE, tx: Sender<()>, notify: HandleSend) {
    // 안전성: 감시 대상 폴더 핸들 — 함수 끝에서 CloseHandle
    let Ok(dir) = (unsafe {
        CreateFileW(
            &HSTRING::from(path.as_os_str()),
            FILE_LIST_DIRECTORY.0,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            None,
            OPEN_EXISTING,
            FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OVERLAPPED,
            None,
        )
    }) else {
        return; // 열기 실패 — 감시 없이 동작 (F5 수동 갱신만)
    };
    // 안전성: overlapped 완료 이벤트 — 함수 끝에서 CloseHandle
    let Ok(io_event) = (unsafe { CreateEventW(None, true, false, None) }) else {
        // 안전성: dir은 위에서 연 유효 핸들
        unsafe {
            let _ = CloseHandle(dir);
        }
        return;
    };

    let mut buf = vec![0u32; BUF_LEN_U32];
    let filter = FILE_NOTIFY_CHANGE_FILE_NAME
        | FILE_NOTIFY_CHANGE_DIR_NAME
        | FILE_NOTIFY_CHANGE_ATTRIBUTES
        | FILE_NOTIFY_CHANGE_SIZE
        | FILE_NOTIFY_CHANGE_LAST_WRITE;
    // overlapped는 발행이 걸린 채 반복을 넘나들므로 루프 밖에 고정한다 (주소 안정).
    // 불변식: 'watch 루프 진입 시점에는 항상 발행이 살아있다 — 발행 사이 변경도
    // OS가 핸들에 버퍼링하지만, 발행을 유지하면 대기·소비 로직이 단순해진다
    let mut overlapped = OVERLAPPED {
        hEvent: io_event,
        ..Default::default()
    };

    // 대기 결과 — WAIT_FAILED는 Timeout으로 묶는다 (창 종료로 수렴해 다음 발행이 오류를 드러냄)
    enum Waited {
        Stop,
        Io,
        Timeout,
    }
    let wait_both = |timeout: u32| {
        // 안전성: 두 핸들 모두 이 스레드가 살아있는 동안 유효 — 인덱스 0(stop) 우선
        let w = unsafe { WaitForMultipleObjects(&[stop, io_event], false, timeout) };
        if w == WAIT_OBJECT_0 {
            Waited::Stop
        } else if w.0 == WAIT_OBJECT_0.0 + 1 {
            Waited::Io
        } else {
            Waited::Timeout
        }
    };
    let consume = |overlapped: &OVERLAPPED| {
        let mut bytes = 0u32;
        // 오류·bytes==0(버퍼 오버플로)도 "변경 있음"으로 흡수 — 전체 재열거라 동일 (T3 Edge)
        // 안전성: 완료된 overlapped 결과 조회
        unsafe {
            let _ = GetOverlappedResult(dir, overlapped, &mut bytes, false);
        }
    };
    let issue = |buf: &mut [u32], overlapped: &mut OVERLAPPED| {
        // 안전성: dir·buf·overlapped·io_event는 호출 동안 유효 (스레드 지역 소유)
        unsafe {
            let _ = ResetEvent(io_event);
            ReadDirectoryChangesW(
                dir,
                buf.as_mut_ptr() as *mut core::ffi::c_void,
                std::mem::size_of_val(buf) as u32,
                false, // 하위 폴더 미감시 — 표시 중 폴더만 (FR-10)
                filter,
                None,
                Some(overlapped),
                None,
            )
            .is_ok()
        }
    };

    if issue(&mut buf, &mut overlapped) {
        'watch: loop {
            // 첫 변경 대기
            match wait_both(INFINITE) {
                Waited::Stop => break 'watch,
                Waited::Io => consume(&overlapped),
                Waited::Timeout => {} // INFINITE에선 WAIT_FAILED뿐 — 아래 재발행이 오류를 드러냄
            }
            // 디바운스 — 조용한 300ms가 확인될 때까지 후속 완료를 흡수한 뒤 1회만 통지
            loop {
                if !issue(&mut buf, &mut overlapped) {
                    // 재발행 실패(폴더 삭제 등) — 마지막 통지 후 종료:
                    // 수신 측 재열거가 접근 불가 문구를 표시한다
                    notify_once(&tx, &notify);
                    break 'watch;
                }
                match wait_both(DEBOUNCE_MS) {
                    Waited::Stop => break 'watch,
                    // 창 안의 후속 변경 — 소비하고 창을 다시 연다
                    Waited::Io => consume(&overlapped),
                    // 조용 300ms — 발행은 pending인 채 통지 후 첫 대기로 복귀 (불변식 유지)
                    Waited::Timeout => break,
                }
            }
            notify_once(&tx, &notify);
        }
    } else {
        // 최초 발행 실패 — 통지 1회 후 감시 없이 종료 (수신 측이 상태 문구 표시)
        notify_once(&tx, &notify);
    }

    // 안전성: 이 스레드만 쓰는 핸들 정리 — 잔여 발행을 취소하고 그 완료까지 기다린 뒤
    // 닫는다. 대기 없이 반환하면 취소 중인 I/O가 스택 버퍼(buf)에 늦게 써서
    // 해제 후 쓰기가 될 수 있다 (bwait=true가 취소 완료를 동기화)
    unsafe {
        let _ = CancelIo(dir);
        let mut bytes = 0u32;
        let _ = GetOverlappedResult(dir, &overlapped, &mut bytes, true);
        let _ = CloseHandle(io_event);
        let _ = CloseHandle(dir);
    }
}

/// 채널 송신 + 창 메시지 게시 (창 파괴·수신자 소멸은 무해 실패)
fn notify_once(tx: &Sender<()>, notify: &HandleSend) {
    let _ = tx.send(());
    if notify.0 != 0 {
        // 안전성: PostMessageW는 스레드 간 안전 — 창이 파괴됐으면 실패만 반환
        unsafe {
            let _ = PostMessageW(
                Some(windows::Win32::Foundation::HWND(
                    notify.0 as *mut core::ffi::c_void,
                )),
                WM_APP_DIR_CHANGED,
                WPARAM(0),
                LPARAM(0),
            );
        }
    }
}
