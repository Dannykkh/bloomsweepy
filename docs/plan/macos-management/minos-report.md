# Minos 실행 결과 — Mac management

2026-10-20, macOS26.4 Apple Silicon/8GiB. 실제 컴포넌트 + Tauri mock IPC, 실제 Finder writes0, 사용자 파일 삭제0. 최신 제품 소스의 핵심 검증과 브라우저 자동화 실패를 구분한다.

## 결과

**최종 전체10경로 실행: 10 PASS / 0 FAIL, 통과율100%, exit0(session32142). Minos gate 통과.** 정식 Playwright1.64.1 + ARM64 Node + 전용 ARM64 Chrome Headless Shell156.0.8078.4(v1248) 환경이다. 초기 alpha/설치 Chrome 환경의7/10 실패 이력은 아래에 보존한다.

| 경로 | 결과 |
|---|---|
| 초안→같은 렌더 중복 Apply1회→busy 잠금→Restore1회→원래 부재 | PASS |
| managed/type/error 값 변경 금지 | PASS |
| 미지원 OS에 변경 컨트롤 없음 | PASS |
| 읽기 실패 안내/새로고침 가능 | PASS (초기 환경 alert timeout 후 격리 재검증) |
| write-error 후 stale 잠금/명시 새로고침, 자동 재시도 없음 | PASS |
| external-change 후 stale 잠금/새로고침 | PASS (초기 환경 click timeout/applies1 후 격리 재검증) |
| 복원 실패가 성공/재시도로 바뀌지 않음 | PASS |
| 변경 중 화면 이동→늦은 응답이 화면을 다시 열지 않음→재진입 상태 | PASS (초기 환경 완료 로그 timeout 후 격리 재검증) |
| 1280px overflow/font/Tab/14px/44px/reduced motion | PASS |
| 390px 동일 검사/전체 화면 캡처 | PASS |

11개 QA 그룹(MAC-01–11) 중 브라우저의 세부 경로10개이며, Rust 테스트 수와 합산해 UI 통과율을 부풀리지 않는다. 실패 assertion은 유지했고 skip/fixme 코드로 감추지 않았다. 세 경로만 재실행한 session46138도3/3, 이어 전체 session32142가10/10 통과했다. 격리 비교 중 제품 View 코드와 핵심 assertion을 추가 수정하지 않았다.

## Healer와 원인 분류

1. 개발 중 실제 React 오류: deferred state updater에서 event.currentTarget을 읽음. 이벤트 시점 문자열 캡처로 제품 코드를 수정했다.
2. busy 검사 타이밍: 초기 읽기까지 느린 fixture를 읽기200ms/쓰기1800ms로 분리해 처리 중 상태를 검증했다.
3. fixture favicon404는 data favicon으로 해결. fixture의 중첩 스크롤/긴 빈 하단은 호스트 한정 height/overflow 수정 후 재캡처했다.
4. 간헐 대기: visible 문서에서도 초기 읽기/처리 완료가 멈추는 현상. DOM log는 React state이므로 화면의 reads0만으로 실제 invoke0이라고 단정하지 않았다.
5. 실패 진단 자체의 무제한 page.evaluate를 발견해 Node watchdog을 추가했다. direct stats getter/선택적 profiler/브라우저 환경 옵션도 추가했다. timeout 확대는 원인 해결이나 성공으로 간주하지 않는다.
6. 계측 재실행(session91542): 실패3개만 재실행하여0/3, 모두 DOMContentLoaded15s timeout. 진단 evaluate2.5s 및 profiler stop5s도 응답 불가. 원시 CPU profile을 얻지 못했으므로 핫 함수나 제품 원인을 꾸며내지 않는다. 서버HTTP200은 확인했다.
7. 독립 환경 비교: headful Chrome + --disable-gpu, profiler 끔(session30706)도0/3, exit1. alert/초기 ready30초 timeout과 DOM 진단 응답 불가가 남았다.
8. 정식1.64.1/정합 Headless Shell156/ARM64 Node 환경으로 분리한 뒤 실패3경로3/3, 전체10경로10/10 PASS. alpha패키지·브라우저·Node 아키텍처가 함께 달라졌으므로 특정 하나를 원인으로 확정할 수 없다. 기존 alpha/설치 Chrome 조합은 향후 검증 환경으로 사용하지 않는다.

초기 정지의 세부 원인은 **미확정: 자동화/브라우저/아키텍처 환경 경계**이다. 정합된 격리 환경의 통과는 해당10경로의 증거이며 실제 설치형 전체 UI나 어느 브라우저에서도 정지하지 않는다는 보장은 아니다. Chrome renderer의 CPU 고부하만으로 제품/Chrome 결함이라고 단정하지 않는다.

## 반복 실행

기존 Vite fixture를 loopback1431에서 실행한 다음, 설치돼 있는 Playwright와 브라우저만 사용한다(자동 다운로드 없음).

```sh
PLAYWRIGHT_BROWSERS_PATH=/absolute/path/to/test-browsers \
BROOM_PLAYWRIGHT_MODULE=/absolute/path/to/playwright-1.64.1/index.mjs \
/absolute/path/to/arm64/node scripts/test-mac-settings-ui.mjs
```

실패 경로 필터는 `BROOM_QA_CASE='load-error,external-change,late response'`다. `BROOM_QA_PROFILE=1`은 선택적 CPU sampling이며 원시 파일은 임시폴더에만 쓴다. Chrome context/browser는 finally에서 닫고, 진단 timeout을 PASS로 바꾸지 않는다.

## 기타 실행 근거

- Clock fix 포함 Rust workspace:478 PASS/6ignored, exit0(session76735).
- 실제 고정 Finder 읽기 전용 smoke:1 PASS, macOS26.4aarch64, 다섯 설정 unset/None, Finderwrite0/journal생성0(session20459).
- 최신 TypeScript check exit0(session18310), frontend136/136 PASS(session47965), production build PASS(session12779).
- 1280/390 스크린샷은 메인이 실제로 열어 전체 내용·행·컨트롤·기록 표시를 확인했다. [제한 UI 감사](ui-audit.md).
- 실제 Finder 변경, 사용자 캐시 삭제, 설치형 교체/E2E, Windows 실행은 NOT RUN. 설치 성공으로 표현하지 않는다.

## 최종 추가 기록

2026-10-20 21:02 KST, full session32142 exit0: passed10/failed0/actualOsWrites0。 Playwright1.64.1 registry확인 및 버전 실측, Headless Shell156.0.8078.4 arm64 전용 다운로드, 실행 Node arm64 실측. 기존 기본 Node는x64였으나 그것만이 원인이라고 단정하지 않는다. 1280/390 최신 캡처를 메인이 다시 열어 전체 표시를 확인했다. 전용 브라우저215MiB는 이번 비교만을 위한 임시 산출물이며 정리 후 재실행에는 같은 고정 버전 브라우저 준비가 필요하다.

#tags: minos, 브라우저테스트, 미해결타임아웃, 검증한계, arch:016
