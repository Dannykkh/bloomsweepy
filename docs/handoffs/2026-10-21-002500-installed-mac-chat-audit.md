# 설치된 Mac 앱·실제 채팅 검사 인계

Project: BroomSweepy
Branch: main
Date: 2026-10-21 00:25 KST
HEAD: 85620ca41ae0c5be82b5a9a36672186fa77640b9

## Origin

사용자: “맥교체하고 테스트 진행. 채팅창에서도 다 제어가 되는지 사용하기 편하게 되어있는지 확인”. 현재 응답으로 새 테스트 폴더의 remove-me.txt 한 파일만 휴지통 이동을 허용했다. 세션 UUID와 최초 턴 시각은 전달되지 않아 추측하지 않는다. 이 인계는 컨텍스트 압축에 따른 연속성 보존이며 하루 전체 대화를 정제한 것이 아니다.

## Current State Summary

최신 작업트리로 Mac ARM64 app-only release를 빌드·교체·실행했고 실제 Codex/GPT-6.1-Sol/중간 채팅 시험을 종료했다. 버전은 계속1.7.0 로컬 개발본이다. 공개 release/commit/push 없음. 사용자 데이터·원래 권한 선택은 유지했다. 앱은 테스트 workspace 대화16메시지·CLI 준비됨·작업 없음으로 실행 중이다.

기능 전체 완료 판정은 아니다. 실제 시험과 증거는 [QA 정본](../qa/2026-10-21-installed-chat-audit.md), 실제 화면8장은 [캡처 폴더](../qa/screenshots/2026-10-21-installed-chat)에 있다. 사용성 P2 두 건은 보고만 했으며 제품 코드는 고치지 않았다.

## Implemented Features

이번 턴 제품 기능 추가 없음. 새 파일 `apps/bloomsweepy-mcp/tests/installed-os-settings-flow.mjs`는 실제 설치 sidecar의 읽기 전용 discovery/OS 조회 계약 하네스다. ARM64 Node로 실행 exit0,12도구/27capabilities/format2·Mac5행·Windows unsupported0행·로컬 presentation 비노출·설정/권한 무변경 PASS. 기존 common-flow 하네스의 옛 capability 수는 변경하지 않았다.

검사 도구의 late fatal/종료 확인 결함은 최종 리뷰 후 최소 보강했다. 정상·late malformed·개행 없는 frame·후손 stdio 유지 합성4/4와 node --check PASS. main이 수정본으로 실제 설치 sidecar를 재실행해 exit0·같은12/27/5/unsupported0 계약을 재확인했다. 제품 설치본을 다시 바꿀 변경은 아니다.

실제 설치본에서 파일 큰 항목 조회, 같은 지도, Finder5값 조회, 성능/앱 목록, 자체 allocator 메모리 정리 실행, 승인 단일 파일 이동, 이름 검색이 통과했다. 문서 본문 검색은 정확한 공개 root 불일치로 차단됐으며 성공 경로 NOT RUN이다.

## Feature/Flow/Decision Snapshot

- 내장 채팅은 strict JSON→공통 dispatcher→실제 결과 분석이다. 외부 MCP wire 자체를 거치지는 않으므로 각각 검증했다.
- full_access에서 정확한 remove-me.txt 요청은 재검사 후 추가 확인 없이1개만 휴지통으로 이동됐다. 다른3개 SHA256이 보존됐다. OS 휴지통 복원/비우기 미실행.
- 메모리 정리는 별도 로컬 검토 모달에서 실행했으며 반환 가능 자체 메모리 없음으로 완료. 다른 앱/WebKit/CLI·CPU 정리가 아니다.
- Finder 쓰기/복원은 채팅에서 제공하지 않는 현재 계약. 조회 성공을 쓰기 가능으로 표현하지 않는다.
- 문서 내용 공개는 전체 접근과 별도이며 기존 토글은 현재 대화 root가 아니라 색인 root들을 함께 허용한다. 시험을 위해 다른 범위를 허용하지 않았다.

## Important Context

- 설치: `/Applications/BroomSweepy.app`, main SHA256 `d8a54253a5e15fbcc218ff0ffcc79ab00726c72d2f11b299c0302abe750ad069`, PID89925 관찰.3실행 파일 ARM64·fresh bundle과 cmp 동일, codesign deep/strict PASS.
- 최초 bundle resource seal 실패를 기존 entitlements로 MCP→worker→app ad-hoc 재서명해 복구했다. 공증·배포 DMG는 아님.
- 원래 앱 rollback: `/private/tmp/broomsweepy-installed-chat-audit-I9gHOi/Previous-BroomSweepy.app`. 삭제하지 말 것.
- fixture: `/private/tmp/broomsweepy-installed-chat-audit-I9gHOi/workspace`. remove-me.txt만 이동; large-report.txt/readme.md/keep/keep-note.txt와 empty-folder/keep 보존. 승인 범위를 다른 삭제로 확대하지 말 것.
- 앱 종료 상태의 대화/권한 main DB는 교체 전후 해시 동일. 재실행 후 새 테스트 대화의 정상 DB 변경과 구분한다. Application Support를 직접 수정하지 않았다.
- 시스템 Node는 x64. 실제 하네스 명령은 `/Users/dannysmacair/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/bin/node apps/bloomsweepy-mcp/tests/installed-os-settings-flow.mjs /Applications/BroomSweepy.app/Contents/MacOS/bloomsweepy-mcp`.
- 많은 기존 dirty/untracked 파일을 보존했다. `.codex/loop-state.md`, demo-assets, 개인 일별 대화2026-10-12/20은 열거나 수정하지 말 것. 현재 작업물 외 자동 stage/commit/cleanup 금지.

## Immediate Next Steps

1. 사용자에게 설치 교체·실제 통과 흐름과 미완료 경계를 보고한다. 이번 검사만으로 모든 제어/Windows/장시간 메모리 완료를 주장하지 않는다.
2. 수정 요청이 오면 P2-1: AssistantView의 원래 파일 결과 턴 연결을 보존하거나 명시적 이전 결과로 분리한다. 재진입 회귀 시나리오를 추가한다.
3. P2-2: 문서 공개의 현재 대화 root·색인 root·실제 전송 범위를 표시하고 현재 합성 폴더만 개별 허용할 수 있는 동선을 설계한다. 기존 토글로 권한을 넓히지 않는다.
4. 위 동선/권한이 해결되면 합성 ORCHID-SYNTHETIC-7421의 실제 문서검색 성공을 시험한다. OS 변경/앱 제거/다른 프로세스 종료/영구삭제/Windows 실기/장시간 soak는 별도 승인·환경에서 수행한다.

## Composition Diagram

```text
실제 Wry 채팅 → Codex(JSON 조회/검토 선택) → 공통 app dispatcher
                                              ├─ native 파일/성능/앱/Finder 조회 → 제한 결과 → Codex 답변
                                              ├─ 자체 메모리 검토 → 로컬 모달 → allocator 실행 → 완료 메시지
                                              └─ 본문 검색 → 별도 공개 root 비교 → 이번 시험 permission_required
외부 설치 MCP stdio → 같은 dispatcher/권한 경계 (별도 하네스로 검증)
```

## Session Memory Review

루트는 실제 `.git`가 있는 `/Users/dannysmacair/Documents/git/BroomSweepy`로 확정했고 출력 폴더는 외부 symlink가 아니다. Mnemo handoff-memory/project-storage/session-learning/project-skill-improvement를 읽었다. architecture 인덱스와018 본문은 실제 존재한다. 닥터 마지막 방문2026-09-29로30일 미만이므로 자동 진단 조건 불충족, 실행하지 않았다. 인계 validate_handoff는 READY·필수 항목/파일 링크/민감값 검사 PASS, MEMORY77줄/4997B로 상한 이내다.

기억018의 검증일·실제 Mac stdio/채팅 근거를 QA에 연결하고 MEMORY 목표·날짜만 갱신했다. `build_anchor_index.py --file memory/architecture/018-windows-management.md`는 의존 결정 없음. 재검색으로 인덱스→018→QA 연결을 확인한다. 결정 변경/기억 supersede 없음. 세션 UUID 미제공으로 gotcha/learned 원시 로그 정제와 offset 변경은 보류, 과거 백로그는 처리하지 않았다.

candidate: defer · target: none · reason: 프로젝트 전용 SKILLS-CATALOG/SKILL 파일이 검색되지 않았으며 신규 로컬 스킬 생성은 현재 범위 밖이다. 전역 설치 미러를 수정하지 않았다. 실제 audit 관찰은 QA에 남겼다.

## Verification

ARM64 빌드 exit0·서명 deep/strict PASS·설치cmp PASS·DB 보존 PASS·실제 MCP 하네스 exit0·실제 채팅9단계 완료/차단 상태 구분. 사진은 실행 중 실제 설치 화면의 원본 바이트다. Windows 실기·문서 본문 성공·장시간 전체 메모리 검증 NOT RUN.

#tags: mac-install, actual-chat, mcp-contract, permission-scope, ux-audit, arch:018
