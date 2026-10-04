# 저자원 안정화와 일반 폴더 작업

- 사용자 승인: 전체적으로 파악한 문제 수정 진행. RAM8GiB/여유 디스크 약16GiB를 기준 환경으로 유지.
- Phase1–3: 기존 진단·사전 로드, 세 대안 비교, 스트리밍/자원 예산/확인 계획 채택. 도면 및 소유권 기록.
- workpm native-first: 독립 파일 작업만 위임, 컴파일과 테스트는 Lead 직렬 실행. MCP NOT RUN. 사용자 파일/클라우드 순회/실제 OOM/디스크 채우기/설치/릴리스는 하지 않음.
- Phase4: streaming_walk·index_budget·resource_guard·document_worker, 일반 폴더 계획/UI/번역 통합. jwalk 의존 제거, 새 helper 패키징/CI 준비.
- Phase5: 독립 교차검토 4개 확정 문제 수정(인프라 실패 색인소실/직접정리RAM/OS자원오류/마운트루트). 재검토 PASS. Rust workspace와 frontend, 20회합성반복 최고RSS, UI 5경로 실제 실행. 상세 숫자/제약은 docs/qa/2026-09-07-low-resource-folder-actions.md.
- 완료 범위는 소스와 debug/mock 검증. 서명 설치본/Windows/nativeTrash/프로세스트리 장시간/실제 Codex 새계약 검증을 완료로 표시하지 않음. 최종메모리와handoff에 재진입 근거 저장.

## 추가 요청: 앱 관리와 선택 항목 열기

- WorkPM Phase1: 앱 관리/파일 열기를 독립 읽기 전용 작업자로 조사. 소유권 중첩 수정·사용자 데이터 실행 없음.
- 앱 본체 Trash/OS 제거 화면만/관련 데이터 포함 3대안 적합성·안전성·구현 부담 점수 제시. 앱 본체만 추천했지만 **범위 선택 응답 대기**.
- 설치 작업의 arm64 executable 생성은 성공했으나 x64 Node/Tauri 패키저의 기본 대상 불일치로 Intel sidecar를 찾음. `--target aarch64-apple-darwin` 명시해 재빌드 중. 기존 /Applications 앱은 아직 교체하지 않음.
- 신규 계획: docs/plan/application-management-open-actions/plan.md. 도메인사전은 기존 UI/코드 컨텍스트의 8개 용어, 글로벌 승격 없이 프로젝트 범위로 작성.

### 승인 후 구현 진행

- 사용자 “진행하자. 윈도우는 윈도우에 맞춰서 진행”으로 정식 제거 우선 + 별도 관련 데이터 선택 승인. Mac 관련 데이터는 정확한 bundle ID 캐시/환경설정만, Windows는 고정 OS 설치된 앱 제거 설정으로 연결하고 AppData 추측 삭제 없음.
- Phase2–3: application-management/storage-open 도면 및 소유권/API 계약 확정. 기존 일반 폴더 App 보호 유지, no-follow 검사·공유 작업 잠금·one-shot·실제 결과 표시가 경계.
- Phase4: application_backend/storage_open_impl/application_ui 독립 소유권으로 위임; Lead는 네비게이션/command등록/번역/검증 통합. Native-first, MCP NOT RUN. 실제 사용자 앱 삭제/Trash 비우기 없음.
- 이전 승인 수정분 arm64 app 번들 성공 확인, 신규 기능 포함 최종 재빌드/설치까지 기존 설치본 유지.

### 설치와 공정 점검 완료

- WorkPM Phase5: 독립 검토 후 alias/link-parent dispatch, 중첩 앱 소유관계, 불확실 OS journal, StrictMode/부분후보/화면이동 상태 문제 수정. Rust269+frontend43, release helper2, Clippy/format/build PASS. 실제 Windows 빌드·실행은 NOT RUN.
- 배포용 heap probe debug/release 차이를 재현해 ALLOCATOR 직접 진단으로 수정하고 prepare-sidecar 필수 gate 추가. 전체 RSS 보장으로 표현하지 않음.
- 09:38–09:44KST 새 arm64 /Applications/BroomSweepy.app 설치·서명·hash 일치·helper heap check PASS. 복구용 이전 앱은 Library/Application Support/BroomSweepy-install-wVDxAR/BroomSweepy.app 보존. 버전1.6.1 로컬 개발본이며 새 릴리즈/커밋/푸시 없음.
- 설치본 앱62개·보호·검토취소·EmptyTrash경고취소·검색상태복귀 확인. 자체160byte fixture의 Finder폴더/TextEdit파일/Reveal선택 실제확인; 사용자 앱 삭제/실제 비우기/권한변경0.
- 자체fixture는 backup/native-open-fixture로 보관, 테스트 Finder/TextEdit창·브라우저tab6·Vite1421 정리. 앱 재시작 후 설치된 앱 화면을 표시. 최종 기록은 docs/qa/2026-09-07-application-management-open-actions.md.
