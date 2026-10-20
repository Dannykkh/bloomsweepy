# Architecture index

기존 단일본 [architecture.md](../architecture.md)는 보존한다. 이번 작업의 번호 항목만 이 인덱스로 연결한다.

- [001 — 일반 파일 대화 작업 공간](001-conversational-file-workspace.md): SUPERSEDED by011; 공급자 중립/범위 상한 계승, 버튼 전용·5분 TTL 변경.
- [002 — POSIX 내부 링크의 불투명 폴더 이동](002-opaque-folder-symlinks.md): 원본을 따라가지 않고 링크 자체를 검토/이동.
- [003 — 큰 항목 발견과 지도 공유](003-conversational-storage-map.md): 읽기 전용 용량순 발견, 삭제 판단 분리, 단일 generation 공유.
- [004 — 앱 기능 정본·LLM 조사](004-app-tool-investigation.md): 앱 조회 선택→실제 결과 분석→추가 조회, 동의/최종 승인 분리.
- [005 — 정리 후보 트리](005-cleanup-candidate-tree.md): 상속 체크·하위 제외·중복 없는 frontier·보호 하위 검증. 이 맥 설치형209B 실제 이동/보존/취소 확인.
- [006 — 프로젝트 기록 Git 공유](006-project-record-versioning.md): 사용자가 메모리·대화·docs 공유를 승인. 정제 기록은 추적하고 원시 관찰/중복 상태는 로컬 유지.
- [007 — 네이티브 제목줄·글래스 합성](007-native-window-material.md): Visible OS chrome, 단일 wash, 중복 blur 제거, 웹·Windows opaque 유지. 이 맥 설치형 검증.
- [008 — 대화 중심 작업면](008-conversation-workbench.md): 하단 입력·실제 단계·접힌 근거·Settings/Dialog 공유. 권한과 최종 실행은 유지.
- [009 — 선택적 권한 유지](009-opt-in-permission-lifetime.md): 기본 session, 명시적 remember, 정확한 폴더·기준 재검증과 영속 철회. 계획·최종 승인은 제외.
- [010 — 선택 폴더 검사](010-selected-folder-inspection.md): 검사 스위치 없이 앱 폴더 선택으로 범위 연결. 외부 임의 경로·자동 삭제는 불가.
- [011 — 채팅 삭제 결정](011-conversational-trash-consent.md): 한 번 예/아니오, 단순 직접 명령만 native 확인 생략 opt-in, 무시간 일회용 계획과 실행 재검증.
- [012 — CLI 모델·추론 선택](012-cli-model-selection.md): 입력창 CLI/모델·강도 두 진입점·설정 공유·공급자별 복원, 실제 model/effort·미지원 fallback 차단; 빈 목록·읽기 전용 예시 분리.
- [013 — 공통 파일·MCP 결과](013-common-file-mcp-results.md): 작은 기능 index/상세, 같은 파일 dispatcher, 외부 범위 epoch, 상태 분석 전용 응답.
- [014 — 선택적 변경 파일 검토](014-optional-file-change-review.md): 기본 OFF·단일 폴더 metadata 주기 비교, 넘김≠안전, 비영속 상한·세대 취소·동일 범위 읽기 전용 AI/MCP.
- [015 — 내장 채팅 권한](015-native-chat-access.md): 기본 confirm·명시적 readonly/full, Rust epoch·유한 로컬 볼륨 검색.009/011 계승·외부/OS/본문 별도. 이 Mac 설치형 현재값 복원·native popup stacking 확인.
- [016 — Mac 설정·개발 캐시](016-macos-management.md): Bool5개·선기록/복원·로컬 실행, 공통 읽기 전용 AI, 기존 bounded 정리 후보.
- [017 — 작업 중심 메뉴·설정](017-workspace-navigation.md): 상위 현재 위치, 6분류 mount 보존, 시스템 정리 명시 진입, compact dashboard.
- [018 — Windows 설정 조회·연결](018-windows-management.md): 고정10행 저장값·8개 로컬 진입, 공통 읽기 전용 AI, 실효/지원 미확인, OS 메뉴 분리.
- [019 — Mac A형 그룹 설정](019-mac-control-a.md): 파일표시3/경로정보2, 초안·적용·기록 직접조작, 단일제목·nonsticky·중복메뉴 제거.
