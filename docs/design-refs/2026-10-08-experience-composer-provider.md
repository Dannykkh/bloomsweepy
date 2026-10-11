# Experience Contract: Composer CLI selector

## Source Mode

Mode: product-derived. 기존 compact-composer의 국소 delta. 사용자 첨부는 현재 앱의
중복 모델 진입점 증거이며 새로운 브랜드 벤치마크가 아니다. 사용자 요청: 앞쪽 버튼을
Codex·Claude Code·AGY·Grok 선택으로 바꾸고 모델·추론을 해당 CLI에 맞춘다.

## Product Facts

| Claim | Source | Captured at | Freshness/status | Allowed presentation |
|---|---|---|---|---|
| 공급자별 모델/모델별 강도 선호 공유 | useAssistantModelPreference.ts | 2026-10-08 | current | 기존 hook 재사용 |
| 목록·강도는 공급자 상태 metadata | AssistantComposerControls.tsx | 2026-10-08 | current | 임의 선택지 추가 금지 |

## Benchmark Sources

해당 없음 — product-derived. 사용자 첨부 current-state crop에는 모바일·상태 전환 증거 없음.

## Page Goal

입력창에서 CLI를 바꾸고 그 CLI의 모델·추론을 선택한다. 모델 진입점의 중복을 없앤다.

## Audience and Tasks

저자원 Mac 사용자. 초안 → CLI 선택 → 모델·강도 확인/변경 → 질문 전송.
공급자 전환은 초안을 지우거나 전송하지 않으며 이전 선택은 공급자별로 복원한다.

## System Roles

NOT APPLICABLE: single role. CLI 선택은 권한 부여가 아니다.

## Header and Navigation

헤더의 중복 공급자 select를 제거하고 기존 연결 상태·재확인만 유지한다.
입력창 왼쪽 CLI 버튼 → 모델·강도 통합 버튼 → 우측 전송/취소.

## Core Message

첫 버튼은 실행할 CLI, 둘째는 그 CLI의 모델·강도. 설명 문장은 정상 입력부에 추가하지 않는다.

## Content Integrity

| Content item | Classification | Evidence | Presentation rule |
|---|---|---|---|
| CLI 목록/모델/강도 | verified | host metadata | 해당 공급자의 상태와 지원 목록만 사용 |
| QA 모델 | prototype | synthetic fixture | 실제 AI 실행·접근권 증거로 사용 금지 |

## Section Order

기존 대화/진행 상태 → textarea → CLI/모델·강도 toolbar → 전송. 팝업은 위쪽에 열린다.

## CTA Strategy

Primary: 질문 보내기/AI 응답 취소. Secondary: 대화 상대 선택, 모델·추론 선택.
선택 버튼은 type=button. CLI 선택 완료와 Escape는 질문을 전송하지 않는다.

## Trust Strategy

미설치 CLI도 목록에 상태와 함께 표시하지만 기존 ready 검사로 전송을 막는다.
공급자 전환 시 예전 팝업을 닫고 저장한 모델/강도를 복원한다. busy/checking은 변경을 잠근다.
stale/저장 실패/미지원/기본값 구분 및 읽기 전용 참고 목록은 기존 계약을 유지한다.

## Asset Provenance

해당 없음 — 프로젝트 내부 Pretendard/Lucide/CSS만 사용. 새 외부 자산·dependency 없음.

## Desktop Structure

기존 1280px/760px 작업면과 compact composer 유지. 왼쪽 CLI는 짧은 라벨,
모델·강도 버튼은 남은 폭, send는44px 고정. 팝업 최대360px.

## Mobile Transformations

| Desktop element | Operation | Mobile result | Reason |
|---|---|---|---|
| CLI 버튼 | retain | 44px hit area·CLI 정체성 유지 | 실행 주체 혼동 방지 |
| 모델·강도 라벨 | compress | 모델 ellipsis·강도 유지·전체 접근성 이름 | 전송 영역 보존 |
| popup | compress | 컨테이너 안으로 제한 | 좁은 pane에서 선택 가능 |
| 헤더 select | remove | 상태·재확인만 | 중복 설정 제거 |

## States

| State | Trigger | User sees | Available action | Recovery |
|---|---|---|---|---|
| loading | CLI 조회 | 기존 상태·비활성 선택 | 대기 | 재확인 |
| empty | CLI 모델 없음 | 기존 상태별 안내 | 다른 CLI/기본값/설치 안내 | 실제 목록 복구 |
| error | stale·저장 실패 | 기존 경고·저장값 | 명시 reset/재선택 | 전송 조건 복구 |
| success | CLI 선택 | 해당 모델·강도 복원 | 모델/강도 선택·전송 | 재선택 |
| working | 요청·세션 작업 | 선택 비활성·실제 진행/취소 | 취소 | 종료 후 재선택 |

## Performance Budget

새 timer/blur/이미지/폰트/의존성0. 기존 bounded popup/list/event cleanup 재사용.

## Accessibility Contract

44px 버튼, labelled nonmodal dialog/listbox, 방향키/Home/End 이동, Escape 닫기와 trigger focus 복귀.
busy/provider/model 변경 시 이전 팝업 종료. IME guard·초안 보존·reduced-motion 유지.

## Adopt

기존 compact-composer의 위쪽 popup과 모델 부제 → 모델 목록 진입.

## Adapt

중복 모델 trigger를 CLI trigger로 대체, 나머지는 모델·강도 통합 진입점.

## Avoid

세 번째 중복 모델 버튼, 새 저장소, 가짜 최신 모델, 자동 CLI 설치, 권한 확대.

## Prompt Contract

GOAL — 입력창에서 CLI와 해당 모델·강도 선택.
AUDIENCE — 단일 사용자 저자원 Mac.
TASK — CLI 전환·모델/강도 선택·전송.
FLOW — CLI→기존 선호→해당 catalog→모델/강도→기존 요청.
HEADER — 중복 select 제거, 상태·재확인 유지.
MESSAGE — CLI와 모델/강도를 구분.
FACTS — 실제 metadata와 기존 선호.
CONTENT_INTEGRITY — fixture는 합성, 참고 목록은 읽기 전용.
SECTION_ORDER — 입력→두 trigger→send, 위쪽 popup.
CTA — send/cancel primary, 선택은 type=button.
TRUST — stale·미설치·저장 실패·지원 근거 보존.
ASSETS — 프로젝트 내부만.
LAYOUT — CLI/통합 trigger와44pxsend.
RESPONSIVE — 모델 압축·CLI 유지·bounded popup.
STATES — loading/empty/error/success/working.
PERFORMANCE — 새 의존성·상시 timer 없음.
ACCESSIBILITY — 키보드/포커스/라벨/44px/reduced-motion.
PRESERVE — Settings 공유·선호·권한·진행/취소·기존 재질.
EXCLUDE — AI 실제 전송·파일 작업·설치·커밋·푸시.
SUCCESS — CLI별 복원·목록/강도 갱신·모바일 겹침/오발송0.

## Success Checks

- CLI 순환 후 각각 저장한 모델/강도 복원.
- 미지원 모델과 Ollama는 임의 추론 선택 없이 모델 변경 가능.
- busy와 provider 전환 시 이전 popup이 남지 않음.
- desktop/390px에서 선택·전송 버튼 겹침 없음.
- 실제 설치본·AI 응답 검증과 합성 UI 검증을 구분.

#tags: composer-provider, cli선택, 모델선택, 추론강도
