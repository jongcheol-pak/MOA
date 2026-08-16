# UI 문구·디자인 검토 반영 (2026-08-16)

## 목표
2026-08-16 문구·디자인 검토에서 나온 지적을 전부 반영한다. 세 갈래다 —
**① 동작과 어긋나는 문구**(사용자를 오도한다) **② 일반 사용자 이해도** **③ 화면 설계**(안전장치·빈 상태·어포던스).

## 범위
- `src/i18n/mod.rs` 문구 카탈로그와 동적 함수
- `src/ui/` 화면 계층 — sidebar · dock · queue_panel · panel · remote_states · site_manager · widgets · status_bar
- `src/remote/types.rs`·`connection.rs`·`src/panel/tabs.rs` — 실패 사유 분기에 필요한 실패 종류 전달
- 범위 밖: 팔레트 자체 변경, 원본 디자인 HTML, 레이아웃 치수(도크 탭 순서 제외)

## 사전 승인 항목 (일괄 승인 대상 — 사용자 "모두 수정")
- `ConnPhase::Failed`·`TabPhase::Error`에 실패 종류 필드 추가 (crate 내부 구조 변경, 직렬화 대상 아님)
- `i18n::dynamic::remote_op_failed`·`op_chmod` 제거 후 작업별 실패 문장 4개로 대체
- 도크 탭 순서 변경 (원본 디자인 HTML에서 벗어남)
- 전송 큐 크기 표기 단위 확장 (원본은 KB 고정)

## 작업 단계

- [x] **T1 — 문구 오류 정리 (조사·맞춤법·문장)**
  - `remote_hint_tail` 한국어 앞 공백 제거, `remote_error_slash` 조사 붙임
  - `site_charset_warning` 이중피동 제거
  - `err_not_found`·`err_unsupported`·`create_failed`·`name_decode_failed`에서 `을(를)` 병기 제거
  - 검증: `cargo test i18n`

- [x] **T2 — 작업 실패 문장 재구성**
  - `remote_op_failed`(명사+실패) 제거 → `op_mkdir_failed`·`op_delete_failed`·`op_rename_failed`·`op_chmod_failed`
  - `app.rs::op_label` → `op_failure_message`
  - 검증: `cargo test`

- [x] **T3 — 연결 실패 사유 분기**
  - `RemoteError::failure_kind()` + `FailureKind{Connect,Auth,HostKey,Other}`
  - `ConnPhase::Failed`·`TabPhase::Error`가 종류를 함께 나른다
  - `failure_reason`이 종류별 안내를 고른다 (암호화 안내는 연결 실패에만)
  - 검증: `cargo test remote`, `cargo test ui`

- [x] **T4 — 사이드바 `삭제` → 사이드바에서 숨기기**
  - 라벨 교체, 파괴색(`CLOSE_HOT`) hover 제거, 숨긴 뒤 토스트로 되돌리는 길 안내
  - 검증: `cargo test sidebar`

- [x] **T5 — 사이트 관리자 삭제 확인 대화**
  - `ui::dialog::show` 셸로 확인 대화, 워크스페이스 삭제 대화와 같은 구성
  - 검증: `cargo test site_manager`

- [x] **T6 — 툴팁·어포던스**
  - 도크 아이콘 4종·사이드바 `+`·사이트 관리자 전송 모드/암호화에 툴팁
  - `서버 로그 보기` hover 색·손가락 커서
  - 실패 화면 `재시도`를 주 버튼으로 (하단 버튼 줄과 같은 굵게 표현)
  - 검증: `cargo build` + 사용자 화면 확인

- [x] **T7 — 빈 상태 세 곳**
  - 사이드바(사이트 0개) · 빈 폴더 · 빈 전송 큐
  - 검증: `cargo test`

- [x] **T8 — 시각 정리**
  - 읽는 문구 자리의 `TEXT_DIM` → `TEXT_MUTED` (대비 3.1:1 → 5.9:1)
  - `design_button` 눌림 상태를 hover와 구분
  - 도크 탭 순서 `전송 큐·성공·실패 | 서버 로그` + 구분선
  - `format_size` MB/GB 확장
  - 검증: `cargo test queue`, 사용자 화면 확인

- [x] **T9 — 전체 검증**: `cargo fmt` · `cargo clippy --all-targets -- -D warnings` · `cargo build` · `cargo test`

- [x] **T10 — 문서 갱신**: README의 해당 화면 설명

## 승인 필요 항목 (이번 회차에서 하지 않음)
- 타이틀바 설정 메뉴의 비활성 항목 4개 제거 여부 — 채울 계획이 있는지는 사용자만 안다
- `기본(E)` 등 접근 키 표기 — **확인 결과 수정 불필요**로 판정(아래 참조)

## 검증 결과

- `cargo fmt --check` 통과 · `cargo clippy --all-targets -- -D warnings` 경고 0
- `cargo test` **736 + 통합 8 전부 통과** (lib 3회·전체 3회 반복 실행에서 실패 0)
  - 중간 한 회차에 1건이 실패로 찍혔으나 **이후 6회 반복에서 재현되지 않았고** 이름을 잡지 못했다.
    같은 시각에 clippy가 함께 돌던 부하 상황이라 시간에 민감한 원격 시험으로 보인다
- `cargo build --release` 통과
- **화면에 그려진 결과는 `⏳ 미확인`** — 데스크톱 GUI라 자동 캡처가 되지 않는다(과거 시도 기록:
  `2026-08-15-dialog-style.md`). 아래 항목은 사용자 화면 확인이 필요하다:
  - 사이트 삭제 확인이 사이트 관리자 **위에** 뜨고 그동안 뒤 대화가 눌리지 않는가
    (egui 중첩 모달의 입력 차단은 다음 프레임부터라, 그 한 프레임을 코드로 따로 막아 두었다)
  - 도크 탭 구분선 위치·빈 상태 세 곳의 자리·`TEXT_MUTED`로 올린 문구의 실제 대비
  - 실패 화면 `재시도`의 굵기가 하단 버튼 줄과 같아 보이는가

### 하지 않은 것 — 암호화·8진수 툴팁
전송 모드에는 툴팁을 달았지만 **암호화 드롭다운과 권한 8진수 칸에는 달지 않았다.**
암호화는 선택지 문구 자체가 설명이고(`TLS를 통한 명시적 FTP 필요` 등 넷 다), 8진수는 바로 옆에
읽기·쓰기·실행 체크박스 아홉이 같은 값을 보여 준다 — 툴팁을 더하면 같은 설명이 두 벌이 된다.

### 접근 키 `(E)` 판정
`site_mode_default => "기본(E)"`의 `E`는 FileZilla가 `D&efault`에 배정한 것과 같다. 같은 대화에
`삭제(D)`가 이미 있어 `D`는 쓸 수 없다. 접근 키 처리 자체가 아직 구현되지 않아(Alt 조합 처리 없음)
표기는 현재 장식이지만, 바꾸면 나중에 구현할 때 배정이 어긋난다 — **그대로 둔다.**
