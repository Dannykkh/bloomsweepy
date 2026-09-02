# CodeMap Info

> 코드맵을 처음 여는 사람과 AI CLI를 위한 자동 생성 안내입니다. 수동 편집하지 마세요.

## How To Use This Map

1. `codemap/index.md`에서 관련 카테고리와 보조 색인을 고릅니다.
2. 카테고리 `.md`에서 파일 heading과 `(L123)` 라인 힌트를 확인합니다.
3. 원본 파일의 해당 라인 근처만 먼저 읽습니다.
4. 그래도 부족하면 `rg --json` 또는 `rg --vimgrep` 결과를 파일별로 압축해 후보를 좁힙니다.

## Task Router

- 코드 위치 찾기: `codemap/index.md` -> 관련 카테고리 `.md` -> 원본 파일
- API/엔드포인트 흐름: `codemap/api-index.md`, `codemap/routes-index.md`, 필요 시 `codemap/GRAPH_REPORT.md`
- UI/View/Component 흐름: `codemap/ui-index.md`, 관련 카테고리 `.md`, 필요 시 `codemap/GRAPH_REPORT.md`
- 문서/핸드오프/ADR/메모리: `codemap/documents.md`
- 파일명만 아는 경우: `codemap/files.md`
- 사람용 프로젝트 개요: `codemap/wiki/index.md`
- 요구사항/입력 자료: `codemap/references.md`
- 이미지/스크린샷 자료: `codemap/assets.md`

## Generated Outputs

- `index.md`: 전체 진입점과 카테고리별 파일 수
- `info.md`: 작업별 라우터와 코드맵 사용 규칙
- `LINT_REPORT.md`: CodeMap 문서 링크와 필수 산출물 무결성 검사 결과
- `projects.md`: 하위 프로젝트, manifest, 참조 관계
- 카테고리 `.md`: 파일별 symbol/route/UI hint와 line hint
- `files.md`: 전체 파일 평면 색인
- `documents.md`: 루트 메타, handoff, ADR, chronos, memory 문서 색인
- `references.md`: reference 계열 입력 자료 색인
- `assets.md`: 이미지와 스크린샷 색인
- `routes-index.md`, `api-index.md`, `ui-index.md`: 흐름 추적용 빠른 신호 색인
- `wiki/*.md`: CodeMap과 project memory에서 파생한 사람용 Code Wiki
- `GRAPH_REPORT.md`: graph generation이 실행됐을 때 생기는 선택 산출물

## Current Snapshot

- Project: `bloomsweepy`
- Detected projects: 0
- Source categories: 9
- Files: 332
- Documents: 41
- Assets: 308
- Routes: 0
- API signals: 0
- UI signals: 90

## Source Categories

- `services.md`, `viewmodels.md`, `views.md`, `apps-bloomsweepy-mcp.md`, `apps-desktop.md`, `broomsweepy.md`, `broomsweepy-models.md`, `crates-bloomsweepy-control.md`, `crates-bloomsweepy-core.md`
