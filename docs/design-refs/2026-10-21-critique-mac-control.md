# Render critique — Mac controls

2026-10-21 · 이번 실행의 실제 native/IAB 캡처만 사용. root와 읽기전용 독립 비평자가 같은 화면을 직접 관찰했다. 사용자 선택은 아직 없다.

## Current product

두 native 단계는 [제한 감사](../qa/mac-design-review-2026-10-21/review.md)에 있다. 원인은 색/블러만이 아니라 중복 제목·긴 설명·현재값/초안/동작의 위계와 중복 메뉴 연결이다. 스크롤 겹침은 AppShell.tsx424의 외부 utility-header, App.css472의 sticky/top0/z-index10·투명 배경,490의 비고정 적용 범위에 Mac 경로가 빠진 데서 발생한다. 향후 Mac 경로 modifier로 한정해 수정할 수 있으나 이번에는 변경하지 않았다.

## Wireframe observations

desktop5개 조작·기능2그룹·기록 직접 진입은 이해 가능했다. narrow는44px 제어를 다음 행에 두며 가로 잘림이 없었다. 두 값 모두 미선택일 때 의미가 모호해 styled3안에 ‘변경할 값 / 미선택’을 추가했다. 이후 반복은 이 구조를 보존하고 시각 구도만 비교했다.

## Actual render observations

- A: 이름·현재 저장값·변경 선택·명시 적용이 가까워 가장 직접적. 두 기능 그룹의 대비가 명확하고 desktop 첫 화면에서5개가 보인다. row에 짧은 상태가 반복되는 밀도는 실제 제품 반영 때 다시 볼 수 있다.
- B: 큰 기능 아이콘과 제목이 강하지만 첫 기능만 큰 이유가 약하며 narrow에선 스크롤이 가장 길다. 신규 기능으로 채운 카드가 아니라 같은5개를 다른 비중으로 배치한 비교안이다.
- C: 다른 메뉴로 보내지 않고 선택 즉시 같은 페이지의 상세가 바뀐다. desktop 분리는 명확하나 좁은 화면에서 조작이 목록 아래로 밀린다. C 선택 시 mobile 구조를 별도로 재검토한다.

독립 비평도 A 추천이며 B의 근거 약한 단일 강조·C의 narrow 이동 부담을 같은 한계로 관찰했다. 숫자 점수나 ‘macOS native와 동일’ 인증으로 바꾸지 않는다.

## Verified / limits

- Experience Contract validator PASS, worker의 최종 npm run check exit0, git diff --check PASS.
- root의 실제 Cua 조작3/3: A/B 확장자, C 숨김 파일의 선택→미적용 안내→mock 적용→기록→원래 null 복원. C 항목 전환은 동일 URL 내 detail만 바뀜을 확인했다.
- 실제390×844 세 안 모두 main 가로 overflow0·최소 visible 버튼/링크/summary 높이44px. viewport override는 reset했다.
- local Pretendard 소스/alias·경로는 확인했다. IAB read-only evaluate의 FontFaceSet에 load API가 없어 실제 font-face/글리프 적용 측정은 NOT RUN이다. 빈 fonts 배열을 실제 누락 버그로 판정하지 않는다.
- 전체 키보드 순환·실제 스크린리더·수치 대비·native glass·실제 OS apply/restore·Windows·제품 반영은 NOT RUN이다. 모의 동작을 설치본 검증으로 승격하지 않는다.

## Preserve / next

원래 제품·DB·권한·native guard·사용자 dirty파일은 유지했다. 시안3개와 근거 문서만 새로 만들었다. 사용자 선택 후 선택된 안으로 상태/원복/메뉴·제목 정책을 구체화하고 제품 회귀를 실행한다. 전역 스킬/토큰/자동화 변경은 없다.

#tags: mac-design, screenshot-critique, mock-flow, native-unverified
