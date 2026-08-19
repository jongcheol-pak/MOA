//! 앱 아이콘 — `docs/AppIcon.ico`를 실행 파일에 담아 창 아이콘·타이틀바 아이콘으로 쓴다.
//!
//! ICO 안에는 크기별 이미지가 여러 장 들어 있다. 여기서는 **32bpp DIB(BMP) 항목만** 읽어
//! RGBA로 펼친다 — 256px 항목은 PNG로 담겨 있는데, 그것을 풀려면 PNG 디코더(새 의존성)가
//! 필요한 반면 타이틀바(20px)·창 아이콘(64px)에는 작은 항목이면 충분하기 때문이다.
//!
//! 실행 파일 자체의 아이콘은 `build.rs`가 같은 파일을 리소스로 담아 처리한다.
//! 이 모듈이 다루는 것은 **창 안에 그릴 픽셀**과, 그 리소스를 **창에 붙이는 일**
//! (`apply_to_window` — 작업 표시줄·Alt+Tab이 읽는 창 아이콘)이다.
use eframe::egui;
use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    GetSystemMetrics, ICON_BIG, ICON_SMALL, IMAGE_ICON, LR_DEFAULTCOLOR, LR_SHARED, LoadImageW,
    SM_CXICON, SM_CXSMICON, SM_CYICON, SM_CYSMICON, SendMessageW, WM_SETICON,
};
use windows::core::PCWSTR;

/// 아이콘 원본 — exe에 정적으로 담긴다(실행 시 파일을 찾지 않는다)
pub const ICO_BYTES: &[u8] = include_bytes!("../../docs/AppIcon.ico");

/// ICO 디렉터리 항목 하나 (16바이트 고정)
const DIR_ENTRY_SIZE: usize = 16;
/// `BITMAPINFOHEADER` 크기 — 이 값이어야 우리가 아는 DIB다
const DIB_HEADER_SIZE: u32 = 40;

/// 펼쳐 놓은 아이콘 한 장
pub struct IconImage {
    pub width: u32,
    pub height: u32,
    /// 스트레이트 알파 RGBA, 위에서 아래로
    pub rgba: Vec<u8>,
}

/// 창 아이콘(작업 표시줄·Alt+Tab)으로 쓸 이미지. 실패하면 `None` — OS 기본 아이콘이 쓰인다
pub fn icon_data() -> Option<egui::IconData> {
    let image = decode(ICO_BYTES, 64)?;
    Some(egui::IconData {
        rgba: image.rgba,
        width: image.width,
        height: image.height,
    })
}

/// exe 리소스의 아이콘 그룹 id — `build.rs`가 이 번호로 담는다(그쪽 `GROUP_ID`와 짝이다)
const ICON_GROUP_ID: u16 = 1;

/// exe 리소스의 아이콘을 이 창의 큰·작은 아이콘으로 붙인다 (작업 표시줄·Alt+Tab).
///
/// **eframe도 창 아이콘을 설정하지만 작업 표시줄이 그 값을 집어가지 못한다** — 창 아이콘
/// (`WM_GETICON`)과 exe 리소스가 모두 정상인데 버튼만 Windows 기본 아이콘이었다
/// (2026-08-19 실측). 원인은 아래 재전송 주석에 적은 **같은 핸들 재설정**이며,
/// eframe이 붙인 값을 그대로 다시 넣어서는 바뀌지 않는다. 호출 시점은 `ui::app`이 쥔다.
///
/// 아이콘을 ICO에서 직접 펼치지 않고 `LoadImageW`로 exe 리소스에서 얻는 이유: OS가
/// 요청한 크기에 가장 알맞은 항목을 스스로 고른다. `LR_SHARED`라 수명도 시스템이 쥐어
/// `DestroyIcon`이 필요 없고, 여러 번 불러도 핸들이 새지 않는다
pub fn apply_to_window(hwnd: HWND) {
    // 안전성: 인자 없는 `GetModuleHandleW`는 이 실행 파일의 인스턴스를 돌려준다(실패 없음에
    // 가깝지만 반환이 `Result`라 그대로 받는다). 이어지는 호출은 그 핸들과 방금 받은 창
    // 핸들만 쓰며, 아이콘은 `LR_SHARED`라 우리가 해제하지 않는다
    unsafe {
        let Ok(instance) = GetModuleHandleW(PCWSTR::null()) else {
            return;
        };
        let name = PCWSTR(ICON_GROUP_ID as usize as *const u16);
        for (kind, cx, cy) in [
            (ICON_BIG, SM_CXICON, SM_CYICON),
            (ICON_SMALL, SM_CXSMICON, SM_CYSMICON),
        ] {
            let size = (GetSystemMetrics(cx), GetSystemMetrics(cy));
            // 실패하면 그 크기는 건너뛴다 — OS 기본 아이콘이 남을 뿐 앱은 그대로 돈다
            let Ok(icon) = LoadImageW(
                Some(instance.into()),
                name,
                IMAGE_ICON,
                size.0,
                size.1,
                LR_DEFAULTCOLOR | LR_SHARED,
            ) else {
                continue;
            };
            // **비웠다가 다시 붙인다** — 같은 핸들을 다시 넣으면 값이 그대로라
            // 작업 표시줄이 변화를 알아채지 못한다(2026-08-19 실측: 창 아이콘은 제대로
            // 붙었는데 버튼은 기본 아이콘이었고, 바깥에서 **다른** 핸들을 보냈을 때만
            // 바뀌었다). 지운 상태는 다음 줄에서 곧바로 덮이므로 화면에 드러나지 않는다
            SendMessageW(hwnd, WM_SETICON, Some(WPARAM(kind as usize)), None);
            SendMessageW(
                hwnd,
                WM_SETICON,
                Some(WPARAM(kind as usize)),
                Some(LPARAM(icon.0 as isize)),
            );
        }
    }
}

/// 타이틀바에 그릴 텍스처. 한 번만 만들어 앱이 들고 있는다
pub fn load_texture(ctx: &egui::Context, size: u32) -> Option<egui::TextureHandle> {
    let image = decode(ICO_BYTES, size)?;
    let color = egui::ColorImage::from_rgba_unmultiplied(
        [image.width as usize, image.height as usize],
        &image.rgba,
    );
    Some(ctx.load_texture("app_icon", color, egui::TextureOptions::LINEAR))
}

/// ICO에서 `preferred`(px)에 가장 알맞은 32bpp 항목을 골라 RGBA로 펼친다.
///
/// 고르는 기준은 "`preferred` 이상 중 가장 작은 것"이다 — 확대는 뭉개지지만 축소는
/// 깨끗하기 때문이다. 그보다 큰 항목이 없으면 있는 것 중 가장 큰 것을 쓴다
pub fn decode(bytes: &[u8], preferred: u32) -> Option<IconImage> {
    let count = read_u16(bytes, 4)? as usize;
    // 항목 형식(reserved=0, type=1)이 아니면 ICO가 아니다
    if read_u16(bytes, 0)? != 0 || read_u16(bytes, 2)? != 1 || count == 0 {
        return None;
    }
    let mut best: Option<(u32, usize, usize)> = None; // (한 변, 데이터 시작, 길이)
    for index in 0..count {
        let entry = 6 + index * DIR_ENTRY_SIZE;
        // 폭·높이 0은 256px을 뜻한다 (1바이트에 담기지 않아 정해 둔 약속)
        let width = match *bytes.get(entry)? {
            0 => 256,
            w => w as u32,
        };
        let offset = read_u32(bytes, entry + 12)? as usize;
        let size = read_u32(bytes, entry + 8)? as usize;
        let data = bytes.get(offset..offset.checked_add(size)?)?;
        // PNG로 담긴 항목은 건너뛴다 (헤더 크기 자리가 40이 아니다)
        if read_u32(data, 0)? != DIB_HEADER_SIZE || read_u16(data, 14)? != 32 {
            continue;
        }
        if best.is_none_or(|(current, _, _)| is_better(width, current, preferred)) {
            best = Some((width, offset, size));
        }
    }
    let (_, offset, size) = best?;
    dib_to_rgba(&bytes[offset..offset + size])
}

/// `preferred`에 비추어 `candidate`가 `current`보다 나은 크기인가
fn is_better(candidate: u32, current: u32, preferred: u32) -> bool {
    match (candidate >= preferred, current >= preferred) {
        // 둘 다 충분하면 작은 쪽(덜 줄여도 되는 쪽)
        (true, true) => candidate < current,
        (true, false) => true,
        (false, true) => false,
        // 둘 다 모자라면 큰 쪽(덜 늘려도 되는 쪽)
        (false, false) => candidate > current,
    }
}

/// ICO 안의 DIB 한 장을 RGBA로 펼친다.
///
/// 아이콘의 DIB는 높이가 **실제의 두 배**로 적혀 있다 — 색 픽셀 뒤에 1bpp 마스크가
/// 이어 붙기 때문이다. 32bpp 항목은 알파를 스스로 들고 있어 그 마스크는 읽지 않는다
fn dib_to_rgba(data: &[u8]) -> Option<IconImage> {
    let width = read_u32(data, 4)?;
    let height = read_u32(data, 8)? / 2;
    // BI_RGB(무압축)만 읽는다 — 아이콘에 압축 DIB가 쓰이는 일은 없다
    if width == 0 || height == 0 || read_u32(data, 16)? != 0 {
        return None;
    }
    let pixels = data.get(DIB_HEADER_SIZE as usize..)?;
    let (w, h) = (width as usize, height as usize);
    let needed = w.checked_mul(h)?.checked_mul(4)?;
    if pixels.len() < needed {
        return None;
    }
    // DIB는 아래에서 위로 쌓이고 BGRA 순서다 — 행을 뒤집으며 RGBA로 옮긴다
    let mut rgba = vec![0u8; needed];
    for y in 0..h {
        let src = (h - 1 - y) * w * 4;
        let dst = y * w * 4;
        for x in 0..w {
            let (s, d) = (src + x * 4, dst + x * 4);
            rgba[d] = pixels[s + 2];
            rgba[d + 1] = pixels[s + 1];
            rgba[d + 2] = pixels[s];
            rgba[d + 3] = pixels[s + 3];
        }
    }
    // 알파가 전부 0이면 32bpp인데 알파 채널을 쓰지 않는 아이콘이다 — 불투명으로 되살린다
    // (`ui::icon_tex`의 시스템 아이콘 변환과 같은 처리)
    if rgba.chunks_exact(4).all(|px| px[3] == 0) {
        for px in rgba.chunks_exact_mut(4) {
            px[3] = 255;
        }
    }
    Some(IconImage {
        width,
        height,
        rgba,
    })
}

fn read_u16(bytes: &[u8], at: usize) -> Option<u16> {
    let raw = bytes.get(at..at + 2)?;
    Some(u16::from_le_bytes([raw[0], raw[1]]))
}

fn read_u32(bytes: &[u8], at: usize) -> Option<u32> {
    let raw = bytes.get(at..at + 4)?;
    Some(u32::from_le_bytes([raw[0], raw[1], raw[2], raw[3]]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 앱_아이콘을_요청한_크기로_읽는다() {
        // 32·64는 파일에 실제로 들어 있는 크기다 — 그대로 나와야 한다
        for size in [32, 64] {
            let image = decode(ICO_BYTES, size).expect("아이콘을 읽지 못했다");
            assert_eq!((image.width, image.height), (size, size));
            assert_eq!(image.rgba.len(), (size * size * 4) as usize);
        }
    }

    #[test]
    fn 없는_크기는_한_단계_큰_것으로_대신한다() {
        // 20px 항목은 없다 — 24px로 줄여 그리는 편이 16px을 늘리는 것보다 깨끗하다
        let image = decode(ICO_BYTES, 20).expect("아이콘을 읽지 못했다");
        assert_eq!(image.width, 24);
    }

    #[test]
    fn 가장_큰_항목을_넘겨_요청하면_있는_것_중_가장_큰_것을_준다() {
        // 256px 항목은 PNG라 건너뛴다 — 그다음인 128px이 나와야 한다
        let image = decode(ICO_BYTES, 512).expect("아이콘을 읽지 못했다");
        assert_eq!(image.width, 128);
    }

    #[test]
    fn 아이콘에_보이는_픽셀이_있다() {
        // 전부 투명하면 화면에 아무것도 안 나온다 — 알파 처리가 뒤집힌 회귀를 막는다
        let image = decode(ICO_BYTES, 32).expect("아이콘을 읽지 못했다");
        assert!(image.rgba.chunks_exact(4).any(|px| px[3] > 0));
    }

    #[test]
    fn 아이콘_형식이_아니면_읽지_않는다() {
        assert!(decode(b"", 32).is_none());
        assert!(decode(&[0u8; 64], 32).is_none());
    }
}
