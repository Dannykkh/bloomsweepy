# 구현 기록: 선택형 Docker 용량 관리

## 선택한 경계

- Docker 기능은 `설정 > 개발 도구 관리`에서 사용자가 켜야만 나타난다. 기본값은 꺼짐이며, 꺼진 상태에서는 Docker CLI 탐색과 프로세스 실행을 하지 않는다.
- 켜면 `용량 관리` 다음에 조건부 `Docker 용량` 메뉴를 추가하고, 대시보드에는 Docker 정보를 표시하지 않는다. 설정은 기능 토글과 전용 화면 이동만 소유한다.
- Docker 대화는 폴더 선택 없이 별도 Docker 대상 세션으로 시작한다. 제한된 사용량 요약만 AI CLI에 전달하며, AI는 설명과 우선순위 제안만 하고 실제 조회·미리보기·실행은 BroomSweepy가 담당한다.
- 볼륨 사용량은 원인 파악을 위해 표시하지만 모든 정리 경로에서 제외한다. 가상 디스크 파일을 직접 수정하거나 삭제하지 않는다.
- 정리는 앱이 만든 5분 유효 미리보기, 범주 선택, 복구 불가 확인을 모두 통과한 뒤 고정된 명령만 한 번 실행한다.

허용한 명령:

- `docker builder prune --force --filter until=168h`
- `docker image prune --force --filter until=168h`
- `docker container prune --force --filter until=168h`

`docker system prune`, `--all`, `--volumes`, 공급자가 생성한 셸 문자열은 실행 경로에 없다.

## 구현 구성

- Rust의 `docker_tools` 모듈이 설정 SQLite, Docker 상태 조회, 일회용 미리보기, 실행 직렬화, 취소, 실행 이력, 종료 시 자식 프로세스 회수를 소유한다.
- 명령 출력은 파일당 1 MiB로 제한하고 상태 조회 30초, 정리 단계 10분 제한을 둔다. 앱 종료와 취소 때 추적 중인 자식 프로세스를 종료하고 회수한다.
- 설정 화면은 사용 여부와 `Docker 용량 열기`만 보여준다. 전용 `DockerManagementView`가 범주별 사용량, 상태, 다시 확인, 대화 시작과 정리 검토를 소유한다.
- 대화 세션 저장소 스키마 v2는 `folder`와 `docker` 범위를 구분한다. Docker 세션은 `docker://local` 범위를 사용하며 폴더 선택기를 열지 않는다.
- 확인창은 기본적으로 빌드 캐시만 선택하고 정확한 명령과 볼륨 제외를 실행 직전에 표시한다. 공급자 Markdown과 메타 태그는 저장·표시 전에 평문으로 정규화한다.

## 실제 실행 검증

Windows Tauri 앱과 로컬 Docker Desktop을 사용해 설정, 조건부 메뉴, 전용 화면, Docker 대상 대화 세션과 읽기 전용 정리 미리보기를 검증했다. 검증 동안 최종 정리 버튼은 누르지 않았고 prune 명령은 실행하지 않았다.

| 확인 항목 | 결과 |
|---|---|
| 기능을 끈 상태 | Docker 메뉴 숨김, 설정에 사용량 행 없음 |
| 사용자가 켠 뒤 상태 조회 | CLI 29.5.3, Engine 29.5.3 연결 확인 |
| 메뉴와 대시보드 | `대시보드 → 용량 관리 → Docker 용량` 순서, 대시보드 Docker 항목 없음 |
| 범주 표시 | 이미지·컨테이너·볼륨·빌드 캐시 표시 |
| 정리 미리보기 | 고정 명령 3개, 기본 선택 1개, 확인 전 실행 버튼 비활성 |
| 볼륨 | 사용량만 표시, 정리 선택지 없음 |
| Docker 대화 | 폴더 선택 없이 `scopeKind=docker`, `scopeRoot=docker://local` 세션 생성 |
| 상태 복원 | 검증 전 사용 설정을 보존하고 임시 Docker 대화 세션 삭제 |

검증 산출물:

- `.termsnap/runtime-e2e/docker-opt-in-settings-1280.png`
- `.termsnap/runtime-e2e/docker-workspace-1280.png`
- `.termsnap/runtime-e2e/docker-workspace-760.png`
- `.termsnap/runtime-e2e/docker-chat-target-1280.png`
- `.termsnap/runtime-e2e/docker-chat-target-760.png`

## 접근성과 반응형

- 1280×820과 최소 창 760×600에서 가로 넘침이 없었다.
- 보이는 텍스트의 계산된 최소 크기는 14px였다.
- 최소 창에서 Docker 범주와 행동은 한 열 흐름으로 재배치된다. 대화 대상과 AI 선택기는 컨테이너 폭 700px 이하에서 두 줄로 내려가며 가로 스크롤을 만들지 않는다.
- 확인창은 처음 연 요소로 포커스를 복원하고, 초점을 내부에 가두며, Escape로 닫을 수 있다. 체크박스·스위치에는 명시적 이름을 제공한다.
- 300ms 미만 상태 조회에는 로딩 문구를 표시하지 않아 빠른 응답의 깜빡임을 줄였다.

## 검증 결과

- `npm run check`: PASS
- `npm run build`: PASS
- 프런트 정책 테스트 14개: PASS
- `cargo fmt --all -- --check`: PASS
- `cargo clippy --workspace --all-targets -- -D warnings`: PASS
- `cargo test --workspace`: 133 PASS, 5 IGNORED, 0 FAILED
- Docker 조건부 메뉴·전용 화면·미리보기 E2E: PASS
- Docker 대상 대화 세션 E2E: PASS

제외된 Rust 테스트 5개는 관리자 권한 NTFS 실검사와 실제 Windows 휴지통 이동처럼 로컬 상태를 바꾸는 테스트다. Docker 정리 실행은 안전 검증 범위에서 의도적으로 `NOT RUN`이다.

## 디자인 절차 상태

- Experience Contract: 완료.
- 방향 후보 A/B/C 비교: 완료, A 채택.
- 실제 Tauri 렌더와 최소 창 검토: 완료.
- Product Design Gate: `UNKNOWN`. 현재 환경에서 대응하는 제품 디자인 심사 연결을 찾지 못해 `DESIGN.md`, 로컬 UI 가이드, 실제 렌더 계측으로 대체했다.
- 신규 이미지·폰트·장식 모션: 없음.
