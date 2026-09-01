import { Terminal } from "lucide-react";
import { formatBytes, formatCount, formatDate } from "../lib/format";
import type { ControlOperationStatus, ControlStatus, ScanConfig } from "../types";

interface ControlStatusPanelProps {
  status: ControlStatus;
  canEnableSearch: boolean;
  updatingSearchAccess: boolean;
  searchAccessError: string | null;
  onToggleSearchAccess: () => void;
  scanRoot: string | null;
  scanConfig: ScanConfig;
  canEnableScan: boolean;
  updatingScanAccess: boolean;
  scanAccessError: string | null;
  onToggleScanAccess: () => void;
}

const operationNames: Record<string, string> = {
  storageScan: "파일 검사",
  scan: "파일 검사",
  fileScan: "파일 검사",
  file_scan: "파일 검사",
  driveScan: "드라이브 검사",
  drive_scan: "드라이브 검사",
  directoryScan: "폴더 분석",
  directory_scan: "폴더 분석",
  cleanupScan: "정리 후보 검사",
  cleanup_scan: "정리 후보 검사",
  documentIndex: "문서 읽기",
  document_index: "문서 읽기",
  documentSearch: "문서 검색",
  document_search: "문서 검색",
  fileCatalog: "빠른 파일 목록 만들기",
  file_catalog: "빠른 파일 목록 만들기",
  fileSearch: "파일 찾기",
  file_search: "파일 찾기",
  trashReview: "휴지통 이동 확인",
  trash_review: "휴지통 이동 확인",
};

const operationStateNames: Record<ControlOperationStatus["state"], string> = {
  queued: "기다리는 중",
  running: "진행 중",
  completed: "완료",
  failed: "완료하지 못함",
  cancelled: "취소됨",
};

function operationLabel(operation: ControlOperationStatus): string {
  return operationNames[operation.kind] ?? "요청한 작업";
}

function connectionCopy(status: ControlStatus): { title: string; detail: string } {
  const operation = status.activeOperation;

  if (!status.bridgeAvailable) {
    return {
      title: "로컬 연결 기능을 시작하지 못했습니다",
      detail: "연결 없이도 앱의 검사와 검색은 그대로 사용할 수 있습니다.",
    };
  }

  if (operation) {
    const source = operation.source === "chatCli" ? "CLI에서 요청한" : "앱에서 시작한";
    return {
      title: `${source} ${operationLabel(operation)} ${operationStateNames[operation.state]}`,
      detail: operation.message ?? "현재 상태를 확인하고 있습니다.",
    };
  }

  if (status.connectedClients > 0) {
    return {
      title: `로컬 CLI ${formatCount(status.connectedClients)}개 연결됨`,
      detail: "허용한 범위 안에서 검사와 검색 요청을 받을 수 있습니다.",
    };
  }

  if (status.lastConnectedAtUnixMs) {
    return {
      title: "현재 연결된 로컬 CLI가 없습니다",
      detail: `마지막 연결 ${formatDate(status.lastConnectedAtUnixMs)}`,
    };
  }

  return {
    title: "연결된 로컬 CLI가 없습니다",
    detail: "CLI 설치 뒤 BroomSweepy MCP 연결 도구를 별도로 등록해야 합니다.",
  };
}

export function ControlStatusPanel({
  status,
  canEnableSearch,
  updatingSearchAccess,
  searchAccessError,
  onToggleSearchAccess,
  scanRoot,
  scanConfig,
  canEnableScan,
  updatingScanAccess,
  scanAccessError,
  onToggleScanAccess,
}: ControlStatusPanelProps) {
  const copy = connectionCopy(status);
  const operation = status.activeOperation;
  const pendingReview = status.pendingReview;
  const processedItems = operation?.processedItems ?? null;
  const processedBytes = operation?.processedBytes ?? null;
  const searchEnabled = status.searchAccess.files || status.searchAccess.documents;
  const scanEnabled = status.scanAccess.enabled;
  const allowedSearches = [
    status.searchAccess.files ? "파일" : null,
    status.searchAccess.documents ? "문서" : null,
  ].filter(Boolean);
  const scanBusy = operation?.kind === "storageScan" && operation.state === "running";

  return (
    <section
      className={`control-status-panel ${status.bridgeAvailable ? "is-available" : "is-unavailable"}`}
      aria-labelledby="control-status-title"
    >
      <span className="control-status-panel__icon" aria-hidden="true">
        <Terminal size={17} />
        <span className="control-status-panel__dot" />
      </span>

      <div className="control-status-panel__body">
        <div className="control-status-panel__copy">
          <span id="control-status-title">연결 상태</span>
          <strong>{copy.title}</strong>
          <p>{copy.detail}</p>
        </div>

        {operation && processedItems !== null ? (
          <div className="control-status-panel__progress" aria-label={`${operationLabel(operation)} 진행 상황`}>
            <span className="control-status-panel__track is-indeterminate" aria-hidden="true">
              <span />
            </span>
            <small>
              {formatCount(processedItems)}개
              {processedBytes !== null ? ` · ${formatBytes(processedBytes)}` : ""}
            </small>
          </div>
        ) : null}

        {status.lastError ? (
          <p className="control-status-panel__error" role="alert">
            로컬 CLI 요청 오류: {status.lastError}
          </p>
        ) : null}
      </div>

      <div className="control-status-panel__meta">
        <span className="control-status-panel__connection">
          {status.bridgeAvailable
            ? status.connectedClients > 0
              ? `${formatCount(status.connectedClients)}개 연결`
              : "연결 없음"
            : "연결 꺼짐"}
        </span>
        {pendingReview ? (
          <span
            className="control-status-panel__review"
            title={`확인 가능 시각 ${formatDate(pendingReview.expiresAtUnixMs)}까지`}
          >
            앱에서 최종 확인 대기 · {formatCount(pendingReview.itemCount)}개 · {formatBytes(pendingReview.totalBytes)}
          </span>
        ) : null}
      </div>

      <div className="control-status-panel__permissions">
        <div className="control-permission control-permission--search">
          <div>
            <strong>파일·문서 검색 허용</strong>
            <p id="control-search-description">
              {searchEnabled
                ? `${allowedSearches.join("·")} 목록 검색을 이번 실행에서 허용했습니다.`
                : "앱이 이미 만든 파일·문서 목록만 검색합니다."}
            </p>
            <small id="control-search-help">
              {!status.bridgeAvailable
                ? "로컬 연결 기능이 준비되면 허용할 수 있습니다."
                : !searchEnabled && !canEnableSearch
                  ? "빠른 파일 목록이나 문서 목록을 먼저 만들어 주세요."
                  : "허용해도 새 파일 검사를 시작하거나 파일을 바꾸지 않습니다."}
            </small>
            {searchAccessError ? <small role="alert">검색 허용 오류: {searchAccessError}</small> : null}
          </div>
          <button
            type="button"
            className="control-status-panel__access-button"
            aria-pressed={searchEnabled}
            aria-describedby="control-search-description control-search-help"
            disabled={
              !status.bridgeAvailable ||
              updatingSearchAccess ||
              (!searchEnabled && !canEnableSearch)
            }
            onClick={onToggleSearchAccess}
          >
            {updatingSearchAccess
              ? "바꾸는 중…"
              : searchEnabled
                ? "검색 허용 끄기"
                : "이번 실행에서 검색 허용"}
          </button>
        </div>

        <div className="control-permission control-permission--scan">
          <div className="control-permission__scan-copy" id="control-scan-description">
            <div className="control-permission__heading">
              <strong>폴더 검사 허용</strong>
              <span className={scanEnabled ? "is-enabled" : ""}>
                {scanEnabled ? "이 실행에서 허용됨" : "허용 안 됨"}
              </span>
            </div>
            <p>로컬 CLI는 아래 폴더와 현재 설정으로 검사 시작만 요청합니다. 실제 파일 확인은 이 앱이 수행합니다.</p>
            <code dir="auto" translate="no">
              {status.scanAccess.root ?? scanRoot ?? "먼저 검사할 폴더를 선택하세요"}
            </code>
            <small>
              큰 파일 {formatBytes(scanConfig.minLargeFileBytes)} 이상 · 중복 {formatBytes(scanConfig.minDuplicateFileBytes)} 이상 · 결과 {formatCount(scanConfig.maxLargeFiles)}/{formatCount(scanConfig.maxDuplicateGroups)}개
            </small>
            <small>파일을 수정하거나 이동하지 않습니다. 앱을 닫거나 폴더·설정을 바꾸면 허용이 꺼집니다.</small>
            {scanAccessError ? <small className="is-error" role="alert">검사 허용 오류: {scanAccessError}</small> : null}
          </div>
          <button
            type="button"
            className="control-permission__scan-button"
            aria-pressed={scanEnabled}
            aria-describedby="control-scan-description"
            disabled={
              !status.bridgeAvailable ||
              updatingScanAccess ||
              scanBusy ||
              (!scanEnabled && !canEnableScan)
            }
            onClick={onToggleScanAccess}
          >
            {updatingScanAccess
              ? "바꾸는 중…"
              : scanEnabled
                ? "검사 허용 끄기"
                : "이번 실행에서 검사 허용"}
          </button>
        </div>
      </div>
    </section>
  );
}
