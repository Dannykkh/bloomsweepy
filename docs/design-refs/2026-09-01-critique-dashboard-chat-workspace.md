# Critique: 대시보드와 폴더 중심 대화 후보

## Candidate A — Activity Dashboard

- Screenshot: `.termsnap/design-candidates/dashboard-chat-a.png`
- 드라이브를 행으로 비교하고 최근 정리와 새 파일을 1.7:1로 나눠 현재값과 변화가 한 화면에 들어온다.
- 수치가 열로 정렬돼 여러 드라이브 비교가 빠르고, 기존 다크 단층 표면을 유지한다.
- 선택: 별도 대시보드 요구와 Data Instrument 계약을 가장 직접적으로 만족한다.

## Candidate B — Drive Card Wall

- Screenshot: `.termsnap/design-candidates/dashboard-chat-b.png`
- 드라이브별 독립 블록은 각 볼륨을 크게 보이게 하지만 비교 열이 깨지고 작은 수치보다 막대가 과도하게 커진다.
- 탈락: 사용자가 지적한 박스 과밀을 다시 만들고 760px에서 스크롤 비용이 커진다.

## Candidate C — Chat First

- Screenshot: `.termsnap/design-candidates/dashboard-chat-c.png`
- 폴더 범위, transcript, composer가 한 pane에 있어 대화 화면 자체의 구조는 가장 명확하다.
- 탈락이 아니라 별도 `대화` 화면의 구조로 채택한다. 앱 첫 화면으로 쓰면 전체 디스크 상태가 사라지므로 대시보드를 대체하지 않는다.

## Preserve

- A의 행 기반 드라이브, 하단 비대칭 활동 목록, freshness 문구.
- C의 폴더 scope 한 줄, 넓은 transcript, 하단 composer.
- 기존 14px 최소 글자와 760×600 세로 스크롤.

## Remaining Verification

- 실제 런타임 값에서 드라이브가 1개·5개 이상일 때 첫 화면 밀도를 확인한다.
- 파일 목록 없음·기준 목록만 있음·새 파일 있음 세 상태를 확인한다.
- 작업 저널 손상·없음·부분 완료 상태가 대시보드 전체를 막지 않는지 확인한다.
- Codex 미설치·로그아웃·정상 응답을 실제 subprocess로 확인한다.
