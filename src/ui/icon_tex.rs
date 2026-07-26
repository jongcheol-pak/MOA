//! 시스템 아이콘 → egui 텍스처 변환 (FR-5)
//!
//! `IconCache`는 **시스템 이미지 리스트의 인덱스**만 들고 있다(ListView 전용 설계).
//! egui는 이미지 리스트를 그릴 수 없으므로 인덱스를 HICON으로 꺼내 RGBA 픽셀로 바꾼 뒤
//! 텍스처로 올린다. 변환·해제에 필요한 unsafe는 전부 이 파일에 격리한다.
use eframe::egui;
use std::collections::HashMap;
use windows::Win32::Graphics::Gdi::{
    BI_RGB, BITMAP, BITMAPINFO, BITMAPINFOHEADER, CreateCompatibleDC, DIB_RGB_COLORS, DeleteDC,
    DeleteObject, GetDIBits, GetObjectW, HBITMAP, HDC,
};
use windows::Win32::UI::Controls::{HIMAGELIST, ILD_TRANSPARENT, ImageList_GetIcon};
use windows::Win32::UI::WindowsAndMessaging::{DestroyIcon, GetIconInfo, HICON, ICONINFO};

/// 이미지 리스트 인덱스 → egui 텍스처 캐시.
/// 같은 확장자 행이 수천 개여도 텍스처는 아이콘 종류 수만큼만 만들어진다.
pub struct IconTextures {
    by_index: HashMap<i32, Option<egui::TextureHandle>>,
    /// 이번 프레임에 만든 수 — 한 프레임에 몰리면 렌더가 수 초 멈춘다(PoC 실측 3096ms)
    created_this_frame: usize,
}

/// 한 프레임에 새로 만들 텍스처 수 상한.
/// 실측에서 텍스처 다수가 한 프레임에 생성되며 3초급 스파이크가 났다 —
/// 넘치는 것은 다음 프레임으로 미루고 그 프레임에는 아이콘 없이 그린다(몇 프레임 안에 채워진다)
const MAX_NEW_TEXTURES_PER_FRAME: usize = 8;

impl Default for IconTextures {
    fn default() -> IconTextures {
        IconTextures::new()
    }
}

impl IconTextures {
    pub fn new() -> IconTextures {
        IconTextures {
            by_index: HashMap::new(),
            created_this_frame: 0,
        }
    }

    /// 프레임 시작 시 호출 — 프레임당 생성 상한을 초기화한다
    pub fn begin_frame(&mut self) {
        self.created_this_frame = 0;
    }

    /// 인덱스에 해당하는 텍스처. 변환 실패한 인덱스는 `None`으로 기억해 재시도하지 않는다
    pub fn get(
        &mut self,
        ctx: &egui::Context,
        himl: HIMAGELIST,
        index: i32,
    ) -> Option<&egui::TextureHandle> {
        if !self.by_index.contains_key(&index) {
            // 상한을 넘으면 이번 프레임에는 만들지 않는다 — 캐시에 넣지도 않으므로 다음 프레임에 재시도된다
            if self.created_this_frame >= MAX_NEW_TEXTURES_PER_FRAME {
                return None;
            }
            let image = icon_to_image(himl, index);
            let handle = image.map(|img| {
                self.created_this_frame += 1;
                ctx.load_texture(format!("icon{index}"), img, egui::TextureOptions::LINEAR)
            });
            self.by_index.insert(index, handle);
        }
        self.by_index.get(&index).and_then(|h| h.as_ref())
    }
}

/// 이미지 리스트의 아이콘 하나를 RGBA 이미지로 변환한다.
/// 실패(잘못된 인덱스·비트맵 조회 실패 등)하면 None — 호출부는 아이콘 없이 그린다.
fn icon_to_image(himl: HIMAGELIST, index: i32) -> Option<egui::ColorImage> {
    if index < 0 {
        return None;
    }
    // 안전성: 이미지 리스트에서 아이콘 사본을 얻고, 이 함수를 벗어나기 전에 반드시 해제한다.
    // 중간 실패 경로에서도 DestroyIcon이 호출되도록 결과를 받아 마지막에 정리한다
    unsafe {
        let hicon = ImageList_GetIcon(himl, index, ILD_TRANSPARENT);
        if hicon.is_invalid() {
            return None;
        }
        let image = hicon_to_image(hicon);
        let _ = DestroyIcon(hicon);
        image
    }
}

/// HICON → RGBA. 32bpp 컬러 비트맵을 top-down으로 읽어 BGRA를 RGBA로 바꾼다.
///
/// 안전성 주의: 유효한 HICON에만 호출한다. 내부에서 얻은 GDI 개체는 모두 이 함수에서 해제한다
unsafe fn hicon_to_image(hicon: HICON) -> Option<egui::ColorImage> {
    unsafe {
        let mut info = ICONINFO::default();
        GetIconInfo(hicon, &mut info).ok()?;
        // 마스크·컬러 비트맵은 GetIconInfo가 사본을 주므로 호출부가 해제해야 한다
        let result = color_bitmap_to_image(info.hbmColor);
        if !info.hbmColor.is_invalid() {
            let _ = DeleteObject(info.hbmColor.into());
        }
        if !info.hbmMask.is_invalid() {
            let _ = DeleteObject(info.hbmMask.into());
        }
        result
    }
}

/// 32bpp 컬러 비트맵을 RGBA 이미지로 읽는다.
///
/// 안전성 주의: 유효한 HBITMAP에만 호출한다
unsafe fn color_bitmap_to_image(hbm: HBITMAP) -> Option<egui::ColorImage> {
    unsafe {
        if hbm.is_invalid() {
            // 1bpp 마스크만 있는 흑백 아이콘 — 드물어 그리지 않는다(아이콘 없이 표시)
            return None;
        }
        let mut bitmap = BITMAP::default();
        let written = GetObjectW(
            hbm.into(),
            size_of::<BITMAP>() as i32,
            Some(&mut bitmap as *mut BITMAP as *mut core::ffi::c_void),
        );
        if written == 0 || bitmap.bmWidth <= 0 || bitmap.bmHeight <= 0 {
            return None;
        }
        let (width, height) = (bitmap.bmWidth as usize, bitmap.bmHeight as usize);

        // biHeight 음수 = top-down (첫 행이 이미지 위쪽) — 뒤집기 없이 그대로 쓸 수 있다
        let mut header = BITMAPINFO::default();
        header.bmiHeader.biSize = size_of::<BITMAPINFOHEADER>() as u32;
        header.bmiHeader.biWidth = bitmap.bmWidth;
        header.bmiHeader.biHeight = -bitmap.bmHeight;
        header.bmiHeader.biPlanes = 1;
        header.bmiHeader.biBitCount = 32;
        header.bmiHeader.biCompression = BI_RGB.0;

        let mut pixels = vec![0u8; width * height * 4];
        let hdc: HDC = CreateCompatibleDC(None);
        if hdc.is_invalid() {
            return None;
        }
        let lines = GetDIBits(
            hdc,
            hbm,
            0,
            height as u32,
            Some(pixels.as_mut_ptr() as *mut core::ffi::c_void),
            &mut header,
            DIB_RGB_COLORS,
        );
        let _ = DeleteDC(hdc);
        if lines == 0 {
            return None;
        }

        // GDI는 BGRA 순서로 주고 알파는 프리멀티플라이일 수 있다.
        // egui의 from_rgba_unmultiplied는 스트레이트 알파를 받으므로 되돌린다
        for px in pixels.chunks_exact_mut(4) {
            let (b, g, r, a) = (px[0], px[1], px[2], px[3]);
            if a == 0 {
                px.copy_from_slice(&[0, 0, 0, 0]);
                continue;
            }
            let unmul = |c: u8| ((c as u32 * 255 + a as u32 / 2) / a as u32).min(255) as u8;
            px[0] = unmul(r);
            px[1] = unmul(g);
            px[2] = unmul(b);
            px[3] = a;
        }
        // 알파가 전부 0이면 32bpp인데 알파 채널을 쓰지 않는 아이콘이다 — 불투명으로 되살린다
        if pixels.chunks_exact(4).all(|px| px[3] == 0) {
            let mut opaque = vec![0u8; width * height * 4];
            let raw = read_raw_bgra(hbm, width, height)?;
            for (dst, src) in opaque.chunks_exact_mut(4).zip(raw.chunks_exact(4)) {
                dst.copy_from_slice(&[src[2], src[1], src[0], 255]);
            }
            return Some(egui::ColorImage::from_rgba_unmultiplied(
                [width, height],
                &opaque,
            ));
        }
        Some(egui::ColorImage::from_rgba_unmultiplied(
            [width, height],
            &pixels,
        ))
    }
}

/// 알파 없는 32bpp 아이콘을 위해 원본 BGRA를 한 번 더 읽는다.
///
/// 안전성 주의: `color_bitmap_to_image`가 유효성을 확인한 뒤에만 호출한다
unsafe fn read_raw_bgra(hbm: HBITMAP, width: usize, height: usize) -> Option<Vec<u8>> {
    unsafe {
        let mut header = BITMAPINFO::default();
        header.bmiHeader.biSize = size_of::<BITMAPINFOHEADER>() as u32;
        header.bmiHeader.biWidth = width as i32;
        header.bmiHeader.biHeight = -(height as i32);
        header.bmiHeader.biPlanes = 1;
        header.bmiHeader.biBitCount = 32;
        header.bmiHeader.biCompression = BI_RGB.0;

        let mut raw = vec![0u8; width * height * 4];
        let hdc = CreateCompatibleDC(None);
        if hdc.is_invalid() {
            return None;
        }
        let lines = GetDIBits(
            hdc,
            hbm,
            0,
            height as u32,
            Some(raw.as_mut_ptr() as *mut core::ffi::c_void),
            &mut header,
            DIB_RGB_COLORS,
        );
        let _ = DeleteDC(hdc);
        if lines == 0 { None } else { Some(raw) }
    }
}
