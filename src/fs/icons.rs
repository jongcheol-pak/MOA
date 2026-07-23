//! 셸 아이콘·종류 문자열 조회 — 시스템 이미지 리스트 공유 + 확장자 캐시 (plan D8)
use std::collections::HashMap;
use windows::Win32::Storage::FileSystem::{FILE_ATTRIBUTE_DIRECTORY, FILE_ATTRIBUTE_NORMAL};
use windows::Win32::UI::Controls::HIMAGELIST;
use windows::Win32::UI::Shell::{
    SHFILEINFOW, SHGFI_SMALLICON, SHGFI_SYSICONINDEX, SHGFI_TYPENAME, SHGFI_USEFILEATTRIBUTES,
    SHGetFileInfoW,
};
use windows::core::HSTRING;

/// 개별(파일별) 아이콘이 필요한 확장자 — 실행 파일·바로가기는 파일마다 아이콘이 다르다
const PER_FILE_ICON_EXTS: [&str; 3] = ["exe", "lnk", "ico"];

/// 확장자 → 시스템 이미지 리스트 인덱스/종류명 캐시.
/// 아이콘 자체를 복사하지 않고 시스템 공유 이미지 리스트 인덱스만 보관한다 (NFR-2)
pub struct IconCache {
    himl: HIMAGELIST,
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
        IconCache {
            himl: HIMAGELIST(himl as isize),
            icon_by_ext: HashMap::new(),
            type_by_ext: HashMap::new(),
            icon_by_path: HashMap::new(),
            dir_icon: info.iIcon,
            dir_type: wide_to_string(&info.szTypeName),
        }
    }

    /// ListView LVSIL_SMALL에 연결할 시스템 이미지 리스트
    pub fn himl(&self) -> HIMAGELIST {
        self.himl
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
