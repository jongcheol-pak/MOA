//! 전송 큐 — 무엇을 어디로 옮길지의 목록과 그 상태 (FR-36).
//!
//! **순수 모델이다** — 스레드도 I/O도 없다. 실제로 옮기는 것은 `remote::transfer`(T18)가
//! 이 큐에서 꺼내 연결 워커에 맡긴다. 나누는 이유는 큐의 규칙(자리 배정·필터·요약)을
//! 서버 없이 전부 시험할 수 있게 하기 위함이다 (D25).
//!
//! **방향·식별자는 새로 만들지 않는다** — `remote::connection`이 T4에서 이미
//! `TransferId`·`TransferDirection`을 도입했고(워커 명령이 그것을 싣는다), 여기서 같은 뜻의
//! 타입을 또 만들면 경계마다 옮겨 담게 된다 (plan T17 신규 심볼 목록의 `Direction`이 이것이다).
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use crate::remote::connection::{TransferDirection, TransferId};
use crate::remote::types::{RemotePath, SiteId};

/// 크기를 모르거나 셀 수 없을 때 화면에 보일 값 (plan Edge Case: size 0 → 진행률 `—`)
pub const UNKNOWN: &str = "—";

/// 전송 한 건의 상태.
///
/// `Active`가 **보낸 바이트와 속도를 함께** 드는 이유: 화면이 둘을 같은 줄에 그리는데
/// (`전송 중 · 12.4 MB/s` — 인벤토리 #45), 따로 두면 한쪽만 갱신된 순간이 보인다
#[derive(Debug, Clone, PartialEq)]
pub enum TransferState {
    /// 자리를 기다린다
    Wait,
    Active {
        sent: u64,
        /// 초당 바이트 — 0이면 아직 잴 수 없다
        speed: u64,
    },
    Done,
    /// 서버가 준 사유를 그대로 보인다 (인벤토리 #48)
    Error {
        message: String,
    },
}

impl TransferState {
    pub fn is_active(&self) -> bool {
        matches!(self, TransferState::Active { .. })
    }

    pub fn is_done(&self) -> bool {
        matches!(self, TransferState::Done)
    }

    pub fn is_error(&self) -> bool {
        matches!(self, TransferState::Error { .. })
    }

    /// 아직 끝나지 않았는가 — 대기·진행이 여기 든다
    pub fn is_pending(&self) -> bool {
        matches!(self, TransferState::Wait | TransferState::Active { .. })
    }
}

/// 큐에 담긴 전송 한 건.
///
/// **폴더가 아니라 파일 단위다** — 폴더 전송은 큐에 넣기 전에 펼친다(plan 비추상화 선언).
/// 그래야 진행률·이어받기·취소가 한 파일을 기준으로 단순해진다
#[derive(Debug, Clone, PartialEq)]
pub struct TransferItem {
    pub id: TransferId,
    /// 어느 사이트의 전송인가 — 연결별 탭(인벤토리 #36)과 자리 배정이 이것으로 갈린다
    pub site: SiteId,
    pub direction: TransferDirection,
    pub local: PathBuf,
    pub remote: RemotePath,
    /// 전체 크기. **0이면 모른다는 뜻**이다(서버가 알리지 않는 경우가 있다)
    pub size: u64,
    pub state: TransferState,
}

impl TransferItem {
    /// 0.0~1.0 진행률. 크기를 모르면 `None`이다 — 화면이 `—`를 보인다
    pub fn progress(&self) -> Option<f32> {
        if self.size == 0 {
            return None;
        }
        let sent = match &self.state {
            TransferState::Active { sent, .. } => *sent,
            TransferState::Done => self.size,
            _ => 0,
        };
        Some((sent as f32 / self.size as f32).clamp(0.0, 1.0))
    }

    /// 아직 보내야 할 바이트 — 남은 시간 계산의 재료다
    fn remaining(&self) -> u64 {
        match &self.state {
            TransferState::Active { sent, .. } => self.size.saturating_sub(*sent),
            TransferState::Wait => self.size,
            _ => 0,
        }
    }
}

/// 큐 화면의 필터 (인벤토리 #29·#31·#32).
///
/// 셋은 **같은 항목 집합을 거른다** — `Done`·`Error`는 `All`의 부분집합이라
/// 건수를 합치면 전체와 맞는다 (README §6)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum QueueFilter {
    #[default]
    All,
    Done,
    Error,
}

impl QueueFilter {
    /// 이 거르개에 걸리는가 — `filter`·`count`·`counts_by_site` 셋이 같은 판정을 쓴다
    fn matches(self, state: &TransferState) -> bool {
        match self {
            QueueFilter::All => true,
            QueueFilter::Done => state.is_done(),
            QueueFilter::Error => state.is_error(),
        }
    }
}

/// 상태 표시줄·큐 머리글이 쓰는 요약 (인벤토리 #54)
#[derive(Debug, Clone, PartialEq)]
pub struct QueueSummary {
    /// 아직 끝나지 않은 건수(대기 + 진행)
    pub pending: usize,
    /// 지금 진행 중인 전송들의 속도 합 (바이트/초)
    pub speed: u64,
    /// 남은 시간(초). 속도가 0이면 잴 수 없어 `None`이다 — 화면이 `—`를 보인다
    pub eta_secs: Option<u64>,
}

/// 전송 큐 — 목록과 상태의 정본 (FR-36).
#[derive(Debug, Default)]
pub struct TransferQueue {
    items: Vec<TransferItem>,
    /// 다음에 발급할 번호
    next_id: u64,
    /// `⏸`로 멈춘 상태 — 멈춰 있으면 새 자리를 내주지 않는다(진행 중인 것은 러너가 멈춘다)
    paused: bool,
}

impl TransferQueue {
    pub fn new() -> TransferQueue {
        TransferQueue::default()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn items(&self) -> &[TransferItem] {
        &self.items
    }

    pub fn get(&self, id: TransferId) -> Option<&TransferItem> {
        self.items.iter().find(|item| item.id == id)
    }

    pub fn is_paused(&self) -> bool {
        self.paused
    }

    /// 전송 한 건을 대기로 넣고 그 번호를 돌려준다.
    ///
    /// **같은 파일을 두 번 넣어도 막지 않는다** — 덮어쓰기 확인은 실제로 쓰기 직전에
    /// 실행기가 한다(plan Edge Case). 여기서 막으면 사용자가 일부러 다시 받는 것도 막힌다
    pub fn enqueue(
        &mut self,
        site: SiteId,
        direction: TransferDirection,
        local: PathBuf,
        remote: RemotePath,
        size: u64,
    ) -> TransferId {
        let id = TransferId(self.next_id);
        self.next_id += 1;
        self.items.push(TransferItem {
            id,
            site,
            direction,
            local,
            remote,
            size,
            state: TransferState::Wait,
        });
        id
    }

    /// 그 사이트에서 **지금 시작해도 되는** 전송들을 앞에서부터 골라 준다.
    ///
    /// `slots`는 연결 관리자가 배정한 전송 채널 수다 (D4). **`0`을 받아도 1로 본다** —
    /// 상한이 1인 사이트는 탐색 채널이 전송을 겸하므로(매니저만 아는 사정) 0을 그대로 쓰면
    /// 그 설정에서 전송이 영원히 시작되지 않는다.
    ///
    /// 이미 진행 중인 건수를 빼고 남는 자리만 내준다 — 한 사이트가 가득 차도 다른 사이트는
    /// 그대로 나온다(NFR-11)
    pub fn next_for(&self, site: SiteId, slots: u8) -> Vec<TransferId> {
        if self.paused {
            return Vec::new();
        }
        let slots = slots.max(1) as usize;
        let active = self
            .items
            .iter()
            .filter(|item| item.site == site && item.state.is_active())
            .count();
        let free = slots.saturating_sub(active);
        self.items
            .iter()
            .filter(|item| item.site == site && item.state == TransferState::Wait)
            .take(free)
            .map(|item| item.id)
            .collect()
    }

    /// 상태를 갈아 끼운다. 없는 번호면 아무 일도 하지 않는다
    pub fn update(&mut self, id: TransferId, state: TransferState) {
        if let Some(item) = self.items.iter_mut().find(|item| item.id == id) {
            item.state = state;
        }
    }

    /// 취소 — 항목을 **목록에서 지운다**. 활성 자리가 그 자리에서 비워진다 (Acceptance ④).
    ///
    /// 실패로 남기지 않는 이유: 실패 탭(인벤토리 #32)은 서버가 거부한 것을 모아 보는 자리인데,
    /// 사용자가 스스로 그만둔 것까지 섞이면 무엇을 다시 걸어야 할지 가려낼 수 없다
    pub fn cancel(&mut self, id: TransferId) -> bool {
        let before = self.items.len();
        self.items.retain(|item| item.id != id);
        self.items.len() != before
    }

    /// `⏸` 토글 — 멈추면 새 자리를 내주지 않는다.
    ///
    /// 진행 중인 전송을 여기서 되돌리지 않는다(그것은 워커에 닿아야 하는 일이라 T18의 몫이다) —
    /// 큐는 "새로 시작하지 않는다"만 안다
    pub fn set_paused(&mut self, paused: bool) {
        self.paused = paused;
    }

    /// 고른 항목들을 다시 대기로 되돌린다 (행 메뉴 `전체 다시 시도`).
    ///
    /// **실패한 것만** 바꾼다 — 목록에 완료·진행 중이 섞여 있어도 그것들은 건드리지 않는다
    /// (`전체`라는 말이 "보이는 목록"을 뜻하지 "모든 상태"를 뜻하지는 않는다)
    pub fn retry(&mut self, ids: &[TransferId]) {
        let ids: HashSet<TransferId> = ids.iter().copied().collect();
        for item in self
            .items
            .iter_mut()
            .filter(|item| ids.contains(&item.id) && item.state.is_error())
        {
            item.state = TransferState::Wait;
        }
    }

    /// 고른 항목들을 목록에서 지운다 (행 메뉴 `삭제`·`전체 삭제`의 여러 건 판).
    ///
    /// 진행 중인 것의 워커 정지와 `.part` 정리는 `transfer::TransferRunner::cancel`이 맡는다 —
    /// 큐는 목록만 안다(`cancel`과 같은 분담)
    pub fn remove(&mut self, ids: &[TransferId]) {
        let ids: HashSet<TransferId> = ids.iter().copied().collect();
        self.items.retain(|item| !ids.contains(&item.id));
    }

    /// 끝난 것들을 치운다 (`✕` — 인벤토리 #33). 실패는 남긴다 — 다시 걸 수 있어야 한다
    pub fn clear_done(&mut self) {
        self.items.retain(|item| !item.state.is_done());
    }

    /// 연결이 끊긴 사이트의 진행 중 항목을 **대기로 되돌린다** (plan Edge Case).
    ///
    /// 실패로 두지 않는 이유: 서버가 거부한 것이 아니라 우리 쪽 연결이 사라진 것이고,
    /// 다시 연결되면 그대로 이어 보내면 된다
    pub fn requeue_site(&mut self, site: SiteId) {
        for item in self
            .items
            .iter_mut()
            .filter(|item| item.site == site && item.state.is_active())
        {
            item.state = TransferState::Wait;
        }
    }

    /// 필터에 걸리는 항목들 — 화면이 그대로 그린다
    pub fn filter(&self, filter: QueueFilter) -> Vec<&TransferItem> {
        self.items
            .iter()
            .filter(|item| filter.matches(&item.state))
            .collect()
    }

    /// 필터별 건수 — 탭 라벨의 `(N)`이다 (인벤토리 #29·#31·#32)
    pub fn count(&self, filter: QueueFilter) -> usize {
        self.items
            .iter()
            .filter(|item| filter.matches(&item.state))
            .count()
    }

    /// 대기 건수·속도 합·남은 시간 (인벤토리 #54).
    ///
    /// 남은 시간은 **남은 바이트 ÷ 지금 속도**다 — 속도가 0이면 나눌 수 없어 `None`이고
    /// 화면은 `—`를 보인다(0초로 적으면 곧 끝난다는 거짓말이 된다)
    pub fn summary(&self) -> QueueSummary {
        let mut pending = 0usize;
        let mut speed = 0u64;
        let mut remaining = 0u64;
        for item in &self.items {
            if item.state.is_pending() {
                pending += 1;
                remaining = remaining.saturating_add(item.remaining());
            }
            if let TransferState::Active { speed: rate, .. } = &item.state {
                speed = speed.saturating_add(*rate);
            }
        }
        QueueSummary {
            pending,
            speed,
            eta_secs: (speed > 0).then(|| remaining / speed),
        }
    }

    /// 전체 진행률 0.0~1.0 — 상태 표시줄의 막대다 (인벤토리 #55).
    ///
    /// 크기를 아는 항목만 센다. 아무것도 없으면 `None`이라 막대를 비워 둔다
    pub fn overall_progress(&self) -> Option<f32> {
        let mut total = 0u64;
        let mut sent = 0u64;
        for item in self.items.iter().filter(|item| item.size > 0) {
            total = total.saturating_add(item.size);
            sent = sent.saturating_add(match &item.state {
                TransferState::Active { sent, .. } => *sent,
                TransferState::Done => item.size,
                _ => 0,
            });
        }
        (total > 0).then(|| (sent as f32 / total as f32).clamp(0.0, 1.0))
    }

    /// 사이트별 건수 — 연결별 탭의 `(N)`이다 (인벤토리 #36).
    ///
    /// **거르개를 함께 받는다** — `성공` 탭을 보는데 아래 줄이 전체 건수를 적으면
    /// 목록이 비어 있는데 `(1)`이 서 있게 된다 (2026-08-18 사용자 보고).
    /// 탭에 어떤 사이트가 서는지(멤버십)는 이 건수로 정하지 않는다 — 그것까지 걸러 내면
    /// 그 거르개에 항목이 없는 서버가 탭에서 통째로 사라진다(호출부가 `All`로 따로 구한다)
    pub fn counts_by_site(&self, filter: QueueFilter) -> HashMap<SiteId, usize> {
        let mut counts = HashMap::new();
        for item in self.items.iter().filter(|item| filter.matches(&item.state)) {
            *counts.entry(item.site).or_insert(0) += 1;
        }
        counts
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn site(n: u32) -> SiteId {
        SiteId(n)
    }

    fn queue_with(count: usize, site_id: SiteId) -> (TransferQueue, Vec<TransferId>) {
        let mut queue = TransferQueue::new();
        let ids = (0..count)
            .map(|i| {
                queue.enqueue(
                    site_id,
                    TransferDirection::Download,
                    PathBuf::from(format!(r"C:\down\{i}.bin")),
                    RemotePath::new(&format!("/pub/{i}.bin")),
                    1000,
                )
            })
            .collect();
        (queue, ids)
    }

    #[test]
    fn 배정된_자리_수만큼만_활성으로_내준다() {
        // Acceptance ① — 고정 2가 아니라 매니저가 보고한 값이다 (D4)
        let (mut queue, ids) = queue_with(5, site(1));
        assert_eq!(queue.next_for(site(1), 2).len(), 2);

        queue.update(ids[0], TransferState::Active { sent: 0, speed: 0 });
        queue.update(ids[1], TransferState::Active { sent: 0, speed: 0 });
        assert!(
            queue.next_for(site(1), 2).is_empty(),
            "자리가 찼는데 더 내줬다"
        );
        // 한 건이 끝나면 그 자리가 다음 대기에게 간다
        queue.update(ids[0], TransferState::Done);
        assert_eq!(queue.next_for(site(1), 2), vec![ids[2]]);
    }

    #[test]
    fn 상한이_1인_사이트에도_한_건은_내준다() {
        // 2차 리뷰 M1 — 0을 그대로 쓰면 그 설정에서 전송이 영원히 시작되지 않는다.
        // 큐는 탐색 채널이 전송을 겸한다는 사정을 모른다
        let (queue, ids) = queue_with(3, site(1));
        assert_eq!(queue.next_for(site(1), 0), vec![ids[0]]);
        assert_eq!(queue.next_for(site(1), 1), vec![ids[0]]);
    }

    #[test]
    fn 한_사이트가_가득_차도_다른_사이트는_계속_나온다() {
        // NFR-11 — 한 서버의 지연이 다른 서버의 전송을 막지 않는다
        let mut queue = TransferQueue::new();
        let busy = queue.enqueue(
            site(1),
            TransferDirection::Upload,
            PathBuf::from(r"C:\a.bin"),
            RemotePath::new("/a.bin"),
            10,
        );
        let other = queue.enqueue(
            site(2),
            TransferDirection::Upload,
            PathBuf::from(r"C:\b.bin"),
            RemotePath::new("/b.bin"),
            10,
        );
        queue.update(busy, TransferState::Active { sent: 0, speed: 0 });

        assert!(queue.next_for(site(1), 1).is_empty());
        assert_eq!(queue.next_for(site(2), 1), vec![other]);
    }

    #[test]
    fn 필터는_같은_집합을_거르고_건수가_전체와_맞는다() {
        // Acceptance ② (README §6) — 성공·실패 탭이 전체의 부분집합이다
        let (mut queue, ids) = queue_with(4, site(1));
        queue.update(ids[0], TransferState::Done);
        queue.update(ids[1], TransferState::Done);
        queue.update(
            ids[2],
            TransferState::Error {
                message: "550 권한 거부".to_owned(),
            },
        );

        assert_eq!(queue.count(QueueFilter::All), 4);
        assert_eq!(queue.count(QueueFilter::Done), 2);
        assert_eq!(queue.count(QueueFilter::Error), 1);
        // 끝나지 않은 것 1건을 더하면 전체가 된다
        let pending = queue
            .items()
            .iter()
            .filter(|item| item.state.is_pending())
            .count();
        assert_eq!(
            queue.count(QueueFilter::Done) + queue.count(QueueFilter::Error) + pending,
            queue.count(QueueFilter::All)
        );
        // 거른 항목이 실제로 그 상태다
        assert!(
            queue
                .filter(QueueFilter::Done)
                .iter()
                .all(|item| item.state.is_done())
        );
        assert!(
            queue
                .filter(QueueFilter::Error)
                .iter()
                .all(|item| item.state.is_error())
        );
    }

    #[test]
    fn 요약은_대기_건수와_속도와_남은_시간을_센다() {
        // Acceptance ③
        let (mut queue, ids) = queue_with(3, site(1));
        queue.update(
            ids[0],
            TransferState::Active {
                sent: 400,
                speed: 200,
            },
        );
        queue.update(ids[1], TransferState::Done);

        let summary = queue.summary();
        assert_eq!(summary.pending, 2, "진행 1 + 대기 1");
        assert_eq!(summary.speed, 200);
        // 남은 바이트 = 진행 600 + 대기 1000 = 1600, 속도 200 → 8초
        assert_eq!(summary.eta_secs, Some(8));
    }

    #[test]
    fn 속도가_0이면_남은_시간을_적지_않는다() {
        // Acceptance ③ 뒷문장 — 0초로 적으면 곧 끝난다는 거짓말이 된다
        let (queue, _) = queue_with(2, site(1));
        let summary = queue.summary();
        assert_eq!(summary.pending, 2);
        assert_eq!(summary.speed, 0);
        assert_eq!(summary.eta_secs, None);
        assert_eq!(UNKNOWN, "—");
    }

    #[test]
    fn 취소하면_활성_자리가_곧바로_빈다() {
        // Acceptance ④
        let (mut queue, ids) = queue_with(3, site(1));
        queue.update(ids[0], TransferState::Active { sent: 10, speed: 5 });
        assert!(queue.next_for(site(1), 1).is_empty());

        assert!(queue.cancel(ids[0]));
        assert_eq!(queue.len(), 2, "취소한 항목은 목록에서 사라진다");
        assert_eq!(queue.next_for(site(1), 1), vec![ids[1]]);
        // 없는 번호를 취소해도 조용히 아무 일도 없다
        assert!(!queue.cancel(ids[0]));
    }

    #[test]
    fn 사이트별_건수가_연결별_탭의_숫자다() {
        // Acceptance ⑤ (인벤토리 #36)
        let mut queue = TransferQueue::new();
        for _ in 0..3 {
            queue.enqueue(
                site(1),
                TransferDirection::Download,
                PathBuf::from(r"C:\a"),
                RemotePath::new("/a"),
                1,
            );
        }
        queue.enqueue(
            site(2),
            TransferDirection::Download,
            PathBuf::from(r"C:\b"),
            RemotePath::new("/b"),
            1,
        );

        let counts = queue.counts_by_site(QueueFilter::All);
        assert_eq!(counts.get(&site(1)), Some(&3));
        assert_eq!(counts.get(&site(2)), Some(&1));
        assert_eq!(counts.values().sum::<usize>(), queue.len());
    }

    #[test]
    fn 목록_단위로_다시_걸고_지운다() {
        // 2026-08-18 행 메뉴 `전체 다시 시도`·`전체 삭제` — 대상은 호출부가 고른 번호들이다
        let mut queue = TransferQueue::new();
        let 건다 = |queue: &mut TransferQueue, n: u8| {
            queue.enqueue(
                site(1),
                TransferDirection::Upload,
                PathBuf::from(format!(r"C:\{n}")),
                RemotePath::new(&format!("/{n}")),
                1,
            )
        };
        let 실패 = 건다(&mut queue, 1);
        let 완료 = 건다(&mut queue, 2);
        let 대기 = 건다(&mut queue, 3);
        queue.update(
            실패,
            TransferState::Error {
                message: "550".to_owned(),
            },
        );
        queue.update(완료, TransferState::Done);

        // `retry`는 **실패한 것만** 되돌린다 — 목록에 섞인 완료·대기는 그대로
        queue.retry(&[실패, 완료, 대기]);
        assert_eq!(queue.count(QueueFilter::Error), 0);
        assert_eq!(queue.count(QueueFilter::Done), 1, "완료는 그대로여야 한다");
        assert_eq!(queue.len(), 3);

        // `remove`는 상태를 가리지 않고 고른 것을 지운다
        queue.remove(&[실패, 대기]);
        assert_eq!(queue.len(), 1);
        assert!(queue.get(완료).is_some());

        // 목록에 없는 번호를 섞어도 조용히 지나간다
        queue.remove(&[TransferId(9999)]);
        assert_eq!(queue.len(), 1);
    }

    #[test]
    fn 사이트별_건수가_거르개를_따른다() {
        // 2026-08-18 — `성공` 탭인데 아래 줄이 전체 건수를 적던 것을 고친다
        let mut queue = TransferQueue::new();
        let 실패 = queue.enqueue(
            site(1),
            TransferDirection::Upload,
            PathBuf::from(r"C:"),
            RemotePath::new("/a"),
            1,
        );
        let 완료 = queue.enqueue(
            site(1),
            TransferDirection::Upload,
            PathBuf::from(r"C:"),
            RemotePath::new("/b"),
            1,
        );
        queue.update(
            실패,
            TransferState::Error {
                message: "550".to_owned(),
            },
        );
        queue.update(완료, TransferState::Done);

        assert_eq!(
            queue.counts_by_site(QueueFilter::All).get(&site(1)),
            Some(&2)
        );
        assert_eq!(
            queue.counts_by_site(QueueFilter::Done).get(&site(1)),
            Some(&1)
        );
        assert_eq!(
            queue.counts_by_site(QueueFilter::Error).get(&site(1)),
            Some(&1)
        );
        // 거르개에 하나도 안 걸리면 그 사이트 자리 자체가 없다 —
        // 탭을 세우는 쪽은 `All` 집계를 따로 봐야 한다(호출부 규약)
        let mut 완료만 = TransferQueue::new();
        let 그것 = 완료만.enqueue(
            site(2),
            TransferDirection::Upload,
            PathBuf::from(r"C:\c"),
            RemotePath::new("/c"),
            1,
        );
        완료만.update(그것, TransferState::Done);
        assert!(
            !완료만
                .counts_by_site(QueueFilter::Error)
                .contains_key(&site(2))
        );
    }

    #[test]
    fn 일시정지_중에는_새_전송이_시작되지_않는다() {
        // plan Edge Case — 멈춘 동안 등록한 것은 대기에만 쌓인다
        let (mut queue, ids) = queue_with(2, site(1));
        queue.set_paused(true);
        assert!(queue.is_paused());
        assert!(queue.next_for(site(1), 2).is_empty());

        queue.enqueue(
            site(1),
            TransferDirection::Upload,
            PathBuf::from(r"C:\c"),
            RemotePath::new("/c"),
            1,
        );
        assert_eq!(queue.len(), 3);

        queue.set_paused(false);
        assert_eq!(queue.next_for(site(1), 1), vec![ids[0]]);
    }

    #[test]
    fn 연결이_끊기면_진행_중이던_것이_대기로_돌아간다() {
        // plan Edge Case — 서버가 거부한 것이 아니라 우리 쪽 연결이 사라진 것이다
        let (mut queue, ids) = queue_with(2, site(1));
        queue.update(
            ids[0],
            TransferState::Active {
                sent: 50,
                speed: 10,
            },
        );
        queue.requeue_site(site(1));
        assert_eq!(queue.get(ids[0]).expect("항목").state, TransferState::Wait);
        // 실패로 남지 않아 다시 걸 것이 없다
        assert_eq!(queue.count(QueueFilter::Error), 0);
    }

    #[test]
    fn 끝난_것만_치우고_실패는_남긴다() {
        let (mut queue, ids) = queue_with(3, site(1));
        queue.update(ids[0], TransferState::Done);
        queue.update(
            ids[1],
            TransferState::Error {
                message: "550".to_owned(),
            },
        );
        queue.clear_done();
        assert_eq!(queue.len(), 2, "실패와 대기는 남는다");
        assert_eq!(queue.count(QueueFilter::Done), 0);
        assert_eq!(queue.count(QueueFilter::Error), 1);
    }

    #[test]
    fn 크기를_모르면_진행률이_없다() {
        // plan Edge Case — 0으로 나눌 수 없고, 0%로 적으면 시작도 안 한 것처럼 보인다
        let mut queue = TransferQueue::new();
        let id = queue.enqueue(
            site(1),
            TransferDirection::Download,
            PathBuf::from(r"C:\x"),
            RemotePath::new("/x"),
            0,
        );
        assert_eq!(queue.get(id).expect("항목").progress(), None);
        // 크기를 아는 항목이 하나도 없으면 전체 진행률도 없다
        assert_eq!(queue.overall_progress(), None);
    }

    #[test]
    fn 전체_진행률은_크기를_아는_항목만_센다() {
        let (mut queue, ids) = queue_with(2, site(1));
        queue.update(ids[0], TransferState::Done);
        queue.update(
            ids[1],
            TransferState::Active {
                sent: 500,
                speed: 1,
            },
        );
        // (1000 + 500) / 2000
        assert_eq!(queue.overall_progress(), Some(0.75));
        assert_eq!(queue.get(ids[1]).expect("항목").progress(), Some(0.5));
        assert_eq!(queue.get(ids[0]).expect("항목").progress(), Some(1.0));
    }

    #[test]
    fn 같은_파일을_두_번_넣어도_막지_않는다() {
        // plan Edge Case — 덮어쓰기 확인은 쓰기 직전에 실행기가 한다.
        // 여기서 막으면 사용자가 일부러 다시 받는 것도 막힌다
        let mut queue = TransferQueue::new();
        let first = queue.enqueue(
            site(1),
            TransferDirection::Download,
            PathBuf::from(r"C:\same.bin"),
            RemotePath::new("/same.bin"),
            10,
        );
        let second = queue.enqueue(
            site(1),
            TransferDirection::Download,
            PathBuf::from(r"C:\same.bin"),
            RemotePath::new("/same.bin"),
            10,
        );
        assert_ne!(first, second, "번호는 따로 발급된다");
        assert_eq!(queue.len(), 2);
    }

    #[test]
    fn 빈_큐도_말이_되는_값을_준다() {
        let queue = TransferQueue::new();
        assert!(queue.is_empty());
        assert_eq!(queue.count(QueueFilter::All), 0);
        assert_eq!(queue.overall_progress(), None);
        assert!(queue.counts_by_site(QueueFilter::All).is_empty());
        let summary = queue.summary();
        assert_eq!(summary.pending, 0);
        assert_eq!(summary.eta_secs, None);
        assert!(queue.next_for(site(1), 3).is_empty());
    }

    #[test]
    fn 만_건에서도_요약과_필터가_한_번씩만_훑는다() {
        // Acceptance ⑥ — 화면이 매 프레임 부르므로 항목마다 다시 훑으면 제곱이 된다.
        // 계산량 자체는 재기 어려워, **한 번 훑어 나오는 값**과 일치하는지로 고정한다
        let (mut queue, ids) = queue_with(10_000, site(1));
        for id in ids.iter().take(2_000) {
            queue.update(*id, TransferState::Done);
        }
        let summary = queue.summary();
        assert_eq!(summary.pending, 8_000);
        assert_eq!(queue.count(QueueFilter::Done), 2_000);
        assert_eq!(queue.filter(QueueFilter::All).len(), 10_000);
        assert_eq!(queue.overall_progress(), Some(0.2));
    }

    #[test]
    fn 전부_실패해도_요약이_성립한다() {
        // plan Edge Case
        let (mut queue, ids) = queue_with(3, site(1));
        for id in &ids {
            queue.update(
                *id,
                TransferState::Error {
                    message: "550 권한 거부".to_owned(),
                },
            );
        }
        let summary = queue.summary();
        assert_eq!(summary.pending, 0);
        assert_eq!(summary.speed, 0);
        assert_eq!(summary.eta_secs, None);
        assert_eq!(queue.count(QueueFilter::Error), 3);
        // 실패는 보낸 것이 없으니 전체 진행률은 0이다(막대가 비어 보인다)
        assert_eq!(queue.overall_progress(), Some(0.0));
    }
}
