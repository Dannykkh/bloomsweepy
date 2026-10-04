# Architecture - 설계 결정

> MEMORY.md 키워드 인덱스에서 이 파일로 연결됩니다.

---

### 운영체제 휴지통 비우기 별도 영구 삭제 예외
`tags: empty-system-trash, irreversible-confirmation, finder, one-shot-nonce`
`date: 2026-09-07`
`source: codex`

- ✅ CURRENT: 사용자 요청으로 공간 정리 공통 상단에 OS 휴지통 열기/비우기 추가. 연결된 드라이브의 현재 사용자 휴지통 전체·다른 앱 항목 포함, 선택 폴더 제한 없음. 체크박스와 2분 일회용 nonce, main WebView 전용, AI/MCP 비노출.
- 전체 목록·내용·크기 사전 스캔 없이 macOS 고정 Finder empty 요청(AppleEvents 목적/entitlement), Windows SHEmptyRecycleBinW(NULL,NULL,0) 사용. 권한 상승·재귀 삭제 fallback 없음. OS 요청 뒤 앱 취소 불가, 불명확 결과는 재시작 전 재요청 차단. 작업 기록은 복원 보장이 아님.
- Rust246+frontend40 및 실제 React/mock UI 검증 통과. **실제 OS 비우기 0회, 권한 승인 없음, 설치본 미교체, Windows native/서명 bundle 검증 미실시.**
- **참조**: [검증 기록](../docs/qa/2026-09-07-empty-system-trash.md)

### 제한된 탐색·문서 격리·일반 폴더 검토 구현
`tags: streaming-walk, document-worker, folder-review, resource-guards`
`date: 2026-09-07`
`source: codex`

- ✅ CURRENT: jwalk 대신 요청형 DFS(열린 디렉터리128/경로256KiB), 후보 바이트 예산, 호스트512MiB·시스템여유256MiB 샘플 가드. SQLite 캐시4MiB/temp1MiB/DB256MiB/WAL256MiB/합산512MiB/여유2GiB. 샘플 경계는 OS 하드쿼터가 아니다.
- PDF·Office는 동봉 document-worker 단일 자식에서 Rust 할당128MiB/입력32MiB/출력4MiB/15초로 격리. 신뢰된 형제 바이너리만 실행하고 실패 시 자신의 자식만 종료·회수한다. 개발·패키징에 MCP와 함께 준비해야 한다.
- 폴더 지도 더보기→전체 경로/포함 항목 검토→하위 항목 체크→5분 일회용 최종 버튼→메타데이터 재검증→기존 OS Trash·저널. 일반 다운로드 폴더의 중첩 숨김/앱/프로젝트 포함, root/시스템/클라우드/링크/다른 장치 거부.
- 내부 링크 전면 거부 부분은 **SUPERSEDED** (superseded-by: [002](architecture/002-opaque-folder-symlinks.md), #nested-link-rejection): 실제 node_modules/.bin 폴더 관리 차단을 확인해 POSIX 내부 링크만 불투명하게 취급. 선택 대상/부모 링크·클라우드/다른 장치 보호와 나머지 계약은 유지한다.
- 작은 합성 반복 및 UI/mock Trash 검증 완료. 설치본1.6.1 미교체, Windows/전체 프로세스 트리 장시간/실제 OS Trash E2E는 남음.
- **참조**: [검증 기록](../docs/qa/2026-09-07-low-resource-folder-actions.md)

### 현재 8 GiB 맥북을 저자원 검증 기준으로 선정
`tags: low-resource-baseline, 8gb-mac, memory-budget, disk-budget, graceful-failure`
`date: 2026-09-07`
`source: codex`

- ✅ CURRENT: 사용자는 RAM·저장 공간이 부족한 현재 맥에서 최대한 시험하기를 원한다. 고사양 기기로 옮겨 통과시키는 것으로 이 검증을 대체하지 않는다. 06:40 KST 기준 RAM8GiB, 여유 디스크 약16GiB이며 아직 최소 지원 사양 합격을 공표한 것은 아니다.
- 합격 기준은 검사 중 최고 메모리·DB/WAL/임시 공간의 공동 예산, 부족 시 안전한 중단, 데이터 보존, 취소·재시도·반복 후 자원 회복이다. RAM 절감을 위해 디스크를 무제한 사용하는 설계도 금지한다.
- 작은 합성 자료와 낮은 테스트 예산/오류 주입부터 검증하고 실제 RAM 고갈·디스크 채우기 실험은 하지 않는다. 기본 앱 계측과 개발 빌드 공존 부하는 분리한다.
- **참조**: [저자원 검증 기준](../docs/qa/low-resource-acceptance.md)

### 대화형 빈 폴더 도구의 미출시 소스 구현
`tags: empty-folder-tools, one-shot-plan, structured-envelope, token-privacy`
`date: 2026-09-06`
`source: codex`

- SUPERSEDED (superseded-by: [001](architecture/001-conversational-file-workspace.md), #empty-folder-tools): 제품 방향의 첫 구현으로 세션 폴더 로컬 검사 → 후보 검토·제외 → 정확한 경로 확인 카드 → 최종 버튼 → 기존 OS Trash/이력 경로를 연결했다. 당시 설치본 v1.6.1은 그대로이며 실제 새 CLI 계약 검증은 남았다. 2026-10-04에 일반 파일·폴더와 실제 Codex/설치 검증으로 확장했고 빈 폴더 기능은 유지한다.
- 모델은 strict JSON으로 검사·목록·선택 변경만 요청한다. 최대 200개 로컬 후보 중 모델에는 24개 페이지를 전달하며 임의 경로·셸·삭제·승인은 허용하지 않는다. 5분 일회용 계획은 세션·revision·선택 ID를 검증하고 실행 전에 원자적으로 소비한다.
- 로컬 검사에는 LLM 토큰을 쓰지 않지만 제한된 이름·통계·후보 ID·질문·대화는 공급자에게 전달될 수 있다. 자동 문맥에 전체 경로·본문을 넣지 않으며 사용자 입력까지 익명화하지 않는다. 절감률은 미측정이고 잘 구성한 CLI와 차이가 작을 수도 있다.
- **참조**: [구현 계약](../docs/plan/conversational-empty-folders/plan.md), [검증 기록](../docs/plan/conversational-empty-folders/verification.md)

### 대화형 파일관리 시스템으로 제품 방향 확정
`tags: conversational-file-management, product-direction, ai-tools, rust, readme`
`date: 2026-09-06`
`source: codex`

- ✅ CURRENT: 사용자는 검사 결과 해설이 아니라 대화 안에서 검색·검사·후보 검토·제외 조건 조정·최종 확인·실행 결과 확인을 끝내는 파일관리 시스템을 원한다고 명시했다. README 제목과 소개를 네 언어 및 desktop README에 반영했다.
- 목표 역할 분리는 AI의 요청 해석·앱 도구 선택, Rust의 실제 파일 작업·실행 직전 재검증, 채팅의 대상 미리보기·최종 확인·결과 표시다. 트리맵·파일 목록은 함께 사용하는 작업 화면이며 공급자별 CLI는 공통 앱 도구에 연결하는 방향이다. 기존 Rust 기반·SwiftUI 참고 원칙은 유지한다.
- 현재1.6.1의 내장 채팅은 저장된 제한 요약을 재사용한다. 외부 CLI/MCP와 내장 채팅 연결, 질문에 따른 추가 스캔, 후보 조정 및 파일 작업 실행은 아직 미구현이며 빈 폴더는 발견·목록 표시만 지원한다. README는 제품 목표와 현재 기능을 구분하고 미구현을 완료처럼 홍보하지 않는다.
- **참조**: [README 제품 방향](../README.md), [호출 흐름 설명](../EXPLANATION-ai-assistant.md)

### Swift 트리맵 동작과 Rust 파일 신원 검증
`tags: treemap-actions, file-reveal, scan-identity, os-trash, accessibility`
`date: 2026-09-05`
`source: codex`

- 파일 클릭은 파일 관리자에서 위치 열기, 폴더 클릭은 드릴다운이다. 블록·순위는 경로 기준 hover/focus와 색상을 공유하고, 더보기 메뉴는 파일 한 개의 휴지통 이동만 별도 확인 후 제공한다.
- Rust는 최신 지도 generation과 비직렬화 파일 신원·정밀 수정 시각을 보관하고, 변경·링크·폴더·범위 밖 항목을 거부한 뒤 기존 fingerprint·journal·OS Trash 절차를 재사용한다. 이동 후 현재 폴더를 다시 검사한다.
- Mac 설치와 읽기 전용 UI 검증, 206개 자동 테스트 통과. 실제 테스트 파일의 휴지통 이동은 사용자 확인 대기이며 Windows 런타임은 미검증이다.
- **참조**: [구현·검증 기록](../docs/design-refs/2026-09-05-impl-log-treemap-actions.md)

### Rust 단일 기반과 SwiftUI 경험 통합
`tags: rust-base, tauri, swiftui, native-glass, direct-dmg`
`date: 2026-09-04`
`source: codex`

- ✅ CURRENT: Rust/Tauri를 유일한 제품·배포 기반으로 유지하고, SwiftUI 앱은 UI·UX와 기능 흐름의 golden master로만 사용한다.
- 첫 화면은 네이티브 글래스, 큰 저장공간 링, 결과 중심 원클릭 동작을 우선한다. macOS 직접 배포판은 Tauri private API를 쓰며 Mac App Store 빌드는 별도 불투명 폴백이 필요하다.
- **참조**: [대화 기록](../conversations/2026-09-04-codex.md), [디자인 계약](../docs/design-refs/2026-09-04-experience-rust-swift-experience.md)

### 다중 드라이브 카드 덱과 검사 범위 분리
`tags: multi-drive, drive-deck, flip-animation, scan-root, accessibility`
`date: 2026-09-04`
`source: codex`

- ✅ CURRENT: 시스템 드라이브를 큰 카드로 시작하고 나머지 사용자 드라이브를 작은 원 카드 rail로 표시한다. 선택은 표시만 바꾸며 보고서를 지우거나 스캔하지 않는다.
- `이 드라이브 검사`에서만 선택 드라이브를 실제 scan root로 확정한다. 카드 교대는 460ms FLIP transform이며 reduced-motion은 즉시 전환하고 외장 드라이브 제거 시 시스템·고정 드라이브 순으로 폴백한다.
- **참조**: [디자인 계약](../docs/design-refs/2026-09-04-experience-rust-swift-experience.md), [구현 로그](../docs/design-refs/2026-09-04-impl-log-rust-swift-experience.md)

### macOS 디스크 이미지와 실제 드라이브 분리
`tags: macos, disk-image, hdiutil, volume-filter, dmg`
`date: 2026-09-04`
`source: codex`

- ✅ CURRENT: macOS 백엔드가 `/usr/bin/hdiutil info -plist`의 마운트 지점을 한 번 수집해 `VolumeInfo.isDiskImage`로 전달하고, 대시보드만 해당 볼륨을 숨긴다.
- `readOnly`, `removable`, 파일시스템·이름 휴리스틱은 실제 외장 매체를 오인할 수 있어 사용하지 않는다. 조회 실패 시 실제 드라이브를 숨기지 않도록 fail-open한다.
- **참조**: [대화 기록](../conversations/2026-09-04-codex.md)

### Rust 성능 계측과 macOS 정상 종료 경계
`tags: rust, tauri, performance-monitor, sysinfo, appkit, graceful-termination`
`date: 2026-09-04`
`source: codex`

- ✅ CURRENT: 실제 CPU·메모리·swap과 상위 프로세스는 지속되는 Rust `sysinfo` sampler로 읽고, Swift의 메모리 압박형 `purgeMemory`는 이식하지 않는다.
- macOS 일반 GUI 앱만 만료·단일사용 불투명 token과 snapshot에서 보관한 동일 AppKit 객체의 실행 직전 신원 재검증 뒤 정상 종료를 요청한다. PID 재조회·force kill·raw PID 명령·자동 폴백은 없고 다른 플랫폼은 읽기 전용이다.
- **참조**: [성능 계약](../docs/design-refs/2026-09-04-experience-rust-performance.md), [아키텍처](../docs/architecture/startup-memory-status.md)

### macOS 원클릭 current-process 메모리 반환
`tags: rust, tauri, memory-cleaner, malloc-zone, current-process, one-click`
`date: 2026-09-05`
`source: codex`

- ✅ CURRENT: `메모리 정리`는 macOS에서 `malloc_zone_pressure_relief(NULL, 0)`로 BroomSweepy 호스트 프로세스의 반환 가능한 malloc page만 OS에 돌려준다. Swift의 대량 임시 할당과 URL cache 초기화는 이식하지 않는다.
- allocator가 보고한 반환 바이트만 인과적 결과로 표시하고 RSS·시스템 available 전후값은 확보량으로 주장하지 않는다. 다른 앱·cache·swap·누수 메모리는 건드리지 않는다.
- **참조**: [메모리 정리 delta](../docs/design-refs/2026-09-05-brief-memory-clean-delta.md), [아키텍처](../docs/architecture/startup-memory-status.md)

### CPU와 RAM의 동등한 원형 계기
`tags: dual-rings, cpu, ram, scoped-cleanup, ui-ux`
`date: 2026-09-05`
`source: codex`

- ✅ CURRENT: 공통 시스템 성능 제목 아래 CPU와 RAM을 같은 크기의 원으로 나란히 보여주고 RAM 사용/전체·사용 가능·스왑을 함께 표시한다. 좁은 콘텐츠 폭에서는 세로 배치한다.
- 정리 동작 자체는 위 current-process 정책을 유지한다. UI 명칭은 `앱 메모리 정리 · BroomSweepy 전용`으로 명확히 하며 CPU 정리나 시스템 전체 메모리 확보를 주장하지 않는다. 사용률을 메모리 압력으로 환산하지 않는다.
- **참조**: [동일 원형 계기 delta](../docs/design-refs/2026-09-05-brief-dual-performance-rings.md)

### 성능 링의 연속적인 값 전환
`tags: smooth-metrics, css-transition, reduced-motion, performance`
`date: 2026-09-05`
`source: codex`

- ✅ CURRENT: 사용자 피드백에 따라 CPU·RAM 원의 stroke-dashoffset만 900ms CSS ease-out으로 연결한다. 숫자·ARIA는 실제 최신값을 즉시 표시하고 계측 주기는 유지한다.
- 초기값은 0에서 출발하지 않고, 도중 새 값이 오면 현재 그려진 위치에서 전환한다. reduced-motion에서는 전환을 제거한다. 프레임별 React 갱신이나 추가 라이브러리는 쓰지 않는다.
- **참조**: [모션 계약](../docs/design-refs/2026-09-05-motion-performance-rings.md)

### 플랫폼별 앱 제거와 명시적 로컬 열기
`tags: application-management, windows-uninstall, related-data, file-open, safety`
`date: 2026-09-07`
`source: codex`

- ✅ CURRENT: 사용자가 정식 제거 우선 + Mac 본체 Trash + 관련 데이터 별도 선택, Windows OS 제거 절차 분리를 승인. 앱 목록은 제한된 메타데이터만 조회한다.
- Mac 관련 후보는 정확한 bundle ID의 Caches/Preferences만. 알려진 앱 루트의 제한된 소유관계 확인이 불완전하면 후보를 주지 않는다. Windows는 고정 Installed apps 설정을 열며 UninstallString/폴더/AppData를 직접 제거하지 않는다.
- 열기/위치 표시는 공통 Rust 검사로 링크·클라우드·실행형을 제한한다. 링크/별칭은 검증한 부모만 표시하며 삭제와 OS 오류는 성공으로 단정하지 않는다.
- **참조**: [구현 계획](../docs/plan/application-management-open-actions/plan.md)

### 성능 메뉴는 대시보드 바로 다음
`tags: navigation-order, dashboard, performance`
`date: 2026-09-07`
`source: codex`

- ❌ SUPERSEDED (superseded-by: #앱-관리를-성능-다음-독립-메뉴로): 사용자 요청으로 메뉴를 대시보드 → 성능 → 공간 정리 → 파일 관리 → AI 도우미 → 설정 순서로 변경. 선택적 Docker 관리는 AI 도우미 뒤를 유지한다.
- 데스크톱/축소 아이콘 레일/모바일 오버레이는 AppShell의 같은 navigation 배열을 사용한다. OS별 기능이나 라우팅은 변경하지 않는다.
- **참조**: [디자인 계약](../DESIGN.md#spatial-model), [AppShell](../apps/desktop/src/components/AppShell.tsx)

### Rust macOS 메뉴 막대 상주와 저빈도 집계
`tags: macos-menu-bar, native-popover, resident-window, aggregate-sampler`
`date: 2026-09-07`
`source: codex`

- ✅ CURRENT: Swift MenuBarExtra 흐름을 Rust/objc2의 NSStatusItem + NSPopover로 이식한다. 아이콘 옆 RAM 숫자만 선택 표시하며 패널/설정과 NSUserDefaults를 동기화한다.
- 창 닫기는 숨김, 열기는 기존 창 복원, 종료/⌘Q는 실제 종료. 네이티브 아이콘 생성 실패 때는 닫기 동작을 변경하지 않는다. Windows 트레이는 유지한다.
- 별도 WebView 없이 10초 CPU/RAM/루트 statvfs 집계 하나만 유지한다. 파일·앱 목록·측정 이력은 수집하지 않고 UI 대기열도 한 건으로 제한한다. 숨겨진 메인 창의 기존 상태/메모리는 유지하므로 닫기가 메모리 해제는 아니다.
- **참조**: [경험 계약](../docs/design-refs/2026-09-07-experience-macos-menu-bar.md), [구현·검증 로그](../docs/design-refs/2026-09-07-impl-log-macos-menu-bar.md)

### 앱 관리를 성능 다음 독립 메뉴로
`tags: navigation-order, applications-nav, performance`
`date: 2026-09-07`
`source: codex`

- ✅ CURRENT (supersedes: #성능-메뉴는-대시보드-바로-다음): 추가 사용자 요청으로 대시보드 → 성능 → 앱 관리 → 공간 정리 → 파일 관리 → AI 도우미 → 설정 순서로 변경. Docker 조건부 위치는 유지한다.
- 앱 관리는 공간 정리 하위 탭과 활성 메뉴 집합에서 분리한다. 기존 앱 목록·제거 안전장치·휴지통 진입점은 유지하며 메뉴 표기는 짧은 `앱 관리`를 사용한다.
- **참조**: [디자인 계약](../DESIGN.md#spatial-model), [AppShell](../apps/desktop/src/components/AppShell.tsx)
