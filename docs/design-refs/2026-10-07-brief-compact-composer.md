# Brief: compact model and reasoning composer

사용자는 현재 입력창 캡처를 첨부하고 두 번째 Codex 선택 UI처럼 바꾸기를 요청했다.
Source mode는 screenshot-reference delta이며 AI 도우미의 하단 입력부를 기준으로 한다.
최우선 과업은 질문 작성 → 모델/추론 선택 → 전송이며 설정 설명이 대화 높이를 차지하지 않게 한다.
후속 요청으로 입력창 아래 상시 설명과 정상 상태의 팝업 설명을 제거한다. 오류·미지원·저장값
복구에 필요한 안내는 남기고, 전송 범위와 권한 설명은 기존 연결·권한/설정 경로에서 확인한다.
또한 Claude Code·Grok·Antigravity(agy)의 모델·추론 선택을 채팅과 설정에 함께 연결한다.
Settings의 기존 Picker와 공급자·모델별 선호 hook을 재사용하며 별도 저장소를 만들지 않는다.
실제 CLI 목록과 모델별 지원 증거만 표시한다. 최신 모델을 앱이 임의로 고정하거나 목록 갱신으로
사용자의 모델을 자동 변경하지 않는다. 진행·취소, 권한·삭제 경계는 보존한다.
성공 조건: 입력창 안의 한 줄 툴바, 위로 열리는 compact 팝업, 실제 지원 단계 슬라이더,
기본값 복원, 390px/760×600/1280×820에서 잘림 없이 키보드 조작 가능, 공급자별 선택의
설정/채팅 동기화와 CLI 인자 연결. 설치되지 않은 CLI의 실제 실행 검증은 별도로 표시한다.

범위 추가의 근거는 같은 대화의 “그 아래쪽 설명자체를 빼지?”와
“클로드코드나 agy도 … 그록도 … 최신의 모델과 추론강도 … 옵션쪽도 수정” 요청이다.
이는 기존의 backend/catalog 변경 제외 범위를 해당 공급자 선택 연동에 한해 확장한다.
계정·로그인·실행 권한·파일 작업 정책은 확장하지 않는다.

Product Design Gate: UNKNOWN. available 목록에는 product-design@openai-curated-remote
0.1.56/installed:false가 있으나 해당 marketplace의 source와 설치로 추가되는 권한·도구가
현재 메타데이터로 확인되지 않았다. 세션 Product Design capability도 없다. 로컬 React/CSS
어댑터를 사용하며 plugin/설정 변경·adapter 비교는 NOT RUN이다.

#tags: compact-composer, screenshot-reference, model-picker, reasoning, multi-provider
