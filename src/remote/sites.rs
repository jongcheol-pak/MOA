//! 등록된 사이트 목록 (FR-27·FR-28).
//!
//! 사이트 관리자가 다루는 목록의 정본이다. 비밀번호는 여기 들어오기 전에 `remote::secret`이
//! 봉인하며, 이 타입이 그대로 직렬화돼도 평문은 어디에도 없다.
//!
//! **`hide`와 `remove`는 다르다**(README §1): 사이드바 컨텍스트 메뉴의 `삭제`는 `hide`를 거쳐
//! **목록에서만** 뺀다 — 사이트 기록은 관리자 목록에 남아 되돌릴 수 있다(그 조작이 함께 걷어내는
//! 연결·원격 탭·전송 큐는 이 저장소가 모르는 일이며 `ui::app::detach_site`가 맡는다).
//! 기록 자체를 지우는 것은 사이트 관리자의 `삭제(D)`이고 그것이 `remove`다.
use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::remote::secret;
use crate::remote::types::{SiteId, SiteRecord};

/// 등록된 사이트 전부
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SiteStore {
    #[serde(default)]
    sites: Vec<SiteRecord>,
    /// 사이드바에서 숨긴 사이트 — 목록에는 그대로 남아 있다
    #[serde(default)]
    hidden: BTreeSet<SiteId>,
    /// 다음에 발급할 식별자. 이름이 바뀌어도 참조가 끊기지 않도록 번호로 잡는다
    #[serde(default)]
    next_id: u32,
}

impl SiteStore {
    pub fn new() -> SiteStore {
        SiteStore::default()
    }

    pub fn sites(&self) -> &[SiteRecord] {
        &self.sites
    }

    pub fn is_empty(&self) -> bool {
        self.sites.is_empty()
    }

    pub fn get(&self, id: SiteId) -> Option<&SiteRecord> {
        self.sites.iter().find(|site| site.id == id)
    }

    pub fn get_mut(&mut self, id: SiteId) -> Option<&mut SiteRecord> {
        self.sites.iter_mut().find(|site| site.id == id)
    }

    /// 사이드바에 보일 사이트들 — 숨긴 것은 빠진다
    pub fn visible(&self) -> impl Iterator<Item = &SiteRecord> {
        self.sites
            .iter()
            .filter(|site| !self.hidden.contains(&site.id))
    }

    /// 새 사이트를 만든다. 같은 이름이 이미 있으면 `(2)`·`(3)`을 붙여 피한다
    pub fn add(&mut self, name: &str) -> SiteId {
        let id = self.take_id();
        let name = self.unique_name(name, None);
        self.sites.push(SiteRecord::new(id, name));
        id
    }

    /// 이미 만들어진 기록을 넣는다 (세션 복원용). 식별자는 그대로 쓰고 이름만 겹치지 않게 한다
    pub fn insert(&mut self, mut record: SiteRecord) {
        record.name = self.unique_name(&record.name, Some(record.id));
        self.next_id = self.next_id.max(record.id.0.saturating_add(1));
        match self.sites.iter_mut().find(|site| site.id == record.id) {
            Some(existing) => *existing = record,
            None => self.sites.push(record),
        }
    }

    /// 이름을 바꾼다. 겹치면 `(2)`·`(3)`을 붙인다. 없는 사이트면 `false`
    pub fn rename(&mut self, id: SiteId, name: &str) -> bool {
        if self.get(id).is_none() {
            return false;
        }
        let name = self.unique_name(name, Some(id));
        match self.get_mut(id) {
            Some(site) => {
                site.name = name;
                true
            }
            None => false,
        }
    }

    /// 목록에서 지운다 (사이트 관리자의 `삭제(D)`). 숨김 표시도 함께 정리한다
    pub fn remove(&mut self, id: SiteId) -> bool {
        let before = self.sites.len();
        self.sites.retain(|site| site.id != id);
        self.hidden.remove(&id);
        self.sites.len() != before
    }

    /// 설정을 그대로 베낀 새 사이트를 만든다 (`복제(I)`). 이름은 `(2)`·`(3)`으로 갈린다
    pub fn duplicate(&mut self, id: SiteId) -> Option<SiteId> {
        let source = self.get(id)?.clone();
        let new_id = self.take_id();
        let name = self.unique_name(&source.name, None);
        self.sites.push(SiteRecord {
            id: new_id,
            name,
            ..source
        });
        Some(new_id)
    }

    /// 사이드바에서만 감춘다 — 사이트는 관리자 목록에 그대로 남는다 (README §1)
    pub fn hide(&mut self, id: SiteId) {
        if self.get(id).is_some() {
            self.hidden.insert(id);
        }
    }

    pub fn unhide(&mut self, id: SiteId) {
        self.hidden.remove(&id);
    }

    pub fn is_hidden(&self, id: SiteId) -> bool {
        self.hidden.contains(&id)
    }

    /// 비밀번호를 봉인해 담는다. **봉인에 실패하면 담지 않고 `false`**를 돌려준다 —
    /// 호출부는 "비밀번호를 저장하지 못했습니다"로 알리고 평문으로 대신 담지 않는다 (FR-28)
    pub fn set_password(&mut self, id: SiteId, plain: &str) -> bool {
        let Some(sealed) = secret::seal(plain) else {
            return false;
        };
        match self.get_mut(id) {
            Some(site) => {
                site.password_sealed = sealed;
                true
            }
            None => false,
        }
    }

    /// 연결 직전에 비밀번호를 푼다. 저장된 것이 없거나(빈 값) 풀지 못하면 `None`이다 —
    /// 다른 PC에서 가져온 설정이 여기 걸린다(그때는 다시 입력받는다)
    pub fn password(&self, id: SiteId) -> Option<String> {
        let site = self.get(id)?;
        if site.password_sealed.is_empty() {
            return None;
        }
        secret::unseal(&site.password_sealed)
    }

    fn take_id(&mut self) -> SiteId {
        let id = SiteId(self.next_id);
        self.next_id += 1;
        id
    }

    /// 겹치지 않는 이름을 만든다 — `사이트`가 있으면 `사이트 (2)`, 그것도 있으면 `사이트 (3)`.
    ///
    /// `exclude`는 자기 자신이다(이름을 바꾸지 않고 저장할 때 자기 이름에 걸려 `(2)`가 붙는 것을 막는다).
    fn unique_name(&self, desired: &str, exclude: Option<SiteId>) -> String {
        let base = match desired.trim() {
            "" => crate::i18n::site_default_name(),
            trimmed => trimmed,
        };
        if !self.name_taken(base, exclude) {
            return base.to_owned();
        }
        // 2부터 올려 가며 빈자리를 찾는다
        let mut suffix = 2u32;
        loop {
            let candidate = format!("{base} ({suffix})");
            if !self.name_taken(&candidate, exclude) {
                return candidate;
            }
            suffix += 1;
        }
    }

    fn name_taken(&self, name: &str, exclude: Option<SiteId>) -> bool {
        self.sites
            .iter()
            .any(|site| site.name == name && Some(site.id) != exclude)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::remote::types::Protocol;

    #[test]
    fn 같은_이름은_번호를_붙여_피한다() {
        let mut store = SiteStore::new();
        let first = store.add("사이트");
        let second = store.add("사이트");
        let third = store.add("사이트");

        assert_eq!(store.get(first).expect("첫째").name, "사이트");
        assert_eq!(store.get(second).expect("둘째").name, "사이트 (2)");
        assert_eq!(store.get(third).expect("셋째").name, "사이트 (3)");
    }

    #[test]
    fn 이름을_비워_두면_기본_이름이_붙는다() {
        // 기본 이름은 카탈로그가 정한다 — 언어를 고정하고 원문과 견준다
        let _guard =
            crate::i18n::LanguageGuard::lock(crate::app::settings::LanguageSetting::Korean);
        let mut store = SiteStore::new();
        let first = store.add("   ");
        let second = store.add("");
        assert_eq!(store.get(first).expect("첫째").name, "새 사이트");
        assert_eq!(store.get(second).expect("둘째").name, "새 사이트 (2)");
    }

    #[test]
    fn 이름을_그대로_두고_저장해도_번호가_붙지_않는다() {
        let mut store = SiteStore::new();
        let id = store.add("배포 서버");
        assert!(store.rename(id, "배포 서버"));
        assert_eq!(store.get(id).expect("사이트").name, "배포 서버");
        // 다른 사이트 이름과 겹치면 번호가 붙는다
        let other = store.add("스테이징");
        assert!(store.rename(other, "배포 서버"));
        assert_eq!(store.get(other).expect("사이트").name, "배포 서버 (2)");
    }

    #[test]
    fn 복제는_설정을_베끼고_이름만_가른다() {
        let mut store = SiteStore::new();
        let id = store.add("배포 서버");
        if let Some(site) = store.get_mut(id) {
            site.protocol = Protocol::Sftp;
            site.host = "example.test".to_owned();
            site.port = 2222;
            site.user = "deploy".to_owned();
        }
        let copy = store.duplicate(id).expect("복제");

        let original = store.get(id).expect("원본").clone();
        let copied = store.get(copy).expect("사본");
        assert_eq!(copied.name, "배포 서버 (2)");
        assert_eq!(copied.protocol, original.protocol);
        assert_eq!(copied.host, original.host);
        assert_eq!(copied.port, original.port);
        assert_eq!(copied.user, original.user);
        assert_ne!(copied.id, original.id, "식별자는 새로 받는다");
    }

    #[test]
    fn 없는_사이트는_복제도_이름_바꾸기도_되지_않는다() {
        let mut store = SiteStore::new();
        assert!(store.duplicate(SiteId(99)).is_none());
        assert!(!store.rename(SiteId(99), "아무거나"));
        assert!(!store.remove(SiteId(99)));
        assert!(store.is_empty());
    }

    #[test]
    fn 숨기기는_목록에서_지우지_않는다() {
        // README §1 — 사이드바 컨텍스트 메뉴의 `삭제`는 바로가기만 없앤다
        let mut store = SiteStore::new();
        let kept = store.add("보이는 사이트");
        let hidden = store.add("숨긴 사이트");
        store.hide(hidden);

        assert!(store.is_hidden(hidden));
        assert_eq!(store.sites().len(), 2, "관리자 목록에는 그대로 있다");
        let visible: Vec<&str> = store.visible().map(|site| site.name.as_str()).collect();
        assert_eq!(visible, vec!["보이는 사이트"]);

        store.unhide(hidden);
        assert_eq!(store.visible().count(), 2);
        // 숨긴 채로 지우면 숨김 표시도 함께 사라진다
        store.hide(hidden);
        assert!(store.remove(hidden));
        assert!(!store.is_hidden(hidden));
        assert_eq!(store.sites().len(), 1);
        assert_eq!(store.get(kept).expect("남은 사이트").name, "보이는 사이트");
    }

    #[test]
    fn 비밀번호는_봉인해_담고_풀어_쓴다() {
        let mut store = SiteStore::new();
        let id = store.add("배포 서버");
        assert!(store.set_password(id, "진짜비밀번호"));
        assert_eq!(store.password(id).as_deref(), Some("진짜비밀번호"));
    }

    #[test]
    fn 저장한_설정에_평문_비밀번호가_없다() {
        // 설정 파일은 그대로 디스크에 남는다 — 여기 평문이 보이면 봉인의 뜻이 없다
        let mut store = SiteStore::new();
        let id = store.add("배포 서버");
        if let Some(site) = store.get_mut(id) {
            site.host = "example.test".to_owned();
            site.user = "deploy".to_owned();
        }
        assert!(store.set_password(id, "찾을수있는평문"));

        let json = serde_json::to_string(&store).expect("직렬화");
        assert!(!json.contains("찾을수있는평문"), "평문이 남았다: {json}");
        // 왕복해도 같고 비밀번호도 그대로 풀린다
        let back: SiteStore = serde_json::from_str(&json).expect("역직렬화");
        assert_eq!(back, store);
        assert_eq!(back.password(id).as_deref(), Some("찾을수있는평문"));
    }

    #[test]
    fn 비밀번호가_없으면_풀_것도_없다() {
        let mut store = SiteStore::new();
        let id = store.add("익명 서버");
        assert_eq!(store.password(id), None);
        // 빈 비밀번호를 담아도 마찬가지다
        assert!(store.set_password(id, ""));
        assert_eq!(store.password(id), None);
        assert!(store.get(id).expect("사이트").password_sealed.is_empty());
    }

    #[test]
    fn 다른_pc에서_가져온_봉인은_풀리지_않는다() {
        // 그때는 사용자에게 다시 입력받는다 — 조용히 빈 비밀번호로 연결하지 않는다
        let mut store = SiteStore::new();
        let id = store.add("남의 설정");
        if let Some(site) = store.get_mut(id) {
            site.password_sealed = vec![0xde, 0xad, 0xbe, 0xef];
        }
        assert_eq!(store.password(id), None);
    }

    #[test]
    fn 복원한_기록은_식별자를_지키고_다음_번호를_밀어_올린다() {
        let mut store = SiteStore::new();
        store.insert(SiteRecord::new(SiteId(7), "복원된 사이트".to_owned()));
        assert_eq!(store.get(SiteId(7)).expect("복원").name, "복원된 사이트");
        // 다음에 만드는 사이트가 7번을 다시 쓰면 참조가 뒤섞인다
        let fresh = store.add("새것");
        assert_eq!(fresh, SiteId(8));

        // 같은 식별자를 다시 넣으면 갈아 끼운다(이름은 자기 자신과 겹치지 않는다)
        store.insert(SiteRecord::new(SiteId(7), "복원된 사이트".to_owned()));
        assert_eq!(store.sites().len(), 2);
        assert_eq!(store.get(SiteId(7)).expect("복원").name, "복원된 사이트");
    }
}
