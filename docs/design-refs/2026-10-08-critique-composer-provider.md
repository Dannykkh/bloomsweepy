# Render critique — Composer CLI delta

검증 범위는 기존 compact composer의 두 진입점·헤더 select 제거에 한정한다.

## Observation and baseline gate

실제1280×820 캡처에서 CLI popup은 입력창 위에 분리되어 열리며 각 CLI 상태/선택 체크를 읽을 수 있다.
하단 첫 버튼은 Codex, 둘째는 모델·강도로 구분돼 기존 모델 진입점 중복이 제거됐다.
390×844에서 Claude Code 전체 이름, 현재 낮음 강도,44pxsend가 유지되고 긴 모델만 압축된다.
popup은278px폭으로 pane 안에 들어가며 모델 부제와 slider/reset을 조작할 수 있다.
Pretendard Variable 실제 로드와44px hit area/포커스 복귀를 확인했다. 다크 popup의 computed
foreground/background는 기존 DESIGN tokens `oklch(0.96 0.008 265)`/`oklch(0.155 0.018 268)`;
이미지에서 한글 라벨·모델·현재값 대비가 읽히고 새 색·표면을 추가하지 않았다.

## Preserve / refinement

기존 glass·textarea·roundsend·실제 진행/취소, static track과 supported-only 목록을 보존했다.
선택 버튼은 자연 폭, 긴 모델만 줄어들게 정리해 desktop에서 모델과 강도가 멀리 떨어지지 않게 했다.
지원하지 않는 Ollama 등에 의미 없는 기본 강도를 반복 표시하지 않으면서 모델 진입은 유지했다.
정상 입력부의 설명 문장을 되살리지 않았다. 오류/미설치·stale는 필요한 복구 안내를 보존했다.
신규 브랜드 후보3개 exemption은 direction 문서에 남겼고 별도 시각 방향/효과를 탐색하지 않았다.

## Remaining

다른 테마·실제 OS reduced-motion·native 설치형/Windows·전체 앱 성능 감사는 NOT RUN.
현재 delta 렌더는 Experience Contract와 일치한다. 전체 앱 미학 점수나 계정 모델 접근권을 주장하지 않는다.
실제 캡처 경로/viewport/상태·기능 검증은 [QA](../qa/2026-10-08-composer-provider.md) 참조.

#tags: render-critique, composer-provider, 반응형, 접근성
