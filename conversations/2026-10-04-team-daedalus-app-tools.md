# 앱 기능 정본과 LLM 조사 루프

date: 2026-10-04
source: codex

## 현재 요청

사용자는 내장 AI/MCP에 문서 검색·앱 관리·성능 조회가 연결되지 않았음을 확인한 뒤 “모두 메꿔볼까?”라고 요청했다. 조회 실행은 앱, 조회 선택과 결과 검토·분석·추가 조사는 LLM이 사람처럼 담당해야 한다고 보완했다. 현재 사용자 턴 시각은 미확인이다.

## Phase 1 decision

공통 typed 기능 정본 + 기존 앱 서비스 + bounded 조사 루프를 선택한다. 프롬프트 설명만 보강하면 호출 누락이 남고, CLI 자체 파일/셸 조사로 대체하면 사용자 앱 데이터 요구와 안전 경계에 어긋난다. 현재 명시 요청·보완 지시를 승인 범위로 사용하고 삭제/권한 자동 변경/릴리스로 확대하지 않는다. [대안 점수와 설계](../docs/plan/app-tool-integration/plan.md)

## Phase 2/3 milestone

조사 루프 도면과 독립 파일 소유권을 고정했다. 조회/준비 helper를 공유하되 final approve/execute는 모델 enum에서 제외한다. 문서 일치 문맥의 AI 공개에는 기존 documents 권한을 사용한다. 프로세스 snapshot/앱 inventory 수명과 UI의 자동 refresh 함정을 확인했으며 준비된 검토를 중복 생성하지 않는다.

## 상태

조사/설계 및 공통 기능·조사 루프·결과/준비된 검토 UI 구현 완료. control23/MCP12/desktop191/frontend43 검사, Clippy, TypeScript/Vite 빌드 통과. CUA 합성 결과/최종 분석 유지와 만료된 검토 무실행 확인. 실제 설치앱 Codex 흐름·전체 integration 회귀·Windows·메모리 soak는 아직 미완료이며 릴리스·커밋·설치를 하지 않았다.

## 현재 질문과 검증 경계

사용자는 “나의 개념이 틀린가?”라고 물었다. 개념은 맞다. LLM이 앱 조회를 선택하고 앱이 실제 목록을 수집·검색·측정해 반환하며 LLM이 분석/추가 조회한다. 앱 엔진을 CLI 자체 탐색으로 대체하는 것이 아니다. 다만 이름·크기만으로 삭제 안전을 확정하지 않으며 정확한 대상 검증과 사용자 최종 확인은 별도다.

전체 회귀 중 디스크 여유2048MiB 보호가 실제로 발동했다. 이번 빌드가 생성한 단일579,497,608바이트 정적 라이브러리만 제거하여 공간을 회복했으며 보호 규칙과 사용자 파일은 유지했다. 최종 결과/남은 작업은 QA와 핸드오프에 기록한다.

WorkPM 구현 위임은 새 thread cap 때문에 같은 관련 조사 worker를 재개해 수행했고 파일 소유권은 유지했다. 모든 worker 소유권은 반환됐다.

#tags: app-capabilities app-owned-results llm-investigation mcp workpm arch:004
