# 선택적 변경 파일 검토 검증

2026-10-08, Apple Silicon macOS. 설치 앱 교체·릴리스·커밋·푸시는 이번 검증에 포함하지 않았다.
개인 폴더 검사나 파일 삭제 없이 Rust 임시 fixture와 합성 UI만 사용했다.

## 실행 결과

| 검사 | 실제 결과 |
|---|---|
| `cargo test -p bloomsweepy-control -p bloomsweepy-mcp` | control 29, MCP 15 통과 |
| `cargo test -p bloomsweepy-core -p bloomsweepy-desktop --lib` | core 98, desktop 280 통과; 기존 opt-in 4 ignored |
| `npm run test:all` | 119/119 통과; 변경 검토 helper 7개 포함 |
| `npm run check` | 통과 |
| `npm run build` | 통과; 기존 단일 JS chunk 500kB 경고 유지 |
| `cargo fmt --all -- --check`, `git diff --check` | 통과 |
| Experience Contract validator | valid, missing/empty/placeholder/error/warning 없음 |
| 독립 소스 검토 | 차단 결함 없음; 폴더 미측정 용량·생략 문구·신원 실패 문서 정합성 수정 확인 |

Rust는 jobs1, incremental off, test/dev debug info off로 실행했다. 변경을 포함한 최신 core/desktop 실행에서
성공했으며, 이전의 컴파일·APFS 테스트 실패를 통과로 집계하지 않았다.

## 실제 파일·런타임 테스트 근거

- 임시 로컬 파일 생성·수정·이름 변경·제거, 폴더 생성·제거·이름 변경, Unix 실행 비트 변경.
- hardlink의 모호한 이름 변경, 같은 이전 이름의 다른 새 파일을 구분.
- linked ancestor·루트 교체·cloud 제외·읽기 실패·중간 취소·개수/경로 상한을 완전 snapshot으로 반영하지 않음.
- 기본 OFF, 설정만 재시작 시 유지, 기록은 비영속; 잘못된/과대 설정 저장소 거부.
- 한 스케줄러의 12회 ON/OFF 재사용과 활동 세대 취소, 늦은 결과 반영 거부.
- 반복 변경은 수동 넘김을 새 기록으로 교체, 최근 기록 250개/1MiB와 페이지 20행 제한.
- 모델용 projection의 전체/이전 경로·내부 ID·로컬 오류 제거, 정확한 루트 일치, 폴더 용량 null.

첫 mac 실행에서 APFS가 invalid UTF-8 파일 생성을 `Illegal byte sequence`로 거부하여 테스트 하나가 실패했다.
실제 실행 속성 테스트는 유지하고, native 경로의 lossy 문자열 충돌/거부는 순수 경로 검증으로 분리했다.
실제 비 UTF-8 파일 캡처 fixture는 Linux에서만 실행한다. 이 맥에서 Linux 실행을 했다는 뜻은 아니다.

## 실제 UI 관찰

`file-review-fixture.html`은 제품 패널에 합성 metadata만 주입하고 실제 파일/OS 작업 0회를 화면에 표시한다.
CUA를 통해 다음을 관찰했다.

- 기본 OFF·폴더 미선택은 사용 스위치 비활성. 합성 폴더 선택 후 ON은 기준 생성 상태와 0 변경 기록.
- 넘김 후 `검토 필요 5 → 4`, `넘김 1 → 2`; 목록에 직접 넘김과 규칙으로 넘김이 구분됨.
- 같은 파일 재변경 후 검토 필요 5로 복귀하고 새 변경/크기·시각이 표시됨(5초 상태 조회).
- 전체 45개에서 `1–20 / 45 → 21–40 / 45` 페이지 이동.
- 검사 미완료는 이전 완전 검사 유지와 더 작은 로컬 폴더/재시도 안내. 설치 API 미연결은 조작 비활성.
- 보관·표시 상한은 일부 생략/전체 목록 아님을 명시. 폴더는 미측정 용량.
- 1280px와 390px에서 document scrollWidth가 각각 viewport와 동일. 긴 한글/영문 경로의 가로 넘침 없음.
- 글꼴 상태 loaded; 버튼/select 14px·44px. 스위치 클릭 label/control의 높이도 44px 이상.
- Tab 이동의 실제 select focus outline 2px를 관찰. reduced-motion은 CSS delta 검토; OS 설정 전환 실측은 안 함.

증거: [desktop](screenshots/2026-10-08-file-review-desktop.jpg), [390px](screenshots/2026-10-08-file-review-mobile.jpg).
스크린샷은 viewport 캡처이며 목록 전체와 모든 오류 상태를 한 이미지로 증명하지 않는다.

## NOT RUN / 제한

- 새 설치형 앱에서 실제 폴더 선택·IPC·5/15/30분 wall-clock 종단 검사: NOT RUN. 설치 앱은 기존 개발본.
- 실제 AI CLI/MCP가 이 신규 capability를 호출하는 종단 대화: NOT RUN. 공유 typed dispatcher/discovery/projection은 테스트함.
- Windows 컴파일·설치·원격 볼륨 실측: NOT RUN. Windows 분기 소스 검토와 기존 cross-platform helper 테스트만 수행.
- 장시간 메모리 soak/시스템 종료 지연·단일 OS I/O 강제 시간 제한: NOT RUN/보장하지 않음.
- light theme·실제 screen reader·OS reduced-motion 전환·전체 Web Interface Guidelines 감사: NOT RUN.
- DESIGN.md 대화형 보조 lint: NOT RUN, headless 무출력을 성공으로 집계하지 않음.

주기 사이 잠깐 존재한 파일과 동일 metadata로 복원된 내용 변경은 놓칠 수 있다. 이 검증은 악성코드 탐지율,
안전 판정, 랜섬웨어 방어, 자동 정리/격리의 증명이 아니다.

#tags: file-change-review, macos-tests, bounded-metadata, synthetic-ui, privacy-scope
