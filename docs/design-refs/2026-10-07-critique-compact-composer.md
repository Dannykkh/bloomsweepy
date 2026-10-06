# Compact composer 렌더 비평·구현 결과

2026-10-07 KST. 사용자 참조 두 장과 [경험 계약](2026-10-07-experience-compact-composer.md)의 delta.

## Implemented

입력부는 전체폭 textarea와 작은 모델/추론 trigger,44px원형 전송 버튼 한 줄로 축소했다.
정상 설명을 제거하고 실제 모델/지원 단계/초기화만 팝업에 남겼다. 팝업은 최대360px이며
기존 토큰의 blue-violet track 한 곳만 정적 signature로 사용한다. unsupported 저장 ID와
stale effort는 숨기지 않고 경고 및 명시 reset을 제공한다. 큰 select는 Settings에서 재사용한다.
Claude/Grok/Agy 모델 catalog와 실제 argv 확장은 사용자 후속 요청에 따른 기능 범위다.

## Observed and repaired

- 첫 렌더에서 DESIGN의 semantic violet 이름과 실제 CSS 변수 이름이 달라 track이
  사라짐을 확인했다. 기존 `--accent-violet`로 수정하고 실제 렌더에서 재확인했다.
- 760×600에서 부모 chat의 overflow가 popup 상단을 자르는 문제를 확인했다.
  chat 작업면만 visible로 바꾸고 transcript의 제한 스크롤과 전역 경계를 유지했다.
- 독립 리뷰의 미지원 CLI 자동 model fallback을 수정했다. 추론 override가 없는 저장
  model만으로도 전송이 차단되고, Settings 명시 reset 후 복구됨을 실제 fixture로 확인했다.

## Render / accessibility evidence

1280×820,760×600,390×844에서 최종 다중 provider fixture를 다시 관찰했다.
최소 창 popup x107/y297/w360/h211, 좁은 화면 popup w278/h211로 컨테이너 안에 들어간다.
모델의 긴 라벨은 시각적으로 생략하되 전체 aria-label을 보존한다. send와 selector 높이44px.
화면 전환 CSS가 정착한 후 popup 가림·가로 overflow 없음. 좁은 화면은 보조 회귀 검사이며
이 제품의 실제 native 창 최소값은760×600이다.

실제 `document.fonts.status=loaded`, `check('14px "Pretendard Variable"','한글Aa')=true`.
computed font14px/현재 강도16px, 실제 한글·라틴 렌더를 확인했다. popup은 불투명
canvas-elevated로 native glass 뒤 배경에 따른 대비 흔들림을 줄인다. 실측 computed OKLCH를
선형 sRGB luminance로 환산한 text/muted/violet 대비는17.41/7.88/5.79:1이다.
range keyboard·Escape focus 복귀·Enter 비전송·명시 reset·busy locking은 fixture PASS.
reduced-motion CSS에서는 popup animation을 제거한다. OS preference 강제 변경·스크린리더
실사용·지속 FPS 측정은 NOT RUN이며 접근성 인증을 주장하지 않는다.

## Judgment

모델/강도 조절이 대화 높이를 차지하지 않고 기존 입력·진행·취소 계약을 유지한다.
실제 기본 강도가 미확인인 CLI는 처음부터 가짜 slider 위치를 만들지 않고 지원 버튼을
보여 준다. 강도를 선택한 후에는 같은 range로 바뀐다. 참조의 미구현 microphone과
상시 particle 모션은 제외했다. 이런 적응은 저자원 Mac에서 실제 기능과 상태를 우선한다.

최종 ARM 설치본에서도 Claude 지원5버튼→선택 range와 Codex range의 실제 렌더를 확인했다.
정상 하단 설명은 없고 기존 진행/취소/입력 공간은 유지된다.
[설치된 Codex 화면](2026-10-07-compact-composer/installed-codex.jpg)과
[설치된 Claude 화면](2026-10-07-compact-composer/installed.jpg)은 fixture가 아닌 실제 창이다.

새 dependency/상시 timer/Canvas/WebGL/blur는0이다. Vite JS945.17kB/CSS194.08kB,
기존500kB chunk warning은 남는다. 이 변경을 번들 최적화나 장시간 메모리 검증으로
확대해서 보고하지 않는다. 설치형 증거와 범위는 [QA](../qa/2026-10-07-multi-provider-model-selection.md)를 따른다.

## 후속: 목록이 없는 공급자

2026-10-07 약03:05 KST, 기존 compact 구조와 토큰을 유지한 예외 상태 안내다.
미설치/로그인/서비스/조회 상태와 실제 목록을 구분하며 Grok/Agy의 공식 예시는
별도 읽기 전용 section으로만 표시한다. 정상 목록이 들어오면 section은 사라진다.
부재한 지원 강도/실행 가능한 모델을 꾸미지 않고 상시 하단 설명은 되살리지 않는다.

760×600/390×844 합성 화면에서 가로 overflow/팝업 clipping 없음, Agy7행 bounded scroll과
설치 안내의 Tab 접근을 확인했다. 좁은 화면 팝업 x25/y313/w278/h420으로 창 안에 들어간다.
링크44px·focus2px·한글 폰트 로드 확인. primary link 초기 대비4.46:1은 기준에 못 미쳐
기존 text token17.41:1로 바꿨으며 underline으로 링크 식별을 유지한다.
disabled placeholder에 현재 선택 체크/aria-selected를 붙이지 않는다. 문서 예시는 list,
번역하면 안 되는 모델 label/ID는 translate=no로 표시한다.

[Grok 미설치 예시](2026-10-07-compact-composer/grok-uninstalled.jpg)와
[Antigravity 미설치 예시](2026-10-07-compact-composer/antigravity-uninstalled.jpg)는 합성 QA다.
이 단계 당시 새 bundle의 실제 설치 렌더는 Mac 잠금 때문에 NOT RUN이었다.
후속 사용자 교체 요청에서 잠금 해제 후 새 bundle을 설치하고 실제 창을 확인했다.
[설치된 Grok 빈 목록](2026-10-07-compact-composer/installed-grok-empty.jpg)과
[설치된 Antigravity 빈 목록](2026-10-07-compact-composer/installed-antigravity-empty.jpg)은 실제 설치본 증거다.
검증 범위와 보존 확인은 [QA 설치 교체 절](../qa/2026-10-07-multi-provider-model-selection.md#후속-사용자-요청에-따른-설치-교체--2026-10-07-kst)을 따른다.
최종 JS954.59kB/CSS196.59kB, 기존 chunk warning과 장시간/FPS/OS 스크린리더 미검증 한계 유지.

#tags: 채팅ui, 렌더검증, 컴팩트입력창, 접근성, 빈카탈로그, arch:012
