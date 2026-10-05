# 별도 검사 허용 스위치 제거 검증

2026-10-05, Mac ARM64/8GiB, main 로컬 개발본. 공개 v1.7.0 변경이 아니다.

## 계약

앱의 폴더 선택이 읽기 전용 범위를 지정한다. Settings·대화 팝업에 별도 검사 허용
스위치는 없다. 내장 AI는 세션 root, 외부 MCP는 앱에서 연결한 root·현재 ScanConfig를
사용한다. MCP에 임의 경로, 승인·삭제 실행을 추가하지 않는다. 공개 권한과 최종 확인은 유지한다.

## 실행한 검사

- `npm run check`, `npm run test:all`: PASS, 프런트60/실패0. 신규 범위 전환7개 포함.
- `CARGO_INCREMENTAL=0 cargo test -p bloomsweepy-desktop --lib -j 1`: PASS, 219/실패0/기존 opt-in3 ignored.
- `CARGO_INCREMENTAL=0 cargo test -p bloomsweepy-control -p bloomsweepy-mcp --lib -j 1`: control23/MCP12 PASS.
- 정확한 루트·현재 설정 연결, 이전 범위 선철회, 철회 실패 시 변경 중단, 새 연결 실패 시
  로컬 검사 유지, bridge 없음, 같은 폴더 재시도·중복 저장 방지를 합성 테스트로 검증했다.
- Rust store에서 첫 선택·다른 폴더 재선택·범위 해제를 확인하고 시스템/검색/정리 공개나
  작업/최종 계획이 자동으로 생성되지 않음을 확인했다. 기존 wire의 임의 경로 거부 테스트도 PASS.
- 실제 Settings와 AssistantView를 재사용한 격리 브라우저 fixture에서 양쪽 스위치 제거와
  안내를 확인했다. 파일 작업·AI 전송0. 처음 안내가 grid의 좁은 열에 배치되는 렌더 문제를
  발견해 전체 폭 notice로 수정하고 시각 확인했다.
- `git diff --check`: PASS.

## 빌드·설치

- 프런트 production build, ARM64 Rust release 앱·MCP·문서 worker 빌드: PASS.
  문서 worker의 `--check-heap-budget`: `heap-budget-rejection-ok`.
- `npm run tauri -- build --bundles app`의 자동 패키징은 FAIL: 이 맥의 Node가 x64로
  실행돼 bundler가 없는 x86_64 sidecar를 찾았다. 생성된 Info.plist·아이콘을 유지하고
  ARM64 앱과 sidecar 두 개를 묶어 로컬 ad-hoc 서명으로 완성했다. 자동 패키징 성공이나
  Apple 공증을 의미하지 않는다. `file`로 실행파일 세 개의 ARM64를 확인했다.
- bundle·설치본 `codesign --verify --deep --strict`, Info.plist lint: PASS.
  설치된 앱 실행파일 SHA256:
  `7c823b74a271c98ad57df2bd4979e78ecb0560156677e9c2dc9a1bc828610c0b`.
- 기존 앱을 정상 종료한 뒤 `/private/tmp/broomsweepy-scan-scope-backup-TDLS2A/BroomSweepy.app`
  에 보존하고 `/Applications/BroomSweepy.app`을 교체·재실행했다. 버전은 로컬 1.7.0이다.
- 실제 설치본 Settings에서 별도 검사 스위치가 없고 안내가 전체 폭으로 보이는 것을 확인했다.
  Remember, 시스템 검사 ON, 정리 검토 ON, 파일 검색 공개 OFF와 기존 시작·언어 설정이
  유지됐다. 개인 폴더 선택이나 권한 변경은 하지 않았다.

실제 네이티브 폴더 선택→검사 전체 흐름: NOT RUN — 범위 연결은 위의 unit/store 검사로 검증.
일반 사용자 파일·실제 AI 전송·Windows 런타임: NOT RUN — 이번 옵션 변경 검증에 필요하지 않음.
CLI 자체 도구의 완전 차단은 별도 미완료 이슈다. 이번 변경이 해결했다는 의미가 아니다.

## 자원·보존

초기 여유755MiB. 별도 release-profile 테스트는 의존성 재빌드가 시작돼 중단하고 기존
debug test cache/jobs1/incremental0로 검증했다. 앱 빌드 중 여유315MiB에서 이번 검증이
생성한 테스트 실행파일3개·desktop 객체16개(총113MiB)만 경로/시각 확인 후 제거했다.
다음 Cargo test로 재생성할 수 있다. 개인 파일·앱 데이터·이전 앱 백업은 보존했다.
설치 완료 뒤 이번 빌드가 생성한 사용 종료 static archive·dylib 두 개(약92MiB)도
정확한 경로·시각을 확인해 제거했다. 재빌드로 복원 가능하다. 마지막 여유342MiB로
여전히 매우 부족하며 대규모 문서 색인 등의 자원 제한은 해제하지 않았다.

Settings 증거: `docs/ui-audit/screenshots/2026-10-05-no-scan-toggle-settings.png` (로컬 ignored).
실제 설치본 증거: `docs/ui-audit/screenshots/2026-10-05-no-scan-toggle-native.png` (로컬 ignored).
이 검증 기록 작성 시점에는 커밋/푸시 전이었다. 후속 인계·커밋 범위는
[핸드오프](../handoffs/2026-10-05-164444-mac-chat-permissions-push.md)를 따른다.

#tags: folder-scan, scoped-mcp, no-permission-toggle, regression, arch:009, arch:010
