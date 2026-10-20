# Direction comparison — Mac controls

2026-10-21 · USER_SELECTION: “추천대로가자”. 직전 추천인 A · 시스템 설정형을 선택했다. 비교 시안은 그대로 보존하고 A의 실제 제품 반영·검증을 진행한다.

## Candidate sources

공통 source: `apps/desktop/mac-control-design.html`, `src/mac-control-design.tsx`, `src/mac-control-design.css`. 기존 Vite1431 서버를 재사용한다. `candidate=a|b|c`, `wireframe=1`, `narrow=1`을 지원한다. 외부 API/IPC/저장소/실제OS호출 없이 같은5개 null 상태·같은 선택/적용/복원을 비교한다.

| ID | Composition | Hierarchy/base/material | Desktop1280×820 | Narrow390×844 |
|---|---|---|---|---|
| A | 두 기능 묶음의 compact grouped rows | graphite·blue·연결된 낮은 표면, 이름/현재값/조작 정렬 | [A](../qa/mac-design-review-2026-10-21/05-a-desktop.png) | [A narrow](../qa/mac-design-review-2026-10-21/08-a-narrow.png) |
| B | 첫 기능 큰 면적+작은 기능들, 아래 두 패널 | silver·무채색·밝은 분리 표면, 큰 아이콘/타입 | [B](../qa/mac-design-review-2026-10-21/06-b-desktop.png) | [B narrow](../qa/mac-design-review-2026-10-21/08-b-narrow.png) |
| C | 좌 항목 선택·우 상세, 같은 화면에서 조작 | mid-slate·teal·inset inspector, 선택 기능 집중 | [C](../qa/mac-design-review-2026-10-21/07-c-desktop.png) | [C narrow](../qa/mac-design-review-2026-10-21/08-c-narrow.png) |

실제 캡처는00:50–00:53 KST 전후 저장한 원본 바이트를 root가 다시 열어 봤다. 미리보기 팔레트는 비교 전용으로 DESIGN 정본을 대체하지 않는다. 표시된 모의 적용을 실제 Mac 변경으로 표현하지 않는다. 장식효과0, 대표 모션 없음, CSS 상태 피드백과 reduced-motion만 사용한다.

## Recommendation / trade-offs

A를 추천한다. 현재5개에는 전체 항목과 조작이 같은 화면에 있고, 사용자 요구의 직접 조작·Mac식 그룹 구성이 가장 가까웠다. B는 기능 식별이 강하지만 확장자 설정만 크게 강조하는 근거가 약하고 좁은 화면 스크롤이 늘어난다. C는 기능별 상세 분리가 명확하지만 narrow에서 실제 조작부가 아래로 밀리며 현재5개에는 단계가 늘어난다. 선택되지 않은 두 안의 요소를 몰래 혼합하지 않는다.

[A 초안 선택 후 적용 버튼이 활성화된 실제 preview](../qa/mac-design-review-2026-10-21/09-a-pending.png). 브라우저는 이 모의 초안 상태로 열어 두었으며 viewport override는 reset했다. 실제 제품/OS에는 미적용이다.

선택된 방향은 A의 두 기능 묶음, 정렬된 현재값/변경값/적용, 직접 보이는 변경 기록이다. 기존 AppShell 내비게이션과 native glass를 재사용하고 시안의 가짜 sidebar·프리뷰 툴바는 제품에 넣지 않는다. B/C의 레이아웃을 혼합하지 않는다. 실제 OS 설정 변경은 디자인 검증에 포함하지 않고 fixture로 적용/복원/실패를 확인한다. 설치본은 새 빌드 교체 후 읽기 전용으로 확인한다.

#tags: mac-design, selected-a, grouped-controls, production-implementation
