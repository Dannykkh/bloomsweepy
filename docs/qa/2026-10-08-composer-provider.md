# Composer CLI selector QA

Date: 2026-10-08, Mac development checkout. 설치된 앱과 구분한다.

## Automated

- `npm run check`: PASS (fixture HMR cleanup 포함 최종 소스).
- `npm run test:all`:119 PASS,0 failed/skip. 기존 공급자별 모델/강도 저장·지원 metadata·stale·locale 회귀 포함.
- `npm run build`: PASS. 변경된 제품 코드의 production bundle 생성, 기존500kB chunk 경고 유지.
- Experience Contract validator: PASS, warnings0.
- `git diff --check`: PASS.
- 독립 read-only review: Controls/View/shared hook 지정 변경에서 의미 있는 회귀 발견0.

## Synthetic runtime

실제 React 컴포넌트 + 기존 Tauri mock fixture. 실제 CLI, 파일 작업, 외부 AI 전송 없음.

| Scenario | Observation | Result |
|---|---|---|
| Codex→Claude→Grok→AGY→Ollama | 각 provider 모델/지원 강도, CLI 기본값 분리 | PASS |
| 모델별 지원 | Codex Fast6단계, Claude5, Grok4, AGY Pro2, Ollama0 | PASS (합성 metadata) |
| 초안 | provider 전환 후 동일 초안 유지, 선택/Escape 실행0 | PASS |
| Codex 복귀 | Fast/ultra 복원, Settings에서 확인 | PASS |
| Settings 공유 | Grok4.7/xhigh 표시→Codex Fast/ultra→medium→chat 반영 | PASS |
| 재로드 | Codex Fast/medium 유지 | PASS (WebView 재시작 아님) |
| 응답 중 잠금 | 두 trigger disabled, mock request grok/grok-4.7/xhigh | PASS |
| 취소 | 기존 선택 유지·두 trigger 활성화 | PASS |
| provider.busy | 두 trigger disabled→상태 재확인 후 활성화 | PASS |
| 미설치 Grok | 상태 목록에서 선택 가능, 저장 ID/effort 경고 유지, send 차단 | PASS |
| 공식 참고 | 미설치 popup의 별도 읽기 전용 목록/설치 링크, provider popup에는 없음 | PASS |
| 키보드 | 선택행 focus, Home→첫 항목, Enter CLI 선택, Escape trigger 복귀 | PASS |
| 390px | CLI/강도 유지·모델 압축, popup/send 겹침 없음 | PASS |

390px 실측: composer x14–366, CLI x25–136.17, 통합 x138.17–303, send x311–355;
각 button44px높이, popup x25–303. Pretendard Variable computed family·font status loaded·font check true.

스크린샷:

- [desktop](screenshots/2026-10-08-composer-provider-desktop.jpg):1280×820 dark, 약12:24 KST.
- [mobile](screenshots/2026-10-08-composer-provider-mobile.jpg):390×844 dark, 약12:21 KST.

fixture provider label은 제품 이름과 일치하게 바꿨으며 합성 표식/모델 이름·응답/카운터를 유지했다.
fixture source HMR 중 createRoot 중복 warning을 관찰해 module disposal에서 root를 unmount하도록
수정했다. 제품 main entry가 아니라 QA harness cleanup이며 native RAM 개선 근거로 사용하지 않는다.
최종 소스의 별도 새 탭에서 시작·저장 선택 복원을 확인하고 콘솔 error0을 확인했다.
검증용 탭을 모두 닫고 뷰포트를 복원했으며 직접 실행한 Vite 서버도 정상 종료했다.

## NOT RUN / boundaries

실제 설치본 교체·새 native IPC/CLI 생성·계정 접근권·서버 강도 적용·Windows·장시간 메모리
검증은 NOT RUN. 제품 변경은 frontend와 선호 read-only snapshot guard이며 native 계약은 그대로다.
공간 약1.3GiB 환경에서 Rust/Tauri 전체 재빌드·설치는 하지 않았다. 다른 테마/실제 OS reduced-motion
모드는 NOT RUN; 기존 토큰과 CSS reduced-motion 규칙은 유지했다. 비동기 경합의 revision/live snapshot
검사는 코드 리뷰로 확인했고 지연한 실제 host 경합 실험은 NOT RUN.

#tags: qa, composer-provider, cli선택, 모델선택, 추론강도, arch:012
