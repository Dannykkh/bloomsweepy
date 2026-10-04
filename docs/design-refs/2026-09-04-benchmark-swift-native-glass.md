# Benchmark: Swift Native Glass Dashboard

## Sources

| Source | Captured at | Use |
|---|---|---|
| `demo-assets/01_dashboard.png` | 2026-09-04 재검토 | 실제 Swift 대시보드의 구도·재질·밀도 |
| `BroomSweepy/ContentView.swift` | 2026-09-04 | `NavigationSplitView`, 카테고리 색, 사이드바 행 |
| `BroomSweepy/Views/DashboardView.swift` | 2026-09-04 | 브리핑, 중앙 링, CTA, 빠른 실행, material |
| `BroomSweepy/Views/SmartCleanView.swift` | 2026-09-04 | scanning → result → cleaning → done 흐름 |
| `docs/assets/screenshots/v1.2.0-dashboard.png` | 2026-09-04 재검토 | 현재 Rust 화면의 비교 기준 |

- 소스 모드: 프로젝트 소유 제품 레퍼런스를 이용한 delta recreation.
- 외부 카피·브랜드·저작물은 사용하지 않는다.
- 모바일 레퍼런스는 없다. 데스크톱 최소 창 760×600을 실제 렌더로 검증한다.

## Visual Observation

### Swift reference

- 약 220px 사이드바는 브랜드와 7개 카테고리를 큰 색상 아이콘으로 분리한다.
- 메인 첫 화면은 얇은 상태 브리핑, 화면 면적을 지배하는 중앙 링, 두 행동, 네 개의 큰 타일 순서다.
- 링 주변에 넓은 빈 공간이 있어 `저장공간을 다루는 앱`이라는 한 문장이 먼저 읽힌다.
- `.ultraThinMaterial`은 창 뒤의 바탕을 받아들이고 얇은 반사선만 더해, 패널을 별도 불투명 카드처럼 보이지 않게 한다.
- 빠른 실행은 아이콘이 먼저 보이고 설명은 짧아 원클릭 도구처럼 인식된다.

### Current Rust reference

- 페이지 제목, 설명, 툴바 설명, 드라이브 표가 연속되어 같은 정보를 여러 번 설명한다.
- 첫 화면에 드라이브 행 네 개와 하단 패널 두 개가 같은 무게로 놓여 중심 장면이 없다.
- 불투명 창 위 CSS 반투명 패널은 실제 배경 혼합이 거의 없어 `글래스`보다 어두운 표처럼 보인다.
- 메뉴 설명이 모든 행에 반복되고 아이콘이 같은 파란 계열이라 카테고리 구분이 약하다.

## Adopt

- 한 화면·한 초점: 중앙 저장공간 링과 `전체 검사 시작`을 첫 장면의 signature로 사용한다.
- 카테고리별 색상 아이콘, 넓은 행 간격, 활성 항목 하나를 채택한다.
- 실제 macOS vibrancy와 단층 반투명 표면을 사용한다.
- 빠른 실행을 큰 아이콘과 짧은 결과형 라벨로 만든다.
- 긴 작업은 Swift Smart Clean처럼 단계와 다음 행동이 명확한 상태 흐름으로 표현한다.

## Adapt

- Swift의 건강 링은 검증된 `디스크 사용률·남은 용량` 링으로 바꾼다.
- `캐시 정리·메모리·보안 검사·유지보수` 타일은 현재 Rust에서 실제 동작하는 `정리 후보·큰 파일·중복 파일·파일 찾기`로 바꾼다.
- Swift의 두 solid CTA 중 하나만 Rust의 primary로 두고, 정리 후보 검사는 secondary glass action으로 낮춘다.
- macOS 전용 재질은 Windows에서 지원되는 창 효과 또는 불투명 폴백으로 변환한다.
- 사이드바의 구현되지 않은 카테고리는 숨기고 기존 leaf route를 `공간 정리`, `파일 관리` 아래 묶는다.

## Avoid

- 근거 없는 건강 점수 95, `좋은 아침` 같은 시간대 추정 카피, 메모리 정리 효과를 표시하지 않는다.
- blue-to-violet sweep을 링과 브랜드 마크 밖의 일반 카드·버튼에 반복하지 않는다.
- 글래스 패널 안에 다시 backdrop blur 패널을 중첩하지 않는다.
- 모든 타일을 같은 파란 원형 배경 아이콘으로 만들지 않는다. 색은 실제 작업 범주를 구분하는 보조 신호로만 쓴다.
- `원클릭 정리`라는 말로 최종 확인과 휴지통 재검증을 생략하지 않는다.

## Benchmark Conclusion

차이는 색상보다 구도와 재질이다. Rust의 실제 데이터와 안전 계약을 유지한 채 `상태 브리핑 → 큰 링 → 한 개의 주 행동 → 네 개의 원클릭 진입점`으로 첫 화면을 재편해야 Swift의 시원한 UX가 살아난다.
