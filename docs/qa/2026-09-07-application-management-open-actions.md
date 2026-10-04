# 앱 관리·로컬 열기 — 플랫폼별 검증

## 범위

- 승인: 정식 제거 우선 + Mac 일반 앱 본체 Trash + 관련 데이터 별도 선택; Windows에 맞는 OS 제거 절차.
- 설치 앱 목록은 독립적·bounded metadata 조회. 아이콘/앱 전체 크기를 미리 계산하지 않는다. Mac UI 목록은 알려진 루트의 직계 앱이며 전체 디스크의 앱 발견을 보장하지 않는다.
- Mac 캐시·환경설정 후보는 exact bundle ID, 알려진 루트의 깊이/항목 제한 소유관계 확인 및 최종 재검증. 불완전한 확인은 후보 제외. 이 확인이 디스크의 모든 복사본 부재를 증명하지는 않는다.
- Windows 레지스트리 목록과 고정 `ms-settings:appsfeatures`. 원시 제거 문자열/임의 앱 폴더/AppData/레지스트리 삭제 없음. Store 앱 등 목록 밖의 앱은 OS 설정에서 관리한다.
- 열기/위치 표시는 실제 종류·no-follow·클라우드·온라인 전용·실행형 검사 후 OS 요청. 링크/Finder 별칭은 검증한 부모만 표시한다. 경로 기반 최종 OS 요청의 TOCTOU를 완전히 제거했다고 주장하지 않는다.

## 자동 검증 — Apple Silicon Mac

| 검증 | 실행 결과 |
|---|---|
| `cargo fmt --all -- --check` | PASS |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS |
| `cargo test --workspace --all-targets` | 269 PASS, 1 ignored (설치 CLI 선택 진단) |
| `npm run check` | PASS |
| `npm run test:all` | 43 PASS, locale 키/placeholder 포함 |
| `git diff --check` | PASS |
| `cargo test --release -p bloomsweepy-core --test document_worker_boundary --target aarch64-apple-darwin` | 2 PASS (최종 설치 검사에서 추가) |

Rust 결과 세부: control17 + core78 + cloud4 + scan2 + resource2 + document-worker1 + desktop155 + MCP10 =269. 새 앱/별칭/journal 테스트는 합성 파일과 mock backend로 실행했으며 사용자 앱이나 실제 OS Trash를 삭제하지 않았다.

## 실제 React 브라우저 + mock IPC

실제 OS 호출은 모두 0. 개발 전용 fixture는 배포 빌드에 포함되지 않는다.

- StrictMode + 독점 prepare fixture: 본체 검토 1회 발행, 2개 체크 모두 필요, 미선택 기본값, Cancel 초점.
- 본체 mock 이동1회 → 별도 후보 기본미선택 → 캐시만 mock 이동1회 → 미선택 환경설정 후보 보존.
- 만료 계획 실행 버튼 disabled. Escape 취소 시 dispatch0, trigger 버튼으로 초점 복귀.
- 응답 유실 fixture는 미확인 오류와 재실행 차단 표시. 처리 중 Escape로 확인창이 닫히지 않음.
- Windows fixture: Mac Trash 버튼 없음, 설정 버튼1회 → 설정 요청1/Trash0, 제거 완료로 표시하지 않음.
- 57개 앱의 검색 동작과 50개 페이지 구성 확인. 760×600 Windows 화면 가로 넘침 없음, 주요 작업 표시.
- FileTable checkbox는 open0; 열기 버튼 doubleclick은 IPC1; 스크립트는 위치 안내; 명시적 위치는 `reveal_local_path`.
- Cleanup 후보 폴더 열기, treemap 메뉴 폴더 열기는 inspect IPC. 내부 하위폴더 탐색은 별도 OS 호출 없음. 빈폴더 Open/Reveal 버튼 확인.
- 열기 실패 fixture는 오류 표시. 작은 창 treemap의 메뉴·열기 버튼 가시성 확인.
- 화면 이동 후보 보존은 독립 정적 검토 완료. 설치본에서도 검색한 앱 목록 상태가 대시보드 이동·복귀 뒤 유지됨을 확인했다. 실제 앱 제거 후 관련 후보의 네이티브 보존은 삭제를 하지 않았으므로 미실행.

## 독립 검토와 흐름도 대조

WorkPM native read-only 역할을 활용한 embedded review gate. 외부 CLI 전체 dirty-worktree 리뷰는 실행하지 않음. MCP NOT RUN. 설계/보안/테스트/API/유지보수/성능 체크리스트를 전달했고 Lead가 실행 결과를 검증했다.

| 발견 사항 | 수정·근거 |
|---|---|
| 과도한 메타데이터로 누락된 앱을 완전한 소유관계로 오인 | 누락 issue 기록, 후보 제외; oversized duplicate 합성 테스트 |
| 중첩 설치된 동일 bundle ID 누락 | 별도 bounded ownership walk; nested copy 합성 테스트 |
| OS가 이동 후 오류를 반환하면 완료로 기록 | 미완료 journal + refresh로 풀리지 않는 inspection latch; move-then-error mock |
| alias를 문서로 실행 / 링크의 cloud target reveal | FinderInfo32bytes nofollow + parent-only dispatch; alias/link 합성 테스트 |
| StrictMode 계획 중복 / 잔여 후보 소실 | effect 폐기 gate, movedPaths만 제외, lazy retained view; 정적 재검토 및 mock 회귀 |

`application-management.mmd`의 실패·취소·만료·불확실 잠금·별도 데이터 단계, `storage-open.mmd`의 intent/종류/최종재검증/OS요청과 코드가 대응한다. 도메인 사전의 `applicationTrashPlan`, `applicationDataCandidate`, `fileInspection` 의미를 유지하며 Windows 설정 열기를 제거 완료로 표현하지 않는다.

## 설치형 및 남은 제한

### 배포용 메모리 자가검사 회귀

첫 arm64 최종 번들의 `--check-heap-budget`는 exit1/출력없음, debug helper는 exit0으로 재현되어 설치를 멈췄다. 자가검사의 사용하지 않는 전역 alloc/dealloc 쌍을 최적화가 생략하거나 성공으로 취급할 수 있다는 [Rust GlobalAlloc 안전 계약](https://doc.rust-lang.org/std/alloc/trait.GlobalAlloc.html#safety)에 따라, 같은 전역 allocator 인스턴스의 구체적인 alloc/dealloc 메서드를 직접 호출하도록 진단만 수정했다. 배포용 경계 테스트2개와 standalone release 진단이 통과했다. 이는 진단 최적화 문제라는 해석을 뒷받침하지만 전체 프로세스 RSS 상한을 실증한 것은 아니다. prepare-sidecar에서도 이 검사를 필수 실행한다.

- 09:38–09:44 KST 최종 arm64 번들로 `/Applications/BroomSweepy.app` 교체 완료. 이전 앱은 `/Users/dannysmacair/Library/Application Support/BroomSweepy-install-wVDxAR/BroomSweepy.app`에 보관했으며 설정·대화 데이터 디렉터리는 교체하지 않았다.
- 새 본체 SHA256 `a3f8991b83c3cf1919743edd66feb8ed7e41f87bca74257c6eb762da29dbffdf`, helper `e9068a569b065557b171f7746154490b16e3b1020000686b4572bca735fb9ff0`: 설치본과 빌드 산출물 일치. 3개 실행파일 arm64, deep/strict 서명검사, helper버전1.6.1, 최종 설치 helper heap probe PASS. Finder 권한 목적 설명과 entitlement 포함 확인; 권한 요청/승인은 수행하지 않음.
- 설치본 실제 앱62개 목록·검색, 시스템/자기자신/실행앱/보조서비스 보호 표시 확인. 일반 앱 검토창의 두 체크 미선택/최종버튼 비활성/취소 확인. 사용자 앱 이동0회.
- 네이티브 휴지통 비우기 경고창도 범위/영구삭제 경고·미선택·최종버튼 비활성·취소 확인. 실제 비우기0회.
- 직접 만든160bytes/2files 폴더의 지도 검사 후 폴더 열기→Finder에 `Test folder/item.txt`, 파일 열기→TextEdit에 합성 readme내용, 위치 표시→Finder readme선택까지 확인. 검사 창만 닫고 fixture는 backup디렉터리의 `native-open-fixture`로 이동했다. 새 앱을 재시작해 임시 검사 상태도 정리한다.
- 프런트 배포 번들은 JS 약792.55kB/압축228.50kB이며 기존500kB 청크 경고는 남는다. 실제 앱 전체 RSS·장시간 누적 안정성 증명이 아니다.
- Windows 네이티브 빌드·동작은 이 Mac에서 실행하지 못했다. 기존 Windows CI가 전체 Cargo/프런트 테스트와 빌드를 수행하도록 되어 있으나 이번 변경을 push하지 않았으므로 그 실행을 PASS로 주장하지 않는다.
- 실제 사용자 앱 삭제, 실제 Empty Trash, Finder Automation 권한 변경, 시간 초과 유발, 전체 앱 장시간 저메모리 soak는 미실행.
- 버전은1.6.1 로컬 개발 빌드이며 새 릴리즈/커밋/푸시 없음. ad-hoc 서명은 Apple 공증이 아니다.
