---
name: BroomSweepy Cross-Platform Desktop
colors:
  canvas: "oklch(0.115 0.018 276)"
  canvas-elevated: "oklch(0.155 0.018 268)"
  surface: "oklch(0.205 0.016 265 / 0.58)"
  surface-strong: "oklch(0.235 0.016 265 / 0.82)"
  surface-soft: "oklch(0.18 0.014 268 / 0.44)"
  line: "oklch(0.91 0.018 265 / 0.16)"
  reflection: "oklch(0.98 0.008 265 / 0.28)"
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
  xxl: 40px
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
  storage-hero:
    backgroundColor: "{colors.surface}"
    textColor: "{colors.text}"
    rounded: "{rounded.xl}"
    padding: "{spacing.xxl}"
  sidebar-active:
    backgroundColor: "{colors.primary}"
    textColor: "{colors.on-primary}"
    rounded: "{rounded.sm}"
    padding: 10px
---

## Overview

BroomSweepy는 저장공간 상태를 한눈에 보여 주고 한 번의 클릭으로 원인을 찾게 하는 데스크톱 데이터 도구다. Rust/Tauri가 유일한 제품 기반이며 기존 SwiftUI 앱은 UI·UX golden master다. 첫 장면은 네이티브 글래스 위의 큰 저장공간 링과 한 개의 주 행동이 지배하고, 실제 결과와 안전 판단은 아래로 갈수록 조밀해진다. 시각적으로 감수하는 한 가지 위험은 파란색에서 보라색으로 이어지는 기존 브랜드 스윕이며 저장공간 링과 브랜드 마크에만 제한한다.

## Product Grounding

- 실물 은유: 디스크 플래터, 파일 인덱스, 스캔 빔, 안전 봉인.
- 사용 상황: 저장공간 부족을 발견한 사용자가 원인을 찾고, 삭제 전에 근거를 검토한다.
- Interface Mode: `Data Instrument`가 주 유형이고 긴 스캔 중에는 `Waiting State` 계약을 적용한다.
- Primary action: 대시보드의 `전체 검사 시작`. 폴더 선택이 필요하면 같은 흐름에서 이어진다.
- Density: 첫 뷰포트는 low, 실제 결과는 controlled-medium. 삭제 결정 주변은 다시 여유 있게 둔다.

## Colors

- `canvas`와 `canvas-elevated`가 창의 깊이를 만든다. 순수 검정은 사용하지 않는다.
- `primary`는 선택, 포커스, 주 스캔 행동에만 쓴다.
- `primary → sweep-violet` 스윕은 저장공간 링과 브랜드 마크에만 쓴다. 일반 텍스트와 여러 버튼에 반복하지 않는다.
- `success`, `warning`, `danger`는 상태 의미에만 사용하고 아이콘 또는 텍스트 라벨을 함께 제공한다.
- 레거시 폴백은 CSS 변수 바로 앞에 sRGB 값을 선언한다. 신규 정본 값은 위 `oklch()` 토큰이다.

## Typography

한글과 라틴을 한 목소리로 유지하기 위해 본문과 제목 모두 실제 로드한 Pretendard Variable을 사용한다. 경로, 크기, 진행률처럼 자릿수 비교가 중요한 값만 JetBrains Mono를 쓴다. 기능 중심 데이터 도구이므로 serif와 과도한 디스플레이 타이포를 사용하지 않는다. 사용자에게 보이는 라벨, 설명, 경로, 표 셀, 버튼 글자는 14 CSS px(`0.875rem`) 미만으로 줄이지 않는다. 더 좁은 창에서는 글자를 축소하지 않고 줄바꿈, 재배치, 세로 스크롤로 공간을 확보한다.

## Spatial Model

- 넓은 창: 약 220px 고정 글래스 사이드바 + 유연한 메인 작업면. 1280×820을 기본 렌더로 삼는다.
- 기본 전역 메뉴는 `대시보드`, `성능`, `앱 관리`, `공간 정리`, `파일 관리`, `AI 도우미`, `설정`이다. 앱 관리는 독립 메뉴이며 공간 정리의 보조 탭에 중복 배치하지 않는다. 나머지 leaf route는 공간 정리와 파일 관리의 보조 탭으로 보존한다. 설정에서 Docker 관리를 켠 경우에만 AI 도우미 뒤에 `Docker 관리`를 추가한다.
- 메인 첫 화면은 짧은 브리핑, 2:1 비대칭 저장공간 hero, 240px 안팎의 선택 드라이브 링, `이 드라이브 검사`, 네 개의 빠른 실행 순서다. 여러 물리 드라이브가 있으면 시스템 드라이브를 크게 시작하고 나머지는 작은 링 카드로 옆에 둔다. 최근 정리·최근 파일은 첫 장면 아래의 보조 증거다.
- 용량 관리는 폴더 선택과 지도 생성을 한 행동으로 묶고 저장공간 트리맵을 첫 작업면에 둔다. 큰 파일·중복 검사는 지도 아래의 선택 행동 한 개로 분리한다.
- 대화는 `세션 선택 → 대화 대상 → 대화 기록 → 입력창`의 단일 작업면이다. 폴더 대화는 폴더 선택과 읽기 전용 용량 계산부터 시작하고, Docker 대화는 Docker를 대상으로 바로 시작해 폴더 선택을 요구하지 않는다. 기존 세션은 앱 소유 로컬 저장소에서 이어서 연다. 폴더 범위에는 완료된 논리 용량과 해당 드라이브 총용량 대비 비율을 작은 원형 그래프와 정확한 텍스트로 함께 표시한다. 사용할 수 있는 로컬 AI CLI가 없으면 입력을 비활성화하고, 외부 터미널 연결과 권한은 접힌 고급 항목에 둔다.
- Docker 관리는 `설정 > 개발 도구 관리`에서 기본값을 끈다. 꺼져 있으면 CLI 탐색·백그라운드 조회·대시보드 항목·전용 메뉴를 만들지 않는다. 켜면 설정에는 상태 설명과 전용 화면 이동만 남기고, 조건부 `Docker 용량` 화면에서 상태·범주별 사용량·정리 검토·Docker 대화 진입을 처리한다. 대시보드에는 Docker를 섞지 않는다.
- 카드의 반복보다 빈 공간, 얇은 선과 명도 차를 우선한다. 네이티브 vibrancy는 창 전체의 바탕이며 sidebar와 storage hero만 한 단계의 재질 표면을 더한다.
- 대화 제목은 일반 문서 흐름에서 본문과 함께 스크롤한다. 긴 응답이나 키보드 포커스 위에 고정 제목을 겹치지 않으며, 배경·글꼴·대화 기록과 입력 순서는 유지한다.
- 920px 미만에서는 사이드바를 72px 아이콘 레일로 압축한다. 680px 미만에서는 오버레이 내비게이션으로 교체한다.

## Components

- Native glass: OS 창 효과 → 투명 canvas → sidebar/hero의 반투명 표면 순서다. 패널 안에 다시 backdrop blur를 중첩하지 않는다.
- Storage hero: 디스크 사용률·남은 용량·검사 상태·primary 하나를 포함한다. 작은 드라이브 카드를 선택하면 그 카드만 큰 자리로 교대하고, 실제 스캔 루트는 `이 드라이브 검사`를 눌렀을 때 확정한다. 건강 점수나 서로 겹치는 결과 용량을 합산하지 않는다.
- Quick action: 44px 이상의 전체 버튼, 큰 아이콘, 결과형 이름, 한 줄 설명 순서다. 카테고리 색은 아이콘과 작은 상태에만 쓴다.
- Performance instrument: 공통 `시스템 성능` 제목 아래 CPU와 메모리를 같은 크기의 원형·같은 폭 pane으로 나란히 둔다. CPU는 blue-violet, 메모리는 blue이며 정확한 퍼센트와 RAM used/total·사용 가능·스왑을 함께 표시한다. `앱 메모리 정리`는 macOS에서 BroomSweepy 자신의 malloc 영역만 반환하고 allocator가 보고한 바이트만 결과로 표시한다. 아래 프로세스 행은 이름·PID·CPU·resident memory·종료 가능 여부를 보여 주며 40개 이하로 제한한다. 사용률을 메모리 압력 등급으로 바꾸거나 시스템 전체 여유 메모리 변화를 `확보량`으로 만들지 않는다.
- Process termination: 일반 GUI 앱 하나의 정상 종료만 별도 확인 후 요청한다. 확인창은 대상 이름과 저장하지 않은 작업 경고를 표시하고 취소에 기본 포커스를 둔다. raw PID, 강제 종료, 자동 폴백, 프로세스 트리 종료는 제공하지 않는다.
- Primary button: 흐름마다 하나만 강조한다. `:active`에서 `scale(0.98)`을 사용한다.
- Navigation row: 32px 색상 아이콘, 이름, 짧은 설명 또는 상태값 순서다. 선택 행은 얇은 틴트이며 활성 항목은 정확히 하나다.
- File result row: 이름과 경로는 좌측, 크기와 날짜는 우측 정렬한다. 선택은 체크박스와 배경을 함께 쓴다.
- Installed apps: 성능 다음의 독립 `앱 관리` 메뉴에서 전체 드라이브 검사 없이 조회한다. 이름·버전·위치와 제거 방식, 검색·50개 페이지를 제공하며 크기 미측정은 0 B와 구분한다. Mac은 전용 제거기 안내 후 앱 본체만 별도 확인하고, 관련 캐시·환경설정은 기본 미선택의 추가 검토로 분리한다. Windows는 정식 제거 화면 연결이며 화면 열림을 제거 완료로 표시하지 않는다.
- Explicit open: 파일 결과와 정리 후보에 `열기`와 `위치 표시`를 구분해 제공한다. 폴더 `열기`는 OS 파일 탐색기, `하위 폴더 탐색`은 앱의 용량 지도다. 실행형/패키지/링크는 실행하지 않고 위치를 표시하며 클라우드/온라인 전용 항목은 열기를 거부한다.
- Danger action: 삭제 대상 수와 회수 가능 여부를 버튼 인접 영역에 표시한다.
- Conversational empty-folder review (v1.7.0): 채팅 안에서 앱 소유 후보 카드 → 선택 조정 → 정확한 경로의 최종 목록 → 별도 확인 버튼 → 항목별 결과를 표시한다. 원문 대화나 모델 응답은 승인이 아니다. 전체 경로는 로컬 카드에만, 모델 후보 페이지는 이름·익명 번호 중심으로 최대 24개만 전달한다. 빈 폴더도 필요할 수 있으며 큰 공간 확보를 약속하지 않는다. 5분 만료·선택 변경·재검사·재시작 시 계획을 무효화한다.
- Treemap actions: 폴더 클릭은 하위 탐색, 파일 클릭은 파일 관리자에서 위치 열기다. 같은 경로의 블록과 순위는 hover·keyboard focus 시 함께 강조한다. 순위 행의 44px 더보기와 우클릭 메뉴에서 추가 작업을 제공하며 파일이나 일반 폴더를 별도 검토·최종 확인 후 휴지통으로 이동한다. 폴더는 하위 항목 전체 이동 경고와 체크박스를 추가한다. 저장공간 헤더는 일반 문서 흐름으로 두고 드릴다운마다 자동 스크롤하지 않는다.

## State Contracts

- Loading: 300ms 전에는 표시하지 않는다. 이후 현재 단계, 처리 항목 수, 진행률, 취소를 노출한다.
- Empty: 오류처럼 보이지 않게 완료 상태와 다음 스캔 행동을 함께 보여준다.
- Permission: 거부된 경로와 다시 선택하는 행동을 설명한다.
- Baseline: 첫 파일 목록은 비교 기준이라고 표시하고, 다음 갱신부터 새로 발견한 파일만 최근 파일로 보여준다.
- Error: 실패 원인, 영향 범위, 재시도 행동을 한 블록에서 제공한다.
- Success: 찾은 항목 수와 용량, 스캔 기준 시각을 고정한다.
- Stale: 마지막 스캔 시각을 표시하고 파일시스템 변경 후 다시 스캔하도록 안내한다.
- Performance stale: 마지막 정상 snapshot을 유지하되 오래된 측정임을 표시하고 모든 종료 행동을 비활성화한다. 첫 CPU sample은 300ms 뒤부터 `측정 준비 중`으로 표시한다.
- Memory clean: 클릭 즉시 중복 실행을 잠그고 `정리 중…`을 표시한다. 완료 뒤 `반환한 앱 메모리` 또는 `이미 반환할 메모리 없음`을 알리고 snapshot을 갱신한다. 실패는 기존 수치를 유지하고 같은 자리에서 재시도할 수 있다.
- External tool: `사용 안 함`, `CLI 없음`, `서비스 중지`, `준비됨`을 구분한다. 정리 중 취소는 이미 완료된 Docker prune을 되돌릴 수 없다는 사실과 부분 완료 결과를 함께 표시한다.

## Motion

- Engine ladder 1: CSS 전환을 우선한다. 기본은 `transform`과 `opacity`이며, 실제 수치 변화의 연속성을 위해 성능 링의 `stroke-dashoffset`만 예외로 허용한다.
- 첫 진입 시 브리핑, 링, 빠른 실행이 60ms 간격으로 한 번 등장한다.
- 드라이브 카드 교대는 전후 위치를 한 번씩 읽는 FLIP 방식으로 실제 카드에 Web Animations `transform`을 460ms 적용한다. 작은 카드는 큰 자리로 커지며 들어오고 기존 큰 카드는 작은 자리로 밀려 축소된다. 폭·높이 자체는 애니메이트하지 않는다.
- 저장공간 링은 선택 또는 런타임 값이 바뀔 때만 전환한다. 무한 회전이나 지속 글로우는 사용하지 않는다.
- CPU·메모리 링은 새 snapshot이 도착할 때 현재 표시 위치에서 목표값까지 900ms ease-out으로 이어진다. 숫자·ARIA 값은 최신 측정값을 즉시 표시하고 초기 렌더는 0부터 재생하지 않는다. 측정 주기는 유지하며 프레임마다 React 상태를 갱신하지 않는다. reduced-motion에서는 링도 즉시 최종값으로 표시한다.
- `prefers-reduced-motion: reduce`에서는 모든 entrance와 크기 전환을 제거한다.

## Platform Adaptation

- macOS 메뉴 막대: Swift 기준의 상주 아이콘과 선택적 RAM % 제목, CPU·RAM·시스템 디스크 팝오버. Rust에서 AppKit 네이티브 폰트·의미색·팝오버 재질을 사용하며 웹 폰트/카드 규칙을 기계적으로 적용하지 않는다. 별도 WebView 없이 10초 집계 측정, 창 닫기는 숨김·종료는 별도, 표시 끔은 숫자만 숨긴다.
- macOS 직접 배포 빌드: 투명 WKWebView와 `underWindowBackground` native vibrancy를 사용한다. 이 경로는 Tauri의 macOS private API를 요구하므로 Mac App Store용 빌드에는 사용하지 않고 불투명 CSS 폴백을 둔다.
- Windows: WebView2 CSS 블러를 기본으로 하고, 창 재질은 지원되는 경우에만 적용한다.
- OS별 제목 표시줄과 창 버튼 위치는 네이티브 관례를 따른다. macOS 신호등 버튼을 Windows에 복제하지 않는다.
- 기능 가용성은 숨기거나 거짓 성공으로 표시하지 않고 플랫폼 capability로 설명한다.

## Copy Rules

- 행동은 결과로 이름 붙인다: `스캔 시작`, `스캔 취소`, `휴지통으로 이동`.
- `최적화`, `안전` 같은 표현은 실제 판정 근거가 함께 있을 때만 쓴다.
- 성능 화면은 `CPU 정리`나 `즉시 최적화`라고 쓰지 않는다. 버튼은 `앱 메모리 정리`라고 이름 붙여 자체 allocator 반환임을 구별하고, 바로 아래에서 BroomSweepy 본체에만 적용되는 범위를 설명한다. 앱 종료는 `종료 요청`, 상태는 `사용 중`, `사용 가능`, `마지막 측정`으로 이름 붙인다.
- 삭제 전에 대상, 예상 확보 용량, 복구 위치를 평문으로 보여준다.
- Docker 정리는 운영체제 휴지통을 거치지 않으므로 `복원할 수 없음`, 대상 범주, 7일 보존 기준, 볼륨 제외를 최종 실행 버튼 바로 앞에 표시한다.
- 운영체제 휴지통 비우기는 일반 휴지통 이동과 분리한다. 공간 정리 공통 상단에 열기/비우기를 제공하고, 다른 앱의 항목을 포함한 현재 사용자·연결된 드라이브 휴지통 전체가 실행 시점에 영구 삭제됨을 명시한다. 체크박스와 2분 일회용 확인, 기본 취소 포커스, 배경 inert 모달을 사용한다. 실제 완료나 확보 용량을 추정해 성공으로 표시하지 않는다.
- 운영체제 이름은 하드코딩된 마케팅 카피가 아니라 capability 설명에만 사용한다.

## Accessibility

- 모든 아이콘 버튼에 접근 가능한 이름을 제공한다.
- 키보드 포커스는 인지 가능한 2px 상당의 한 겹 선으로 표시하고 색 대비만으로 상태를 구분하지 않는다. 입력 묶음의 컨테이너와 내부 입력에 포커스 선을 중복해서 그리지 않는다.
- 진행 상태는 `role=status`, 오류는 `role=alert`로 알린다. 메모리 정리는 클릭으로 시작한 진행·완료만 polite live region에서 한 번 알리고 2초 측정 갱신과 섞지 않는다.
- 2초 주기의 CPU·메모리 값은 live region으로 매번 읽지 않는다. 사용자가 누른 새로 고침과 종료 결과만 알리고, 종료 dialog는 취소 기본 포커스·focus trap·Escape·호출 버튼 포커스 복귀를 제공한다.
- 본문 텍스트와 컨트롤은 WCAG AA 대비를 목표로 하며 반투명 표면에서도 실제 합성색을 검증한다.

## Performance

- 스크롤 컨테이너에는 블러를 중첩하지 않는다.
- 정적 grain은 storage hero 한 곳에만 0.03 이하로 허용하고 애니메이트하지 않는다.
- 파일 레코드는 Rust에 보관하고 UI에는 집계와 페이지 단위 결과만 전달한다.
- 대화 공급자에는 전체 경로·파일 내용 대신 최대 24개의 직계 항목 이름과 용량을 포함한 제한된 폴더 요약만 질문 입력으로 전달한다.
- Docker 관리를 끈 상태에서는 Docker 실행 파일을 찾거나 프로세스를 시작하지 않는다. 켠 상태의 사용량과 정리 출력은 크기와 시간 상한을 두고, 정리 명령은 한 번에 하나씩 실행한다.
- 폰트는 사용 weight만 로드하고, 아이콘은 트리 셰이킹 가능한 SVG 컴포넌트를 사용한다.
- 한 번에 렌더하는 파일 행 수를 제한하고 긴 목록은 가상화 또는 페이지네이션한다.
- 성능 sampler는 같은 `System` 인스턴스를 유지하고 polling을 겹치지 않는다. IPC에는 CPU·메모리 상위 집합을 dedupe한 최대 40개 프로세스만 보내며 명령행과 환경변수는 수집하지 않는다.
