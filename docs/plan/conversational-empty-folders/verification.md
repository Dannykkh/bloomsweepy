# 대화형 빈 폴더 관리 검증 기록

검증일: 2026-09-06. 대상은 macOS Apple Silicon에서 작업한 **미출시 소스**다. 설치된 v1.6.1 및 공개 릴리스는 변경하지 않았다.

## 결과

소스 구현과 아래 자동·합성 UI 검증은 통과했다. 실제 Codex가 새 구조화 응답 계약을 따르는지, 새 네이티브 설치본에서 전체 흐름이 완료되는지는 아직 검증하지 않았다. 따라서 배포 완료 또는 실제 AI 종단 간 검증 완료로 해석하면 안 된다.

| 검사 | 결과 | 범위 |
|---|---|---|
| `cargo test --workspace --quiet` | 205개 통과, 1개 ignored | core, desktop, control, MCP 및 통합 테스트 |
| `cargo test -p bloomsweepy-desktop --lib --quiet` | 121개 통과, 1개 ignored | 세션 상태 및 데스크톱 최종 변경 회귀 |
| `cargo test -p bloomsweepy-desktop assistant_ --quiet` | 32개 통과, 1개 ignored | 마지막 문맥 중복 제거·전송 안내 수정 후 재검증 |
| `cargo fmt --all -- --check` | 통과 | Rust 형식 |
| `cargo clippy --workspace --all-targets -- -D warnings` | 통과 | 전체 Rust 정적 검사 |
| `npm run check` | 통과 | 프론트 타입 검사 |
| `npm run test:all` | 37개 통과 | 프론트 로직 및 번역 키·플레이스홀더 |
| `npm run build` | 통과 | 기존 500 kB 청크 크기 경고는 남음 |
| `git diff --check` | 통과 | 공백·패치 검사 |
| README 링크·명령·이미지 검사 | 통과 | 5개 README의 로컬 링크 및 기존 명령 블록·이미지 참조 보존 |

ignored 테스트는 설치된 CLI를 대상으로 하는 별도 opt-in 진단이다. 실제 모델 요청으로 대체 실행하지 않았다.

## 도면과 소스 대응

[기준 도면](../../flow-diagrams/conversational-empty-folders.mmd)은 다음 구현과 대응한다.

| 도면 단계 | 구현 |
|---|---|
| 질문 및 제한된 AI 문맥 | `assistant_provider.rs`: `ask_assistant_inner`; `assistant_tools.rs`: `TOOL_CONTRACT`, `prompt_context` |
| 구조화 응답 검증 | `assistant_tools.rs`: `parse_envelope`, 허용된 `AssistantAction`만 역직렬화 |
| 로컬 검사와 후보 생성 | `assistant_tools.rs`: `dispatch`; 저장된 세션 폴더에서 검사 |
| 목록·선택 수정 | `assistant_tools.rs`: `apply_review_action`; 세션·revision·현재 페이지 후보 ID 검증 |
| 후보 및 정확한 최종 목록 | `AssistantView.tsx`, `AssistantEmptyFolderCard.tsx` |
| 5분 일회용 계획 | `prepare_assistant_empty_plan`, `confirm_assistant_empty_plan`, `claim_plan` |
| 실행 직전 재검증 | core `actions.rs`: `validate_empty_directory_trash`, `capture_empty_directory` 및 공통 verified 재검증 |
| 휴지통 및 결과 기록 | `trash_actions.rs`: `trash_verified_empty_directories` 및 기존 verified 실행·journal 파이프라인 |
| 화면·이력 동기화 | `App.tsx`: 공통 `runTrashAction`; 공급자 없는 앱 결과는 `BroomSweepy`로 저장 |

모델은 검사·목록·선택 변경만 요청한다. 모델 응답에 승인, 휴지통 이동, 임의 경로, 셸 명령을 허용하지 않는다. 한 사용자 요청에 허용된 앱 작업 하나를 처리하며, 무제한 자율 실행 루프는 아니다.

## 안전 경계 테스트

- 잘못된 JSON, 알 수 없는 필드, 실행·승인·임의 경로 요청을 거부한다.
- 모델 문맥은 현재 후보 24개로 제한하고 전체 경로와 파일 본문은 넣지 않는다. 로컬 검토 카드는 정확한 경로를 표시한다.
- 다른 세션·검사 번호·후보 ID, 현재 모델 페이지 밖 후보 변경, 만료·재사용 계획을 거부한다.
- 동시에 같은 계획을 실행하려는 요청 중 하나만 계획을 취득한다. 재검사·선택 변경·재시작은 이전 계획을 무효화한다.
- 숨김 내용 생성, 디렉터리 신원 변경, 부모 링크 교체, 보호 경로, 알려진 클라우드 경로, 취소를 검사한다.
- 합성 폴더 두 개를 mock Trash 디렉터리로 옮기고 보관 항목은 유지한다. 새 숨김 파일이 생긴 후보에는 Trash 호출이 발생하지 않는다.
- 실제 OS 휴지통 API는 경로 기반이다. 실행 직전 재검증을 하더라도 다른 프로세스와의 원자적 경쟁 방지를 보장하지 않는다.

## 합성 채팅 UI 검증

`apps/desktop/assistant-tools-fixture.html`은 실제 `AssistantView`에 mock IPC를 연결한다. `/Demo/QA Workspace`의 가상 후보만 사용했으며 사용자 파일이나 실제 AI를 호출하지 않았다.

1. “빈 폴더 검사해 줘”로 후보 3개 표시.
2. “2번 보관해 줘”로 2번을 제외하고 선택 2개 유지.
3. “응 삭제해”만 입력했을 때 실행 횟수 0 유지.
4. 검토 버튼으로 선택한 1·3번의 정확한 최종 경로만 표시.
5. 최종 버튼 후 mock 실행 1회, 2/2 결과와 항목별 상태 표시. 2번은 결과에서 제외.
6. 영문·만료 fixture에서 최종 실행 버튼 비활성화 및 만료 안내 확인.
7. 선택 재검토·전체 해제 후 선택 0개이면 검토 버튼 비활성화.
8. 긴 이름·경로 줄바꿈과 앱 결과의 `BroomSweepy` 표시 확인. 검증 흐름에서 콘솔 오류·경고 없음.

이 fixture는 입력별 mock 응답이므로 자연어 해석 품질이나 실제 CLI 준수율을 입증하지 않는다. 별도 모바일 뷰포트나 Windows 네이티브 화면 테스트도 수행하지 않았다.

## 검토 중 수정한 사항

검토 엔진: `sequential-main`. 앞선 위임의 계정 사용량 제한으로 독립 에이전트 및 native/external CLI 리뷰는 **NOT RUN**이며 우회 실행하지 않았다.

- Serde의 unit variant가 알 수 없는 필드를 무시하던 문제: 빈 struct variant로 바꾸고 주입 필드 거부 테스트 추가.
- 형제 폴더 이동으로 루트의 링크 수가 변해 다음 후보를 잘못 차단하던 문제: 루트 비교에는 안정적인 신원만 사용하고 후보 자체의 비어 있음·신원·수정 시각은 계속 검증.
- 앱 작업 결과가 AI 답변으로 저장되지 않도록 공급자 없는 `BroomSweepy` 결과를 분리하고 마지막 AI 공급자는 유지.
- 작업 후 공통 파일 목록·분석·이력 갱신 경로에 연결하고, 비동기 이전 workspace 응답이 최신 상태를 덮지 않도록 차단.
- 최종 확인 버튼을 기존 Trash 스타일로 맞추고 신규 카드 글꼴을 디자인 기준에 맞춤.
- AI 문맥에서 이미 전달한 전체 폴더 요약의 중복을 제거. 절감률은 측정하지 않았으며 특정 수치를 주장하지 않음.

## 남은 검증

- 실제 Codex로 검사 요청 → 후속 제외 요청 → 목록 응답의 새 계약 준수 확인.
- 새 macOS 번들에서 합성 임시 폴더만 대상으로 사용자 확인 → 실제 OS 휴지통 → 이력 확인.
- Windows에서 새 검사·계획·휴지통 경로 및 UI 런타임 확인.
- 독립 리뷰 후 배포 판단. 이 작업에서는 커밋·푸시·릴리스·설치 교체를 수행하지 않음.
