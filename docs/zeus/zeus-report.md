# Zeus Report — macOS management

- 입력: “제우스 계획을 세우고 프로그램 개선까지 진행하자”
- 날짜: 2026-10-20 (Asia/Seoul). 정확한 시작 시각·총 소요 미측정.
- 결과: **PARTIAL — 1차 구현·모의 UI10/10·격리 실제 Wry IPC negative10/10 완료. 설치형 전체 IPC/정상 설정 변경·복원과 포괄 보안 검증이 남아 CONDITIONAL 유지**.
- 이전 실행 아카이브: `docs/zeus/archive/2026-10-20-macos-management/`에 보존. 삭제하지 않았다.

## Phase 0: Description Parsing

Utility/기존 Rust·Tauri·React·TypeScript·SQLite 데스크톱 개선. 새 서비스/인증/의존성을 만들지 않았다. 현재 대화의 목표는 Mac 상태 이해·되돌릴 수 있는 설정·공간 확보이며, 무제한 시스템 변경 권한으로 해석하지 않았다.

## Phase 1: Planning

완료. [계획](../plan/macos-management/plan.md), spec/API/schema, CPS 문제3개, ecosystem5개, 구현 섹션3개, 흐름3개, 운영/QA11그룹을 작성했다. 네이티브 팀 검토와 외부 읽기 전용 Codex 계획 검토의5계약 보완을 반영했다. 단일 모델 계열이며 다중 모델 합의가 아니다. Antigravity는 미설치라 NOT RUN.

## Phase 2: Implementation (agent-team)

3섹션/2wave 완료: 독립 native 설정·개발 캐시·UI 작업 뒤 기존 구성 지점에 메인이 조립했다. R1–R7 모두 구현 추적7/7이며 계획한 fake/synthetic/격리 native/모의 UI 검증 증거7/7을 확보했다. 이는 실제 Finder 변경·설치형·포괄 보안 검증까지7/7이라는 뜻이 아니다.

- `맥 관리`: Finder 표시 Bool5개 조회, 초안/명시 Apply, durable 기록, 원래 설정 부재까지 checked Restore.
- 개발 캐시: Xcode DerivedData7일/iOS DeviceSupport30일(수정일) Review 후보를 기존 cleanup/trash 엔진에 연결. 미사용/삭제 안전/물리 확보량 보장은 없음.
- 공통 AI/MCP: 같은 engine의 read-only `macos.inspect`와 Mac 화면 연결. OS 쓰기는 로컬 trusted main 창의 버튼에만 있고 파일 full_access를 상속하지 않음.
- README·DESIGN·아키텍처 계약·용어·계획·기억016 갱신. 설치된 v1.7.0 앱은 교체하지 않았다.

## Phase 3: Verification (Argos)

**CONDITIONAL**. [상세 검증과 후속 수정](../plan/macos-management/verify-report.md). 요구7/7 정적 추적, API5개. 후속 도면은 settings17/cleanup11/inspect8노드이며 설치형 전체 분기 실행을 뜻하지 않는다. Runtime에서 clock rollback·pending journal·값 부재·관리형/오류·권한 helper·cleanup review/bounds·공통 결과 상한을 검증했다. native code review의 P2(시각 역행 시 복원 순서)1건은 rowid 정렬과 회귀 테스트로 수정했다.

브라우저는 정합된 격리 환경에서10/10 PASS다. 후속 격리 실제 Wry/JS→native invoke negative10/10과 거부8건 각각의 상태 불변도 PASS다. 제품 전체 setup/설치형 IPC·정상 Finder 변경 성공으로 확대하지 않는다. 포괄 보안/이력 secret/Rust 공급망 검사도 NOT RUN이다. npm full 감사에서 발견한 source-map-js High1은1.2.2 최소 업데이트 후 full/production 모두0이다. [제한 UI 감사](../plan/macos-management/ui-audit.md)는 전체 앱 출시 승인이나 네이티브 glass 검증이 아니다.

## Phase 4: Environment — native dev-server fallback

docker-deploy 모듈을 읽고 기존 네이티브 데스크톱 구조에 적용했다. Docker 명령은 없어 설치/Compose 생성 없이 계약상 native Vite fallback을 사용했다. loopback127.0.0.1:1431 fixture HTTP200. CoreFoundation은 Docker에서 검증할 수 없으며 native test로 검증했다. 기존 다른 프로세스/포트는 변경하지 않았다.

## Phase 5: Testing

| 검사 | 실제 결과 |
|---|---|
| Rust workspace, clock fix 포함 | 478 PASS / 6 ignored, exit0 |
| 실제 Finder 읽기 전용 smoke | 1 PASS, macOS26.4aarch64, 고정5개 unset/None |
| 프런트 회귀 | 후속141/141 PASS(종류별 확인문구5개 추가) |
| Mac 설정 집중 회귀 | 후속16 PASS/1 opt-in ignored; restore 최종 기록 실패 회귀 추가. 기존 workspace전체478과 별도 집계 |
| TypeScript / production build | PASS (기존 large-chunk/dead-code 경고 유지) |
| 전체 실제 컴포넌트 모의 UI | **10/10 PASS, 0 FAIL, 100%, exit0(session32142)** |
| 격리 실제 Wry IPC negative | 10/10 PASS, 거부8건 각각 상태 불변. 설치 앱 전체 경로 아님 |
| npm full / production audit | source-map-js1.2.2 이후 각각0 vulnerabilities |

[Minos 보고서](../plan/macos-management/minos-report.md)에 재실행·시간초과·환경 비교를 기록했다. 테스트 실패를 skip하거나 성공으로 바꾸지 않았다. OS Finder 쓰기0, 사용자 자료 삭제0. 격리 무작위 테스트 preference domain과 synthetic 파일만 테스트에 사용했다.

## Errors & Recovery

| 단계 | 발견 | 조치 / 상태 |
|---|---|---|
| 계획 | CF scope/중단 복구/기록 보존/외부 CAS/IPC 권한 계약 부족 | 다섯 계약 보완 |
| 구현 | React updater가 늦게 event.currentTarget 접근 | 값 즉시 캡처로 수정 |
| 통합 | capability26개 discovery16514B로16KiB 초과 | format2 index의 중복 inputTypes만 제거; 상세 계약·ID·권한 보존, 상한 유지 |
| 리뷰 | 벽시계 역행으로 원복 체인 순서 왜곡 | rowid 정렬 + 시각 역행 회귀 PASS |
| UI QA | fixture 스크롤/긴 빈 하단, favicon404 | fixture 한정 수정·캡처 재확인 |
| UI QA | 초기 alpha환경 간헐 timeout / 진단 자체 무한 대기 | watchdog 추가, 정식1.64.1/전용 ARM64 browser156/ARM64 Node 비교에서3/3 및10/10 통과. 환경 변수 여러 개가 달라 세부 원인은 단정하지 않음 |

## Decision Ledger

| 결정 | 선택·근거 | 기각한 대안 / 되돌리는 법 |
|---|---|---|
| 첫 개선 범위 | 고정 Finder5개+개발 캐시+공통 조회. 기존 엔진 재사용 | 전체 upstream41/AI14 이식은 OS·복구·권한 근거가 달라 보류. 개별 호환성 증거를 모아 별도 확장 |
| AI 권한 | 설정 조회만, 변경은 로컬 UI | 파일 full_access를 OS에 상속하지 않음. 별도 OS 변경 권한 계약을 설계하면 재검토 |
| Finder 반영 | 저장 검증만, 자동 재시작 없음 | 자동 kill/restart 안 함. 필요하면 명시 재시작 기능과 효과 검토를 별도 추가 |
| 원복 보존 | 전체128기록 유지, 상한 뒤 신규 변경 거부 | 조용한 가지치기 기각. 별도 보존/압축·체인 증명을 설계한 뒤 변경 |
| 위험 기능 | AI 모델/프로필/서비스/스냅샷/시뮬레이터 영구삭제 제외 | 각 OS 지원·관리자 권한·실제 복구 증거가 생기면 재설계 |

## Next Steps / 미완료

1. 격리 Wry negative는 완료했다. 제품 setup/설치형 전체 IPC·정상 변경/복원·stale/중복/경합의 남은 경계를 검증한다. 현재 출시 승인 아님.
2. 설치형 E2E·Windows 검증·포괄 보안의 남은 범위 수행. 실제 Finder 변경은 별도 명시 대상/복원 계획 아래 확인. Windows는 [별도 조사/범위](../plan/windows-management/research-and-scope.md)로 분리했으며 아직 새 설정 기능을 구현하지 않았다.
3. 보고서와 결정 검토 후 요청 시 설치앱 교체/커밋/푸시/릴리즈. 이번에는 수행하지 않았다. 초기 alpha 조합은 테스트 기본으로 사용하지 않고 같은 고정 도구/브라우저를 준비한다.

안전한 재개 지점은 [인계](../handoffs/2026-10-20-204748-macos-management-validation.md)에 있다. 보고서를 작성했다는 이유로 미달 gate를 완료로 판정하거나 완료 promise를 출력하지 않는다.

#tags: 제우스, 맥관리, 부분완료, 검증한계, arch:016
