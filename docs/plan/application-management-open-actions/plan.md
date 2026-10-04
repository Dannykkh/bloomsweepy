# 앱 목록·앱 정리·파일/폴더 열기

## 요청과 현재 상태

**현재:** 구현·독립 검토·Mac 자동/브라우저/안전한 설치형 확인 완료. 새 arm64 앱 교체 및 기존 앱 복구 보관 완료. Windows는 OS별 구현과 mock 검증 완료, 네이티브 실행 별도. 아래 초기 조사 항목의 승인 대기는 해소된 과거 상태다.

- 사용자: 기존 소스 설치 진행 중 앱 목록/삭제, 공간 정리의 선택 파일·폴더 열기를 추가 요청.
- 기존 설치 승인 작업: 최신 저자원/폴더 Trash/Empty Trash를 arm64로 빌드하여 현재 Mac 설치본 교체. 실제 OS 비우기는 하지 않음.
- 신규 앱 삭제 범위: 2026-09-07 async 선택 질문 전달. **응답 대기이며 승인받았다고 간주하지 않음.**
- WorkPM native-first, 2개 읽기 전용 연구 작업자. 구현/설치/테스트 실행은 아직 신규 기능에 착수하지 않음. MCP NOT RUN.
- 후속 사용자 질문: 정식 제거 루틴 연결과 관련 데이터 선택 정리 중 어떤 방식이 안전하고 깔끔한지 비교 요청. **선택 승인으로 해석하지 않음.** 공식 안내에 따라 정식 제거 우선, macOS 전용 제거기 없을 때 앱 본체 Trash, 잔여 데이터는 별도 선택 정리하는 조합을 권고. 관련 데이터 정리는 추가 구현 범위이며 아직 승인/구현되지 않음.
- 기존 승인 수정분은 explicit arm64 target 빌드·ad-hoc 번들 생성 성공(session 39585 exit 0). 공증·설치 교체·설치본 검증은 수행하지 않음.

## Phase 1 — 조사·제안

| 대안 | 적합성 | 안전성 | 적은 구현 부담 | 판단 |
|---|---:|---:|---:|---|
| Mac 앱 본체 Trash + Windows OS 제거 화면 | 5 | 4 | 4 | 추천. 문서·설정 보존, 앱 내 정리 가능 |
| 목록과 OS 제거/관리 화면 연결만 | 3 | 5 | 5 | 위험은 작지만 Mac 앱 내 제거 요청을 덜 충족 |
| 앱 본체와 관련 데이터 선택 정리 | 5 | 2 | 1 | 공유 데이터/서비스/오탐 범위 추가 설계 필요 |

모든 대안은 파일·폴더 열기 동작 구분 및 제한된 Rust inspection 경계를 포함한다.

### 조사 근거

- `system_inventory.rs`: 알려진 macOS 앱 폴더 직계 `.app` 목록 이미 존재. 현재 전체 드라이브 검사 뒤에만 호출. 독립 호출로 분리 가능. 10,000 entries/4 MiB plist/100 issues 상한; 용량 미측정.
- `folder_actions.rs`와 core `actions.rs`: nonce/TTL/신원 재검증을 참고할 수 있으나 일반 폴더의 Applications/app 차단을 해제하면 안 됨. 앱 전용 no-follow 경계가 필요.
- `system_performance.rs`의 GUI 앱 조회는 Background 앱을 제외하므로 제거 보호에 그대로 사용하면 안 됨.
- `FileTable`은 숨은 더블클릭/Enter만, `CleanupView`는 reveal만, treemap은 drilldown/reveal만 제공한다. 명시적 버튼 필요.
- 기존 `bridge.inspectFile`의 opener open-path 경로 scope가 비어 있어 네이티브 실행이 거부될 수 있음. 전역 `**` 허용 대신 파일시스템 종류를 재검사하는 Rust command 권장.

### Blind spots / 자기검증

- 가장 어려운 결정: 앱 본체 제거와 설정·공유 데이터·전용 제거 프로그램의 의미 분리. 사용자에게 범위를 질문했다.
- 거부할 대안: 레지스트리 UninstallString 직접 실행, 모든 경로 opener 허용, 시스템 앱 보호 해제, 앱 내부 링크 추적, 실제 앱 삭제를 테스트로 실행.
- 가장 불확실한 부분: 전용 제거기가 필요한 앱과 백그라운드 서비스, OS 경로 기반 Trash의 마지막 TOCTOU. `완전 제거`를 보장하지 않으며 실행 중/보호된 앱은 중단해야 함.
- 저자원: 앱별 전체 크기/아이콘 순회 금지, inventory 하나만 보관, 검색+50~100개 페이지, 전체 문자열 예산 필요.
- [프로젝트 용어 사전](../../domain-dictionary.md): 컨텍스트 기반 확정. 글로벌 시드 없음, 프로젝트 특수 용어 글로벌 승격 없음.

## Phase 2–5

- [x] 사용자 삭제 범위 선택 수신: “진행하자. 윈도우는 윈도우에 맞춰서 진행”. 정식 제거 우선 + 관련 데이터 별도 선택 정리 승인.
- [x] 선택안의 성공/차단/취소/오류 흐름도 확정: `docs/flow-diagrams/application-management.mmd`, `storage-open.mmd`.
- [x] 고유 파일 소유권·API 계약 확정 (아래).
- [x] 구현 + 독립 검토 + mock/합성 파일 테스트 (Rust269, TS/43개 테스트, 추가 release helper2개 통과)
- [x] 설치형 무해한 파일/폴더 열기, 앱 목록/확인창 검증 (사용자 앱 삭제 없음)
- [x] 최종 arm64 앱 교체, 복구용 이전 앱 보관

## Phase 5 — 완료 검토

- [QA 실행·검토·플랫폼 제한 기록](../../qa/2026-09-07-application-management-open-actions.md).
- API/도면/도메인 사전 대조 완료. 독립 backend/UI/open 검토에서 발견한 문제 수정 후 재검토 완료. 외부 CLI 리뷰/MCP는 NOT RUN.
- Mac의 승인된 안전 검증은 완료. Windows 실기기·실제 제거/휴지통 비우기·장시간 저메모리 soak는 별도 검증이며 완료로 주장하지 않는다. 커밋/릴리즈/푸시 없음.

## 승인된 구현 경계와 소유권

- Mac: 알려진 앱 루트의 목록을 독립 조회. 시스템/현재 앱/실행 중 앱/변경된 앱 차단. 정식 제거기 우선 안내·위치 표시, 전용 제거기 없는 일반 앱만 별도 확인 후 본체 Trash. 자동으로 제거기를 추측 실행하지 않음.
- 관련 데이터: Mac의 정확한 bundle ID 기반 전용 캐시·환경설정만 별도 후보 검토. 기본 선택 없음, 경로·종류·근거 노출, bounded metadata traversal, 링크/클라우드/공유/사용자 문서 제외. Application Support·Containers는 작업 데이터 가능성 때문에 자동 후보에서 제외. Windows는 레지스트리 문자열/이름 유사성으로 AppData를 추측 삭제하지 않으며 정식 제거 절차에 맡긴다.
- Windows: 제한된 설치 정보 목록 + 고정 `ms-settings:appsfeatures` 연결. 사용자 선택/제거 완료는 OS 소유. 화면 열림을 삭제 완료로 기록하지 않음. macOS 본체 Trash 명령은 cfg 경계에서 거부.
- 앱 관리 UI는 성능 다음의 독립 메뉴로 배치(2026-09-07 추가 사용자 요청), 검색/50개 페이지. 전체 드라이브 검사나 앱별 eager 용량/아이콘 계산 없음.
- Worker `application_backend`: `application_actions.rs`, `system_inventory.rs`, `trash_actions.rs`, `action_recovery.rs`, 필요 시 core `actions.rs`만. backend DTO 계약 아래. `lib.rs` 등록은 Lead 소유.
- Worker `storage_open_impl`: 새 `file_inspection.rs`, 파일 열기 전용 bridge 모듈, `FileTable.tsx`, `CleanupView.tsx`, `StorageTreemapPanel.tsx`, `TreemapItemMenu.tsx`, 새 공유 open 버튼/CSS 및 테스트. 기존 bridge의 inspectFile만 이 모듈로 re-export하는 통합은 Lead. lib.rs/core/types/i18n/App.css 수정 금지.
- Worker `application_ui`: 새 `ApplicationsView.tsx`, `lib/applicationBridge.ts`, `lib/applicationTypes.ts`, 전용 CSS, mock fixture. 다른 기존 파일 수정 금지. 한국어 msg를 사용하고 번역 키 목록 보고.
- Lead: 도면·로그·용어·lib.rs 등록·App.tsx 통합·types/bridge·i18n·검증/설치. Cargo와 테스트는 한 번에 하나.

### 앱 DTO/IPC 계약

- `get_application_inventory()` → `{platform: 'macos'|'windows'|'unsupported', inventoryId, applications: [{id, displayName, displayVersion: string|null, publisher: string|null, installLocation: string|null, estimatedBytes: number|null, removalMode:'trashBundle'|'systemSettings'|'protected', protectionReason:string|null}], issues:string[]}`. inventoryId/app id opaque, bounded inventory 하나.
- `prepare_application_trash({request:{inventoryId,applicationId}})` → `{planId, displayName, path, expiresAtUnixMs, relatedData: [{id,path,kind:'cache'|'preferences',evidence,estimatedBytes:number|null}], warnings:string[]}`. 별도 이름/경로 backend 검증, 이 계획은 본체+옵션별 명시 선택 리뷰.
- `confirm_application_trash({request:{planId, bundleOnlyAcknowledged:true, noUninstallerAcknowledged:true}})` → 기존 `TrashOperationResult`. 기본 본체만, 관련 데이터 자동삭제 없음. 짧은 nonce TTL·한번 사용·공유 작업 잠금·journal.
- `prepare_application_data_trash({request:{inventoryId,applicationId,candidateIds:string[]}})` → 동일 plan shape. 본체가 제거된 뒤에도 해당 inventory identity를 통한 한정 후보 검토 허용. confirm은 `confirm_application_data_trash({request:{planId,relatedDataAcknowledged:true}})` → `TrashOperationResult`.
- `dismiss_application_plan({planId})` → void.
- `open_application_uninstall_settings()` → void, Windows만 고정 OS 설정 URI. `revealPath(installLocation)`으로 Mac 위치 표시.
- 앱 본체/잔여 데이터 계획 구분 및 이중 실행·만료·선택 변경·refresh 무효화 필요. 불일치/변경은 실행 거부 후 재검토.

## 플랫폼 근거

- [Apple 앱 제거 가이드](https://support.apple.com/en-us/102610): 앱별 전용 제거기 존재 가능, 앱 삭제와 사용자 문서 삭제 구분.
- [Windows Settings URI](https://learn.microsoft.com/en-us/windows/apps/develop/launch/launch-settings): 고정 OS 관리 화면 연결.
