# 설치된 대화 도구의 일반 파일 관리 확장

- date: 2026-10-04
- source: codex
- session: 01a06a3f-f2f6-72c0-9c0c-78a13e23b651
- 최초 사용자 턴 시각은 미확인. 아래 앱 검증 시각은 실제 AX 상태로 확인한 KST.

## 현재 사용자 요청

“지금 실행된 프로그램의 대화내역을 확인해볼래? 뭘 못한다고만 하네, 삭제를 못한다며?”에 이어 “그니까 못하는거잖아. 파일관리를 지금. 다 잘할수 있게 해줘.”

수정 전 설치앱은 `promo-video`에 파일14,914개가 있으므로 빈 폴더 도구로 처리할 수 없다고 답했다. 기존 일반 파일/폴더 OS Trash 검증기는 있었지만 내장 AI 프로토콜과 연결되지 않았다.

## 15:31 KST — 첫 설치 검증

일반 도구를 연결한 앱에서 실제 Codex에 `promo-video` 최종 검토만 요청했다. 로컬 검색 및 후보 선택은 됐으나 `node_modules/.../.bin/semver` 심볼릭 링크를 기존 폴더 검증기가 거부했다. 사용자 폴더는 이동하지 않았다.

## 15:39 KST — 최종 설치 검증

POSIX 폴더 내부 링크를 원본과 분리해 fingerprint하고 원본을 탐색하지 않도록 보완했다. 재빌드/설치 후 같은 기존 대화에서 최종 검토 요청이 성공했다. 실제 카드에는 논리1.2 GB, 파일14,914개, 폴더1,296개, 링크29개를 표시했다. 체크 전 최종 버튼은 잠겨 있으며 `promo-video`는 이동하지 않았다.

Rust273/frontend43 자동 회귀, 실제 Codex2요청 계약, 직접 만든 16 B 파일+폴더의 네이티브 Trash 시험 통과. 링크 보완 후 native 시험은 비선택 링크 원본 보존도 확인했다. 설치앱 요청→검토와 native 시험의 실행을 분리해 보고한다. Windows 및 장시간 검증은 남아 있고, 커밋/푸시/릴리스는 하지 않았다.

근거: [QA](../docs/qa/2026-10-04-conversational-files.md), [작업 인계](../docs/handoffs/2026-10-04-120000-conversational-file-management.md).

#tags: conversational-files, folder-trash, codex, macos, app-owned-tools, arch:001, arch:002, supersedes:#empty-folder-tools, supersedes:#nested-link-rejection
