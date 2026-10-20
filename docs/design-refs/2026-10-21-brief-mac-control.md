# Mac 설정: 설명보다 조작이 먼저

2026-10-21 · source: codex · 단계: 시안 선택 전, 제품 코드 미변경.

사용자 피드백: 맥스럽지 않고 AI 생성 화면처럼 보이며 글이 빡빡하다. 기능·특징이 시각적으로 구분돼야 하고, 찾기 어려운 버튼을 눌렀더니 단순 메인 메뉴 연결인 경험은 없어야 한다.

주 과업은 Finder 설정을 찾아 현재 저장 상태를 이해하고, 선택→명시 적용→변경 기록에서 복원하는 것이다. 이번 시안은 실제 구현된5개만 사용한다. 미구현 Apple Intelligence/Dock/개인정보 기능을 지원하는 것처럼 채우지 않는다. 디자인 선택 전에 OS 변경/설치/권한/채팅·Windows 기능을 수정하지 않는다.

사용자 선택이 필요한 부분은 화면 구성이다. A: compact grouped controls, B: feature-first tiles, C: category/detail inspector를 같은5개·동일 null 상태로 렌더한다. 이 세 가지는 실제 제품이 아니라 OS 연결 없는 로컬 미리보기다. Mac 원복·실효값 미확인·명시적 적용 계약은 공통이다.

원본 근거: 사용자 첨부 이미지와 이번 audit의 [설치 첫 화면](../qa/mac-design-review-2026-10-21/01-installed-before.png), [하단 연결](../qa/mac-design-review-2026-10-21/02-installed-links.png). 전자는 중복 제목/안내가 첫 화면을 차지하고, 후자는 보조 이동 버튼이 기능처럼 보이며 스크롤 시 상위 제목이 내용과 겹친다.

선택 후 제품 구현은 MacManagementView의 기존 비동기 lock/stale/managed/null/expected-value/복원 계약을 재사용해야 한다. Mac 설정 이름으로 다른 메뉴 연결을 핵심 기능처럼 강조하지 않는다.

## Product Design Gate

UNKNOWN: 세션에는 product-design0.1.56 스킬이 노출돼 있지만 CLI marketplace 목록은 openai-primary-runtime/openai-bundled/openai-curated이며 installed/available JSON 조회가 유효한 selector 응답을 주지 않았다. 설치 부재로 단정하지 않는다. 새 설치·추천 없이 기존 React/CSS/Lucide 로컬 어댑터를 사용한다. 원격 Sites 배포/새 템플릿 초기화 없음. 최초 어댑터 유무 대조는 NOT RUN — 원격 renderer 없음.

## Module Coverage

- design-plan: /Users/dannysmacair/.codex/skills/design-plan/SKILL.md; experience/render/layout 기준 읽음.
- frontend-design: /Users/dannysmacair/.codex/.olympus/source-skills/frontend-design/SKILL.md; 기능형 playbook·layout anatomy 읽음. 기존 Pretendard/Lucide 재사용, 레시피 라이브러리 없음.
- ui-ux-auditor: /Users/dannysmacair/.codex/.olympus/source-skills/ui-ux-auditor/SKILL.md; 제한 UI 관찰만, 전체9영역 점수·제품 자동수정은 NOT RUN.
- web-design-guidelines: /Users/dannysmacair/.codex/.olympus/source-skills/web-design-guidelines/SKILL.md; 선택 전 preview 정합성 기준만. 제품 전체 감사 아님.
- mermaid-diagrams: NOT APPLICABLE — 단일 설정 화면의 시안3개이며 새 다중 페이지/라우트 없음.
- design-system-starter/Stitch/skill-evolve/autoresearch: NOT REQUESTED.
- 대표 모션: NOT APPLICABLE — 장식0, 버튼 상태 전환만; CSS/reduced-motion 사용.

#tags: mac-design, controls-first, explicit-actions, design-preview
