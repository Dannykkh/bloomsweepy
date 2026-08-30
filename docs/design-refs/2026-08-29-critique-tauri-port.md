# Critique: Existing BroomSweepy Direction

## Observed Reference

- Source: `docs/promo.mp4`, 1440×920 근방의 다크 테마 대시보드 장면.
- 관찰: 좌측 과업 내비게이션, 상단 건강 상태, 중앙 저장공간 링, 두 개의 주요 버튼, 하단 네 개의 빠른 행동.

## What to Preserve

- 저장공간 링은 앱의 기능을 즉시 설명하는 고유한 signature다.
- 사이드바의 아이콘, 이름, 짧은 설명, 상태값은 기능 탐색과 시스템 상태를 한 번에 전달한다.
- 어두운 반투명 패널과 얇은 테두리는 청소 도구의 차가운 계기판 인상을 만든다.

## What to Correct During Port

- 기존 화면의 파란색 스캔과 초록색 최적화 버튼이 같은 위계로 경쟁한다. 첫 vertical slice에서는 스캔만 primary로 둔다.
- 하단 네 개의 같은 크기 카드 대신 실제 스캔 요약과 파일 목록을 연결해 정보 흐름을 완성한다.
- 상태색을 색만으로 표시하지 않고 텍스트와 아이콘을 병행한다.
- Windows에서 macOS 창 장식을 모방하지 않고 플랫폼 제목 표시줄을 따른다.

## Preserved Baseline

- 고정 사이드바와 중앙 링의 비율.
- near-black 배경, 한 계층의 dark glass, 얇은 반사 테두리.
- 파란색에서 보라색으로 이어지는 저장공간 스윕.
- 한국어 우선의 촘촘한 데스크톱 타이포.

## Remaining Validation

- Tauri 실제 렌더의 합성 대비와 블러 성능.
- 760×600 최소 창에서 스캔·취소·결과 접근 가능성.
- Windows 125%·150% 배율에서 경로와 숫자 정렬.
- macOS WKWebView 렌더는 macOS 빌드 환경에서 별도 확인.
