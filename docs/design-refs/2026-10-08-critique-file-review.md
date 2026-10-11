# Render critique — 변경 파일 검토

기존 Settings/Data Instrument 방향의 국소 확장이다. 새로운 브랜드·내비게이션·카드 그리드 방향을 선택하지 않았다.
Product Design adapter ABSENT 상태에서 기존 토큰과 로컬 제품 컴포넌트를 사용했다.

## 관찰과 수정

- 1280px: 범위/주기 → 4열 검사 상태 → 검토 행 순서 유지. 불필요한 대시보드 지표/안전 점수 없음.
- 390px: 폴더/동작은 세로 배치, 상태는 단일 열, 긴 경로와 행은 줄바꿈. 두 폭에서 가로 넘침 없음.
- 실제 Tab focus outline 2px, 조작 44px, 버튼/select 14px, 글꼴 loaded 확인.
- 폴더 용량 0B 오인을 제거하고 미측정으로 표시. 넘김은 안전 판정이 아님을 유지.
- 생략은 오래된 기록만 버린다는 표현을 수정: core report cap도 새 변경 일부를 생략할 수 있어 전체 목록 아님을 명시.
- config 저장 실패에 성공 알림이 뜨지 않도록 status.error를 사용자 오류로 표시.

## Delta audit

반응형/다크 테마/긴 콘텐츠/폼 레이블/키보드 포커스/진행·실패·미연결 상태/페이지 경계/자원 상한은
해당 스크린샷·DOM·테스트 범위에서 검토했다. 새로운 외부 링크·인증·유료 동작·장식 모션 없음.
reduced-motion CSS의 눌림 축소 비활성화를 확인했다. 전체 접근성 자동 감사, light theme,
screen reader, OS motion 설정 전환, 설치 WebView 종단은 NOT RUN이며 완전 준수 점수를 만들지 않는다.

증거와 실행 명령은 `docs/qa/2026-10-08-file-change-review.md`를 정본으로 한다.
