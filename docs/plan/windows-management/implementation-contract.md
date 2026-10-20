# Windows 전용 관리 — 구현 계약

2026-10-20 · 사용자: Windows에 맞는 메뉴/속성을 구현한 뒤 Mac 회귀 테스트.

## 기능 경계

기존 React/Rust composition에서 Windows에만 `Windows 설정`을 노출한다. Mac은 기존 `맥 설정`과 Finder5개/복원 흐름을 그대로 유지한다. 공통 성능·앱·파일·정리 엔진과 파일 권한을 OS 변경 권한으로 확대하지 않는다.

Windows에서는 고정10개 레지스트리 **저장값 조회**와 고정8개 정식 설정 진입만 제공한다. 실제 Windows 기능의 작동/동의/설치/지원 상태를 저장DWORD로 추정하지 않는다. 직접 쓰기·삭제·관리자 상승·PowerShell·서비스 제어·OS 재시작·Defender 변경은 없다. 설정 화면으로 요청을 전달했다는 결과는 실제 화면 열림이나 설정 변경 완료가 아니다.

## 고정 읽기 목록

64bit view, KEY_READ. 값은4byte REG_DWORD만 읽고 미설정/접근거부/형식불일치/미지원값/기타오류를 구분한다. OS build/edition은 별도 고정 metadata key의 최대256byte REG_SZ, UBR은DWORD다. 모든 목록/문자열은 상한이 있다.

| ID | 범위 | 고정 위치 / 값 |
|---|---|---|
| explorer_hide_extensions | HKCU | Software\Microsoft\Windows\CurrentVersion\Explorer\Advanced / HideFileExt |
| explorer_hidden_files | HKCU | 같은 Advanced / Hidden |
| explorer_protected_files | HKCU | 같은 Advanced / ShowSuperHidden |
| explorer_full_path | HKCU | Software\Microsoft\Windows\CurrentVersion\Explorer\CabinetState / FullPath |
| legacy_copilot | HKCU 정책 | Software\Policies\Microsoft\Windows\WindowsCopilot / TurnOffWindowsCopilot |
| recall_snapshots_user | HKCU 정책 | Software\Policies\Microsoft\Windows\WindowsAI / DisableAIDataAnalysis |
| recall_snapshots | HKLM 정책 | 같은 WindowsAI / DisableAIDataAnalysis |
| recall_availability | HKLM 정책 | 같은 WindowsAI / AllowRecallEnablement |
| notepad_ai | HKLM 정책 | Software\Policies\WindowsNotepad / DisableAIFeatures |
| paint_image_creator | HKLM 정책 | Software\Microsoft\Windows\CurrentVersion\Policies\Paint / DisableImageCreator |

Hidden0/1/2를 화면의 켜짐/꺼짐으로 합성하지 않는다. legacy Copilot은 폐기이며 새 앱의 상태로 해석하지 않는다. 정책5종/6행은 OS build·에디션·앱·하드웨어·관리 정책 지원을 미확인으로 표시한다. Recall의 사용자/기기 정책을 우선순위로 합성하지 않는다. 탐색기 역시 저장값과 실제 표시를 구별한다.

## API

- `get_windows_settings` → `supported/platform/osBuild/osRevision/edition/settings/targets/warnings`.
- Row: `id/title/description/category/scope/state/value/valueType/support/targetId`.
- `open_windows_settings_target {targetId}` → `targetId/requested/changed:false`. 현재 로컬 main origin만, Windows만. 임의 URI/실행파일/인수는 받지 않는다.
- Target8: folder_options, storage, startup, apps, taskbar, privacy, about, optional_features. URI는 고정 `ms-settings:` 목록이며 folder_options는 GetSystemDirectoryW로 얻은 control.exe + `/name Microsoft.FolderOptions`다.
- `windows.inspect`는 같은 읽기 엔진의 최대10행 projection. 외부는 기존 시스템·앱 조회 공개 권한이 필요하다. 요청은 무인자이고 target/쓰기/설정 열기를 받지 않는다. 경고 상세/로컬 실행 경로는 모델로 보내지 않는다.
- NonWindows: unsupported, 빈행/빈targets, Windows reader/launcher0. UI 진입 오류/AI의 다른OS조회도 변경을 실행하지 않는다.

## UI / 디자인

`DESIGN.md`와 승인된 compact-dark-glass Data Instrument 방향을 그대로 확장한다. 새 브랜드/자산/효과/의존성은 없다. 기존 Mac 행과 명시 상태·새로고침 패턴을 재사용하므로 새 시각 방향3후보는 필요하지 않다.

순서: 짧은 범위/OS정보 → 정식 설정 진입 → 탐색기4행 → 기본 접힌 AI 정책6행 → 공통 기능 연결. 저장값0과미설정/오류를 구별하며 원시값은DWORD로 표기한다. 단일 읽기/열기 잠금, unmount 뒤 응답 무시,14px/44px·keyboard·390px·모션감소 유지. 대상 열기 실패는 오류+재시도이며 성공을 과장하지 않는다.

## 검증 계획

1. Pure model/state/allowlist/unsupported/deny-by-default tests를 Mac에서 실행.
2. Windows native adapter를 GNU target으로 타입 검사하되 전체Windows Tauri실행/설치와 구분.
3. Native+MCP 동등 결과/외부 공개권한/무변경/16KiB 상한 검사.
4. 실제 App 컴포넌트로 Mac/Windows UI 분리·값·분류·좁은 화면 검사.
5. Mac 실제 Finder5개 읽기 smoke와 Windows unsupported native 경로를 실행. 사용자 OS 설정 쓰기/자료 삭제/실제 AI 전송은 하지 않는다.

Windows 실기/installer/정식 설정 페이지의 실제 표시와 Mac 설치본 교체는 이 Mac에서 검증했다고 주장하지 않는다. 결과는 별도 검증 보고서에 기록한다.

## 공식 근거 (2026-10-20 조회)

- [Microsoft Explorer 구현](https://github.com/microsoft/winget-dsc/blob/main/resources/Microsoft.Windows.Developer/Microsoft.Windows.Developer.psm1), [FolderOptions OpenSpecs](https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-gppref/a6ca3a17-1971-4b22-bf3b-e1a5d5c50fca).
- [WindowsAI 정책](https://learn.microsoft.com/en-us/windows/client-management/mdm/policy-csp-windowsai), [Notepad 정책](https://learn.microsoft.com/en-us/windows/client-management/manage-notepad).
- [Windows 설정 URI](https://learn.microsoft.com/en-us/windows/apps/develop/launch/launch-settings), [Control Panel canonical names](https://learn.microsoft.com/en-us/windows/win32/shell/controlpanel-canonical-names), [실행 계약](https://learn.microsoft.com/en-us/windows/win32/shell/executing-control-panel-items).

#tags: windows관리, 플랫폼분리, 저장값조회, 고정설정연결, 맥회귀
