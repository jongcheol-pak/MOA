//! 파일 썸네일 미리보기 — 워커 스레드 + LRU 캐시 (FR-24·NFR-9).
//!
//! 셸 형식 아이콘(`fs::icons`)과 달리 **파일마다 디스크를 읽으므로** UI 스레드에서 부를 수 없다
//! (AGENTS: UI 스레드 블로킹 I/O 금지). 요청을 워커에 보내고 결과를 채널로 받는다.
//!
//! 캐시는 **RGBA 이미지 단계에서** 상한을 건다 — 텍스처가 아니라 여기서 걸어야 메모리 상한이
//! 정확히 지켜진다(256×256 RGBA 한 장이 256KB, 200장이면 약 50MB — NFR-9).
//! 이 모듈은 UI를 모른다: 픽셀과 크기만 돌려주고 텍스처로 올리는 일은 `ui` 계층이 한다.
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, Sender, TryRecvError, channel};
use windows::Win32::Foundation::SIZE;
use windows::Win32::Graphics::Gdi::{
    BI_RGB, BITMAP, BITMAPINFO, BITMAPINFOHEADER, CreateCompatibleDC, DIB_RGB_COLORS, DeleteDC,
    DeleteObject, GetDIBits, GetObjectW, HBITMAP, HDC,
};
use windows::Win32::System::Com::{COINIT_APARTMENTTHREADED, CoInitializeEx, CoUninitialize};
use windows::Win32::UI::Shell::{
    IShellItemImageFactory, SHCreateItemFromParsingName, SIIGBF_BIGGERSIZEOK, SIIGBF_RESIZETOFIT,
};
use windows::core::HSTRING;

/// 패널 하나가 들고 있을 썸네일 수 상한 (NFR-9 — 약 50MB).
/// 넘으면 가장 오래 안 쓴 것부터 버린다
pub const MAX_CACHED: usize = 200;

/// 만들 썸네일의 한 변 — 아주 큰 아이콘(256px)에 맞춘다.
/// 더 작은 보기 모드는 이 한 장을 줄여 쓴다(작게 만들어 두면 큰 모드에서 뭉개진다)
pub const THUMB_PX: i32 = 256;

/// 워커가 돌려주는 썸네일 픽셀. `ui` 계층이 이것을 텍스처로 올린다
#[derive(Clone, PartialEq, Debug)]
pub struct ThumbnailImage {
    pub width: usize,
    pub height: usize,
    /// RGBA 스트레이트 알파 (egui `ColorImage::from_rgba_unmultiplied`가 받는 형식)
    pub rgba: Vec<u8>,
}

/// 워커에게 보내는 요청. 세대 번호를 실어 보내 **늦게 도착한 이전 폴더의 결과**를 가려낸다
enum Request {
    Make { generation: u64, path: PathBuf },
    Stop,
}

/// 썸네일 캐시 — 요청 큐·결과 수신·LRU 축출을 함께 관리한다.
///
/// 패널마다 하나씩 둔다(NFR-9의 상한이 패널당이다). 폴더를 떠나면 `clear`로 비운다
pub struct ThumbnailCache {
    tx: Sender<Request>,
    rx: Receiver<(u64, PathBuf, Option<ThumbnailImage>)>,
    /// 완성된 썸네일. `None`은 **만들 수 없는 파일**(썸네일 없는 형식)이며,
    /// 다시 요청하지 않기 위해 실패도 기억한다
    ready: HashMap<PathBuf, Option<ThumbnailImage>>,
    /// 최근 사용 순서 — 앞이 가장 오래됐다. 항목 수가 상한을 넘으면 앞에서 버린다
    order: Vec<PathBuf>,
    /// 요청을 보냈고 아직 결과가 안 온 것 — 같은 파일을 거듭 요청하지 않는다
    pending: Vec<PathBuf>,
    /// 폴더를 떠날 때마다 오르는 번호. 결과에 실려 돌아오며, 지금 세대와 다르면 버린다 —
    /// 폴더를 빠르게 오가면 이전 폴더의 요청이 나중에 도착해 캐시 자리를 차지한다
    /// (`ui::panel`의 `DirLoad`가 쓰는 것과 같은 방식)
    generation: u64,
}

impl ThumbnailCache {
    pub fn new() -> ThumbnailCache {
        let (request_tx, request_rx) = channel::<Request>();
        let (result_tx, result_rx) = channel();
        std::thread::spawn(move || worker(request_rx, result_tx));
        ThumbnailCache {
            tx: request_tx,
            rx: result_rx,
            ready: HashMap::new(),
            order: Vec::new(),
            pending: Vec::new(),
            generation: 0,
        }
    }

    /// 썸네일을 요청한다. 이미 있으면 **최근 사용으로 올리고** 끝낸다.
    ///
    /// 화면에 보이는 항목마다 매 프레임 불리므로, 여기서 올려야 보이는 것이 축출되지 않는다 —
    /// 그리기는 텍스처만 보고 픽셀 캐시를 건드리지 않아 이 경로가 유일한 갱신 지점이다
    pub fn request(&mut self, path: &Path) {
        if self.ready.contains_key(path) {
            self.touch(path);
            return;
        }
        if self.pending.iter().any(|p| p == path) {
            return;
        }
        self.pending.push(path.to_path_buf());
        // 워커가 죽었으면(앱 종료 중) 전송 실패는 무해하다
        let _ = self.tx.send(Request::Make {
            generation: self.generation,
            path: path.to_path_buf(),
        });
    }

    /// 도착한 결과를 받아들인다. **새로 준비된 경로들**을 돌려준다 —
    /// 호출부(`ui`)가 그것만 텍스처로 올리면 된다
    pub fn poll(&mut self) -> Vec<PathBuf> {
        let mut arrived = Vec::new();
        loop {
            match self.rx.try_recv() {
                Ok((generation, path, image)) => {
                    // 폴더를 떠난 뒤 도착한 결과는 `accept`가 걸러낸다
                    if self.accept(generation, path.clone(), image) {
                        arrived.push(path);
                    }
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => break,
            }
        }
        arrived
    }

    /// 준비된 썸네일. 없으면 `None`(아직이거나 만들 수 없는 파일).
    /// 꺼내 쓰면 **최근 사용으로 올린다** — 화면에 보이는 것이 먼저 버려지지 않게 한다
    pub fn get(&mut self, path: &Path) -> Option<&ThumbnailImage> {
        if self.ready.contains_key(path) {
            self.touch(path);
        }
        self.ready.get(path)?.as_ref()
    }

    /// 폴더를 떠날 때 호출 — 그 폴더의 썸네일을 즉시 놓는다 (NFR-9).
    ///
    /// **세대를 올려** 진행 중이던 요청의 결과가 나중에 도착해도 버려지게 한다.
    /// 워커는 이미 만들던 것을 끝까지 만들지만 그 결과는 `poll`이 걸러낸다
    pub fn clear(&mut self) {
        self.ready.clear();
        self.order.clear();
        self.pending.clear();
        self.generation = self.generation.wrapping_add(1);
    }

    /// 캐시에 든 항목 수 (실패로 기억한 것 포함) — 상한 검증용
    pub fn len(&self) -> usize {
        self.ready.len()
    }

    /// 테스트에서 워커를 거치지 않고 결과를 넣는다 —
    /// 텍스처 캐시 동기화처럼 셸 호출과 무관한 로직을 검증하는 데 쓴다
    #[cfg(test)]
    pub fn accept_for_test(&mut self, path: PathBuf, image: Option<ThumbnailImage>) {
        self.insert(path, image);
    }

    /// 만들어진 썸네일이 있는 경로들 — 텍스처 캐시가 동기화에 쓴다.
    /// 실패로 기억한 것(`None`)은 올릴 그림이 없으므로 뺀다
    pub fn ready_paths(&self) -> Vec<PathBuf> {
        self.ready
            .iter()
            .filter(|(_, image)| image.is_some())
            .map(|(path, _)| path.clone())
            .collect()
    }

    /// 이 경로의 썸네일이 캐시에 있는가 (실패 기억은 제외).
    /// 텍스처 캐시가 "픽셀이 사라진 텍스처"를 찾아내는 데 쓴다
    pub fn has_image(&self, path: &Path) -> bool {
        self.ready.get(path).is_some_and(|image| image.is_some())
    }

    /// 최근 사용 순서를 바꾸지 않고 들여다본다 — 동기화 중에는 순서를 흔들면 안 된다
    pub fn peek(&self, path: &Path) -> Option<&ThumbnailImage> {
        self.ready.get(path)?.as_ref()
    }

    /// 캐시가 쥐고 있는 픽셀 바이트 합 — NFR-9 상한이 실제로 지켜지는지 재는 데 쓴다.
    /// 이론값(200 × 256KB)이 아니라 **실제 담긴 이미지**의 크기다 —
    /// 썸네일은 비율을 지켜 만들어져 원본이 정사각형이 아니면 256×256보다 작다
    pub fn memory_bytes(&self) -> usize {
        self.ready
            .values()
            .filter_map(|image| image.as_ref())
            .map(|image| image.rgba.len())
            .sum()
    }

    pub fn is_empty(&self) -> bool {
        self.ready.is_empty()
    }

    /// 도착한 결과 하나를 세대 검사 후 받아들인다. 담았으면 `true`.
    /// `poll`과 테스트가 같은 판정을 쓰도록 한 곳에 둔다
    fn accept(&mut self, generation: u64, path: PathBuf, image: Option<ThumbnailImage>) -> bool {
        if generation != self.generation {
            return false;
        }
        self.pending.retain(|p| p != &path);
        let is_image = image.is_some();
        self.insert(path, image);
        is_image
    }

    fn insert(&mut self, path: PathBuf, image: Option<ThumbnailImage>) {
        if self.ready.insert(path.clone(), image).is_none() {
            self.order.push(path);
        }
        self.evict();
    }

    /// 최근 사용으로 올린다
    fn touch(&mut self, path: &Path) {
        if let Some(index) = self.order.iter().position(|p| p == path) {
            let entry = self.order.remove(index);
            self.order.push(entry);
        }
    }

    /// 상한을 넘으면 가장 오래 안 쓴 것부터 버린다
    fn evict(&mut self) {
        while self.order.len() > MAX_CACHED {
            let oldest = self.order.remove(0);
            self.ready.remove(&oldest);
        }
    }
}

impl Default for ThumbnailCache {
    fn default() -> ThumbnailCache {
        ThumbnailCache::new()
    }
}

impl Drop for ThumbnailCache {
    fn drop(&mut self) {
        // 워커를 세운다 — 보내지 못해도(이미 죽음) 무해하다
        let _ = self.tx.send(Request::Stop);
    }
}

/// 워커 스레드 본체 — 요청을 받아 썸네일을 만들고 결과를 돌려준다.
///
/// **스레드마다 COM을 따로 초기화한다** — 셸 인터페이스는 아파트 단위라
/// 메인 스레드의 초기화가 여기까지 미치지 않는다
fn worker(rx: Receiver<Request>, tx: Sender<(u64, PathBuf, Option<ThumbnailImage>)>) {
    // 안전성: 이 스레드에서 초기화하고, **성공했을 때만** 같은 스레드에서 해제한다 —
    // 실패한 초기화를 짝지어 해제하면 COM 참조 수가 어긋난다.
    // 실패해도 셸 호출이 동작하는 경우가 있어 작업 자체는 계속 시도한다
    let initialized = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) }.is_ok();
    while let Ok(request) = rx.recv() {
        match request {
            Request::Make { generation, path } => {
                let image = make_thumbnail(&path);
                if tx.send((generation, path, image)).is_err() {
                    break; // 수신부가 사라졌다 — 패널이 닫혔거나 앱이 끝났다
                }
            }
            Request::Stop => break,
        }
    }
    if initialized {
        // 안전성: 위에서 성공한 초기화와 같은 스레드에서 1회 호출
        unsafe {
            CoUninitialize();
        }
    }
}

/// 파일 하나의 썸네일을 만든다. 만들 수 없으면 `None`(형식 아이콘으로 폴백된다).
///
/// 안전성: COM이 초기화된 스레드에서만 호출한다. 얻은 HBITMAP은 이 함수 안에서 해제한다
fn make_thumbnail(path: &Path) -> Option<ThumbnailImage> {
    unsafe {
        let factory: IShellItemImageFactory =
            SHCreateItemFromParsingName(&HSTRING::from(path.as_os_str()), None).ok()?;
        let size = SIZE {
            cx: THUMB_PX,
            cy: THUMB_PX,
        };
        // RESIZETOFIT은 비율을 지키며 맞추고, BIGGERSIZEOK은 원본이 더 크면 큰 것을 받아
        // 축소 품질을 지킨다. ICONONLY를 주지 않으므로 썸네일이 있으면 그것이 온다
        let bitmap = factory
            .GetImage(size, SIIGBF_RESIZETOFIT | SIIGBF_BIGGERSIZEOK)
            .ok()?;
        let image = bitmap_to_rgba(bitmap);
        let _ = DeleteObject(bitmap.into());
        image
    }
}

/// 32bpp 비트맵을 RGBA로 읽는다.
///
/// `ui::icon_tex`의 아이콘 변환과 같은 GDI 절차지만 그쪽은 `ui` 계층이라 여기서 쓸 수 없다
/// (`fs`는 `ui`를 모른다). 세 번째 사용처가 생기면 공통 위치를 찾는다.
///
/// 안전성: 유효한 HBITMAP에만 호출한다. 내부에서 만든 DC는 이 함수에서 해제한다
unsafe fn bitmap_to_rgba(bitmap: HBITMAP) -> Option<ThumbnailImage> {
    unsafe {
        let mut info = BITMAP::default();
        let written = GetObjectW(
            bitmap.into(),
            size_of::<BITMAP>() as i32,
            Some(&mut info as *mut BITMAP as *mut core::ffi::c_void),
        );
        if written == 0 || info.bmWidth <= 0 || info.bmHeight <= 0 {
            return None;
        }
        let (width, height) = (info.bmWidth as usize, info.bmHeight as usize);

        // biHeight 음수 = top-down (첫 행이 이미지 위쪽) — 뒤집기 없이 그대로 쓴다
        let mut header = BITMAPINFO::default();
        header.bmiHeader.biSize = size_of::<BITMAPINFOHEADER>() as u32;
        header.bmiHeader.biWidth = info.bmWidth;
        header.bmiHeader.biHeight = -info.bmHeight;
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
            bitmap,
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

        // 알파 채널을 **쓰지 않는** 비트맵인지 먼저 판정한다 — 전부 0이면 그렇다.
        // 픽셀마다 판정하면 진짜 투명한 부분(로고 주변 등)까지 불투명으로 메워
        // 검은 테두리가 생긴다. `ui::icon_tex`의 아이콘 변환과 같은 규칙이다
        let opaque_bitmap = pixels.chunks_exact(4).all(|px| px[3] == 0);
        for px in pixels.chunks_exact_mut(4) {
            let (b, g, r, a) = (px[0], px[1], px[2], px[3]);
            if opaque_bitmap {
                // 알파를 안 쓰는 비트맵 — 색만 옮기고 불투명으로 둔다
                px.copy_from_slice(&[r, g, b, 255]);
                continue;
            }
            if a == 0 {
                // 실제로 투명한 픽셀이다 — 색까지 지워야 가장자리에 잔상이 남지 않는다
                px.copy_from_slice(&[0, 0, 0, 0]);
                continue;
            }
            // GDI는 프리멀티플라이 알파를 줄 수 있다 — 스트레이트 알파로 되돌린다
            let unmul = |c: u8| ((c as u32 * 255 + a as u32 / 2) / a as u32).min(255) as u8;
            px[0] = unmul(r);
            px[1] = unmul(g);
            px[2] = unmul(b);
            px[3] = a;
        }
        Some(ThumbnailImage {
            width,
            height,
            rgba: pixels,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn image(size: usize) -> ThumbnailImage {
        ThumbnailImage {
            width: size,
            height: size,
            rgba: vec![0; size * size * 4],
        }
    }

    /// 워커를 띄우지 않고 캐시 자료구조만 검사한다 — 축출 규칙은 셸과 무관하다
    fn cache() -> ThumbnailCache {
        ThumbnailCache::new()
    }

    #[test]
    fn 상한을_넘으면_가장_오래된_것부터_버린다() {
        // 상한이 없으면 큰 폴더를 훑는 동안 메모리가 끝없이 는다 (NFR-9)
        let mut cache = cache();
        for index in 0..MAX_CACHED + 10 {
            cache.insert(PathBuf::from(format!("f{index}.jpg")), Some(image(1)));
        }
        assert_eq!(cache.len(), MAX_CACHED, "상한을 넘겨 들고 있다");
        // 처음 10개는 밀려났다
        assert!(cache.get(Path::new("f0.jpg")).is_none());
        assert!(cache.get(Path::new("f9.jpg")).is_none());
        assert!(cache.get(Path::new("f10.jpg")).is_some());
    }

    #[test]
    fn 최근에_쓴_것은_살아남는다() {
        // 화면에 보이는 썸네일이 먼저 버려지면 스크롤할 때마다 다시 만든다
        let mut cache = cache();
        for index in 0..MAX_CACHED {
            cache.insert(PathBuf::from(format!("f{index}.jpg")), Some(image(1)));
        }
        // 가장 오래된 것을 한 번 쓰면 최근으로 올라간다
        assert!(cache.get(Path::new("f0.jpg")).is_some());
        cache.insert(PathBuf::from("new.jpg"), Some(image(1)));
        assert!(
            cache.get(Path::new("f0.jpg")).is_some(),
            "방금 쓴 것이 버려졌다"
        );
        assert!(
            cache.get(Path::new("f1.jpg")).is_none(),
            "그다음이 밀려야 한다"
        );
    }

    #[test]
    fn 만들_수_없는_파일은_실패를_기억한다() {
        // 기억하지 않으면 스크롤할 때마다 같은 파일을 다시 요청한다
        let mut cache = cache();
        let path = PathBuf::from("문서.txt");
        cache.insert(path.clone(), None);
        assert!(
            cache.get(&path).is_none(),
            "만들 수 없는데 무언가를 돌려줬다"
        );
        assert_eq!(cache.len(), 1, "실패가 기억되지 않았다");
        // 이미 아는 파일은 다시 요청하지 않는다
        cache.request(&path);
        assert!(cache.pending.is_empty());
    }

    #[test]
    fn 같은_파일을_거듭_요청하지_않는다() {
        let mut cache = cache();
        let path = PathBuf::from("사진.jpg");
        cache.request(&path);
        cache.request(&path);
        cache.request(&path);
        assert_eq!(cache.pending.len(), 1, "같은 요청이 쌓였다");
    }

    #[test]
    fn 보이는_항목을_다시_요청하면_최근으로_올라간다() {
        // 그리기는 텍스처만 보고 픽셀 캐시를 건드리지 않는다 — 화면에 보이는 항목마다
        // 매 프레임 불리는 `request`가 유일한 LRU 갱신 지점이다.
        // 이것이 없으면 지금 보고 있는 썸네일이 축출돼 스크롤할 때마다 다시 만든다
        let mut cache = cache();
        for index in 0..MAX_CACHED {
            cache.insert(PathBuf::from(format!("f{index}.jpg")), Some(image(1)));
        }
        let oldest = PathBuf::from("f0.jpg");
        cache.request(&oldest); // 화면에 보여서 다시 요청됐다
        cache.insert(PathBuf::from("new.jpg"), Some(image(1)));
        assert!(
            cache.has_image(&oldest),
            "보이는 항목인데 축출됐다 — request가 LRU를 갱신하지 않는다"
        );
        assert!(
            !cache.has_image(Path::new("f1.jpg")),
            "그다음이 밀려야 한다"
        );
    }

    #[test]
    fn 준비된_경로만_동기화_대상이다() {
        // 실패로 기억한 것(None)은 올릴 그림이 없다 — 텍스처 캐시가 헛돌면 안 된다
        let mut cache = cache();
        cache.insert(PathBuf::from("사진.jpg"), Some(image(1)));
        cache.insert(PathBuf::from("문서.txt"), None);
        let paths = cache.ready_paths();
        assert_eq!(paths.len(), 1);
        assert!(paths[0].ends_with("사진.jpg"));
        assert!(cache.has_image(Path::new("사진.jpg")));
        assert!(!cache.has_image(Path::new("문서.txt")));
        assert!(cache.peek(Path::new("문서.txt")).is_none());
    }

    #[test]
    fn 들여다보기는_순서를_바꾸지_않는다() {
        // 동기화 중에 순서가 흔들리면 축출 대상이 프레임마다 달라진다
        let mut cache = cache();
        for index in 0..3 {
            cache.insert(PathBuf::from(format!("f{index}.jpg")), Some(image(1)));
        }
        let before = cache.order.clone();
        let _ = cache.peek(Path::new("f0.jpg"));
        let _ = cache.ready_paths();
        assert_eq!(cache.order, before, "들여다보기가 순서를 바꿨다");
    }

    #[test]
    fn 폴더를_떠난_뒤_도착한_결과는_버린다() {
        // 폴더를 빠르게 오가면 이전 폴더의 요청이 나중에 도착한다 — 담아 두면
        // 지금 폴더의 캐시 자리를 뺏는다 (`DirLoad`와 같은 세대 방식)
        let mut cache = cache();
        let old = PathBuf::from("이전폴더/사진.jpg");
        cache.request(&old);
        let stale_generation = cache.generation;
        cache.clear(); // 폴더 이동 — 세대가 오른다
        assert_ne!(cache.generation, stale_generation, "세대가 오르지 않았다");

        // 워커가 이전 세대로 보낸 결과가 뒤늦게 도착한 상황을 그대로 재현한다
        cache.accept(stale_generation, old.clone(), Some(image(1)));
        assert!(cache.is_empty(), "떠난 폴더의 결과가 담겼다");

        // 지금 세대의 결과는 정상으로 담긴다
        let now = PathBuf::from("현재폴더/사진.jpg");
        let generation = cache.generation;
        cache.accept(generation, now.clone(), Some(image(1)));
        assert!(cache.get(&now).is_some());
    }

    #[test]
    fn 폴더를_떠나면_비운다() {
        // 떠난 폴더의 썸네일을 들고 있으면 여러 폴더를 오갈 때 상한이 의미를 잃는다 (NFR-9)
        let mut cache = cache();
        for index in 0..20 {
            cache.insert(PathBuf::from(format!("f{index}.jpg")), Some(image(1)));
        }
        cache.request(Path::new("진행중.jpg"));
        cache.clear();
        assert!(cache.is_empty());
        assert!(cache.pending.is_empty(), "진행 중 표시가 남았다");
    }

    #[test]
    fn 같은_경로를_다시_넣어도_두_번_세지_않는다() {
        // 감시 갱신 등으로 같은 파일이 다시 오면 순서 목록에 중복이 쌓일 수 있다
        let mut cache = cache();
        let path = PathBuf::from("사진.jpg");
        cache.insert(path.clone(), Some(image(1)));
        cache.insert(path.clone(), Some(image(2)));
        assert_eq!(cache.len(), 1);
        assert_eq!(cache.order.len(), 1, "순서 목록에 중복이 쌓였다");
    }

    /// 워커를 거쳐 결과가 도착할 때까지 기다린다(최대 `timeout_ms`).
    ///
    /// **성공·실패를 가리지 않고 "결과가 왔는지"로 판정한다** — `poll`은 텍스처로 올릴
    /// 성공분만 돌려주므로, 그것만 보면 만들 수 없는 파일에서 영영 기다린다.
    /// 첫 호출은 COM 초기화까지 겹쳐 수 초가 걸리므로 여유를 넉넉히 둔다
    fn wait_for(cache: &mut ThumbnailCache, path: &Path, timeout_ms: u64) -> bool {
        let start = std::time::Instant::now();
        cache.request(path);
        while start.elapsed().as_millis() < timeout_ms as u128 {
            cache.poll();
            if cache.ready.contains_key(path) {
                return true;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        false
    }

    #[test]
    fn 실제_파일의_썸네일을_만든다() {
        // 셸 호출 경로가 살아 있는지 본다 — 자료구조 테스트만으로는 GetImage가
        // 실제로 그림을 주는지 알 수 없다. 아이콘이라도 오면 경로는 성립한 것이다
        let dir = std::env::temp_dir().join(format!("fe_thumb_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("샘플.txt");
        std::fs::write(&file, b"hello").unwrap();

        let mut cache = ThumbnailCache::new();
        let arrived = wait_for(&mut cache, &file, 20_000);
        let _ = std::fs::remove_dir_all(&dir);

        assert!(
            arrived,
            "20초 안에 결과가 오지 않았다 — 워커나 셸 호출이 막혔다"
        );
        let image = cache.get(&file).expect("결과는 왔는데 그림이 없다");
        assert!(image.width > 0 && image.height > 0);
        assert_eq!(
            image.rgba.len(),
            image.width * image.height * 4,
            "픽셀 수와 크기가 어긋난다"
        );
        // 전부 투명하면 화면에 아무것도 안 보인다 — 알파 처리가 잘못된 경우다
        assert!(
            image.rgba.chunks_exact(4).any(|px| px[3] > 0),
            "모든 픽셀이 투명하다"
        );
    }

    #[test]
    fn 캐시가_가득_차도_상한_안에_머문다() {
        // NFR-9의 실질 — 상한이 "장수"로만 걸려 있으면 한 장이 커질 때 메모리가 함께 는다.
        // 여기서는 **실제 담긴 바이트**로 잰다(장당 최대 256×256×4 = 256KB)
        const PER_IMAGE: usize = (THUMB_PX * THUMB_PX * 4) as usize;
        let mut cache = cache();
        for index in 0..MAX_CACHED + 50 {
            cache.insert(
                PathBuf::from(format!("f{index}.jpg")),
                Some(ThumbnailImage {
                    width: THUMB_PX as usize,
                    height: THUMB_PX as usize,
                    rgba: vec![0; PER_IMAGE],
                }),
            );
        }
        let bytes = cache.memory_bytes();
        assert_eq!(cache.len(), MAX_CACHED, "장수 상한이 깨졌다");
        assert_eq!(
            bytes,
            MAX_CACHED * PER_IMAGE,
            "가득 찬 캐시의 실제 크기가 예상과 다르다"
        );
        // 약 50MB — NFR-9가 정한 패널당 상한
        assert!(
            bytes <= 55 * 1024 * 1024,
            "가득 찬 캐시가 {}MB로 상한을 넘는다",
            bytes / (1024 * 1024)
        );
    }

    #[test]
    fn 실제_썸네일의_장당_크기를_잰다() {
        // 실측 — 이 값이 plan의 메모리 기록 근거다. 썸네일은 비율을 지켜 만들어지므로
        // 정사각형이 아닌 원본은 256×256보다 작게 나온다
        let dir = std::env::temp_dir().join(format!("fe_thumb_mem_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("샘플.txt");
        std::fs::write(&file, b"hello").unwrap();

        let mut cache = ThumbnailCache::new();
        let arrived = wait_for(&mut cache, &file, 20_000);
        let bytes = cache.memory_bytes();
        let size = cache.get(&file).map(|i| (i.width, i.height));
        let _ = std::fs::remove_dir_all(&dir);

        assert!(arrived, "결과가 오지 않았다");
        println!("MEASURED 장당 {bytes} bytes, 크기 {size:?}");
        assert!(bytes > 0);
        assert!(
            bytes <= (THUMB_PX * THUMB_PX * 4) as usize,
            "한 장이 256×256 RGBA보다 크다 — 상한 산정이 어긋난다"
        );
    }

    #[test]
    fn 썸네일_크기는_가장_큰_보기_모드에_맞춘다() {
        // 작게 만들어 두면 아주 큰 아이콘(256px)에서 늘려야 해 뭉개진다
        assert_eq!(THUMB_PX, 256);
    }
}
