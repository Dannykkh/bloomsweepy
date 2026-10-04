# 운영체제 휴지통 비우기 — 미출시 소스 검증

> 후속09:44KST: 새 로컬 arm64 설치본으로 교체했고, 실제 앱의 미선택 경고창·최종버튼 비활성·취소까지 확인했다. 실제 OS 비우기/권한 변경은 여전히0회. 아래 미교체 내용은 초기 검사 시점 기록이며 최신 결과는 [앱 관리·설치 QA](2026-09-07-application-management-open-actions.md) 참조.

## 범위와 안전 경계

- 현재 8 GiB Apple Silicon Mac, Data 볼륨 여유 약 15 GiB에서 실행했습니다.
- 사용자 요청은 버튼 개발입니다. 실제 사용자 휴지통 비우기, OS 자동화 권한 승인, 설치 앱 교체, 커밋·푸시는 수행하지 않았습니다.
- 공간 정리 공통 상단의 열기/비우기, 별도 native HTML dialog, 미체크 기본값, 다른 앱·연결 드라이브 포함 경고, 일회용 승인, 작업 중 배경 inert를 구현했습니다.
- macOS Finder 고정 명령/제한된 helper와 Windows Shell API를 소스에 연결했습니다. OS의 실제 메모리 사용량은 앱의 stdout/시간 제한으로 통제되지 않습니다.
- 기존 파일별 휴지통 이동·복구 저널과 구분합니다. 작업 기록은 백업이 아니며, OS 비우기 결과의 항목별 성공·확보 바이트를 추정하지 않습니다.

## 실행 검증

| 검사 | 결과 |
|---|---|
| `cargo test -p bloomsweepy-desktop --lib empty_trash -- --test-threads=1` | 6 통과, 실제 OS 비우기 호출 없음 |
| `CARGO_INCREMENTAL=0 CARGO_BUILD_JOBS=1 cargo test --workspace -- --test-threads=1` | 246 통과, 기존 1 ignored |
| `cargo clippy --workspace --all-targets -- -D warnings` | 통과 (macOS 대상) |
| `cargo fmt --all -- --check` | 통과 |
| `npm run check` / `npm run test:all` | 통과 / 40 통과 |
| `npm run build` | 통과, 기존 단일 JS 청크 500 kB 초과 경고는 남음 |
| Info.plist / entitlements.plist `plutil -lint` | 통과 |
| `git diff --check` | 통과 |

Rust 신규 검사는 확인 누락, ID 교체, 오래된 dismiss의 새 계획 보존, 만료, 취소, 재사용, 경로·명령 추가 필드 거부, 불명확 결과의 재요청 차단, 출력 판정, mock 중복 제출을 검증합니다. 기존 runtime 테스트도 단일 실행/commit 취소 경계를 확인합니다.

## 실제 React 화면 + 모의 IPC 검증

`empty-trash-fixture.html`에서 CUA로 클릭/키보드를 사용했습니다. 실제 Native/OS 호출은 **0회**입니다. 테스트용 페이지는 Vite의 기본 배포 진입점에 포함되지 않습니다.

1. 최초 열기: 취소 버튼에 포커스, 체크박스 미선택, 최종 버튼 비활성화.
2. Escape: 창 닫힘, mock dispatch 0회, 원래 비우기 버튼으로 포커스 복귀.
3. 체크 후 최종 확인: mock dispatch 1회, 재실행 버튼 제거, `요청됨` 결과 표시. OS 완료/확보 용량을 확정하지 않음.
4. 만료된 계획: 만료 안내, 체크 및 최종 실행 비활성화.
5. 불명확 결과: OS 작업이 계속될 수 있음을 안내, 닫은 뒤 비우기 재요청 불가.
6. 권한 거부: 자동화/Finder 권한 안내, 성공 문구나 권한 자동 변경 없음.
7. 3초 대기 + OS 취소: 앱 Escape로 실행 상태를 숨길 수 없음, 중복 실행 불가. 이후 일부 삭제 가능성 안내.
8. IPC 오류(영어): 완료 불명확으로 처리, 재요청 차단. 닫은 후 활성화된 Open Trash 버튼으로 포커스 복귀.
9. 작업 중 및 미지원 플랫폼: 비우기 비활성화, OS 휴지통 열기는 별도 유지.
10. 760×600 최소 창에서 경고·체크박스·최종 버튼이 보이는지 스크린샷으로 확인. 한국어/영어 실제 UI, 일본어/중국어는 카탈로그 키·플레이스홀더 테스트로 확인.

OS/MCP/AI 경계도 검색 및 기존 정확한 MCP 도구 목록 테스트로 확인했습니다. 새로운 세 명령은 Tauri handler에만 있으며 control protocol, MCP, assistant tools에는 추가하지 않았습니다.

## 미검증 / 후속 확인

- **실제 사용자 휴지통은 비우지 않습니다.** 실제 OS 성공·취소·잠긴 파일·외장 드라이브는 폐기 가능한 별도 테스트 계정에서 승인 후 검증해야 합니다.
- macOS 앱 번들의 Finder 자동화 권한창, hardened runtime 서명, 실제 Finder 완료 시점은 미검증입니다. 권한 목적 설명과 entitlement 설정만 코드/형식 검사했습니다.
- Windows 소스 경로는 추가했으나 현재 Mac에서는 Windows 네이티브 컴파일/OS 대화상자를 실행하지 않았습니다. 기존 Windows CI 및 별도 PC 검증이 필요합니다.
- Finder 요청 성공을 완전 삭제/정확한 공간 반환의 증거로 사용하지 않습니다. 타임아웃은 helper만 종료하며 Finder 작업을 취소하지 못할 수 있습니다.
- 설치된 앱은 교체하지 않았습니다. 이 문서는 릴리즈 완료나 설치형 E2E 합격 보고서가 아닙니다.

구현 경계와 공식 API 근거: [안전한 휴지통 작업](../architecture/safe-trash-actions.md).
