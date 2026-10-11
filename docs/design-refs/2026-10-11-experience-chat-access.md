# Experience Contract: Chat access modes

## Source Mode

Mode: product-derived, approved compact-composer delta. 사용자 요청은 채팅에서 전체 사용
접근을 선택하고 로컬 드라이브를 검색·관리하는 것이다. 기존 브랜드/작업면은 유지한다.

## Product Facts

| Claim | Source | Captured at | Freshness/status | Allowed presentation |
|---|---|---|---|---|
| 권한 수명 session/remember | permission_settings.rs | 2026-10-11 | current | 기존 저장·복원 계약 재사용 |
| 정확한 이름의 휴지통 요청만 확인 생략 | assistantConfirmation.ts, control_server.rs | 2026-10-11 | current | 모호한 요청은 추가 확인 |
| OS 전체 디스크 접근과 앱 설정은 별개 | 현재 native 권한 경계 | 2026-10-11 | current | OS 권한 부여라고 표현 금지 |

## Benchmark Sources

해당 없음 — 기존 승인된 composer의 국소 변경. 새 외부 벤치마크를 복제하지 않는다.

## Page Goal

질문 전 현재 권한을 식별하고 조회만/확인 후 실행/전체 접근 중 선택한다.
앱의 저장 성공이 표시값을 결정하며, 모델 응답은 권한을 변경하지 못한다.

## Audience and Tasks

저자원 Mac의 단일 사용자. 초안 작성 → 권한 선택 → 질문 → 실제 진행·결과.
폴더 대화를 먼저 열고, 전체 접근에서 로컬 볼륨 목록/유한 이름 검색 → 정확한 대상 열기 → 기존 단일 루트 검토.
권한 변경은 초안이나 공급자·모델·추론 선택을 바꾸지 않는다.

## System Roles

NOT APPLICABLE: single role. 세 모드는 사용자 역할이 아니라 사용자가 선택하는 실행 정책이다.

## Header and Navigation

기존 대상·연결 팝업 유지. 입력창 toolbar: CLI → 모델·추론 → 권한 → 전송/취소.
설정과 대화 연결 팝업의 같은 ControlStatusPanel에서 동일 권한을 선택할 수 있다.

## Core Message

조회만은 실행 준비/실행 금지, 확인 후 실행은 기존 범위에서 검토 후 확인,
전체 접근은 로컬 볼륨 검색과 명확히 지시한 휴지통 이동의 추가 확인 생략이다.
영구삭제·OS 권한·외부 MCP·문서 내용 공개를 함께 켜는 모드가 아니다.

## Content Integrity

| Content item | Classification | Evidence | Presentation rule |
|---|---|---|---|
| 현재 모드 | verified | Rust ControlStatus | 저장 응답 후 변경, 실패 시 이전 값 |
| 볼륨/검색 결과 | verified | bounded app dispatcher | 부분 결과와 읽지 못한 항목 표시 |
| QA 결과 | prototype | synthetic fixture | 실제 AI/디스크 검증과 구분 |

## Section Order

기존 대화·진행 → 전체폭 textarea → CLI/모델·추론/권한 toolbar → 전송.
권한 팝업은 위쪽, 제목 → 세 선택지와 짧은 범위 설명 → 권한 한계 순서다.

## CTA Strategy

Primary: 질문 보내기/AI 응답 취소. Secondary: 권한 선택, 결과의 대상 열기.
전체 접근 선택 자체가 명시적 opt-in이며 별도 중복 확인창을 추가하지 않는다.
권한 변경 중 전송·정리 확인을 잠그며 실패는 같은 자리에서 재시도한다.

## Trust Strategy

전체 접근 설명은 선택 직전에 팝업 안에서 보여준다. 영구삭제/OS 권한/외부 연결/내용 공개
경계는 간결하게 명시한다. 앱이 저장하지 못한 값은 활성 권한으로 보여주지 않는다.
조회만은 UI 비활성화뿐 아니라 Rust dispatcher/direct 확인/자동 실행에서도 막는다.
권한 축소 시 이전 범위·검토를 재사용하지 않는다. 기존 확인 생략 flag로 전체 접근을 이관하지 않는다.

## Asset Provenance

해당 없음 — 프로젝트 내부 Pretendard/Lucide/CSS만 사용. 새 자산/의존성 없음.

## Desktop Structure

기존 bounded chat pane과 고정 하단 composer 유지. 권한 trigger44px, popup 최대360px.
모델/권한 묶음은 남은 폭 사용, 전송44px 고정. 권한 설명을 입력창 아래에 상시 추가하지 않는다.

## Mobile Transformations

| Desktop element | Operation | Mobile result | Reason |
|---|---|---|---|
| CLI/권한 | retain | 44px 선택 영역과 현재 정책 유지 | 실행 주체·권한 식별 |
| 모델 라벨 | compress | ellipsis, 접근성 이름에는 전체 값 | 전송/권한 우선 |
| toolbar | reorder | 좁으면 권한/전송 행 분리 | 긴 다국어 라벨 겹침 방지 |
| popup | compress | composer 전체폭 안으로 제한 | 작은 trigger 폭에 종속 금지 |
| 상세 권한 설명 | collapse | 팝업/설정 안에서만 표시 | 대화 공간 보존 |

## States

| State | Trigger | User sees | Available action | Recovery |
|---|---|---|---|---|
| loading | 저장 중 | 이전 모드·비활성 trigger | 대기 | 저장 결과 적용 |
| empty | 모드 필드 없는 이전 host | 확인 후 실행 | host 재시작/업데이트 | 전체 접근 추정 금지 |
| error | 저장 실패 | 이전 모드와 오류 | 재선택 | 성공 응답 적용 |
| success | 저장 완료 | 선택된 현재 모드 | 질문/재선택 | 자유롭게 축소 |
| working | 질문/실행 중 | mode/모델 변경 잠금·실제 진행 | 기존 취소 | 종료 후 변경 |
| permission | 조회만/OS 거부/축소 | 실행 불가/부분 결과 | 권한 선택/OS 설정 | 새 조회·새 검토 |

## Performance Budget

새 타이머·블러·네트워크·폰트·이미지·의존성0. 전체 검색은 앱의 유한 metadata walker,
결과 페이지와 IPC 상한을 유지한다. 대화에 전체 경로/파일 내용을 자동 추가하지 않는다.

## Accessibility Contract

44px trigger, labelled dialog/listbox, 방향키/Home/End, Escape와 focus 복귀.
상태는 텍스트+선택 표시로 구분하고 색만 사용하지 않는다. 저장 오류 role=alert.
앱이 지원하는 다크 테마의390px/1280px·키보드를 실제 렌더에서 확인한다.
라이트 테마는 현재 앱 미지원으로 N/A. reduced-motion은 CSS 정적 검증과 실제 OS 설정 검증을 구분한다.

## Adopt

기존 위쪽 compact popup, 단일 textarea, provider별 선호와 실제 진행 표시.

## Adapt

세 번째 trigger는 모델 중복이 아닌 권한 정책. 기존 permissionLifetime에 같은 모드 저장.

## Avoid

권한 자동 부여, 모델이 지정한 권한값, 가짜 전체 검색 완료, 영구삭제 확대,
클라우드 hydration/네트워크 볼륨 검색, 실제 사용자 파일을 이용한 파괴적 QA.

## Prompt Contract

GOAL — 채팅 권한을 명확히 선택하고 앱이 강제한다.
AUDIENCE — 저자원 Mac의 단일 사용자.
TASK — 모드 선택→질문→앱 조회→검토/실행.
FLOW — main-window IPC→저장 성공→shared ControlStatus→Rust 검사.
HEADER — 기존 대상/연결 팝업 유지.
MESSAGE — 조회만/확인 후 실행/전체 접근의 차이.
FACTS — 저장된 권한과 실제 유한 조회만 표시.
CONTENT_INTEGRITY — 합성 fixture는 실제 OS/CLI 검증과 구분.
SECTION_ORDER — textarea→CLI/모델/권한→send, 위쪽 popup.
CTA — send/cancel primary, mode는 secondary.
TRUST — 범위/내용/OS/MCP 권한 분리·권한 축소 fail-closed.
ASSETS — 내부 자산만.
LAYOUT — 기존 composer에44px trigger, max360px popup.
RESPONSIVE — 모델 압축·필요시 행 분리·bounded popup.
STATES — loading/empty/error/success/working/permission.
PERFORMANCE — bounded metadata/result·새 의존성0.
ACCESSIBILITY — listbox keyboard/focus/라벨/44px/reduced-motion.
PRESERVE — 초안·모델·추론·진행·취소·기존 외부MCP 계약.
EXCLUDE — 사용자파일 삭제·실제 권한 변경·설치·커밋·푸시.
SUCCESS — 저장/복원/축소·조회만 차단·전체 로컬 조회·모바일 겹침0.

## Success Checks

- 실패한 저장을 전체 접근으로 표시하지 않는다.
- 조회만에서 review/confirm/auto 경로가 모두 차단된다.
- 전체 접근은 명시 선택해야 하며 OS/MCP/문서 공개는 그대로다.
- bounded 전체 검색은 부분 결과를 알리고 동명이인은 자동 삭제하지 않는다.
- 입력창과 전송/권한 버튼은390px/1280px 다크 테마에서 식별 가능하다. 라이트는 미지원 N/A.

#tags: chat-access, 권한모드, 전체접근, bounded-search
