# 정리 후보 트리 검증

2026-10-05 · source: codex · main / base 842d916 · 현재 개발본

이 문서는 초기 구현 단계의 검증이다. 이후 같은 날 맥 설치 교체와 실제3대상209B 휴지통 이동, 보존/취소/재검사/재실행 및 AI 로컬 카드 진입을 통과했다. 아래 설치형 NOT RUN 표시는 당시 상태이며 최신 근거는 [설치형 후속 QA](2026-10-05-cleanup-tree-native.md)를 따른다. Windows·외부 LLM 조사 종단·장시간 메모리 검증은 여전히 남아 있다.

## 범위와 판정

삭제 후보를 트리로 확인하고 전체·개별 선택하며, 상위 선택을 하위에 상속한다. 하위 제외가 있으면 상위 폴더 자체를 이동하지 않는다. 기존 정리 후보 탭 안에서 시스템 후보와 분리하고, 지도/AI 측정 결과를 명시적 버튼으로 연결했다. 사용자 파일 삭제·설치 교체·릴리스는 이번 작업의 대상이 아니다.

소스 구현, 자동 회귀, 합성 화면 검증은 통과했다. 설치된 네이티브 앱의 새 명령 실행과 실제 OS Trash, Windows, 장시간 메모리 검증은 NOT RUN이며 이 기록은 배포 승인이나 전체 앱 완성 판정이 아니다.

## 실제 실행 결과

| 검사 | 결과 |
|---|---|
| `CARGO_INCREMENTAL=0 cargo test --workspace --lib --quiet -- --test-threads=2` | control23/core82/desktop203/MCP12, 합계320 통과·3 ignored·실패0 |
| `CARGO_INCREMENTAL=0 cargo clippy --workspace --all-targets -- -D warnings` | 통과 |
| `npm run check` 및 `npm run test:all` (apps/desktop) | TypeScript 통과, 51개 통과 |
| `npm run build` (apps/desktop) | TypeScript/Vite 통과, JS872.06kB/gzip250.47kB. 기존500kB chunk 경고 유지 |
| 경험 계약 validator | 통과; States의 success 용어 경고를 Completion success로 보완 |
| i18n | English/한국어/日本語/简体中文 키·placeholder 검사 통과 |
| `git diff --check` | 통과 |

첫 Rust 컴파일은 새 core helper의 오류 타입 변환 두 곳에서 실패했고 `.to_string()`으로 수정했다. 첫 최종 Clippy는 정렬/나눗셈/불필요 참조4건을 지적했고 수정 후 재실행했다. 검사 실패를 보호 규칙 완화로 해결하지 않았다. 3 ignored는 기존 opt-in 실제 CLI/네이티브 휴지통 검사이며 이번에 실행하지 않았다.

## 삭제 계약 검사

새 트리11개, assistant seed1개, strict core1개 회귀 검사를 추가했다. production bridge의 실제 invoke 인자를 mockIPC로 검증하는2개 검사와 프런트 표시 helper6개를 추가했다. mock IPC 통과는 실제 native IPC 실행을 뜻하지 않는다.

| 계약 | 실제 검증 근거 |
|---|---|
| 상위 선택→하위 제외→재선택 | Rust frontier 테스트: 제외 파일과 partial 조상을 실행 대상에 넣지 않음 |
| 부모/자식 중복과 페이지 이동 | Rust: frontier 중복 제거, 이전 페이지 선택 보존 |
| 부분/누락·미측정 용량 | Rust: 불완전 partial branch 검토 거부, 읽지 못한 폴더 크기 null 유지 |
| 숨은 보호 하위 | core strict audit 및 Rust: 미전개 `.git` 등 보호 항목이 있는 전체 폴더 검토 거부; 기존 다른 폴더 정책 보존 |
| raw path/임의 셸/승인 주입 | Rust strict DTO: node ID 외 권한 필드 거부;7개 bridge 명령의 정확한 wire 인자 검사 |
| source/revision/신원/범위 | Rust: report 교체·삭제, assistant old revision/삭제된 workspace, 노드 변경·루트 교체·범위 이탈 차단 |
| TTL/선택 변경/중첩 확인/재실행 | Rust: 만료·잘못된 revision/ID·ack 누락 거부, one-shot claim, 잘못된 계획 ID로 유효 계획을 소비하지 않음 |
| 단일 작업 잠금과 취소 | 기존 작업 lease를 tree blocking worker에 clone 전달하는 코드 검토 및 컴파일 통과. 호출 future drop 후 실제 worker 완료 시점의 런타임 계측은 NOT RUN |
| main Webview 최종 확인 | 명령의 main label 검사와 모델/MCP 미노출 코드 검토. 다른 Webview의 런타임 호출 거부는 NOT RUN |
| 지도·AI 통합 | App/Overview/Assistant 카드 composition과 production bridge 검사. 실제 설치된 앱에서 진입→새 native IPC→완료는 NOT RUN |

정리 실행은 기존 검증/OS Trash/저널/항목별 결과를 재사용한다. 논리 이동 용량은 즉시 확보되는 공간이 아니다. 휴지통 비우기·영구 삭제를 추가하거나 실행하지 않았다.

## 합성 화면 실검증

CUA로 실제 React `cleanup-tree-fixture.html`을 렌더했다. 실제 OS 호출·사용자 파일 검사/이동·AI 전송은 각각0이며, fixture의 모의 이동을 native 성공으로 보고하지 않는다.

- Old project 선택 후 Documents를 펼치면 미전개 하위도 체크 상태를 상속했다. KEEP 파일을 해제하면 부모 둘은 mixed였고, 검토 대상은 archive.zip500B + draft.txt60B =560B뿐이었다.
- 취소는 모의 실행0, 검토 버튼으로 포커스 복귀. 재검토 후 모의 실행1, 결과2개·560B, KEEP 생존과 결과 포커스를 확인했다.
- 전체 폴더 계획은 중첩 내용 확인 전 최종 버튼이 비활성화됐다. 기본 포커스는 취소이며 검토 중 배경은 inert다.
- 키보드 Space 체크, mixed-only 상태(checked count0)의 모두 해제 활성화→실제 해제, expired 트리의 변경/검토 차단을 확인했다.
- error 모드의 하위 탐색 실패는 오류와 재시도 안내로 표시됐다. partial 모드는 읽지 못한/생략된 자료 경고와 합성 검토 거부를 표시했다.
- 760×600에서 긴 한국어/경로가 줄바꿈되고 document scrollWidth=760으로 가로 넘침이 없었다. dark glass 화면을 직접 확인했고 JetBrains Mono 실제 로드를 확인했다.

빛 테마는 현재 제품에 없으므로 N/A다. OS reduced-motion은 당시false였다. CSS의 reduced-motion 처리와 큰 트리 DOM 상한은 정적/단위 검사했으며 실제 reduced-motion override, wide/expired-plan fixture 렌더는 NOT RUN이다. 화면 이미지는 도구에서 확인했으나 저장된 캡처 파일은 없다.

## 저자원과 성능

이전 공간 정리에서 검증한 ignored incremental 캐시만 제거해 실제 여유1.5GiB를 확보했다. 이번 Cargo는 CARGO_INCREMENTAL=0으로 실행했다. 트리 상태 하나·5분·16가지·2,048노드·경로8MiB·페이지50·실행100개, UI200행으로 제한한다. 분기 확장은 단일 작업 잠금/기존 스캔 가드를 재사용하고 지도 보고서나 assistant 상태를 교체하지 않는다.

2,048노드 테스트의 view 생성은 이 맥에서8.683292ms, 직렬화696,405B로 측정했다. 단발 테스트 수치이며 전체 앱 메모리·WebView·파일 검사 시간이나 장시간 누수 없음을 보장하지 않는다. 기존 한도를 낮춰 통과시키지 않았다.

## 독립 검토와 수정

네이티브 `codex review`는 실행을 시도했으나 현재 CLI의 gpt-6.1-sol 설정이 이 ChatGPT 계정 경로에서 지원되지 않아400으로 실패했다. 설정/모델을 임의 변경하지 않았다. 대신 독립 read-only 리뷰와 메인의 testing/maintainability/security/performance/API/design 점검을 수행했다.

리뷰에서 assistant mutation 후 stale revision 재사용, 전체 선택 안내, 반복 선택 집계 비용, 미측정0 위장, blocking worker lease, 잘못된 계획 ID 소비, 이름 검색만 한 진입점 문제를 수정했다. 메인은 scan 오류 전파, mixed-only 모두 해제, 디스크 경계를 추가 점검했다. 전체 회귀와 빌드/Clippy를 수정 후 통과했다.

## 남은 검증

디스크 여유를 확인한 다음 네이티브 번들 빌드/설치 교체 후 새 진입점과 실제 IPC, 합성 임시 파일의 OS Trash 및 취소를 확인해야 한다. 현재 /Applications 설치본은 교체하지 않았다. Windows 실기기, 전체 integration suite, 장시간 메모리 soak, future-drop 작업 잠금, main 외 Webview 거부 실측은 NOT RUN이다. 실제 사용자 자료는 삭제하지 않았고 rollback 앱도 유지했다.
