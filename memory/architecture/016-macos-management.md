# 고정 Mac 설정과 개발 캐시 관리

status: CURRENT
date: 2026-10-20
source: codex
tags: macos-management, finder-preferences, durable-undo, developer-cache, read-only-mcp
evidence: conversations/2026-10-20-team-poseidon.md#사용자-요청 (기록 20:38 KST; 원래 턴 시각 미제공)
alternatives: all-upstream-tweaks — OS26.4 AI profile deprecation/private API/서비스 권한을 섞으므로 제외, 개별 호환성 증거 뒤 확장; links-only — 실제 원복 제공 불가; chat-full-access-system-write — 파일 이동 권한과 OS 변경 권한이 달라 제외.
depends-on: [[004-app-tool-investigation]], [[009-opt-in-permission-lifetime]], [[013-common-file-mcp-results]], [[015-native-chat-access]]
sources: docs/plan/macos-management/spec.md; docs/plan/macos-management/api-spec.md; docs/plan/macos-management/research.md
files: apps/desktop/src-tauri/src/mac_settings.rs, apps/desktop/src-tauri/src/mac_settings/native.rs, apps/desktop/src-tauri/src/mac_cleanup.rs, apps/desktop/src-tauri/src/app_tools.rs, crates/bloomsweepy-control/src/capabilities.rs, crates/bloomsweepy-core/src/cleanup.rs, apps/desktop/src/views/MacManagementView.tsx
reopen-when: 설정 5개 외 추가, 새 macOS major, profile/service/AI model mutation, 채팅 OS 실행 권한, 128기록 초과 보존 요구가 생기면 호환성·권한·복원 계약을 재설계한다.
last_verified: 2026-10-20 (기존 full Rust478/6ignored·readonly1·mock UI10/10; 후속 집중 Rust16/1ignored·frontend141·격리 Wry IPC negative10/10 PASS. 설치형 전체/정상 Finder 변경·Windows는 NOT RUN, docs/plan/macos-management/verify-report.md)

Finder 표시 Bool5개만 고정 ID로 조회·변경한다. current-user/any-host 명시값 부재는 OFF가 아니다.
관리형/host별/타입 오류를 차단하고, 신뢰된 main 창의 명시 Apply/Restore만 변경한다. AI/MCP는
같은 엔진의 read-only macos.inspect와 화면 연결만 제공한다. file full_access를 OS 변경에 상속하지 않는다.

SQLite FULL prepared 커밋→native write→stored readback→결과 커밋. 중단 기록을 재생하지 않고
현재값 비교로 감지한 충돌만 거부한다. CFPreferences에는 외부 CAS/ABA 보호가 없다. 실패한
동기화는 해당 domain을 프로세스 내 잠가 조회가 잠재 쓰기를 재시도하지 않도록 한다. 원래값
부재까지 복원하며 모든 체인을128건 보존한 뒤 신규 변경을 거부한다. Finder는 자동 재시작하지 않는다.

DerivedData7일/iOS DeviceSupport30일은 수정일 기반 Review 후보다. home 아래 고정 조상 link는
제외하고 기존 bounded scan→선택→재검증→Trash journal을 사용한다. 미사용/삭제 안전/물리 확보량을 보장하지 않는다.

26번째 capability가 discovery16KiB 회귀를 일으켜 format2에서 중복 inputTypes만 index에서
제외했다. 기능 ID·예제·권한과 필수 상세의 전체 계약은 유지하며 상한을 올리지 않는다.

초기 alpha/설치 Chrome의 간헐 정지는 정식 Playwright1.64.1/전용 ARM64 Headless Shell156/ARM64 Node
환경에서 동일 핵심 assertion10/10 통과했다. 세부 원인은 변수별 분리가 없어 단정하지 않는다.
이를 실제 Tauri 설치형/IPC 우회 보안 검증 성공으로 확대하지 않는다.

후속 격리 실제 Wry/JS→native negative10/10과 거부8건 각각 설정/저널 불변을 확인했다.
이 증거는 제품 설치형 전체 setup/정상 설정 변경 성공과 구별한다. restore()의 fake 복원 effect 후
최종 기록 실패도 pending 증거→재시작 조회→추가 쓰기 없는 명시 완료 회귀로 검증했다.
개발 캐시와 앱 데이터의 Review 문구를 구분하되 기존 승인/재검증 게이트는 유지한다.
