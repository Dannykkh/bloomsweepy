# System Map

> 프로젝트 구성도와 리포트/코드맵 지식 표면을 함께 보는 Code Wiki 뷰입니다.


## Runtime Ownership

```mermaid
flowchart TB
    App[TermSnap App]
    MainVm[MainViewModel]
    Project[ProjectSessionViewModel / Main Tab]
    SubTabs[SubSessions]
    Local[LocalTerminalViewModel]
    Chat[ChatSessionViewModel]
    Pairing[PairingSessionViewModel]
    Discussion[DiscussionSessionViewModel]
    Report[ReportPanelViewModel]
    Listener[AICLIResponseListener]
    Scheduler[ProjectTaskSchedulerService]
    ProjectStore[(project/.termsnap)]
    ExternalOrch[(project/.orchestrator)]

    App --> MainVm
    MainVm --> Project
    Project --> SubTabs
    Project --> Listener
    Project --> Scheduler
    Project --> ProjectStore
    Project --> ExternalOrch
    SubTabs --> Local
    SubTabs --> Chat
    SubTabs --> Pairing
    SubTabs --> Discussion
    SubTabs --> Report
```

## Code Knowledge Surface

```mermaid
flowchart LR
    Source[Source files] --> CodeMap[CodeMapService]
    Docs[Docs and handoffs] --> CodeMap
    Memory[Project memory] --> Wiki[Code Wiki]
    Guides[Curated guides] --> Wiki
    CodeMap --> Wiki
    CodeMap --> Graph[Knowledge Graph]
    Wiki --> Report[Report tab]
    Graph --> Report
    CodeMap --> Agents[AI CLI navigation]
```

## Current CodeMap Shape

- Projects: 0
- Files: 325
- Documents/assets: 41 / 295
- Routes/API/UI signals: 0 / 0 / 89

## Main Implementation Areas

| Area | Files | Why it matters |
|------|-------|----------------|
| [apps-desktop](../apps-desktop.md) | 41 | .ts×16, .tsx×14, .rs×11 |
| [views](../views.md) | 39 | WPF View/Window/Panel |
| [services](../services.md) | 22 | 서비스 클래스/메서드 |
| [crates-bloomsweepy-core](../crates-bloomsweepy-core.md) | 11 | .rs×11 |
| [apps-bloomsweepy-mcp](../apps-bloomsweepy-mcp.md) | 3 | .rs×3 |
| [broomsweepy](../broomsweepy.md) | 2 | .swift×2 |
| [broomsweepy-models](../broomsweepy-models.md) | 2 | .swift×2 |
| [crates-bloomsweepy-control](../crates-bloomsweepy-control.md) | 1 | .rs×1 |
| [viewmodels](../viewmodels.md) | 1 | ViewModel 클래스/속성/명령 |
