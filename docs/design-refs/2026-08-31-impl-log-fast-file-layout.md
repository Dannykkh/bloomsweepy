# Implementation Log: Fast File Search 화면 구조

## Direction Decision

기존 `existing-dark-glass` 방향과 `DESIGN.md`가 이미 사용자 승인된 제품 정본이고 이번 작업은 한 화면의 네이티브/WebView 좌표계 불일치 교정이므로 3방향 후보 렌더를 생략했다. 색·타이포·컴포넌트 외형은 바꾸지 않고 화면 구조와 검증 방법만 다뤘다.

## Diagnosis

- 사용자 캡처: 약 1280px 네이티브 창 안에서 760×600 웹뷰가 렌더됨.
- 구조 신호: 기본 216px 사이드바가 아니라 920px 이하용 72px 레일이 활성화됨.
- 원인: 로컬 E2E 스크립트의 `Emulation.setDeviceMetricsOverride`가 WebView만 축소함.
- 제품 기준: 실제 재실행에서는 `clientWidth=1280`, `clientHeight=820`, `mainClientWidth=1054`로 전체 창을 채움.

## Changes

- 로컬 E2E 스크립트에서 device metrics override를 제거하고 시작 시 `Emulation.clearDeviceMetricsOverride`를 호출하도록 변경.
- 화면 캡처 선택 인수와 viewport·screen·DPR 계측을 추가.
- 좁은 화면은 CDP 에뮬레이션 대신 Win32 네이티브 창 리사이즈로 검증.
- 빈 결과 이전 상태에 항상 스크롤 범위를 요구하던 테스트 조건을 실제 콘텐츠 상태 기준으로 교정.
- `카탈로그`, `색인`, `MFT·USN`, `해시·바이트` 같은 구현 용어를 `파일 목록`, `문서 미리 읽기`, `빠른 읽기`, `내용을 끝까지 비교`로 바꿈.
- `glob`, `ext`, `size` 같은 특수 문법은 기본 화면에서 숨기고 `검색을 더 정확하게 하는 법` 안에서 쉬운 뜻과 함께 제공.

## Render Critique

### 1280×820

- 앱 셸이 전체 client rect를 채우고 검은 빈 영역이 없다.
- 216px 사이드바, 유틸리티 헤더, 검색 명령, 파일 목록 상태, 결과 순서가 명확하다.
- 두 결과만 있는 fixture에서는 하단 여백이 생기지만 이는 미사용 창 영역이 아니라 동일한 canvas이며 구조 결함이 아니다.

### Compact native window

- 72px 아이콘 레일이 실제 창 폭에서만 활성화된다.
- 검색 상태와 필터가 전폭 행으로 재배치되고 `.main-content`의 세로 스크롤로 하단 결과에 접근한다.
- native outer 760×600에서 WebView client는 Windows 창 테두리를 제외한 744×561로 계측됐다.

## Remaining Checks

완료. 추가로 남은 차단 항목은 없다.

## Verification

- TypeScript `tsc --noEmit`: 통과.
- Rust workspace: 67개 통과, 5개 관리자 권한·실제 휴지통 시험은 명시적으로 ignored, 실패 0개.
- 구조화 검색 E2E: 1280×820과 실제 760×600 네이티브 창 모두 통과.
- 화면 경계: 두 크기 모두 app shell과 WebView client rect가 일치하고 수평 넘침 0.
- 컴팩트 스크롤: 760×600에서 main client 600px, scroll 924px로 하단 행동 접근 가능.
- 오류 복구: 잘못된 `glob` 조건을 쉬운 한국어로 설명한 뒤 다음 검색 성공.
- 정식 패키지: MSI와 NSIS 생성 완료, 정식 실행 파일 응답 확인, 원격 디버깅 9334 포트 없음.
