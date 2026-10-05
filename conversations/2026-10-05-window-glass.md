# 타이틀바와 글래스 재질 복원

date: 2026-10-05
source: codex
세션 UUID·사용자 턴의 정확한 시각은 미제공. 이 파일은 이번 UI 변경의 정제 기록이며 원본 세션 전체가 아니다.

## 현재 요청

사용자: “아 맞다. 이 프로그램 타이틀바가 사라졌던데? 글래스모피즘도 약하고?”

읽기 전용 확인 뒤 사용자: “진행하자”. 승인 범위는 기존 Rust 앱의 네이티브 제목줄·글래스 재질 개선과 이 맥의 설치본 검증이다. 이전 작업의 커밋/푸시 승인을 이번 변경에 자동 적용하지 않는다.

## 관찰과 선택

`hiddenTitle: true`와 중첩된 어두운 body/shell/sidebar가 제목과 네이티브 재질을 가렸다. 첫 설치에서 Transparent 제목줄은 안정적인 제목 대비를 확인하지 못했다. 최종적으로 OS Visible 제목줄을 사용하고, 내용 영역에는 한 번의 24–44% wash와 28% sidebar를 사용했다. 웹·Windows는 opaque canvas를 유지하고 네이티브 sidebar/hero의 중복 CSS blur만 제거했다. Swift golden master와 기존 기능·배치는 변경하지 않았다.

## 실제 확인

2026-10-05 08:54 KST까지 최종 설치형 제목/버전/창 버튼, 대시보드·CPU/RAM·AI 화면, 기존 합성 대화 보존 및 close-to-hide/다시 열기를 확인했다. TypeScript, 프런트51개 테스트, 최종 config2개, 생산 빌드, ARM64 앱 번들 및 로컬 ad-hoc 서명 검증 통과. 브라우저 compact/narrow·한글 폰트·키보드 focus·오류/로딩 상태 확인. Windows 실행·장시간 자원·네이티브 compact resize·모든 배경 대비는 미실시다. 드래그/최소화 입력은 실행했지만 창 전역 좌표/최소화 플래그는 독립 계측하지 못했다.

근거: [구현·렌더 기록](../docs/design-refs/2026-10-05-impl-log-window-glass.md), [architecture007](../memory/architecture/007-native-window-material.md). UI 구현·설치 단계에서는 외부 LLM 요청·파일 삭제·설정 변경·커밋/푸시·릴리스 교체를 하지 않았다.

## 후속 커밋·푸시 요청

사용자: “커밋 푸시하자.” 이번 UI 수정과 관련 DESIGN/경험/검증·인계·메모리·대화 기록의 커밋 및 main 푸시를 승인했다. 기존 사용자 `demo-assets/`, 무시된 화면 캡처·빌드/설치 파일은 제외한다. 버전 변경·태그·GitHub Release·공개 설치 파일 교체는 이번 요청에 포함하지 않는다. 기존 검증 뒤 제품 코드 변경이 없으므로 전체 빌드/테스트를 반복하지 않고 staged 범위·공백·정제 기록·원격 HEAD를 확인한다.

#tags: window-chrome native-glass macos-install render-qa arch:007
