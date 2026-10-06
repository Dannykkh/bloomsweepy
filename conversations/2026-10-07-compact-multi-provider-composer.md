# 컴팩트 입력창과 다중 CLI 모델·추론 선택

date: 2026-10-07
source: codex
scope: 현재 사용자 요청만 기록. 원본 세션 UUID/턴 도착 시각은 제공되지 않음.

## 현재 요청

사용자는 앱의 큰 모델/추론 select와 반복 설명이 있는 첫 이미지 대신,
두 번째 Codex 참조의 작은 선택 버튼·현재 추론 강도·모델 이름·보라색 slider·원형 전송 UI를 요청했다.
후속 요청: “그 아래쪽 설명자체를 빼지?”
후속 요청: “클로드코드나 agy도 모델이 다 다르잖아? 그들도 해줘. 그록도 해주고,
최신의 모델과 추론강도를 설정할수있게 해주라. 옵션쪽도 수정해야할것이야.”
추가 요청: “오퍼스 5.5 페이블 5.1 소넷 5.5 등등 최신 버전을 보여줘야지. 그냥 글자만 나오면 어떻해?”
추가 요청: “안티나 그록은 아예 사용할 모델리스트도 없네”
설치 요청: “교체하자.”

## 결정과 구현

대화 입력창 안에 작은 model/effort trigger, 위쪽 popup,44px원형 전송 버튼을 배치했다.
정상 상태의 아래 설명은 제거하고 stale/미지원/기본값 미확인 복구 안내는 보존한다.
가짜 microphone·고정 Ultra·애니메이션 particle·추가 의존성은 넣지 않았다.
설정은 기존 Picker를 재사용하며 같은 provider/model별 선호를 사용한다.

Claude는 설치된 CLI2.1.291의 질문 없는 correlated initialize response를 읽고,
default 중복 행 제외·실 ID와 지원 effort만 추출한다. 실패/구형 CLI는 기존 aliases를
보존하지만 effort/default/version을 추정하지 않는다. 정확한 단일 [1m] suffix만 허용한다.
Grok/Agy는 광고된 models command의 제한된 출력만 읽는다. Grok의 정확한 확인 모델
matrix와 Agy의 실제 family variant만 지원 강도로 연결하고, unknown을 임의 확장하지 않는다.
명시 model/effort는 실제 CLI argv에 전달하며 CLI 기본값이면 생략한다.

## 검토와 수정

독립 리뷰에서 모델 선택 미지원 설치본으로 바뀌면 저장 model이 숨겨지고 null로
자동 fallback할 수 있음을 확인했다. 저장 ID 표시·전송 차단·명시 reset으로 수정했다.
Grok의 빈 tools가 deny-all이 아니라는 공식 source를 확인해 명시 deny '*'와 help gate를 추가했다.
상태/목록 조회는 로그인·모델 계정 접근권·서버의 실제 effort 적용 증명과 구분한다.

## 검증 범위

합성 UI에서 keyboard/Escape/default/stale/busy·760/390반응형 및 Claude/Grok/Agy
요청 선택값·Settings 공유·reload를 확인했다. 실제 삭제·개인 파일 전송·LLM 생성 없음.
Claude update2.1.147→2.1.291 성공. 현재 로그인되지 않은 상태이며 새 로그인은 진행하지 않는다.
Grok/Agy CLI는 이 Mac에 없어 실제 metadata/생성 검증은 NOT RUN이다.
최종 검사와 설치 결과는 [QA](../docs/qa/2026-10-07-multi-provider-model-selection.md)를 따른다.
commit/push/release는 요청되지 않아 실행하지 않는다.

## 설치 완료 관찰 — 2026-10-07 01:29 KST

프론트108개·네이티브264개·실제 Claude metadata-only1개 PASS, ARM64 빌드/서명/설치 교체 완료.
이전 앱을 새 tmp 백업에 보존했으며 앱 데이터는 삭제하지 않았다.
실제 Claude4모델/지원5단계·Codex Sol/중간과 Settings 공유를 확인했다.
테스트로 바꾼 선호를 원래 Claude/opus/CLI 기본값으로 복원했다.
정상 종료/프로세스 부재/재실행 후 기존 대화11개·모델 선호·권한 설정 보존 PASS.
실제 생성은 이번 검사 범위에서 하지 않았고 Claude 로그인 필요/Grok·Agy 미설치 한계는 남는다.

## 후속 버전 라벨 수정

CLI가 실제 `resolvedModel`을 제공하지만 앱이 displayName만 썼던 누락을 확인했다.
실제 metadata에서 opus/sonnet은5.5, haiku는4.5, 기존 pinned Fable는5다.
metadata-only startup --model fable은 실제 alias/5.1/지원5강도 행을 추가한다.
따라서 같은 한 번의 초기화에서 별도 실제 최신 행을 받으며, 고정 버전 매핑·기존 Fable5 오표시·질문 생성은 없다.
canonical ID의 제한된 숫자 버전만 라벨로 만들고 실행 ID/선호/권한을 보존한다.
설정 tooltip도 버전 라벨과 ID를 함께 표시한다. 최종 검증/설치 기록은 연결한 QA의 후속 절을 따른다.

2026-10-07 01:57 KST 관찰: check/108frontend/267native(4ignored)/실제metadata opt-in1 PASS.
ARM 재빌드·서명·설치 해시f029c0d7… 일치, 기존앱별도tmp백업 완료.
실제 popup의 Opus5.5/Fable5.1/Sonnet5.5/Haiku4.5 및 별도Fable5(1M),
Fable5.1→IDfable/Settings공유·tooltip을 확인했다. 기존Claudeopus/default 선호와
이 턴 시작 시 사용 중이던CodexSol/중간을 복원했다. 대화11개·권한 보존, 질문/실행0.

## 후속: Grok/Agy 빈 목록

이 Mac의 실제 CLI 후보와 공식 설치 경로를 읽기 전용으로 점검해 두 CLI 미설치를 확인했다.
Grok 공식 `~/.grok/bin` 후보 누락은 수정했다. 목록이 없어도 단순 CLI 기본값으로 보이는
UI를 설치/로그인/서비스/호환성/조회 상태별 안내로 바꿨다. 공식 CLI 문서의 모델 예시는
Grok2개/Agy7개를 읽기 전용 별도 목록에만 표시하며 실제 선택·강도·선호·argv로 사용하지 않는다.
실제 목록이 생기면 참고 목록은 사라진다. 새 UI는 기존 디자인 토큰과 bounded scroll을 재사용했다.

2026-10-07 약03:05 KST:112frontend/269native·4ignored·fmt/diff PASS.
합성760/390화면에서 예시/설치안내·전송차단·Settings 공유·정상 목록 복귀 PASS,
실제 질문/파일 작업0. ARM 새 bundle5fca6b1a… build/deep strict 서명 PASS.
Mac이 잠겨 실제 앱 교체/새 UI/재시작은 NOT RUN이다. 현재 설치본f029c0d7…는 유지했다.
사용자에게 잠금 해제와 두 공식 CLI 설치 여부를 질문했으나 응답은 아직 없다.
CLI 설치·새 로그인·권한 확대·강제 앱 종료·commit/push는 하지 않았다.
최종 근거는 QA의 “후속: 빈 Grok/Agy 목록” 절이다.

## 새 설치본 교체

사용자 “교체하자” 후 Mac 잠금이 풀린 실제 화면을 확인했다. 기존 앱을 정상 ⌘Q로 종료하고
프로세스 부재를 확인한 뒤 `/private/tmp/broomsweepy-empty-catalog-backup-8Q7RnR/BroomSweepy.app`에
보존했다. 새 ARM bundle을 `/Applications`에 교체하고 deep strict·빌드/설치 해시5fca6b1a…
일치와 실제 실행을 확인했다. 개발본1.7.0 표시는 유지한다.

실제 Settings의 Grok2/Agy7 읽기 전용 예시·설치 안내·선택 비활성화, Grok 채팅 popup과
질문 차단을 확인했다. native 공식 안내가 정확한 Safari 페이지를 여는 것을 확인하고 검증 탭만 닫았다.
기존 대화11개와 권한·자동 시작·메뉴 막대·Docker 설정은 유지됐다. 검증 후 원래
Codex/GPT-6.1-Sol/중간으로 복원하고 앱을 열어 두었다. CLI 설치/로그인/AI 질문/파일 작업0.
설치형 확인은 앞선 Mac 잠금 NOT RUN을 해소하지만 실제 Grok/Agy CLI 동작 검증은 아니다.
세부 근거와 실제 이미지 링크는 QA의 “후속: 사용자 요청에 따른 설치 교체” 절이다.

## 핸드오프·커밋·푸시 요청 — 2026-10-07 KST

사용자 후속 요청: “커밋 푸시”, 이어서 “핸드오프하고, 커밋 푸시”.
앞선 commit/push 미실행 문장은 당시 상태이며 이번 요청에서 프로젝트 변경 공개가 승인됐다.
현재 구현·README·설계/QA·정제 기억·대화·실제 설치 증거를 함께 정리한다.
새 인계는 [커밋 인계](../docs/handoffs/2026-10-07-050539-provider-mcp-push.md)를 따른다.
코드와 문서 독립 읽기 전용 점검에서 확정 blocker 없음. benchmark의 완료 상태와 설치 이미지2개의
실제 JPEG 확장자를 정정했으며 이미지 픽셀·소스 기능은 변경하지 않았다.
옛 Swift 전체 데스크톱 캡처인 demo-assets5개는 개인 대화/로컬 경로 때문에 커밋에서 제외하고 로컬 보존한다.
SSH fetch는 키 인증 실패했지만 기존 원격 주소를 바꾸지 않고 같은 저장소 HTTPS fetch 및 push dry-run은 성공했다.
HEAD와 원격 main은 준비 시점9e9d775로 동일하다. 최종 커밋·푸시 결과는 Git과 사용자 완료 응답을 따른다.
추가 빌드/CLI 설치/로그인/AI 질문/파일 삭제/권한 변경/릴리스는 하지 않는다.

#tags: 채팅ui, 모델선택, 추론강도, 다중공급자, 앱교체, 커밋푸시, 핸드오프, arch:012
