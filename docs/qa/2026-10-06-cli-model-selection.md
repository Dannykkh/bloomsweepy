# CLI 모델 선택 QA

2026-10-06 KST · macOS ARM648GiB · 현재 개발본, 공개 v1.7.0과 구분.

## 자동 검사

- `npm run check`: PASS. 처음 실행한 `npm run test`는 정의되지 않은 script로 NOT RUN이며,
  실제 프로젝트 명령인 `npm run test:all`로74개 PASS(신규 선호8개 포함).
- `cargo test -p bloomsweepy-desktop --lib --release --target aarch64-apple-darwin -j 1`:
 225PASS/3ignored. 설치CLI 진단·실제Codex계약·nativeTrash opt-in은 이 검사에서 미실행.
- visibility 실제값 수정 후 provider 필터:31PASS/2ignored. 신규6개 모델 관련 검사 포함.
- ARM64 app-only production build PASS, Tauri JS청크 약919kB 경고는 남음. 신규 의존성 없음.
- `git diff --check`, rustfmt PASS. 권한/기존 삭제 회귀 테스트 포함.

## 합성 UI — 실제 공급자/파일 작업 없음

자체 Vite1420의 assistant-tools-fixture, CUA만 사용.

| 시나리오 | 관찰 |
|----------|------|
| Codex default→Deep 선택→질문 | request model fixture-deep, provider codex, synthetic 답변 |
| Claude 전환 | Codex ID가 유출되지 않고 기본값; Sonnet 선택 후 Codex는 Deep 복원 |
| Settings 변경→chat→reload | Fast 선택이 같은 창/다시 로드 후 유지 |
| catalog 실패 | 선택 fixture-fast 유지, “목록에서 확인되지 않음” 및 unavailable 설명 |
| CLI 기본값 질문 | request model null; mock footer CLI default |
| bundled 출처 | 내장 목록/계정 사용권 미확인 설명 |
| Ollama | 설치 fixture-local:small 선택, required 계약 |
| busy Settings | provider/model disabled, refresh enabled; 다음 상태 조회 후 두 선택기 복구 |
| metadata 없는 구형 host | Codex 기본값 disabled picker, 질문 가능 |
| 기존 native-shaped auto-trash | 명시 요청1회→mock 실행1회; 실제 파일·AI 전송 없음 |

390×844에서 composer bottom672·select height44·document scrollWidth380/viewport390.
1280×720에서도 가로 overflow 없음. 글자14px·Pretendard 실제 로드, 키보드2px outline
확인. OS 전체 접근성/모션 설정은 변경하지 않았다. 기존 reduced-motion 규칙은 유지한다.

## 설치형 관찰과 수정

첫 ARM64 설치에서 Codex ready이나 catalog empty/unavailable을 발견했다. 실제 live
`debug models` exit0/354470B/models7 중 list4·hide3, bundled exit0/517840B/models11 중
list6·hide5. parser가 show_ui만 받던 것이 원인. 실제 공개 list와 호환 show_ui만 명시
허용하며 hide/unknown은 제외하도록 수정. CLI 환경/cwd/인증 문제로 오판하지 않는다.

첫 설치에서도 Claude sonnet 선택이 Settings와 공유되며 기존 Remember·확인 생략 ON을
변경하지 않고 복원한 것을 확인했다. 최신 visibility/44px 수정 후 실제 설치형 결과는
아래 최종 검증에 기록한다. 합성 답변을 실제 응답으로 보고하지 않는다.

## 범위 한정 UI 감사

frontend-design의 Agent Workbench·effect budget0 적용. 기존 글래스/폰트/하단 dock 유지,
native select만 크기/표시를 보정. 새 blur·애니메이션·자산·추론 강도 UI 없음. 최신 Web
Interface Guidelines의 label/name/focus/dark options/min-width/44px 검토.

| 영역 | 점수/10 | 근거·한계 |
|------|---------|-----------|
| 테마 |9| 고정 dark, 기존 재질/글자/option 대비; light테마 없음 |
| 반응형 |9|1280·390 실제 렌더, dock/select 유지 |
| 접근성 |8| labelled select/name/description·keyboard outline·44px; 전체 OS스크린리더 NOT RUN |
| 로딩/성능 |8| 상태 메시지·카탈로그 상한, busy 복구; 장시간/FPS NOT RUN |
| 폼 UX |9| provider별 기억·default·실패/stale 설명·실행 중 제한 |
| 내비게이션 |9| 기존 Settings/Chat 위치, 공유선호 |
| 타이포/간격 |9|Pretendard 로드·14px·narrow reflow |
| 애니메이션 |10|신규 모션 없음, 기존 reduced-motion 유지 |
| AI Slop |9|앱 과업에 필요한2개 선택기, 장식/새 효과 없음 |

가중8.85/10(B). 변경 컨트롤 범위의 렌더 리뷰이지 앱 전체 출시/보안 인증이 아니다.
screenshots/2026-10-06/model-picker-keyboard.png, model-picker-390.png는 로컬 ignored
증거이며 마케팅 이미지/공개 릴리스 파일이 아니다.

## 최종 설치형 검증 — 2026-10-06 10:16–10:19 KST

최종 수정 ARM64 빌드(3m53s)·서명 deep/strict PASS, SHA256
277c38d26ca7ed217aa65e38728a8e5b3358bee58a783048eb60df0ebfbb9dcf.
사용자가 잠금을 해제한 뒤 기존 앱을 CUA ⌘Q로 정상 종료하고 최종 번들을 /Applications에
설치했다. 설치본의 위 SHA256 일치·ARM64·deep/strict 서명 PASS. 데이터 폴더는 교체하지 않았다.

- 최종 앱 시작 시 기존 Claude Code/sonnet 선택 복원 PASS.
- Codex로 전환하면 CLI 출처의 실제 공개 모델4개 표시:
  GPT-6-Astra, GPT-5.6-Sol, GPT-5.6-Terra, GPT-5.6-Luna.
- GPT-5.6-Luna 선택 후 자체 consent-fixed 세션에서 파일 조회·검사·삭제를 하지 말라는
  단문 질문2회. 실제 답변 “모델 연결 확인 완료”, “17 곱하기 19는 323입니다.” 완료.
  조사상한·검토필요·저장실패 경고 없이 입력창/선택기가 다시 활성화되었다.
- Settings에 같은 Codex/GPT-5.6-Luna 표시 PASS.
- ⌘Q 후 main process 부재를 확인하고 다시 실행. Codex/GPT-5.6-Luna와 자체 대화12개 복원 PASS.
- Remember·확인 생략·시스템/앱 조회·정리 검토 ON, 자동 시작·메뉴 사용량 ON, Docker OFF 유지.
- 보존 테스트 파일 consent-cancel.txt/keep-this.txt 각각29B 유지. 이번 검증에서 파일 작업 없음.

응답 헤더의 모델 이름은 요청값 echo이지 서버가 실제 해결한 모델 버전의 독립 증명이 아니다.
별도 `--model` 인자 전달은 소스와 이미 통과한 argv 단위 검사로 확인했다. 짧은 실제 실행의
argv 관찰은 선행 `codex login status`만 포착하여 exec 인자는 실측 증거로 사용하지 않는다.

로컬 ignored 증거: docs/ui-audit/screenshots/2026-10-06/model-native-catalog.png,
model-native-response.png, model-native-settings.png, model-native-restarted.png.
작업 전 원본은 /private/tmp/broomsweepy-model-selection-backup-DtiHZi/BroomSweepy.app,
초기 모델 빌드는 같은 폴더의 BroomSweepy-before-catalog-fix.app에 보존했다.

## 남은 검증

Windows 런타임, Grok/Antigravity(미설치), Claude 실제 응답(사용자 이전 범위대로 생략),
Ollama 실제 생성(미설치), 계정별 전체 모델, 장시간 자원/FPS/OS스크린리더는 NOT RUN.
이번 검증은 실제 파일 삭제/휴지통 비우기/로그인/권한/CLI 전역 설정 변경 없음.

#tags: 모델선택, cli카탈로그, ui검증, macos, arch:012
