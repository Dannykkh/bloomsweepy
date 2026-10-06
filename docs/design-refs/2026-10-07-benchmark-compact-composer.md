# Benchmark: user-supplied composer crops

- Mode: 영감 각색. 사용자 첨부, 2026-10-07 제공(원본 촬영 시각 미제공).
- Current: /var/folders/qs/25yy4t_52rlf66ksv05dnh5r0000gn/T/codex-clipboard-630825d9-379b-49c6-b47b-cc6536a13f43.png,1700×512 crop.
- Reference: /var/folders/qs/25yy4t_52rlf66ksv05dnh5r0000gn/T/codex-clipboard-c712913a-64bf-4001-99cd-a05c5f862981.png,610×352 crop.
- Scope: 입력부·모델/추론 선택의 구조/위계. 코드·브랜드·자산을 복사하지 않는다.

## Observation sequence

현재: 입력창 → 큰 모델/추론 select 두 개 → 상시 설명 두 줄 → 전송 안내.
참조: 입력창 하단 작은 추론 선택 버튼/음성/원형 위 화살표 → 위에 펼쳐진 라운드 팝업.
팝업: 번개/현재 강도(Ultra)/초기화 → 모델 이름+chevron → 넓은 purple track/큰 thumb.
현재값이 중심이고 모델/설명은 보조다. 별도의 저장 버튼 없이 조작 의미가 읽힌다.
스크린샷에는 전체 페이지·모바일·실제 상태 전환·키보드·모션 증거가 없으므로 추정하지 않는다.

## Adopt / Adapt / Avoid

- Adopt: 작은 toolbar trigger, 위쪽 팝업, 강도 중심 헤더, 모델 보조 행, 초기화, 원형 전송.
- Adapt: 실제 supportedEfforts만 정수 단계로 조절; 프로젝트 Pretendard/neutral glass/브랜드 violet 사용.
  CLI 기본값은 explicit none과 구분하며 재설정은 빈 override다. 사용자 후속 요청대로
  상시 설명과 정상 상태의 팝업 설명은 제거하며 오류/미지원/복구 안내만 필요한 상태에서 표시한다.
- Avoid: 미구현 마이크, 임의 모델/Ultra 고정, 상시 움직이는 파티클, OpenAI 로고, 가짜 처리율.

## Responsive and states

모바일 reference evidence unavailable(첨부는 crop). 구현은 모델 라벨 compress, 정상 설명 remove,
입력/전송 retain, 팝업을 컨테이너 폭으로 clamp하고 위로 연다. 미지원/목록 없음/stale는
이유와 기본값 복원·CLI 재확인을 제공하며 busy에는 변경을 잠근다.

Gate A–D: visible crop 전체 순서·누락 증거·제품 변환 기록 완료.
Gate E–F: compact fixture의 키보드·busy·stale·760×600/390px 렌더를 메인이 확인했다.
후속 공급자 통합112frontend/269native 검사·ARM bundle·실제 설치 UI 검증은 완료했다.
Grok/Agy 실제 CLI·Claude 실제 생성·Windows·장시간 검증은 별도 NOT RUN이며 합성 관찰을 소급 적용하지 않는다.
세부 상태는 [다중 공급자 QA 기록](../qa/2026-10-07-multi-provider-model-selection.md)을 따른다.

## Provider adaptation boundary

Codex의 시각적 위계만 참고한다. Claude/Grok/Agy에도 같은 선택 UI를 사용하지만 모델 목록과
추론 단계는 각 CLI의 계약으로 결정한다. Codex의 Ultra나 모델명을 다른 공급자에 복제하지 않는다.
Settings는 기존 Picker를 유지하고 채팅과 같은 공급자·모델별 선호를 사용한다.

#tags: codex-reference, compact-composer, adopt-adapt-avoid, ui, multi-provider
