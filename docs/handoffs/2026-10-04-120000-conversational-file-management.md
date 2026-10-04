# 대화형 일반 파일 관리 — 로컬 구현/설치 검증 완료

- Project: BroomSweepy
- Updated: 2026-10-04 15:53 KST
- Branch/base: main / 842d916
- Origin-source: session 01a06a3f-f2f6-72c0-9c0c-78a13e23b651, 현재 사용자 턴 시각 미확인.

## Origin

사용자가 실행앱 대화가 일반 폴더 삭제를 반복적으로 못 한다고 지적한 뒤 “그니까 못하는거잖아. 파일관리를 지금. 다 잘할수 있게 해줘.”라고 구현을 요청했다. 기존 v1.7.0 대화13개에서 실제 promo-video/14,914파일을 빈 폴더 전용 도구로 처리하지 못하는 상태를 확인했다.

## Current State Summary

일반 파일·내용 있는 폴더의 검색/용량 검사/하위 탐색/선택 변경/최종 휴지통 검토를 같은 대화에 연결했다. 최종 arm64 앱을 /Applications/BroomSweepy.app에 설치하고 실제 Codex→promo-video 검토 카드까지 확인했다. 사용자 폴더는 이동하지 않았다. 표시 버전1.7.0은 그대로이며 GitHub 다운로드·커밋·태그·푸시는 변경하지 않았다.

설치 SHA256: 61773657b0348631079143460761e41d3cd728a454600e87ee3b1f27148a9f9d.
원래 앱 백업: /private/tmp/broomsweepy-previous-app-obVm8L/BroomSweepy.app.
중간 개발본 백업: /private/tmp/broomsweepy-intermediate-app-6MFxPr/BroomSweepy.app.

## Feature/Flow/Decision Snapshot

### Feature Boundary

모델은 strict JSON 앱 작업만 요청하며 셸/임의 경로/최종 승인/직접 삭제를 호출하지 못한다. 앱은 세션 루트의 bounded metadata 작업 공간, 정확한 경로/포함 항목 카드,5분 일회용 계획, 실행 직전 신원 재검증과 기존 OS Trash/저널을 소유한다. 열기/위치 표시는 기존 안전 검사로 연결한다.

이름 변경·일반 이동·새 폴더 생성·영구 삭제는 지원 범위가 아니다. Windows reparse/junction은 fail-closed이고 실기 검증도 남아 있다. 클라우드/온라인 전용/시스템/범위 밖/다른 장치 보호는 유지한다.

### Composition Diagram

```mermaid
flowchart LR
 A[대화와 제한된 이름/통계] --> B[공급자 중립 JSON 요청]
 B --> C[Rust 세션 루트 검색/탐색/검토]
 C --> D[로컬 확인 카드]
 D --> E[사용자 체크와 최종 버튼]
 E --> F[일회용 계획 소비/신원 재검증]
 F --> G[OS Trash와 저널]
 G --> H[같은 대화의 실제 결과/부분 실패]
```

### Decision Records

- [001 일반 파일 대화 작업 공간](../../memory/architecture/001-conversational-file-workspace.md): 검색200/모델·UI24/선택100/상태16세션. CLI 읽기 전용과 앱 도구 가능 여부를 구분한다.
- [002 내부 링크 분리](../../memory/architecture/002-opaque-folder-symlinks.md): 실제 .bin/semver 차단을 관측해 POSIX 내부 링크만 불투명 snapshot에 포함. 원본은 읽거나 대상에 합산하지 않고 링크 수를 표시. 선택/부모 링크와 Windows junction은 계속 차단한다.

## Implemented Features

| 기능 | 위치 | 검증 |
|---|---|---|
| metadata 이름 검색·범위 검증 | crates/bloomsweepy-core/src/local_search.rs | 상한/취소/클라우드/링크 경계 |
| 세션 작업 공간·nonce 계획 | assistant_files.rs | stale ID/TTL/변경/선택/동명·부분 결과 |
| 일반 모델 도구·사실 분리 | assistant_tools.rs, assistant_provider.rs | 실제 Codex2요청 및 기존 대화 |
| 파일/폴더 Trash와 이력 | trash_actions.rs, action_recovery.rs | 합성 native Trash/저널 파싱 |
| POSIX 내부 링크 snapshot | crates/bloomsweepy-core/src/actions.rs | 외부·깨진·순환 링크/retargeting/원본 보존 |
| 검색/검토 카드·열기·위치·체크 | AssistantFileCard, AssistantView, App | React fixture + native AX/화면 |
| 4개 언어·현재 개발본 안내 | i18n, README4종/DesktopREADME/DESIGN | 키 parity/형식 |

## Important Context

- 실제15:31 요청은 내부 링크 차단으로 실패. 보완한 최종 설치본은15:39 같은 대화의 요청→검토 성공: 논리1.2 GB/파일14,914/폴더1,296/링크29.
- 사용자 폴더의 체크/최종 이동 버튼은 누르지 않음. 설치앱 요청→계획과 native Trash 합성 실행은 분리된 증거이며 사용자 삭제 E2E로 과장하지 않음.
- Rust273/frontend43 통과. 실제 Codex2요청 및 native Trash16 B는 opt-in ignored 시험을 별도 실행. Native 시험은 보완 전/후2회, 생성한 자료만 이동했고 외부 링크 원본 보존 확인.
- 빌드/형식/타입 및 ad-hoc deep/strict 서명 검사 PASS. JS chunk >500kB 경고 남음. Apple 공증이 아님.
- Clippy5개 스타일 경고(동등한 if/closure borrow 정리)를 소스에서 수정한 뒤 workspace lib Clippy와 assistant_files5개 회귀 PASS. 설치본은 스타일 정리 직전의 동일 기능 빌드이며, lint-only 정리 후 재패키징하지 않음.
- Windows, 장시간 전체 RSS/디스크 soak, 사용자 앱 제거/휴지통 비우기: NOT RUN.
- 이번에 만든 arm64 target 캐시 약800MB만 제거(재빌드 가능). 기존 debug 캐시 보존. 여유 디스크 일회 관측4.9 GiB. dev server/fixture 탭/viewport override 정리. 설치앱 실행 상태.
- 기존 docs/domain-dictionary.md 및 다수 미추적 자료/promo-video는 사용자 작업으로 보존. 전체 git add/cleanup 금지.

## Critical Files / Files Modified

새 파일: crates/bloomsweepy-core/src/local_search.rs; apps/desktop/src-tauri/src/assistant_files.rs; apps/desktop/src/components/AssistantFileCard.tsx/css; QA/인계/이번 대화 기록/architecture001·002·index.
연결 변경: crates/bloomsweepy-core/src/actions.rs, directory.rs, lib.rs; desktop assistant_provider/tools/sessions/lib/trash_actions/action_recovery/folder_actions; frontend types/bridge/App/AssistantView/DashboardView/StorageTreemapPanel/fixtures/i18n; README4종/DesktopREADME/DESIGN/MEMORY/legacy architecture.

정확한 결과/명령/설치 정보: [QA](../qa/2026-10-04-conversational-files.md).

## Session Memory Review

- 프로젝트 .git 실체로 루트 확정. MEMORY→기존 architecture 실제 본문 확인. doctor2026-09-29(30일 이내)이므로 반복 진단 없음.
- 재사용 결정001/002와 인덱스 추가. legacy 빈 폴더 범위와 내부 링크 거부 정책만 상호 SUPERSEDED 연결. 과거 기억 이관/백로그 정제 없음.
- evidence는 이번 세션 앱 관찰15:31/15:39 및 QA. 실제 session UUID 확인, 최초 사용자 턴 시각은 미확인.
- MEMORY의 conversational-files/folder-trash로001/002 재검색 및 근거 확인. 인덱스72줄/4701 B로 상한 이내.
- 대응 프로젝트 로컬 스킬/카탈로그 없음: candidate=no, target=none. 전역 스킬 수정/자동 생성/반복 비교 없음.
- validate_handoff.py PASS(required sections/secrets/tags), READY. 초기 두 축약 경로를 실제 core 경로로 수정. 검증기 점수는 산출물 형식 평가이지 기능 합격 점수가 아님.
- 별도 gotcha/learned는 architecture001/002와 중복이라 추가하지 않음. observations offset은 건드리지 않음.
- code map은2026-09-02 자동 산출물이라 새 위치는 직접 소스로 보완. Component map 없음.

## Immediate Next Steps

현재 요청의 로컬 수정·설치는 완료. 실제 일반 폴더 검토 성공과 최종 확인이 필요한 동작을 사용자에게 설명한다. 릴리스/커밋/푸시는 별도 요청 시 사용자 변경 범위를 분리한 뒤 진행한다. Windows/soak 또는 이름 변경·일반 이동 도구 확장은 미검증/미지원 범위를 명시하고 별도 구현/검증한다.

#tags: conversational-files, folder-trash, codex, macos, bounded-search, arch:001, arch:002, supersedes:#empty-folder-tools, supersedes:#nested-link-rejection
