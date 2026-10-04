# 앱 기능 계약·조사 루프 검증

2026-10-04 · source: codex · 현재 개발본, 공개 배포/설치 판정 아님

## 요구

LLM은 앱 조회를 선택하고 실제 결과를 분석·추가 조회한다. 데이터 수집/검색/실행은 앱이 담당한다. 모델에 최종 실행/승인 도구를 제공하지 않는다.

## 실제 실행

- 최종 공통 control23 + MCP12 테스트 통과(메인 재실행).
- desktop 전체 lib: 191 통과, 3 ignored, 실패0. 실제 앱 CPU61 evidence가 다음 CLI 입력에 들어가는 scripted 2라운드, 중복/4행동/48KiB, 읽지 않는 CLI stdin timeout/cancel, 상태 소유권·동의 철회·원래 루트·완료 이력 상한 검사를 포함한다. 3 ignored는 실제 CLI/네이티브 휴지통의 opt-in 검사다.
- 최종 `cargo clippy --workspace --all-targets -- -D warnings`, `git diff --check` 통과.
- 프런트엔드 `npm run test:all`: 43 통과. `npm run build`: TypeScript 및 Vite 빌드 통과. 기존 500kB 초과 chunk 경고는 남아 있다.
- 독립 읽기 검토에서 direct Control presentation 공개, inspection 동의 철회, blocking stdin, 타 대화 작업 취소, executing 상태 오분류를 발견했다. 모델/외부 결과에서 presentation 제거, await 후 동의 재확인, 익명 파일 stdin, Native/External·세션별 작업 소유권, wire 변경 요청 재전송 차단 및 실제 Running/Failed 상태를 보완했다.
- CUA 합성 화면: CPU12.5%·RAM5.6/7.5GB·앱 목록·미측정 표시·부분 결과·최종 분석 및 기존 파일 카드가 함께 유지됨을 확인했다. 앱 삭제 검토를 열 때 추가 preparation0, execution0이었다. 이미 만료된 같은 계획은 확인 체크/실행 버튼이 차단됐으며 취소했다. 이는 합성 어댑터이며 실제 Codex 호출 성공의 근거가 아니다.

## 저용량 환경 회귀

전체 workspace 검사 첫 실행에서 기존 core81 및 일부 integration 검사는 통과했으나 `macos_resource_stability`가 디스크2048MiB 최소 여유 보호 때문에 실패했다. 이어 workspace lib 실행도 여유1.9GiB에서 같은 보호 규칙 때문에 core15개가 실패했다. 보호 규칙을 낮추거나 사용자 색인을 지우지 않았다.

이번 실행에서 생성한 단일 `target/debug/deps/libbloomsweepy_desktop_lib.a`(579,497,608바이트)만 생성 시각/소유 범위를 확인해 제거했다. 재생성 가능한 빌드 산출물이며 소스·설치 앱·사용자 데이터는 유지했다. 여유는2.7GiB로 회복됐고, 이후 최종 frontend 검사 후 약2.4GiB였다.

실제 최종 재실행 결과: `cargo test --workspace --lib -- --test-threads=2`는 control23/core81/desktop191/MCP12, 합계307 통과·3 ignored·실패0이다. 최초 저용량 guard 실패를 숨기지 않으며, 회복 후 lib 회귀와 전체 integration 회귀를 구분한다.

실패했던 `cargo test -p bloomsweepy-core --test macos_resource_stability -- --test-threads=1`도 회복 후1개 통과했다. 작은 합성 반복 검사에 대한 자원 상한 검증이며 실제 전체 드라이브/장시간 메모리 soak 성공으로 확대하지 않는다.

## 안전 경계 / NOT RUN

실제 사용자 파일 삭제·앱 제거·프로세스 종료·휴지통 비우기·Docker prune 및 외부 권한 자동 설정은 하지 않았다. 설치앱의 실제 Codex 새 조사 루프/Windows 실행/장시간 메모리 검증 NOT RUN. 이전 일반 파일·지도 QA의 성공은 이번 기능 전체 성공이 아니다.

## 남은 검증

모든 integration 회귀, 유효한 검토 계획의 각 기능 UI/키보드·반응형·밝은 테마, main WebView 외 최종 실행 거부 런타임, 실제 앱 조회→Codex 분석 및 설치 교체가 남았다. 전체 네이티브 번들 빌드는 최소 디스크 여유를 재확인한 뒤 수행한다. 추가 native CLI 리뷰 엔진은 NOT RUN이며 현재 독립 검토는 읽기 전용 팀 검토다.
