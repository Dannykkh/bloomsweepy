import { Boxes, LoaderCircle } from "lucide-react";
import { useEffect, useState } from "react";
import type { DockerManagementStatus } from "../types";

interface DockerManagementPanelProps {
  status: DockerManagementStatus | null;
  loading: boolean;
  changing: boolean;
  error: string | null;
  onEnabledChange: (enabled: boolean) => Promise<void>;
  onOpenDocker: () => void;
}

export function DockerManagementPanel({
  status,
  loading,
  changing,
  error,
  onEnabledChange,
  onOpenDocker,
}: DockerManagementPanelProps) {
  const [showWorking, setShowWorking] = useState(false);

  useEffect(() => {
    if (!loading && !changing) {
      setShowWorking(false);
      return;
    }
    const timer = window.setTimeout(() => setShowWorking(true), 300);
    return () => window.clearTimeout(timer);
  }, [changing, loading]);

  return (
    <section className="settings-panel developer-tools-panel">
      <div className="settings-panel__heading">
        <Boxes size={20} aria-hidden="true" />
        <div>
          <h2>개발 도구 관리</h2>
          <p>Docker를 사용하는 경우에만 켜세요.</p>
        </div>
      </div>

      <label className="setting-row developer-tool-toggle">
        <span>
          <strong>Docker 용량 관리</strong>
          <small>
            켜면 사이드바에 Docker 전용 메뉴가 나타납니다. 대시보드에는 표시하지 않습니다.
          </small>
        </span>
        <span className="developer-tool-toggle__control">
          <input
            type="checkbox"
            name="dockerManagementEnabled"
            role="switch"
            aria-label="Docker 용량 관리 사용"
            checked={status?.enabled ?? false}
            disabled={loading || changing || status?.busy}
            onChange={(event) => void onEnabledChange(event.currentTarget.checked)}
          />
          <strong>{status?.enabled ? "사용" : "사용 안 함"}</strong>
        </span>
      </label>

      {showWorking ? (
        <div className="docker-tool-state" role="status">
          <LoaderCircle className="is-spinning" size={18} aria-hidden="true" />
          <span>{changing ? "Docker 설정 적용 중…" : "Docker 설정 확인 중…"}</span>
        </div>
      ) : null}

      {!loading && status?.enabled ? (
        <div className="docker-settings-link-row">
          <span>
            <strong>Docker 용량 메뉴가 켜졌습니다</strong>
            <small>상태 확인과 정리 검토는 전용 화면에서 진행합니다.</small>
          </span>
          <button className="secondary-button" type="button" onClick={onOpenDocker}>
            Docker 용량 열기
          </button>
        </div>
      ) : null}

      {error ? <p className="docker-tool-error" role="alert">{error}</p> : null}
    </section>
  );
}
