# 맥 관리 화면 — 제한 범위 UI/UX 검토

2026-10-20, Codex. 대상은 신규 MacManagementView 실제 컴포넌트와 모의 native bridge이며 앱 전체나 설치형 UI 감사를 대신하지 않는다. DESIGN.md/Data Instrument, frontend-design과 ui-ux-auditor 내부 모듈을 적용했다. 기존 토큰·행 기반 작업면·명시적 상태를 유지했다.

메인이 [1280px](../../qa/screenshots/2026-10-20-mac-settings-desktop.jpg)와 [390px](../../qa/screenshots/2026-10-20-mac-settings-narrow.jpg) 캡처를 직접 열어 확인했다. 전체5행·관련 화면 연결·기록이 잘리지 않고 표시된다. 앞선 fixture의 중첩 스크롤/빈 하단을 수정한 뒤 재촬영했다. 캡처는 모의 데이터임을 하단에 명시한다.

## 관찰 점수

0–10의 제한 범위 평가이며, 미검증 항목을 최고점으로 간주하지 않는다. CSV 근거는 ui-ux-auditor/references/ux-guidelines.csv의 이슈 번호다.

| 영역 | 점수 | 관찰·근거·한계 |
|---|---|---|
| 다크/라이트 | 8 | 기존 제품은 dark 고정이다. 텍스트·선택기·아이콘이 dark 캡처에서 보이며 root color-scheme도 dark. 별도 light 테마 지원 추가나 모든 합성 대비의 수치 검증은 하지 않음 (#36) |
| 반응형 | 8 | 1280/390 가로 overflow0, 좁은 화면 행 재배치, 버튼/select 높이44px 이상 실측. 태블릿/모든 배율 미검증 |
| 접근성 | 7 | Tab 첫 버튼 focus 표시, 네이티브 select/labels, status/alert, 상태를 글자로 구분. 전체 키보드 순환·스크린리더/수치 대비는 미검증 (#28/#37/#41) |
| 로딩/성능 | 7 | 읽기·변경·오류/빈 상태·중복 차단·화면 이탈 경로가 정식/정합된 ARM64 격리 환경에서10/10 통과했다. 초기 alpha환경 정지의 세부 원인과 설치형 성능은 미검증. 기존 앱 번들 큰 청크 경고도 남음 (#10/#32/#50) |
| 폼 UX | 8 | 미설정≠꺼짐, 선택은 초안, 명시 Apply와 same-render 중복 차단, 복원은 원래 부재까지. 실제 Finder 쓰기는 테스트하지 않음 (#32/#61) |
| 내비게이션 | 7 | 성능/앱/정리 이동 버튼이 일정한 위치에 표시됨. 기존 앱 shell 통합은 소스/타입 검증, 이번 캡처는 단독 fixture이며 전체 shell 렌더를 증명하지 않음 |
| 타이포/간격 | 8 | Pretendard 실제 face loaded, 최소14px 실측, 24/18/16px 계층과 행 구분, 본문1.5 줄간격. 좁은 화면 설명은 길어 다소 밀도 높음 (#50/#72/#74) |
| 모션 | 8 | 장식 움직임을 추가하지 않음. reduced-motion match와 CSS 전환 제거 확인. 네이티브 창 전환은 미검증 (#7/#9/#13) |
| AI Slop | 9 | 장식용 hero/블롭/상투적 중앙 정렬 피처 카드 없음. 관련 화면3개는 실제 이동 작업이고 신규 기능 홍보 그리드가 아님. 기존 glass/blue 토큰 재사용 |

가중 참고 점수7.7/10(B*, 제한 범위). 초기 비교 전 점수7.5에서 격리10/10 실행 증거로 로딩/성능만5→7로 갱신했다. **출시 승인 지표가 아니다.** CSS/화면 검토를 기능 성공으로 바꾸지 않는다. 실제 macOS glass·Finder 반영·설치형·전체 앱 테마 검증은 NOT RUN이다. 최종 두 캡처는21:02 KST 실행 뒤 메인이 다시 직접 관찰했다.

## 수정·발견

- 실제 컴포넌트의 지연 state updater에서 event.currentTarget을 읽던 오류: 이벤트 시점 값 캡처로 수정.
- fixture의 중첩 스크롤/긴 빈 하단: fixture 호스트에만 auto height/visible overflow 적용 후 전체 캡처 확인.
- fixture favicon404: data favicon으로 제거. 제품 네트워크 오류를 숨긴 것이 아님.
- 테스트 실패 보고의 무제한 evaluate: Node watchdog으로 제한. timeout은 그대로 실패다.
- 초기 alpha/설치 Chrome의 간헐 정지는 세부 원인 미확정이지만, 정식 Playwright/전용 ARM64 브라우저의 동일10경로는 모두 통과했다. Minos 보고서 최신 결과가 정본이다.

#tags: ui검토, 반응형, 접근성, 브라우저검증, arch:016
