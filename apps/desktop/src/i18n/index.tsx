import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useState,
  type ReactNode,
} from "react";
import { setFormattingLanguage } from "../lib/format";
import { setApplicationLanguage } from "../lib/bridge";
import {
  LANGUAGE_STORAGE_KEY,
  normalizeLanguagePreference,
  type LanguagePreference,
  type ResolvedLanguage,
} from "./preference";
import japaneseCatalog from "./ja.json";
import simplifiedChineseCatalog from "./zh-CN.json";

export type { LanguagePreference, ResolvedLanguage } from "./preference";

const englishMessages = {
  "본문으로 건너뛰기": "Skip to main content",
  "내비게이션 닫기": "Close navigation",
  "내비게이션 열기": "Open navigation",
  "BroomSweepy 내비게이션": "BroomSweepy navigation",
  "주요 화면": "Primary views",
  "저장공간 도구": "Storage instrument",
  "대시보드": "Dashboard",
  "디스크와 최근 변화": "Drives and recent changes",
  "용량 관리": "Storage",
  "지도·큰 파일·중복": "Map, large files, duplicates",
  "Docker 용량": "Docker storage",
  "이미지·캐시·컨테이너": "Images, cache, containers",
  "파일 이름 찾기": "Find files",
  "이름과 위치로 찾기": "Search by name and location",
  "문서 내용 찾기": "Search documents",
  "문장으로 찾기": "Search by text",
  "대화": "Chat",
  "설치된 AI CLI": "Installed AI CLI",
  "설정": "Settings",
  "스캔 기준과 안전": "Scan rules and safety",
  "오늘의 저장공간": "Storage today",
  "드라이브 상태와 최근 변화를 한 번에 확인합니다.":
    "See drive health and recent changes at a glance.",
  "폴더 용량 지도": "Folder storage map",
  "큰 사각형부터 따라가며 용량이 늘어난 위치를 찾습니다.":
    "Follow the largest rectangles to find where storage is being used.",
  "개발 도구": "Developer tools",
  "Docker가 보고한 이미지·컨테이너·볼륨·빌드 캐시 사용량을 확인합니다.":
    "Review Docker-reported usage for images, containers, volumes, and build cache.",
  "파일 찾기": "File search",
  "파일을 열지 않고 이름이나 폴더 위치로 찾습니다.":
    "Find files by name or folder without opening them.",
  "큰 파일": "Large files",
  "크기와 위치를 나란히 비교합니다.": "Compare size and location side by side.",
  "문서 찾기": "Document search",
  "선택한 폴더의 문서를 미리 읽어 내용으로 빠르게 찾습니다.":
    "Index documents in the selected folder for fast content search.",
  "정리 후보": "Cleanup candidates",
  "오래된 임시 파일과 삭제 후 남은 흔적을 근거별로 검토합니다.":
    "Review old temporary files and leftovers with supporting evidence.",
  "중복 파일": "Duplicate files",
  "파일 내용을 끝까지 비교해 실제로 같은 결과만 표시합니다.":
    "Compare complete file contents and show only confirmed matches.",
  "선택한 대상과 대화": "Chat with a selected target",
  "폴더 또는 Docker를 고르면 앱이 확인하고 로컬 AI CLI가 결과를 설명합니다.":
    "Choose a folder or Docker; the app inspects it and a local AI CLI explains the result.",
  "앱 설정": "App settings",
  "스캔 설정": "Scan settings",
  "분석 범위와 결과 한도를 조정합니다.": "Adjust scan scope and result limits.",
  "찾을 위치": "Search location",
  "문서를 읽을 폴더": "Document folder",
  "검사할 폴더": "Folder to scan",
  "기본 디스크 상태": "Primary disk status",
  "{{size}} 여유": "{{size}} free",
  "디스크 확인 중": "Checking disk",
  "마지막 검사 {{date}}": "Last scan {{date}}",
  "폴더 선택": "Choose folder",
  "표시 언어": "Display language",
  "이 앱의 메뉴와 설명에 사용할 언어입니다.":
    "Choose the language used for this app's menus and descriptions.",
  "언어": "Language",
  "이 앱의 표시만 바뀌며 파일과 운영체제 설정은 바뀌지 않습니다.":
    "Only this app's display changes. Files and operating-system settings are not changed.",
  "시스템 설정": "System setting",
  "한국어": "Korean",
  "English": "English",
  "日本語": "Japanese",
  "简体中文": "Simplified Chinese",
  "현재 적용: {{language}}": "Currently applied: {{language}}",
  "언어 선택은 이 컴퓨터에만 저장됩니다.":
    "Your language choice is stored only on this computer.",
  "언어 선택을 저장하지 못했습니다. 현재 실행 중에는 선택한 언어를 사용합니다.":
    "The language choice could not be saved. The selected language will remain active for this session.",
  "로그인 시 자동 시작": "Start at login",
  "운영체제에 등록된 실제 시작 상태를 표시합니다.":
    "Shows the startup state currently registered with the operating system.",
  "BroomSweepy 자동 시작": "BroomSweepy autostart",
  "기본값은 사용 안 함입니다. 켜면 로그인할 때 창을 띄우지 않고 백그라운드에서 시작합니다.":
    "Off by default. When enabled, BroomSweepy starts in the background without opening a window when you sign in.",
  "Windows 시작 앱 또는 macOS 로그인 항목 설정에서도 끌 수 있습니다.":
    "You can also turn it off in Windows Startup apps or macOS Login Items.",
  "자동 시작 사용": "Use autostart",
  "시작 설정 적용 중…": "Applying startup setting…",
  "시작 설정 확인 중…": "Checking startup setting…",
  "확인 불가": "Unavailable",
  "켜짐": "On",
  "꺼짐": "Off",
  "자동 시작 설정을 처리하지 못했습니다. {{detail}}":
    "Could not update the startup setting. {{detail}}",
  "요청한 자동 시작 상태가 운영체제에 반영되지 않았습니다.":
    "The operating system did not apply the requested autostart state.",
  "시스템 메모리 상태": "System memory status",
  "운영체제가 보고하는 현재 메모리 수치를 읽습니다.":
    "Reads the current memory figures reported by the operating system.",
  "메모리 상태 새로 고침": "Refresh memory status",
  "메모리 상태 확인 중…": "Checking memory status…",
  "메모리 사용률": "Memory usage",
  "전체 메모리": "Total memory",
  "사용 가능": "Available",
  "스왑 사용량": "Swap usage",
  "스왑/커밋 추정": "Swap/commit estimate",
  "Windows에서는 커밋 사용량에서 전체 물리 메모리를 뺀 추정치이며 페이지 파일의 실제 사용량이 아닙니다.":
    "On Windows, this is estimated as committed memory minus total physical memory; it is not actual pagefile usage.",
  "{{platform}} · {{date}} 확인": "{{platform}} · checked {{date}}",
  "이 화면은 상태만 읽습니다. 다른 앱의 메모리, 누수 메모리 또는 대기(standby) 메모리를 정리하지 않습니다.":
    "This view only reads status. It does not clean memory used by other apps, leaked memory, or standby memory.",
  "메모리 상태를 확인하지 못했습니다. {{detail}}":
    "Could not check memory status. {{detail}}",
  "파일 크기 기준": "File size thresholds",
  "다음 스캔부터 적용됩니다.": "Applied to the next scan.",
  "큰 파일 최소 크기": "Minimum large-file size",
  "결과 목록에 보여줄 최소 파일 크기": "Smallest file size shown in results",
  "중복 검사 최소 크기": "Minimum duplicate-file size",
  "작은 파일을 비교하는 작업량을 줄입니다": "Reduces work spent comparing small files",
  "결과 한도": "Result limits",
  "메모리 사용량과 화면 밀도를 제어합니다.": "Controls memory use and on-screen density.",
  "큰 파일 결과": "Large-file results",
  "크기순으로 보관할 최대 항목 수": "Maximum items retained by size",
  "중복 그룹 결과": "Duplicate-group results",
  "낭비 용량순으로 보관할 최대 그룹 수": "Maximum groups retained by wasted space",
  "{{count}}개": "{{count}} items",
  "현재 안전 계약": "Current safety contract",
  "스캔은 파일을 수정하거나 이동하지 않습니다.": "Scanning does not modify or move files.",
  "같은 저장공간을 가리키는 여러 파일 이름은 중복 낭비로 세지 않습니다.":
    "Multiple names pointing to the same stored data are not counted as duplicate waste.",
  "일부 내용으로 후보를 줄인 뒤 전체 내용을 끝까지 비교해 중복을 확정합니다.":
    "Candidates are narrowed with samples, then confirmed by comparing all content.",
  "선택 항목은 실행 직전 재검증하고 운영체제 휴지통으로만 이동합니다.":
    "Selected items are rechecked immediately before being moved to the operating-system Trash or Recycle Bin.",
  "일반 파일은 영구 삭제하지 않으며 Windows 설치 정보도 변경하지 않습니다.":
    "Regular files are never permanently deleted, and Windows installation records are not changed.",
  "Docker 정리는 예외적으로 휴지통을 거치지 않아 별도 확인 뒤에만 실행합니다.":
    "Docker cleanup is irreversible and runs only after a separate confirmation.",
  "파일 검사가 완료됐습니다.": "File scan completed.",
  "파일 검사를 취소했습니다.": "File scan cancelled.",
  "파일 검사를 완료하지 못했습니다. {{detail}}": "File scan failed. {{detail}}",
  "오류 내용을 확인해 주세요.": "Review the error details.",
  "채팅에서 요청한 검사를 준비하고 있습니다": "Preparing the scan requested from chat",
  "채팅에서 요청한 검사를 진행하고 있습니다": "Running the scan requested from chat",
  "채팅에서 요청한 검사를 완료하지 못했습니다": "The scan requested from chat did not complete",
  "드라이브: {{detail}}": "Drives: {{detail}}",
  "최근 정리: {{detail}}": "Recent cleanup: {{detail}}",
  "최근 파일: {{detail}}": "Recent files: {{detail}}",
  "확인할 정리 계획이 없거나 확인 시간이 지났습니다.":
    "There is no cleanup plan to review, or its review window has expired.",
  "파일 위치를 열지 못했습니다. {{detail}}": "Could not reveal the file location. {{detail}}",
  "스캔 작업을 준비하고 있습니다": "Preparing the scan",
  "드라이브 분석을 준비하고 있습니다": "Preparing drive analysis",
  "안전하게 스캔을 중단하고 있습니다": "Stopping the scan safely",
  "안전하게 드라이브 분석을 중단하고 있습니다": "Stopping drive analysis safely",
  "폴더 구조 분석을 준비하고 있습니다": "Preparing folder analysis",
  "안전하게 폴더 지도 분석을 중단하고 있습니다": "Stopping folder-map analysis safely",
  "정리 후보 위치를 준비하고 있습니다": "Preparing cleanup locations",
  "안전하게 정리 후보 분석을 중단하고 있습니다": "Stopping cleanup analysis safely",
  "문서 검색 목록을 준비하고 있습니다…": "Preparing the document index…",
  "현재 문서 확인을 마친 뒤 안전하게 멈추고 있습니다…":
    "Finishing the current document and stopping safely…",
  "파일 검색 목록을 준비하고 있습니다…": "Preparing the file index…",
  "최근 파일을 읽지 못했습니다. {{detail}}": "Could not read recent files. {{detail}}",
  "현재 파일 확인을 마친 뒤 목록 만들기를 안전하게 멈추고 있습니다…":
    "Finishing the current file and stopping index creation safely…",
  "다른 스캔 또는 정리 작업이 진행 중입니다": "Another scan or cleanup task is already running",
  "서버에 보관된 스캔 결과와 선택 항목을 대조하고 있습니다":
    "Comparing selected items with the scan report stored by the app",
  "현재 항목을 마친 뒤 안전하게 작업을 중단하고 있습니다":
    "Finishing the current item and stopping safely",
  "외부 AI가 제안한 중복 파일 정리": "Duplicate cleanup suggested by external AI",
  "외부 AI가 제안한 정리 후보": "Cleanup suggested by external AI",
  "외부 AI는 익명 후보 번호와 용량 요약만 보고 이 계획을 만들었습니다. 아래 정확한 경로는 이 앱 안에서만 표시되며, 지금 확인해야 파일 이동을 시작합니다.":
    "The external AI created this plan from anonymous candidate IDs and size summaries only. Exact paths are shown only inside this app, and files move only after your confirmation.",
  "확인하고 휴지통으로 이동": "Confirm and move to Trash",
  "외부 AI 정리 계획 확인 대기": "External AI cleanup plan awaiting review",
  "{{count}}개 · 앱에서 정확한 경로를 확인해야 실행됩니다.":
    "{{count}} items · Review exact paths in the app before anything runs.",
  "불러오는 중…": "Loading…",
  "검토하기": "Review",
  "휴지통 이동 중": "Moving to Trash",
  "저장공간 맵 분석 중": "Analyzing storage map",
  "정리 후보 분석 중": "Analyzing cleanup candidates",
  "문서 검색 준비 중": "Preparing document search",
  "파일 목록 만드는 중": "Building file index",
  "드라이브 분석 중": "Analyzing drive",
  "채팅에서 요청한 파일 검사 중": "Running file scan requested from chat",
  "스캔 진행 중": "Scan in progress",
  "선택 항목을 안전하게 다시 확인하고 있습니다": "Safely rechecking selected items",
  "폴더 구조를 확인하고 있습니다": "Inspecting folder structure",
  "남은 파일과 제거 정보를 대조하고 있습니다": "Comparing leftover files with uninstall records",
  "문서 내용을 검색할 수 있게 정리하고 있습니다…": "Preparing document content for search…",
  "파일 이름과 경로를 수집하고 있습니다": "Collecting file names and paths",
  "저장공간을 분류하고 있습니다": "Classifying storage usage",
  "파일을 확인하고 있습니다": "Inspecting files",
  "취소": "Cancel",
  "알 수 없는 오류가 발생했습니다": "An unknown error occurred",
  "스캔할 폴더 선택": "Choose a folder to scan",
  "용량 지도": "Storage map",
  "용량 관리 화면": "Storage views",
  "파일을 확인하는 중": "Inspecting files",
  "{{count}}개 파일 확인": "{{count}} files inspected",
  "분석 준비 완료": "Ready to analyze",
  "디스크 사용률 {{percent}}퍼센트, {{size}} 남음":
    "Disk usage {{percent}} percent, {{size}} remaining",
  "디스크를 선택하지 않음": "No disk selected",
  "{{percent}}% 사용": "{{percent}}% used",
  "{{name}} 파일을 기본 앱으로 열었습니다.": "Opened {{name}} in the default app.",
  "{{name}}은 직접 열도록 허용한 문서·미디어 형식이 아니라 폴더에서 위치만 표시했습니다.":
    "{{name}} is not an approved document or media type, so only its folder location was revealed.",
  "{{name}} 파일을 열지 못했습니다: {{detail}}": "Could not open {{name}}: {{detail}}",
  "파일 분석 결과": "File analysis results",
  "선택": "Select",
  "파일": "File",
  "수정": "Modified",
  "크기": "Size",
  "{{name}}, 더블클릭하거나 Enter 키를 눌러 확인":
    "{{name}}, double-click or press Enter to inspect",
  "더블클릭하여 기본 앱으로 열기": "Double-click to open in the default app",
  "이 그룹에는 보관할 파일을 하나 이상 남겨야 합니다":
    "At least one file must remain in this group",
  "휴지통으로 이동할 파일 선택": "Select file to move to Trash",
  "{{name}} 휴지통 이동 대상으로 선택": "Select {{name}} to move to Trash",
  "더블클릭하면 기본 앱으로 엽니다. 실행 파일과 스크립트는 안전을 위해 폴더에서만 표시합니다.":
    "Double-click to open with the default app. Executables and scripts are revealed in their folder for safety.",
  "작업 결과": "Operation result",
  "휴지통 이동을 중단했습니다": "Stopped moving items to Trash",
  "일부 항목만 처리했습니다": "Only some items were processed",
  "휴지통 이동을 완료했습니다": "Finished moving items to Trash",
  "실제로 확보된 공간이 아니라 휴지통으로 옮긴 파일 크기의 합계입니다.":
    "This is the total size moved to Trash, not space already reclaimed.",
  "이동": "Moved",
  "이동한 용량": "Size moved",
  "실패": "Failed",
  "건너뜀": "Skipped",
  "확인이 필요한 항목": "Items needing review",
  "작업 기록": "Operation journal",
  "마지막 기록 동기화를 완료하지 못했습니다.": "The final journal sync did not complete.",
  "파일 상태가 바뀌었으므로 기존 분석 결과는 폐기했습니다. 계속하려면 다시 스캔하세요.":
    "File state changed, so the previous analysis was discarded. Scan again to continue.",
  "다시 스캔": "Scan again",
  "휴지통으로 이동": "Move to Trash",
  "되돌릴 수 있는 작업": "Recoverable operation",
  "확인 창 닫기": "Close confirmation dialog",
  "대상": "Items",
  "선택한 파일 크기": "Selected file size",
  "복구 위치": "Recovery location",
  "운영체제 휴지통": "Operating-system Trash or Recycle Bin",
  "휴지통으로 이동할 항목": "Items to move to Trash",
  "이동 직전에 파일 신원과 변경 여부를 다시 검사합니다. 휴지통에서 복원할 수 있지만, 실제 여유 공간은 휴지통을 비운 뒤에 늘어납니다.":
    "File identity and changes are checked again immediately before moving. Items can be restored from Trash, but free space increases only after Trash is emptied.",
  "한 번 더 확인할 프로그램 설정 {{count}}개에는 계정이나 설정 데이터가 포함될 수 있음을 확인했습니다.":
    "I understand that {{count}} application-setting items require extra review and may include account or configuration data.",
  "안전 검사를 준비하고 있습니다": "Preparing safety checks",
  "작업 중단 요청": "Request stop",
  "운영체제 용량, 완료된 휴지통 기록, 마지막 파일 목록을 함께 봅니다.":
    "View operating-system storage, completed Trash operations, and the latest file index together.",
  "확인 중": "Checking",
  "새로 고침": "Refresh",
  "다시 시도": "Try again",
  "드라이브 용량": "Drive storage",
  "운영체제가 보고한 현재 값": "Current values reported by the operating system",
  "드라이브를 확인하고 있습니다": "Checking drives",
  "드라이브 정보를 읽지 못했습니다": "Drive information is unavailable",
  "새로 고침한 뒤에도 보이지 않으면 운영체제 권한을 확인해 주세요.":
    "If drives still do not appear after a refresh, check operating-system permissions.",
  "다시 확인": "Check again",
  "최근 정리": "Recent cleanup",
  "운영체제 휴지통으로 이동한 논리 용량": "Logical size moved to the operating-system Trash",
  "아직 정리 기록이 없습니다": "No cleanup history yet",
  "파일을 선택하고 최종 확인한 뒤 휴지통으로 옮긴 결과만 여기에 남습니다.":
    "Only completed moves to Trash after final confirmation appear here.",
  "용량 관리 열기": "Open storage",
  "최근 기록 일부를 읽지 못했습니다. 중단된 작업은 위쪽 복구 안내에서 확인하세요.":
    "Some recent records could not be read. Check the recovery notice above for interrupted operations.",
  "최근 추가된 파일": "Recently added files",
  "BroomSweepy가 이전 목록 이후 새로 발견": "Newly discovered since the previous BroomSweepy index",
  "파일을 휴지통으로 옮긴 뒤 목록이 오래됐습니다. 새로 고쳐야 최근 파일이 정확합니다.":
    "The file index is stale after moving items to Trash. Refresh it for accurate recent files.",
  "먼저 파일 목록을 만들어 주세요": "Build a file index first",
  "첫 목록은 비교 기준으로만 저장하며 기존 파일을 모두 새 파일로 표시하지 않습니다.":
    "The first index becomes a baseline and does not mark every existing file as new.",
  "파일 목록 만들기": "Build file index",
  "비교 기준 목록이 준비됐습니다": "Baseline index is ready",
  "마지막 목록 {{date}}. 다음 갱신부터 새로 발견한 파일을 표시합니다.":
    "Last index {{date}}. Newly discovered files will appear after the next refresh.",
  "목록 새로 고침": "Refresh index",
  "선택하면 파일 위치를 엽니다": "Select to reveal the file location",
  "{{parent}} · {{date}} 발견": "{{parent}} · discovered {{date}}",
  "새 파일 {{count}}개": "{{count}} new files",
  "최근 항목만 표시": "showing recent items only",
  "이전 목록 이후 새 파일이 없습니다": "No new files since the previous index",
  "마지막 비교 {{date}}": "Last comparison {{date}}",
  "파일 시스템 정보 없음": "File-system information unavailable",
  "시스템": "System",
  "이동식": "Removable",
  "사용 중": "Used",
  "남음": "Free",
  "사용": "Used",
  "용량 보기": "View storage",
  "중복 파일 정리": "Duplicate-file cleanup",
  "정리 후보 이동": "Cleanup-candidate move",
  "파일 정리": "File cleanup",
  "사용자 취소": "Cancelled by user",
  "일부만 완료": "Partially completed",
  "완료": "Completed",
  "요청 {{requested}}개 중 {{moved}}개 이동 · {{status}}":
    "Moved {{moved}} of {{requested}} requested · {{status}}",
  "큰 파일과 중복 파일 검사": "Large-file and duplicate scan",
  "큰 파일과 중복을 확인하고 있습니다": "Checking large files and duplicates",
  "더 정리할 항목 찾기": "Find more items to clean up",
  "{{count}}개 · {{size}} 확인": "{{count}} items · {{size}} inspected",
  "큰 파일을 모으고, 중복 후보만 내용을 비교합니다.":
    "Collects large files and compares content only for duplicate candidates.",
  "검사 취소": "Cancel scan",
  "큰 파일·중복 찾기": "Find large files and duplicates",
  "검사를 취소했습니다.": "Scan cancelled.",
  "검사 결과": "Scan results",
  "확인할 항목을 고르세요": "Choose what to review",
  "{{count}}개 · {{size}}": "{{count}} items · {{size}}",
  "{{count}}그룹 · {{size}}": "{{count}} groups · {{size}}",
  "임시 파일과 삭제 후 남은 흔적": "Temporary files and post-uninstall leftovers",
  "컴퓨터 전체 용량을 종류별로 보기": "View whole-computer storage by category",
  "설치된 앱, 임시 파일, 문서, 사진처럼 시스템 드라이브를 나눠 봅니다.":
    "Break down the system drive into installed apps, temporary files, documents, photos, and more.",
  "저장공간 트리맵": "Storage treemap",
  "사각형이 클수록 더 많은 용량을 사용합니다. 폴더를 누르면 안쪽으로 이동합니다.":
    "Larger rectangles use more space. Select a folder to move inside it.",
  "분석 취소": "Cancel analysis",
  "처음 폴더 다시 보기": "Return to starting folder",
  "지도 다시 만들기": "Rebuild map",
  "읽기 전용": "Read-only",
  "저장공간 맵 경로": "Storage-map path",
  "폴더 구조를 분석하고 있습니다": "Analyzing folder structure",
  "{{count}}개 항목 · {{size}} 확인": "{{count}} items · {{size}} inspected",
  "현재 폴더 요약": "Current-folder summary",
  "현재 범위": "Current scope",
  "직계 항목": "Direct items",
  "하위 파일": "Files below",
  "빈 폴더": "Empty folders",
  "파일과 폴더 크기 비교 지도": "File and folder size comparison map",
  "하위 폴더 탐색": "Explore subfolder",
  "현재 폴더 용량 순위": "Current-folder storage ranking",
  "용량 순위": "Storage ranking",
  "폴더를 선택하면 하위로 이동": "Select a folder to move inside",
  "{{files}}개 파일 · {{folders}}개 폴더": "{{files}} files · {{folders}} folders",
  "표시할 용량 항목이 없습니다": "No storage items to display",
  "현재 폴더에는 용량이 있는 파일이 없거나 접근할 수 없습니다.":
    "This folder has no non-empty files, or its contents are inaccessible.",
  "직접 포함된 항목이 하나도 없는 폴더만 표시합니다.":
    "Shows only folders containing no direct items.",
  "접근 가능한 범위에서 빈 폴더가 발견되지 않았습니다.":
    "No empty folders were found in the accessible scope.",
  "화면에는 8개만 표시합니다. 전체 {{total}}개 중 최대 {{kept}}개 경로를 보관했습니다.":
    "Only 8 are shown. Up to {{kept}} paths were retained out of {{total}} total.",
  "화면에는 8개만 표시합니다. 전체 {{total}}개 중 나머지 경로도 스캔 결과에 보관돼 있습니다.":
    "Only 8 are shown. The remaining paths out of {{total}} total are retained in the scan result.",
  "{{duration}} · 읽기 전용 분석": "{{duration}} · read-only analysis",
  "접근 제한 {{count}}개": "{{count}} access-restricted items",
  "작은 직계 항목 {{count}}개 집계": "{{count}} smaller direct items aggregated",
  "개별 항목 보관 안전 상한 도달": "Per-item retention safety limit reached",
  "큰 사각형부터 폴더 안쪽으로 이동합니다": "Follow the largest rectangles into folders",
  "먼저 검사할 폴더를 선택하세요": "Choose a folder to scan first",
  "지도를 다시 만들 수 있습니다.": "You can rebuild the map.",
  "폴더를 고르면 용량 지도를 바로 만듭니다.": "Choosing a folder builds its storage map immediately.",
  "위 안내의 버튼으로 폴더 용량 지도를 만드세요.": "Use the button above to build a folder storage map.",
  "폴더를 고른 뒤 용량 지도를 만들 수 있습니다.": "Choose a folder, then build its storage map.",
  "기타 {{count}}개": "{{count}} other items",
  "설치된 앱": "Installed apps",
  "앱 본체와 로컬 애플리케이션 데이터": "App bundles and local application data",
  "시스템 사용 및 예약": "System use and reserved",
  "운영체제와 보호된 시스템 영역": "Operating-system and protected system areas",
  "임시 파일": "Temporary files",
  "캐시, 로그, 빌드 및 작업 중간 파일": "Caches, logs, builds, and working files",
  "휴지통": "Trash",
  "복원할 수 있도록 보관 중인 파일": "Files retained for recovery",
  "데스크톱": "Desktop",
  "현재 사용자의 바탕 화면": "Current user's desktop",
  "문서": "Documents",
  "문서 폴더와 일반 문서 형식": "Document folders and common document formats",
  "다운로드": "Downloads",
  "브라우저와 앱에서 내려받은 파일": "Files downloaded by browsers and apps",
  "사진": "Photos",
  "사진 폴더와 이미지 형식": "Photo folders and image formats",
  "동영상": "Videos",
  "비디오 폴더와 영상 형식": "Video folders and video formats",
  "음악 및 오디오": "Music and audio",
  "음악 폴더와 오디오 형식": "Music folders and audio formats",
  "압축 및 디스크 이미지": "Archives and disk images",
  "압축 파일과 ISO 이미지": "Archive files and ISO images",
  "개발 파일": "Development files",
  "소스, 의존성, 빌드 및 테스트 산출물": "Source, dependencies, builds, and test artifacts",
  "다른 사용자": "Other users",
  "현재 계정 외 사용자 프로필": "User profiles other than the current account",
  "기타": "Other",
  "아직 명확한 범주로 분류되지 않은 파일": "Files not yet assigned to a clear category",
  "현재 OS": "Current OS",
  "드라이브 사용량": "Drive usage",
  "{{mount}}의 실제 파일을 읽기 전용으로 분류합니다.":
    "Classifies actual files on {{mount}} without modifying them.",
  "분석할 수 있는 드라이브를 확인하고 있습니다.": "Checking for a drive that can be analyzed.",
  "다시 분석": "Analyze again",
  "드라이브 분석": "Analyze drive",
  "저장공간 범주를 준비하고 있습니다": "Preparing storage categories",
  "{{count}}개 파일 · {{size}} 확인": "{{count}} files · {{size}} inspected",
  "저장공간 범주": "Storage categories",
  "{{platform}} 설치 기록과 앱 {{count}}개 대조":
    "Compared {{count}} apps with {{platform}} installation records",
  "{{count}}개 파일": "{{count}} files",
  "분석 전": "Not analyzed",
  "{{duration}} · 읽을 수 있는 파일 크기 합계 {{size}}":
    "{{duration}} · {{size}} total readable file size",
  "드라이브 분류 단계는 파일을 변경하지 않습니다": "Drive classification does not change files",
  "{{count}}개 접근 제한": "{{count}} access-restricted",
  "하드링크 {{count}}개 제외": "{{count}} hard links excluded",
  "파일에 표시된 크기 기준": "Based on displayed file sizes",
  "위치 목록 안전 상한 도달": "Location-list safety limit reached",
  "하드링크 집계 상한 도달": "Hard-link aggregation limit reached",
  "용량이 큰 위치": "Largest locations",
  "상위 {{count}}개": "Top {{count}}",
  "{{path}}의 저장공간 맵 열기": "Open storage map for {{path}}",
  "큰 파일과 중복 파일을 한 번에 검사하세요": "Scan large files and duplicates together",
  "한 번의 자세한 검사로 큰 파일 목록과 실제 중복 결과를 함께 만듭니다.":
    "One detailed scan produces both a large-file list and confirmed duplicate results.",
  "현재 결과": "Current results",
  "파일 이름 또는 경로 검색": "Search file name or path",
  "파일 이름 또는 경로 검색…": "Search file name or path…",
  "큰 파일부터 보기": "Largest files first",
  "크기순 파일 목록": "Files sorted by size",
  "삭제 기능 없이 분석만 수행합니다": "Analysis only; this view does not delete files",
  "검색 조건과 일치하는 큰 파일이 없습니다.": "No large files match the search.",
  "기존 결과는 안전을 위해 폐기했습니다. 다시 스캔하세요.":
    "The previous result was discarded for safety. Scan again.",
  "한 번의 자세한 검사로 큰 파일을 찾고, 중복 후보는 내용을 처음부터 끝까지 비교합니다.":
    "One detailed scan finds large files and compares duplicate candidates from beginning to end.",
  "전체 내용 검증 완료": "Full-content verification complete",
  "{{groups}}개 그룹에서 {{size}}를 중복으로 확인했습니다.":
    "Confirmed {{size}} of duplicate data across {{groups}} groups.",
  "하드링크 식별자 안전 상한에 도달했습니다. 이후 하드링크 파일은 중복 분석에서 제외했습니다.":
    "The hard-link identity safety limit was reached. Later hard-linked files were excluded from duplicate analysis.",
  "중복 결과 필터": "Duplicate-result filters",
  "중복 파일 종류": "Duplicate-file type",
  "모든 파일 {{count}}": "All files {{count}}",
  "동일 사진 {{count}}": "Identical photos {{count}}",
  "사진 보기는 내용이 완전히 같은 이미지 파일만 포함합니다.":
    "The photo view includes only image files with identical content.",
  "휴지통 이동 선택 요약": "Trash selection summary",
  "각 그룹의 보관본 한 개는 선택할 수 없습니다.": "One retained copy in each group cannot be selected.",
  "서로 다른 폴더에 흩어진 중복이 {{count}}개 그룹 있습니다. 각 위치를 비교한 뒤 이동 대상을 선택하세요.":
    "{{count}} duplicate groups are spread across different folders. Compare locations before selecting items to move.",
  "내용이 같은 사진이 없습니다": "No identical photos",
  "검증된 중복 파일이 없습니다": "No verified duplicate files",
  "비슷하게 찍힌 사진은 아직 포함하지 않고, 파일 내용이 완전히 같은 사진만 보여줍니다.":
    "Similar-looking photos are not included; only photos with identical file content are shown.",
  "크기만 같고 내용이 다른 파일은 중복으로 표시하지 않았습니다.":
    "Files with the same size but different content were not marked as duplicates.",
  "중복 파일 그룹": "Duplicate-file groups",
  "중복 그룹": "Duplicate group",
  "내용 확인 번호 {{hash}} · 전체 내용 비교 완료":
    "Verification ID {{hash}} · full-content comparison complete",
  "서로 다른 폴더 {{count}}곳": "{{count}} different folders",
  "{{count}}개 선택": "{{count}} selected",
  "그룹에 표시할 파일이 없습니다.": "No files to display in this group.",
  "선택한 중복 파일을 휴지통으로 이동할까요?": "Move the selected duplicate files to Trash?",
  "휴지통 이동을 완료하지 못했습니다": "Could not complete the move to Trash",
  "오래된 임시 파일": "Old temporary files",
  "최근 사용 흔적이 없는 사용자 임시 항목": "User temporary items with no recent activity",
  "프로그램 설정 폴더": "Application data folders",
  "설치 앱과 이름이 맞지 않는 오래된 데이터": "Old data not matching installed applications",
  "오래된 캐시": "Old caches",
  "운영체제가 지정한 캐시 위치": "Operating-system cache locations",
  "{{name}} 위치를 파일 탐색기에서 표시했습니다.": "Revealed {{name}} in the file manager.",
  "{{name}} 위치를 표시하지 못했습니다: {{detail}}": "Could not reveal {{name}}: {{detail}}",
  "정리 후보 스캔을 완료하지 못했습니다": "Cleanup-candidate scan failed",
  "정리 후보 분석이 완료됐습니다": "Cleanup-candidate analysis completed",
  "삭제 후 남은 흔적을 근거별로 찾습니다": "Find post-uninstall leftovers using supporting evidence",
  "{{duration}} 동안 {{count}}개 항목을 확인했습니다.": "Inspected {{count}} items in {{duration}}.",
  "Windows 임시 폴더, 프로그램 설정 폴더, 설치 기록을 바꾸지 않고 서로 비교합니다.":
    "Compares Windows temporary folders, application data folders, and installation records without changing them.",
  "임시 파일과 오래된 캐시를 읽기 전용으로 확인합니다.":
    "Inspects temporary files and old caches without changing them.",
  "스캔 취소": "Cancel scan",
  "정리 후보 스캔": "Scan cleanup candidates",
  "확인한 항목": "Items inspected",
  "확인한 용량": "Size inspected",
  "발견 후보": "Candidates found",
  "위치 진행": "Locations processed",
  "정리 후보 요약": "Cleanup-candidate summary",
  "정리 가능성 높음": "Likely safe to clean",
  "오래된 임시 파일·캐시": "Old temporary files and caches",
  "검토 필요": "Review required",
  "프로그램 설정 폴더·설치 기록": "Application data folders and installation records",
  "후보 용량": "Candidate size",
  "Windows 설치 기록 제외": "Excludes Windows installation records",
  "선택한 파일 후보만 운영체제 휴지통으로 이동합니다":
    "Only selected file candidates are moved to the operating-system Trash",
  "옮기기 직전에 파일이 바뀌지 않았는지 다시 확인하고 작업 기록을 남깁니다. 프로그램 설정 폴더는 한 번 더 확인해야 하며 Windows 설치 정보는 바꾸지 않습니다.":
    "Files are checked again for changes immediately before moving, and an operation journal is kept. Application data folders require extra confirmation, and Windows installation records are not changed.",
  "정리 후보 필터": "Cleanup-candidate filters",
  "정리 후보 종류": "Cleanup-candidate type",
  "전체": "All",
  "프로그램 설정": "Application data",
  "캐시": "Cache",
  "{{count}}개 표시": "{{count}} shown",
  "한 번 더 확인할 프로그램 설정 {{count}}개 포함":
    "Includes {{count}} application-data items requiring extra review",
  "이동 직전 모든 항목을 다시 검사합니다.": "All items are checked again immediately before moving.",
  "파일 정리 후보": "File cleanup candidates",
  "선택한 종류의 정리 후보가 없습니다.": "No cleanup candidates match the selected type.",
  "더블클릭하여 파일 탐색기에서 위치 표시": "Double-click to reveal in the file manager",
  "휴지통으로 이동할 후보 선택": "Select candidate to move to Trash",
  "{{count}}개 항목": "{{count}} items",
  "삭제 후 남은 흔적": "Post-uninstall leftovers",
  "깨진 제거 프로그램 정보": "Broken uninstaller records",
  "{{count}}개 검토 대상": "{{count}} to review",
  "서로 다른 경로 증거가 두 개 이상 끊긴 제거 정보가 없습니다.":
    "No uninstaller record has at least two independent pieces of broken path evidence.",
  "컴퓨터": "Computer",
  "사용자": "User",
  "스캔 안전 한도에 도달해 일부 항목은 생략했습니다. 현재 결과를 삭제 판단의 전체 목록으로 사용하면 안 됩니다.":
    "Some items were omitted after reaching the scan safety limit. Do not treat this result as a complete deletion list.",
  "아직 정리 후보를 확인하지 않았습니다": "Cleanup candidates have not been scanned yet",
  "최근 사용 시각, 설치 앱 인벤토리, 경로 존재 여부를 함께 대조해 단순 파일명보다 보수적으로 분류합니다.":
    "Classifies conservatively by comparing recent activity, installed-app inventory, and path existence instead of relying on names alone.",
  "선택한 정리 후보를 휴지통으로 이동할까요?": "Move the selected cleanup candidates to Trash?",
  "텍스트·코드": "Text and code",
  "워드·엑셀·한글": "Word, Excel, and HWPX",
  "PDF": "PDF",
  "HWPX": "HWPX",
  "텍스트": "Text",
  "표": "Spreadsheet",
  "발표": "Presentation",
  "{{name}} 문서를 기본 앱으로 열었습니다.": "Opened {{name}} in the default app.",
  "{{name}}은 직접 열도록 허용한 문서 형식이 아니라 폴더에서 위치만 표시했습니다.":
    "{{name}} is not an approved document type, so only its folder location was revealed.",
  "{{name}} 문서를 열지 못했습니다: {{detail}}": "Could not open {{name}}: {{detail}}",
  "문서 내용에서 찾기": "Search document content",
  "문서 안의 단어와 문장을 찾습니다": "Find words and phrases inside documents",
  "파일은 이 기기 안에서만 읽고 검색하기 좋게 정리합니다. 문서 내용은 외부로 보내지 않습니다.":
    "Files are read and indexed only on this device. Document content is not sent outside.",
  "이 기기 안에서만 처리": "Processed only on this device",
  "문서 내용 검색어": "Document-content query",
  "선택한 폴더의 문서 목록을 먼저 새로고침하세요…": "Refresh the selected folder's document index first…",
  "예: 계약 변경, 오류 코드, 회의 결정…": "For example: contract change, error code, meeting decision…",
  "먼저 검색할 폴더의 문서를 읽어 두세요…": "Index documents in a folder before searching…",
  "검색 중…": "Searching…",
  "내용 검색": "Search content",
  "문서 형식 필터": "Document-format filter",
  "검색 범위가 선택되지 않았습니다": "No search scope selected",
  "선택한 폴더가 지금 읽어 둔 문서 목록과 다릅니다":
    "The selected folder differs from the current document index",
  "이전 폴더의 결과와 섞이지 않도록 새 폴더의 문서를 다시 읽어 주세요.":
    "Re-index the new folder to avoid mixing it with results from the previous folder.",
  "새 폴더 다시 읽기": "Re-index new folder",
  "문서 검색 준비 상태": "Document-search readiness",
  "문서를 읽고 검색 목록을 새로 만들고 있습니다…": "Reading documents and rebuilding the search index…",
  "{{count}}개 문서를 검색할 수 있습니다": "{{count}} documents are searchable",
  "아직 읽어 둔 문서가 없습니다": "No documents have been indexed yet",
  "문서 파일을 확인하고 있습니다…": "Inspecting document files…",
  "{{date}} 새로고침 · 읽은 문서 {{size}} · {{duration}}":
    "Refreshed {{date}} · {{size}} indexed · {{duration}}",
  "처음에는 문서 내용을 읽고, 다음부터는 바뀐 문서만 다시 읽습니다.":
    "The first run reads document content; later runs re-read only changed documents.",
  "읽기 취소": "Cancel reading",
  "문서 목록 새로고침": "Refresh document index",
  "문서 미리 읽기": "Index documents",
  "확인한 파일": "Files inspected",
  "읽을 문서": "Candidate documents",
  "검색 준비됨": "Search-ready",
  "다시 안 읽음": "Reused",
  "문서를 검색할 수 있게 준비하지 못했습니다": "Could not prepare documents for search",
  "최근 문서 읽기 결과": "Latest document-index result",
  "새로 읽음": "Newly read",
  "변경 없음": "Unchanged",
  "구형 HWP": "Legacy HWP",
  "PDF 안에서 마우스로 선택할 수 있는 글자만 찾습니다. 사진처럼 저장된 PDF와 비밀번호가 걸린 문서는 읽지 않습니다.":
    "Only selectable text in PDFs is indexed. Image-only and password-protected PDFs are not read.",
  "한 번에 읽을 수 있는 문서 수를 넘었습니다": "The document limit for one run was exceeded",
  "일부 문서가 빠졌을 수 있습니다. 더 작은 폴더를 선택해 다시 읽어 주세요.":
    "Some documents may be missing. Choose a smaller folder and index it again.",
  "읽지 못한 문서와 건너뜀 사유 {{count}}개": "{{count}} unread documents and skip reasons",
  "경로 정보 없음": "Path unavailable",
  "화면에는 처음 20개 사유만 표시합니다.": "Only the first 20 reasons are shown.",
  "문서를 검색하지 못했습니다": "Document search failed",
  "이 기기에서 찾은 결과": "Results found on this device",
  "“{{query}}” 검색 결과": "Results for “{{query}}”",
  "{{documents}}개 문서에서 {{matches}}개를 찾았습니다.":
    "Found {{matches}} matches in {{documents}} documents.",
  "상위 100개만 표시합니다.": "Only the top 100 results are shown.",
  "일치하는 문서가 없습니다": "No matching documents",
  "단어를 줄이거나 다른 문서 형식을 선택해 다시 검색해 보세요.":
    "Try fewer words or choose another document format.",
  "{{name}}, 더블클릭하거나 Enter 키를 눌러 열기": "{{name}}, double-click or press Enter to open",
  "{{name}} 폴더에서 표시": "Reveal {{name}} in folder",
  "위치 표시": "Reveal location",
  "첫 버전 검색 범위": "Initial search coverage",
  "TXT·Markdown·코드·CSV·JSON, 워드·엑셀·파워포인트·HWPX, 글자를 선택할 수 있는 PDF를 지원합니다. 구형 HWP와 사진으로 된 PDF는 아직 내용 검색을 지원하지 않습니다.":
    "Supports TXT, Markdown, code, CSV, JSON, Word, Excel, PowerPoint, HWPX, and PDFs with selectable text. Legacy HWP and image-only PDFs are not yet supported for content search.",
  "폴더": "Folder",
  "모든 크기": "Any size",
  "100 MB 이상": "100 MB or larger",
  "1 GB 이상": "1 GB or larger",
  "10 GB 이상": "10 GB or larger",
  "검색어와 가까운 순": "Best match",
  "이름 가나다순": "Name A–Z",
  "용량 큰 순": "Largest first",
  "최근에 바뀐 순": "Recently modified",
  "{{name}} 폴더의 위치를 표시했습니다.": "Revealed the location of {{name}}.",
  "{{name}}을 열지 못했습니다: {{detail}}": "Could not open {{name}}: {{detail}}",
  "내 파일에서 찾기": "Search my files",
  "파일 이름이나 폴더 위치로 찾으세요": "Search by file name or folder location",
  "찾고 싶은 이름을 입력하세요. 파일을 열지 않고 저장된 이름과 위치만 확인합니다.":
    "Enter a name to find. Search checks stored names and locations without opening files.",
  "이 기기 안에서만 검색": "Search only on this device",
  "파일명 또는 경로 검색어": "File-name or path query",
  "선택한 위치의 파일 목록을 먼저 새로고침하세요…": "Refresh the selected location's file index first…",
  "옮긴 파일을 반영하도록 파일 목록을 새로고침하세요…": "Refresh the file index to reflect moved files…",
  "예: 8월 보고서…": "For example: August report…",
  "먼저 찾을 위치의 파일 목록을 만드세요…": "Build a file index for the search location first…",
  "즉시 검색": "Instant search",
  "검색을 더 정확하게 하는 법": "How to refine your search",
  "둘 중 하나": "Either term",
  "이름 모양": "Name pattern",
  "파일 종류": "File type",
  "파일만": "Files only",
  "폴더 위치": "Folder path",
  "100 MB보다 큼": "Larger than 100 MB",
  "이 날짜 뒤": "After date",
  "이 날짜 앞": "Before date",
  "이 단어 빼기": "Exclude term",
  "찾을 대상 선택": "Choose search target",
  "파일 종류 필터": "File-type filter",
  "파일 크기": "File size",
  "최소 파일 크기": "Minimum file size",
  "정렬 기준": "Sort by",
  "검색 결과 정렬": "Sort search results",
  "선택한 위치가 지금 만든 파일 목록과 다릅니다":
    "The selected location differs from the current file index",
  "이전 위치의 결과와 섞이지 않도록 새 위치의 목록을 다시 만드세요.":
    "Rebuild the index for the new location to avoid mixing it with previous results.",
  "새 위치 다시 읽기": "Index new location",
  "옮긴 파일이 검색 목록에 아직 남아 있습니다": "Moved files are still in the search index",
  "예전 위치를 보여주지 않도록 검색을 잠시 멈췄습니다. 파일 목록을 새로고침하세요.":
    "Search is paused to avoid showing old locations. Refresh the file index.",
  "지금 새로고침": "Refresh now",
  "검색용 파일 목록 상태": "File-index status",
  "파일 이름과 위치를 읽고 있습니다…": "Reading file names and locations…",
  "파일과 폴더 {{count}}개를 바로 찾을 수 있습니다": "{{count}} files and folders are searchable",
  "아직 검색할 파일 목록이 없습니다": "No file index is available yet",
  "폴더 안의 파일을 확인하고 있습니다…": "Inspecting files in the folder…",
  "{{date}} 새로고침 · {{provider}} · {{mode}} · 파일 {{files}}개 · 폴더 {{folders}}개 · {{duration}}":
    "Refreshed {{date}} · {{provider}} · {{mode}} · {{files}} files · {{folders}} folders · {{duration}}",
  "찾을 위치를 선택하지 않음": "No search location selected",
  "{{root}} · 처음 한 번 이름과 위치를 읽습니다.": "{{root}} · Names and locations are read once initially.",
  "찾을 위치 선택": "Choose search location",
  "파일 목록 새로고침": "Refresh file index",
  "한 번 더 눌러 지우기": "Press again to clear",
  "파일 목록 지우기": "Clear file index",
  "확인함": "Inspected",
  "목록에 넣음": "Indexed",
  "파일 목록을 만들지 못했습니다": "Could not build the file index",
  "최근 파일 목록 만들기 결과": "Latest file-index result",
  "삭제 반영": "Removed entries",
  "읽기 실패": "Read failures",
  "파일 내용은 읽지 않고 이름·위치·크기·바뀐 시각만 저장합니다. 찾은 결과가 곧 지워도 되는 파일이라는 뜻은 아닙니다.":
    "File content is not read. Only name, location, size, and modification time are stored. A search result is not a recommendation to delete.",
  "한 번에 담을 수 있는 파일 수를 넘었습니다": "The file limit for one index was exceeded",
  "일부 파일이 빠졌을 수 있습니다. 더 작은 폴더를 선택하세요.":
    "Some files may be missing. Choose a smaller folder.",
  "확인하지 못한 위치 {{count}}개": "{{count}} locations could not be inspected",
  "검색 조건을 이해하지 못했습니다": "The search query could not be understood",
  "파일과 폴더 {{count}}개를 {{duration}}에 확인했습니다.":
    "Searched {{count}} files and folders in {{duration}}.",
  "처음 100개만 보여줍니다.": "Only the first 100 results are shown.",
  "일치하는 파일이나 폴더가 없습니다": "No matching files or folders",
  "검색어를 짧게 하거나 파일 종류와 크기 조건을 바꿔보세요.":
    "Try a shorter query or change the file-type and size filters.",
  "폴더 위치 일치": "Folder-path match",
  "현재 검색 방식": "Current search method",
  "Windows 드라이브 전체를 찾을 때는 가능한 경우 빠른 방식으로 파일 목록을 읽습니다. 권한이 없거나 특정 폴더를 고르면 일반 방식으로 안전하게 확인합니다.":
    "For a whole Windows drive, a fast index method is used when available. Without permission or for a selected folder, the app safely uses a portable scan.",
  "바로가기": "Link",
  "Windows 빠른 읽기": "Windows fast index",
  "일반 폴더 확인": "Portable folder scan",
  "바뀐 항목만 확인": "Changed items only",
  "전체 다시 확인": "Full refresh",
  "이름 모양 검색에는 * 또는 ?를 제외한 글자가 3자 이상 필요합니다. 예: glob:report-*.pdf":
    "A name-pattern search needs at least three characters besides * or ?. Example: glob:report-*.pdf",
  "OR의 앞과 뒤에 각각 찾을 이름이나 위치를 입력하세요.":
    "Enter a name or location on both sides of OR.",
  "OR로 나눈 각 조건에는 3자 이상의 이름이나 위치가 필요합니다.":
    "Each condition separated by OR needs a name or location of at least three characters.",
  "괄호는 사용할 수 없습니다. 두 조건 중 하나를 찾으려면 사이에 OR를 넣으세요.":
    "Parentheses are not supported. Put OR between two alternatives.",
  "큰따옴표로 묶은 문구의 끝에 닫는 큰따옴표를 넣으세요.":
    "Add a closing quote to the quoted phrase.",
  "파일 종류에는 pdf, jpg처럼 영문과 숫자만 입력하세요.":
    "Use only letters and numbers for file types, such as pdf or jpg.",
  "파일 크기 조건을 확인하세요. 예: size:>100mb": "Check the file-size condition. Example: size:>100mb",
  "날짜는 2026-01-31처럼 연도-월-일 순서로 입력하세요.":
    "Enter dates in year-month-day order, such as 2026-01-31.",
  "검색 조건을 확인하세요. 자세한 예시는 ‘검색을 더 정확하게 하는 법’에서 볼 수 있습니다.":
    "Check the search query. More examples are available under ‘How to refine your search’.",
  "개발 도구 관리": "Developer-tool management",
  "Docker를 사용하는 경우에만 켜세요.": "Enable this only if you use Docker.",
  "Docker 용량 관리": "Docker storage management",
  "켜면 사이드바에 Docker 전용 메뉴가 나타납니다. 대시보드에는 표시하지 않습니다.":
    "When enabled, a Docker view appears in the sidebar. It is not shown on the dashboard.",
  "Docker 용량 관리 사용": "Enable Docker storage management",
  "사용 안 함": "Off",
  "Docker 설정 적용 중…": "Applying Docker setting…",
  "Docker 설정 확인 중…": "Checking Docker setting…",
  "Docker 용량 메뉴가 켜졌습니다": "Docker storage view is enabled",
  "상태 확인과 정리 검토는 전용 화면에서 진행합니다.":
    "Review status and cleanup in the dedicated view.",
  "Docker 용량 열기": "Open Docker storage",
  "Docker 정리를 준비하고 있습니다": "Preparing Docker cleanup",
  "Docker가 관리하는 데이터": "Docker-managed data",
  "Docker 정리 전 확인": "Review before Docker cleanup",
  "Docker 정리 확인 창 닫기": "Close Docker cleanup dialog",
  "아래 명령은 Docker CLI에 직접 전달됩니다. 운영체제 휴지통을 거치지 않으며, 이미 완료된 단계는 취소해도 되돌릴 수 없습니다.":
    "These commands are sent directly to Docker CLI. They do not use the operating-system Trash, and completed steps cannot be undone by cancelling.",
  "정리 가능 최대": "Maximum reclaimable",
  "Docker 볼륨은 정리하지 않습니다": "Docker volumes are not cleaned",
  "데이터베이스와 사용자 파일이 들어 있을 수 있어 사용량만 표시합니다.":
    "Volumes may contain databases and user files, so only usage is shown.",
  "선택한 항목의 정리 가능 최대": "Maximum reclaimable for selected items",
  "실제 확보량은 Docker의 공유 계층과 실행 시점 상태에 따라 더 작을 수 있습니다.":
    "Actual reclaimed space may be lower due to Docker shared layers and runtime state.",
  "선택한 Docker 데이터는 휴지통으로 가지 않으며 복원할 수 없음을 확인했습니다.":
    "I understand that the selected Docker data does not go to Trash and cannot be restored.",
  "현재 Docker 단계를 중단하고 있습니다": "Stopping the current Docker step",
  "Docker 정리 진행률": "Docker cleanup progress",
  "닫기": "Close",
  "중단 요청": "Request stop",
  "선택 항목 정리": "Clean selected items",
  "완료 안 됨": "Not completed",
  "Docker가 보고한 정리량 {{size}}. 볼륨은 변경하지 않았습니다.":
    "Docker reported {{size}} reclaimed. Volumes were not changed.",
  "정리 이력 저장은 완료하지 못했습니다.": "Cleanup history could not be saved.",
  "Docker 작업을 완료하지 못했습니다": "Docker operation failed",
  "이미지": "Images",
  "컨테이너": "Containers",
  "볼륨": "Volumes",
  "빌드 캐시": "Build cache",
  "7일 이상 사용하지 않은 빌드 캐시": "Build cache unused for at least 7 days",
  "7일 이상 된 매달린 이미지": "Dangling images at least 7 days old",
  "7일 이상 된 중지 컨테이너": "Stopped containers at least 7 days old",
  "다음 빌드 때 다시 만들어질 수 있습니다.": "It can be rebuilt during a later build.",
  "태그가 없는 이미지 계층이며 필요하면 다시 내려받거나 빌드해야 합니다.":
    "These are untagged image layers and may need to be downloaded or built again.",
  "중지된 컨테이너의 쓰기 계층은 복원할 수 없습니다.":
    "Writable layers of stopped containers cannot be restored.",
  "Docker 정리를 완료했습니다": "Docker cleanup completed",
  "Docker 정리를 일부만 완료했습니다": "Docker cleanup partially completed",
  "Docker 정리를 취소했습니다": "Docker cleanup cancelled",
  "Docker 정리에 실패했습니다": "Docker cleanup failed",
  "Docker가 이 정리 단계를 완료했습니다": "Docker completed this cleanup step",
  "이 단계는 시작하지 않았습니다": "This step was not started",
  "Docker 용량 관리가 꺼져 있습니다": "Docker storage management is off",
  "설정에서 기능을 켜면 이 메뉴와 Docker 사용량이 표시됩니다.":
    "Enable the feature in Settings to show this view and Docker usage.",
  "Docker 용량 요약": "Docker storage summary",
  "Docker 범주 합계": "Docker category total",
  "Docker CLI가 보고한 논리 합계": "Logical total reported by Docker CLI",
  "볼륨을 제외한 참고 상한": "Reference upper bound excluding volumes",
  "Docker 사용량 확인 중": "Checking Docker usage",
  "Docker 연결됨": "Docker connected",
  "Docker를 사용할 수 없음": "Docker unavailable",
  "Docker 설정을 확인하고 있습니다.": "Checking Docker configuration.",
  "확인 안 됨": "Unknown",
  "Docker 사용량": "Docker usage",
  "전체 {{total}}개 · 사용 중 {{active}}개": "{{total}} total · {{active}} active",
  "사용량": "Usage",
  "보호": "Protected",
  "자동 정리 안 함": "Not auto-cleaned",
  "{{date}}에 Docker CLI로 확인": "Checked with Docker CLI at {{date}}",
  "Docker 사용량 확인 시각 없음": "Docker usage time unavailable",
  "최근 Docker 정리 · {{date}} · {{message}}": "Recent Docker cleanup · {{date}} · {{message}}",
  "Docker 다음 작업": "Next Docker action",
  "무엇을 정리할지 먼저 판단하세요": "Review what to clean first",
  "대화는 현재 Docker 요약만 전달합니다. 실제 정리는 이 앱의 최종 확인 뒤에만 실행됩니다.":
    "Chat receives only the current Docker summary. Cleanup runs only after final confirmation in this app.",
  "Docker 대화 시작": "Start Docker chat",
  "정리할 항목 없음": "Nothing to clean",
  "Docker 정리 검토": "Review Docker cleanup",
  "Docker 상태를 확인하지 못했습니다": "Could not check Docker status",
  "CLI 없음": "CLI missing",
  "앱 도구 없음": "App helper missing",
  "연결 안 됨": "Not connected",
  "연결됨": "Connected",
  "다른 설정과 충돌": "Conflicts with another configuration",
  "연결 복구 필요": "Connection repair needed",
  "상태 확인 실패": "Status check failed",
  "개발 빌드": "Development build",
  "연결을 해제": "disconnect",
  "연결": "connect",
  "{{client}}의 BroomSweepy MCP 연결을 해제할까요?\n\n앱이 등록한 항목과 현재 설정이 정확히 같을 때만 제거합니다.{{path}}":
    "Disconnect BroomSweepy MCP from {{client}}?\n\nOnly the exact entry registered by this app will be removed.{{path}}",
  "{{client}}에 BroomSweepy MCP를 연결할까요?\n\n정리 후보는 제한된 요약과 익명 번호만 전달됩니다. 파일·문서 검색을 따로 허용하면 경로와 일치 문맥이 전달될 수 있습니다. 파일 이동은 BroomSweepy 앱의 최종 확인 없이는 실행되지 않습니다.{{path}}":
    "Connect BroomSweepy MCP to {{client}}?\n\nCleanup candidates are shared only as limited summaries and anonymous IDs. If file or document search is separately allowed, paths and matching context may be shared. Files never move without final confirmation in BroomSweepy.{{path}}",
  "\n\nBroomSweepy MCP 실행 파일:\n{{path}}": "\n\nBroomSweepy MCP executable:\n{{path}}",
  "외부 AI에 BroomSweepy 연결": "Connect BroomSweepy to external AI",
  "Codex·Claude Code가 파일 대신 로컬 앱의 제한된 결과를 읽게 합니다.":
    "Lets Codex and Claude Code read limited results from the local app instead of scanning files themselves.",
  "MCP 연결 상태 새로 고침": "Refresh MCP connection status",
  "요청 요약·도구 선택·판단은 외부 AI가 맡지만, 검사·검색·재검증·휴지통 이동은 이 컴퓨터의 BroomSweepy가 수행합니다. MCP에는 승인이나 영구 삭제 명령이 없습니다.":
    "External AI handles request summaries, tool choice, and reasoning. BroomSweepy on this computer performs scans, searches, rechecks, and Trash moves. MCP exposes no approval or permanent-delete command.",
  "설치된 연결 도구를 확인하고 있습니다.": "Checking installed connection tools.",
  "연결을 쓰려면 {{client}}을 다시 시작해 주세요.": "Restart {{client}} to use the connection.",
  "처리 중…": "Working…",
  "연결 해제": "Disconnect",
  "연결 복구": "Repair connection",
  "Claude Desktop 확장과 Claude Code 연결은 서로 다릅니다. 현재 자동 연결은 Codex와 Claude Code만 지원합니다.":
    "Claude Desktop extensions and Claude Code connections are different. Automatic setup currently supports Codex and Claude Code only.",
  "MCP 연결 상태를 확인하지 못했습니다.": "Could not check MCP connection status.",
  "파일 검사": "File scan",
  "드라이브 검사": "Drive scan",
  "폴더 분석": "Folder analysis",
  "정리 후보 검사": "Cleanup candidate scan",
  "문서 읽기": "Document indexing",
  "문서 검색": "Document search",
  "빠른 파일 목록 만들기": "Build fast file index",
  "휴지통 이동 확인": "Trash move review",
  "기다리는 중": "Queued",
  "진행 중": "In progress",
  "완료하지 못함": "Failed",
  "취소됨": "Cancelled",
  "요청한 작업": "Requested task",
  "로컬 연결 기능을 시작하지 못했습니다": "Could not start local connectivity",
  "연결 없이도 앱의 검사와 검색은 그대로 사용할 수 있습니다.":
    "You can still use scans and searches in the app without a connection.",
  "CLI에서 요청한": "CLI-requested",
  "앱에서 시작한": "App-started",
  "현재 상태를 확인하고 있습니다.": "Checking the current status.",
  "로컬 CLI {{count}}개 연결됨": "{{count}} local CLI clients connected",
  "허용한 범위 안에서 검사와 검색 요청을 받을 수 있습니다.":
    "Scan and search requests can be accepted within the allowed scope.",
  "현재 연결된 로컬 CLI가 없습니다": "No local CLI is currently connected",
  "마지막 연결 {{date}}": "Last connected {{date}}",
  "연결된 로컬 CLI가 없습니다": "No local CLI connected",
  "CLI 설치 뒤 BroomSweepy MCP 연결 도구를 별도로 등록해야 합니다.":
    "After installing the CLI, register the BroomSweepy MCP helper separately.",
  "연결 상태": "Connection status",
  "{{operation}} 진행 상황": "{{operation}} progress",
  "로컬 CLI 요청 오류: {{detail}}": "Local CLI request error: {{detail}}",
  "{{count}}개 연결": "{{count}} connected",
  "연결 없음": "Not connected",
  "연결 꺼짐": "Connection off",
  "확인 가능 시각 {{date}}까지": "Available for review until {{date}}",
  "검토 열기": "Open review",
  "파일·문서 검색 허용": "Allow file and document search",
  "{{targets}} 목록 검색을 이번 실행에서 허용했습니다.":
    "{{targets}} index search is allowed for this run.",
  "앱이 이미 만든 파일·문서 목록만 검색합니다.":
    "Searches only file and document indexes already created by the app.",
  "로컬 연결 기능이 준비되면 허용할 수 있습니다.":
    "You can allow this after local connectivity is ready.",
  "빠른 파일 목록이나 문서 목록을 먼저 만들어 주세요.":
    "Build a fast file index or document index first.",
  "허용해도 새 파일 검사를 시작하거나 파일을 바꾸지 않습니다.":
    "Allowing this does not start a new scan or change files.",
  "검색 허용 오류: {{detail}}": "Search permission error: {{detail}}",
  "바꾸는 중…": "Updating…",
  "검색 허용 끄기": "Disable search access",
  "이번 실행에서 검색 허용": "Allow search for this run",
  "폴더 검사 허용": "Allow folder scan",
  "이 실행에서 허용됨": "Allowed for this run",
  "허용 안 됨": "Not allowed",
  "로컬 CLI는 아래 폴더와 현재 설정으로 검사 시작만 요청합니다. 실제 파일 확인은 이 앱이 수행합니다.":
    "The local CLI can only request a scan of the folder below with the current settings. This app performs the actual inspection.",
  "큰 파일 {{large}} 이상 · 중복 {{duplicate}} 이상 · 결과 {{largeCount}}/{{duplicateCount}}개":
    "Large files {{large}}+ · duplicates {{duplicate}}+ · results {{largeCount}}/{{duplicateCount}}",
  "파일을 수정하거나 이동하지 않습니다. 앱을 닫거나 폴더·설정을 바꾸면 허용이 꺼집니다.":
    "Files are not modified or moved. Access is revoked when you close the app or change the folder or settings.",
  "검사 허용 오류: {{detail}}": "Scan permission error: {{detail}}",
  "검사 허용 끄기": "Disable scan access",
  "이번 실행에서 검사 허용": "Allow scan for this run",
  "정리 계획 검토 허용": "Allow cleanup-plan review",
  "외부 AI가 익명 후보 번호로 계획을 만들 수 있습니다. 실제 경로와 승인은 앱에만 표시됩니다.":
    "External AI can build a plan using anonymous candidate IDs. Actual paths and approval are shown only in the app.",
  "AI에는 종류와 용량 요약만 전달하고, 파일 이동은 앱에서 다시 확인합니다.":
    "Only type and size summaries are sent to AI. File moves require another review in the app.",
  "진행 중인 검사나 휴지통 작업이 끝난 뒤 이 권한을 바꿀 수 있습니다.":
    "You can change this permission after the active scan or Trash operation finishes.",
  "MCP에는 승인·실행·영구 삭제 기능이 없습니다. 앱을 닫으면 허용이 꺼집니다.":
    "MCP has no approval, execution, or permanent-delete capability. Access is revoked when the app closes.",
  "큰 파일·중복 검사 또는 정리 후보 검사를 먼저 완료해 주세요.":
    "Complete a large-file and duplicate scan or a cleanup-candidate scan first.",
  "정리 검토 허용 오류: {{detail}}": "Cleanup review permission error: {{detail}}",
  "정리 검토 허용 끄기": "Disable cleanup review",
  "이번 실행에서 검토 허용": "Allow review for this run",
  "이전 휴지통 작업 확인 결과": "Previous Trash operation review",
  "작업 복구": "Operation recovery",
  "확인이 필요한 이전 휴지통 작업이 있습니다": "A previous Trash operation needs review",
  "이전 작업 기록을 일부만 확인했습니다": "Only part of the previous operation record was verified",
  "중단된 휴지통 작업을 자동으로 대조했습니다": "Interrupted Trash operations were checked automatically",
  "자동 복원이나 영구 삭제는 하지 않았습니다. 원본과 운영체제 휴지통을 직접 확인하세요.":
    "Nothing was restored or permanently deleted automatically. Check the original location and operating-system Trash or Recycle Bin.",
  "원본 경로, 완료 기록, 운영체제 휴지통을 비교했으며 파일을 추가로 변경하지 않았습니다.":
    "The original paths, completion records, and operating-system Trash or Recycle Bin were compared without changing any files.",
  "이전 작업 알림 닫기": "Dismiss previous-operation notice",
  "복구 대조 요약": "Recovery comparison summary",
  "중단 기록": "Interrupted records",
  "{{count}}건": "{{count}}",
  "확인 항목": "Checked items",
  "직접 확인": "Needs review",
  "확인 시각": "Checked at",
  "{{date}} 작업": "Operation on {{date}}",
  "{{count}}개 계획": "{{count}} planned",
  "{{count}}개 직접 확인": "{{count}} need review",
  "자동 대조 기록 저장": "Automatic comparison saved",
  "자동 대조 완료": "Automatic comparison complete",
  "나머지 {{count}}개 항목은 작업 기록에 보존되어 있습니다.":
    "The remaining {{count}} items are preserved in the operation record.",
  "나머지 {{count}}건은 작업 기록에 보존되어 있습니다.":
    "The remaining {{count}} operations are preserved in the operation record.",
  "대조 중 확인하지 못한 내용": "Items that could not be verified",
  "휴지통 여는 중": "Opening Trash",
  "운영체제 휴지통 열기": "Open operating-system Trash",
  "이전 휴지통 작업을 확인하지 못했습니다": "Could not review previous Trash operations",
  "이전 작업 기록을 확인하고 있습니다": "Reviewing previous operation records",
  "원본 경로와 운영체제 휴지통을 대조하는 중입니다. 파일은 변경하지 않습니다.":
    "Comparing original paths with the operating-system Trash or Recycle Bin. No files are being changed.",
  "이전 작업 확인 오류 닫기": "Dismiss previous-operation review error",
  "“{{scope}}”의 메시지 {{count}}개를 삭제할까요?\n\n이 대화와 저장된 {{summary}}만 삭제하며 실제 데이터는 그대로 둡니다.":
    "Delete {{count}} messages from “{{scope}}”?\n\nOnly this conversation and its saved {{summary}} will be deleted. The actual data remains unchanged.",
  "Docker 요약": "Docker summary",
  "폴더 요약": "folder summary",
  "AI 응답은 받았지만 대화 기록에 저장하지 못했습니다. {{detail}}":
    "The AI response was received but could not be saved to chat history. {{detail}}",
  "대화 기록 관리": "Manage chat history",
  "새 폴더 확인 중": "Checking new folder",
  "새 폴더 대화": "New folder chat",
  "Docker 대화": "Docker chat",
  "저장된 대화 선택": "Choose saved chat",
  "대화 기록 불러오는 중": "Loading chat history",
  "저장된 대화 없음": "No saved chats",
  "{{scope}} 대화 삭제": "Delete {{scope}} chat",
  "현재 대화 삭제": "Delete current chat",
  "현재 대화만 삭제": "Delete only the current chat",
  "삭제": "Delete",
  "현재 대화 대상": "Current chat target",
  "대화 대상": "Chat target",
  "새 대화를 시작하세요": "Start a new chat",
  "새 Docker 대화": "New Docker chat",
  "설치된 AI CLI 상태 확인 중": "Checking installed AI CLI status",
  "대화 상대 선택": "Choose AI CLI",
  "AI CLI 확인 중": "Checking AI CLI",
  "Ollama 모델 선택": "Choose Ollama model",
  "설치된 모델 없음": "No installed models",
  "AI CLI 상태 다시 확인": "Refresh AI CLI status",
  "Docker 용량 대화": "Docker storage chat",
  "폴더 분석 대화": "Folder analysis chat",
  "나": "You",
  "AI 도우미": "AI assistant",
  "{{provider}} 응답 생성 중 · 언제든 취소할 수 있습니다":
    "{{provider}} is generating a response · you can cancel at any time",
  "{{provider}}가 앱의 {{summary}}을 읽고 있습니다": "{{provider}} is reading the app's {{summary}}",
  "Docker 사용량과 정리 검토": "Docker usage and cleanup review",
  "Docker 범주 합계 {{size}}": "Docker category total {{size}}",
  "Docker 상태를 확인해 주세요": "Check Docker status",
  "볼륨 제외 참고 상한 {{size}} · 실제 디스크 사용량과 다를 수 있음":
    "Estimated maximum excluding volumes {{size}} · may differ from actual disk usage",
  "Docker 용량에 관해 질문": "Ask about Docker storage",
  "선택한 폴더에 관해 질문": "Ask about the selected folder",
  "AI 응답 취소": "Cancel AI response",
  "질문 보내기": "Send question",
  "폴더나 파일 내용이 아니라, BroomSweepy가 Docker CLI로 읽은 범주별 용량 요약만 {{provider}}에 전달합니다.":
    "Only category-level storage summaries read by BroomSweepy through Docker CLI are sent to {{provider}}, not folder or file contents.",
  "파일 내용이나 전체 경로가 아니라, BroomSweepy가 만든 폴더 이름·크기 요약만 {{provider}}에 전달합니다.":
    "Only folder-name and size summaries created by BroomSweepy are sent to {{provider}}, not file contents or full paths.",
  "연결과 권한": "Connections and permissions",
  "외부 터미널 제어와 전송 범위 확인": "Review external terminal control and shared data",
  "Docker 조회는 BroomSweepy가 수행합니다. 앱은 범주별 사용량과 정리 가능 참고 상한만 선택한 AI CLI의 질문 입력으로 보냅니다.":
    "BroomSweepy performs Docker inspection. The app sends only category usage and an estimated cleanup maximum in the prompt to the selected AI CLI.",
  "폴더 선택과 읽기 검사는 BroomSweepy가 수행합니다. 앱은 파일 내용과 전체 경로를 빼고 제한된 요약만 선택한 AI CLI의 질문 입력으로 보냅니다.":
    "BroomSweepy selects and scans the folder. The app excludes file contents and full paths, sending only a limited summary in the prompt to the selected AI CLI.",
  "아래 설정은 별도 터미널 제어용입니다.": "The settings below are for separate terminal control.",
  "폴더 선택부터 시작합니다": "Start by choosing a folder",
  "설정에서 Docker 관리를 켜세요": "Enable Docker management in Settings",
  "정리 가능 최대 {{size}}": "Up to {{size}} reclaimable",
  "{{drive}} 전체의 {{share}}": "{{share}} of {{drive}}",
  "대화 기록을 불러오고 있습니다": "Loading chat history",
  "이 컴퓨터에 저장된 최근 폴더 대화를 확인합니다.": "Checking recent folder chats stored on this computer.",
  "새 폴더를 살펴보고 있습니다": "Inspecting the new folder",
  "새 대화는 폴더 선택부터 시작합니다": "A new chat starts by choosing a folder",
  "폴더를 고르면 앱이 용량을 계산하고 빈 대화를 만듭니다.":
    "Choose a folder and the app will calculate its size and create an empty chat.",
  "새 대화": "New chat",
  "{{scope}} 대화 준비됨": "{{scope}} chat is ready",
  "범주 합계 {{total}} · 정리 가능 최대 {{reclaimable}}":
    "Category total {{total}} · up to {{reclaimable}} reclaimable",
  "{{size}} · 파일 {{count}}개 · {{date}} 검사": "{{size}} · {{count}} files · scanned {{date}}",
  "예: Docker에서 무엇이 가장 크고 무엇부터 정리할까?":
    "Example: What uses the most Docker storage, and what should I clean first?",
  "예: 어느 폴더가 가장 크고 무엇부터 확인해야 해?":
    "Example: Which folder is largest, and what should I review first?",
  "설치된 AI CLI를 확인하고 있습니다.": "Checking installed AI CLI tools.",
  "설치된 AI CLI를 확인하고 있습니다…": "Checking installed AI CLI tools…",
  "Ollama 모델을 선택해 주세요…": "Choose an Ollama model…",
  "새 대화를 눌러 폴더를 선택해 주세요…": "Choose New chat, then select a folder…",
  "Docker 대화를 준비하고 있습니다…": "Preparing Docker chat…",
  "새 폴더 검사가 끝나면 질문할 수 있습니다…": "You can ask after the new folder scan finishes…",
  "{{scope}} · 메시지 {{count}}개 · {{date}}": "{{scope}} · {{count}} messages · {{date}}",
  "{{provider}} · 설치 안 됨": "{{provider}} · not installed",
  "{{provider}} · 로그인 필요": "{{provider}} · sign-in required",
  "{{provider}} · 모델 {{count}}개": "{{provider}} · {{count}} models",
  "{{provider}} · 모델 없음": "{{provider}} · no models",
  "{{provider}} · 로그인됨": "{{provider}} · signed in",
  "Codex는 앱 전용 빈 폴더에서 읽기 전용 샌드박스로 실행합니다. Codex 자체 읽기 도구의 실제 범위는 Codex 샌드박스 정책을 따릅니다.":
    "Codex runs in a read-only sandbox from an empty app-specific folder. The actual reach of Codex read tools follows Codex sandbox policy.",
  "Claude Code는 세션 저장과 도구 사용을 끄고, 승인 질문 없이 안전 모드로 실행합니다.":
    "Claude Code runs in a safe mode with session storage and tool use disabled and without approval prompts.",
  "Grok은 단일 응답 모드에서 내장 도구, 하위 에이전트, 웹 검색을 끕니다. Grok CLI 자체 계정과 세션 정책은 그대로 적용됩니다.":
    "Grok runs in single-response mode with built-in tools, subagents, and web search disabled. Grok CLI account and session policies still apply.",
  "Antigravity는 비대화형 응답 모드와 샌드박스로 실행합니다. Antigravity 자체 계정과 설정 정책은 그대로 적용됩니다.":
    "Antigravity runs in non-interactive response mode with a sandbox. Antigravity account and configuration policies still apply.",
  "Ollama에는 도구를 제공하지 않습니다. 로컬 모델이면 요약이 컴퓨터 안에서 처리되고, cloud 모델이면 Ollama 서비스로 전송됩니다.":
    "No tools are provided to Ollama. A local model processes the summary on this computer; a cloud model sends it to the Ollama service.",
  "AI CLI를 고르면 이곳에 해당 공급자의 실행 권한을 표시합니다.":
    "Choose an AI CLI to see its execution permissions here.",
  "선택한 AI CLI": "the selected AI CLI",
  "{{provider}}를 먼저 설치해 주세요…": "Install {{provider}} first…",
  "Ollama에 대화용 모델을 먼저 설치해 주세요…": "Install a chat model in Ollama first…",
  "{{provider}}에서 먼저 로그인해 주세요…": "Sign in to {{provider}} first…",
  "AI CLI 응답을 받지 못했습니다": "Could not receive a response from the AI CLI",
  "Docker 사용량 다시 확인 중": "Rechecking Docker usage",
  "{{kind}} 정리 중": "Cleaning {{kind}}",
  "Docker 정리를 진행하고 있습니다": "Docker cleanup is in progress",
  "시작 전": "Not started",
  "원본 유지": "Original retained",
  "이동 기록 확인": "Move record confirmed",
  "휴지통 확인": "Found in Trash",
  "실패·원본 확인": "Failed · original confirmed",
  "양쪽에 존재": "Present in both locations",
  "위치 불명": "Location unknown",
  "휴지통 확인 불가": "Could not inspect Trash",
  "경로 확인 불가": "Could not inspect path",
} as const;

export type MessageKey = keyof typeof englishMessages;
export type MessageValues = Record<string, string | number>;
export type Translate = (message: MessageKey, values?: MessageValues) => string;

const japaneseMessages = japaneseCatalog as Record<MessageKey, string>;
const chineseMessages = simplifiedChineseCatalog as Record<MessageKey, string>;

interface LanguageContextValue {
  preference: LanguagePreference;
  language: ResolvedLanguage;
  storageError: boolean;
  setPreference: (preference: LanguagePreference) => void;
  t: (message: MessageKey, values?: MessageValues) => string;
}

const LanguageContext = createContext<LanguageContextValue | null>(null);

function readPreference(): LanguagePreference {
  if (typeof window === "undefined") return "en";
  try {
    return normalizeLanguagePreference(window.localStorage.getItem(LANGUAGE_STORAGE_KEY));
  } catch {
    return "en";
  }
}

function interpolate(template: string, values?: MessageValues): string {
  if (!values) return template;
  return template.replace(/\{\{(\w+)\}\}/g, (match, key: string) =>
    Object.prototype.hasOwnProperty.call(values, key) ? String(values[key]) : match,
  );
}

export function LanguageProvider({ children }: { children: ReactNode }) {
  const [preference, setPreferenceState] = useState<LanguagePreference>(readPreference);
  const [storageError, setStorageError] = useState(false);
  const language = preference;

  setFormattingLanguage(language);

  useEffect(() => {
    document.documentElement.lang = language;
    void setApplicationLanguage(language).catch(() => {
      // The web UI remains usable if a platform has no native language surface.
    });
  }, [language]);

  const setPreference = useCallback((next: LanguagePreference) => {
    setPreferenceState(next);
    try {
      window.localStorage.setItem(LANGUAGE_STORAGE_KEY, next);
      setStorageError(false);
    } catch {
      setStorageError(true);
    }
  }, []);

  const t = useCallback(
    (message: MessageKey, values?: MessageValues) => {
      const template =
        language === "ko"
          ? message
          : language === "ja"
            ? japaneseMessages[message] ?? englishMessages[message]
            : language === "zh-CN"
              ? chineseMessages[message] ?? englishMessages[message]
              : englishMessages[message];
      return interpolate(template, values);
    },
    [language],
  );

  const value = useMemo(
    () => ({ preference, language, storageError, setPreference, t }),
    [language, preference, setPreference, storageError, t],
  );

  return <LanguageContext.Provider value={value}>{children}</LanguageContext.Provider>;
}

export function useLanguage(): LanguageContextValue {
  const value = useContext(LanguageContext);
  if (!value) throw new Error("useLanguage must be used within LanguageProvider");
  return value;
}
