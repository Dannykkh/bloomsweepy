# Windows 전용 관리 구현과 Mac 회귀

2026-10-20 · Codex · 기록23:42 KST. 원래 사용자 턴 시각과 세션 ID는 제공되지 않았다. 개인 일별 대화와 별도로 이번 작업의 요청·결정·검증 근거만 기록한다.

## 사용자 요청

“윈도우까지 완료한거야? 윈도우는 윈도우에 맞는 메뉴와 속성들이 나와야할거 같은데” 이후 현재 요청: “맞아. 그렇게 구현하고 난다음에 맥을 테스트 해보자.”

## 구현 결정

Windows에서는 별도 Windows 설정 메뉴를 표시한다. 고정 Explorer4개와 AI 정책6개의 **저장값**을 조회하고8개 공식 Windows 설정 진입을 제공한다. 사용자/컴퓨터 Recall은 독립 행이다. DWORD를 실효 ON/OFF나 지원 확정으로 해석하지 않는다. PowerShell 스크립트·관리자 상승·registry 쓰기·패키지/서비스 제거·보안 보호 변경은 포함하지 않는다.

공통 `windows.inspect`는 UI와 같은 native reader를 사용하고10행·16KiB 모델 projection을 거친다. AI/MCP에는 opener/쓰기/복원 도구를 주지 않으며 기존 파일 full_access를 OS 권한으로 확장하지 않는다. 로컬 main의 고정 settings target 연결만 requested/changed:false로 응답한다. Mac 메뉴·Finder5개/저널·공통 파일/앱/성능 엔진은 유지한다.

## 작업과 증거

- 구현: [Windows 계약](../docs/plan/windows-management/implementation-contract.md).
- 실렌더에서 첫조회 오류가 '이전값'이라고 잘못 안내하는 것을 발견해 수정하고 회귀 테스트를 추가했다.
- 최종 프론트170/170, 타입검사·빌드 PASS. Windows model/native actual-source GNU target 타입검사 PASS. Mac 실제 Finder5개 읽기 smoke1/1 PASS, OS 쓰기 없음.
- 실제 App 합성 IPC로 Windows/Mac 메뉴 분리,10행/8연결, 기본 접힘, 값/오류 구별, 연결 실패·키보드,390px/1280px,Mac설정·시스템정리동선·분류 전환을 검증했다. 합성 화면은 설치형 실행의 증거가 아니다.
- 전체 Rust workspace490PASS/6ignored·exit0. 실제 Wry 부정 IPC15/15 및 거부 직후12/12 Finder5개·테스트 journal/sidecar 불변 확인. [QA 보고서](../docs/qa/windows-management/verify-report.md).
- Windows 실제 실행/installer/설정 페이지 표시, Mac 새 설치본 교체, 실제 LLM 종단 요청은 이번에 실행하지 않았다. 사용자 자료 삭제/유효 OS 설정 변경/AI 전송도 하지 않았다. 커밋·푸시·릴리스 요청은 없었다.

#tags: windows관리, 저장값조회, 고정설정연결, 맥회귀, 검증경계, arch:018
