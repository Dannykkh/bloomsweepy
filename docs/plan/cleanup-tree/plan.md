# 정리 후보 트리

2026-10-05 · source: codex · WorkPM native 5단계 장부

## 승인과 범위

사용자는 삭제 후보 전용 페이지, 하위 트리, 전체/개별 선택, 상위 선택의 하위 자동 선택을 제안하고 “그럼 다음을 진행”으로 구현을 승인했다. 기존 정리 후보 탭을 확장하며 실제 사용자 파일 삭제·앱 설치·릴리스는 이번 검증에 포함하지 않는다. 기존 dirty/untracked 작업을 보존한다.

## 1 — 조사와 선택

기존 assistant workspace는 폴더 탐색 때 선택과 revision을 교체한다. StoredReports.directory 또한 단일 지도 generation이다. 둘을 트리 펼침 상태로 재사용하면 이전 선택과 지도 결과가 사라진다.

| 대안 | 목표 적합 | 안전성 | 노력 효율 | 합계 |
|---|---:|---:|---:|---:|
| 앱 소유 bounded 트리 상태 + 기존 검사/휴지통 엔진 | 5 | 5 | 3 | 13 |
| assistant workspace를 탐색마다 교체 | 2 | 3 | 5 | 10 |
| 프런트가 경로와 삭제 목록을 추론 | 3 | 1 | 4 | 8 |

첫 대안을 채택한다. 검사 결과는 삭제 안전성 판정이 아니다. 기존 시스템 캐시 후보의 특수한 허용 정책은 일반 폴더 트리에 섞지 않는다.

## 2 — 도면과 계약

[행동 흐름](../../flow-diagrams/cleanup-tree.mmd), [경험 계약](../../design-refs/2026-10-05-experience-cleanup-tree.md), [레이아웃](../../design-refs/2026-10-05-layout-cleanup-tree.md).

현재 directory generation 또는 assistant session/revision으로 트리를 연다. raw path는 선택·승인 권한이 아니다. opaque node ID와 selectionRevision을 사용한다. 선택은 서버의 include/exclude 규칙이다. 상위 선택은 미전개 하위 항목까지 상속되지만, 하위 제외가 있으면 상위 자체를 이동 대상으로 만들지 않는다. 중복 없는 frontier만 검토하며, 부분/누락 분기를 안전하게 재구성할 수 없으면 검토를 거부한다.

전체 폴더 이동은 보호 하위 항목을 포함하지 않는 strict 재검사를 거친다. generation/revision, 루트 신원, 노드 신원, TTL, 취소, 단발 계획을 검증한다. 최종 실행은 main Webview의 사용자 확인만 가능하며 AI/MCP에는 승인 도구를 추가하지 않는다.

## 3 — 책임과 구현

- Backend worker: 새 cleanup_tree.rs, assistant source snapshot helper, core의 새 strict helper. 기존 정책은 변경하지 않는다.
- UI worker: 새 panel/CSS/types/helper/fixture/tests. IPC는 props로 분리한다.
- Lead: lib/bridge/App/지도·AI 진입점/i18n, 문서/기억, 실검증과 완료 판정.

트리 workspace는 하나, 유효 시간 5분, branch 16개·node 2048개·경로 8MiB·페이지 50개·선택 대상 100개 이하를 목표로 한다. 프런트 렌더는 200개 이하이며 더 보기로 제한을 명시한다. 상세 한도는 구현 정본을 따른다.

## 4 — 검증

필수: 상위 선택/하위 제외/재선택, 보호 항목, partial/미측정 용량, 폴더 중복 제거, 제한 초과, stale source/신원/selectionRevision/TTL, 반복 승인, 취소, 지도 generation 보존, 실제 앱 진입점, i18n placeholder, keyboard/focus/compact/reduced-motion.

사용자 파일을 OS 휴지통에 옮기는 테스트는 하지 않는다. 임시 fixture의 검토와 mock 실행 경로로 살아남아야 하는 하위 항목을 검증한다. native 설치형/Windows 검증은 별도이며 브라우저 fixture를 실제 OS 검증으로 보고하지 않는다. 디스크 여유 확인 후 CARGO_INCREMENTAL=0을 사용한다.

## 5 — 현재 상태

조사·설계·backend/UI·기존 페이지 통합을 완료했고 worker 소유권을 반환받았다. Rust320개·프런트51개 검사, Clippy, 프런트 빌드와 합성 화면 검증을 통과했다. 독립 안전 리뷰의 지적을 수정했다. [실제 QA와 NOT RUN](../../qa/2026-10-05-cleanup-tree.md)이 완료 판정의 근거다. 설치된 앱의 새 native IPC/OS Trash 및 Windows·장시간 자원 검증은 별도 다음 단계이며 배포 완료로 보고하지 않는다.
