# 채팅 권한·전체 로컬 접근 QA

2026-10-11 / macOS ARM / 개발 worktree. 공개v1.7.0·설치본 검증과 구분한다.
실제 사용자 파일 작업, 권한 자동 활성화, AI 전송, 설치/배포는 하지 않았다.

## 프런트엔드

- `npm run test:all`:126 PASS/0 fail.
- `npm run check`:PASS.
- `npm run build`:PASS. 최종 단일 JS chunk 약1,008.71kB 경고는 남음; bundle 분할은 이번 범위 밖.
- 문구 최종 수정 뒤 `npm run test:i18n`:4 PASS/0 fail.
- Experience Contract validator:PASS, 오류/경고0.

## 실제 브라우저·합성 데이터

`assistant-tools-fixture.html?chat-access=confirm&multi-provider`를 사용했다. 디스크·CLI 실행은 mock이다.

| 확인 | 결과 |
|---|---|
|1280×900 다크 popup/44px 버튼/font|PASS, page scrollWidth1280|
|390×844 toolbar/popup|PASS, page scrollWidth380≤390, 전송·권한44px|
|KO/EN/JA/ZH-CN 좁은 화면|PASS, 번역/글리프/팝업 화면 경계 확인. 긴 영어 설명은 내부 스크롤|
|End→Enter/Escape|PASS, 이동만으로 권한 변경하지 않음; 닫으면 trigger로 포커스 복귀|
|저장 지연|PASS,900ms 응답 전 confirm 라벨/잠금, ACK 후 full_access|
|저장 실패|PASS, confirm 유지+오류 표시, 전체 조회 버튼 없음|
|드라이브→전체 이름 검색→대상 열기|PASS, accessKind search의 entries 표시. 같은 payload의 volumes와 혼동하지 않음|
|전체 접근→조회만 축소|PASS, 이전 카드/검토 제거, 새 파일 검사는 가능하고 선택/최종 검토 차단|
|조회 전용 결과 안내|PASS, 검사 완료와 실행 준비 금지를 설명하며 삭제 선택을 권하지 않음|
|설정/연결 popup 상태 공유|PASS, composer와 동일 full_access, 중복 old flag checkbox 없음|
|브라우저 오류|문구 HMR 중 LanguageProvider 일시 오류 관찰; full reload 후 오류/경고 없음|
|라이트|N/A, 현재 앱 미지원|
|reduced-motion|CSS animation/active-transform 해제 분기 확인; 실제 OS 설정 NOT RUN|

화면: [desktop](screenshots/2026-10-11-chat-access-desktop-dark.jpg),
[mobile](screenshots/2026-10-11-chat-access-mobile-popup-dark.jpg),
[저장 실패](screenshots/2026-10-11-chat-access-save-error.jpg),
[EN](screenshots/2026-10-11-chat-access-mobile-en.jpg),
[JA](screenshots/2026-10-11-chat-access-mobile-ja.jpg),
[ZH](screenshots/2026-10-11-chat-access-mobile-zh.jpg).

## Rust 및 독립 검토

Rust 실행 검증: desktop lib286 PASS/4 opt-in ignored, core108 PASS, control29 PASS.
최후 plan ID 대소문자 binding 보강 뒤 desktop assistant_access focused2 PASS 재컴파일.
`CARGO_PROFILE_TEST_DEBUG=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_INCREMENTAL=0 CARGO_BUILD_JOBS=1`로
한 번에 하나씩 실행했다. 기본 debug 첫 컴파일은 저공간 보호를 위해 중단 후 기존 저용량 캐시를
재사용했다. rustfmt·git diff check PASS. 최종 디스크 여유3.6GiB.
독립 소스 리뷰는 cleanup 실행의 native 취소 연결 누락 및 provider 최종 응답 epoch 재검사
누락을 발견했고 수정 소스를 재검토했다. 부분/동명이인 자동 이동 차단, external 네 작업
Unsupported, JS/Rust mode/action 계약 일치를 확인했다. 소스 리뷰를 실제 실행 성공으로 세지 않는다.

## NOT RUN

새 설치본 교체/완전 재시작 저장 복원, 실제 CLI 종단, 사용자 파일 휴지통 이동, Windows 런타임,
네트워크/USB 실장치, 실제 native IPC의 동시 mode 변경 경쟁상태, 실제 전체 볼륨 검색,
장시간 RAM/디스크 soak, Product Design 실행 어댑터 비교는 실행하지 않았다.
전체 검색 시간 제한은 파일시스템 호출 사이의 협력 취소이며30초 강제 종료 보장이 아니다.
이미 완료된 OS 작업을 mode 변경으로 되돌리지 않는다. 설치본·실장치 안전성을 단위 테스트만으로 보장하지 않는다.

#tags: qa, chat-access, native-permission, bounded-search, arch:015

## 11:22 KST 설치 후속

위 결과와 NOT RUN은10:49 개발 스냅샷이다. 이후 사용자 “설치하자” 요청으로 이 Mac의
ARM64 개발본 설치·현재 권한/대화 재시작 복원·native popup 표시를 확인했다.
설치 smoke와 발견/수정한 WebKit dock stacking 회귀는 [별도 QA](2026-10-11-local-install.md)에 기록한다.
새 권한 opt-in·실제 파일 작업·CLI 종단·Windows 등은 여전히 NOT RUN이다.
