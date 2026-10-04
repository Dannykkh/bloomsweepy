# Critique: Swift Reference to Rust Dashboard

## Screenshot-first Observation

### Swift reference

- 5초 이해: 큰 링과 두 행동이 화면 중심을 차지해 저장공간 도구임을 즉시 알 수 있다.
- 위계: 얇은 상태 브리핑 → 링 → CTA → 빠른 실행 순서가 명확하다.
- 구도: 사이드바의 촘촘함과 메인의 넓은 빈 공간이 대비를 만든다.
- 리듬: 상태 패널, 큰 hero, 낮은 빠른 실행 타일이 서로 다른 높이로 이어진다.
- 재질: 실제 데스크톱 배경을 받아들이는 material이 패널 경계를 부드럽게 만든다.

### Current Rust reference

- 5초 이해: 드라이브 표는 읽히지만 사용자가 다음에 무엇을 눌러야 할지 약하다.
- 위계: h1, 설명, 툴바 설명, 패널 제목이 비슷한 크기로 반복된다.
- 구도: 동일한 표면에 행과 카드가 쌓여 signature 장면이 없다.
- 리듬: 드라이브 표와 하단 2개 패널이 controlled-high density로 시작해 여유가 없다.
- 재질: 불투명 웹뷰 배경 때문에 blur가 실제 창 뒤의 색을 거의 받지 못한다.

## Product-safe Corrections

- Swift의 건강 점수와 메모리 수치는 삭제하고 실제 디스크 사용률·남은 용량을 링 중심에 둔다.
- Swift의 캐시·메모리·보안·유지보수 타일은 실제 Rust 기능 네 가지로 바꾼다.
- 두 solid CTA가 경쟁하지 않게 전체 검사만 primary로 둔다.
- 드라이브별 상세와 최근 활동은 hero 아래로 내리되 기능은 제거하지 않는다.
- sidebar의 색은 카테고리 식별에만 사용하고 활성 상태에는 텍스트·배경을 함께 제공한다.

## Baseline-worthiness Gate

- Page Goal: reference 구조는 첫 화면에서 현재 상태와 주 행동을 드러낸다 — 통과.
- Message → Evidence → CTA: 실제 디스크 값 → 링 → 전체 검사로 치환하면 성립한다 — 통과.
- Responsive: 760×600에서는 hero와 타일 재배치가 필요하다 — 구현 후 검증 필요.
- Product specificity: 저장공간 링과 Rust 안전 카피는 BroomSweepy 고유 배치다 — 통과.
- Accessibility/performance: native effect 폴백과 reduced-motion이 필요하다 — 구현 후 검증 필요.

## Preserve

- 약 220px 사이드바, 큰 색상 아이콘, 중앙 링, 넓은 hero 여백, 빠른 실행의 큰 클릭 면적.
- 링에만 제한한 blue-violet sweep과 얇은 반사 edge.
- 대시보드 첫 화면은 낮은 밀도, 결과는 아래로 갈수록 높은 밀도.

## Remaining Verification

- 실제 Tauri 1280×820·760×600 screenshot을 reference와 나란히 본다.
- macOS native vibrancy가 CSS 불투명 레이어에 가려지지 않는지 실제 설치 앱에서 확인한다.
- 키보드만으로 전체 검사, 취소, 네 타일, 보조 내비를 이동한다.
- 일본어·중국어에서 메뉴와 타일이 잘리지 않는지 확인한다.
- 스캔 진행 dock과 hero 상태가 중복되지 않는지 확인한다.
