import { Link2, RefreshCw, ShieldCheck } from "lucide-react";
import { useEffect, useState } from "react";
import {
  getMcpRegistrationStatuses,
  registerMcpClient,
  unregisterMcpClient,
} from "../lib/bridge";
import type {
  McpClientKind,
  McpClientRegistrationStatus,
  McpRegistrationState,
} from "../types";

const stateLabels: Record<McpRegistrationState, string> = {
  clientMissing: "CLI 없음",
  helperMissing: "앱 도구 없음",
  notRegistered: "연결 안 됨",
  registeredManaged: "연결됨",
  registeredOther: "다른 설정과 충돌",
  pathStale: "연결 복구 필요",
  checkFailed: "상태 확인 실패",
  debugBuild: "개발 빌드",
};

export function McpConnectionPanel() {
  const [statuses, setStatuses] = useState<McpClientRegistrationStatus[]>([]);
  const [loading, setLoading] = useState(true);
  const [busyClient, setBusyClient] = useState<McpClientKind | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let disposed = false;
    void getMcpRegistrationStatuses()
      .then((next) => {
        if (!disposed) setStatuses(next);
      })
      .catch((reason: unknown) => {
        if (!disposed) setError(normalizeError(reason));
      })
      .finally(() => {
        if (!disposed) setLoading(false);
      });
    return () => {
      disposed = true;
    };
  }, []);

  async function refresh() {
    if (busyClient) return;
    setLoading(true);
    setError(null);
    try {
      setStatuses(await getMcpRegistrationStatuses());
    } catch (reason) {
      setError(normalizeError(reason));
    } finally {
      setLoading(false);
    }
  }

  async function changeConnection(status: McpClientRegistrationStatus) {
    if (busyClient) return;
    const disconnect = status.canUnregister;
    const verb = disconnect ? "연결을 해제" : "연결";
    const helperPath = status.helperPath
      ? `\n\nBroomSweepy MCP 실행 파일:\n${status.helperPath}`
      : "";
    const accepted = window.confirm(
      disconnect
        ? `${status.label}의 BroomSweepy MCP ${verb}할까요?\n\n앱이 등록한 항목과 현재 설정이 정확히 같을 때만 제거합니다.${helperPath}`
        : `${status.label}에 BroomSweepy MCP를 ${verb}할까요?\n\n정리 후보는 제한된 요약과 익명 번호만 전달됩니다. 파일·문서 검색을 따로 허용하면 경로와 일치 문맥이 전달될 수 있습니다. 파일 이동은 BroomSweepy 앱의 최종 확인 없이는 실행되지 않습니다.${helperPath}`,
    );
    if (!accepted) return;

    setBusyClient(status.client);
    setError(null);
    try {
      const next = disconnect
        ? await unregisterMcpClient(status.client)
        : await registerMcpClient(status.client);
      setStatuses((current) =>
        current.map((item) => (item.client === next.client ? next : item)),
      );
    } catch (reason) {
      setError(normalizeError(reason));
    } finally {
      setBusyClient(null);
    }
  }

  return (
    <section className="settings-panel mcp-connection-panel">
      <div className="settings-panel__heading">
        <Link2 size={20} aria-hidden="true" />
        <div>
          <h2>외부 AI에 BroomSweepy 연결</h2>
          <p>Codex·Claude Code가 파일 대신 로컬 앱의 제한된 결과를 읽게 합니다.</p>
        </div>
        <button
          type="button"
          className="icon-button mcp-connection-panel__refresh"
          aria-label="MCP 연결 상태 새로 고침"
          disabled={loading || Boolean(busyClient)}
          onClick={() => void refresh()}
        >
          <RefreshCw size={16} aria-hidden="true" />
        </button>
      </div>

      <div className="mcp-connection-panel__notice">
        <ShieldCheck size={17} aria-hidden="true" />
        <p>
          요청 요약·도구 선택·판단은 외부 AI가 맡지만, 검사·검색·재검증·휴지통 이동은 이 컴퓨터의 BroomSweepy가 수행합니다. MCP에는 승인이나 영구 삭제 명령이 없습니다.
        </p>
      </div>

      {loading && statuses.length === 0 ? (
        <p className="mcp-connection-panel__empty">설치된 연결 도구를 확인하고 있습니다.</p>
      ) : (
        <div className="mcp-client-list">
          {statuses.map((status) => {
            const working = busyClient === status.client;
            return (
              <div className="mcp-client-row" key={status.client}>
                <span className="mcp-client-row__copy">
                  <span>
                    <strong>{status.label}</strong>
                    <b data-state={status.state}>{stateLabels[status.state]}</b>
                  </span>
                  <small>{status.detail}</small>
                  {status.helperPath ? (
                    <code title={status.helperPath}>{status.helperPath}</code>
                  ) : null}
                  {status.restartRequired ? (
                    <small className="is-success">연결을 쓰려면 {status.label}을 다시 시작해 주세요.</small>
                  ) : null}
                </span>
                {status.canRegister || status.canUnregister ? (
                  <button
                    type="button"
                    className={status.canUnregister ? "secondary-button" : "primary-button"}
                    disabled={working || Boolean(busyClient)}
                    onClick={() => void changeConnection(status)}
                  >
                    {working
                      ? "처리 중…"
                      : status.canUnregister
                        ? "연결 해제"
                        : status.state === "pathStale"
                          ? "연결 복구"
                          : "연결"}
                  </button>
                ) : null}
              </div>
            );
          })}
        </div>
      )}

      {error ? <p className="mcp-connection-panel__error" role="alert">{error}</p> : null}
      <p className="mcp-connection-panel__footnote">
        Claude Desktop 확장과 Claude Code 연결은 서로 다릅니다. 현재 자동 연결은 Codex와 Claude Code만 지원합니다.
      </p>
    </section>
  );
}

function normalizeError(reason: unknown): string {
  if (reason instanceof Error) return reason.message;
  if (typeof reason === "string") return reason;
  return "MCP 연결 상태를 확인하지 못했습니다.";
}
