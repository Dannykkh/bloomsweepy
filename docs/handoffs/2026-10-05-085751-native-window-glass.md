# Handoff: macOS 제목줄·글래스 재질 복원

## Session Metadata

Project: BroomSweepy. Branch/base: main / 7b135a11f46bb8892eb1a1042c6193177083553a. Source: codex. 기록 시작: 2026-10-05 08:57:51 Asia/Seoul. 컨텍스트 압축 뒤 작업 연속성을 위한 자동 인계다. 세션 UUID·사용자 턴 시각은 미제공이며 하루 전체 관찰을 이번 세션으로 추정하지 않는다.

## Origin

사용자 “아 맞다. 이 프로그램 타이틀바가 사라졌던데? 글래스모피즘도 약하고?”의 진단 뒤 “진행하자”로 이 국소 UI 개선과 맥 설치본 검증을 승인했다. 근거: conversations/2026-10-05-window-glass.md#현재-요청. 이전 커밋/푸시 승인을 재사용하지 않았다. 디자인 방향은 기존 Swift golden master이며 새 화면/업무 기능을 요구한 것이 아니다.

## Current State Summary

구현·ARM64 빌드·로컬 ad-hoc 서명·이 맥 설치본 교체·네이티브/브라우저 scoped UI 확인 완료. 최종 OS 제목줄에 BroomSweepy/1.7.0/창 버튼이 보이고 내용 영역의 기존 네이티브 재질이 더 드러난다. 기존 대화·설정 유지. UI 완료 당시 변경은 미커밋이며 Git/공개 릴리스는 그대로였다. 이번 테스트용 Vite 서버는 종료했다.

## Feature/Flow/Decision Snapshot

파일관리·AI·삭제/승인·성능 데이터 처리 경계는 변경하지 않았다. host 감지→material data attribute→CSS 토큰→AppKit content 합성만 조정했다. 창 제목은 이미 기존 native 코드가 설정하며 Visible/hiddenTitle false로 OS가 표시한다. 재사용 결정은 architecture007이고 이전 제품 아키텍처001–006을 대체하지 않는다.

## Implemented Features

| 변경 | 근거 | 확인 |
|---|---|---|
| 제목줄 복원 | OS Visible, hiddenTitle false, 기존 decorations 유지 | 최종 설치형 제목·버전·traffic lights |
| 재질 가독성 | 한 body wash24–44%, transparent shell, native sidebar28%, 보조 글자 밝기, 중복 sidebar/hero CSS blur 제거 | active/inactive 네이티브 렌더 |
| 플랫폼 fallback 유지 | macOS Tauri만 native-glass; 웹/Windows opaque; reduced-transparency CSS | 브라우저 실제 렌더·config 검사, OS 설정/Windows 런타임 미검증 |

## Composition Diagram

```text
main.tsx host gate → html[data-window-material] → App.css material tokens
macOS config Visible chrome + existing AppKit material → native window render
browser/Windows defaults → opaque render
```

## Feature Boundary / Menu / Screen Map

기존 navigation·224px sidebar/72px compact rail·dashboard ring·CPU/RAM·assistant·삭제 승인 UI를 유지했다. 가짜 HTML 창 버튼, 신규 blur/animation/의존성/권한/시작프로그램 변경은 없다. 개인 파일 작업·외부 LLM 요청은0이다. 창 제어 입력 성공과 독립 상태 계측을 구분한다.

## Critical Files / Files Modified

- apps/desktop/src-tauri/tauri.macos.conf.json: OS 제목줄 설정.
- apps/desktop/src/main.tsx 및 apps/desktop/src/App.css: host gate·재질 토큰.
- apps/desktop/tests/tauriConfig.test.ts: chrome 회귀 검사.
- DESIGN.md 및 docs/design-refs/2026-09-04-experience-rust-swift-experience.md: 정본 delta.
- docs/design-refs/2026-10-05-impl-log-window-glass.md: 렌더·검증·NOT RUN 정본.
- memory/architecture/007-native-window-material.md, MEMORY.md, memory/architecture/index.md, conversations/2026-10-05-window-glass.md: 결정·근거·인덱스.

## Decisions Made

Transparent 제목줄의 첫 설치형에서 안정적인 제목 대비를 확인하지 못해 Visible OS chrome을 사용했다. HTML 복제와 추가 CSS blur는 OS 접근성·기존 재질·저사양 범위에 이점이 없어 제외했다. architecture007에 대안·재검토 조건·근거를 남겼다. 기존 Rust 기본/Swift UI 방향과 공개 기록 공유 정책은 변경하지 않았다.

## Validation

TypeScript/프런트51개 PASS; 최종 설정 변경 뒤 config2개 PASS; production build/ARM64 bundle/deep strict ad-hoc signing PASS. 기존872kB chunk advisory 유지. 전체 Rust 회귀는 이번 재질/설정 delta에서 재실행하지 않았다. 이전 테스트 결과를 이번 신규 실행으로 보고하지 않는다.

브라우저1280×820/760×600/390×844: overflow 없음, compact rail72px, drawer Escape focus 복귀, 한글 Pretendard 실제 로드, 긴 메시지·error/loading·local-only input 상태 및 keyboard focus 확인. 실제 네이티브 dashboard/performance/assistant·저장 대화·close-to-hide/reopen 확인. drag/minimize/raise 입력은 실행했지만 global displacement/minimized flag는 독립 계측하지 못했다.

설치형 스크린샷은 docs/ui-audit/screenshots/window-glass-native-macos.png (inactive 창). compact JPG는 browser fixture이며 native 근거가 아니다. 기존 ignore 정책에 따라 스크린샷은 로컬 유지.

NOT RUN: Windows 런타임, native compact resize, OS reduced-transparency/motion 설정 변경, 임의 wallpaper별 대비 인증, 장시간 자원/FPS soak, 전체 Rust 회귀. 현재 visual 확인을 모든 플랫폼/메모리 안정성 보증으로 확대하지 않는다.

## Important Context

8GiB Mac·디스크 여유 약2.3–2.5GiB라 single-job/non-incremental과 기존 ARM64 sidecar를 재사용했다. final 설치 호스트 SHA256: 77614c5b67d45935c8804bffe1bc8399166e0b0d5cfba2f9cac9393994d3737c. /Applications/BroomSweepy.app은1.7.0 로컬 개발본이며 Apple 공증·새 공개 릴리스가 아니다. 원본 rollback: /private/tmp/broomsweepy-glass-install-backup-3BovoX/BroomSweepy.app; 첫 Transparent 시안도 /private/tmp/broomsweepy-glass-variant-one-l4pPuS/BroomSweepy.app에 보존했다. 앱 데이터를 지우지 않았다. 기존 사용자 demo-assets/를 수정하지 않았다.

## Immediate Next Steps

이번 UI 요청에는 남은 구현이 없고 설치된 변경과 실제 스크린샷을 전달했다. 후속으로 사용자 “커밋 푸시하자.”가 이번 UI 변경과 관련 정제 기록의 커밋·main 푸시를 승인했다. staging 범위·공백/기록·원격 HEAD를 확인하고 실행한다. 기존 demo-assets/·무시된 화면 캡처·빌드/설치 파일은 제외하며 버전·태그·공개 릴리스 변경은 하지 않는다. 제품 코드가 검증 이후 바뀌지 않아 전체 테스트·빌드를 반복하지 않는다.

다음 별도 제품 검증은 기존 실제 LLM 앱 도구 루프·Windows·장시간 자원 항목이며 UI 복원으로 그 항목까지 완료했다고 표시하지 않는다. 현재 앱을 다시 빌드할 필요는 없다. 커밋/푸시 결과는 실제 Git HEAD·원격 main으로 확인한다.

## Session Memory Review

project-storage로 확정한 실제 Git 루트 안의 non-symlink 기억/대화/docs에 기록했다. 기존 architecture/index·006 본문이 있고 doctor chart2026-09-29는30일 이내라 doctor SKIPPED. 수정 전 anchor 조회를 했고 새007을 MEMORY/index에서 재검색한다. MEMORY는74줄4,958B로 상한 내다. component-map.json이 없어 부품 지도 절차 N/A.

이번 UI 결정은007에 저장했다. 세션 UUID/시작 시각이 없어 raw gotcha/learned 관찰 정제는 보류하며 기존 백로그·offset을 변경하지 않았다. self-improvement/session-learning/project-skill-improvement 계약을 읽었다. candidate: defer; target: none; reason: 해당 프로젝트-local 스킬/세션 관찰 출처가 없어 전역 스킬 개선을 추정하지 않음. 신규 글로벌 규칙/스킬을 수정하지 않았다. 인계 validator READY86/100, diff 공백 검사 통과. MEMORY/index/대화에서007 연결을 재검색했다. validator의 파일 참조 자동 검출은0건이므로 이를 모든 링크의 자동 검증으로 보고하지 않는다.

#tags: window-chrome native-glass macos-install render-qa arch:007
