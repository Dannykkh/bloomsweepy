# Argos 재감리 — Mac management

## 후속 수정 — 2026-10-20 22:07 KST

사용자가 미완료 개발·수정을 계속하도록 요청해 아래 A1–A5를 보완했다. **후속 수정은 완료했지만 기존 전체 감리 판정을 무조건 PASS로 바꾸지는 않는다.** 아래 재감리 본문은 수정 전 관찰 기록으로 보존한다.

| 항목 | 조치 / 새 증거 |
|---|---|
| A1 | source-map-js1.2.1→1.2.2 한 패키지만 업데이트. package.json/나머지 의존성 버전 유지. `npm audit --json` 및 `--omit=dev` 모두 exit0/전체0. Vite→PostCSS→수정 패키지 연결 확인 |
| A2 | CleanupView가 기존 optional 확인문구를 전달. 개발캐시/앱데이터/혼합/기타 Review 영향 분리, 선택 요약도 일반 검토 항목으로 정정. Review 수·체크박스·allowReviewCandidates·native 승인 검사는 그대로 유지. 종류별 신규 회귀5개 포함 frontend141/141 PASS |
| A3 | domain-dictionary의 canRestore 정의에 최신 기록·미완료 행의 원래값 존재 시 명시적 journal-only 완료를 반영. 정본 API 계약으로 연결 |
| A4 | settings flow17노드로 no-op·무쓰기 복구·prepared 후 재검사·write/readback 실패·최종 commit 실패 분기 명시. cleanup11/inspect8은 변경 없음. 코드와 수동 대조했으며 모든 분기를 설치형 IPC로 실행했다는 뜻은 아님 |
| A5 | 실제 restore()의 fake native 복원 성공 뒤 `restored` UPDATE만 실패 주입. restorePrepared 내구성→재시작 조회→명시적 무쓰기 완료→중복 거부 회귀 PASS. mac_settings 집중16 PASS/1 opt-in ignored, 실제 Finder 쓰기 없음 |

새 실행: npm full audit70430/production audit37147 exit0, frontend전체88044 141PASS exit0, `npm run build`3113 TypeScript/Vite exit0(기존 큰 청크 경고), Rust집중43671 16PASS/1ignored exit0, cargo fmt/diff 검사 PASS. 기존 workspace478PASS/6ignored는 이전 전체 실행이며 새479전체 실행으로 합산하지 않는다. A2는 문구/전달·분류 단위검사이며 새 설치형 렌더 검증은 NOT RUN이다.

mac_settings.rs의 추가 테스트82줄을 제외한 SHA256이 기존 `a19a7ef…64668`과 정확히 일치함을 읽기 전용으로 확인했다. 생산 native/IPC 동작은 변경하지 않았으므로 기존 격리 Wry10/10 및 무변경 관찰 증거를 재사용했다. 정상 Finder 변경·복원 성공/제품 설치형 전체 IPC/Windows 실행/포괄 보안 검사 공백은 계속 남는다. 개발 의존성 공지 A1은 해소됐지만 이 검증 한계가 해소된 것은 아니다.

Windows는 [RemoveWindowsAI 조사와 후속 범위](../windows-management/research-and-scope.md)로 분리했다. 해당 제거 스크립트 실행·Windows 설정 관리 기능 구현·설치 앱 교체·커밋/푸시는 하지 않았다.

## 수정 전 재감리 기록

2026-10-20 · Codex · main/HEAD `85620ca` 위의 미커밋 개발본. 사용자 요청: “아르고스”. 제품 소스·의존성·설치 앱은 변경하지 않았다. 이전 보고서는 [archive/verify-report-2026-10-20-2134.md](archive/verify-report-2026-10-20-2134.md)에 보존했다.

## 판정: CONDITIONAL

기능 요구7/7은 구현·테스트 경로에 연결된다. 기존 Rust478 PASS/6ignored, frontend136 PASS, UI10/10, production build 통과 증거를 같은 소스 지문과 대조해 재사용했다. **새로운 실제 Wry IPC 검사10개도 통과**했다. 격리된 실제 WebView→Tauri→생산 명령 경로를 사용했으며 JavaScript mockIPC나 가짜 invoke key를 주입하지 않았다.

다만 전체 npm 감사에서 개발 의존성 `source-map-js1.2.1`의 High 공지1건을 확인했다. 배포 의존성 감사는0건이다. 실제 정상 설정 변경·복원 성공/설치 앱 전체 경로/전체 보안 스캐너는 검증하지 않았으므로 무조건 PASS·출시 승인으로 판정하지 않는다.

### 주요 발견

| ID | 분류 | 근거 / 영향 / 권고 |
|---|---|---|
| A1 | High advisory / 개발 공급망 | `apps/desktop/package-lock.json:1965`의 source-map-js1.2.1. Vite7.3.6→PostCSS8.5.26 경로. 조작된 source map 처리 시 CPU/메모리 자원 고갈 공지. 1.2.2 이상 수정본으로 업데이트하고 full audit/build를 재확인 권장 |
| A2 | P3 / 사용자 설명 | `CleanupView.tsx:447`이 종류별 reviewAcknowledgementLabel을 전달하지 않아 `SafetyActionDialog.tsx:164`의 “프로그램 설정…계정이나 설정 데이터” 문구가 개발 캐시에도 표시됨. 기존 승인 검사는 유지되지만 재빌드/재다운로드 영향으로 설명하는 편이 정확함 |
| A3 | P3 / 계약 문서 | `docs/domain-dictionary.md:20`의 canRestore 정의는 현재값==적용값만 허용한다고 쓰지만 `mac_settings.rs:474–480/:661–672`는 미완료 행의 원래값이 이미 존재하면 무쓰기 기록 완료도 허용. API 계약은 이 예외를 설명하므로 사전도 맞추는 것이 좋음 |
| A4 | P3 / 도면 추상화 | settings.mmd는 validate→journal→write를 선형 표시. 생산 코드의 동일값 no-op(:597), 원래값이 이미 있는 명시 복구(:661), write 성공 뒤 final commit 실패 경로를 별도 분기로 나타내지 않음. 노드 존재29/29가 모든 경로의 완전 일치를 뜻하지 않음 |
| A5 | P3 / 추가 회귀 권장 | apply 최종 commit 실패 주입(:989)과 restorePrepared 복구 fixture(:1027)는 있으나, 실제 restore 함수의 native 복원 성공 후 restored UPDATE만 실패시키는 결합 시험은 없음. 구현 오류로 확정한 것은 아님 |

A1의 공지와 패치 범위는 [유지관리자 수정 PR](https://github.com/7rulnik/source-map-js/pull/79), [1.2.2 릴리스](https://github.com/7rulnik/source-map-js/releases/tag/v1.2.2)로 확인했다. npm full audit exit1은 Critical0/High1, `--omit=dev` exit0은 전체0이다. `postcss/lib/previous-map.js:5/:42–44`에서 source-map-js를 사용하지만, 설치 앱의 일반 파일 스캔이 이 Node 빌드 코드를 실행하는 경로는 확인하지 못했다. **앱의 원격 취약점 실증이나 이전 브라우저 자동화 정지의 원인이라는 뜻은 아니다.** 자원 고갈 PoC는 이 Mac에서 실행하지 않았다.

## Module Coverage

카탈로그에서 source-only 정확한 경로를 해석하고 메인이 각 SKILL 전체와 필수 참조를 읽었다. 프로젝트/활성 루트의 동일 이름 파일은 없었다.

| 모듈 | 해석 경로 | 적용 / 한계 |
|---|---|---|
| Argos | `/Users/dannysmacair/.codex/skills/argos/SKILL.md` | 전체 Phase0–8, verify-protocol 전체 |
| code-reviewer | `/Users/dannysmacair/.codex/.olympus/source-skills/code-reviewer/SKILL.md` | security-audit 전체. 동일 diff의 기존 native review 재사용, 중복 실행 없음. 포괄 보안 scanner 범위는 미실행을 표시 |
| flow-verifier | `/Users/dannysmacair/.codex/.olympus/source-skills/flow-verifier/SKILL.md` | verify 경로로 노드·분기·오류·코드에만 있는 경로 대조 |
| frontend-design | `/Users/dannysmacair/.codex/.olympus/source-skills/frontend-design/SKILL.md` | DESIGN.md 및 전체 공유 블랙리스트 대조, 재디자인하지 않음 |
| ui-ux-auditor | `/Users/dannysmacair/.codex/.olympus/source-skills/ui-ux-auditor/SKILL.md` | 동일 소스의 렌더/9영역7.7(B*) 증거 재사용. 실제 설치형·수치 대비·스크린리더 미검증 |

독립 읽기 전용 기능 검증/품질·하네스 검증/infra·신뢰 경계 검토를 병렬 수행했다. 작업자들은 파일을 쓰지 않았으며 최종 판정과 보고서는 메인이 소유한다. 역할 수를 다중 모델 합의로 표현하지 않는다.

## Phase 0 — CPS 추적

- 문제3/3: 흩어진 설정/원복→R1–3·5, 개발 캐시→R4·5, AI의 앱 사실 사용→R6·7.
- Ecosystem5/5: CFPreferences·SQLite는 section01, cleanup engine은02, React/Tauri/common dispatcher는03.
- sections/index manifest3개와 실제 section 파일3개가 대응한다. 단일 로컬 사용자이며 AI caller는 별도 권한 경계다.

## Phase 1 — 정적 구현

| 요구 | 근거 | 판정 |
|---|---|---|
| R1 고정5개/부재·관리형·오류 구분 | mac_settings.rs:92/:420, native.rs:136 | 구현됨. 부재를 OFF로 만들지 않음 |
| R2 고정 입력/로컬 적용/선기록 | mac_settings.rs:62/:226/:281/:537/:610 | 구현됨. 원격/다른 창과 schema 거부는 아래 실측 |
| R3 원래 부재 복원/충돌/상한/중단 | mac_settings.rs:378/:515/:625/:661 | 구현됨. 외부 atomic CAS/ABA는 계약상 보장 제외 |
| R4 개발 캐시/Review/기존 안전 경계 | mac_cleanup.rs:9, core cleanup.rs:283/:319/:496, trash_actions.rs:512/:938 | 구현됨. Review 승인/재검증 소비 경로 연결. A2 문구 불일치 |
| R5 초안/상태/복원/화면 | MacManagementView.tsx:39/:66/:148/:171, CSS, AppShell/App 라우트 | 구현됨. 같은 소스 mock UI10/10 |
| R6 공통 읽기/별도 공개 허용 | app_tools.rs:132/:186/:221/:247, capabilities.rs의 MacosInspect | 구현됨. AI write variant 없음,5행/16KiB/늦은 취소·권한 철회 검사 |
| R7 격리 테스트 | FakeStore/native random domain/합성 캐시/mock fixture/아래 Wry probe | 계획한 범위 검증. 정상 OS 변경 성공·설치형은 별도 |

일반 diff 품질은 [native-code-review.md](reviews/native-code-review.md)의 source:native 결과를 재사용했다. P2 시계 역행은 rowid 정렬과 회귀로 해결됐다. 이번 정적 재검토에서 요구 기능의 P0/P1 누락이나 사용자 데이터 삭제로 이어지는 우회를 입증하지 못했다. 이는 저장소 전체 무결함 주장이 아니다.

## Phase 2 — 실행 증거

| 검사 | 결과 | 새 실행 / 재사용 |
|---|---|---|
| Rust workspace | 478 PASS/6ignored, exit0 | 이전76735, 소스 지문 일치 |
| frontend | 136 PASS, TypeScript/build PASS | 47965/18310/12779 재사용. 기존 큰 청크 경고 유지 |
| 실제 Finder 읽기 | 1 PASS, macOS26.4aarch64,5unset/None | 20459 재사용. Finder 쓰기 없음 |
| 실제 컴포넌트 mock UI | 10/10 PASS, exit0 | 32142 재사용, 전용 stable ARM64 환경. 설치형 증거 아님 |
| 격리 실제 Wry IPC | 10/10 PASS, exit0 | 새93576; 요청별 무변경 관찰을 강화한 최종51486도10/10 + 거부8건 각각 상태 불변 PASS |
| npm full audit | Critical0/High1, exit1 | 새95671 |
| npm production audit | 전체0, exit0 | 새79505 |

원래 passing suite를 형식적으로 반복하지 않았다. 이번 변경은 보고서/검증 산출물뿐이며 기존 제품 소스·lockfile을 수정하지 않았다. 최초 하네스 컴파일의 외부 모듈 하위 경로 오류는 생산 소스를 수정하는 대신 byte-identical 사본의 원래 디렉터리 구조를 보존해 해결했다. 첫10/10 뒤 거부마다 상태를 관찰하도록 하네스를 강화했고 최종51486도 exit0이다. [기계 판독 실행 결과](../../qa/macos-management-argos/native-ipc-results.json), [하네스·재현 절차](../../qa/macos-management-argos/README.md).

검사 process는 모두 종료됐고 임시 crate184KiB/감사 package21.9MiB만 정리했다. 재현용 하네스 원문/manifest와 실제 JSON 응답은 보존했다. 공유 캐시나 기존 앱/개인 자료는 삭제하지 않았다.

## Phase 3 — API 계약

`get_mac_settings`, `apply_mac_setting`, `restore_mac_setting`, 공통 `macos.inspect`, `ui.view:macos_management`의5/5 계약이 기존 composition point에 연결된다. 새로운 HTTP 업무 endpoint는 없다. 로컬 IPC와 공통 LLM projection은 대상과 권한이 달라 의도적으로 분리되며 병렬 설정 엔진이 아니다.

### 새 실제 IPC 검증

실제 Wry/JavaScript native invoke를 사용한다. 생산 mac_settings.rs/native.rs를 byte-identical하게 임시 crate의 원래 모듈 구조로 복사했다. 원본과 사본 SHA256은 각각 a19a7ef…64668 / be89484…c38a로 일치했다. 생성한 숨김 로컬 main/secondary와 스크립트 없는 loopback 원격 페이지만 사용했다. UI mockIPC, 임의 invoke key, 원격 권한 강제 부여는 사용하지 않았다.

| TC-ID | 계약 | 정상 대조/변조 | 실제 응답 / 관찰 |
|---|---|---|---|
| IPC-CONTROL-READ | 정상 조회 | local main/get | 성공, 설정5개, audit journal 없음 |
| IPC-CONTROL-CANARY | 호출 경로 대조 | main/apply, 정상형식·알 수 없는 ID | 생산 definition에서 “지원하지 않는 Mac 설정” 거부. 정상 OS 변경 성공 대조는 아님 |
| IPC-SECONDARY-APPLY | API-AUTH | 같은 apply payload, secondary local | 생산 guard의 “로컬 기본 창” 거부 |
| IPC-LOCAL-RESTORE-ID | INPUT-SCHEMA | main/restore, invalid-hex | 복원 식별자 오류 |
| IPC-SECONDARY-RESTORE | API-AUTH | 같은 restore, secondary local | 생산 guard가 식별자 검사 이전에 거부 |
| IPC-FIELD-EXTRA | API-FIELD | apply에 domain 추가 | Tauri serde unknown field 거부 |
| IPC-FIELD-MISSING | INPUT-SCHEMA | expectedValue 누락 | missing field 거부 |
| IPC-FIELD-TYPE | INPUT-SCHEMA | enabled를 문자열로 | invalid type 거부 |
| IPC-REMOTE-APPLY | API-AUTH | main을 loopback HTTP로 이동, 같은 canary | “Command apply_mac_setting not allowed by ACL”, 명령 본문 이전 거부 |
| IPC-STATE-UNCHANGED | 부수 효과 | 처음/마지막 read와 journal 비교 | 고정5개 settings 동일, journal 존재/내용 동일 |

실제 허용된 Finder ID를 mutation에 보내지 않았다. 거부 코드가 잘못돼도 canary는 definition 단계에서 차단돼 native write에 도달하지 않는다. 최종 실행에서는 거부8건 각각 직후 secondary의 정상 조회로 설정5개와 journal 존재/내용을 비교했고 모두 불변이었다. 임시 app ID의 journal만 관찰하고 기존 설치 앱 journal은 사용하지 않았다. 원격 거부는 Tauri ACL 단계의 증거이며, command-body remote guard를 실행한 척하지 않는다. WebKit 자체 캐시·전체 파일시스템까지 무쓰기였다는 주장은 하지 않는다.

이 증거로 기존의 “실제 IPC negative 전혀 미실행” 공백은 줄었다. 그러나 **제품 전체 setup/기존 capability 정책/설치 패키지, 같은 endpoint의 실제 정상 변경·복원 성공, stale·중복·경합 업무 전이 전체는 NOT RUN**이다. API-OBJECT의 다른 계정/조직은 단일 로컬 역할이라 N/A, 복원 객체의 정상 소유 기록/재전송은 현재 fake-store 시험 근거이지 실제 설치형 IPC 실증이 아니다. 따라서 Phase7-3a 전체 PASS로 확대하지 않는다.

## Phase 4 — QA

MAC-01–11 모두 구현·기존 실행에 추적된다. MAC-01–06/10–11은 기존 Rust/protocol/build, MAC-07–09는 실제 컴포넌트 mock UI10/10을 근거로 한다. 이번 실제 IPC TC10개는 별도 집계이며 기존 UI10개 또는 Rust478개에 합쳐 임의의 전체 통과율을 만들지 않는다. [QA 목록](qa-scenarios.md), [Minos 이력](minos-report.md).

## Phase 5 — 도면

settings10/10, cleanup11/11, inspect8/8로 **노드 존재29/29**가 대응한다. journal commit→effect→readback→final status, Review→승인→재검증, permission→read→권한/취소 재검사→bounded projection의 순서를 확인했다. 다만 A4의 no-op/무쓰기 복구/final commit 실패는 추가 코드 경로다. 추상도면의 노드 매칭과 모든 분기 일치를 구분해 **branch coverage는 PARTIAL MATCH**로 기록한다.

## Phase 6 — 디자인

기존 Data Instrument/14px/44px/row·divider/동작 상태/토큰 방향을 따른다. 동일 컴포넌트의 최신1280/390 렌더·font loaded·Tab 첫 focus·reduced-motion·overflow0과 [9영역7.7/10(B*)](ui-audit.md)을 재사용했다. 색8, 반응형8, 접근성7, 로딩7, 폼8, 내비7, 타입8, 모션8, AI slop9의 가중값이다. 현재 단독 fixture의 캡처이지 앱 shell·native glass 전체 렌더가 아니다.

전체 공유 블랙리스트를 소스/기존 캡처와 대조했다. semantic error block의 좌측 danger 선은 장식용 카드와 구분하며 예외 근거를 남긴다. 신규 화면에서 Hard Rejection을 발견하지 못했다. screen reader/모든 배율/정량 합성 대비/설치형 검증은 NOT RUN이다.

## Phase 7 — Security Audit

Mode: Argos comprehensive 계약 아래 infra·공급망·변경 기능 경계를 점검. 스캐너 미실행은 완전 감사로 간주하지 않는다.

- 입력·권한: 고정 ID/Bool/required nullable expectedValue/unknown fields 거부, 실제 local/secondary/remote 검사, AI typed read-only만 존재.
- 출력·비용: 모델용5행/16KiB, journal/domain/key/path 제외, 조회 전후 공개권한·취소·native epoch 확인.
- CI: 두 check workflow contents:read, pull_request이며 pull_request_target/명시 secret 주입/배포 write는 발견하지 못함. lockfiles tracked, sidecar build --locked/shell:false.
- 공급망: A1 개발 의존 High1. action mutable tag는 SHA pin 권고이며 실제 변조는 미검증.
- sidecar: MCP helper는 sibling+symlink 거부/실행 비트/버전 검사. document worker는 sibling/is_file 기반으로 링크·서명 runtime 검증은 더 약하나 앱 디렉터리 쓰기 전제가 필요하다. 실제 교체·권한 상승은 미검증인 방어 심화 권고다.
- secret: gitleaks/trufflehog 등 redacting scanner 미설치. 명시 env/key 파일명과 Git 이력 메타데이터에서는 후보 경로가 없었으나 본문/이력 전체 secret 감사·재발급 여부는 NOT RUN. raw secret grep/git log -p, 개인 대화·memory·demo-assets 내용 읽기는 하지 않음.

| STRIDE | 현재 대응 / 한계 |
|---|---|
| Spoofing | local main guard + upstream remote ACL을 격리 Wry에서 확인. 제품 전체 권한 경계는 별도 |
| Tampering | typed allowlist·현재값 비교·파라미터 SQL. 외부 CAS/ABA 보장 없음 |
| Repudiation | FULL synchronous prepared journal. 실제 복원 후 final commit 실패 결합 시험 A5 권고 |
| Information disclosure | bounded projection/공개 허용. provider 자체 간접 prompt injection 우회는 미실증 |
| DoS | 고정 설정5개/history128/cleanup bounds. dev dependency A1 별도 위험 |
| Elevation | file full_access를 OS 쓰기에 상속하지 않음. 정상 OS mutation IPC 성공 경로 미실증 |

Coverage gaps: Rust supply-chain audit/전체 이력 secret·OWASP/fuzz scanners, 실제 CI log/provenance/notarization, Windows ACL/실행, sidecar 교체·서명 공격, 실제 CLI 공급자 보안 동작, 설치형 전체 IPC/정상 OS 변경·복원은 NOT RUN/UNVERIFIED다. 미실행 항목의 취약점 수를0으로 계산하지 않는다.

## Phase 8 — 용어와 경계

storedPreference/macSettingChange/developerCache/macosInspect의 의미·UI 표기는 대체로 부합한다. canRestore는 A3의 미완료 기록 무쓰기 완료 예외를 보완해야 한다(신규5용어 중1개 정의 불완전). 부재≠꺼짐/논리크기≠실제확보/캐시후보≠삭제안전의 구분은 유지된다. 전체 저장소 금지어 자동 감사는 미실행이다.

## 조치 분류와 재개

- FIXABLE 후보: A1의 dev dependency 업데이트, A2 종류별 최종 확인문구, A3 사전·A4 도면 보완, A5 실패 주입 회귀 추가. 이번은 감리 요청이라 제품 소스/lockfile을 바꾸지 않았다.
- 추가 실행: 설치형 정상 변경·복원과 전체 IPC business-flow 검증. 실제 Finder 설정을 바꾸는 검사는 별도 정확한 대상·원복 계획이 필요하다.
- 기존 빌드/테스트 green과 새로운 negative 통과를 근거로 기능 구현 상태는 양호하다. **개발 공급망 High 공지와 명시된 검증 범위/문서 경고가 남아 CONDITIONAL**이다. 오류 수정이나 출시 완료를 주장하지 않는다.

#tags: 아르고스, 감리보고서, 네이티브ipc, 개발의존성, 검증한계, arch:016
