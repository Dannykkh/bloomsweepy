# 입력창 CLI 선택 변경

date: 2026-10-08
source: codex
scope: 현재 사용자 첨부/요청만. 원본 turn timestamp와 session UUID는 제공되지 않음.

## 현재 요청

"입력창 보니까 똑같은 의미가 두갠데? 앞쪽에걸, 코덱스, 클로드코드, agy, grok등으로 변경가능하게 하는게 나을듯? 거기에 맞춰서 모델과 추론강도 선택하는 ui의 값이 변경되고, 그게 더 맞지 않나?"

## 결정과 구현

기존 모델 trigger와 reasoning popup의 모델 부제가 중복이었다. 첫 trigger를 CLI 선택으로,
둘째를 모델·추론 통합으로 바꾸고 기존 모델 부제→목록을 재사용했다. 헤더의 중복 select는 제거하고
상태·재확인을 유지했다. 별도 선호를 만들지 않고 공용 hook과 actual provider metadata를 사용한다.
초안/권한/전송/진행/취소는 보존. 비동기 조회는 최신 선호·revision/unmount guard를 적용했다.

## 검증과 경계

119개 기존 frontend 테스트·타입·빌드·contract 통과, 독립 코드 리뷰에서 의미 있는 회귀 없음.
합성 실제 React fixture에서5CLI, 공급자별 목록/강도·초안·Settings/재로드 복원·keyboard,
390px·미설치·busy/취소, mock request grok/grok-4.7/xhigh 전달을 확인했다.
합성 검증은 실제 계정 응답이 아니다. 앱 설치본 교체/새CLI설치/Windows/장시간 메모리 검증은 하지 않았다.
주기 temp 자동삭제·커밋·푸시는 이번 요청 범위 밖. 이전 file-review 변경은 보존했다.

#tags: 입력창, cli선택, 모델선택, 추론강도, 설정공유, arch:012
