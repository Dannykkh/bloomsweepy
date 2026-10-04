# 큰 항목 발견과 대화·용량지도의 단일 검사 결과

status: CURRENT
date: 2026-10-04
source: codex
tags: largest-items, shared-treemap, read-only-advice, bounded-snapshot, scan-generation
evidence: conversations/2026-10-04-codex-storage-map.md#현재-요청 (사용자 턴 시각 미확인)
alternatives: 채팅/지도 각각 재검사 — 비용과 결과 시점이 달라져 제외; 모든 탐색 폴더 보고서 영구 누적 — 제한된 Mac 메모리·디스크 환경에서 불필요해 제외; 크기/캐시 유사 이름으로 자동 삭제 판단 — 백업·필요성·재생성 가능성의 근거가 없어 제외.
depends-on: [[001-conversational-file-workspace]]
sources: apps/desktop/README.md; docs/qa/2026-10-04-conversational-storage-map.md
files: apps/desktop/src-tauri/src/assistant_files.rs; apps/desktop/src-tauri/src/assistant_tools.rs; apps/desktop/src/views/AssistantView.tsx; apps/desktop/src/App.tsx
reopen-when: 여러 폴더 지도의 영구 캐시/통합 트리를 요구하거나 재생성·백업을 검증할 새로운 정보원이 생길 때.
last_verified: 2026-10-04

“여기서 가장 큰 폴더/데이터, 삭제해도 돼?”는 `files/largest`의 읽기 전용 fresh scan이다. 직계 항목을 논리 바이트로 비교하고 폴더는 하위 합계임을 밝힌다. 크기·수정일·이름만으로 안전 삭제를 보장하지 않는다. 자동 선택/최종 계획은 없고 명시적 제거 요청·최종 앱 버튼 경계를 유지한다.

scan/largest/browse/parent는 bounded `StoredReports.directory` 스냅샷 한 개를 교체하며 작업 공간에는 generation과 카드용 bounded children만 보유한다. 지도 조회는 세션/revision/generation/root가 맞을 때만 반환한다. 이름 검색의 미측정 폴더는 지도 보고서로 위장하지 않는다. 일반 검사로 결과가 만료되면 재검사, 자동 동기화는 채팅 유지, 명시적 지도 버튼만 화면 이동한다. 전체 폴더 트리의 지속 누적은 아니다.

Rust 회귀275, frontend43, 실제 Codex 합성3요청 계약, 합성 UI 카드/지도 동일1,420,002,400 B·미선택·만료 안내·325/760px 검사 통과. 실제 설치 앱에서도 자연어 발견→하위 browse→삭제 판단 설명→동일 target 지도 및 범위 유지 재검사를 확인했다. 사용자 파일 삭제는 실행하지 않았다. [상세 QA](../../docs/qa/2026-10-04-conversational-storage-map.md)

대화 복귀 뒤 새 로컬 검사 메시지를 추가할 때 saved sequence와 pending 배열 index가 같은 React 행 키로 충돌했다. 두 prefix를 구분해 검사→지도→복귀→검색의 신규 경고0을 확인했다. 순번을 저장 신원과 혼용하지 않는다.
