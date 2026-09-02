import { Terminal } from "lucide-react";
import { useLanguage, type MessageKey, type Translate } from "../i18n";
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
  canEnableCleanup: boolean;
  cleanupAccessLocked: boolean;
  updatingCleanupAccess: boolean;
  cleanupAccessError: string | null;
  onToggleCleanupAccess: () => void;
  onReviewPending: () => void;
}

const operationNames: Record<string, MessageKey> = {
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

const operationStateNames: Record<ControlOperationStatus["state"], MessageKey> = {
  queued: "기다리는 중",
  running: "진행 중",
  completed: "완료",
  failed: "완료하지 못함",
  cancelled: "취소됨",
};

function operationLabel(operation: ControlOperationStatus, t: Translate): string {
  return t(operationNames[operation.kind] ?? "요청한 작업");
}

function connectionCopy(status: ControlStatus, t: Translate): { title: string; detail: string } {
  const operation = status.activeOperation;

  if (!status.bridgeAvailable) {
    return {
      title: t("로컬 연결 기능을 시작하지 못했습니다"),
      detail: t("연결 없이도 앱의 검사와 검색은 그대로 사용할 수 있습니다."),
    };
  }

  if (operation) {
    const source = operation.source === "chatCli" ? t("CLI에서 요청한") : t("앱에서 시작한");
    return {
      title: `${source} ${operationLabel(operation, t)} ${t(operationStateNames[operation.state])}`,
      detail: t("현재 상태를 확인하고 있습니다."),
    };
  }

  if (status.connectedClients > 0) {
    return {
      title: t("로컬 CLI {{count}}개 연결됨", { count: formatCount(status.connectedClients) }),
      detail: t("허용한 범위 안에서 검사와 검색 요청을 받을 수 있습니다."),
    };
  }

  if (status.lastConnectedAtUnixMs) {
    return {
      title: t("현재 연결된 로컬 CLI가 없습니다"),
      detail: t("마지막 연결 {{date}}", { date: formatDate(status.lastConnectedAtUnixMs) }),
    };
  }

  return {
    title: t("연결된 로컬 CLI가 없습니다"),
    detail: t("CLI 설치 뒤 BroomSweepy MCP 연결 도구를 별도로 등록해야 합니다."),
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
  canEnableCleanup,
  cleanupAccessLocked,
  updatingCleanupAccess,
  cleanupAccessError,
  onToggleCleanupAccess,
  onReviewPending,
}: ControlStatusPanelProps) {
  const { t } = useLanguage();
  const copy = connectionCopy(status, t);
  const operation = status.activeOperation;
  const pendingReview = status.pendingReview;
  const processedItems = operation?.processedItems ?? null;
  const processedBytes = operation?.processedBytes ?? null;
  const searchEnabled = status.searchAccess.files || status.searchAccess.documents;
  const scanEnabled = status.scanAccess.enabled;
  const cleanupEnabled = status.cleanupAccess.enabled;
  const allowedSearches = [
    status.searchAccess.files ? t("파일") : null,
    status.searchAccess.documents ? t("문서") : null,
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
          <span id="control-status-title">{t("연결 상태")}</span>
          <strong>{copy.title}</strong>
          <p>{copy.detail}</p>
        </div>

        {operation && processedItems !== null ? (
          <div className="control-status-panel__progress" aria-label={t("{{operation}} 진행 상황", { operation: operationLabel(operation, t) })}>
            <span className="control-status-panel__track is-indeterminate" aria-hidden="true">
              <span />
            </span>
            <small>
              {t("{{count}}개", { count: formatCount(processedItems) })}
              {processedBytes !== null ? ` · ${formatBytes(processedBytes)}` : ""}
            </small>
          </div>
        ) : null}

        {status.lastError ? (
          <p className="control-status-panel__error" role="alert">
            {t("로컬 CLI 요청 오류: {{detail}}", { detail: status.lastError })}
          </p>
        ) : null}
      </div>

      <div className="control-status-panel__meta">
        <span className="control-status-panel__connection">
          {status.bridgeAvailable
            ? status.connectedClients > 0
              ? t("{{count}}개 연결", { count: formatCount(status.connectedClients) })
              : t("연결 없음")
            : t("연결 꺼짐")}
        </span>
        {pendingReview ? (
          <button
            type="button"
            className="control-status-panel__review"
            title={t("확인 가능 시각 {{date}}까지", { date: formatDate(pendingReview.expiresAtUnixMs) })}
            onClick={onReviewPending}
          >
            {t("검토 열기")} · {t("{{count}}개", { count: formatCount(pendingReview.itemCount) })} · {formatBytes(pendingReview.totalBytes)}
          </button>
        ) : null}
      </div>

      <div className="control-status-panel__permissions">
        <div className="control-permission control-permission--search">
          <div>
            <strong>{t("파일·문서 검색 허용")}</strong>
            <p id="control-search-description">
              {searchEnabled
                ? t("{{targets}} 목록 검색을 이번 실행에서 허용했습니다.", { targets: allowedSearches.join("·") })
                : t("앱이 이미 만든 파일·문서 목록만 검색합니다.")}
            </p>
            <small id="control-search-help">
              {!status.bridgeAvailable
                ? t("로컬 연결 기능이 준비되면 허용할 수 있습니다.")
                : !searchEnabled && !canEnableSearch
                  ? t("빠른 파일 목록이나 문서 목록을 먼저 만들어 주세요.")
                  : t("허용해도 새 파일 검사를 시작하거나 파일을 바꾸지 않습니다.")}
            </small>
            {searchAccessError ? <small role="alert">{t("검색 허용 오류: {{detail}}", { detail: searchAccessError })}</small> : null}
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
              ? t("바꾸는 중…")
              : searchEnabled
                ? t("검색 허용 끄기")
                : t("이번 실행에서 검색 허용")}
          </button>
        </div>

        <div className="control-permission control-permission--scan">
          <div className="control-permission__scan-copy" id="control-scan-description">
            <div className="control-permission__heading">
              <strong>{t("폴더 검사 허용")}</strong>
              <span className={scanEnabled ? "is-enabled" : ""}>
                {scanEnabled ? t("이 실행에서 허용됨") : t("허용 안 됨")}
              </span>
            </div>
            <p>{t("로컬 CLI는 아래 폴더와 현재 설정으로 검사 시작만 요청합니다. 실제 파일 확인은 이 앱이 수행합니다.")}</p>
            <code dir="auto" translate="no">
              {status.scanAccess.root ?? scanRoot ?? t("먼저 검사할 폴더를 선택하세요")}
            </code>
            <small>
              {t("큰 파일 {{large}} 이상 · 중복 {{duplicate}} 이상 · 결과 {{largeCount}}/{{duplicateCount}}개", {
                large: formatBytes(scanConfig.minLargeFileBytes),
                duplicate: formatBytes(scanConfig.minDuplicateFileBytes),
                largeCount: formatCount(scanConfig.maxLargeFiles),
                duplicateCount: formatCount(scanConfig.maxDuplicateGroups),
              })}
            </small>
            <small>{t("파일을 수정하거나 이동하지 않습니다. 앱을 닫거나 폴더·설정을 바꾸면 허용이 꺼집니다.")}</small>
            {scanAccessError ? <small className="is-error" role="alert">{t("검사 허용 오류: {{detail}}", { detail: scanAccessError })}</small> : null}
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
              ? t("바꾸는 중…")
              : scanEnabled
                ? t("검사 허용 끄기")
                : t("이번 실행에서 검사 허용")}
          </button>
        </div>

        <div className="control-permission control-permission--cleanup">
          <div>
            <strong>{t("정리 계획 검토 허용")}</strong>
            <p>
              {cleanupEnabled
                ? t("외부 AI가 익명 후보 번호로 계획을 만들 수 있습니다. 실제 경로와 승인은 앱에만 표시됩니다.")
                : t("AI에는 종류와 용량 요약만 전달하고, 파일 이동은 앱에서 다시 확인합니다.")}
            </p>
            <small>
              {cleanupAccessLocked
                ? t("진행 중인 검사나 휴지통 작업이 끝난 뒤 이 권한을 바꿀 수 있습니다.")
                : canEnableCleanup || cleanupEnabled
                ? t("MCP에는 승인·실행·영구 삭제 기능이 없습니다. 앱을 닫으면 허용이 꺼집니다.")
                : t("큰 파일·중복 검사 또는 정리 후보 검사를 먼저 완료해 주세요.")}
            </small>
            {cleanupAccessError ? <small role="alert">{t("정리 검토 허용 오류: {{detail}}", { detail: cleanupAccessError })}</small> : null}
          </div>
          <button
            type="button"
            className="control-permission__scan-button"
            aria-pressed={cleanupEnabled}
            disabled={
              !status.bridgeAvailable ||
              cleanupAccessLocked ||
              updatingCleanupAccess ||
              (!cleanupEnabled && !canEnableCleanup)
            }
            onClick={onToggleCleanupAccess}
          >
            {updatingCleanupAccess
              ? t("바꾸는 중…")
              : cleanupEnabled
                ? t("정리 검토 허용 끄기")
                : t("이번 실행에서 검토 허용")}
          </button>
        </div>
      </div>
    </section>
  );
}
