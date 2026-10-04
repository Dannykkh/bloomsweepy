# 저자원 보호와 일반 폴더 이동 — 소스 검증

> 후속09:44KST: 저자원 보호를 포함한 새 로컬 arm64 앱으로 교체하고 문서 helper의 release 메모리 진단을 검증했다. 아래 미교체 표현은 초기 검증 시점이며 최신 상태는 [앱 관리·설치 QA](2026-09-07-application-management-open-actions.md) 참조. 전체 앱 장시간 soak와 기존 종료 원인은 여전히 미확정이다.

2026-09-07, Apple Silicon MacBook Air / RAM 8 GiB / Data 여유 약 16 GiB. 설치된 v1.6.1은 교체하지 않았다. 이전 종료의 직접 원인은 여전히 미확정이며, 이번 결과는 소스에서 확인한 위험 경로의 수정과 제한된 시험 결과다.

## 구현된 정책

| 경계 | 정책 | 보장 범위 |
|---|---|---|
| 탐색 | 요청할 때 한 항목씩 DFS, 열린 디렉터리 최대 128, 유지 경로 256 KiB, 개별 경로 16 KiB | 디렉터리 전체 목록·무제한 대기열 없음. 자원 고갈은 실패 반환과 핸들 해제 |
| 호스트 RAM | 프로세스 메모리 512 MiB 초과 또는 시스템 여유 256 MiB 미만이면 중단; 500ms 간격 | 샘플 기반 조기 중단. WebKit·CLI 포함 프로세스 트리 하드쿼터 아님 |
| 검색 작업 집합 | 중복 후보 추정 32 MiB, 큰 파일 후보 4 MiB에서 2 MiB로 축소, 직접 자식/위치 경로 8 MiB | 기존 개수 제한과 함께 적용; 생략/제한 상태 표시 |
| SQLite | 연결 캐시 4 MiB, 임시 캐시 1 MiB, temp FILE, mmap 및 추가 SQL 작업 스레드 없음 | SQLite 전체 RSS의 하드 상한 아님 |
| 저장소 | DB별 256 MiB 페이지 한도, WAL 256 MiB, DB+보조 파일 합산 512 MiB; DB/임시 볼륨 최소 2 GiB 여유 | 페이지 한도는 강제. 파일 크기·여유 공간은 50ms 감시와 COMMIT 직전 검사 |
| 문서 파서 | PDF·Office를 전용 자식 프로세스 하나에서 처리. Rust live allocation 128 MiB, 입력 32 MiB, 출력 4 MiB, 15초 | 부모가 시간/취소/출력을 제한하고 자신이 생성한 자식만 종료·회수. OS 매핑·allocator 오버헤드 제외 |
| 폴더 검토 | 최대 20,000항목·깊이 64·누적 경로 8 MiB·30초, 파일 본문 미열람 | 전체 경로·파일/폴더 수·논리 용량 확인. 숨김 파일/중첩 앱/프로젝트도 포함 |
| 폴더 실행 | 세대에 묶인 5분 일회용 계획, 하위 항목 체크와 최종 버튼, 실행 직전 메타데이터 재검증 | 검사/사용자 기본/시스템/직접 앱/클라우드/링크/다른 장치 경계 거부; 기존 OS Trash·저널 재사용 |

일반 접근 제한과 문서 자체 손상은 제한된 문제 목록에 남긴다. 메모리·핸들·저장소 고갈 또는 문서 도구 기반시설 실패는 전체 색인 트랜잭션을 중단하고 기존 완료 FTS를 보존한다. COMMIT 성공 뒤 선택적 WAL 정리가 실패했다고 이미 확정한 작업을 롤백했다고 보고하지 않는다.

## 실제 실행 검증

- `CARGO_INCREMENTAL=0 CARGO_BUILD_JOBS=1 cargo test --workspace -- --test-threads=1`: 전체 회귀 실행. 최종 개수는 아래 마감 기록 참조. 외부 CLI 실환경 진단 1개는 의도된 opt-in ignored.
- `cargo clippy --workspace --all-targets -- -D warnings`: 통과. 새 합성 권한 테스트의 `0` 표기를 `0o0`으로 수정 후 재실행.
- `cargo fmt --all -- --check`, `git diff --check`: 통과.
- 프론트 `npm run check`, `npm run test:all`: 타입 통과, 37개 테스트 통과. 한국어/영어 기준 일본어·중국어 키/placeholder 회귀 포함.
- `npm run build`, `node --check scripts/prepare-sidecar.mjs`: 통과. 기존 단일 JS 청크 500 kB 경고는 남음(출력 약 729 kB, gzip 약 213 kB).
- 작은 PDF/Office 추출, 별도 도구의 초과 할당 거부(실제 대용량 할당 없음), 출력 상한·시간 제한·취소·회수 검사.
- 낮은 가상 여유 공간/SQL 페이지 한도/긴 SQL 취소를 주입하여 rollback·기존 FTS·재시도·4회 재색인 뒤 WAL 회수 확인. 실제 디스크를 채우지 않음.
- 합성 폴더의 숨김 파일·중첩 앱/프로젝트·링크/클라우드·변경·취소·만료·세대·중복 승인·장치 경계 회귀. OS 대신 임시 mock Trash에 이동하고 파일 내용과 저널 확인.

### 반복 시험 실측

`/usr/bin/time -l target/debug/deps/macos_resource_stability-00ea589af28586f7 --exact repeated_native_scans_preserve_results_and_bound_resources --nocapture --test-threads=1` (중간 검증 빌드):

- 4,096개 합성 파일의 스캔/지도/카탈로그 20회: RSS 18,624 → 18,816 KiB, 파일 디스크립터 4 → 4.
- 128개 합성 문서 색인/검색 20회: RSS 19,104 → 19,104 KiB, 디스크립터 4 → 4.
- 최고 RSS 20,332,544 bytes(약 19.4 MiB), 최고 physical footprint 12,469,312 bytes. 실행 5.11초.
- 취소 후 재시도, 20회 중복 검사, 심볼릭 링크 순환 차단, 8 GiB **희소 파일의 논리 용량** 집계 통과. 8 GiB 실제 데이터 읽기 시험이 아님.

이 수치는 작은 합성 자료를 처리하는 코어 테스트 프로세스다. 설치 앱+WebKit+CLI 전체 메모리나 전체 드라이브/장시간 검증으로 해석하지 않는다.

### 브라우저 UI 검증

CUA로 실제 React `treemap-fixture.html`을 렌더링하고 확인했다. IPC와 이동은 mock이므로 실제 사용자 파일은 변경되지 않는다.

1. Archives 더보기 → 검토 → 전체 경로와 개수 표시; 체크 전 최종 버튼 disabled, 체크 후 enabled. 최종 클릭 뒤 `trash:/Fixture/Treemap/Archives`와 지도 카드 제거 확인.
2. `?expired=1`: 만료 안내, 체크해도 실행 불가, 취소 후 이동 없음.
3. `?failure=1`: 변경 오류 표시, 이미 사용한 계획 재실행 disabled, 이동 없음.
4. `?prepare-failure=1`: 검토 오류 표시, 계획 없는 확인 창에서 실행 disabled.
5. 기존 report.pdf 파일 검토는 폴더 체크박스 없이 기존 최종 확인 유지. 취소 후 이동 없음.
6. 1280×720 확인 창 스크린샷: 전체 경로·경고·체크박스·확인 버튼 가시성 확인.

## 독립 검토와 도면 대응

workpm 계약에 따라 파일별 구현 후 자기 코드가 아닌 영역을 읽기 전용 교차 검토했다. 이 런타임의 기존 위임을 사용했고, 별도 외부 AI CLI에 전체 dirty/untracked 자료를 전달하는 리뷰는 실행하지 않았다. Engine: full-pass, 최종 범위 PASS. 확정 발견 4개 수정, 잔여 확정 발견 0개. 스타일성 중복 억제 0개. 설치/Windows/장시간 계측은 아래 별도 경계로 유지한다.

| 수정한 발견 | 대응 |
|---|---|
| worker 설치/시간/자원 실패가 문서 손상으로 합쳐져 기존 FTS 소실 | `WorkerError`와 `ExtractionError::WorkerFailure`, 즉시 rollback, 기존 색인/재시도 테스트 |
| 정리 후보의 직접 파일 경로가 RAM 검사 우회 | 시작·루트·직접 항목 반복에 공통 검사 |
| EMFILE/ENFILE/ENOMEM이 일반 읽기 오류로 취급 | OS 자원 오류 분류 후 탐색 종료·프레임 해제, 합성 오류 테스트 |
| 선택 폴더 자체가 다른 장치의 마운트 루트 | 부모/선택 폴더 device 비교를 순회 전에 추가, 합성 identity 테스트 |

[흐름도](../flow-diagrams/low-resource-folder-actions.mmd) 대응:

| 노드/분기 | 실제 소스 |
|---|---|
| request/budget/kind | `ScanRuntime::begin_inner`, 각 스캔·폴더 명령, `resource_guard::ensure_operation_memory` |
| scan/issue | `StreamingWalk::next`, scan_path/directory/drive/cleanup의 오류 및 제한 처리 |
| index/extract | file_catalog/document_search의 `build_*_guarded`, `document_worker::extract_document/run_bounded` |
| rollback/stop/commit | `index_budget::run_guarded`, SQL transaction RAII, WorkerFailure 반환, COMMIT 뒤 checkpoint |
| capture/plan | `validate_directory_trash_folder`, `FolderActionsState::replace`, `prepare_directory_folder_plan` |
| valid/recheck | `FolderActionsState::claim`, `confirm_directory_folder_plan`, `revalidate_verified_trash_item` |
| trash/result | `trash_actions::execute_verified_items`, 기존 저널/부분 실패 처리, StorageTreemapPanel·App 표시 |

소스에 맞춰 일반 접근 오류의 계속 처리, 문서 도구 실패 rollback, 계획 소진 후 재검증을 도면에 추가했다. 추상화한 노드/분기 대응은 MATCH이며 OS 호출의 원자성을 새로 보장한다는 의미가 아니다.

## 남은 검증과 한계

- **미설치/미릴리스**: `/Applications/BroomSweepy.app`는 기존 v1.6.1. 새 코드 commit/push/tag/배포 없음.
- 새 문서 바이너리와 준비 스크립트/Tauri externalBin 및 macOS/Windows CI 검사 추가. 이 맥에서는 debug helper와 테스트 빌드만 실행; 최종 서명 앱/설치 파일 안의 helper 실행은 배포 단계에서 다시 확인 필요.
- 설치 앱 전체 프로세스 트리의 최고 footprint, 시스템 압박·swap의 장시간 추세, 실제 OS Trash 및 재시작 복구 E2E는 남아 있음.
- Windows 네이티브 실행/설치 미실행. 기존 NTFS의 중간 Vec/HashMap을 모두 스트리밍으로 바꾼 것은 아님.
- SQLite 정렬 임시 파일은 temp 테이블 페이지 상한만으로 모두 제한되지 않음. 별도 볼륨 여유 공간 감시는 샘플링이고 원자적 디스크 하드쿼터가 아님.
- 폴더 비교는 메타데이터 기반이며 파일 내용의 완전 불변 증명이 아님. OS 경로 기반 이동 직전의 TOCTOU 한계는 남음.
- 실제 Codex와 새 대화형 빈 폴더 도구 계약의 종단 검증은 이전 작업의 미완료 사항으로 유지.

## 마감 기록

- 최종 `cargo test --workspace --quiet -- --test-threads=1`: **240 passed, 0 failed, 1 ignored**. Control17 + Core78 + Core통합9 + Desktop126 + MCP10.
- 프론트 **37 passed**, 타입 및 production frontend build 통과. 설치형 앱 번들 빌드 완료와 구분한다.
- 07:31 KST Data 볼륨 사용 가능 약 **15 GiB**(컴파일 전 약16 GiB). 빌드 산출물의 공간 사용을 포함한 관찰이며 사용자 자료나 기존 캐시를 지우지 않았다.
- 설치된 앱의 plist를 재확인: **1.6.1**, 교체 없음.
- 문서 helper를 최신 core 소스에서 다시 빌드하고 Tauri용 binary에 복사한 뒤 SHA-256 일치 확인 후 전체 테스트 재실행. Tauri build 단계는 `binaries/`를 `target/debug/`로 복사할 수 있으므로 이전 helper가 남지 않게 준비 순서를 지켰다.
- 임시 UI 탭과 이번에 띄운 1421 Vite 서버 종료. 원래 브라우저·기존 앱·다른 서버는 그대로 유지.
