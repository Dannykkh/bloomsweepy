import { Database, HardDrive, ShieldCheck } from "lucide-react";
import { DockerManagementPanel } from "../components/DockerManagementPanel";
import { McpConnectionPanel } from "../components/McpConnectionPanel";
import type { DockerManagementStatus, ScanConfig } from "../types";

interface SettingsViewProps {
  config: ScanConfig;
  dockerStatus: DockerManagementStatus | null;
  dockerLoading: boolean;
  dockerChanging: boolean;
  dockerError: string | null;
  onConfigChange: (config: ScanConfig) => void;
  onDockerEnabledChange: (enabled: boolean) => Promise<void>;
  onOpenDocker: () => void;
}

const megabyte = 1024 * 1024;

export function SettingsView({
  config,
  dockerStatus,
  dockerLoading,
  dockerChanging,
  dockerError,
  onConfigChange,
  onDockerEnabledChange,
  onOpenDocker,
}: SettingsViewProps) {
  function update<Key extends keyof ScanConfig>(key: Key, value: ScanConfig[Key]) {
    onConfigChange({ ...config, [key]: value });
  }

  return (
    <div className="settings-layout">
      <section className="settings-panel">
        <div className="settings-panel__heading">
          <HardDrive size={20} aria-hidden="true" />
          <div>
            <h2>파일 크기 기준</h2>
            <p>다음 스캔부터 적용됩니다.</p>
          </div>
        </div>

        <label className="setting-row">
          <span>
            <strong>큰 파일 최소 크기</strong>
            <small>결과 목록에 보여줄 최소 파일 크기</small>
          </span>
          <select
            value={config.minLargeFileBytes}
            onChange={(event) => update("minLargeFileBytes", Number(event.currentTarget.value))}
          >
            <option value={10 * megabyte}>10 MB</option>
            <option value={50 * megabyte}>50 MB</option>
            <option value={100 * megabyte}>100 MB</option>
            <option value={500 * megabyte}>500 MB</option>
            <option value={1024 * megabyte}>1 GB</option>
          </select>
        </label>

        <label className="setting-row">
          <span>
            <strong>중복 검사 최소 크기</strong>
            <small>작은 파일을 비교하는 작업량을 줄입니다</small>
          </span>
          <select
            value={config.minDuplicateFileBytes}
            onChange={(event) => update("minDuplicateFileBytes", Number(event.currentTarget.value))}
          >
            <option value={100 * 1024}>100 KB</option>
            <option value={megabyte}>1 MB</option>
            <option value={10 * megabyte}>10 MB</option>
            <option value={50 * megabyte}>50 MB</option>
          </select>
        </label>
      </section>

      <section className="settings-panel">
        <div className="settings-panel__heading">
          <Database size={20} aria-hidden="true" />
          <div>
            <h2>결과 한도</h2>
            <p>메모리 사용량과 화면 밀도를 제어합니다.</p>
          </div>
        </div>

        <label className="setting-row">
          <span>
            <strong>큰 파일 결과</strong>
            <small>크기순으로 보관할 최대 항목 수</small>
          </span>
          <select
            value={config.maxLargeFiles}
            onChange={(event) => update("maxLargeFiles", Number(event.currentTarget.value))}
          >
            <option value={100}>100개</option>
            <option value={250}>250개</option>
            <option value={500}>500개</option>
          </select>
        </label>

        <label className="setting-row">
          <span>
            <strong>중복 그룹 결과</strong>
            <small>낭비 용량순으로 보관할 최대 그룹 수</small>
          </span>
          <select
            value={config.maxDuplicateGroups}
            onChange={(event) => update("maxDuplicateGroups", Number(event.currentTarget.value))}
          >
            <option value={50}>50개</option>
            <option value={100}>100개</option>
            <option value={250}>250개</option>
          </select>
        </label>
      </section>

      <DockerManagementPanel
        status={dockerStatus}
        loading={dockerLoading}
        changing={dockerChanging}
        error={dockerError}
        onEnabledChange={onDockerEnabledChange}
        onOpenDocker={onOpenDocker}
      />

      <McpConnectionPanel />

      <section className="safety-contract">
        <ShieldCheck size={22} aria-hidden="true" />
        <div>
          <h2>현재 안전 계약</h2>
          <ul>
            <li>스캔은 파일을 수정하거나 이동하지 않습니다.</li>
            <li>같은 저장공간을 가리키는 여러 파일 이름은 중복 낭비로 세지 않습니다.</li>
            <li>일부 내용으로 후보를 줄인 뒤 전체 내용을 끝까지 비교해 중복을 확정합니다.</li>
            <li>선택 항목은 실행 직전 재검증하고 운영체제 휴지통으로만 이동합니다.</li>
            <li>일반 파일은 영구 삭제하지 않으며 Windows 설치 정보도 변경하지 않습니다.</li>
            <li>Docker 정리는 예외적으로 휴지통을 거치지 않아 별도 확인 뒤에만 실행합니다.</li>
          </ul>
        </div>
      </section>
    </div>
  );
}
