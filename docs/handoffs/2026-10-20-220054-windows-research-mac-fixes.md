# Windows 조사와 Mac 감리 후속

Date: 2026-10-20 22:00 KST
Project: BroomSweepy
Branch: main · HEAD 85620ca · 미커밋 개발본
Origin: RemoveWindowsAI를 병렬 조사하고 Mac/Windows 기능을 구분하며 미완료 개발·수정을 계속해 달라는 사용자 요청.
Origin source: 현재 사용자 턴(세션 ID·정확한 턴 시각 미제공). 컨텍스트 압축 후 연속성 기록이며 작업을 중단하거나 완료 판정하지 않는다.

## Current State Summary

- 이전 Mac 관리 구현과 Argos 재감리 결과는 `docs/plan/macos-management/verify-report.md`에 있다. 현재 요청은 그 보고서의 A1–A5 수정도 허용한다.
- 22:09 후속 완료. `mac_management_plan_review`는 RemoveWindowsAI commit6c30eeb의 실제 소스/복원 범위/Microsoft 공식 정책을 읽기 전용 조사했고, `docs/plan/windows-management/research-and-scope.md`로 통합했다. 스크립트 실행·AV/registry 변경 없음.
- `mac_settings_backend`: mac_settings.rs tests만82줄 추가. A5 실제 restore→fake native 복원→최종 SQLite commit 실패→명시적 무쓰기 복구 회귀 통과.
- `mac_settings_ui`: CleanupView/기존 cleanupTree helper/tests/locales6파일. A2 개발캐시/앱데이터/혼합 Review 확인 문구 구분. 승인 게이트 유지.
- 메인: source-map-js1.2.2 한 패키지만 최소 업데이트, A3 사전/A4 17노드 흐름도, README/보고서/Zeus 상태와016 검증 근거 동기화 완료. 작업자 모두 종료/파일 소유권 반환, 별도 실행 중 검사 없음.

## Feature/Flow/Decision Snapshot

### Implemented Features

none — 새 기능은 추가하지 않았다. 기존 정리 확인 문구/개발 의존성을 수정하고 복원 회귀·문서를 보완했다.

### Feature Boundary

Windows 제품 기능은 아직 추가하지 않았다. 기존 기능은 고정 Finder Bool 5개·내구성 복원 저널·개발 캐시 Review·읽기 전용 macos.inspect다. 공통 UI/engine과 OS별 adapter 경계를 유지한다. 입력→검증→prepared 저널→native effect→readback→최종 기록이며, AI에는 OS 쓰기 권한을 제공하지 않는다. 정확한 도면은 `docs/plan/macos-management/flow-diagrams/`를 따른다. settings에는 no-op/무쓰기 복구/final commit 실패 분기를 보완했다.

## Important Context

- 이전 전체 증거: Rust478 PASS/6ignored, mock UI10/10, 실제 격리 Wry IPC negative10/10. 이번 집중 Rust16PASS/1ignored(session43671), frontend전체141PASS(88044), TypeScript/production build3113 exit0, npm full70430/production37147 audit 각각0, fmt/diff PASS.
- 새 테스트82줄 제외 mac_settings.rs SHA는 기존 a19a7ef…64668과 정확히 일치한다. 생산 native 동작을 바꾸지 않아 기존 Wry 증거를 재사용하며 전체 Rust479를 새로 실행했다고 합산하지 않는다. A2의 새로운 설치형 렌더는 NOT RUN이다.
- Windows 실행/설치, 실제 Finder 정상 쓰기·복원/설치형 전체 검사는 NOT RUN. 이번에도 실제 사용자 설정·파일을 변경하는 테스트는 하지 않는다.
- 커밋/푸시/릴리스/설치 교체는 현재 요청 범위가 아니다.
- 사용자 소유 `demo-assets/`와 기존 개인 conversations는 읽거나 삭제/스테이징하지 않는다. 기존 변경을 보존한다. `.codex/loop-state.md`는 수정하지 않는다.

## Immediate Next Steps

1. 현재 사용자 요청의 Mac 감리 보완A1–A5/Windows 병렬 조사는 완료. 상세 결과는 verify-report의22:07 후속 수정 절에 있다. 전체 제품/출시 판정은 여전히 조건부다.
2. 다음 Windows 구현은 research-and-scope의 고정5행 읽기 전용 inventory+허용된 OS 설정 URI 연결 제안에서 시작한다. 임의 PowerShell/패키지 강제 제거/Defender 변경은 가져오지 않는다.
3. Mac 설치형 정상 변경·복원은 정확한 대상·원복 계획 아래 별도 검증한다. Windows 실행·CI와 포괄 보안은 별도 실행 증거가 필요하다. 요청 없이 설치/커밋/푸시하지 않는다.

## Session Memory Review

저장 루트는 실제 `.git`가 있는 프로젝트 루트로 확인했고 docs/handoffs/memory는 루트 내부 실제 디렉터리다. Mnemo project-storage/handoff-memory/self-improvement/session-learning/project-skill-improvement를 읽었다. architecture/016 본문과 MEMORY 연결을 확인했다. 닥터 방문2026-09-29로30일 이내라 반복 진단하지 않았다. 새 아키텍처 결정을 만들지 않고016의 오래된 검증 설명을 실제 Wry/집중 복원 결과로 갱신했으며 build_anchor_index로 mac_settings.rs→016을 재검색했다. MEMORY77줄/4963B 상한 유지. 신규 gotcha/learned는 확정 버그/실제 세션 ID 근거가 없어 생성하지 않았다. 로컬 프로젝트 스킬 대상도 없어 개선 candidate는 보류하고 전역 스킬은 수정하지 않았다. 인계 검증은 READY82/100(필수 섹션·참조·민감값 검사 통과)이며 완료 내용 반영 후 재검증한다.

#tags: 윈도우조사, 맥감리후속, 병렬작업, 재개지점, arch:016
