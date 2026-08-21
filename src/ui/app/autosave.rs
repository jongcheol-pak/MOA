//! 세션 자동 저장 — 무엇이 바뀌든 곧 파일에 담기게 한다 (2026-08-21 사용자 요청).
//!
//! 종전에는 종료(`on_exit`)와 몇몇 지점(`persist_session` — 사이트 목록·앱 설정·트레이로
//! 숨기기)에서만 적었다. 그래서 탭을 열거나 분할을 바꾼 뒤 앱이 비정상 종료되면(패닉·강제
//! 종료·전원 차단) 그것이 통째로 사라졌다. 설치 프로그램이 실행 중인 앱을 강제로 닫는 경로가
//! 생기면서 그 창은 더 넓어졌다.
//!
//! **저장할 자리를 심지 않고 변화를 관측한다** — 상태를 바꾸는 곳마다 사람이 저장을 부르게
//! 하면 기능이 늘 때마다 빠뜨리고, 빠뜨린 것은 앱이 비정상 종료되기 전까지 드러나지 않는다.
//! 대신 일정 간격으로 지금 상태를 지난번 관측과 견주고, 달라진 뒤 잠깐 더 바뀌지 않으면
//! 그때 한 번 적는다.
//!
//! **조용해질 때까지 기다리는 이유**는 창 크기 조절처럼 연속으로 바뀌는 것 때문이다 —
//! 바뀔 때마다 적으면 끄는 동안 초당 수십 번 디스크를 때린다.

/// 지금 상태를 모아 견주는 간격(초).
///
/// 세션을 모으는 것 자체가 공짜가 아니라(워크스페이스·패널·탭 순회 + 사이트·큐 복제)
/// 프레임마다 하지 않는다
pub const CHECK_SECS: f64 = 0.25;

/// 마지막 변화로부터 이만큼 조용하면 적는다(초)
pub const QUIET_SECS: f64 = 0.4;

/// 변화 관측기.
///
/// 타입을 열어 둔 것은 이 상태 기계를 그 자체로 시험하기 위해서다 — 세션을 짓지 않고도
/// 「달라지면 기다렸다 적는다」를 검증할 수 있다
pub struct AutoSave<T> {
    /// 마지막으로 관측한 상태. 파일에 적힌 것과 같을 수도, 아직 아닐 수도 있다
    seen: Option<T>,
    /// 그 관측이 직전과 달라진 시각 — `None`이면 적을 것이 없다
    changed_at: Option<f64>,
    /// 다음 관측 시각
    next_check: f64,
}

impl<T> Default for AutoSave<T> {
    fn default() -> AutoSave<T> {
        // `seen`이 비어 있어 **시작 직후 한 번은 적는다** — 되살린 세션과 실제로 선 상태는
        // 어긋날 수 있어(화면 밖으로 나간 창을 끌어들이는 등) 그 편이 안전하다
        AutoSave {
            seen: None,
            changed_at: None,
            next_check: 0.0,
        }
    }
}

impl<T: PartialEq> AutoSave<T> {
    /// 지금 관측할 차례인가 — 아니면 상태를 모으는 값조차 치르지 않는다
    pub fn due(&self, now: f64) -> bool {
        now >= self.next_check
    }

    /// 아직 적지 못한 변화를 안고 있는가.
    ///
    /// 화면을 다시 깨울지 정하는 데 쓴다 — egui는 할 일이 없으면 프레임을 그리지 않아,
    /// 이것을 보지 않으면 마지막 변화가 **다음에 무슨 일이 일어날 때까지** 파일에 닿지 못한다
    pub fn pending(&self) -> bool {
        self.changed_at.is_some()
    }

    /// 지금 상태를 건넨다 — **적어야 할 때만** 그 상태를 돌려준다.
    ///
    /// 달라졌으면 시계를 다시 세우고 아무것도 돌려주지 않는다. 그 시계가 `QUIET_SECS`를
    /// 넘기도록 더 바뀌지 않았을 때 비로소 적을 것을 내놓는다
    pub fn observe(&mut self, now: f64, snapshot: T) -> Option<&T> {
        self.next_check = now + CHECK_SECS;
        if self.seen.as_ref() != Some(&snapshot) {
            self.seen = Some(snapshot);
            self.changed_at = Some(now);
            return None;
        }
        let changed = self.changed_at?;
        if now - changed < QUIET_SECS {
            return None;
        }
        self.changed_at = None;
        self.seen.as_ref()
    }

    /// 다른 경로가 방금 적었다 — 그것을 기준으로 삼아 곧바로 또 적지 않게 한다
    pub fn mark(&mut self, saved: T) {
        self.seen = Some(saved);
        self.changed_at = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 조용해진_뒤에_한_번_적는다() {
        let mut auto = AutoSave::default();
        // 첫 관측은 「달라진 것」으로 본다 — 되살린 상태와 실제 상태가 어긋날 수 있다
        assert!(auto.observe(0.0, 1).is_none());
        assert_eq!(auto.observe(1.0, 1), Some(&1));
        // 적고 난 뒤로는 바뀐 것이 없다
        assert!(auto.observe(2.0, 1).is_none());
    }

    #[test]
    fn 바뀌는_동안에는_적지_않는다() {
        let mut auto = AutoSave::default();
        assert!(auto.observe(0.0, 1).is_none());
        // 계속 바뀌면 그때마다 시계가 다시 선다 — 창 크기를 끄는 동안이 이렇다
        for (at, value) in [(0.3, 2), (0.6, 3), (0.9, 4)] {
            assert!(auto.observe(at, value).is_none(), "{at}초에 적으려 했다");
        }
        // 멈춰도 `QUIET_SECS`가 차기 전에는 적지 않는다
        assert!(auto.observe(1.2, 4).is_none());
        assert_eq!(auto.observe(1.4, 4), Some(&4));
    }

    #[test]
    fn 다른_경로가_적은_것은_다시_적지_않는다() {
        let mut auto = AutoSave::default();
        assert!(auto.observe(0.0, 1).is_none());
        // 사이트 등록처럼 그 자리에서 적는 경로가 있다 (`persist_session`)
        auto.mark(1);
        assert!(auto.observe(1.0, 1).is_none());
    }

    #[test]
    fn 관측_간격_전에는_상태를_모으지_않는다() {
        let mut auto = AutoSave::default();
        assert!(auto.due(0.0));
        assert!(auto.observe(0.0, 1).is_none());
        assert!(!auto.due(CHECK_SECS - 0.01));
        assert!(auto.due(CHECK_SECS));
    }
}
