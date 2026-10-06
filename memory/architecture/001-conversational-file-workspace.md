# 일반 파일·폴더의 앱 소유 대화 작업 공간

status: SUPERSEDED
superseded-by: [[011-conversational-trash-consent]] — 2026-10-05 사용자 요청으로 버튼 전용/5분 TTL을 단일 인간 결정·선택적 native 확인 생략·무시간 일회용 계획으로 변경. 앱 소유 범위/프로토콜/상한은 계승.
date: 2026-10-04
source: codex
tags: conversational-files, app-owned-tools, one-shot-plan, bounded-search, token-privacy
supersedes: #empty-folder-tools (일반 파일 관리 불가라는 구현 범위/검증 상태만; 빈 폴더 기능은 유지)
evidence: conversations/2026-10-04-codex-file-management.md#현재-사용자-요청 (사용자 턴 시각 미확인), #1539-kst--최종-설치-검증
alternatives: CLI에게 임의 경로/셸/삭제 권한 부여 — 앱 검증·OS Trash·최종 확인 경계가 사라져 채택하지 않음; 별도 대화로 계속 폴더 재선택 — 같은 대화에서 작업을 끝내려는 요구를 충족하지 못함.
depends-on: none — 기존 번호 없는 역할 분리/빈 폴더 계획은 memory/architecture.md의 제품 방향 항목을 이어받음.
sources: docs/qa/2026-10-04-conversational-files.md; apps/desktop/README.md
files: apps/desktop/src-tauri/src/assistant_files.rs; apps/desktop/src-tauri/src/assistant_tools.rs; apps/desktop/src-tauri/src/assistant_provider.rs; crates/bloomsweepy-core/src/local_search.rs; apps/desktop/src/views/AssistantView.tsx
reopen-when: 새로운 파일 작업(이름 변경/일반 이동/생성)을 연결하거나 범위/승인/공급자 프로토콜이 달라질 때; 상한이 반복적으로 정상 과업을 차단할 때.
last_verified: 2026-10-04

`files` strict JSON은 scan/largest/search/review_named/browse/parent/page/select/review만 요청한다. 모델은 실제 완료를 결정하거나 최종 승인·삭제·임의 경로·셸을 호출하지 못한다. 현재 모델의 ‘CLI 읽기 전용’을 앱 도구 불가로 혼동하지 않도록 가장 높은 프로토콜에 역할을 명시한다. 읽기 전용 큰 항목 발견과 지도 공유는 [003](003-conversational-storage-map.md)을 따른다.

작업 공간은 로컬 세션 루트에 갇히며 200 검색 결과/24개 페이지/100선택/16세션, 메타데이터만 보존한다. 자동 모델 문맥은 제한된 이름·크기·ID와 대화이며 전체 경로/본문은 자동 추가하지 않는다. 사용자 입력의 경로나 내용까지 익명화하는 것은 아니다. 토큰 절감률은 미측정이다.

최종5분 일회용 계획은 revision/선택/신원을 재검증하고 main WebView 버튼으로만 소비한 뒤 기존 journal/OS Trash/부분 실패를 실행한다. 파일 및 내용 있는 폴더를 지원하며 빈 폴더 도구도 유지한다. 일반 이동·이름 변경·폴더 생성·영구 삭제는 현재 프로토콜 밖이다.

검증: Rust273/frontend43, 실제 Codex2요청 계약, 합성16 B native Trash 통과. 설치앱에서도 기존 대화의 실제 promo-video 요청→최종 카드 성공. 사용자 폴더 이동은 하지 않았고 Windows/장시간 자원 검증은 남음.
