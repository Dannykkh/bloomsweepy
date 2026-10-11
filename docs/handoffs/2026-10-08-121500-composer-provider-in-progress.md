# Composer provider selector — development verified

Time: 2026-10-08 KST, 최종 상태 갱신12:24:46 이후. 원본 turn timestamp/session UUID 미제공.
Project: BroomSweepy. Branch: main. Trigger: 작업 중 컨텍스트 압축, 원래 요청 계속 수행.
Origin: 첨부 입력창의 첫 모델 버튼을 Codex·Claude Code·AGY·Grok 선택으로 변경하고 해당 모델/강도를 연결.

## Origin

사용자가 첨부 입력창의 앞 버튼을 CLI로 바꾸고 모델/강도 값을 해당 CLI에 맞추도록 요청했다.
근거: conversations/2026-10-08-composer-provider.md#현재-요청. 원본 session UUID 미제공.

## Current State Summary

구현·합성 UI 검증·타입/119frontend/build/contract/독립 리뷰 완료. 설치본은 교체 전.
별도 file-change-review 작업의 dirty changes 모두 보존. 주기 temp 자동삭제는 미구현/미승인.

## Feature/Flow/Decision Snapshot

하단 CLI trigger + 모델·강도 통합 trigger. 기존 header select는 상태/재확인만 유지.
CLI 입력→useAssistantModelPreference provider→provider별 모델/모델별 effort 복원→host catalog 표시
→기존 askAssistant JSON. Settings와 hook 공유, native/MCP/권한 계약 변경 없음.
대안: 세 번째 버튼 추가 — 사용자 지적 중복과 좁은 폭을 악화; 별도 저장 — 동기화/ID 혼용 위험.
기존 architecture012 정본과 compact composer 경험 계약을 확장하며 뒤집지 않는다.

## Composition Diagram

```text
Composer CLI → shared preference → provider model/effort → existing askAssistant JSON
                         ↑
                  Settings picker
```

## Implemented Features

| Feature/Change | Boundary |
|---|---|
| CLI/모델·강도2 trigger | 실제 상태 목록과 기존 통합 popup 재사용 |
| 헤더 중복 select 제거 | 상태·재확인 유지 |
| async 최신 선호 guard | revision/unmount 검사, 기존 busy/IME/권한/취소 보존 |
| 합성 harness cleanup | 실제 provider 이름·root disposal, 신규 번역키3locale |

## Files Modified

- `docs/design-refs/2026-10-08-experience-composer-provider.md` 및 layout/direction/critique/implementation.
- 관련 코드: `apps/desktop/src/components/AssistantComposerControls.tsx`, `apps/desktop/src/components/AssistantComposerControls.css`, `apps/desktop/src/views/AssistantView.tsx`, `apps/desktop/src/hooks/useAssistantModelPreference.ts`.
- readonly agent composer_provider_contract: 기존 hook에서 전체 연결 가능, 늦은 probe 최신 선호 guard 주의.
- 실제 코드/README/정본/architecture012·인덱스·대화·QA·screenshots 갱신. 상세 QA 파일 참조.

## Verification

PASS: contract validator·119frontend·타입/build·diff check·독립 리뷰. 합성5CLI 전환·Settings/재로드·초안·
키보드·busy/취소·미설치·Grok model/effort request·1280/390px. docs/qa/2026-10-08-composer-provider.md 참조.
실제 CLI/개인 파일/AI 요청·새 설치는 이 요청 범위가 아님. Native API 무변경이면 전체 Rust 재빌드 불필요.

## Important Context

남은 Data 공간약1.3GiB. 이전 file-review dirty changes와 demo-assets를 보존했고 삭제/커밋/푸시 없음.
설치된 앱/공개v1.7.0는 개발 소스와 다르다. 합성 QA는 실제 CLI 접근권·Windows·장시간 메모리 증거 아님.

## Immediate Next Steps

현재 입력창 소스 요청은 완료. 설치본 교체가 요청되면 안전한 용량 확인 후 bundle/서명/실행 검증.
주기 temp 후보 검토/자동 휴지통 정책은 별도 사용자의 방향 답변이 필요하며 구현하지 않았다.

## Session Memory Review

프로젝트 실제 .git 및 docs/memory/conversations 디렉터리 확인. MEMORY→architecture012 실제 본문 확인.
doctor chart 2026-09-29,30일 이내·본문 존재하여 진단 생략. 기존 architecture012와 중복 계약이므로
검증 완료 후 해당 항목에 delta 근거를 추가하고 index의CLI entry에서 다시 찾았다. 신규 gotcha/learned 정제는 session UUID/시작 시각
미제공으로 보류; 기존 backlog/offset 변경0. Local skill 개선 candidate 없음(해당 프로젝트 전용 대상 없음).

#tags: 핸드오프, composer-provider, cli선택, arch:012
