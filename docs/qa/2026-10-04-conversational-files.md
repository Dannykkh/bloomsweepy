# 일반 파일·폴더 대화 관리 검증

- Date: 2026-10-04, macOS Apple Silicon / 현재 8 GiB 맥북.
- Branch/base: main / 842d916. GitHub v1.7.0 배포물은 변경하지 않음.
- 요청: 설치된 AI가 일반 폴더를 삭제할 수 없다고 답하는 문제를 실제 파일 관리 흐름으로 해결.

## 구현 범위와 경계

기존 빈 폴더 전용 모델 도구 옆에 `files` 앱 소유 작업 공간을 추가했다. 일반 이름 검색, 폴더 용량 검사, 하위 탐색, 상위 이동, 페이지, 후보 제외/선택, 일반 파일·내용 있는 폴더의 최종 휴지통 검토, 기존 열기/위치 표시를 같은 대화에서 제공한다. 모델은 작업을 요청할 뿐 최종 승인·셸·임의 경로를 전달하지 못한다.

이름 검색 200개/경로 2 MiB/25만 방문/30초, 모델·IPC 페이지 24개, 선택 100개, 상태 16세션. 이름이 중복되거나 검색이 불완전하면 자동 선택하지 않는다. 폴더 검토는 기존 20,000항목/8 MiB 경로/깊이64/30초 예산을 재사용한다. 본문·새 색인 DB·무제한 목록을 추가하지 않는다. 계획은 세션/revision/선택에 묶인 5분 일회용이며 폴더 하위 내용 확인 체크가 필요하다.

POSIX 내부 심볼릭 링크는 링크 자체의 metadata 및 `read_link` 대상 문자열만 fingerprint에 포함한다. 원본을 따라가거나 읽거나 용량에 합산하지 않는다. 링크 수를 최종 화면에 표시한다. 선택 항목·부모 링크, Windows 재분석 지점/정션, 클라우드/온라인 전용/보호 경로/다른 장치/특수 항목 차단은 유지한다. OS 경로 기반 호출의 마지막 순간 경쟁을 완전히 제거했다고 주장하지 않는다.

대화형 이름 변경·일반 이동·새 폴더 생성·영구 삭제는 이 작업의 지원 범위가 아니다. Claude 등 다른 모델 및 Windows 실기를 이번 Mac 검증으로 인증하지 않는다.

## 실제 실행 결과

| 검사 | 결과 | 범위 |
|---|---|---|
| `cargo test --workspace --lib` | PASS: 273, ignored 3 | control17/core81/desktop165/MCP10 |
| `npm run test:all` | PASS: 43 | 공급자 오류, 안전 정책, 4개 언어 키/placeholder 등 |
| `npm run check` | PASS | TypeScript |
| `cargo fmt --all -- --check`, `git diff --check` | PASS | 형식/공백 |
| `cargo clippy --workspace --lib -- -D warnings` | PASS | 신규5개 스타일 경고 수정 후, assistant_files5개 회귀 추가 PASS |
| `live_codex_file_tool_contract --ignored --nocapture` | PASS: 2개 실제 Codex 요청 | 과거 ‘빈 폴더만 가능’ 대화 포함, 일반 scan 및 nonempty `review_named` 계약. 임의 사용자 파일 작업 없음 |
| `native_assistant_file_trash_synthetic_only --ignored --nocapture` | PASS | 직접 만든 파일+내용 있는 폴더 2개/16 B를 실제 macOS Trash로 이동. 추가 링크가 가리키는 비선택 외부 파일의 내용 보존 확인 |
| Tauri arm64 release app build | PASS | 프론트 포함 최종 .app. 기존 JS chunk >500 kB 경고 남음 |
| ad-hoc `codesign --verify --deep --strict` | PASS | 호스트·동봉 MCP/document-worker. Apple 공증이 아님 |

네이티브 Trash 시험은 링크 보완 전/후 각각 한 번 실시했고, 두 번 모두 시험이 생성한 자료만 처리했다. 사용자 폴더나 휴지통 비우기는 실행하지 않았다. 메모리 장시간 안정성 검증으로 간주하지 않는다.

## 설치 앱 실제 Codex → 검토 카드

1. `/Applications/BroomSweepy.app` 교체 후 기존 BroomSweepy 대화 13개와 설정·작업 이력 보존 확인.
2. 15:31 KST: `promo-video` 최종 검토 요청이 앱 검색까지 연결됐지만 기존 코어 검증에서 내부 `.bin/semver` 링크로 중단됨. ‘CLI가 못 한다’는 문제가 아닌 앱의 과도한 링크 정책임을 실제 화면에서 확인.
3. 링크를 원본과 분리하는 코어 검토를 보완하고 재빌드/교체함.
4. 15:39 KST: 같은 저장된 대화에서 “promo-video 폴더의 휴지통 이동 최종 검토만 다시 준비해. 실제 이동은 아직 하지 마.” 요청.
5. 실제 Codex 요청 → 로컬 재검사 → 최종 카드 성공: **1.2 GB 표시, 파일14,914/폴더1,296/링크29**. 앱이 ‘최종 검토 준비, 아직 이동 없음’을 대화에 저장. 하위 내용 체크 전 실행 버튼 disabled 확인.

실제 사용자 `promo-video` 최종 버튼은 누르지 않았다. 설치 앱에서의 요청→검토와 네이티브 실제 Trash 실행은 위처럼 분리된 검증이다.

설치 바이너리 SHA256: `61773657b0348631079143460761e41d3cd728a454600e87ee3b1f27148a9f9d`.
표시 버전은 로컬 패치 전과 같은1.7.0이며 릴리스/태그/커밋/푸시는 하지 않았다.
설치 후 Clippy가 제안한 동등한 if 조건/closure borrow 정리만 소스에 추가했고, 해당5개 회귀를 재실행했다. 설치본은 이 lint-only 정리 직전의 동일 기능 빌드이며 재패키징하지 않았다.
원래 앱의 롤백 사본: `/private/tmp/broomsweepy-previous-app-obVm8L/BroomSweepy.app`.
중간 개발본 사본: `/private/tmp/broomsweepy-intermediate-app-6MFxPr/BroomSweepy.app`.

이번에 만든 arm64 target 캐시 약800MB는 설치 후 제거했으며 재빌드로 재생성 가능하다. 기존 debug 캐시와 사용자 자료는 보존했다. 정리 후 여유 디스크 약4.9 GiB(일회 관측), 임시 dev server/fixture 탭 및 viewport override 정리.

검토2회 후 호스트 단일 프로세스 RSS 일회 관측47,568 KiB. WebView/CLI 자식을 포함한 최고치나 장시간 안정성 측정이 아니다. 15:52에는 만료된 계획을 ‘선택 다시 검토’→최종 검토로 갱신하는 설치 앱 재시도도 PASS, 실제 이동 없음.

## UI 검증

CUA로 실제 React fixture의 검색→내용 있는 폴더 계획→체크 전 잠금→체크 후 mock 실행→같은 채팅 결과→목록 재검사를 확인했다. Mock 실행은 OS 파일 작업이 아니다. 1280×820 및 좁은325px 화면에서 줄바꿈/가로 넘침을 시각 확인했다. 최종 링크 안내는 실제 설치 앱 스크린샷/접근성 트리에서 확인했다. 스크린샷은 세션 도구 출력으로 확인했으며 별도 PNG 저장은 하지 않았다.

기존 DESIGN.md의 재질·타입·카드·확인 흐름을 재사용했다. 9영역 점수는 **추가한 대화 카드 범위의 부분 감사**이며 앱 전체·모든 플랫폼·모든 테마 합격이 아니다.

| 영역 | 점수 | 관찰과 남은 한계 |
|---|---|---|
| 다크/라이트 | 8 | 설치된 다크 글래스에서 읽기 가능. 별도 라이트 테마 실측 NOT RUN |
| 반응형 | 8 | 좁은 폭에서 경로/버튼 줄바꿈, 넘침 없음. 실제760×600 native 창 별도 NOT RUN |
| 접근성 | 8 | native AX label/status/alert, 44px 주요 행동, checkbox label. 전체 스크린리더 과업 NOT RUN |
| 로딩/성능 | 7 | busy·취소·중복 실행 잠금. 큰 JS chunk 경고, 장시간 계측 남음 |
| 폼 UX | 9 | 검색 이름 상한, 오류 원인, 미측정/만료 구분, 확인 전 실행 잠금 |
| 네비게이션 | 8 | 같은 채팅·현재 경로·루트 안 상위 탐색. 기존 앱 전역 내비게이션 유지 |
| 타이포/간격 | 8 | Pretendard computed family/16px 입력 및 실제 한국어 확인. document.fonts 직접 조회는 CUA 제한으로 NOT RUN |
| 애니메이션 | 8 | 새 장식 모션 없이 기존 재질, reduced-motion 대응. OS 설정을 실제 변경하지 않음 |
| AI Slop | 9 | 기존 기능형 목록과 단일 검토 행동, 새 스톡 카드/히어로 없음 |

가중 총점8.05/10, B. 색상·모바일·보조기술 미실측을 점수만으로 해소하지 않는다. 최신 링크 설명의 한국어/영어/일본어/중국어 키 parity 자동 검사 PASS.

## 남은 검증

- Windows GUI/설치/휴지통 런타임: NOT RUN — 현재 Mac 환경.
- 장시간 전체 프로세스 트리 RSS/DB/WAL/임시 디스크: NOT RUN — 이 회귀와 실제 한 번의 검토로 대체하지 않음.
- 실제 사용자 앱 제거/휴지통 비우기, 실제 `promo-video` 이동: NOT RUN — 이번 작업에서 승인하지 않음.
- 기존 v1.7.0 다운로드 업데이트: NOT RUN — 이번 요청은 수정·로컬 설치 범위이며 릴리스 승격 별도.
