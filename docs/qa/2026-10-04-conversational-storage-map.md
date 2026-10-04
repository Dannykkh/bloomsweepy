# 큰 항목 발견 및 대화·용량지도 공유 검증

2026-10-04 · Codex · 현재 개발본, GitHub v1.7.0 배포물과 별개.

## 요청과 구현 경계

“여기서 가장 용량이 큰 폴더나 데이터는 뭐야? 삭제해도 되나? 찾아줄래?”를 `files/largest` 읽기 전용 새 검사로 처리한다. 직계 항목 비교이며 폴더는 하위 합계다. 이름/크기/수정일만으로 백업·필요성·재생성 가능성을 확정하지 않는다. 선택/삭제 계획은 만들지 않는다. 명시적 제거 요청과 최종 로컬 승인 흐름은 유지한다.

scan/largest/browse/parent가 기존 단일 bounded directory 보고서를 교체한다. 세션에는 지도 generation만 추가한다. 카드와 타일맵은 같은 데이터와 generation을 사용하고, 지도 보기에는 추가 파일 검사가 없다. 이름 검색만 한 폴더는 미측정이며 하위 탐색 후 지도에 반영한다. 영구 트리 캐시나 전체 과거 탐색 결과의 누적은 없다.

## 실행 결과

- `cargo test --workspace --lib`: Rust275 통과, opt-in3 제외. 이번 변경의 공유/미선택/계획 없음/경계/세션·revision·generation 만료/이름 검색 미측정 회귀2개 포함.
- `cargo clippy --workspace --lib -- -D warnings`: 통과.
- `npm run check`, `npm run test:all`: TypeScript 및 frontend43 통과; 신규5개 문구의 ja/zh-CN catalog/placeholder 일치 포함.
- `npm run build`: 통과. 기존 단일 JS 번들500 kB 경고는 남음.
- opt-in `live_codex_file_tool_contract`: 실제 설치 CLI에서 합성3요청 통과. 일반 검사→scan, 명시적 promo-video 제거→review_named, 현재 사용자 질문→largest. 사용자 파일 실행/삭제는 이 계약 검사에서 수행하지 않음.
- 합성 UI: 같은 보고서1,420,002,400 B를 카드와 타일맵에서 확인. 질문 후 선택0/계획 없음/Mock executions0. 지도 동기화는 채팅을 유지하고 버튼/키보드 Enter로 지도 전환, 대화 복귀 성공.
- 합성 UI: 이름 검색 시 폴더 용량 미측정 및 지도 버튼 미노출. expired map 조회는 재검사 안내, 지도 화면 전환 없음.
- saved sequence와 pending index가 같은 React 행 키를 만드는 오류를 발견해 구분했다. 새로고침→검사→지도→복귀→검색 후 신규 warning/error0.
- 1280px 기본 화면, 760×600 지도, 325×720 카드에서 가로넘침 없음(각 scrollWidth1280/760/320). 지도 버튼 높이44px. 좁은 카드의 아이콘/텍스트 간격을 inline-flex6px로 수정. 폰트 loaded 상태 확인.

## 디자인 델타 감사

frontend-design의 기존 Data Instrument/글래스 방향, DESIGN 토큰, 기존 카드/타일맵을 유지한다. web-interface-guidelines 원문을 조회했고 changed controls에 semantic buttons, 이름 있는 아이콘, async disabled, 오류 안내, 미선택, 키보드, wrap, bounded rendering을 적용했다. 새 섹션/브랜드 재디자인/장식 모션/외부 에셋 없음.

| 영역 | 점수 | 근거/한계 |
|---|---:|---|
| 다크/라이트 | 7 | 다크 실제 렌더 확인, 기존 theme 토큰 재사용. 라이트 native 검증 NOT RUN |
| 반응형 | 8 | 325/760/1280px 확인, 목록 자체 세로 스크롤 유지(CSV Layout16, Responsive69) |
| 접근성 | 8 | 이름 있는44px 버튼, 미선택 체크박스, Enter 전환·focus outline(CSV Accessibility40/41) |
| 로딩/성능 | 8 | 중복 요청 차단, 단일 bounded snapshot, 새 재검사 없음. 번들 경고/장시간 계측 남음(CSV Interaction32, Performance48) |
| 폼 UX | 8 | 기존 search label/maxLength/submit disabled, 이름 결과 미측정 구분(CSV Forms61/62) |
| 네비게이션 | 8 | 자동 동기화 시 대화 유지, 명시적 지도 버튼·동일 root·만료 안내(CSV Navigation3/6) |
| 타이포/간격 | 8 | 지역화 숫자/날짜·줄바꿈·6px 아이콘 간격(CSV Content85/86) |
| 애니메이션 | 8 | 새 모션 없음, 기존 reduced-motion 계약 유지(CSV Animation9) |
| AI Slop | 9 | 실측 이름/크기, 역할·삭제 경계 명확, 가짜 안전 점수/결과/장식 카드 없음 |

변경 부분의 정성 평가이며 제품 전체 완성도나 Windows 인증 점수가 아니다. 스크린샷은 CUA 도구에서 직접 관찰했으며 파일로 저장된 증거라고 주장하지 않는다.

## 설치 및 미완료

arm64 app bundle 빌드 성공. 호스트 및 기존 sidecar2개 ad-hoc 서명, `codesign --verify --deep --strict` 통과. `/Applications/BroomSweepy.app`의 최종 호스트 SHA256 `e832fe1d31af3fb34ad5ab87c5bff19d015edef1285163adb19d234617a8ceb5`. 이전 앱은 `/private/tmp/broomsweepy-map-rollback-9UnzVB/BroomSweepy.app`에 보존. 최초 지도 패치 중간본은 `/private/tmp/broomsweepy-map-intermediate-2pp20r/BroomSweepy.app`에 보존. 사용자 파일/기존 대화/휴지통을 보존하며 앱 본체만 교체했고 QA 질문·응답은 앱의 기존 대화에 추가됐다.

최초 설치 패치에서 실제 Codex 질문→앱4.0초 검사→31개 항목/선택0→동일7.2 GB 지도, target6.8 GB/하위21,610파일·2,090폴더가 카드/타일/순위에 일치했다. Native 픽셀도 확인했다. 백그라운드 창 캡처가 애니메이션 중간 화면을 보일 때 Window 메뉴의 현재 창 선택 후 정상 렌더를 확인했으며 이를 소스 기능 실패로 단정하지 않았다. 지도 헤더/다시 보기/자세한 검사의 범위가 다른 global root나 null이 아닌 실제 지도 범위를 따르도록 수정해 최종 빌드에 반영했다.

설치 후 이번 작업에서 새로 만든 `target/aarch64-apple-darwin`만 정리(약800MB)했고 기존 `target/debug`는 보존했다. 직접 복구하는 파일은 아니지만 소스로 재빌드 가능하다. 이전 앱 복구본은 삭제하지 않았다.

최종 설치본에서 17:33–17:43 KST 실제 Codex와 현재 사용자 질문을 다시 검증했다. 프로젝트 직계31개 항목에서 target6 GB가 최상위로 나왔고 선택0/최종 검토 비활성이었다. “그럼1번폴더는삭제해도돼?”라는 후속 질문은 제거 계획이 아니라 target 내부의 읽기 전용 browse로 이어졌다. debug5.4 GB, release600.7 MB 등4개 항목/선택0이 표시됐다. 추가 판단 질문에는 실제 Codex가 Rust 빌드 결과·캐시로 추정한 근거, 재생성 조건, 별도 보관 자료 확인 필요성을 설명하고 파일 내용을 읽지 않았다고 명시했다. AI 설명은 십진 GB, 앱 표시는 이진 기반 GB여서 표기 수치가 다르며 같은 정확 바이트 보장은 앱 카드/지도 간 계약이다.

같은 결과 지도 버튼에서 실제 target 범위와 debug5.4 GB/release600.7 MB가 카드·타일·순위에 일치했다. “처음 폴더 다시 보기”는 폴더 선택창 없이 target을 읽기 전용 재검사했고 297ms/접근 제한0으로 완료했다. 실제 다크 렌더를 확인했다. PID26622의 마지막 host RSS는56,784 KiB였으나 이는 WebKit·CLI를 포함한 최대 메모리나 장시간 검증이 아니다. 합성 임시 탭과 Vite 서버는 종료했고 viewport를 복원했다.

Windows runtime, 라이트 native, 장시간 전체 프로세스/최대 메모리 계측은 NOT RUN. 이번 작업에서 사용자 파일 삭제/휴지통 비우기는 실행하지 않았다. 커밋/푸시/릴리스는 요청하지 않아 수행하지 않았다.
