# 선택적 변경 파일 검토 — 현재 요청과 결정

Recorded: 2026-10-08 11:57 Asia/Seoul
Source: codex
사용자 원본 턴 시각·세션 UUID는 현재 컨텍스트에 제공되지 않았다. 위 시각은 기록 시각이며 수신 시각이 아니다.
이 파일은 해당 요청/결정의 필요한 발췌이며 원시 세션 전체를 대체하지 않는다.

## 현재 요청

> 탐지는 못하더라도, 감지는 한다. 이런거니, 한번 파일 검토해보고 넘길건 넘기는 형태의 루틴을 넣을수 있으면 좋겠다. 옵션으로

직전 논의는 새 파일/변경 목록·주기 확인·LLM 분석 가능성이었고, 이번 실행 범위는 선택적 변경 검토 옵션이다.
예약 삭제·백신 탐지·랜섬웨어 차단은 이번 범위로 확대하지 않았다.

## 구현 결정

기본 OFF, 명시적 로컬 폴더 하나, 5/15/30분 metadata 비교. 첫 완전 검사는 baseline.
실행형/이중 확장자/50개 이상 변경은 검토 필요, 그 외 규칙 불일치는 넘김. 넘김은 안전 판정이 아니다.
개별 변경을 직접 넘기거나 다시 검토하고 같은 경로 재변경은 새 검토 기록으로 교체한다.
불완전 검사는 이전 기준을 유지한다. 내용 읽기·자동 AI 호출/전송·삭제/격리/실행은 하지 않는다.
AI/MCP는 같은 앱 결과의 제한된 읽기만 사용하며 옵션 ON을 공개 동의로 재사용하지 않는다.

## 검증과 남은 일

Rust/MCP422, frontend119 통과, typecheck/build/fmt/diff/contract 통과. 실제 임시 metadata 변화,
반복 worker 취소와 합성 UI 넘김·재변경·페이지·1280/390px 관찰.
설치 앱은 미교체, Windows·실제 신규 AI 호출·장시간 soak는 NOT RUN.
정본: `docs/architecture/file-change-review.md`, `docs/qa/2026-10-08-file-change-review.md`.

#tags: file-change-review, opt-in, metadata, skip-not-safe, bounded-runtime, arch:014
