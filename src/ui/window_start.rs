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
//!
//! 그 작업 영역은 **논리 픽셀로 옮겨서** 넘긴다 — Win32는 물리 픽셀로 주고
//! `ViewportBuilder`는 논리 픽셀로 받는다. 그대로 넘겼더니 배율 150% 화면에서 창이 화면
//! 전체 크기로 떴다가 최대화 순간 세로가 48논리픽셀 줄어, 크기가 튀지 않게 하려던 위
//! 조치가 도리어 무너져 있었다(2026-08-14 실측).
//!
//! **어느 화면의 작업 영역인지는 화면 목록에서 고른다**(`pick_monitor`) — 저장된 좌표가
//! 논리 픽셀이라, 물리 좌표를 받는 `MonitorFromPoint`에 그대로 넣으면 배율이 100%가 아닌
//! 다중 화면에서 이웃 화면을 고른다. 화면 구성은 실행 사이에 바뀔 수 있으므로(화면을 더
//! 붙이거나 뽑거나) 저장해 두지 않고 시작할 때마다 읽고, 창이 있던 화면이 사라졌으면
//! 주 화면으로 물러선다.
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

/// 물리 픽셀 사각형을 논리 픽셀로 옮긴다.
///
/// `ViewportBuilder`의 위치·크기는 논리 픽셀인데 Win32의 모니터 정보는 물리 픽셀이다.
/// 그대로 넘기면 배율이 100%가 아닌 화면에서 창이 그만큼 크게 만들어진다.
///
/// **남는 픽셀은 버린다**(내림) — 올림하면 작업 영역보다 한 픽셀 큰 창이 되어
/// 같은 어긋남이 반대 방향으로 생긴다. 배율이 0 이하면(조회 실패) 그대로 둔다
pub fn to_logical(rect: StartRect, scale: f32) -> StartRect {
    if scale <= 0.0 {
        return rect;
    }
    StartRect {
        x: (rect.x as f32 / scale) as i32,
        y: (rect.y as f32 / scale) as i32,
        w: (rect.w as f32 / scale) as i32,
        h: (rect.h as f32 / scale) as i32,
    }
}

/// 화면 하나 — 창이 어느 화면에 있었는지 고르고 그 화면에 맞춰 띄우는 데 필요한 것들.
///
/// `bounds`·`work`는 Win32가 주는 그대로 **물리 픽셀**이다. 논리 픽셀로 옮기는 것은
/// 화면이 정해진 다음이다 — 배율이 화면마다 다를 수 있어 먼저 옮기면 어느 배율로 옮길지
/// 정할 수 없다
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Monitor {
    /// 화면 전체 — 창이 **어느 화면에 있었는지** 판정하는 데 쓴다
    pub bounds: StartRect,
    /// 작업 표시줄을 뺀 영역 — 최대화로 되살릴 때 이만큼으로 띄운다
    pub work: StartRect,
    /// 그 화면의 배율 (1.5 = 150%)
    pub scale: f32,
}

/// 저장된 창이 놓여 있던 화면을 고른다.
///
/// 저장된 좌표는 **논리 픽셀**이라 물리 좌표계인 화면 사각형과 그대로 견줄 수 없다.
/// 그래서 화면마다 *"이 화면에 있었다면 물리 좌표가 얼마였겠는가"*(논리 × 그 화면의 배율)를
/// 되짚어 견준다 — 화면마다 각자의 배율로 재므로 배율이 섞여 있어도 판정이 갈리지 않는다.
///
/// 판정은 점 하나가 아니라 **창이 그 화면에 얼마나 온전히 드는가**로 한다. 배율이 다른
/// 화면이 섞이면 같은 저장 좌표가 **두 화면 모두에서 말이 되기 때문**이다 — 예를 들어
/// 논리 x=960은 100% 화면에서는 물리 960이고 200% 화면에서는 물리 1920이라, 두 화면이
/// 나란히 있으면 어느 쪽 점이든 상대 화면 안에 든다. 창 전체로 재면 **온전히 들어가는
/// 해석**이 이긴다(부분만 걸치는 쪽은 비율이 낮다).
///
/// 어느 화면에도 걸치지 않으면(그 화면을 뽑았거나 해상도·배치가 바뀐 경우) **주 화면**으로
/// 물러선다. 화면 목록이 비었을 때만 `None`이다
pub fn pick_monitor(window: &WindowState, monitors: &[Monitor]) -> Option<Monitor> {
    let mut best: Option<(f32, Monitor)> = None;
    for monitor in monitors {
        let ratio = coverage(window, monitor);
        // 동점이면 앞선 것을 둔다 — 목록은 주 화면이 맨 앞이다
        if ratio > best.map_or(0.0, |(best, _)| best) {
            best = Some((ratio, *monitor));
        }
    }
    best.map(|(_, monitor)| monitor)
        .or_else(|| monitors.first().copied())
}

/// 그 화면의 배율로 되짚은 창이 화면 안에 드는 비율 (0.0 = 안 걸침, 1.0 = 온전히 들어감).
///
/// 되짚기: 저장된 논리 좌표·크기에 그 화면의 배율을 곱해 *"이 화면에 있었다면 물리로
/// 어디였겠는가"*를 만든다. 넓이 비율로 재므로 배율이 큰 화면이 창을 물리적으로 크게
/// 만든다고 유리해지지 않는다
fn coverage(window: &WindowState, monitor: &Monitor) -> f32 {
    // 배율을 못 읽어 0이 들어와도 창을 잃지 않는다 — 100%로 본다
    let scale = if monitor.scale > 0.0 {
        monitor.scale
    } else {
        1.0
    };
    let x = (window.x as f32 * scale) as i32;
    let y = (window.y as f32 * scale) as i32;
    let w = (window.w as f32 * scale) as i32;
    let h = (window.h as f32 * scale) as i32;
    if w <= 0 || h <= 0 {
        return 0.0;
    }
    let bounds = monitor.bounds;
    let overlap_w = (x + w).min(bounds.x + bounds.w) - x.max(bounds.x);
    let overlap_h = (y + h).min(bounds.y + bounds.h) - y.max(bounds.y);
    if overlap_w <= 0 || overlap_h <= 0 {
        return 0.0;
    }
    (overlap_w as f32 * overlap_h as f32) / (w as f32 * h as f32)
}

/// 저장된 창이 놓여 있던 화면의 작업 영역 — **논리 픽셀**이다.
///
/// 화면을 하나도 못 읽으면 `None`이다 — 호출부가 저장값으로 물러선다
pub fn work_area_for(window: &WindowState) -> Option<StartRect> {
    let picked = pick_monitor(window, &monitors())?;
    Some(to_logical(picked.work, picked.scale))
}

/// 창이 화면에 이만큼은 걸쳐 있어야 "보인다"고 본다 — 넓이의 1/5.
///
/// 한 픽셀만 걸쳐도 그대로 두면 사실상 보이지 않는 창을 살려 둔 셈이 되고, 너무 크게 잡으면
/// 화면 가장자리에 일부러 붙여 둔 창까지 끌어온다
const MIN_VISIBLE: f32 = 0.2;

/// 저장된 자리가 지금 붙어 있는 화면 어디에도 없으면 **주 화면 안으로 당긴 사각형**을 준다.
///
/// 화면 어딘가에 걸쳐 있으면 `None`이다 — 그대로 띄운다. 화면이 여럿일 때 이 판정을
/// **창이 있는 화면 하나만 보고** 하면(egui가 주는 화면 크기가 그렇다) 옆 화면에 둔 창이
/// 늘 "화면 밖"으로 읽혀 주 화면으로 끌려온다(2026-08-14 실측: 옆 화면에 최대화해 둔 창이
/// 뜨자마자 주 화면 306,166으로 옮겨졌다). 그래서 붙어 있는 화면 **전부**를 본다
pub fn rescue_offscreen(window: &WindowState) -> Option<StartRect> {
    let screens = monitors();
    let primary = screens.first()?;
    if screens
        .iter()
        .any(|monitor| coverage(window, monitor) >= MIN_VISIBLE)
    {
        return None;
    }
    Some(clamp_into(window, to_logical(primary.work, primary.scale)))
}

/// 창을 그 화면 작업 영역 안으로 당긴다 — 화면보다 크면 화면 크기로 줄인다
fn clamp_into(window: &WindowState, screen: StartRect) -> StartRect {
    if screen.w <= 0 || screen.h <= 0 {
        // 화면 크기를 모르면 저장값을 그대로 믿는다 — 지어낸 자리보다 낫다
        return StartRect {
            x: window.x,
            y: window.y,
            w: window.w,
            h: window.h,
        };
    }
    let w = window.w.min(screen.w);
    let h = window.h.min(screen.h);
    StartRect {
        x: window.x.clamp(screen.x, screen.x + screen.w - w),
        y: window.y.clamp(screen.y, screen.y + screen.h - h),
        w,
        h,
    }
}

/// 지금 붙어 있는 화면들 — **주 화면이 맨 앞**이다(`pick_monitor`가 물러설 곳).
///
/// 창을 만들기 전에 부르므로 창 핸들에 기대는 조회(`GetDpiForWindow` 등)는 쓸 수 없다.
/// 화면 구성은 실행할 때마다 달라질 수 있어 목록을 들고 있지 않고 그때그때 읽는다
fn monitors() -> Vec<Monitor> {
    use windows::Win32::Foundation::{LPARAM, RECT, TRUE};
    use windows::Win32::Graphics::Gdi::{
        EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFO,
    };
    use windows::Win32::UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI};
    use windows::Win32::UI::WindowsAndMessaging::MONITORINFOF_PRIMARY;
    use windows::core::BOOL;

    unsafe extern "system" fn collect(
        monitor: HMONITOR,
        _hdc: HDC,
        _clip: *mut RECT,
        data: LPARAM,
    ) -> BOOL {
        let mut info = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        let mut dpi_x = 0u32;
        let mut dpi_y = 0u32;
        // 안전성: `data`는 아래 `EnumDisplayMonitors`에 넘긴 `Vec<Monitor>`의 포인터다 —
        // 열거가 끝날 때까지 그 벡터는 살아 있고 콜백은 같은 스레드에서 차례로만 불린다.
        // `info`·`dpi_*`는 이 프레임이 소유하며 `cbSize`를 채워 넘긴다
        unsafe {
            if !GetMonitorInfoW(monitor, &mut info).as_bool() {
                return TRUE; // 이 화면만 건너뛰고 열거는 계속한다
            }
            let dpi_ok =
                GetDpiForMonitor(monitor, MDT_EFFECTIVE_DPI, &mut dpi_x, &mut dpi_y).is_ok();
            let out = &mut *(data.0 as *mut Vec<Monitor>);
            // 배율을 못 읽으면 100%로 본다 — 그 화면에서는 물리와 논리가 같다
            let scale = if dpi_ok { dpi_x as f32 / 96.0 } else { 1.0 };
            let found = Monitor {
                bounds: to_rect(info.rcMonitor),
                work: to_rect(info.rcWork),
                scale,
            };
            // 주 화면을 맨 앞에 둔다 — 창이 어느 화면에도 들지 않을 때 물러설 곳이다
            if info.dwFlags & MONITORINFOF_PRIMARY != 0 {
                out.insert(0, found);
            } else {
                out.push(found);
            }
        }
        TRUE
    }

    let mut found: Vec<Monitor> = Vec::new();
    // 안전성: 열거는 이 호출 안에서 끝나고 그동안 `found`는 이 프레임에 살아 있다.
    // 화면 전체를 훑도록 장치 컨텍스트와 자를 영역은 주지 않는다
    unsafe {
        let _ = EnumDisplayMonitors(
            None,
            None,
            Some(collect),
            LPARAM(&mut found as *mut Vec<Monitor> as isize),
        );
    }
    found
}

fn to_rect(rect: windows::Win32::Foundation::RECT) -> StartRect {
    StartRect {
        x: rect.left,
        y: rect.top,
        w: rect.right - rect.left,
        h: rect.bottom - rect.top,
    }
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

    #[test]
    fn 작업_영역을_그_화면의_배율로_나눈다() {
        // 150% 화면의 실측값 — 물리 2560x1528이 최대화된 창의 논리 크기 1706x1018이 된다.
        // 나누지 않고 넘기면 winit이 물리 3840x2292 창을 만들려 해 화면 크기로 잘리고,
        // 첫 프레임 뒤 최대화가 걸리는 순간 세로가 48만큼 줄며 화면 전체가 다시 배치된다
        let physical = StartRect {
            x: 0,
            y: 0,
            w: 2560,
            h: 1528,
        };
        assert_eq!(
            to_logical(physical, 1.5),
            StartRect {
                x: 0,
                y: 0,
                w: 1706,
                h: 1018
            }
        );
    }

    #[test]
    fn 배율이_100퍼센트면_그대로다() {
        assert_eq!(to_logical(work(), 1.0), work());
    }

    #[test]
    fn 배율을_모르면_그대로_둔다() {
        // 배율 조회가 실패해 0이 넘어와도 0으로 나눠 창을 잃지 않는다
        assert_eq!(to_logical(work(), 0.0), work());
    }

    /// 화면 하나 — 물리 사각형과 배율만 주면 되는 자리라 짧게 만든다
    fn monitor(x: i32, w: i32, h: i32, scale: f32) -> Monitor {
        Monitor {
            bounds: StartRect { x, y: 0, w, h },
            // 아래쪽 작업 표시줄 72픽셀을 뺀 자리
            work: StartRect {
                x,
                y: 0,
                w,
                h: h - 72,
            },
            scale,
        }
    }

    /// 그 화면 **왼쪽 위에 붙여** 최대화해 둔 창의 저장 상태 — 좌표는 **논리 픽셀**이다.
    ///
    /// 한가운데가 아니라 왼쪽 위인 이유: 논리 좌표를 물리 좌표계에 그대로 견주는 옛 판정은
    /// 창이 화면 한가운데 있으면 **우연히 맞는다**(나눈 값이 그 화면 범위에 그대로 떨어진다).
    /// 왼쪽에 붙은 창이라야 판정이 실제로 갈린다
    fn saved_on(monitor: &Monitor) -> WindowState {
        WindowState {
            x: (monitor.bounds.x as f32 / monitor.scale) as i32,
            y: (monitor.bounds.y as f32 / monitor.scale) as i32,
            w: 1398,
            h: 932,
            maximized: true,
        }
    }

    #[test]
    fn 화면이_하나면_그_화면을_고른다() {
        let only = monitor(0, 2560, 1600, 1.5);
        assert_eq!(pick_monitor(&saved_on(&only), &[only]), Some(only));
    }

    #[test]
    fn 배율이_같은_두_화면에서_창이_있던_쪽을_고른다() {
        // 저장된 좌표는 논리 픽셀이라 물리 좌표계인 화면 사각형과 단위가 다르다 —
        // 되짚지 않고 그대로 견주면 둘째 화면의 창이 첫째 화면 안에 든 것처럼 보인다
        let first = monitor(0, 2560, 1600, 1.5);
        let second = monitor(2560, 2560, 1600, 1.5);
        let screens = [first, second];
        assert_eq!(pick_monitor(&saved_on(&second), &screens), Some(second));
        assert_eq!(pick_monitor(&saved_on(&first), &screens), Some(first));
    }

    #[test]
    fn 배율이_다른_두_화면에서도_창이_있던_쪽을_고른다() {
        // 주 화면 100%, 옆 화면 200% — 화면마다 각자의 배율로 되짚어야 갈린다
        let first = monitor(0, 1920, 1080, 1.0);
        let second = monitor(1920, 3840, 2160, 2.0);
        let screens = [first, second];
        assert_eq!(pick_monitor(&saved_on(&second), &screens), Some(second));
        assert_eq!(pick_monitor(&saved_on(&first), &screens), Some(first));
    }

    #[test]
    fn 두_화면_모두에서_말이_되면_온전히_드는_쪽을_고른다() {
        // 논리 x=960은 100% 화면에서 물리 960, 200% 화면에서 물리 1920이라 두 해석이 다
        // 성립한다. 점 하나로 재면 앞선 화면이 먼저 잡히지만, 창 전체로 재면 잘리지 않는
        // 쪽이 이긴다 — 저장된 창은 옆 화면 왼쪽에 온전히 놓여 있었다
        let first = monitor(0, 1920, 1080, 1.0);
        let second = monitor(1920, 3840, 2160, 2.0);
        let saved = WindowState {
            x: 960,
            y: 0,
            w: 1398,
            h: 932,
            maximized: true,
        };
        assert_eq!(pick_monitor(&saved, &[first, second]), Some(second));
        assert!(
            coverage(&saved, &second) > coverage(&saved, &first),
            "옆 화면에 온전히 드는데 주 화면 쪽 비율이 더 높다"
        );
    }

    #[test]
    fn 화면_밖_창은_주_화면_안으로_들어온다() {
        // 화면을 떼거나 배치를 바꿔 지난번 좌표가 어디에도 없는 경우 — 그대로 띄우면
        // 창이 보이지 않는다 (옛 `session::clamp_window` 시험을 이 자리로 옮겼다)
        let screen = StartRect {
            x: 0,
            y: 0,
            w: 1920,
            h: 1080,
        };
        let far = WindowState {
            x: 5000,
            y: -300,
            w: 1200,
            h: 800,
            maximized: false,
        };
        let fixed = clamp_into(&far, screen);
        assert_eq!(fixed.x, 1920 - 1200);
        assert_eq!(fixed.y, 0);
        assert_eq!((fixed.w, fixed.h), (1200, 800));
    }

    #[test]
    fn 화면보다_큰_창은_화면_크기로_줄인다() {
        let screen = StartRect {
            x: 0,
            y: 0,
            w: 1920,
            h: 1080,
        };
        let huge = WindowState {
            x: 0,
            y: 0,
            w: 4000,
            h: 3000,
            maximized: false,
        };
        assert_eq!(
            clamp_into(&huge, screen),
            StartRect {
                x: 0,
                y: 0,
                w: 1920,
                h: 1080
            }
        );
    }

    #[test]
    fn 화면_크기를_모르면_저장값을_그대로_쓴다() {
        let saved = WindowState {
            x: 100,
            y: 50,
            w: 1200,
            h: 800,
            maximized: false,
        };
        let unknown = StartRect {
            x: 0,
            y: 0,
            w: 0,
            h: 0,
        };
        assert_eq!(
            clamp_into(&saved, unknown),
            StartRect {
                x: 100,
                y: 50,
                w: 1200,
                h: 800
            }
        );
    }

    #[test]
    fn 주_화면이_원점이_아니어도_그_화면_안으로_들어온다() {
        // 주 화면이 왼쪽 화면 오른편에 놓인 배치 — 0,0으로 당기면 남의 화면에 떨어진다
        let screen = StartRect {
            x: 1920,
            y: 0,
            w: 1920,
            h: 1080,
        };
        let far = WindowState {
            x: 9000,
            y: 9000,
            w: 1200,
            h: 800,
            maximized: false,
        };
        let fixed = clamp_into(&far, screen);
        assert_eq!(fixed.x, 1920 + 1920 - 1200);
        assert_eq!(fixed.y, 1080 - 800);
    }

    #[test]
    fn 옆_화면에_둔_창은_끌어오지_않는다() {
        // 화면 하나만 보고 판정하면 옆 화면의 창이 늘 "화면 밖"이 되어 주 화면으로 끌려간다
        let second = monitor(1920, 1920, 1080, 1.0);
        let saved = saved_on(&second);
        assert!(
            coverage(&saved, &second) >= MIN_VISIBLE,
            "옆 화면에 온전히 놓인 창이 화면 밖으로 읽힌다"
        );
    }

    #[test]
    fn 어느_화면에도_걸치지_않으면_주_화면으로_물러선다() {
        // 화면 배치가 바뀌어 저장된 자리가 어느 화면에도 없는 경우
        let primary = monitor(0, 1920, 1080, 1.0);
        let saved = WindowState {
            x: 30_000,
            y: 30_000,
            w: 1398,
            h: 932,
            maximized: true,
        };
        assert_eq!(coverage(&saved, &primary), 0.0);
        assert_eq!(pick_monitor(&saved, &[primary]), Some(primary));
    }

    #[test]
    fn 쓰던_화면이_사라지면_주_화면으로_물러선다() {
        // 둘째 화면에 두고 껐다가 그 화면을 뽑고 실행한 경우 — 창을 잃지 않는다
        let first = monitor(0, 1920, 1080, 1.0);
        let second = monitor(1920, 3840, 2160, 2.0);
        assert_eq!(
            pick_monitor(&saved_on(&second), &[first]),
            Some(first),
            "없는 화면 자리에 창을 띄우려 한다"
        );
    }

    #[test]
    fn 화면을_하나도_못_읽으면_저장값으로_물러선다() {
        // 목록이 비면 고를 것이 없다 — `start_rect`가 저장된 사각형을 그대로 쓴다
        let only = monitor(0, 2560, 1600, 1.5);
        let saved = saved_on(&only);
        assert_eq!(pick_monitor(&saved, &[]), None);
        assert_eq!(
            start_rect(&saved, None),
            StartRect {
                x: saved.x,
                y: saved.y,
                w: 1398,
                h: 932
            }
        );
    }

    #[test]
    fn 단일에서_다중으로_늘어도_주_화면의_창은_그대로다() {
        // 화면 하나로 쓰다가 하나를 더 붙인 경우 — 주 화면에 있던 창은 계속 주 화면이다
        let primary = monitor(0, 2560, 1600, 1.5);
        let added = monitor(2560, 1920, 1080, 1.0);
        assert_eq!(pick_monitor(&saved_on(&primary), &[primary]), Some(primary));
        assert_eq!(
            pick_monitor(&saved_on(&primary), &[primary, added]),
            Some(primary)
        );
    }
}
