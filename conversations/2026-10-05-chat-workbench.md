# 대화 중심 UI 재구성 — 정제된 요청·결정 기록

date: 2026-10-05
source: codex

## 현재 요청

사용자는 진행 상태가 없어 멈춘 것처럼 보이는 채팅, 화면 밖으로 밀려나는 입력창,
채팅을 차지하는 외부 연결·권한 설정을 지적했다. 이어
“openai의 일반적인 채팅ui가 아니네, 완전 수정해줘.”라고 요청했다.
사용자 턴 시각·세션 UUID는 제공되지 않아 추정하지 않았다.

## 결정과 근거

대화 기록만 스크롤하는 중앙 작업면과 항상 보이는 하단 composer를 채택한다.
실제 native/CLI 단계와 경과 시간을 표시하고, 다음 질문은 대기 중에도 작성할 수 있다.
권한/진단은 Settings와 대상 옆 native dialog로 이동하며 값을 자동 변경하지 않는다.
읽기 전용 결과는 접되 부분 결과 신호, 검토·오류·권한 부족은 기본 노출한다.

이전 normal-flow 제목 수정의 목적(메시지 위로 제목이 겹치지 않음)은 유지한다.
sole-main-scroller 방식은 새 사용자 요구로 대체한다. LLM 자체 스트리밍/완료율을
가짜로 만들거나 삭제 승인 계약을 바꾸는 선택은 제외했다.

## 실제 확인

TS, frontend53개, Rust provider25개(2 ignored), production build 통과.
Production AssistantView를 마운트한 합성 어댑터에서 긴 대화, 접힌 결과24행,
760×600/390×700 입력창 유지, 단계·시간·취소·오류, dialog Escape/포커스 복귀,
검토 기본 펼침과 실행0을 확인했다. 실제 개인 대화·파일은 테스트에 사용하지 않았다.
안정된 소스에서 다음 질문/읽던 위치 보존을 확인했고, ARM64 설치형에서7개 저장 메시지,
입력창·Codex 준비 상태·설정 권한·팝업과 Escape 포커스 복귀를 검증했다. 기존 앱은
rollback 폴더에 보관했다. 설치 SHA와 정확한 미실행 경계는 QA 기록을 따른다.

이 요청은 commit/push/release나 권한 변경 요청이 아니다.

#tags: chat-workbench, docked-composer, actual-progress, settings-permissions, arch:008, supersedes:#single-main-chat-scroll
