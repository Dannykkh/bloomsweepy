# Implementation Log: 언어 선택과 앱 아이콘 식별성

## Scope

- 한 설치본 안에서 기본 `English`와 `한국어`, `日本語`, `简体中文`을 즉시 전환하고 로컬에 저장한다.
- 선택 언어를 문서 `lang`과 AI 응답 언어에 반영한다.
- 기존 앱 아이콘의 색과 둥근 사각형을 보존하면서 빗자루 실루엣을 굵게 보정한다.

## Existing Composition Points

- 앱 진입: `apps/desktop/src/main.tsx`
- 설정 화면: `apps/desktop/src/views/SettingsView.tsx`
- 전역 화면 조합: `apps/desktop/src/App.tsx`
- 번들 아이콘: `apps/desktop/src-tauri/icons/`
- Swift 아이콘 원본: `BroomSweepy/Resources/Assets.xcassets/AppIcon.appiconset/`

## Direction Decision

- 신규 화면이나 카드 계층을 만들지 않고 기존 설정 행과 전역 React composition root를 재사용한다.
- 외부 i18n 런타임 의존성 없이 영어·한국어·일본어·중국어(간체) 로컬 사전과 locale context를 추가한다.
- 아이콘은 프로젝트 원본을 편집한 단일 master에서 Windows/macOS 파생 자산을 생성한다.

## Product Design Adapter

- Status: `UNKNOWN`
- Evidence: 설치 및 사용 가능한 플러그인 목록에서 exact Product Design adapter를 확인하지 못했다.
- Fallback: 로컬 `DESIGN.md`, Experience Contract, 실제 렌더·테스트를 사용한다.

## Candidate Render Exemption

- 기존에 승인된 Data Instrument 방향의 국소 설정 행과 브랜드 아이콘 보정이므로 전체 화면 후보 3안은 만들지 않는다.
- 변경 변수는 언어 선택 행, 문자열 locale, 아이콘 실루엣으로 제한한다.

## Verification Plan

- Experience Contract 정적 검증.
- TypeScript check, unit tests, production build.
- English·한국어·日本語·简体中文 선택 저장과 `document.documentElement.lang` 확인.
- 1280×820과 760×600 설정 화면 렌더, keyboard/select 접근성 확인.
- 16px, 32px, 128px 아이콘 실제 파생 렌더 확인.
- Windows Tauri 번들 빌드에서 새 `.ico` 포함 확인.

## Implemented

- `apps/desktop/src/i18n/`에 English 기본 사전, 한국어 원문 키, 전체 일본어·중국어(간체) 사전과 로컬 저장 preference를 추가했다.
- `LanguageProvider`를 기존 React 진입점에 연결하고 모든 주 화면·구성요소의 사용자 문구를 같은 `t()` 계약으로 전환했다.
- 선택 언어를 HTML `lang`, 숫자·날짜 포맷, Windows 트레이 메뉴, AI 응답 언어 요청에 함께 연결했다.
- Rust가 보내는 한국어 진행 설명을 화면에 그대로 노출하지 않고, 작업 종류별 로컬 문구로 표시해 검사 중에도 선택 언어를 유지한다.
- `SettingsView` 첫 영역에 가시적 label과 44px native select를 추가했고, 선택은 이 컴퓨터의 localStorage에만 저장한다.
- 창 제목과 Windows 트레이 tooltip은 릴리스 버전인 `BroomSweepy 1.4.0`으로 맞췄다.
- 기존 남보라색 둥근 타일을 보존하면서 빗자루 손잡이와 솔을 굵게 한 master를 만들고 Tauri·Windows·macOS·모바일 파생 아이콘을 다시 생성했다.
- `README.md`, `README.en.md`, `README.ja.md`, `README.zh-CN.md`에 언어 이동 링크, 언어별 실제 설정 화면, 단일 설치본과 English 기본값을 기록했다.

## Image Generation Prompt

기존 BroomSweepy 아이콘의 남보라색 둥근 타일과 차가운 파란색 계열을 유지한다. 16~32px에서도 먼저 읽히도록 대각선 빗자루를 화면의 중심 대상으로 크게 그리고, 굵은 금색 손잡이와 넓고 밝은 파란 솔을 사용한다. 별 장식은 작은 것 세 개 이하로 줄이고 빗자루와 겹치지 않게 한다. 텍스트와 테두리는 넣지 않으며 타일 바깥 모서리는 투명하게 유지한다.

## Verification Results

- `npm run check`: PASS.
- `npm run build`: PASS. Vite가 500kB 초과 chunk 경고를 남겼지만 빌드는 완료됐다.
- frontend unit tests: 18 PASS. 이 중 언어 preference와 846개 English 키에 대한 일본어·중국어 key/placeholder 동등성 4건을 포함한다.
- 실제 Chromium 렌더: English 기본값, 일본어 재실행 유지, 중국어(간체)·한국어 즉시 전환 PASS.
- 1280×820 및 760×600: document 가로 overflow 없음, native select 키보드 접근 가능, HTML `lang` 동기화 PASS.
- `cargo fmt --all -- --check`: PASS.
- `cargo test -p bloomsweepy-desktop --lib`: 84 PASS, 2 ignored.
- `cargo check --workspace --locked`: PASS.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: PASS.
- `npm run tauri -- build`: PASS. `BroomSweepy_1.4.0_x64_en-US.msi`와 `BroomSweepy_1.4.0_x64-setup.exe` 생성, `bloomsweepy-mcp 1.4.0` sidecar 포함 확인.
- 16px, 32px, 128px 파생 아이콘: 투명 모서리·정확한 크기·빗자루 실루엣 육안 확인 PASS.
- README용 언어별 설정 화면은 로컬 frontend 실제 렌더를 사용했으며, 네이티브 backend가 없는 브라우저에서만 생기는 연결 오류 표시는 캡처에서 제외했다.
