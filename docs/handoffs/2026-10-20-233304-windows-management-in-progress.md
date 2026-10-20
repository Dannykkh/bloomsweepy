# Windows 관리 구현과 Mac 회귀

2026-10-20 23:33 KST · BroomSweepy · main @85620ca

## Origin

현재 사용자: “맞아. 그렇게 구현하고 난다음에 맥을 테스트 해보자.” Windows에 맞는 메뉴·속성을 구현한 다음 Mac 회귀를 실행한다. 컨텍스트 압축 시점에 생성하고 검증 후 갱신한 연속성 기록이다. 원래 사용자 턴 시각·세션 ID는 제공되지 않았다.

## Current State Summary

Windows 전용 조회·공식 연결과 공통 AI/MCP 읽기 연동을 구현했다. 프론트170, Rust workspace490, 실제 Finder읽기1, 실제 Wry 부정IPC15가 통과했다. Windows 실기/installer와 새Mac설치본 교체는 실행하지 않았다. 소스 개발본의 변경이며 공개 릴리스에 자동 반영되지 않는다.

## Feature/Flow/Decision Snapshot

- Windows backend/UI 작업자 모두 소유권 반환. main이 통합·실렌더·문서·최종 판정을 담당한다.
- 기존 dirty Mac 설정/cleanup/UI 작업을 보존했다. `.codex/loop-state.md`, 개인 일별 대화, demo-assets는 열거나 변경하지 않는다. 설치·커밋·푸시는 요청되지 않았다.
- 신규 `windows_settings.rs`, `windows_settings/{model,native}.rs`: 고정10개 레지스트리 저장값 읽기,8개 공식 설정 진입, 로컬 main origin 검사. Windows 이외에는 reader/launcher0, unsupported. 쓰기·관리자 상승·임의 URI/프로세스 입력 없음.
- 신규 WindowsManagementView, `apps/desktop/src/lib/windowsSettings.ts`, unit tests 및 i18n3파일. 공식 설정8→Explorer4→접힌 AI 정책6→공통 관리 연결. 저장값과 실효 상태를 분리한다.
- App/AppShell/types/실제 App fixture: Windows에만 Windows 설정, Mac에만 Mac 설정. 기존6분류 설정·폴더 정리 상태 보존.
- control capability `windows.inspect` 및 app_tools projection 연결. 동일 native reader→최대10행 DTO→채팅 카드. 외부 기존 inspection 권한 적용; 설정 실행 capability는 없다.
- 정본 범위: [구현 계약](../plan/windows-management/implementation-contract.md). README/DESIGN/capability 정본/QA/memory018 갱신 완료.

### Implemented Features

| Feature | 현재 범위 |
|---|---|
| Windows 저장값 조회 | 고정10행·64bit KEY_READ·오류/부재/원시DWORD·범위 구분 |
| 공식 설정 연결 | 고정8 ID, 로컬 main만, requested≠changed |
| 공통 AI/MCP | 같은 reader·bounded10행/16KiB, 외부 inspection 권한 전후 검증, 쓰기/열기 도구 없음 |
| OS UI | Mac/Windows 메뉴 분리, 공식연결 우선·정책기본접힘·좁은 화면 |

### Composition Diagram

```text
Windows View ─ get_windows_settings ─┐
AI/MCP ─ windows.inspect ─ 권한검사 ─┴─ 동일 native reader ─ 10행 DTO
                                    └─ 모델 projection(16KiB) ─ 카드/화면
local main ─ fixed target ID ─ OS/origin gate ─ 공식 설정 요청(changed:false)
Mac ─ 기존 Finder5개/내구성 저널 (Windows reader/launcher 호출0)
```

### Feature Boundary / Decision Records

저장값을 실효 ON/OFF로 합성하지 않고 HKCU/HKLM을 독립 보존한다. file full_access는 OS권한이 아니다. upstream 관리자 스크립트/registry쓰기/패키지·보호 설정 제거는 제외한다. [018](../../memory/architecture/018-windows-management.md)에 대안·재검토 조건을 남겼으며017 메뉴 계약의 확장이지 반전이 아니다.

## Validation

- control tests31/31 PASS (Windows readonly contract,27 IDs,16KiB budget 포함).
- 전체 Rust workspace release490PASS/0failed/6ignored, exit0. desktop323PASS/6ignored 안에 Windows9개·projection검사가 포함된다. 실제 Finder5개 읽기 smoke는 별도1/1 PASS다.
- 실제 native/model source의 x86_64-pc-windows-gnu offline cargo check PASS, 최종0.67초/경고0. 별도 임시 manifest `/private/tmp/broomsweepy-windows-native-check-qmOWhw/Cargo.toml`;21개 lock entries의 버전/checksum은 저장소와 같음. 실제 Windows 실행·전체 Tauri 설치 검사와 다르다.
- 최종 프론트170/170·type/build PASS. 첫조회실패의 잘못된 stale안내를 실제 렌더에서 발견해 수정/재검증했다.
- 실제 Wry 부정IPC15/15, 거부12건 각각 직후 Finder5개와분리journal/sidecar 불변12/12, exit0. [stdout](../qa/windows-management/native-ipc-result.jsonl).
- 실제 App 합성 UI Mac/Windows1280×820/390×844·값/오류/늦은응답/정리동선/설정분류·폰트·overflow·44px PASS. 형식/정적리뷰도PASS. [전체 QA](../qa/windows-management/verify-report.md).

## Important Context

모든 Rust 검사는 jobs1/offline/release를 재사용해8GiB 맥의 비용을 제한했다. 기존 Vite1431은 합성 QA 전용이고 실제 native·AI로 넘기지 않는다. Wry는 별도 nonce app/분리 journal을 쓰는 검사이지 full product setup이 아니다. 실제 사용자 OS 설정 변경·자료 삭제·AI 전송은 하지 않았다. 기존 chunk>500KB/dead_code 경고와 forced-reduced-motion 실행 미확인은 QA에 남겼다.

## Immediate Next Steps

1. Windows 실기에서 일반 사용자·관리형/오류·빌드/SKU별 고정 조회와 공식8진입을 확인하고 전체 Tauri/installer를 검증한다. 교차 타입 검사로 대체하지 않는다.
2. Mac 새 설치본/실제 LLM 종단/장기 RAM 검증을 원하면 별도 범위로 진행한다. 현재 설치본 교체·커밋·푸시·릴리스는 요청되지 않았다.
3. 재개 시 QA 정본과018을 먼저 읽고 기존 dirty Mac 작업/개인 자료를 보존한다. 새 OS 쓰기 요청은 별도 지원·부작용·충돌·복원 계약이 필요하다.

## Session Memory Review

MEMORY.md→architecture/index→017 및 연결된016/010 본문을 확인했다. 마지막 doctor 차트 방문2026-09-29로30일 미만이고 본문이 존재하므로 doctor 재실행 조건 없음. 새018·index와 전용 대화 기록을 추가하고 MEMORY.md의 기존 OS행을 갱신했다(4990byte). build_anchor_index.py에서 Windows backend/UI 두 경로로018 CURRENT를 재검색해 확인했다. 세션 원본 ID가 없으므로 gotcha/learned 과거 백로그 정제는 하지 않았다. 프로젝트 전용 스킬 대상이 없어 개선 candidate는 보류하고 전역 스킬을 수정하지 않았다.

#tags: windows관리, 플랫폼분리, 맥회귀, 검증기록, 핸드오프, arch:018
