# Program Glossary

> 프로젝트 용어를 한 곳에서 확인하기 위한 Code Wiki 뷰입니다.

- Curated source: not found. Fallback terms were generated from CodeMap categories.

## Core Terms

| Term | Korean | Definition | Identifier |
|------|--------|------------|------------|
| Code Map | 코드맵 | 코드 위치 탐색을 빠르게 하기 위한 자동 생성 인덱스 | codemap/index.md, CodeMapService |
| CodeMap Info | 코드맵 안내 | 코드맵 사용 순서와 작업별 라우터를 담은 자동 생성 안내 | codemap/info.md |
| CodeMap Lint Report | 코드맵 린트 리포트 | CodeMap 문서 링크와 필수 산출물 무결성을 검사한 자동 보고서 | codemap/LINT_REPORT.md |
| Code Wiki | 코드 위키 | CodeMap과 프로젝트 메모리에서 파생한 사람용 프로젝트 가이드 | codemap/wiki/index.md |
| Knowledge Graph | 지식 그래프 | 코드/문서 관계를 그래프로 압축해 탐색하는 보조 지식 계층 | codemap/graph.json, graph.html |
| Report Tab | 리포트 탭 | 프로젝트 문서, 메모리, 코드 위키를 WebView2로 읽는 보고서 표면 | ReportPanelViewModel |
| Project Session | 프로젝트 세션 | 한 프로젝트 메인탭의 런타임 aggregate root | ProjectSessionViewModel |
| views | views | WPF View/Window/Panel | codemap/views.md |
| apps-desktop | apps-desktop | .tsx×12, .rs×8, .ts×7 | codemap/apps-desktop.md |
| services | services | 서비스 클래스/메서드 | codemap/services.md |
| crates-bloomsweepy-core | crates-bloomsweepy-core | .rs×11 | codemap/crates-bloomsweepy-core.md |
| apps-bloomsweepy-mcp | apps-bloomsweepy-mcp | .rs×3 | codemap/apps-bloomsweepy-mcp.md |
| broomsweepy | broomsweepy | .swift×2 | codemap/broomsweepy.md |
| broomsweepy-models | broomsweepy-models | .swift×2 | codemap/broomsweepy-models.md |
| crates-bloomsweepy-control | crates-bloomsweepy-control | .rs×1 | codemap/crates-bloomsweepy-control.md |

## Usage Rule

- 새 기능을 설계할 때 먼저 이 용어집에서 owner, lifecycle, source of truth를 확인합니다.
- 용어 정의가 충돌하면 `guide/termsnap-domain-dictionary.md`를 먼저 갱신하고 CodeMap을 재생성합니다.
