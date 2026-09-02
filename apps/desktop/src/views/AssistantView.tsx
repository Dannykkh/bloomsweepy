import {
  Bot,
  Boxes,
  ChevronDown,
  FolderOpen,
  History,
  LoaderCircle,
  MessageSquarePlus,
  RefreshCw,
  Send,
  ShieldCheck,
  Square,
  Trash2,
  UserRound,
} from "lucide-react";
import {
  useEffect,
  useMemo,
  useRef,
  useState,
  type CSSProperties,
  type FormEvent,
  type KeyboardEvent,
} from "react";
import { ControlStatusPanel } from "../components/ControlStatusPanel";
import { DockerCleanupDialog } from "../components/DockerCleanupDialog";
import { useLanguage, type Translate } from "../i18n";
import {
  appendAssistantMessage,
  askAssistant,
  cancelAssistant,
  createAssistantSession,
  createDockerCleanupPreview,
  deleteAssistantSession,
  getAssistantProviderStatus,
  getAssistantSession,
  listAssistantSessions,
} from "../lib/bridge";
import { formatAssistantPlainText } from "../lib/assistantText";
import { isDockerManagementQuestion } from "../lib/dockerIntent";
import { formatBytes, formatCount, formatDate, formatDockerBytes } from "../lib/format";
import { findVolumeForPath } from "../lib/volumePath";
import type {
  AssistantChatTurn,
  AssistantDockerContext,
  AssistantFolderSummary,
  AssistantProviderKind,
  AssistantProviderStatus,
  AssistantScopeKind,
  AssistantSessionDetail,
  AssistantSessionSummary,
  ControlStatus,
  DirectoryScanProgress,
  DirectoryScanReport,
  DockerCleanupPreview,
  DockerManagementStatus,
  ScanConfig,
  ScanUiState,
  VolumeInfo,
} from "../types";

interface AssistantDisplayTurn extends AssistantChatTurn {
  providerLabel?: string;
  sequence?: number;
}

const providerPreferenceKey = "bloomsweepy.assistant-provider";
const ollamaModelPreferenceKey = "bloomsweepy.ollama-model";

interface AssistantViewProps {
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
  directoryProgress: DirectoryScanProgress | null;
  directoryState: ScanUiState;
  volumes: VolumeInfo[];
  dockerStatus: DockerManagementStatus | null;
  launchRequest: { id: number; target: "docker" } | null;
  onLaunchRequestHandled: () => void;
  onPickFolder: () => Promise<DirectoryScanReport | null>;
}

export function AssistantView({
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
  directoryProgress,
  directoryState,
  volumes,
  dockerStatus,
  launchRequest,
  onLaunchRequestHandled,
  onPickFolder,
}: AssistantViewProps) {
  const { language, t } = useLanguage();
  const initialProviderPreference = useRef(readProviderPreference());
  const initialOllamaModelPreference = useRef(readOllamaModelPreference());
  const [providers, setProviders] = useState<AssistantProviderStatus[]>([]);
  const [selectedProviderKind, setSelectedProviderKind] = useState<AssistantProviderKind>(
    initialProviderPreference.current ?? "codex",
  );
  const [checkingProviders, setCheckingProviders] = useState(true);
  const [selectedOllamaModel, setSelectedOllamaModel] = useState(
    initialOllamaModelPreference.current ?? "",
  );
  const [providerError, setProviderError] = useState<string | null>(null);
  const [sessions, setSessions] = useState<AssistantSessionSummary[]>([]);
  const [activeSession, setActiveSession] = useState<AssistantSessionDetail | null>(null);
  const [sessionsLoading, setSessionsLoading] = useState(true);
  const [sessionBusy, setSessionBusy] = useState(false);
  const [sessionError, setSessionError] = useState<string | null>(null);
  const [turns, setTurns] = useState<AssistantDisplayTurn[]>([]);
  const [draft, setDraft] = useState("");
  const [sending, setSending] = useState(false);
  const [cancelling, setCancelling] = useState(false);
  const [dockerContext, setDockerContext] = useState<AssistantDockerContext | null>(null);
  const [dockerPreview, setDockerPreview] = useState<DockerCleanupPreview | null>(null);
  const [dockerReviewLoading, setDockerReviewLoading] = useState(false);
  const [dockerReviewError, setDockerReviewError] = useState<string | null>(null);
  const transcriptEnd = useRef<HTMLDivElement>(null);
  const requestInFlight = useRef(false);
  const sessionLoadRevision = useRef(0);
  const activeScope = activeSession?.session.scopeRoot ?? null;
  const activeScopeKind = activeSession?.session.scopeKind ?? "folder";
  const summary = activeSession?.folderSummary ?? null;
  const volume = useMemo(
    () => findVolumeForPath(volumes, activeScope),
    [activeScope, volumes],
  );
  const provider = useMemo(
    () => providers.find((candidate) => candidate.provider === selectedProviderKind) ?? null,
    [providers, selectedProviderKind],
  );
  const providerModelReady = provider?.provider !== "ollama" || Boolean(selectedOllamaModel);
  const ready = Boolean(
    activeSession
      && summary
      && provider?.available
      && providerModelReady
      && !sending
      && !sessionBusy,
  );

  useEffect(() => {
    let disposed = false;
    void getAssistantProviderStatus()
      .then((nextProviders) => {
        if (disposed) return;
        setProviders(nextProviders);
        const ollama = nextProviders.find((candidate) => candidate.provider === "ollama");
        setSelectedOllamaModel((current) => chooseOllamaModel(ollama?.models ?? [], current));
        if (!initialProviderPreference.current) {
          const firstAvailable = nextProviders.find((candidate) => candidate.available);
          if (firstAvailable) setSelectedProviderKind(firstAvailable.provider);
        }
      })
      .catch((reason) => {
        if (!disposed) setProviderError(normalizeAssistantError(reason, t));
      })
      .finally(() => {
        if (!disposed) setCheckingProviders(false);
      });
    return () => {
      disposed = true;
    };
  }, []);

  useEffect(() => {
    let disposed = false;
    const revision = ++sessionLoadRevision.current;
    void (async () => {
      setSessionsLoading(true);
      setSessionError(null);
      try {
        const nextSessions = await listAssistantSessions();
        if (disposed || revision !== sessionLoadRevision.current) return;
        setSessions(nextSessions);
        if (nextSessions.length > 0) {
          const detail = await getAssistantSession(nextSessions[0].id);
          if (disposed || revision !== sessionLoadRevision.current) return;
          activateSessionDetail(detail);
        }
      } catch (reason) {
        if (!disposed && revision === sessionLoadRevision.current) {
          setSessionError(normalizeAssistantError(reason, t));
        }
      } finally {
        if (!disposed && revision === sessionLoadRevision.current) {
          setSessionsLoading(false);
        }
      }
    })();
    return () => {
      disposed = true;
    };
  }, []);

  async function recheckProvider() {
    setCheckingProviders(true);
    setProviderError(null);
    try {
      const nextProviders = await getAssistantProviderStatus();
      setProviders(nextProviders);
      const ollama = nextProviders.find((candidate) => candidate.provider === "ollama");
      setSelectedOllamaModel((current) => chooseOllamaModel(ollama?.models ?? [], current));
    } catch (reason) {
      setProviderError(normalizeAssistantError(reason, t));
    } finally {
      setCheckingProviders(false);
    }
  }

  function changeProvider(nextProvider: AssistantProviderKind) {
    setSelectedProviderKind(nextProvider);
    setProviderError(null);
    writeProviderPreference(nextProvider);
  }

  function changeOllamaModel(nextModel: string) {
    setSelectedOllamaModel(nextModel);
    setProviderError(null);
    writeOllamaModelPreference(nextModel);
  }

  function activateSessionDetail(detail: AssistantSessionDetail) {
    setActiveSession(detail);
    setTurns(detail.messages.map((message) => ({
      role: message.role,
      content: message.role === "assistant"
        ? formatAssistantPlainText(message.content)
        : message.content,
      providerLabel: message.providerLabel
        ? message.model
          ? `${message.providerLabel} · ${message.model}`
          : message.providerLabel
        : undefined,
      sequence: message.sequence,
    })));
    setDraft("");
    setProviderError(null);
    setDockerContext(null);
    setDockerPreview(null);
    setDockerReviewError(null);
  }

  function updateSessionSummary(nextSession: AssistantSessionSummary) {
    setSessions((current) => [
      nextSession,
      ...current.filter((candidate) => candidate.id !== nextSession.id),
    ]);
    setActiveSession((current) => current?.session.id === nextSession.id
      ? { ...current, session: nextSession }
      : current);
  }

  async function openStoredSession(sessionId: string) {
    if (sessionBusy || sending || sessionId === activeSession?.session.id) return;
    const revision = ++sessionLoadRevision.current;
    setSessionBusy(true);
    setSessionError(null);
    try {
      const detail = await getAssistantSession(sessionId);
      if (revision !== sessionLoadRevision.current) return;
      activateSessionDetail(detail);
    } catch (reason) {
      if (revision === sessionLoadRevision.current) {
        setSessionError(normalizeAssistantError(reason, t));
      }
    } finally {
      if (revision === sessionLoadRevision.current) setSessionBusy(false);
    }
  }

  async function startNewConversation() {
    if (sessionsLoading || sessionBusy || sending) return;
    const revision = ++sessionLoadRevision.current;
    setSessionBusy(true);
    setSessionError(null);
    try {
      const report = await onPickFolder();
      if (!report || revision !== sessionLoadRevision.current) return;
      const detail = await createAssistantSession({
        scopeKind: "folder",
        scopeRoot: report.root,
        folderSummary: buildFolderSummary(report),
      });
      if (revision !== sessionLoadRevision.current) return;
      setSessions((current) => [
        detail.session,
        ...current.filter((candidate) => candidate.id !== detail.session.id),
      ]);
      activateSessionDetail(detail);
    } catch (reason) {
      if (revision === sessionLoadRevision.current) {
        setSessionError(normalizeAssistantError(reason, t));
      }
    } finally {
      if (revision === sessionLoadRevision.current) setSessionBusy(false);
    }
  }

  async function startDockerConversation() {
    if (sessionsLoading || sessionBusy || sending || !dockerStatus?.enabled) return;
    const revision = ++sessionLoadRevision.current;
    setSessionBusy(true);
    setSessionError(null);
    try {
      const detail = await createAssistantSession({
        scopeKind: "docker",
        scopeRoot: "docker://local",
        folderSummary: buildDockerSummary(),
      });
      if (revision !== sessionLoadRevision.current) return;
      setSessions((current) => [
        detail.session,
        ...current.filter((candidate) => candidate.id !== detail.session.id),
      ]);
      activateSessionDetail(detail);
    } catch (reason) {
      if (revision === sessionLoadRevision.current) {
        setSessionError(normalizeAssistantError(reason, t));
      }
    } finally {
      if (revision === sessionLoadRevision.current) setSessionBusy(false);
    }
  }

  useEffect(() => {
    if (!launchRequest || launchRequest.target !== "docker" || sessionsLoading) return;
    onLaunchRequestHandled();
    void startDockerConversation();
  }, [launchRequest?.id, sessionsLoading]);

  async function removeCurrentSession() {
    const current = activeSession?.session;
    if (!current || sessionBusy || sending) return;
    const confirmed = window.confirm(
      t("“{{scope}}”의 메시지 {{count}}개를 삭제할까요?\n\n이 대화와 저장된 {{summary}}만 삭제하며 실제 데이터는 그대로 둡니다.", {
        scope: current.scopeName,
        count: formatCount(current.messageCount),
        summary: current.scopeKind === "docker" ? t("Docker 요약") : t("폴더 요약"),
      }),
    );
    if (!confirmed) return;

    const revision = ++sessionLoadRevision.current;
    setSessionBusy(true);
    setSessionError(null);
    try {
      await deleteAssistantSession(current.id);
      const remaining = sessions.filter((candidate) => candidate.id !== current.id);
      setSessions(remaining);
      if (remaining.length > 0) {
        const detail = await getAssistantSession(remaining[0].id);
        if (revision !== sessionLoadRevision.current) return;
        activateSessionDetail(detail);
      } else {
        setActiveSession(null);
        setTurns([]);
        setDraft("");
        setDockerContext(null);
        setDockerPreview(null);
        setDockerReviewError(null);
      }
    } catch (reason) {
      if (revision === sessionLoadRevision.current) {
        setSessionError(normalizeAssistantError(reason, t));
      }
    } finally {
      if (revision === sessionLoadRevision.current) setSessionBusy(false);
    }
  }

  useEffect(() => {
    transcriptEnd.current?.scrollIntoView({ block: "nearest" });
  }, [sending, turns]);

  useEffect(() => {
    if (selectedOllamaModel) writeOllamaModelPreference(selectedOllamaModel);
  }, [selectedOllamaModel]);

  async function submitQuestion(event?: FormEvent) {
    event?.preventDefault();
    const message = draft.trim();
    const sessionId = activeSession?.session.id;
    if (requestInFlight.current || !ready || !message || !summary || !sessionId) return;
    const includeDockerStatus = activeScopeKind === "docker" || isDockerManagementQuestion(message);

    requestInFlight.current = true;
    const previousTurns = boundedConversationHistory(turns);
    setTurns((current) => [...current, { role: "user", content: message }]);
    setDraft("");
    setSending(true);
    setProviderError(null);
    setSessionError(null);
    setDockerContext(null);
    setDockerPreview(null);
    setDockerReviewError(null);
    let userMessageSaved = false;

    try {
      const userMutation = await appendAssistantMessage({
        sessionId,
        role: "user",
        content: message,
        provider: null,
        model: null,
      });
      userMessageSaved = true;
      updateSessionSummary(userMutation.session);
      const response = await askAssistant({
        provider: selectedProviderKind,
        model: selectedProviderKind === "ollama" ? selectedOllamaModel : null,
        message,
        history: previousTurns,
        summary,
        scopeKind: activeScopeKind,
        includeDockerStatus,
        responseLanguage: language,
      });
      const assistantMessage = formatAssistantPlainText(response.message);
      setTurns((current) => [
        ...current,
        {
          role: "assistant",
          content: assistantMessage,
          providerLabel: response.model ? `${response.label} · ${response.model}` : response.label,
        },
      ]);
      setDockerContext(response.dockerContext);
      try {
        const assistantMutation = await appendAssistantMessage({
          sessionId,
          role: "assistant",
          content: assistantMessage,
          provider: response.provider,
          model: response.model,
        });
        updateSessionSummary(assistantMutation.session);
      } catch (reason) {
        setSessionError(
          t("AI 응답은 받았지만 대화 기록에 저장하지 못했습니다. {{detail}}", { detail: normalizeAssistantError(reason, t) }),
        );
      }
      setProviders((current) => current.map((candidate) => (
        candidate.provider === response.provider ? { ...candidate, busy: false } : candidate
      )));
    } catch (reason) {
      if (!userMessageSaved) {
        setTurns((current) => current.slice(0, -1));
        setDraft(message);
        setSessionError(normalizeAssistantError(reason, t));
      } else {
        setProviderError(normalizeAssistantError(reason, t));
      }
    } finally {
      requestInFlight.current = false;
      setSending(false);
      setCancelling(false);
    }
  }

  async function stopAssistant() {
    if (!sending || cancelling) return;
    setCancelling(true);
    try {
      const requested = await cancelAssistant();
      if (!requested) setCancelling(false);
    } catch (reason) {
      setProviderError(normalizeAssistantError(reason, t));
      setCancelling(false);
    }
  }

  async function prepareDockerCleanupReview() {
    if (dockerReviewLoading || !dockerContext?.enabled || !dockerContext.available) return;
    setDockerReviewLoading(true);
    setDockerReviewError(null);
    try {
      setDockerPreview(await createDockerCleanupPreview());
    } catch (reason) {
      setDockerReviewError(normalizeAssistantError(reason, t));
    } finally {
      setDockerReviewLoading(false);
    }
  }

  function updateDockerContext(statusAfter: DockerManagementStatus) {
    if (!statusAfter.enabled) {
      setDockerContext(null);
      return;
    }
    setDockerContext({
      enabled: statusAfter.enabled,
      available: statusAfter.cliInstalled === true && statusAfter.daemonRunning === true,
      detail: statusAfter.detail,
      capturedAtUnixMs: statusAfter.capturedAtUnixMs,
      totalSizeBytes: statusAfter.totalSizeBytes,
      reclaimableBytes: statusAfter.reclaimableBytes,
      volumesExcluded: true,
      categories: statusAfter.categories,
    });
  }

  function handleComposerKeyDown(event: KeyboardEvent<HTMLTextAreaElement>) {
    if (event.key !== "Enter" || event.shiftKey || event.nativeEvent.isComposing) return;
    event.preventDefault();
    void submitQuestion();
  }

  return (
    <div className="assistant-workspace">
      <section className="assistant-session-toolbar" aria-label={t("대화 기록 관리")}>
        <button
          type="button"
          className="assistant-session-toolbar__new"
          disabled={sessionsLoading || sessionBusy || sending}
          onClick={() => void startNewConversation()}
        >
          {sessionBusy && directoryState === "scanning"
            ? <LoaderCircle className="is-spinning" size={17} aria-hidden="true" />
            : <MessageSquarePlus size={17} aria-hidden="true" />}
          {sessionBusy && directoryState === "scanning" ? t("새 폴더 확인 중") : t("새 폴더 대화")}
        </button>
        {dockerStatus?.enabled ? (
          <button
            type="button"
            className="assistant-session-toolbar__docker"
            disabled={sessionsLoading || sessionBusy || sending}
            onClick={() => void startDockerConversation()}
          >
            <Boxes size={17} aria-hidden="true" />
            {t("Docker 대화")}
          </button>
        ) : null}
        <label className="assistant-session-toolbar__picker">
          <History size={17} aria-hidden="true" />
          <span className="sr-only">{t("저장된 대화 선택")}</span>
          <select
            aria-label={t("저장된 대화 선택")}
            value={activeSession?.session.id ?? ""}
            disabled={sessionsLoading || sessionBusy || sending || sessions.length === 0}
            onChange={(event) => void openStoredSession(event.currentTarget.value)}
          >
            {sessions.length > 0 ? sessions.map((session) => (
              <option value={session.id} key={session.id}>
                {sessionOptionLabel(session, t)}
              </option>
            )) : (
              <option value="">
                {sessionsLoading ? t("대화 기록 불러오는 중") : t("저장된 대화 없음")}
              </option>
            )}
          </select>
        </label>
        <button
          type="button"
          className="assistant-session-toolbar__delete"
          aria-label={activeSession
            ? t("{{scope}} 대화 삭제", { scope: activeSession.session.scopeName })
            : t("현재 대화 삭제")}
          title={t("현재 대화만 삭제")}
          disabled={!activeSession || sessionBusy || sending}
          onClick={() => void removeCurrentSession()}
        >
          <Trash2 size={17} aria-hidden="true" />
          <span>{t("삭제")}</span>
        </button>
      </section>

      {sessionError ? <p className="assistant-session-error" role="alert">{sessionError}</p> : null}

      <section className="assistant-scope" aria-label={t("현재 대화 대상")}>
        <button
          type="button"
          className="assistant-scope__picker"
          disabled={sessionsLoading || sending || sessionBusy}
          onClick={() => void (activeScopeKind === "docker"
            ? startDockerConversation()
            : startNewConversation())}
        >
          {activeScopeKind === "docker"
            ? <Boxes size={18} aria-hidden="true" />
            : <FolderOpen size={18} aria-hidden="true" />}
          <span>
            <small>{t("대화 대상")}</small>
            <span className="assistant-scope__folder-line">
              <strong title={activeScope ?? undefined}>
                {activeSession?.session.scopeName ?? t("새 대화를 시작하세요")}
              </strong>
              {activeScopeKind === "docker" ? (
                <DockerScopeMetrics status={dockerStatus} t={t} />
              ) : (
                <FolderScopeMetrics summary={summary} volume={volume} t={t} />
              )}
            </span>
          </span>
          <span className="assistant-scope__change">
            {activeScopeKind === "docker" ? t("새 Docker 대화") : t("새 폴더 대화")}
          </span>
        </button>
        <div
          className={`assistant-provider-picker ${provider?.available ? "is-ready" : ""} ${provider?.provider === "ollama" ? "has-model" : ""}`}
          title={provider?.detail ?? t("설치된 AI CLI 상태 확인 중")}
        >
          <span className="assistant-provider-picker__dot" aria-hidden="true" />
          <select
            aria-label={t("대화 상대 선택")}
            value={selectedProviderKind}
            disabled={providers.length === 0 || sending || sessionBusy}
            onChange={(event) => changeProvider(event.currentTarget.value as AssistantProviderKind)}
          >
            {providers.length > 0 ? providers.map((candidate) => (
              <option value={candidate.provider} key={candidate.provider}>
                {providerOptionLabel(candidate, t)}
              </option>
            )) : (
              <option value="codex">{t("AI CLI 확인 중")}</option>
            )}
          </select>
          {provider?.provider === "ollama" ? (
            <select
              className="assistant-provider-picker__model"
              aria-label={t("Ollama 모델 선택")}
              value={selectedOllamaModel}
              disabled={provider.models.length === 0 || sending || sessionBusy}
              onChange={(event) => changeOllamaModel(event.currentTarget.value)}
            >
              {provider.models.length > 0 ? provider.models.map((model) => (
                <option value={model.id} key={model.id}>{model.label}</option>
              )) : (
                <option value="">{t("설치된 모델 없음")}</option>
              )}
            </select>
          ) : null}
          <button
            type="button"
            aria-label={t("AI CLI 상태 다시 확인")}
            disabled={checkingProviders || sending || sessionBusy}
            onClick={() => void recheckProvider()}
          >
            <RefreshCw className={checkingProviders ? "is-spinning" : ""} size={16} aria-hidden="true" />
          </button>
        </div>
      </section>

      <section className="assistant-chat" aria-label={activeScopeKind === "docker" ? t("Docker 용량 대화") : t("폴더 분석 대화")}>
        <div className="assistant-transcript">
          {turns.length > 0 ? (
            turns.map((turn, index) => (
              <article
                className={`assistant-message is-${turn.role}`}
                key={turn.sequence ? `${turn.role}-${turn.sequence}` : `${turn.role}-${index}`}
              >
                <span aria-hidden="true">
                  {turn.role === "user" ? <UserRound size={17} /> : <Bot size={17} />}
                </span>
                <div>
                  <strong>{turn.role === "user" ? t("나") : turn.providerLabel ?? t("AI 도우미")}</strong>
                  <p>{turn.content}</p>
                </div>
              </article>
            ))
          ) : (
            <AssistantEmptyState
              summary={summary}
              scopeKind={activeScopeKind}
              dockerStatus={dockerStatus}
              progress={directoryProgress}
              state={directoryState}
              provider={provider}
              sessionsLoading={sessionsLoading}
              sessionBusy={sessionBusy}
              onStartNewConversation={startNewConversation}
              t={t}
            />
          )}
          {sending ? (
            <div className="assistant-thinking" role="status">
              <LoaderCircle size={17} aria-hidden="true" />
              {provider?.provider === "ollama"
                ? t("{{provider}} 응답 생성 중 · 언제든 취소할 수 있습니다", { provider: providerConversationLabel(provider, selectedOllamaModel, t) })
                : t("{{provider}}가 앱의 {{summary}}을 읽고 있습니다", {
                    provider: providerConversationLabel(provider, selectedOllamaModel, t),
                    summary: activeScopeKind === "docker" ? t("Docker 요약") : t("폴더 요약"),
                  })}
            </div>
          ) : null}
          {dockerContext?.enabled ? (
            <aside
              className={`assistant-docker-action ${dockerContext.available ? "is-ready" : "is-unavailable"}`}
              aria-label={t("Docker 사용량과 정리 검토")}
            >
              <Boxes size={18} aria-hidden="true" />
              <span>
                <strong>
                  {dockerContext.available
                    ? t("Docker 범주 합계 {{size}}", { size: formatDockerBytes(dockerContext.totalSizeBytes) })
                    : t("Docker 상태를 확인해 주세요")}
                </strong>
                <small>
                  {dockerContext.available
                    ? t("볼륨 제외 참고 상한 {{size}} · 실제 디스크 사용량과 다를 수 있음", { size: formatDockerBytes(dockerContext.reclaimableBytes) })
                    : dockerContext.detail}
                </small>
              </span>
              {dockerContext.available ? (
                <button
                  className="secondary-button"
                  type="button"
                  disabled={dockerReviewLoading || sending || dockerContext.reclaimableBytes === 0}
                  onClick={() => void prepareDockerCleanupReview()}
                >
                  {dockerReviewLoading
                    ? <LoaderCircle className="is-spinning" size={15} aria-hidden="true" />
                    : <Boxes size={15} aria-hidden="true" />}
                  {dockerContext.reclaimableBytes === 0 ? t("정리할 항목 없음") : t("Docker 정리 검토")}
                </button>
              ) : null}
            </aside>
          ) : null}
          {dockerReviewError ? (
            <p className="assistant-docker-error" role="alert">{dockerReviewError}</p>
          ) : null}
          <div ref={transcriptEnd} />
        </div>

        {providerError ? <p className="assistant-chat__error" role="alert">{providerError}</p> : null}

        <form className="assistant-composer" onSubmit={(event) => void submitQuestion(event)}>
          <textarea
            value={draft}
            maxLength={2_000}
            rows={2}
            name="assistantQuestion"
            autoComplete="off"
            disabled={!summary || !provider?.available || !providerModelReady || sending || sessionBusy}
            aria-label={activeScopeKind === "docker" ? t("Docker 용량에 관해 질문") : t("선택한 폴더에 관해 질문")}
            placeholder={composerPlaceholder(
              sessionBusy,
              Boolean(activeSession),
              activeScopeKind,
              provider,
              selectedOllamaModel,
              t,
            )}
            onChange={(event) => setDraft(event.currentTarget.value)}
            onKeyDown={handleComposerKeyDown}
          />
          {sending ? (
            <button type="button" aria-label={t("AI 응답 취소")} disabled={cancelling} onClick={() => void stopAssistant()}>
              <Square size={16} aria-hidden="true" />
            </button>
          ) : (
            <button type="submit" aria-label={t("질문 보내기")} disabled={!ready || !draft.trim()}>
              <Send size={18} aria-hidden="true" />
            </button>
          )}
        </form>
        <p className="assistant-composer-note">
          {activeScopeKind === "docker"
            ? t("폴더나 파일 내용이 아니라, BroomSweepy가 Docker CLI로 읽은 범주별 용량 요약만 {{provider}}에 전달합니다.", { provider: providerConversationLabel(provider, selectedOllamaModel, t) })
            : t("파일 내용이나 전체 경로가 아니라, BroomSweepy가 만든 폴더 이름·크기 요약만 {{provider}}에 전달합니다.", { provider: providerConversationLabel(provider, selectedOllamaModel, t) })}
        </p>
      </section>

      <details className="assistant-access-details">
        <summary>
          <ShieldCheck size={17} aria-hidden="true" />
          <span>
            <strong>{t("연결과 권한")}</strong>
            <small>{t("외부 터미널 제어와 전송 범위 확인")}</small>
          </span>
          <ChevronDown size={17} aria-hidden="true" />
        </summary>
        <div className="assistant-access-details__copy">
          <p>
            {activeScopeKind === "docker"
              ? t("Docker 조회는 BroomSweepy가 수행합니다. 앱은 범주별 사용량과 정리 가능 참고 상한만 선택한 AI CLI의 질문 입력으로 보냅니다.")
              : t("폴더 선택과 읽기 검사는 BroomSweepy가 수행합니다. 앱은 파일 내용과 전체 경로를 빼고 제한된 요약만 선택한 AI CLI의 질문 입력으로 보냅니다.")}
          </p>
          <p>
            {providerPermissionDetail(provider, t)} {t("아래 설정은 별도 터미널 제어용입니다.")}
          </p>
        </div>
        <ControlStatusPanel
          status={status}
          canEnableSearch={canEnableSearch}
          updatingSearchAccess={updatingSearchAccess}
          searchAccessError={searchAccessError}
          onToggleSearchAccess={onToggleSearchAccess}
          scanRoot={scanRoot}
          scanConfig={scanConfig}
          canEnableScan={canEnableScan}
          updatingScanAccess={updatingScanAccess}
          scanAccessError={scanAccessError}
          onToggleScanAccess={onToggleScanAccess}
          canEnableCleanup={canEnableCleanup}
          cleanupAccessLocked={cleanupAccessLocked}
          updatingCleanupAccess={updatingCleanupAccess}
          cleanupAccessError={cleanupAccessError}
          onToggleCleanupAccess={onToggleCleanupAccess}
          onReviewPending={onReviewPending}
        />
      </details>

      <DockerCleanupDialog
        preview={dockerPreview}
        onClose={() => setDockerPreview(null)}
        onCompleted={updateDockerContext}
      />
    </div>
  );
}

function FolderScopeMetrics({
  summary,
  volume,
  t,
}: {
  summary: AssistantFolderSummary | null;
  volume: VolumeInfo | null;
  t: Translate;
}) {
  if (!summary) {
    return <span className="assistant-scope__size">{t("폴더 선택부터 시작합니다")}</span>;
  }
  const share = folderDriveShare(summary.totalLogicalBytes, volume?.totalBytes ?? 0);
  return (
    <span className="assistant-scope__metrics">
      <span className="assistant-scope__size">{formatBytes(summary.totalLogicalBytes)}</span>
      {share && volume ? <DriveShareIndicator share={share} volume={volume} t={t} /> : null}
    </span>
  );
}

function DockerScopeMetrics({ status, t }: { status: DockerManagementStatus | null; t: Translate }) {
  if (!status?.enabled) {
    return <span className="assistant-scope__size">{t("설정에서 Docker 관리를 켜세요")}</span>;
  }
  return (
    <span className="assistant-scope__metrics">
      <span className="assistant-scope__size">{formatDockerBytes(status.totalSizeBytes)}</span>
      <span className="assistant-drive-share">
        {t("정리 가능 최대 {{size}}", { size: formatDockerBytes(status.reclaimableBytes) })}
      </span>
    </span>
  );
}

interface FolderDriveShare {
  percentage: number;
  label: string;
}

function DriveShareIndicator({
  share,
  volume,
  t,
}: {
  share: FolderDriveShare;
  volume: VolumeInfo;
  t: Translate;
}) {
  const drive = volumeLabel(volume);
  const visiblePercentage = share.percentage > 0
    ? Math.max(share.percentage, 1)
    : 0;
  const label = t("{{drive}} 전체의 {{share}}", { drive, share: share.label });
  return (
    <span className="assistant-drive-share" role="img" aria-label={label} title={label}>
      <span
        className="assistant-drive-share__ring"
        aria-hidden="true"
        style={{
          "--assistant-drive-share": `${Math.min(100, visiblePercentage) * 3.6}deg`,
        } as CSSProperties}
      />
      <span>{drive} {share.label}</span>
    </span>
  );
}

function folderDriveShare(folderBytes: number, driveBytes: number): FolderDriveShare | null {
  if (!Number.isFinite(folderBytes) || !Number.isFinite(driveBytes) || driveBytes <= 0) {
    return null;
  }
  const percentage = Math.max(0, (folderBytes / driveBytes) * 100);
  if (percentage > 0 && percentage < 0.1) {
    return { percentage, label: "<0.1%" };
  }
  if (percentage < 10) {
    return { percentage, label: `${percentage.toFixed(1)}%` };
  }
  return { percentage, label: `${Math.round(percentage)}%` };
}

function volumeLabel(volume: VolumeInfo): string {
  const driveLetter = volume.mountPoint.match(/^[a-z]:/i)?.[0];
  if (driveLetter) return driveLetter.toLocaleUpperCase("en-US");
  return volume.name || volume.mountPoint;
}

function AssistantEmptyState({
  summary,
  scopeKind,
  dockerStatus,
  progress,
  state,
  provider,
  sessionsLoading,
  sessionBusy,
  onStartNewConversation,
  t,
}: {
  summary: AssistantFolderSummary | null;
  scopeKind: AssistantScopeKind;
  dockerStatus: DockerManagementStatus | null;
  progress: DirectoryScanProgress | null;
  state: ScanUiState;
  provider: AssistantProviderStatus | null;
  sessionsLoading: boolean;
  sessionBusy: boolean;
  onStartNewConversation: () => Promise<void>;
  t: Translate;
}) {
  if (sessionsLoading) {
    return (
      <div className="assistant-empty">
        <LoaderCircle className="is-spinning" size={28} aria-hidden="true" />
        <strong>{t("대화 기록을 불러오고 있습니다")}</strong>
        <p>{t("이 컴퓨터에 저장된 최근 폴더 대화를 확인합니다.")}</p>
      </div>
    );
  }

  if (!summary) {
    const scanning = sessionBusy && state === "scanning";
    return (
      <div className="assistant-empty">
        <FolderOpen size={28} aria-hidden="true" />
        <strong>{scanning ? t("새 폴더를 살펴보고 있습니다") : t("새 대화는 폴더 선택부터 시작합니다")}</strong>
        <p>
          {scanning
            ? t("{{count}}개 항목 · {{size}} 확인", { count: formatCount(progress?.processedEntries ?? 0), size: formatBytes(progress?.processedBytes ?? 0) })
            : t("폴더를 고르면 앱이 용량을 계산하고 빈 대화를 만듭니다.")}
        </p>
        {!scanning ? (
          <button type="button" disabled={sessionBusy} onClick={() => void onStartNewConversation()}>
            {t("새 대화")}
          </button>
        ) : null}
      </div>
    );
  }

  return (
    <div className="assistant-empty is-ready">
      {scopeKind === "docker"
        ? <Boxes size={28} aria-hidden="true" />
        : <Bot size={28} aria-hidden="true" />}
      <strong>{t("{{scope}} 대화 준비됨", { scope: summary.scopeName })}</strong>
      {scopeKind === "docker" ? (
        <p>
          {t("범주 합계 {{total}} · 정리 가능 최대 {{reclaimable}}", {
            total: formatDockerBytes(dockerStatus?.totalSizeBytes ?? 0),
            reclaimable: formatDockerBytes(dockerStatus?.reclaimableBytes ?? 0),
          })}
        </p>
      ) : (
        <p>
          {t("{{size}} · 파일 {{count}}개 · {{date}} 검사", {
            size: formatBytes(summary.totalLogicalBytes),
            count: formatCount(summary.totalFiles),
            date: formatDate(summary.completedAtUnixMs),
          })}
        </p>
      )}
      <small>
        {provider?.available
          ? scopeKind === "docker"
            ? t("예: Docker에서 무엇이 가장 크고 무엇부터 정리할까?")
            : t("예: 어느 폴더가 가장 크고 무엇부터 확인해야 해?")
          : provider
            ? providerUnavailableMessage(provider, t).replace(/…$/, "")
            : t("설치된 AI CLI를 확인하고 있습니다.")}
      </small>
    </div>
  );
}

function buildFolderSummary(report: DirectoryScanReport): AssistantFolderSummary {
  return {
    scopeName: report.name,
    completedAtUnixMs: report.completedAtUnixMs,
    totalLogicalBytes: report.totalLogicalBytes,
    totalFiles: report.totalFiles,
    totalDirectories: report.totalDirectories,
    unreadableEntries: report.unreadableEntries,
    emptyDirectoryCount: report.emptyDirectoryCount,
    childrenTruncated: report.childrenTruncated,
    children: report.children.slice(0, 24).map((child) => ({
      name: child.name,
      kind: child.isDirectory ? "directory" : "file",
      logicalBytes: child.logicalBytes,
      fileCount: child.fileCount,
      directoryCount: child.directoryCount,
    })),
  };
}

function buildDockerSummary(): AssistantFolderSummary {
  return {
    scopeName: "Docker",
    completedAtUnixMs: Date.now(),
    totalLogicalBytes: 0,
    totalFiles: 0,
    totalDirectories: 0,
    unreadableEntries: 0,
    emptyDirectoryCount: 0,
    childrenTruncated: false,
    children: [],
  };
}

function composerPlaceholder(
  sessionBusy: boolean,
  hasSession: boolean,
  scopeKind: AssistantScopeKind,
  provider: AssistantProviderStatus | null,
  ollamaModel: string,
  t: Translate,
): string {
  if (!provider) return t("설치된 AI CLI를 확인하고 있습니다…");
  if (!provider.available) return providerUnavailableMessage(provider, t);
  if (provider.provider === "ollama" && !ollamaModel) return t("Ollama 모델을 선택해 주세요…");
  if (!hasSession) return t("새 대화를 눌러 폴더를 선택해 주세요…");
  if (sessionBusy) return scopeKind === "docker"
    ? t("Docker 대화를 준비하고 있습니다…")
    : t("새 폴더 검사가 끝나면 질문할 수 있습니다…");
  return scopeKind === "docker"
    ? t("예: Docker에서 무엇이 가장 크고 무엇부터 정리할까?")
    : t("예: 어느 폴더가 가장 크고 무엇부터 확인해야 해?");
}

function sessionOptionLabel(session: AssistantSessionSummary, t: Translate): string {
  return t("{{scope}} · 메시지 {{count}}개 · {{date}}", {
    scope: session.scopeName,
    count: formatCount(session.messageCount),
    date: formatDate(session.updatedAtUnixMs),
  });
}

function boundedConversationHistory(turns: AssistantDisplayTurn[]): AssistantChatTurn[] {
  const selected: AssistantChatTurn[] = [];
  let selectedCharacters = 0;
  for (let index = turns.length - 1; index >= 0 && selected.length < 20; index -= 1) {
    const turn = turns[index];
    const content = turn.content.slice(0, 2_000);
    if (selectedCharacters + content.length > 24_000) break;
    selected.unshift({ role: turn.role, content });
    selectedCharacters += content.length;
  }
  return selected;
}

function providerOptionLabel(provider: AssistantProviderStatus, t: Translate): string {
  if (!provider.installed) return t("{{provider}} · 설치 안 됨", { provider: provider.label });
  if (provider.authentication === "required") return t("{{provider}} · 로그인 필요", { provider: provider.label });
  if (provider.authentication === "notRequired") {
    return provider.models.length > 0
      ? t("{{provider}} · 모델 {{count}}개", { provider: provider.label, count: provider.models.length })
      : t("{{provider}} · 모델 없음", { provider: provider.label });
  }
  return t("{{provider}} · 로그인됨", { provider: provider.label });
}

function providerPermissionDetail(provider: AssistantProviderStatus | null, t: Translate): string {
  switch (provider?.provider) {
    case "codex":
      return t("Codex는 앱 전용 빈 폴더에서 읽기 전용 샌드박스로 실행합니다. Codex 자체 읽기 도구의 실제 범위는 Codex 샌드박스 정책을 따릅니다.");
    case "claudeCode":
      return t("Claude Code는 세션 저장과 도구 사용을 끄고, 승인 질문 없이 안전 모드로 실행합니다.");
    case "grok":
      return t("Grok은 단일 응답 모드에서 내장 도구, 하위 에이전트, 웹 검색을 끕니다. Grok CLI 자체 계정과 세션 정책은 그대로 적용됩니다.");
    case "antigravity":
      return t("Antigravity는 비대화형 응답 모드와 샌드박스로 실행합니다. Antigravity 자체 계정과 설정 정책은 그대로 적용됩니다.");
    case "ollama":
      return t("Ollama에는 도구를 제공하지 않습니다. 로컬 모델이면 요약이 컴퓨터 안에서 처리되고, cloud 모델이면 Ollama 서비스로 전송됩니다.");
    default:
      return t("AI CLI를 고르면 이곳에 해당 공급자의 실행 권한을 표시합니다.");
  }
}

function readProviderPreference(): AssistantProviderKind | null {
  try {
    const value = window.localStorage.getItem(providerPreferenceKey);
    return value === "codex"
      || value === "claudeCode"
      || value === "grok"
      || value === "antigravity"
      || value === "ollama"
      ? value
      : null;
  } catch {
    return null;
  }
}

function writeProviderPreference(provider: AssistantProviderKind) {
  try {
    window.localStorage.setItem(providerPreferenceKey, provider);
  } catch {
    // The selection still works for this session when storage is unavailable.
  }
}

function readOllamaModelPreference(): string | null {
  try {
    return window.localStorage.getItem(ollamaModelPreferenceKey);
  } catch {
    return null;
  }
}

function writeOllamaModelPreference(model: string) {
  try {
    window.localStorage.setItem(ollamaModelPreferenceKey, model);
  } catch {
    // The selection still works for this session when storage is unavailable.
  }
}

function chooseOllamaModel(
  models: AssistantProviderStatus["models"],
  preferred: string,
): string {
  if (models.some((model) => model.id === preferred)) return preferred;
  const conversational = models.find((model) => !/embed|^bge-/i.test(model.id));
  return conversational?.id ?? models[0]?.id ?? "";
}

function providerConversationLabel(
  provider: AssistantProviderStatus | null,
  ollamaModel: string,
  t: Translate,
): string {
  if (!provider) return t("선택한 AI CLI");
  return provider.provider === "ollama" && ollamaModel
    ? `${provider.label} · ${ollamaModel}`
    : provider.label;
}

function providerUnavailableMessage(provider: AssistantProviderStatus, t: Translate): string {
  if (!provider.installed) return t("{{provider}}를 먼저 설치해 주세요…", { provider: provider.label });
  if (provider.provider === "ollama") return t("Ollama에 대화용 모델을 먼저 설치해 주세요…");
  return t("{{provider}}에서 먼저 로그인해 주세요…", { provider: provider.label });
}

function normalizeAssistantError(reason: unknown, t: Translate): string {
  if (reason instanceof Error) return reason.message;
  if (typeof reason === "string") return reason;
  return t("AI CLI 응답을 받지 못했습니다");
}
