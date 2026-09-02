# Onboarding

> 코드 파일 목록이 아니라 프로젝트를 이해하는 순서입니다.

## Reading Path

1. **프로젝트 경계 확인** — [../projects.md](../projects.md)에서 하위 프로젝트, manifest, 참조 관계를 확인합니다.
2. **사용자 진입점 확인** — [../ui-index.md](../ui-index.md), [../routes-index.md](../routes-index.md), [../api-index.md](../api-index.md)를 먼저 봅니다.
3. **주요 구현 영역 확인** — 아래 카테고리 표에서 파일 수가 큰 영역부터 읽습니다.
4. **세부 위치 찾기** — 필요한 파일은 [../files.md](../files.md)에서 찾고, 각 카테고리 문서의 `(L123)` 라인 힌트로 좁힙니다.
5. **그래프 확인** — 영향 관계가 필요하면 [../GRAPH_REPORT.md](../GRAPH_REPORT.md)와 `graph.html`을 확인합니다.

## Current Shape

- Projects: 0
- Files: 332
- Documents: 41
- Routes/API/UI signals: 0 / 0 / 90

## Areas To Scan First

| Area | Files | Why it matters |
|------|-------|----------------|
| [apps-desktop](../apps-desktop.md) | 44 | .ts×16, .tsx×15, .rs×13 |
| [views](../views.md) | 39 | WPF View/Window/Panel |
| [services](../services.md) | 22 | 서비스 클래스/메서드 |
| [crates-bloomsweepy-core](../crates-bloomsweepy-core.md) | 11 | .rs×11 |
| [apps-bloomsweepy-mcp](../apps-bloomsweepy-mcp.md) | 3 | .rs×3 |
| [broomsweepy](../broomsweepy.md) | 2 | .swift×2 |
| [broomsweepy-models](../broomsweepy-models.md) | 2 | .swift×2 |
| [crates-bloomsweepy-control](../crates-bloomsweepy-control.md) | 1 | .rs×1 |
| [viewmodels](../viewmodels.md) | 1 | ViewModel 클래스/속성/명령 |

## Operating Rule

- CodeMap은 검색량을 줄이는 1차 색인입니다. 답을 바로 단정하지 말고, 필요한 카테고리 문서의 라인 힌트로 실제 파일을 좁혀 읽습니다.
- 그래프와 wiki는 CodeMap에서 파생된 친절한 뷰입니다. 오래된 설명보다 현재 생성된 `codemap/*.md`와 실제 소스가 우선입니다.
