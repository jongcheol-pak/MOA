//! 설치된 글꼴 조회 (FR-48).
//!
//! 파일시스템 열거(`fs`)도 화면(`ui`)도 아닌 **앱 환경 조회**라 여기에 둔다
//! (`app::theme`이 `DwmSetWindowAttribute`를 쓰는 것과 같은 자리).
//!
//! ## 왜 글꼴 파일을 직접 읽는가 (2026-08-13 실측으로 뒤집힌 결정)
//!
//! 처음에는 GDI의 `GetFontData`로 바이트를 받았다. 그런데 **모음 글꼴(TTC)에서 그 데이터가
//! 깨져서 온다** — 굴림은 파일이 13,533,424바이트인데 GDI는 13,533,384바이트를 준다(40바이트 적다).
//! 헤더만 단일 글꼴 모양으로 바꿔 주면서 내부 테이블 오프셋은 원본 파일 기준으로 남겨 두기
//! 때문이다. 매직 넘버는 멀쩡해 보이지만(`0x00010000`) 글꼴 파서가 읽지 못한다.
//! 그 결과 **굴림·굴림체·돋움·돋움체·바탕·궁서가 전부 목록에서 빠졌다**(93개 중 58개만 남음).
//!
//! 그래서 파일을 직접 읽는다. 레지스트리(`HKLM\...\Fonts`)의 이름↔파일 매핑은 쓰지 않는다 —
//! 거기 적힌 이름이 **영문**(`Gulim & GulimChe & Dotum & DotumChe`)인데 화면에 보여야 하는
//! 이름은 한글(`굴림`)이라 짝지을 수 없다. 대신 글꼴 파일 안의 `name` 테이블에서
//! **그 글꼴이 스스로 밝히는 한글 이름**을 읽는다.
//!
//! 한글을 그릴 수 있는지는 GDI 열거(`HANGUL_CHARSET`)에 맡긴다 — OS가 이미 판정해 둔 것을
//! 다시 계산할 이유가 없고, 그 열거는 1ms면 끝난다.
use std::collections::BTreeMap;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use windows::Win32::Foundation::LPARAM;
use windows::Win32::Graphics::Gdi::{
    CreateCompatibleDC, DEFAULT_PITCH, DeleteDC, ENUMLOGFONTEXW, EnumFontFamiliesExW, FF_DONTCARE,
    HANGUL_CHARSET, LOGFONTW, TEXTMETRICW,
};

/// `LOGFONTW.lfFaceName`의 칸 수 — Win32가 정한 상한(널 종단 포함 32)
const FACE_NAME_LEN: usize = 32;

/// 글꼴 파일에서 face 하나를 가리키는 자리
#[derive(Debug, Clone, PartialEq, Eq)]
struct FaceLocation {
    path: PathBuf,
    /// 모음 글꼴 안에서 몇 번째인가 (단일 글꼴이면 0)
    index: u32,
}

/// 화면에 등록할 글꼴 한 벌
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedFont {
    pub bytes: Vec<u8>,
    /// `egui::FontData.index`에 그대로 넣는다 — 모음 글꼴은 이 값으로 face를 고른다
    pub index: u32,
}

/// 설치된 글꼴을 한 번 훑어 둔 목록 (FR-48).
///
/// **여러 글꼴을 차례로 다룰 때는 이것을 만들어 재사용한다** — 글꼴 하나마다 폴더를 다시
/// 훑으면 파일 160여 개를 그 횟수만큼 읽게 된다(실측: 90개 글꼴에 63초). 목록을 만들고
/// 각 글꼴을 검증하는 워커가 그 경로다
pub struct FontCatalog {
    faces: BTreeMap<String, FaceLocation>,
}

impl FontCatalog {
    /// 글꼴 폴더를 훑는다 — 파일마다 이름표만 읽으므로 전체를 메모리에 올리지 않는다
    pub fn scan() -> FontCatalog {
        FontCatalog {
            faces: face_index(),
        }
    }

    /// 한글을 그릴 수 있고 파일에서도 찾아낸 글꼴 이름을 가나다순으로.
    ///
    /// 한글 문자셋으로 거르는 이유: 전체를 보이면 한글 글리프가 없는 글꼴(`Wingdings` 등)을
    /// 고를 수 있고, 그 순간 파일명과 화면 문구가 통째로 두부(□)가 된다 (사용자 결정)
    pub fn korean_names(&self) -> Vec<String> {
        let mut names: Vec<String> = enumerate_hangul_faces()
            .into_iter()
            .filter(|name| self.faces.contains_key(name))
            .collect();
        names.sort();
        names.dedup();
        names
    }

    /// 그 글꼴의 원본 파일 바이트와 face 인덱스. 찾지 못하면 `None`.
    ///
    /// **파일을 통째로 읽는다** — 글꼴 파서가 파일 안의 절대 오프셋을 그대로 쓰므로
    /// 일부만 잘라 주면 읽지 못한다(GDI 경로가 실패한 이유가 바로 그것이다)
    pub fn load(&self, name: &str) -> Option<LoadedFont> {
        let location = self.faces.get(name)?;
        let bytes = std::fs::read(&location.path).ok()?;
        Some(LoadedFont {
            bytes,
            index: location.index,
        })
    }
}

/// 글꼴 하나만 읽으면 될 때 — 화면에 적용할 글꼴을 등록하는 자리가 쓴다.
///
/// 여러 개를 다룰 때는 `FontCatalog::scan()`을 한 번 만들어 쓴다(위 설명 참조)
pub fn load_font(name: &str) -> Option<LoadedFont> {
    FontCatalog::scan().load(name)
}

/// 글꼴 이름 → 파일·face 자리.
///
/// 캐시를 두지 않는다 — 부르는 곳이 워커 스레드뿐이고(대화를 열 때 한 번), 캐시를 두면
/// 글꼴을 새로 설치했을 때 앱을 다시 켜야 목록에 나타난다
fn face_index() -> BTreeMap<String, FaceLocation> {
    let mut index = BTreeMap::new();
    for dir in font_dirs() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if !is_font_file(&path) {
                continue;
            }
            for (name, face) in read_face_names(&path) {
                // 먼저 찾은 것을 남긴다 — 시스템 글꼴이 사용자 글꼴보다 앞에 오도록
                // `font_dirs`가 순서를 정한다
                index.entry(name).or_insert(FaceLocation {
                    path: path.clone(),
                    index: face,
                });
            }
        }
    }
    index
}

/// 글꼴이 놓이는 곳 — 시스템 폴더가 먼저다(같은 이름이면 시스템 것을 쓴다)
fn font_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(windir) = std::env::var_os("SystemRoot") {
        dirs.push(PathBuf::from(windir).join("Fonts"));
    }
    // 관리자 권한 없이 설치한 글꼴은 사용자 폴더에 들어간다
    if let Some(local) = std::env::var_os("LOCALAPPDATA") {
        dirs.push(
            PathBuf::from(local)
                .join("Microsoft")
                .join("Windows")
                .join("Fonts"),
        );
    }
    dirs
}

fn is_font_file(path: &Path) -> bool {
    let Some(ext) = path.extension().and_then(|ext| ext.to_str()) else {
        return false;
    };
    matches!(ext.to_ascii_lowercase().as_str(), "ttf" | "ttc" | "otf")
}

// ── sfnt/TTC 파싱 ──
//
// 글꼴 파일에서 **이름만** 읽는다. 전체를 메모리에 올리지 않고 필요한 조각만 seek해서
// 읽는 이유: 글꼴 폴더에 파일이 160개 넘게 있고 큰 것은 16MB라, 이름을 얻자고 전부 읽으면
// 목록 만들기가 몇 배로 느려진다

/// 모음 글꼴임을 나타내는 표식
const TAG_TTCF: [u8; 4] = *b"ttcf";
/// 이름표 테이블
const TAG_NAME: [u8; 4] = *b"name";
/// `name` 레코드가 뜻하는 이름 종류 — 1은 글꼴 가족 이름(화면에 보이는 그것)
const NAME_ID_FAMILY: u16 = 1;
/// Windows 플랫폼 / UCS-2 인코딩 — 이 조합이어야 UTF-16으로 읽을 수 있다
const PLATFORM_WINDOWS: u16 = 3;
const ENCODING_UCS2: u16 = 1;
/// 한국어(ko-KR) — 있으면 이것을 쓴다
const LANG_KOREAN: u16 = 0x0412;
/// 영어(en-US) — 한국어 이름이 없는 글꼴의 대체
const LANG_ENGLISH: u16 = 0x0409;
/// 모음 글꼴 안의 face 수 상한 — 넘으면 글꼴 파일이 아니거나 손상된 것이다
const MAX_FACES: u32 = 1024;
/// 한 face의 테이블 수 상한 (같은 목적)
const MAX_TABLES: usize = 512;
/// 이름표 테이블 크기 상한 (같은 목적)
const MAX_NAME_TABLE: u32 = 1 << 20;

/// 그 파일에 든 face들의 (이름, 인덱스). 읽지 못하면 빈 목록
fn read_face_names(path: &Path) -> Vec<(String, u32)> {
    let Ok(mut file) = std::fs::File::open(path) else {
        return Vec::new();
    };
    let mut header = [0u8; 12];
    if file.read_exact(&mut header).is_err() {
        return Vec::new();
    }

    let offsets: Vec<u32> = if header[..4] == TAG_TTCF {
        let count = be_u32(&header[8..12]);
        if count == 0 || count > MAX_FACES {
            return Vec::new();
        }
        let mut raw = vec![0u8; count as usize * 4];
        if file.read_exact(&mut raw).is_err() {
            return Vec::new();
        }
        raw.chunks_exact(4).map(be_u32).collect()
    } else {
        // 단일 글꼴 — 파일 처음이 곧 그 face의 시작이다
        vec![0]
    };

    offsets
        .into_iter()
        .enumerate()
        .filter_map(|(face, offset)| {
            read_family_name(&mut file, offset).map(|name| (name, face as u32))
        })
        .collect()
}

/// 그 face의 가족 이름 — 한국어 이름이 있으면 그것을, 없으면 영어 이름을
fn read_family_name(file: &mut std::fs::File, face_offset: u32) -> Option<String> {
    let (name_offset, name_len) = find_table(file, face_offset, TAG_NAME)?;
    if !(6..=MAX_NAME_TABLE).contains(&name_len) {
        return None;
    }
    let mut table = vec![0u8; name_len as usize];
    file.seek(SeekFrom::Start(name_offset as u64)).ok()?;
    file.read_exact(&mut table).ok()?;

    let count = be_u16(table.get(2..4)?);
    let storage = be_u16(table.get(4..6)?) as usize;
    let mut korean = None;
    let mut english = None;
    for record in 0..count as usize {
        let at = 6 + record * 12;
        let Some(record) = table.get(at..at + 12) else {
            break;
        };
        if be_u16(&record[0..2]) != PLATFORM_WINDOWS
            || be_u16(&record[2..4]) != ENCODING_UCS2
            || be_u16(&record[6..8]) != NAME_ID_FAMILY
        {
            continue;
        }
        let language = be_u16(&record[4..6]);
        if language != LANG_KOREAN && language != LANG_ENGLISH {
            continue;
        }
        let len = be_u16(&record[8..10]) as usize;
        let from = storage + be_u16(&record[10..12]) as usize;
        let Some(raw) = table.get(from..from + len) else {
            continue;
        };
        let text = utf16_be(raw);
        if text.is_empty() {
            continue;
        }
        if language == LANG_KOREAN {
            korean = Some(text);
        } else {
            english = Some(text);
        }
    }
    // 화면에 보이는 이름은 GDI 열거가 주는 것과 같아야 한다 —
    // 한국어 Windows에서 그것은 한국어 이름이다
    korean.or(english)
}

/// 그 face의 테이블 디렉터리에서 태그를 찾아 (파일 기준 오프셋, 길이)를 돌려준다
fn find_table(file: &mut std::fs::File, face_offset: u32, tag: [u8; 4]) -> Option<(u32, u32)> {
    let mut head = [0u8; 12];
    file.seek(SeekFrom::Start(face_offset as u64)).ok()?;
    file.read_exact(&mut head).ok()?;
    let tables = be_u16(&head[4..6]) as usize;
    if tables == 0 || tables > MAX_TABLES {
        return None;
    }
    let mut directory = vec![0u8; tables * 16];
    file.read_exact(&mut directory).ok()?;
    directory.chunks_exact(16).find_map(|record| {
        (record[..4] == tag).then(|| (be_u32(&record[8..12]), be_u32(&record[12..16])))
    })
}

fn be_u16(bytes: &[u8]) -> u16 {
    u16::from_be_bytes([bytes[0], bytes[1]])
}

fn be_u32(bytes: &[u8]) -> u32 {
    u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
}

/// 빅엔디언 UTF-16 → 문자열 (글꼴 파일의 이름은 이 형식으로 담긴다)
fn utf16_be(bytes: &[u8]) -> String {
    let units: Vec<u16> = bytes.chunks_exact(2).map(be_u16).collect();
    String::from_utf16_lossy(&units).trim().to_owned()
}

// ── 한글 문자셋 열거 (GDI) ──

/// 한글 문자셋을 가진 글꼴 이름을 모은다 — "이 글꼴로 한글을 그릴 수 있는가"의 판정은
/// OS가 이미 해 두었으므로 그대로 쓴다 (1ms)
fn enumerate_hangul_faces() -> Vec<String> {
    let mut names: Vec<String> = Vec::new();
    // 안전성: DC는 이 함수 안에서 만들고 지운다. 콜백은 열거가 끝나기 전에만 불리므로
    // `names`를 가리키는 포인터가 그 사이에 무효가 되지 않는다
    unsafe {
        let hdc = CreateCompatibleDC(None);
        if hdc.is_invalid() {
            return names;
        }
        let request = LOGFONTW {
            lfCharSet: HANGUL_CHARSET,
            lfPitchAndFamily: DEFAULT_PITCH.0 | FF_DONTCARE.0,
            ..Default::default()
        };
        EnumFontFamiliesExW(
            hdc,
            &request,
            Some(collect_face),
            LPARAM(&mut names as *mut Vec<String> as isize),
            0,
        );
        let _ = DeleteDC(hdc);
    }
    names
}

/// `EnumFontFamiliesExW` 콜백 — 글꼴 하나마다 불린다.
///
/// # 안전성
/// `lparam`은 `enumerate_hangul_faces`가 넘긴 `&mut Vec<String>`이어야 하고,
/// 그 열거가 도는 동안에만 불린다
unsafe extern "system" fn collect_face(
    logfont: *const LOGFONTW,
    _metric: *const TEXTMETRICW,
    _font_type: u32,
    lparam: LPARAM,
) -> i32 {
    unsafe {
        let (Some(logfont), Some(names)) = (
            logfont.cast::<ENUMLOGFONTEXW>().as_ref(),
            (lparam.0 as *mut Vec<String>).as_mut(),
        ) else {
            // 계속 열거한다 — 항목 하나를 못 읽었다고 나머지를 버릴 이유가 없다
            return 1;
        };
        let face = &logfont.elfLogFont.lfFaceName;
        let len = face
            .iter()
            .position(|&ch| ch == 0)
            .unwrap_or(FACE_NAME_LEN.min(face.len()));
        let name = String::from_utf16_lossy(&face[..len]);
        // `@`로 시작하는 것은 세로쓰기용 이름이다 — 같은 글꼴이 두 벌로 보이고
        // 가로쓰기 화면에서 고를 이유가 없다
        if !name.is_empty() && !name.starts_with('@') {
            names.push(name);
        }
        1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 한글_글꼴만_목록에_담긴다() {
        let fonts = FontCatalog::scan().korean_names();
        assert!(
            fonts.iter().any(|name| name == "맑은 고딕"),
            "맑은 고딕이 없다: {fonts:?}"
        );
        for excluded in ["Wingdings", "Webdings", "Marlett"] {
            assert!(
                !fonts.iter().any(|name| name == excluded),
                "한글이 없는 {excluded}이 목록에 있다"
            );
        }
        assert!(
            !fonts.iter().any(|name| name.starts_with('@')),
            "세로쓰기 이름이 남아 있다"
        );
    }

    #[test]
    fn 모음_글꼴도_목록에_남는다() {
        // GDI `GetFontData` 경로에서는 이들이 전부 빠졌다(깨진 데이터를 준다).
        // 파일을 직접 읽는 지금 방식의 존재 이유가 이 시험이다
        let fonts = FontCatalog::scan().korean_names();
        for name in ["굴림", "굴림체", "돋움", "바탕"] {
            assert!(
                fonts.iter().any(|font| font == name),
                "모음 글꼴 {name}이 목록에서 빠졌다"
            );
        }
    }

    #[test]
    fn 모음_글꼴은_face_인덱스가_서로_다르다() {
        // 같은 파일(`gulim.ttc`) 안의 네 글꼴이 같은 인덱스를 가리키면
        // 무엇을 골라도 첫 번째 글꼴만 나온다
        let catalog = FontCatalog::scan();
        let gulim = catalog.load("굴림").expect("굴림을 읽지 못했다");
        let gulimche = catalog.load("굴림체").expect("굴림체를 읽지 못했다");
        assert_eq!(
            gulim.bytes.len(),
            gulimche.bytes.len(),
            "같은 파일이어야 한다"
        );
        assert_ne!(gulim.index, gulimche.index, "face 인덱스가 같다");
    }

    #[test]
    fn 목록의_모든_이름은_파일에서_찾을_수_있다() {
        // 카탈로그를 한 번만 만들어 돈다 — 이름마다 폴더를 다시 훑으면 1분이 넘는다
        let catalog = FontCatalog::scan();
        let fonts = catalog.korean_names();
        assert!(!fonts.is_empty(), "한글 글꼴이 하나도 없다");
        for name in &fonts {
            assert!(catalog.load(name).is_some(), "{name}을 읽지 못했다");
        }
    }

    #[test]
    fn 없는_글꼴은_실패한다() {
        // GDI는 없는 이름을 비슷한 글꼴로 바꿔치기했다(옛 경로의 함정) —
        // 파일에서 찾는 지금 방식은 애초에 그럴 여지가 없다
        assert_eq!(load_font("없는글꼴이름XYZ"), None);
        assert_eq!(load_font(""), None);
    }

    #[test]
    fn 단일_글꼴은_인덱스가_0이다() {
        let malgun = FontCatalog::scan()
            .load("맑은 고딕")
            .expect("맑은 고딕을 읽지 못했다");
        assert_eq!(malgun.index, 0);
        // 파일을 통째로 읽는다 — 잘라 주면 글꼴 파서가 읽지 못한다
        assert!(
            malgun.bytes.len() > 1_000_000,
            "파일이 통째로 읽히지 않았다"
        );
    }
}
