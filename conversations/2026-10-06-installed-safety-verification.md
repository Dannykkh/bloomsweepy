# 새 맥 설치본의 조건부 삭제와 모델 복원 검사

date: 2026-10-06
source: codex
scope: 현재 “설치형 검사까지 완료하자” 요청. 사용자 턴 시각/session UUID 미제공.

9e9d775의 최종 frontend를 ARM64 app-only로 빌드·기존 entitlements로 재서명했다.
기존 앱은 정상 종료/main부재 후 새 backup에 보존하고 /Applications를 교체했다.
새 준비 번들/설치본08faa1ac… 일치·deep strict PASS. 첫 시작 전 권한·대화·개발 도구
DB hash도 동일해 설치 교체가 자료를 덮어쓰지 않았음을 확인했다.

자체 새87B/세파일 폴더만 선택했다. 확인 생략ON에서 실제 Codex에
“문제가 없으면 conditional-cancel.txt 삭제해”를 보내 정확한 native29B 계획/예·아니오
카드까지 준비시켰지만 자동 이동하지 않았다. composer의 “아니오”는 앱 취소 문구·
선택0개·현재 계획/자체 대기 제거로 이어졌고 세파일87B와8079B 저널이 그대로였다.

새 host ⌘Q/process부재→재시작 뒤 Settings/chat 모두6.1-Sol/중간, 취소 대화4개·기존
권한 복원 PASS. 이전 오후 Mac잠금으로 못 한 새 추론 빌드 재시작 검사를 실제 완료했다.
CLI argv probe는 새 요청 완료 후라 포착 못 함; 과거 argv proof를 새 검사로 확대하지 않는다.

실제 자동1/1 이동은 새 direct-auto.txt29B만 승인 요청했고 아직 수행하지 않았다.
이전 승인된 consent-auto 파일에 대한 승인을 새 파일에 재사용하지 않는다.
현재 검증/backup/미실행 범위는 [설치형 QA](../docs/qa/2026-10-06-installed-trash-safety.md).
새 기록은 로컬 갱신이며 이번 요청에서 commit/push/release는 실행하지 않는다.

#tags: 맥설치, 설치형검사, 조건부삭제, 모델복원, arch:011, arch:012
