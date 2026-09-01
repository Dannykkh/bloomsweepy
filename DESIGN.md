---
name: BroomSweepy Cross-Platform Desktop
colors:
  canvas: "oklch(0.115 0.018 276)"
  canvas-elevated: "oklch(0.155 0.018 268)"
  surface: "oklch(0.205 0.016 265 / 0.72)"
  surface-strong: "oklch(0.235 0.016 265 / 0.9)"
  line: "oklch(0.91 0.018 265 / 0.14)"
  text: "oklch(0.96 0.008 265)"
  text-muted: "oklch(0.72 0.018 265)"
  primary: "oklch(0.58 0.19 258)"
  on-primary: "oklch(0.985 0.004 260)"
  sweep-violet: "oklch(0.66 0.19 305)"
  success: "oklch(0.77 0.18 142)"
  warning: "oklch(0.78 0.16 72)"
  danger: "oklch(0.68 0.21 27)"
typography:
  title:
    fontFamily: "Pretendard Variable, Pretendard, sans-serif"
    fontSize: 1.5rem
    fontWeight: 700
    lineHeight: 1.2
    letterSpacing: "-0.02em"
  heading:
    fontFamily: "Pretendard Variable, Pretendard, sans-serif"
    fontSize: 1.125rem
    fontWeight: 650
    lineHeight: 1.35
    letterSpacing: "-0.01em"
  body:
    fontFamily: "Pretendard Variable, Pretendard, sans-serif"
    fontSize: 0.875rem
    fontWeight: 450
    lineHeight: 1.5
  metric:
    fontFamily: "JetBrains Mono, Pretendard Variable, monospace"
    fontSize: 0.875rem
    fontWeight: 600
    lineHeight: 1.3
rounded:
  sm: 8px
  md: 12px
  lg: 18px
  xl: 24px
  pill: 999px
spacing:
  xs: 4px
  sm: 8px
  md: 16px
  lg: 24px
  xl: 32px
components:
  button-primary:
    backgroundColor: "{colors.primary}"
    textColor: "{colors.on-primary}"
    rounded: "{rounded.md}"
    padding: 12px
  glass-panel:
    backgroundColor: "{colors.surface}"
    textColor: "{colors.text}"
    rounded: "{rounded.lg}"
    padding: "{spacing.lg}"
  sidebar-active:
    backgroundColor: "{colors.primary}"
    textColor: "{colors.on-primary}"
    rounded: "{rounded.sm}"
    padding: 10px
---

## Overview

BroomSweepy는 저장공간을 관찰하고 안전하게 정리하는 데스크톱 데이터 도구다. 기존 SwiftUI 앱의 어두운 글래스 재질을 보존하되, 장식적인 글래스모피즘보다 사용 순서, 저장공간 트리맵과 안전 판단을 먼저 읽히게 한다. 시각적으로 감수하는 한 가지 위험은 파란색에서 보라색으로 이어지는 기존 브랜드 스윕이며, 브랜드 마크와 선택 상태에만 제한한다.

## Product Grounding

- 실물 은유: 디스크 플래터, 파일 인덱스, 스캔 빔, 안전 봉인.
- 사용 상황: 저장공간 부족을 발견한 사용자가 원인을 찾고, 삭제 전에 근거를 검토한다.
- Interface Mode: `Data Instrument`가 주 유형이고 긴 스캔 중에는 `Waiting State` 계약을 적용한다.
- Density: controlled-medium. 숫자와 결과는 조밀하게, 삭제 결정 주변은 여유 있게 둔다.

## Colors

- `canvas`와 `canvas-elevated`가 창의 깊이를 만든다. 순수 검정은 사용하지 않는다.
- `primary`는 선택, 포커스, 주 스캔 행동에만 쓴다.
- `primary → sweep-violet` 스윕은 저장공간 링과 브랜드 마크에만 쓴다. 일반 텍스트와 여러 버튼에 반복하지 않는다.
- `success`, `warning`, `danger`는 상태 의미에만 사용하고 아이콘 또는 텍스트 라벨을 함께 제공한다.
- 레거시 폴백은 CSS 변수 바로 앞에 sRGB 값을 선언한다. 신규 정본 값은 위 `oklch()` 토큰이다.

## Typography

한글과 라틴을 한 목소리로 유지하기 위해 본문과 제목 모두 실제 로드한 Pretendard Variable을 사용한다. 경로, 크기, 진행률처럼 자릿수 비교가 중요한 값만 JetBrains Mono를 쓴다. 기능 중심 데이터 도구이므로 serif와 과도한 디스플레이 타이포를 사용하지 않는다. 사용자에게 보이는 라벨, 설명, 경로, 표 셀, 버튼 글자는 14 CSS px(`0.875rem`) 미만으로 줄이지 않는다. 더 좁은 창에서는 글자를 축소하지 않고 줄바꿈, 재배치, 세로 스크롤로 공간을 확보한다.

## Spatial Model

- 넓은 창: 216px 고정 사이드바 + 유연한 메인 작업면.
- 전역 메뉴는 `용량 관리`, `파일 이름 찾기`, `문서 내용 찾기`, 선택형 `AI 도우미`, `설정`만 둔다. 큰 파일·중복·정리 후보는 용량 관리 내부 결과 탭이다.
- 메인 첫 화면: 폴더 선택 → 폴더 용량 지도 → 큰 파일·중복의 3단계 안내 → 저장공간 트리맵. 지도가 준비되면 안내를 축약해 실제 비례 사각형이 첫 화면에 들어오게 한다.
- CLI 연결과 권한은 기본 용량 관리에 섞지 않고 `AI 도우미`에 둔다. 앱 안 대화 공급자가 없는 상태에서 가짜 채팅 입력창을 만들지 않는다.
- 카드의 반복보다 얇은 선과 명도 차를 우선한다. 트리맵 작업면만 강한 유리 셸을 사용한다.
- 920px 미만에서는 사이드바를 72px 아이콘 레일로 압축한다. 680px 미만에서는 오버레이 내비게이션으로 교체한다.

## Components

- Glass panel: 한 계층만 사용한다. 패널 안에 다시 블러 패널을 중첩하지 않는다.
- Primary button: 흐름마다 하나만 강조한다. `:active`에서 `scale(0.98)`을 사용한다.
- Navigation row: 아이콘, 이름, 짧은 설명 또는 상태값 순서다. 활성 항목은 정확히 하나다.
- File result row: 이름과 경로는 좌측, 크기와 날짜는 우측 정렬한다. 선택은 체크박스와 배경을 함께 쓴다.
- Danger action: 삭제 대상 수와 회수 가능 여부를 버튼 인접 영역에 표시한다.

## State Contracts

- Loading: 300ms 전에는 표시하지 않는다. 이후 현재 단계, 처리 항목 수, 진행률, 취소를 노출한다.
- Empty: 오류처럼 보이지 않게 완료 상태와 다음 스캔 행동을 함께 보여준다.
- Permission: 거부된 경로와 다시 선택하는 행동을 설명한다.
- Error: 실패 원인, 영향 범위, 재시도 행동을 한 블록에서 제공한다.
- Success: 찾은 항목 수와 용량, 스캔 기준 시각을 고정한다.
- Stale: 마지막 스캔 시각을 표시하고 파일시스템 변경 후 다시 스캔하도록 안내한다.

## Motion

- Engine ladder 1: CSS `transform`과 `opacity`만 사용한다.
- 첫 진입 시 브리핑, 링, 결과가 60ms 간격으로 한 번 등장한다.
- 저장공간 링은 스캔 결과가 바뀔 때만 전환한다. 무한 회전이나 지속 글로우는 사용하지 않는다.
- `prefers-reduced-motion: reduce`에서는 모든 entrance와 크기 전환을 제거한다.

## Platform Adaptation

- macOS: 창 배경 투명도와 WKWebView 위에 네이티브 vibrancy가 허용되면 사용한다.
- Windows: WebView2 CSS 블러를 기본으로 하고, 창 재질은 지원되는 경우에만 적용한다.
- OS별 제목 표시줄과 창 버튼 위치는 네이티브 관례를 따른다. macOS 신호등 버튼을 Windows에 복제하지 않는다.
- 기능 가용성은 숨기거나 거짓 성공으로 표시하지 않고 플랫폼 capability로 설명한다.

## Copy Rules

- 행동은 결과로 이름 붙인다: `스캔 시작`, `스캔 취소`, `휴지통으로 이동`.
- `최적화`, `안전` 같은 표현은 실제 판정 근거가 함께 있을 때만 쓴다.
- 삭제 전에 대상, 예상 확보 용량, 복구 위치를 평문으로 보여준다.
- 운영체제 이름은 하드코딩된 마케팅 카피가 아니라 capability 설명에만 사용한다.

## Accessibility

- 모든 아이콘 버튼에 접근 가능한 이름을 제공한다.
- 키보드 포커스는 인지 가능한 2px 상당의 한 겹 선으로 표시하고 색 대비만으로 상태를 구분하지 않는다. 입력 묶음의 컨테이너와 내부 입력에 포커스 선을 중복해서 그리지 않는다.
- 진행 상태는 `role=status`, 오류는 `role=alert`로 알린다.
- 본문 텍스트와 컨트롤은 WCAG AA 대비를 목표로 하며 반투명 표면에서도 실제 합성색을 검증한다.

## Performance

- 스크롤 컨테이너에는 블러를 중첩하지 않는다.
- 파일 레코드는 Rust에 보관하고 UI에는 집계와 페이지 단위 결과만 전달한다.
- 폰트는 사용 weight만 로드하고, 아이콘은 트리 셰이킹 가능한 SVG 컴포넌트를 사용한다.
- 한 번에 렌더하는 파일 행 수를 제한하고 긴 목록은 가상화 또는 페이지네이션한다.
