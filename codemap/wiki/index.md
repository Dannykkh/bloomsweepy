# Code Wiki

> CodeMap에서 파생한 사람용 프로젝트 가이드입니다. 원본 산출물은 `codemap/*.md`와 `codemap/graph.*`입니다.

## About

BroomSweepy는 저장공간을 분석하고 파일·큰 파일·검증된 중복 파일·문서 안의 문장을 찾는 데스크톱 도구입니다. 기존 macOS SwiftUI 앱은 유지하면서, Windows와 macOS에서 같은 분석 엔진과 UI를 사용할 수 있는 Tauri 2 기반 앱을 함께 개발합니다.

현재 Tauri 앱은 분석 결과에서 사용자가 직접 선택한 중복 파일과 Temp·캐시·AppData 후보를 운영체제 휴지통으로 이동할 수 있습니다. 영구 삭제와 휴지통 비우기는 제공하지 않으며, 제거 프로그램 레지스트리는 계속 읽기 전용입니다.

> 출처: [README.md](../../README.md)

## Project Snapshot

- Project: `bloomsweepy`
- Detected projects: 0
- Source categories: 7
- Files: 240, documents: 29, assets: 210
- Routes: 0, API signals: 0, UI signals: 76

## Start Here

1. [Onboarding](onboarding.md) — 처음 보는 사람이 읽을 순서
2. [Glossary](glossary.md) — 프로젝트 용어와 영-한 매핑
3. [System Map](system-map.md) — 구성도와 소유권 경계
4. [Lessons and Gotchas](lessons-and-gotchas.md) — 이 프로젝트에서 이미 배운 교훈과 실패
5. [CodeMap index](../index.md) — AI CLI용 정밀 탐색 색인
6. [Graph report](../GRAPH_REPORT.md) — 그래프 생성 결과와 주요 연결

## Fast Maps

| Need | Open |
|------|------|
| 용어집 | [glossary.md](glossary.md) |
| 구성도 | [system-map.md](system-map.md) |
| 전체 구조 | [../index.md](../index.md) |
| 파일 위치 | [../files.md](../files.md) |
| 문서/핸드오프 | [../documents.md](../documents.md) |
| 라우트/엔드포인트 | [../routes-index.md](../routes-index.md) |
| API 호출/HTTP | [../api-index.md](../api-index.md) |
| UI 컴포넌트/View | [../ui-index.md](../ui-index.md) |

## Main Areas

| Area | Files | Why it matters |
|------|-------|----------------|
| [views](../views.md) | 36 | WPF View/Window/Panel |
| [apps-desktop](../apps-desktop.md) | 21 | .tsx×10, .rs×6, .ts×5 |
| [services](../services.md) | 21 | 서비스 클래스/메서드 |
| [crates-bloomsweepy-core](../crates-bloomsweepy-core.md) | 11 | .rs×11 |
| [broomsweepy](../broomsweepy.md) | 2 | .swift×2 |
| [broomsweepy-models](../broomsweepy-models.md) | 2 | .swift×2 |
| [viewmodels](../viewmodels.md) | 1 | ViewModel 클래스/속성/명령 |

## Project Memory

- Relevant gotchas: 0
- Relevant learned patterns: 0
- 세부 내용은 [Lessons and Gotchas](lessons-and-gotchas.md)에서 확인합니다.
