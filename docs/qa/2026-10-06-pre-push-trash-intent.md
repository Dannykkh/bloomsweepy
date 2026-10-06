# 커밋 전 조건부 삭제 오승인 검사

2026-10-06 18:44 KST · source: codex · 현재 사용자 요청 “커밋 푸시”.

## 발견과 영향

독립 pre-push 리뷰에서 `namedTrashRequest`가 조건을 일부 표현의 blacklist로만 제외해
다음 문장에 true를 반환함을 순수 함수 실행으로 재현했다.

- `문제가 없으면 VideoProc 삭제해`
- `백업이 있으면 VideoProc 삭제해`
- `만약 VideoProc 필요 없으면 삭제해`
- `如果没用就把 VideoProc 删除`
- `不要なら VideoProc 削除してください`

확인 생략ON과 단일 완전 native 계획이면 실행 분기로 이어질 수 있어011의 조건부 제외
계약과 불일치했다. 실제 파일·앱을 제거해 시험하지 않았다. 이 발견 전59개 staged 코드·
기록의 credential/private marker 검사와 문서 공개 범위 검토에는 다른 finding이 없었다.

## 수정 계약

정확한 계획 대상 이름을 먼저 식별·마스킹하고, 남은 전체 문장이 제한된 직접 삭제
명령일 때만 true를 허용한다. 조건·상담·인용·추가 명령·알려지지 않은 대상·복합 문장은
기존 예/아니오로 돌아간다. 파일 이름 자체의 조건 단어는 실행 조건으로 오인하지 않는다.
원문의 placeholder/control character로 명령을 위조할 수 없도록 거부한다. 이름이 있어도
문장의 일부만 제거 명령으로 맞는 것을 승인으로 해석하지 않는다.
입력2,000자/이름100개/이름255자 상한과 NFC·대소문자 정규화 뒤의 중복 검사, longest-first
단일 pass 치환을 적용한다. 한국어의 고정 `앱|파일|폴더` 분류어는 허용해 기존
`Synthetic Editor 앱 삭제하자` 동작을 보존하며 임의 잔여 명사는 허용하지 않는다.

## 검증 상태

소유 변경은 `assistantConfirmation.ts`와 단위 검사만이며, native 권한·재검증·일회용
계획·삭제 루틴은 변경하지 않는다.

- TypeScript `npm run check` PASS.
- frontend `npm run test:all` **91/91 PASS**. 기존85에 신규6블록을 추가했다. KO/EN/JA/ZH
  조건·상담·인용·추가 명령, literal 조건 이름·겹침·NFC·문장부호, marker/control/줄바꿈·
  상한, 완전한 native 계획이 있어도 오승인하지 않는 회귀를 포함한다.
- 메인의 기존5개 조건문 순수 함수 재실행 모두false, 명확한 직접 명령은true.
- 최종 classifier 소스 `npm run build`(tsc+Vite) PASS. JS929.29kB로 기존500kB chunk 경고는
  유지되며 이를 해결한 것으로 보고하지 않는다.
- `git diff --check` 및 staged diff 검사는 PASS. 독립 수정 리뷰 **PASS/findings 없음**:
  기존P1 5개를 포함한 순수 함수29사례와 최종 classifier/완전 native 계획 경계를 확인했다.
- native suite는 이 후속 helper 수정에 재실행하지 않았다. 이전 동일 native 소스229PASS/
  3ignored 근거는 [추론 QA](2026-10-06-cli-reasoning-selection.md)에 구분해 남아 있다.

이전14:16 추론 설치본의 GPT-6.1-Sol/medium 실제 응답은 PASS지만 이 후속 안전 수정의
설치본 검증으로 확대하지 않는다. **이번 guard의 네이티브 앱 재빌드·설치/실제 삭제,
Windows·장시간 검사는 NOT RUN**. 현재 사용자 요청은 커밋·푸시이며 설치 교체는 별도다.

후속 사용자 요청의 [새 설치형 QA](2026-10-06-installed-trash-safety.md)에서 guard 앱 교체,
실제 조건부 native 계획/아니오 취소와 재시작을 확인했다. 위 NOT RUN은 커밋 당시 기록이며
새 직접 자동 이동은 후속 정확한 대상 승인 상태/결과를 해당 QA에서 확인한다.

#tags: 조건부삭제, 직접명령, 확인생략, 푸시검토, arch:011
