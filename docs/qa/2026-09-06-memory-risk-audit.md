# macOS 메모리 초과 제보 — 읽기 전용 진단

## 판정

사용자가 macOS에서 메모리 사용량 초과 안내 후 앱이 종료됐다고 제보했다. **종료된 앱 이름·시각·당시 작업이 아직 확인되지 않아 직접 원인은 미확정**이다. 이번 작업은 프로세스/OS 진단 기록, 앱 저장소 크기 및 소스 검토만 수행했다. 앱 코드 변경, 재설치, 데이터 삭제, 대량 검사, OOM 재현 테스트는 하지 않았다.

반복 검색 결과를 무한 추가하는 일반적인 누수는 확인하지 못했다. 그러나 **대규모 검사 중 순간 최대 메모리와 색인 디스크 용량을 충분히 제한하지 못하는 경로**를 확인했다. 아래 주요 경로는 미커밋 대화형 빈 폴더 기능 이전의 HEAD에도 존재한다.

## 관찰값과 한계

2026-09-06 21:44–21:47 KST의 관찰:

- 설치된 BroomSweepy는 v1.6.1. 현재 호스트 프로세스 PID 668은 약 1시간 53분 실행 중이었다.
- 시스템 RAM 8 GiB, swap 사용량 약 907 MiB, Data 볼륨 여유 공간 약 15 GiB. 압축·스왑 사용 자체가 누수나 현재 심각한 압박의 증거는 아니다. 후속 `memory_pressure -Q`의 system-wide free percentage는 53%였다.
- `vmmap -summary`의 physical footprint: 호스트 약 28.4 MiB(현재 프로세스 최고 37.2 MiB), 해당 앱 WebContent 약 149.4 MiB(최고 447.4 MiB). GPU/Networking까지 합친 앱 총량은 아니며 이미 종료된 프로세스의 최고 사용량도 아니다. 낮은 현재값으로 제보를 부정할 수 없다.
- 해당 bundle ID의 Application Support/Cache/WebKit 디렉터리는 각각 60/8/288 KiB. 예상 카탈로그·문서 색인 DB는 현재 Cache에 없었다. 지금 확인한 저장소에서 거대한 검색 데이터 누적 증거는 없다.
- 개발용 `target/`은 약 13 GiB다. 사용자 앱의 검색 데이터와 별개이며 삭제하지 않았다.
- 사용자·시스템 DiagnosticReports에서 신규 BroomSweepy/WebKit 메모리 종료 보고서를 발견하지 못했다. 최근 kernel 로그의 idle-exit 이벤트를 OOM 확정 근거로 취급하지 않았다.
- 20:09의 `rustc` 보고서는 **disk writes / Action taken: none**이며 footprint 표본이 약 278→982 MB다. 빌드 부하의 기록이지 앱 메모리 누수·강제 종료의 증거가 아니다. 오전 resource-stability 보고서 역시 disk-write 기록이지 메모리 초과 종료 기록이 아니다.

메모리 판단은 단순 사용률 대신 압축·스왑·메모리 압박을 함께 보아야 한다. [Apple Activity Monitor 설명](https://support.apple.com/en-au/guide/activity-monitor/actmntr1004/mac).

## 소스에서 확인한 위험

### 1. 검사 대기열의 상한 부재 — 우선순위 높음

`crates/bloomsweepy-core/src/{lib.rs:244,directory.rs:170,drive.rs:197,document_search.rs:340,file_catalog.rs:885}`에서 사용하는 jwalk 0.8.1은 병렬 워커 수를 제한하지만 대기 중 항목의 총 바이트를 제한하지 않는다.

해당 의존성의 `src/core/ordered_queue.rs:50`은 `channel::unbounded()`를 사용하고, `src/core/read_dir_iter.rs:132–155`는 디렉터리 결과를 보낸 뒤 하위 디렉터리를 계속 예약한다. 문서 추출이나 SQLite 기록이 느린 소비자일 때 생산된 결과가 앞서 쌓일 수 있다. 화면에 200개 또는 512개만 표시하는 제한은 이 내부 큐를 제한하지 않는다.

이는 반복 횟수에 따른 확정 누수가 아니라 입력 크기·처리 속도 차이에 따른 **최고 메모리 급증 위험**이다. 한 폴더의 항목을 통째로 모으는 구현까지 고려해야 하므로 단순 워커 수 감소만으로 엄격한 상한을 보장하지 않는다.

### 2. SQLite 임시 데이터와 색인에 총량 예산 없음 — 우선순위 높음

- `file_catalog.rs:1812,1826`, `document_search.rs:857,871`: `PRAGMA temp_store = MEMORY`.
- `file_catalog.rs:504,600,836`: 전체 갱신 중 인덱스와 FTS 재구축.
- 기본 카탈로그 200만 항목, 문서 10만 개 제한은 있으나 전체 RAM·DB 바이트 제한은 없다.

화면 반환 수를 100개로 제한해도 SQL 내부 정렬·색인 생성에 필요한 임시 메모리가 100개 분량으로 제한되는 것은 아니다.

### 3. PDF 추출 중 메모리·취소 경계 부족 — 우선순위 높음

`document_search.rs:1005–1019`는 기본 32 MiB 이내 원본을 읽은 뒤 `pdf_extract::extract_text_from_mem`을 완료하고 나서 최종 문자열을 기본 4 MiB로 제한한다. 파서의 압축 해제·객체·중간 문자열 메모리는 이 최종 크기 제한을 따르지 않는다. 추출 호출 도중의 취소 및 별도 메모리 예산도 없다.

의존성 `pdf-extract-0.12.0/src/lib.rs:2259`의 전체 Document/출력 문자열 및 `lopdf-0.42.0/src/object.rs:849`의 압축 해제 `read_to_end`도 확인했다. 원본이 작다고 최고 메모리가 작다는 보장은 없다. 위험 PDF를 생성하거나 사용자 문서를 읽어 재현하지 않았다.

### 4. 보고서·화면은 대체로 교체형이나 큰 응답 복제는 남음

- `apps/desktop/src-tauri/src/lib.rs:374`의 StoredReports는 종류마다 `Option` 한 개를 교체한다. 검색마다 보고서 목록을 계속 늘리지 않는다.
- `lib.rs:1306,1371`의 전체 보고서 복제·IPC 직렬화와 프론트 결과가 동시에 존재할 수 있다. 중복 그룹 수 제한만으로 그룹 내부 파일 수까지 작게 제한되지는 않는다.
- `FastFileSearchView.tsx:144–184`, `DocumentSearchView.tsx:152–175`: 최신 검색 결과로 state 교체, UI 요청은 100개. 검색마다 이전 결과를 추가하지 않는다.
- `usePerformanceMonitor.ts`는 최신 snapshot 하나를 교체하고, Rust sampler도 저장 snapshot 4개로 제한한다.
- `assistant_sessions.rs`는 세션 200개·세션당 메시지 200개·메시지 길이 상한을 가진다. 새 `assistant_tools.rs`는 workspace 16개·기본 후보 200개로 제한한다. 이 새 기능이 무한 저장을 추가했다는 증거는 없다.

## 데이터가 계속 커지는가?

| 작업 | RAM | 디스크 |
|---|---|---|
| 기존 색인에서 검색 | 제한된 최신 결과와 SQL 작업 메모리 | 검색 결과를 매번 영구 누적하지 않음 |
| 드라이브 검사·색인 생성 | 탐색/추출/색인 중 최고 사용량 증가 가능 | 파일 목록·추출 문서 내용·FTS·WAL 저장으로 증가 가능 |
| 같은 파일 재색인 | 다시 처리하는 동안 일시 증가 가능 | 경로 upsert 및 이전 generation 삭제로 단순 중복 행 누적은 방지 |
| 대화 | 선택한 대화 및 제한된 작업 상태 | 제한된 세션·메시지 저장 |

파일 카탈로그의 upsert/이전 generation 삭제는 `file_catalog.rs:788,595`, 문서는 `document_search.rs:539,585`에 있다. SQLite는 삭제된 페이지를 재사용할 수 있어 행이 줄어도 DB 파일 크기가 바로 줄지는 않는다. 카탈로그 명시적 초기화에는 VACUUM이 있지만 정상 갱신마다 실행하지 않는다 (`file_catalog.rs:1787–1792`).

## 다음 구현 및 검증 우선순위

1. 탐색 결과의 엄격한 대기열·바이트 상한과 소비 속도 연동. 거대한 단일 폴더도 스트리밍 처리.
2. SQLite 임시 파일·캐시 예산, 색인 전체 바이트 예산, 여유 디스크 점검, 메모리 압박 시 새 작업 차단/안전 취소. 기존 데이터의 자동 삭제는 별도 동의 없이 하지 않음.
3. PDF 추출의 별도 워커·시간/메모리/출력 제한 및 취소. 단순 추출 후 truncate는 보호책으로 간주하지 않음.
4. 큰 보고서의 IPC 페이지화 및 필요한 상태만 보관. 호스트뿐 아니라 WebContent와 외부 CLI의 메모리를 구분해 계측.
5. 작은 합성 자료부터 느린 소비자 큐 상한, 실행 중 peak와 완료 후 idle, DB/WAL 크기, 취소 후 회복을 검증. 8 GiB 사용자 맥에서 대형 OOM 실험은 피함.

기존 macOS resource-stability 테스트는 4,096개 파일·128개 작은 문서의 반복 전후 RSS를 비교한다. **실행 중 최고 메모리, 거대한 디렉터리, PDF 압축 해제, 실제 WebKit까지 증명한 테스트가 아니다.** 기존 통과 결과만으로 전체 드라이브의 메모리 안전성을 단정하면 안 된다.
