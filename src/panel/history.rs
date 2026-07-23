//! 탐색 히스토리 — 탭당 독립 (순수 로직, 단위테스트 대상. plan D9)
use std::path::{Path, PathBuf};

/// 브라우저식 히스토리: 커서 뒤를 절단하며 push, back/forward로 이동
pub struct History {
    items: Vec<PathBuf>,
    /// 현재 위치 (items가 비어있지 않으면 항상 유효 인덱스)
    cursor: usize,
}

impl History {
    /// 시작 경로 1개로 초기화
    pub fn new(start: PathBuf) -> History {
        History {
            items: vec![start],
            cursor: 0,
        }
    }

    pub fn current(&self) -> &Path {
        &self.items[self.cursor]
    }

    /// 새 이동 — 커서 뒤(앞으로 가기 목록)를 절단하고 추가.
    /// 현재 위치와 같은 경로면 중복 추가하지 않는다
    pub fn push(&mut self, path: PathBuf) {
        if self.items[self.cursor] == path {
            return;
        }
        self.items.truncate(self.cursor + 1);
        self.items.push(path);
        self.cursor += 1;
    }

    pub fn can_back(&self) -> bool {
        self.cursor > 0
    }

    pub fn can_forward(&self) -> bool {
        self.cursor + 1 < self.items.len()
    }

    /// 뒤로 — 이동했으면 새 현재 경로 반환 (항목은 보존 — 삭제된 폴더도 다시 시도 가능)
    pub fn back(&mut self) -> Option<&Path> {
        if self.can_back() {
            self.cursor -= 1;
            Some(self.current())
        } else {
            None
        }
    }

    /// 앞으로 — 이동했으면 새 현재 경로 반환
    pub fn forward(&mut self) -> Option<&Path> {
        if self.can_forward() {
            self.cursor += 1;
            Some(self.current())
        } else {
            None
        }
    }

    /// 뒤로 대상 미리보기 — 커서를 옮기지 않는다 (열거 성공 시에만 back()으로 커밋)
    pub fn peek_back(&self) -> Option<&Path> {
        if self.can_back() {
            Some(&self.items[self.cursor - 1])
        } else {
            None
        }
    }

    /// 앞으로 대상 미리보기 — 커서를 옮기지 않는다
    pub fn peek_forward(&self) -> Option<&Path> {
        if self.can_forward() {
            Some(&self.items[self.cursor + 1])
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(s: &str) -> PathBuf {
        PathBuf::from(s)
    }

    #[test]
    fn 기본_push_back_forward() {
        let mut h = History::new(p("C:\\a"));
        h.push(p("C:\\b"));
        h.push(p("C:\\c"));
        assert!(h.can_back());
        assert!(!h.can_forward());

        assert_eq!(h.back().unwrap(), p("C:\\b"));
        assert_eq!(h.back().unwrap(), p("C:\\a"));
        assert!(h.back().is_none());
        assert!(h.can_forward());

        assert_eq!(h.forward().unwrap(), p("C:\\b"));
        assert_eq!(h.forward().unwrap(), p("C:\\c"));
        assert!(h.forward().is_none());
    }

    #[test]
    fn 분기_이동은_앞으로_목록을_절단한다() {
        let mut h = History::new(p("C:\\a"));
        h.push(p("C:\\b"));
        h.push(p("C:\\c"));
        h.back();
        h.back(); // 현재 a
        h.push(p("C:\\x")); // b, c 절단
        assert_eq!(h.current(), p("C:\\x"));
        assert!(!h.can_forward());
        assert_eq!(h.back().unwrap(), p("C:\\a"));
        assert_eq!(h.forward().unwrap(), p("C:\\x"));
    }

    #[test]
    fn 같은_경로_중복_push는_무시된다() {
        let mut h = History::new(p("C:\\a"));
        h.push(p("C:\\a"));
        assert!(!h.can_back());
    }

    #[test]
    fn peek은_커서를_옮기지_않는다() {
        let mut h = History::new(p("C:\\a"));
        h.push(p("C:\\b"));
        assert_eq!(h.peek_back().unwrap(), p("C:\\a"));
        assert_eq!(h.current(), p("C:\\b")); // 커서 불변
        h.back();
        assert_eq!(h.peek_forward().unwrap(), p("C:\\b"));
        assert_eq!(h.current(), p("C:\\a"));
    }

    #[test]
    fn back_대상_경로는_삭제돼도_항목이_보존된다() {
        // Edge: back 대상 폴더가 사이에 삭제됨 → 히스토리 항목은 보존 (재시도 가능)
        let mut h = History::new(p("C:\\a"));
        h.push(p("C:\\살아있다"));
        h.push(p("C:\\b"));
        h.back(); // 삭제됐다고 가정해도 항목 유지
        assert_eq!(h.current(), p("C:\\살아있다"));
        assert!(h.can_forward());
    }
}
