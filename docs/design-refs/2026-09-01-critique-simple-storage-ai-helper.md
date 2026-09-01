# Critique: 쉬운 용량 관리 후보

## Candidate A — Guide First Map

- Screenshot: `.termsnap/design-candidates/simple-storage-a.png`
- 첫 화면에서 3단계와 다음 행동이 한 줄로 이어지고 지도 헤더와 주요 사각형이 함께 보인다.
- 전역 메뉴가 5개로 줄어 용량 기능의 관계가 선명하다.
- 선택: 사용법 설명과 지도 발견성의 균형이 가장 좋다.

## Candidate B — Split Navigator

- Screenshot: `.termsnap/design-candidates/simple-storage-b.png`
- 단계 안내가 고정된 좌측 pane이라 순서는 분명하다.
- 작업면 안에 다시 내비 pane이 생겨 전역 사이드바와 경쟁하고, 용량 순위가 첫 화면에서 사라진다.
- 탈락: 사용자가 지적한 “박스와 영역이 너무 많다”는 문제를 다른 형태로 반복한다.

## Candidate C — Compact Command

- Screenshot: `.termsnap/design-candidates/simple-storage-c.png`
- 지도 면적과 결과 이동이 가장 크고 단순하다.
- 3단계 관계가 사라져 처음 쓰는 사용자가 지도와 정밀 검사의 차이를 이해하기 어렵다.
- 탈락: 숙련 사용자에게는 빠르지만 현재 사용자 문제인 사용법 부재를 충분히 해결하지 못한다.

## Preserve

- 후보 A의 5개 전역 메뉴, 4개 용량 탭, 3단계 한 줄 안내, 전폭 지도 순서를 보존한다.
- 실제 구현에서는 프로토타입 수치를 사용하지 않고 런타임 보고서만 표시한다.
- 760×600에서 단계가 세로로 바뀌어도 순서와 한 개의 primary CTA를 유지한다.

## Remaining Verification

- 실제 Tauri 기본·최소 창에서 첫 뷰포트와 세로 스크롤을 캡처한다.
- 키보드로 용량 탭, 폴더 선택, 트리맵 폴더, AI 권한 제어에 접근한다.
- 검색 입력 포커스가 내부 2px 표시 하나만 남는지 확인한다.
