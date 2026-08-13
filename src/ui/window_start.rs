//! 창을 **어떤 모습으로 띄울지** 정한다 — 시작 순간의 깜빡임을 없애기 위한 규칙.
//!
//! 세션이 최대화 상태였을 때 `ViewportBuilder::with_maximized(true)`를 그대로 주면
//! winit가 창을 만든 직후 `set_maximized(true)`를 부르고(`window.rs:1402`), 그것이
//! `ShowWindow(hwnd, SW_MAXIMIZE)`로 이어진다(`window_state.rs:363`).
//! **`SW_MAXIMIZE`는 창을 표시한다** — eframe이 흰 화면을 막으려고 `with_visible(false)`로
//! 숨겨 만든 창이(glow_integration.rs:165) 이 순간 강제로 드러나고, 아직 GL로 아무것도
//! 그리지 않아 **흰 사각형이 번쩍인다**(2026-08-13 실측: 창 생성 223ms 전부터 약 0.2초).
//!
//! 그래서 최대화는 **첫 프레임을 그린 뒤**(`ui::app`이 `ViewportCommand::Maximized`로) 건다.
//! 대신 창 자체는 처음부터 **그 모니터의 작업 영역**만 하게 띄운다 — 저장된 일반 크기로
//! 띄우면 최대화가 걸리는 순간 크기가 튀는 것이 보인다.
use crate::app::settings::WindowState;

/// 창을 띄울 사각형 — 위치와 크기
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StartRect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

/// 저장된 창 상태와 그 자리의 작업 영역으로 **띄울 사각형**을 정한다.
///
/// 최대화 상태가 아니면 저장된 사각형 그대로다. 최대화였다면 작업 영역을 쓰되,
/// 작업 영역을 알 수 없으면(모니터 조회 실패) 저장된 사각형으로 물러선다 —
/// 크기가 튀는 것이 창이 엉뚱한 곳에 뜨는 것보다 낫다
pub fn start_rect(window: &WindowState, work_area: Option<StartRect>) -> StartRect {
    let saved = StartRect {
        x: window.x,
        y: window.y,
        w: window.w,
        h: window.h,
    };
    if !window.maximized {
        return saved;
    }
    work_area.unwrap_or(saved)
}

/// 그 점이 놓인 모니터의 작업 영역(작업 표시줄을 뺀 영역).
///
/// 모니터가 없거나 조회에 실패하면 `None`이다 — 호출부가 저장값으로 물러선다
pub fn work_area_at(x: i32, y: i32) -> Option<StartRect> {
    use windows::Win32::Foundation::POINT;
    use windows::Win32::Graphics::Gdi::{
        GetMonitorInfoW, MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromPoint,
    };
    let point = POINT { x, y };
    let mut info = MONITORINFO {
        cbSize: std::mem::size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    // 안전성: `point`·`info`는 스택 소유이고 `cbSize`를 채워 넘긴다.
    // `MonitorFromPoint`는 `MONITOR_DEFAULTTONEAREST`라 늘 유효한 핸들을 준다
    let ok = unsafe {
        let monitor = MonitorFromPoint(point, MONITOR_DEFAULTTONEAREST);
        GetMonitorInfoW(monitor, &mut info).as_bool()
    };
    if !ok {
        return None;
    }
    let work = info.rcWork;
    Some(StartRect {
        x: work.left,
        y: work.top,
        w: work.right - work.left,
        h: work.bottom - work.top,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn window(maximized: bool) -> WindowState {
        WindowState {
            x: 155,
            y: 57,
            w: 1398,
            h: 932,
            maximized,
        }
    }

    fn work() -> StartRect {
        StartRect {
            x: 0,
            y: 0,
            w: 2560,
            h: 1040,
        }
    }

    #[test]
    fn 최대화가_아니면_저장된_자리_그대로다() {
        let rect = start_rect(&window(false), Some(work()));
        assert_eq!(
            rect,
            StartRect {
                x: 155,
                y: 57,
                w: 1398,
                h: 932
            },
            "일반 창인데 작업 영역으로 띄웠다"
        );
    }

    #[test]
    fn 최대화였으면_작업_영역만_하게_띄운다() {
        // 저장된 일반 크기로 띄우면 첫 프레임 뒤 최대화가 걸릴 때 크기가 튀는 것이 보인다
        assert_eq!(start_rect(&window(true), Some(work())), work());
    }

    #[test]
    fn 작업_영역을_모르면_저장된_자리로_물러선다() {
        let rect = start_rect(&window(true), None);
        assert_eq!(rect.w, 1398, "모니터를 몰라 크기를 지어냈다");
        assert_eq!(rect.x, 155);
    }
}
