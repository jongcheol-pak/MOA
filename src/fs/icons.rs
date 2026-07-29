//! 셸 아이콘·종류 문자열 조회 — 시스템 이미지 리스트 공유 + 확장자 캐시 (plan D8).
//!
//! 아이콘 인덱스는 **크기와 무관하게 같은 체계**를 쓴다 — 같은 인덱스를 16px 리스트에서 꺼내면
//! 작은 아이콘이, 256px 리스트에서 꺼내면 큰 아이콘이 나온다. 그래서 크기별 리스트만
//! 따로 들고 있으면 조회 로직은 하나로 충분하다 (FR-23·FR-24).
use std::collections::HashMap;
use windows::Win32::Storage::FileSystem::{FILE_ATTRIBUTE_DIRECTORY, FILE_ATTRIBUTE_NORMAL};
use windows::Win32::UI::Controls::{HIMAGELIST, IImageList};
use windows::Win32::UI::Shell::{
    SHFILEINFOW, SHGFI_SMALLICON, SHGFI_SYSICONINDEX, SHGFI_TYPENAME, SHGFI_USEFILEATTRIBUTES,
    SHGetFileInfoW, SHGetImageList, SHIL_EXTRALARGE, SHIL_JUMBO, SHIL_LARGE, SHIL_SMALL,
};
use windows::core::{HSTRING, Interface};

/// 시스템 이미지 리스트의 아이콘 크기 (FR-23의 보기 모드가 고르는 단계).
///
/// 96px 단계는 시스템에 없다 — `Jumbo`(256px)를 받아 그리는 쪽에서 줄인다 (plan D8)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IconSize {
    /// 16px — 자세히·목록·작은 아이콘
    Small,
    /// 32px — 내용 보기
    Large,
    /// 48px — 보통 아이콘·타일
    ExtraLarge,
    /// 256px — 아주 큰 아이콘·큰 아이콘(줄여서 씀)
    Jumbo,
}

impl IconSize {
    /// 이 단계에 맞는 시스템 이미지 리스트를 고른다.
    ///
    /// 그리려는 크기보다 **작지 않은** 가장 가까운 단계를 쓴다 — 늘리면 뭉개지고 줄이면
    /// 멀쩡하기 때문이다(plan D8)
    pub fn for_px(px: f32) -> IconSize {
        if px <= 16.0 {
            IconSize::Small
        } else if px <= 32.0 {
            IconSize::Large
        } else if px <= 48.0 {
            IconSize::ExtraLarge
        } else {
            IconSize::Jumbo
        }
    }

    /// `SHGetImageList`에 넘길 셸 상수
    fn shil(self) -> u32 {
        match self {
            IconSize::Small => SHIL_SMALL,
            IconSize::Large => SHIL_LARGE,
            IconSize::ExtraLarge => SHIL_EXTRALARGE,
            IconSize::Jumbo => SHIL_JUMBO,
        }
    }
}

/// 개별(파일별) 아이콘이 필요한 확장자 — 실행 파일·바로가기는 파일마다 아이콘이 다르다
const PER_FILE_ICON_EXTS: [&str; 3] = ["exe", "lnk", "ico"];

/// 확장자 → 시스템 이미지 리스트 인덱스/종류명 캐시.
/// 아이콘 자체를 복사하지 않고 시스템 공유 이미지 리스트 인덱스만 보관한다 (NFR-2)
pub struct IconCache {
    himl: HIMAGELIST,
    /// 크기별 시스템 이미지 리스트 — 못 얻은 단계는 담기지 않고 `himl_for`가 16px로 폴백한다
    himl_by_size: HashMap<IconSize, HIMAGELIST>,
    icon_by_ext: HashMap<String, i32>,
    type_by_ext: HashMap<String, String>,
    /// 개별 아이콘(경로별) 캐시 — 파일당 1회만 디스크 조회 (AGENTS UI 스레드 블로킹 최소화)
    icon_by_path: HashMap<String, i32>,
    dir_icon: i32,
    dir_type: String,
}

impl Default for IconCache {
    fn default() -> IconCache {
        IconCache::new()
    }
}

impl IconCache {
    pub fn new() -> IconCache {
        // 시스템 작은 아이콘 이미지 리스트 핸들 확보 (폴더 속성 기준 1회 조회)
        let mut info = SHFILEINFOW::default();
        // 안전성: info는 스택 소유, USEFILEATTRIBUTES라 실제 디스크 접근 없음
        let himl = unsafe {
            SHGetFileInfoW(
                &HSTRING::from("folder"),
                FILE_ATTRIBUTE_DIRECTORY,
                Some(&mut info),
                size_of::<SHFILEINFOW>() as u32,
                SHGFI_SYSICONINDEX | SHGFI_SMALLICON | SHGFI_TYPENAME | SHGFI_USEFILEATTRIBUTES,
            )
        };
        let himl = HIMAGELIST(himl as isize);
        let mut himl_by_size = HashMap::new();
        himl_by_size.insert(IconSize::Small, himl);
        // 큰 단계는 시작할 때 한 번에 얻는다 — 4회 COM 호출이라 시작 시간(NFR-1)에 영향이 없고,
        // 지연 획득으로 두면 스크롤 중에 COM 호출이 끼어든다
        for size in [IconSize::Large, IconSize::ExtraLarge, IconSize::Jumbo] {
            if let Some(list) = system_image_list(size) {
                himl_by_size.insert(size, list);
            }
        }
        IconCache {
            himl,
            himl_by_size,
            icon_by_ext: HashMap::new(),
            type_by_ext: HashMap::new(),
            icon_by_path: HashMap::new(),
            dir_icon: info.iIcon,
            dir_type: wide_to_string(&info.szTypeName),
        }
    }

    /// ListView LVSIL_SMALL에 연결할 시스템 이미지 리스트 (16px)
    pub fn himl(&self) -> HIMAGELIST {
        self.himl
    }

    /// 요청한 크기의 시스템 이미지 리스트.
    ///
    /// 그 단계를 얻지 못했으면 **16px로 폴백한다** — 아이콘이 작게 나올지언정
    /// 목록이 그려지지 않는 것보다 낫다
    pub fn himl_for(&self, size: IconSize) -> HIMAGELIST {
        self.himl_by_size.get(&size).copied().unwrap_or(self.himl)
    }

    /// 폴더 아이콘 인덱스 — 워크스페이스 사이드바가 항목 아이콘으로 직접 그린다 (plan D14)
    pub fn dir_icon(&self) -> i32 {
        self.dir_icon
    }

    /// 항목의 아이콘 인덱스. exe/lnk 등은 전체 경로로 개별 조회(표시 시점 지연 — 보이는 행만)
    pub fn icon_index(&mut self, ext: &str, is_dir: bool, full_path: Option<&str>) -> i32 {
        if is_dir {
            return self.dir_icon;
        }
        if PER_FILE_ICON_EXTS.contains(&ext)
            && let Some(path) = full_path
        {
            // 경로별 1회만 실제 조회 — 스크롤 재방문 시 캐시 히트 (blocking 최소화)
            if let Some(&idx) = self.icon_by_path.get(path) {
                return idx;
            }
            let mut info = SHFILEINFOW::default();
            // 안전성: 실제 파일 경로 조회 — 실패 시 iIcon 0(기본)이 그대로 쓰인다
            unsafe {
                SHGetFileInfoW(
                    &HSTRING::from(path),
                    Default::default(),
                    Some(&mut info),
                    size_of::<SHFILEINFOW>() as u32,
                    SHGFI_SYSICONINDEX | SHGFI_SMALLICON,
                );
            }
            self.icon_by_path.insert(path.to_string(), info.iIcon);
            return info.iIcon;
        }
        if let Some(&idx) = self.icon_by_ext.get(ext) {
            return idx;
        }
        let (idx, type_name) = lookup_by_attributes(ext);
        self.icon_by_ext.insert(ext.to_string(), idx);
        self.type_by_ext.insert(ext.to_string(), type_name);
        idx
    }

    /// 항목의 종류(형식) 문자열 — 셸이 주는 지역화 문자열 그대로
    pub fn type_name(&mut self, ext: &str, is_dir: bool) -> String {
        if is_dir {
            return self.dir_type.clone();
        }
        if let Some(t) = self.type_by_ext.get(ext) {
            return t.clone();
        }
        let (idx, type_name) = lookup_by_attributes(ext);
        self.icon_by_ext.insert(ext.to_string(), idx);
        self.type_by_ext.insert(ext.to_string(), type_name.clone());
        type_name
    }
}

/// 크기별 시스템 이미지 리스트를 얻는다. 실패하면 `None`(호출부가 16px로 폴백).
///
/// `SHGetImageList`는 `IImageList` COM 인터페이스를 주는데, 이미지 리스트 API가 쓰는
/// `HIMAGELIST`는 그 인터페이스 포인터와 같은 값이다(셸의 오래된 계약).
/// 인터페이스를 그대로 두면 drop 시 Release되어 핸들이 죽으므로 **참조를 의도적으로 넘긴다**
fn system_image_list(size: IconSize) -> Option<HIMAGELIST> {
    // 안전성: 프로세스 수명 동안 유지되는 시스템 공유 리스트를 받는다.
    // `into_raw`로 소유권을 넘겨 Release되지 않게 하며, 해제하지 않는 것이 이 API의 관례다
    // (시스템이 소유하고 앱은 빌려 쓴다 — `SHGetFileInfoW`가 주는 핸들과 같은 성질)
    unsafe {
        let list: IImageList = SHGetImageList(size.shil() as i32).ok()?;
        Some(HIMAGELIST(list.into_raw() as isize))
    }
}

/// 확장자만으로 아이콘·종류 조회 (디스크 접근 없음 — 대량 폴더에서도 빠름)
fn lookup_by_attributes(ext: &str) -> (i32, String) {
    let dummy = if ext.is_empty() {
        "file".to_string()
    } else {
        format!("file.{ext}")
    };
    let mut info = SHFILEINFOW::default();
    // 안전성: USEFILEATTRIBUTES — 가상 파일명으로 속성 기반 조회만 수행
    unsafe {
        SHGetFileInfoW(
            &HSTRING::from(dummy.as_str()),
            FILE_ATTRIBUTE_NORMAL,
            Some(&mut info),
            size_of::<SHFILEINFOW>() as u32,
            SHGFI_SYSICONINDEX | SHGFI_SMALLICON | SHGFI_TYPENAME | SHGFI_USEFILEATTRIBUTES,
        );
    }
    (info.iIcon, wide_to_string(&info.szTypeName))
}

fn wide_to_string(buf: &[u16]) -> String {
    let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    String::from_utf16_lossy(&buf[..len])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 요청한_크기보다_작지_않은_단계를_고른다() {
        // 늘린 아이콘은 뭉개지고 줄인 아이콘은 멀쩡하다 — 항상 같거나 큰 단계를 써야 한다
        assert_eq!(IconSize::for_px(16.0), IconSize::Small);
        assert_eq!(IconSize::for_px(32.0), IconSize::Large);
        assert_eq!(IconSize::for_px(48.0), IconSize::ExtraLarge);
        assert_eq!(
            IconSize::for_px(96.0),
            IconSize::Jumbo,
            "96px는 256px를 줄여 쓴다"
        );
        assert_eq!(IconSize::for_px(256.0), IconSize::Jumbo);
    }

    #[test]
    fn 경계_바로_위는_다음_단계로_넘어간다() {
        assert_eq!(IconSize::for_px(17.0), IconSize::Large);
        assert_eq!(IconSize::for_px(33.0), IconSize::ExtraLarge);
        assert_eq!(IconSize::for_px(49.0), IconSize::Jumbo);
    }

    #[test]
    fn 크기별로_서로_다른_이미지_리스트를_얻는다() {
        // 이 획득이 실패하면 모든 단계가 16px로 폴백해 보기 모드를 바꿔도 아이콘이 그대로다.
        // 화면 표시는 T10(격자 렌더)에서 확인하고, 여기서는 **리스트를 실제로 얻었는지**만 본다
        let cache = IconCache::new();
        assert_eq!(
            cache.himl_for(IconSize::Small),
            cache.himl(),
            "16px 단계가 기존 핸들과 달라졌다 — 자세히 보기가 회귀한다"
        );
        let jumbo = cache.himl_for(IconSize::Jumbo);
        assert!(!jumbo.is_invalid(), "256px 리스트가 유효하지 않다");
        assert_ne!(
            jumbo,
            cache.himl(),
            "256px 요청이 16px로 폴백했다 — SHGetImageList가 실패했거나 상수가 틀렸다"
        );
        // 네 단계가 모두 서로 다른 리스트여야 크기 구분이 성립한다
        let mut handles: Vec<isize> = [
            IconSize::Small,
            IconSize::Large,
            IconSize::ExtraLarge,
            IconSize::Jumbo,
        ]
        .iter()
        .map(|size| cache.himl_for(*size).0)
        .collect();
        handles.sort_unstable();
        let before = handles.len();
        handles.dedup();
        assert_eq!(handles.len(), before, "같은 리스트를 가리키는 단계가 있다");
    }

    #[test]
    fn 셸_상수는_서로_겹치지_않는다() {
        // 겹치면 두 단계가 같은 리스트를 가리켜 크기 구분이 사라진다
        let mut shils: Vec<u32> = [
            IconSize::Small,
            IconSize::Large,
            IconSize::ExtraLarge,
            IconSize::Jumbo,
        ]
        .iter()
        .map(|size| size.shil())
        .collect();
        shils.sort_unstable();
        let before = shils.len();
        shils.dedup();
        assert_eq!(shils.len(), before);
    }
}
