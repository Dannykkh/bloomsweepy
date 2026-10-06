# 채팅 휴지통 이동의 인간 결정과 선택적 확인 생략

status: CURRENT
date: 2026-10-05
source: codex
tags: conversational-files, trash-consent, native-opt-in, nonexpiring-plan, bounded-search
supersedes: [[001-conversational-file-workspace]] — 앱 소유 범위/프로토콜/상한은 계승하고 버튼 전용 최종 승인·5분 TTL만 사용자 요구로 변경한다.
evidence: conversations/2026-10-05-conversational-trash-consent.md#사용자-요청-턴-시각-미확인; #1927-kst--검증-기록; conversations/2026-10-06-conversational-trash-regressions-fixed.md#0701-kst--실제-codex-종단; conversations/2026-10-06-commit-push-safety.md
alternatives: 다중 체크/검토 팝업 유지 — 명확한 요청도 재승인해야 해 사용자 과업을 방해; 일괄 무조건 자동 실행 — 상담·모델 출력·불완전/모호 결과도 승인으로 오해하므로 제외; 시간 만료 유지 — 늦게 돌아온 사용자의 결정을 불필요하게 막으므로 제외. 불확실한 대상은 단일 질문으로 돌아간다.
depends-on: [[002-opaque-folder-symlinks]], [[003-conversational-storage-map]], [[004-app-tool-investigation]], [[009-opt-in-permission-lifetime]], [[010-selected-folder-inspection]]
sources: docs/architecture/app-capability-contract.md; docs/qa/2026-10-05-conversational-trash-consent.md; docs/qa/2026-10-06-pre-push-trash-intent.md
files: apps/desktop/src/lib/assistantConfirmation.ts; apps/desktop/src/views/AssistantView.tsx; apps/desktop/src-tauri/src/assistant_files.rs; apps/desktop/src-tauri/src/assistant_tools.rs; apps/desktop/src-tauri/src/application_actions.rs; apps/desktop/src-tauri/src/permission_settings.rs; apps/desktop/src-tauri/src/control_server.rs
reopen-when: 지시어/범주 전체 정리 자동화, 여러 계획 승인, 관련 데이터 자동 제거, 공급자/MCP 실행 또는 다중 사용자 권한 요구가 생기면 재검토한다.
last_verified: 2026-10-06 — 후속 조건부 오승인 복구 frontend91/TypeScript/production frontend build 및 독립29사례 리뷰 PASS/findings 없음. 이 guard의 native 앱 재빌드·설치는 NOT RUN. 이전 native 형태 합성 회귀/설치형 Codex 아니오 취소·ON 정확한29B Trash 종단 PASS와 나머지2파일58B 보존은 별도 이전 검증이며 새 guard의 설치형 근거가 아님. native229PASS/3ignored는 이전 추론 QA이며 후속 helper에는 재실행하지 않음.

`files` strict JSON의 조회·검사·검토만 모델에 제공한다. 앱 소유 작업 공간은 세션 루트 안의
200 결과/24 페이지/100 선택/16 세션과 bounded 메타데이터를 유지한다. 자동 모델 입력은
제한된 이름·크기·ID·질문/대화이며 전체 경로/본문은 자동 추가하지 않는다. 사용자가 입력한
경로나 승인된 문서 검색 본문 공개는 별개이며 토큰 절감률은 미측정이다. 지도 공유는003,
링크 원본을 따라가지 않는 검증은002, 일반 이름 변경/이동/생성 미지원은 그대로다.

기본 채팅 삭제는 정확한 계획에 한 번의 예/아니오. sole pending plan에 대한 직접 인간
응답은 모델 왕복 없이 main UI가 처리한다. 기본 OFF 확인 생략 권한은 원래 인간의 정확한
이름을 포함한 단순 직접 제거 명령과 단일 완전 계획만 실행한다. 상담·부정·조건·다른 경로·누락 이름·
복수 계획은 제외. Rust도 `automatic:true`에서 현재 권한을 확인하고 기존 Trash/journal을
실행한다. 모델·MCP에는 실행/권한 설정 도구를 추가하지 않는다.

파일/빈 폴더/앱 검토 계획은 null expiry, runtime-only, 일회용이다. session/revision/선택/
inventory/kind와 실행 직전 신원·내용·보호 경로·실행 중 앱 검증을 유지한다. 취소·변경·소비·
재시작 후 새 계획이 필요하다. 새 권한 선호만009의 Remember DB에 저장하며 개별 승인과
계획은 저장하지 않는다. 일반 파일/폴더·Mac 앱 본체만 확인 생략 대상이고 관련 데이터,
종류 전체 빈 폴더 정리·프로세스 종료·Docker·영구 삭제는 제외. 트리005/지도/시스템
정리의 별도 TTL·확인은 변경하지 않는다. Windows 앱 제거는 OS 정식 제거 화면 그대로다.

2026-10-05 실검증에서 발견한 회귀: `assistant_provider.rs`의 파일 계획은 항상 `files.workspace`의
`review_required`와 함께 반환된다. `AssistantView`의 `reviews.length===0` 조건은
이를 별도 승인으로 계산해 파일/폴더 확인 생략을 막는다. 합성 fixture는 이 래퍼가
없어 놓쳤다. 실제 인간 아니오 취소 뒤에도 공통 대기 카드가 남는다. 설계 결정은
유지하되 당시에는 구현 회귀 수정과 실제 ON 종단 전이라 완료로 판단하지 않았다.
근거: docs/qa/2026-10-05-conversational-trash-consent.md 실제 검증 절;
docs/handoffs/2026-10-05-201047-conversational-trash-native-verification.md.

2026-10-06 수정: 같은 revision의 native 자체 래퍼는 두 번째 승인으로 세지 않는다.
`source`, capability, reviewPrepared, freshScan, revision 일치와 단일 자체 검토를 요구하고,
다른 결과는 모두 completed여야 한다. 모든 review를 무시하는 대안은 다른 기능의 검토까지
오승인하므로 제외했다. 취소/실행/계획 변경은 자체 revision 대기 래퍼만 제거하고 취소를
대화에 저장한다. fixture도 실제 래퍼를 포함해 합성 응답 누락을 재현 가능한 회귀로 고쳤다.
사용자가 승인한 자체29B 파일만 실제 Codex 종단으로 휴지통 이동했고 확인 생략ON을 유지했다.
관련 데이터/빈 폴더 종류 전체/프로세스/Docker/영구 삭제의 제외 경계는 불변.
Windows와 장시간 soak, 실제 내용 있는 폴더/앱 본체 자동 실행은 NOT RUN.

2026-10-06 커밋 전 독립 리뷰에서 조건 일부만 blacklist로 제외하는 구현이
“문제가 없으면 VideoProc 삭제해” 및 JA/ZH 조건문을 승인하는 P1을 순수 함수로 재현했다.
기존 조건 제외 계약은 유지하고, 정확한 native 대상 이름을 먼저 literal로 마스킹한 뒤
전체 잔여 문장이 지원하는 단순 직접 명령인지 판별하도록 복구했다. blacklist 확장은
언어별 미등록 조건을 다시 놓치므로 제외했다. 모든 이름은 정확히 한 번 필요하며 인용·
추가 명령·복합/지원하지 않는 표현은 예/아니오로 돌아간다. 파일 이름 자체의 조건 단어는
literal로 다룬다. marker/control 충돌·상한 초과도 확인 생략을 막는다. 권한·계획·실행 루틴은
변경하지 않았으며 이 후속 수정의 설치본 실제 실행은 이전29B 시험과 별개다.
