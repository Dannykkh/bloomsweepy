import { Database, HardDrive, Languages, ShieldCheck } from "lucide-react";
import { DockerManagementPanel } from "../components/DockerManagementPanel";
import { McpConnectionPanel } from "../components/McpConnectionPanel";
import { useLanguage, type LanguagePreference } from "../i18n";
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
  const { language, preference, setPreference, storageError, t } = useLanguage();

  function update<Key extends keyof ScanConfig>(key: Key, value: ScanConfig[Key]) {
    onConfigChange({ ...config, [key]: value });
  }

  return (
    <div className="settings-layout">
      <section className="settings-panel settings-language-panel">
        <div className="settings-panel__heading">
          <Languages size={20} aria-hidden="true" />
          <div>
            <h2>{t("표시 언어")}</h2>
            <p>{t("이 앱의 메뉴와 설명에 사용할 언어입니다.")}</p>
          </div>
        </div>

        <label className="setting-row setting-row--language">
          <span>
            <strong>{t("언어")}</strong>
            <small id="language-setting-description">
              {t("이 앱의 표시만 바뀌며 파일과 운영체제 설정은 바뀌지 않습니다.")}
            </small>
            <small className="setting-row__status">
              {t("현재 적용: {{language}}", {
                language:
                  language === "ko"
                    ? "한국어"
                    : language === "ja"
                      ? "日本語"
                      : language === "zh-CN"
                        ? "简体中文"
                        : "English",
              })}
            </small>
          </span>
          <select
            value={preference}
            aria-describedby="language-setting-description"
            onChange={(event) =>
              setPreference(event.currentTarget.value as LanguagePreference)
            }
          >
            <option value="en">English</option>
            <option value="ko">한국어</option>
            <option value="ja">日本語</option>
            <option value="zh-CN">简体中文</option>
          </select>
        </label>
        <p className="settings-language-panel__storage-note">
          {storageError
            ? t("언어 선택을 저장하지 못했습니다. 현재 실행 중에는 선택한 언어를 사용합니다.")
            : t("언어 선택은 이 컴퓨터에만 저장됩니다.")}
        </p>
      </section>

      <section className="settings-panel">
        <div className="settings-panel__heading">
          <HardDrive size={20} aria-hidden="true" />
          <div>
            <h2>{t("파일 크기 기준")}</h2>
            <p>{t("다음 스캔부터 적용됩니다.")}</p>
          </div>
        </div>

        <label className="setting-row">
          <span>
            <strong>{t("큰 파일 최소 크기")}</strong>
            <small>{t("결과 목록에 보여줄 최소 파일 크기")}</small>
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
            <strong>{t("중복 검사 최소 크기")}</strong>
            <small>{t("작은 파일을 비교하는 작업량을 줄입니다")}</small>
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
            <h2>{t("결과 한도")}</h2>
            <p>{t("메모리 사용량과 화면 밀도를 제어합니다.")}</p>
          </div>
        </div>

        <label className="setting-row">
          <span>
            <strong>{t("큰 파일 결과")}</strong>
            <small>{t("크기순으로 보관할 최대 항목 수")}</small>
          </span>
          <select
            value={config.maxLargeFiles}
            onChange={(event) => update("maxLargeFiles", Number(event.currentTarget.value))}
          >
            <option value={100}>{t("{{count}}개", { count: 100 })}</option>
            <option value={250}>{t("{{count}}개", { count: 250 })}</option>
            <option value={500}>{t("{{count}}개", { count: 500 })}</option>
          </select>
        </label>

        <label className="setting-row">
          <span>
            <strong>{t("중복 그룹 결과")}</strong>
            <small>{t("낭비 용량순으로 보관할 최대 그룹 수")}</small>
          </span>
          <select
            value={config.maxDuplicateGroups}
            onChange={(event) => update("maxDuplicateGroups", Number(event.currentTarget.value))}
          >
            <option value={50}>{t("{{count}}개", { count: 50 })}</option>
            <option value={100}>{t("{{count}}개", { count: 100 })}</option>
            <option value={250}>{t("{{count}}개", { count: 250 })}</option>
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
          <h2>{t("현재 안전 계약")}</h2>
          <ul>
            <li>{t("스캔은 파일을 수정하거나 이동하지 않습니다.")}</li>
            <li>{t("같은 저장공간을 가리키는 여러 파일 이름은 중복 낭비로 세지 않습니다.")}</li>
            <li>{t("일부 내용으로 후보를 줄인 뒤 전체 내용을 끝까지 비교해 중복을 확정합니다.")}</li>
            <li>{t("선택 항목은 실행 직전 재검증하고 운영체제 휴지통으로만 이동합니다.")}</li>
            <li>{t("일반 파일은 영구 삭제하지 않으며 Windows 설치 정보도 변경하지 않습니다.")}</li>
            <li>{t("Docker 정리는 예외적으로 휴지통을 거치지 않아 별도 확인 뒤에만 실행합니다.")}</li>
          </ul>
        </div>
      </section>
    </div>
  );
}
