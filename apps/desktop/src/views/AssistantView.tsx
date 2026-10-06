import {
  Bot,
  ArrowDown,
  Boxes,
  ChevronDown,
  FolderOpen,
  History,
  LoaderCircle,
  MessageSquarePlus,
  RefreshCw,
  Send,
  Settings2,
  ShieldCheck,
  Square,
  Trash2,
  UserRound,
  X,
} from "lucide-react";
import {
  useEffect,
  useMemo,
  useRef,
  useState,
  type CSSProperties,
  type FormEvent,
  type KeyboardEvent,
  type ReactNode,
} from "react";
import { ControlStatusPanel, type ControlStatusPanelProps } from "../components/ControlStatusPanel";
import { AssistantEmptyFolderCard, AssistantTrashResultCard } from "../components/AssistantEmptyFolderCard";
import { AssistantFileCard } from "../components/AssistantFileCard";
import { AssistantAppToolCard, appToolTitles, appToolStatusKeys, type AppToolReviewCompletion } from "../components/AssistantAppToolCard";
import { AssistantApplicationConfirmation } from "../components/AssistantApplicationConfirmation";
import { AssistantModelPicker } from "../components/AssistantModelPicker";
import { useAssistantModelPreference } from "../hooks/useAssistantModelPreference";
import { assistantModelRequestValue, assistantModelSelection, assistantReasoningStatus } from "../lib/assistantModelPreference";
import { applicationTrashQuestion, humanTrashDecision, solePendingTrash, namedTrashRequest, workspaceReviewMatches, canAutomaticallyTrashFiles } from "../lib/assistantConfirmation";
import { confirmApplicationTrash, confirmApplicationDataTrash, dismissApplicationPlan } from "../lib/applicationBridge";
import "./AssistantView.css";
import { DockerCleanupDialog } from "../components/DockerCleanupDialog";
import { useLanguage, type Translate } from "../i18n";
import {
  appendAssistantMessage,
  askAssistant,
  listenToAssistantProgress,
  cancelAssistant,
  createAssistantSession,
  createDockerCleanupPreview,
  deleteAssistantSession,
  getAssistantProviderStatus,
  getAssistantSession,
  listAssistantSessions,
  getAssistantEmptyWorkspace,
  selectAssistantEmptyCandidates,
  prepareAssistantEmptyPlan,
  getAssistantFileWorkspace,
  getAssistantDirectoryReport,
  assistantFileAction,
  selectAssistantFiles,
  prepareAssistantFilePlan,
  cancelScan,
} from "../lib/bridge";
import { formatAssistantPlainText } from "../lib/assistantText";
import { elapsedLabel, matchesAssistantProgress } from "../lib/assistantProgress";
import { assistantFailureMessage, assistantProviderStatusKey, isAssistantAuthenticationFailure } from "../lib/assistantProviderStatus";
import { isDockerManagementQuestion } from "../lib/dockerIntent";
import { formatBytes, formatCount, formatDate, formatDockerBytes } from "../lib/format";
import { findVolumeForPath } from "../lib/volumePath";
import type {
  AssistantChatTurn,
  AssistantProgress,
  AppToolResult,
  AppToolLocalCompletion,
  AssistantEmptyWorkspace,
  AssistantFileWorkspace,
  AssistantFileAction,
  TrashOperationResult,
  AssistantDockerContext,
  AssistantFolderSummary,
  AssistantProviderKind,
  AssistantProviderStatus,
  AssistantScopeKind,
  AssistantSessionDetail,
  AssistantSessionSummary,
  DirectoryScanProgress,
  DirectoryScanReport,
  DockerCleanupPreview,
  DockerManagementStatus,
  ScanUiState,
  VolumeInfo,
} from "../types";

interface AssistantDisplayTurn extends AssistantChatTurn {
  providerLabel?: string;
  sequence?: number;
}

interface AssistantViewProps {
  controlSettings: ControlStatusPanelProps;
  directoryProgress: DirectoryScanProgress | null;
  directoryState: ScanUiState;
  volumes: VolumeInfo[];
  dockerStatus: DockerManagementStatus | null;
  launchRequest: { id: number; target: "docker" } | null;
  onLaunchRequestHandled: () => void;
  onPickFolder: () => Promise<DirectoryScanReport | null>;
  onConfirmEmptyPlan: (sessionId: string, revision: string, planId: string, automatic?: boolean) => Promise<TrashOperationResult>;
  onConfirmFilePlan: (sessionId: string, revision: string, planId: string, nestedAck: boolean, automatic?: boolean) => Promise<TrashOperationResult>;
  onDirectoryReport: (report: DirectoryScanReport, open: boolean) => void;
  onOpenCleanupTree?: (sessionId: string, revision: string) => void;
  onAppToolView?: (result: AppToolResult) => void;
  onAppToolReview?: (result: AppToolResult, sessionId: string) => void;
  onAppToolCompleted?: (completion: AppToolReviewCompletion, sessionId: string) => void;
  onAppTrashBusyChange?: (busy: boolean) => void;
  appToolCompletion?: AppToolLocalCompletion | null;
}

export function AssistantView({
  controlSettings,
  directoryProgress,
  directoryState,
  volumes,
  dockerStatus,
  launchRequest,
  onLaunchRequestHandled,
  onPickFolder,
  onConfirmEmptyPlan,
  onConfirmFilePlan,
  onDirectoryReport,
  onOpenCleanupTree,
  onAppToolView,
  onAppToolReview,
  onAppToolCompleted,
  onAppTrashBusyChange,
  appToolCompletion,
}: AssistantViewProps) {
  const { language, t } = useLanguage();
  const { cleanupAccessLocked } = controlSettings;
  const [connectionOpen, setConnectionOpen] = useState(false);
  const connectionDialog = useRef<HTMLDialogElement>(null);
  const connectionTrigger = useRef<HTMLButtonElement>(null);
  const modelPreference = useAssistantModelPreference();
  const initialProviderPreference = useRef(modelPreference.hasSavedProvider);
  const selectedProviderKind = modelPreference.provider;
  const selectedModel = modelPreference.modelFor(selectedProviderKind);
  const selectedReasoningEffort = modelPreference.reasoningEffortFor(selectedProviderKind, selectedModel);
  const [providers, setProviders] = useState<AssistantProviderStatus[]>([]);
  const [appToolResults, setAppToolResults] = useState<AppToolResult[]>([]);
  const handledCompletion = useRef<number | null>(null);
  const [checkingProviders, setCheckingProviders] = useState(true);
  const [providerError, setProviderError] = useState<string | null>(null);
  const [sessions, setSessions] = useState<AssistantSessionSummary[]>([]);
  const [activeSession, setActiveSession] = useState<AssistantSessionDetail | null>(null);
  const [sessionsLoading, setSessionsLoading] = useState(true);
  const [sessionBusy, setSessionBusy] = useState(false);
  const [sessionError, setSessionError] = useState<string | null>(null);
  const [turns, setTurns] = useState<AssistantDisplayTurn[]>([]);
  const [draft, setDraft] = useState("");
  const [sending, setSending] = useState(false);
  const [requestProgress, setRequestProgress] = useState<AssistantProgress | null>(null);
  const [savingResponse, setSavingResponse] = useState(false);
  const [executingTrash, setExecutingTrash] = useState(false);
  const [requestStarted, setRequestStarted] = useState(0);
  const [clockNow, setClockNow] = useState(0);
  const activeProgressId = useRef<string | null>(null);
  const progressUnlisten = useRef<(() => void) | null>(null);
  const [emptyWorkspace, setEmptyWorkspace] = useState<AssistantEmptyWorkspace | null>(null);
  const [fileWorkspace, setFileWorkspace] = useState<AssistantFileWorkspace | null>(null);
  const [emptyActionBusy, setEmptyActionBusy] = useState(false);
  const [trashResult, setTrashResult] = useState<TrashOperationResult | null>(null);
  const [cancelling, setCancelling] = useState(false);
  const [dockerContext, setDockerContext] = useState<AssistantDockerContext | null>(null);
  const [dockerPreview, setDockerPreview] = useState<DockerCleanupPreview | null>(null);
  const [dockerReviewLoading, setDockerReviewLoading] = useState(false);
  const [dockerReviewError, setDockerReviewError] = useState<string | null>(null);
  const transcriptEnd = useRef<HTMLDivElement>(null);
  const transcript = useRef<HTMLDivElement>(null);
  const followLatest = useRef(true);
  const [showLatest, setShowLatest] = useState(false);
  const requestInFlight = useRef(false);
  const sessionLoadRevision = useRef(0);
  const emptyStateRevision = useRef(0);
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
  const providerModelReady = assistantModelSelection(provider) !== "required" || Boolean(selectedModel);
  const reasoning = assistantReasoningStatus(provider, selectedModel, selectedReasoningEffort);
  const ready = Boolean(
    activeSession
      && summary
      && provider?.available
      && !provider.busy
      && !checkingProviders
      && providerModelReady
      && !reasoning.stale
      && !sending
      && !cleanupAccessLocked
      && !sessionBusy,
  );

  const pendingTrash = [
    ...appToolResults.filter(result => applicationTrashQuestion(result)).map(result => ({ kind: "application" as const, result })),
    ...(fileWorkspace?.plan ? [{ kind: "file" as const }] : []),
    ...(emptyWorkspace?.plan ? [{ kind: "empty" as const }] : []),
  ];
  const canAnswerTrash = Boolean(activeSession && pendingTrash.length && !sending && !sessionBusy && !cleanupAccessLocked);

  async function reportTrashCompletion(completion: AppToolReviewCompletion, sessionId: string) {
    if (onAppToolCompleted) { onAppToolCompleted(completion, sessionId); return; }
    setTurns(current => [...current, { role: "assistant", content: completion.message, providerLabel: "BroomSweepy" }]);
    if (completion.trashResult) { setTrashResult(completion.trashResult); setFileWorkspace(null); setEmptyWorkspace(null); }
    await appendAssistantMessage({ sessionId, role: "assistant", content: completion.message, provider: null, model: null })
      .then(mutation => updateSessionSummary(mutation.session)).catch(reason => setSessionError(normalizeAssistantError(reason, t)));
  }

  function answerTrash(confirmed: boolean, message?: string, applicationResult?: AppToolResult) {
    const sessionId = activeSession?.session.id;
    const pending = applicationResult ? { kind: "application" as const, result: applicationResult } : solePendingTrash(pendingTrash);
    if (!sessionId || !pending || requestInFlight.current || sending || sessionBusy || cleanupAccessLocked) return;
    const content = message ?? t(confirmed ? "예, 휴지통으로 이동" : "아니오");
    setDraft("");
    followLatest.current = true;
    setTurns(current => [...current, { role: "user", content }]);
    void appendAssistantMessage({ sessionId, role: "user", content, provider: null, model: null })
      .then(mutation => updateSessionSummary(mutation.session)).catch(reason => setSessionError(normalizeAssistantError(reason, t)));
    if (pending.kind === "file") { void manageFiles(confirmed ? "confirm" : "select", confirmed ? undefined : [], true); return; }
    if (pending.kind === "empty") { void manageEmptyFolders(confirmed ? "confirm" : "select", confirmed ? undefined : []); return; }
    const question = applicationTrashQuestion(pending.result);
    if (!question) return;
    requestInFlight.current = true;
    setSessionBusy(true); setSessionError(null);
    setExecutingTrash(confirmed);
    onAppTrashBusyChange?.(true);
    // Remove the decision immediately; errors are not permission to replay a consumed plan.
    setAppToolResults(current => current.filter(result => result !== pending.result));
    void (async () => {
      let completion: AppToolReviewCompletion;
      try {
        if (!confirmed) {
          await dismissApplicationPlan(question.plan.planId);
          completion = { message: t("취소했습니다. 휴지통으로 이동한 항목은 없습니다.") };
        } else {
          const actual = question.bundle ? await confirmApplicationTrash(question.plan.planId) : await confirmApplicationDataTrash(question.plan.planId);
          completion = { message: t("요청 {{requested}}개 중 {{moved}}개를 휴지통으로 이동했습니다.", { requested: actual.requestedCount, moved: actual.movedCount }), trashResult: actual, mutated: true };
        }
      } catch (reason) {
        completion = { message: t("작업을 완료하지 못했습니다. 대상과 작업 기록을 확인한 뒤 다시 요청하세요. {{detail}}", { detail: normalizeAssistantError(reason, t) }), mutated: confirmed };
      }
      await reportTrashCompletion(completion, sessionId);
    })().finally(() => { requestInFlight.current = false; setSessionBusy(false); setExecutingTrash(false); onAppTrashBusyChange?.(false); });
  }

  useEffect(() => { setAppToolResults([]); }, [activeSession?.session.id]);
  useEffect(() => {
    if (!appToolCompletion || appToolCompletion.sequence === handledCompletion.current || appToolCompletion.sessionId !== activeSession?.session.id) return;
    handledCompletion.current = appToolCompletion.sequence;
    setTurns((current) => [...current, { role: "assistant", content: appToolCompletion.message, providerLabel: "BroomSweepy" }]);
    setAppToolResults((current) => current.filter((result) => result.status !== "review_required"));
    if (appToolCompletion.trashResult) { setTrashResult(appToolCompletion.trashResult); setFileWorkspace(null); setEmptyWorkspace(null); }
    void appendAssistantMessage({ sessionId: appToolCompletion.sessionId!, role: "assistant", content: appToolCompletion.message, provider: null, model: null })
      .then((mutation) => updateSessionSummary(mutation.session))
      .catch((reason) => setSessionError(t("AI 응답은 받았지만 대화 기록에 저장하지 못했습니다. {{detail}}", { detail: normalizeAssistantError(reason, t) })));
  }, [appToolCompletion, activeSession?.session.id]);

  useEffect(() => {
    let disposed = false;
    const revision = ++emptyStateRevision.current;
    setEmptyWorkspace(null);
    setFileWorkspace(null);
    setTrashResult(null);
    const sessionId = activeSession?.session.id;
    if (sessionId && activeSession.session.scopeKind === "folder") {
      void getAssistantEmptyWorkspace(sessionId).then((workspace) => {
        if (!disposed && revision === emptyStateRevision.current) setEmptyWorkspace(workspace);
      }).catch((reason) => { if (!disposed && revision === emptyStateRevision.current) setSessionError(normalizeAssistantError(reason, t)); });
      void getAssistantFileWorkspace(sessionId).then((workspace) => {
        if (!disposed && revision === emptyStateRevision.current) setFileWorkspace(workspace);
      }).catch((reason) => { if (!disposed && revision === emptyStateRevision.current) setSessionError(normalizeAssistantError(reason, t)); });
    }
    return () => { disposed = true; };
  }, [activeSession?.session.id]);

  useEffect(() => {
    const sessionId = activeSession?.session.id;
    if (!sessionId || !fileWorkspace?.mapGeneration) return;
    let disposed = false;
    // Fetch the same generation, not a second disk scan. Older asynchronous replies cannot win.
    void getAssistantDirectoryReport(sessionId, fileWorkspace.revision).then((report) => {
      if (!disposed) onDirectoryReport(report, false);
    }).catch(() => { /* Another scan can expire this snapshot; explicit opening reports the error. */ });
    return () => { disposed = true; };
  }, [activeSession?.session.id, fileWorkspace?.revision, fileWorkspace?.mapGeneration, onDirectoryReport]);

  async function showFileMap() {
    const sessionId = activeSession?.session.id;
    if (!sessionId || !fileWorkspace || requestInFlight.current || sessionBusy || sending || cleanupAccessLocked) return;
    requestInFlight.current = true;
    setSessionBusy(true);
    setSessionError(null);
    try {
      const report = await getAssistantDirectoryReport(sessionId, fileWorkspace.revision);
      onDirectoryReport(report, true);
    } catch (reason) {
      setSessionError(reason instanceof Error ? reason.message : String(reason));
    } finally {
      requestInFlight.current = false;
      setSessionBusy(false);
    }
  }

  async function manageFiles(action: "select" | "prepare" | "confirm" | AssistantFileAction, ids?: string[], nestedAck = false) {
    const sessionId = activeSession?.session.id;
    if (!sessionId || requestInFlight.current || sessionBusy || sending || cleanupAccessLocked) return;
    if (typeof action === "string" && !fileWorkspace) return;
    requestInFlight.current = true;
    ++emptyStateRevision.current;
    setSessionBusy(true);
    setEmptyActionBusy(true);
    setSessionError(null);
    try {
      if (action === "confirm" && fileWorkspace?.plan) {
        setAppToolResults(current => current.filter(result => !workspaceReviewMatches(result, "files.workspace", fileWorkspace.revision)));
        const result = await onConfirmFilePlan(sessionId, fileWorkspace.revision, fileWorkspace.plan.id, nestedAck);
        setFileWorkspace(null); setEmptyWorkspace(null); setTrashResult(result);
        const content = t("요청 {{requested}}개 중 {{moved}}개를 휴지통으로 이동했습니다.", { requested: result.requestedCount, moved: result.movedCount })
          + " " + t("추가 정리 전 파일·폴더를 다시 검사하세요.");
        setTurns((current) => [...current, { role: "assistant", content, providerLabel: "BroomSweepy" }]);
        const mutation = await appendAssistantMessage({ sessionId, role: "assistant", content, provider: null, model: null });
        updateSessionSummary(mutation.session);
      } else {
        const workspace = action === "select"
          ? await selectAssistantFiles(sessionId, fileWorkspace!.revision, ids ?? [])
          : action === "prepare"
            ? await prepareAssistantFilePlan(sessionId, fileWorkspace!.revision)
            : typeof action === "object" ? await assistantFileAction(sessionId, action) : null;
        if (workspace) {
          const cancelled = action === "select" && ids?.length === 0 && Boolean(fileWorkspace?.plan) && !workspace.plan;
          if (fileWorkspace?.plan && fileWorkspace.plan.id !== workspace.plan?.id) {
            setAppToolResults(current => current.filter(result => !workspaceReviewMatches(result, "files.workspace", fileWorkspace.revision)));
          }
          setFileWorkspace(workspace); setEmptyWorkspace(null); setTrashResult(null);
          if (workspace.query === null && workspace.currentPath === activeScope) setActiveSession((current) => current?.session.id === sessionId
            ? { ...current, folderSummary: workspace.summary } : current);
          // Local navigation/review is evidence for the next AI turn, but paths remain local.
          if (action !== "select" || cancelled) {
            const content = cancelled ? t("취소했습니다. 휴지통으로 이동한 항목은 없습니다.") : fileWorkspaceMessage(workspace, t);
            setTurns((current) => [...current, { role: "assistant", content, providerLabel: "BroomSweepy" }]);
            const mutation = await appendAssistantMessage({ sessionId, role: "assistant", content, provider: null, model: null });
            updateSessionSummary(mutation.session);
          }
        }
      }
    } catch (reason) {
      setSessionError(reason instanceof Error ? reason.message : String(reason));
      try { setFileWorkspace(await getAssistantFileWorkspace(sessionId)); } catch { setFileWorkspace(null); }
    } finally {
      requestInFlight.current = false; setSessionBusy(false); setEmptyActionBusy(false);
    }
  }

  async function manageEmptyFolders(action: "select" | "prepare" | "confirm", ids?: string[]) {
    const sessionId = activeSession?.session.id;
    if (!sessionId || !emptyWorkspace || requestInFlight.current || sessionBusy || sending || cleanupAccessLocked) return;
    requestInFlight.current = true;
    ++emptyStateRevision.current;
    setSessionBusy(true);
    setEmptyActionBusy(true);
    setSessionError(null);
    try {
      if (action === "select") {
        const workspace = await selectAssistantEmptyCandidates(sessionId, emptyWorkspace.revision, ids ?? []);
        if (emptyWorkspace.plan && emptyWorkspace.plan.id !== workspace.plan?.id) {
          setAppToolResults(current => current.filter(result => !workspaceReviewMatches(result, "empty.workspace", emptyWorkspace.revision)));
        }
        setEmptyWorkspace(workspace);
        if (emptyWorkspace.plan && ids?.length === 0 && !workspace.plan) {
          const content = t("취소했습니다. 휴지통으로 이동한 항목은 없습니다.");
          setTurns(current => [...current, { role: "assistant", content, providerLabel: "BroomSweepy" }]);
          const mutation = await appendAssistantMessage({ sessionId, role: "assistant", content, provider: null, model: null });
          updateSessionSummary(mutation.session);
        }
      } else if (action === "prepare") {
        setEmptyWorkspace(await prepareAssistantEmptyPlan(sessionId, emptyWorkspace.revision));
      } else if (emptyWorkspace.plan) {
        setAppToolResults(current => current.filter(result => !workspaceReviewMatches(result, "empty.workspace", emptyWorkspace.revision)));
        const result = await onConfirmEmptyPlan(sessionId, emptyWorkspace.revision, emptyWorkspace.plan.id);
        setEmptyWorkspace(null);
        setTrashResult(result);
        const content = t("요청 {{requested}}개 중 {{moved}}개를 휴지통으로 이동했습니다.", { requested: result.requestedCount, moved: result.movedCount })
          + " " + t("추가 정리 전 빈 폴더를 다시 검사하세요.");
        setTurns((current) => [...current, { role: "assistant", content, providerLabel: "BroomSweepy" }]);
        // Count-only execution evidence is retained for the next AI turn; local paths stay in the result card/journal.
        const mutation = await appendAssistantMessage({ sessionId, role: "assistant", content, provider: null, model: null });
        updateSessionSummary(mutation.session);
      }
    } catch (reason) {
      setSessionError(normalizeAssistantError(reason, t));
      // A consumed plan stays consumed even if I/O or persistence failed.
      try { setEmptyWorkspace(await getAssistantEmptyWorkspace(sessionId)); } catch { setEmptyWorkspace(null); }
    } finally {
      requestInFlight.current = false;
      setSessionBusy(false);
      setEmptyActionBusy(false);
    }
  }

  useEffect(() => {
    let disposed = false;
    void getAssistantProviderStatus()
      .then((nextProviders) => {
        if (disposed) return;
        setProviders(nextProviders);
        const ollama = nextProviders.find((candidate) => candidate.provider === "ollama");
        if (!modelPreference.modelFor("ollama") && ollama?.models.length) {
          modelPreference.setModel("ollama", chooseInitialOllamaModel(ollama.models));
        }
        if (!initialProviderPreference.current) {
          const firstAvailable = nextProviders.find((candidate) => candidate.available);
          if (firstAvailable) modelPreference.setProvider(firstAvailable.provider);
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
      if (!modelPreference.modelFor("ollama") && ollama?.models.length) {
        modelPreference.setModel("ollama", chooseInitialOllamaModel(ollama.models));
      }
    } catch (reason) {
      setProviderError(normalizeAssistantError(reason, t));
    } finally {
      setCheckingProviders(false);
    }
  }

  function changeProvider(nextProvider: AssistantProviderKind) {
    modelPreference.setProvider(nextProvider);
    setProviderError(null);
  }

  function changeModel(nextModel: string) {
    modelPreference.setModel(selectedProviderKind, nextModel);
    setProviderError(null);
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
    setAppToolResults([]);
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
    followLatest.current = true;
    setShowLatest(false);
  }, [activeSession?.session.id]);

  useEffect(() => {
    if (followLatest.current && transcript.current) {
      transcript.current.scrollTop = transcript.current.scrollHeight;
    }
  }, [activeSession?.session.id, sending, turns, appToolResults, fileWorkspace, emptyWorkspace]);

  useEffect(() => {
    const dialog = connectionDialog.current;
    if (connectionOpen && dialog && !dialog.open) dialog.showModal();
    else if (!connectionOpen && dialog?.open) dialog.close();
  }, [connectionOpen]);

  useEffect(() => {
    if (!sending) return;
    const timer = window.setInterval(() => setClockNow(Date.now()), 1000);
    return () => window.clearInterval(timer);
  }, [sending]);

  useEffect(() => () => {
    activeProgressId.current = null;
    progressUnlisten.current?.();
    progressUnlisten.current = null;
  }, []);

  async function submitQuestion(event?: FormEvent) {
    event?.preventDefault();
    const message = draft.trim();
    const sessionId = activeSession?.session.id;
    const decision = humanTrashDecision(message);
    if (decision !== null && pendingTrash.length) {
      if (!solePendingTrash(pendingTrash)) { setSessionError(t("확인할 대상이 여러 개입니다. 원하는 카드의 예 또는 아니오를 눌러 주세요.")); return; }
      answerTrash(decision, message);
      return;
    }
    if (requestInFlight.current || !ready || !message || !summary || !sessionId) return;
    const includeDockerStatus = activeScopeKind === "docker" || isDockerManagementQuestion(message);

    requestInFlight.current = true;
    followLatest.current = true;
    setShowLatest(false);
    ++emptyStateRevision.current;
    const previousTurns = boundedConversationHistory(turns);
    setTurns((current) => [...current, { role: "user", content: message }]);
    setDraft("");
    setSending(true);
    const progressId = crypto.randomUUID();
    activeProgressId.current = progressId;
    setRequestProgress(null);
    setSavingResponse(false);
    setRequestStarted(Date.now());
    setClockNow(Date.now());
    setProviderError(null);
    setSessionError(null);
    setDockerContext(null);
    setDockerPreview(null);
    setDockerReviewError(null);
    let userMessageSaved = false;

    try {
      // Observability is best-effort; a missing event channel must not block chat.
      try {
        const unlisten = await listenToAssistantProgress((progress) => {
          if (matchesAssistantProgress(progress, activeProgressId.current, sessionId)) setRequestProgress(progress);
        });
        if (activeProgressId.current === progressId) progressUnlisten.current = unlisten;
        else unlisten();
      } catch { /* Older hosts still provide elapsed time and cancellation. */ }
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
        progressId,
        sessionId,
        provider: selectedProviderKind,
        model: assistantModelRequestValue(provider, selectedModel),
        reasoningEffort: reasoning.requestValue,
        message,
        history: previousTurns,
        summary,
        scopeKind: activeScopeKind,
        includeDockerStatus,
        responseLanguage: language,
      });
      const reviews = (response.appToolResults ?? []).filter(result => result.status === "review_required");
      const applicationReview = reviews.length === 1 ? reviews[0] : null;
      const applicationQuestion = applicationReview ? applicationTrashQuestion(applicationReview) : null;
      const files = response.fileWorkspace;
      const explicitApp = applicationQuestion?.bundle && namedTrashRequest(message, [applicationQuestion.plan.displayName]);
      const explicitFiles = canAutomaticallyTrashFiles(message, files, response.appToolResults ?? []);
      const automaticTrash = controlSettings.status.chatTrashWithoutConfirmation === true
        && !response.emptyWorkspace?.plan
        && ((explicitApp && !files?.plan) || explicitFiles);
      const assistantMessage = automaticTrash ? t("요청한 대상을 확인했습니다. 설정한 권한에 따라 재검사 후 휴지통으로 이동합니다.")
        : response.analysisComplete !== undefined || response.appToolResults?.length
        ? formatAssistantPlainText(response.message)
        : response.fileWorkspace
        ? fileWorkspaceMessage(response.fileWorkspace, t)
        : response.toolAction && response.emptyWorkspace
        ? (response.toolAction === "scan"
            ? t("앱 검사를 완료했습니다. 빈 폴더 {{count}}개를 검토할 수 있습니다.", { count: response.emptyWorkspace.candidates.length })
            : response.toolAction === "selection"
              ? t("후보 선택을 변경했습니다. 현재 {{count}}개가 선택되어 있습니다.", { count: response.emptyWorkspace.selectedIds.length })
              : t("후보 페이지를 갱신했습니다. 아래 카드에서 전체 검토 가능 목록을 확인하세요."))
          + " " + t("아직 휴지통으로 이동한 항목은 없습니다.")
        : formatAssistantPlainText(response.message);
      if (response.emptyWorkspace) {
        setEmptyWorkspace(response.emptyWorkspace);
        setFileWorkspace(null);
        setTrashResult(null);
        setActiveSession((current) => current?.session.id === sessionId
          ? { ...current, folderSummary: response.emptyWorkspace!.summary } : current);
      }
      if (response.fileWorkspace) {
        setFileWorkspace(response.fileWorkspace); setEmptyWorkspace(null); setTrashResult(null);
        if (response.fileWorkspace.query === null && response.fileWorkspace.currentPath === activeScope) setActiveSession((current) => current?.session.id === sessionId
          ? { ...current, folderSummary: response.fileWorkspace!.summary } : current);
      }
      setAppToolResults((response.appToolResults ?? []).slice(-4));
      setTurns((current) => [
        ...current,
        {
          role: "assistant",
          content: assistantMessage,
          providerLabel: automaticTrash || response.toolAction && !response.analysisComplete ? "BroomSweepy" : response.model ? `${response.label} · ${response.model}` : response.label,
        },
      ]);
      setDockerContext(response.dockerContext);
      setSavingResponse(true);
      try {
        const assistantMutation = await appendAssistantMessage({
          sessionId,
          role: "assistant",
          content: assistantMessage,
          provider: automaticTrash || response.toolAction && !response.analysisComplete ? null : response.provider,
          model: automaticTrash || response.toolAction && !response.analysisComplete ? null : response.model,
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
      // The original human request, never the model's reply, determines authority.
      // Ambiguous/multi-plan answers stay at the inline question even when opted in.
      if (automaticTrash) {
        if (explicitApp) onAppTrashBusyChange?.(true);
        setSavingResponse(false);
        setSessionBusy(true);
        setExecutingTrash(true);
        setAppToolResults(current => current.filter(result => result !== applicationReview
          && !(explicitFiles && files && workspaceReviewMatches(result, "files.workspace", files.revision))));
        if (explicitFiles) setFileWorkspace(null);
        try {
          const actual = explicitApp
            ? await confirmApplicationTrash(applicationQuestion!.plan.planId, true)
            : await onConfirmFilePlan(sessionId, files!.revision, files!.plan!.id, true, true);
          await reportTrashCompletion({ message: t("설정한 권한으로 요청 {{requested}}개 중 {{moved}}개를 휴지통으로 이동했습니다.", { requested: actual.requestedCount, moved: actual.movedCount }), trashResult: actual, mutated: true }, sessionId);
        } catch (reason) {
          await reportTrashCompletion({ message: t("작업을 완료하지 못했습니다. 대상과 작업 기록을 확인한 뒤 다시 요청하세요. {{detail}}", { detail: normalizeAssistantError(reason, t) }), mutated: true }, sessionId);
        } finally { setSessionBusy(false); setExecutingTrash(false); if (explicitApp) onAppTrashBusyChange?.(false); }
      }
    } catch (reason) {
      if (!userMessageSaved) {
        setTurns((current) => current.slice(0, -1));
        setDraft((current) => current.trim() ? current : message);
        setSessionError(normalizeAssistantError(reason, t));
      } else {
        const detail = normalizeAssistantError(reason, t);
        try { setEmptyWorkspace(await getAssistantEmptyWorkspace(sessionId)); } catch { setEmptyWorkspace(null); }
        try { setFileWorkspace(await getAssistantFileWorkspace(sessionId)); } catch { setFileWorkspace(null); }
        setProviderError(detail);
        if (isAssistantAuthenticationFailure(reason)) {
          setProviders((current) => current.map((candidate) => candidate.provider === selectedProviderKind
            ? { ...candidate, state: "loginRequired", authentication: "required", available: false, busy: false, detail }
            : candidate));
        }
      }
    } finally {
      activeProgressId.current = null;
      progressUnlisten.current?.();
      progressUnlisten.current = null;
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
    if (event.key !== "Enter" || event.shiftKey || event.nativeEvent.isComposing || sending) return;
    event.preventDefault();
    void submitQuestion();
  }

  return (
    <div className="assistant-workspace">
      <header className="assistant-heading">
      <h1>{t("대화")}</h1>
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
      </header>

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
        <button type="button" ref={connectionTrigger} className="assistant-connections-button" aria-label={t("연결과 권한")}
          title={t("연결과 권한")} onClick={() => setConnectionOpen(true)}>
          <Settings2 size={18} aria-hidden="true" />
        </button>
        <div
          className={`assistant-provider-picker ${provider?.available ? "is-ready" : ""}`}
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
        <div className="assistant-transcript" ref={transcript} tabIndex={0} aria-label={t("대화 기록")}
          onScroll={(event) => {
            const pane = event.currentTarget;
            const nearBottom = pane.scrollHeight - pane.scrollTop - pane.clientHeight < 80;
            followLatest.current = nearBottom;
            setShowLatest(!nearBottom);
          }}>
          <div className="assistant-transcript__content">
          {turns.length > 0 ? (
            turns.map((turn, index) => (
              <article
                className={`assistant-message is-${turn.role}`}
                key={turn.sequence !== undefined ? `${turn.role}-saved-${turn.sequence}` : `${turn.role}-pending-${index}`}
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
          {appToolResults.map((result, index) => <AssistantEvidence key={`${result.capability}-${result.capturedAtUnixMs}-${index}`}
            title={t(appToolTitles[result.capability] ?? "앱에서 확인한 결과")}
            status={t(appToolStatusKeys[result.status]) + (typeof (result.data.returnedCount ?? result.data.matchedCount) === "number"
              ? ` · ${t("{{count}}개", { count: Number(result.data.returnedCount ?? result.data.matchedCount) })}` : "")}
            forceOpen={result.status !== "completed"}
            warning={result.truncated || result.data.truncated === true || result.data.sourceMayBeIncomplete === true ? t("일부 결과만 표시합니다.") : undefined}>
            {applicationTrashQuestion(result) ? <AssistantApplicationConfirmation result={result} busy={sending || sessionBusy || cleanupAccessLocked}
              onDecision={confirmed => answerTrash(confirmed, undefined, result)} /> : <AssistantAppToolCard result={result} busy={sending || sessionBusy || cleanupAccessLocked}
              onView={onAppToolView} onReview={onAppToolReview && activeSession ? (prepared) => onAppToolReview(prepared, activeSession.session.id) : undefined} />
            }
          </AssistantEvidence>)}
          {emptyWorkspace ? <AssistantEvidence title={t("빈 폴더")} status={t("{{count}}개", { count: emptyWorkspace.candidates.length })}
            warning={emptyWorkspace.omittedCount > 0 ? t("일부 결과만 표시합니다.") : undefined}
            forceOpen={Boolean(emptyWorkspace.plan || emptyWorkspace.selectedIds.length)}>
            <AssistantEmptyFolderCard
            workspace={emptyWorkspace} busy={sending || sessionBusy || cleanupAccessLocked}
            onSelect={(ids) => void manageEmptyFolders("select", ids)}
            onPrepare={() => void manageEmptyFolders("prepare")}
            onConfirm={() => void manageEmptyFolders("confirm")}
          /></AssistantEvidence> : null}
          {fileWorkspace ? <AssistantEvidence title={t("앱에서 확인한 결과")} status={t("{{count}}개", { count: fileWorkspace.totalEntries })}
            warning={fileWorkspace.truncated || fileWorkspace.unreadableEntries > 0 ? t("일부 결과만 표시합니다.") : undefined}
            forceOpen={Boolean(fileWorkspace.plan || fileWorkspace.selectedIds.length)}>
            <AssistantFileCard workspace={fileWorkspace} busy={sending || sessionBusy || cleanupAccessLocked}
            onOpenCleanupTree={onOpenCleanupTree && activeSession ? () => onOpenCleanupTree(activeSession.session.id, fileWorkspace.revision) : undefined}
            onShowMap={() => void showFileMap()}
            onAction={(action) => void manageFiles(action)}
            onSelect={(ids) => void manageFiles("select", ids)}
            onPrepare={() => void manageFiles("prepare")}
            onConfirm={(nestedAck) => void manageFiles("confirm", undefined, nestedAck)}
          /></AssistantEvidence> : activeSession && activeScopeKind === "folder" && !emptyWorkspace ? <button type="button" className="assistant-query-action text-button"
            disabled={sending || sessionBusy || cleanupAccessLocked} onClick={() => void manageFiles({ kind: "scan" })}>
            <FolderOpen size={16} aria-hidden="true" />{t("대화 폴더 파일·폴더 검사")}
          </button> : null}
          {trashResult ? <AssistantTrashResultCard result={trashResult} /> : null}
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
        </div>
        <div className="assistant-dock">
        {showLatest ? <button type="button" className="assistant-latest" onClick={() => {
          followLatest.current = true;
          if (transcript.current) transcript.current.scrollTop = transcript.current.scrollHeight;
          setShowLatest(false);
        }}><ArrowDown size={16} aria-hidden="true" />{t("최신 메시지로")}</button> : null}
        {sessionError ? <p className="assistant-session-error" role="alert">{sessionError}</p> : null}
        {providerError ? <p className="assistant-chat__error" role="alert">{providerError}</p> : null}
        {provider && !provider.available ? <button type="button" className="assistant-connection-warning" onClick={() => setConnectionOpen(true)}>
          {providerOptionLabel(provider, t)} · {t("연결과 권한")}
        </button> : null}
        {sending || executingTrash ? <div className="assistant-working">
          <span role="status" aria-live="polite"><LoaderCircle className="is-spinning" size={16} aria-hidden="true" />
            {executingTrash ? t("대상을 다시 확인하고 처리 중…") : cancelling ? t("취소 요청 중…") : savingResponse ? t("대화 기록 저장 중…")
              : requestProgress?.phase === "querying"
                ? t("앱에서 {{task}} 확인 중", { task: t(appToolTitles[requestProgress.capability ?? ""] ?? "파일 검사") })
                : requestProgress?.phase === "analyzing"
                  ? requestProgress.round > 0 ? t("{{provider}} · 앱 결과 분석 중", { provider: provider?.label ?? "AI" })
                    : t("{{provider}} · 응답 대기 중", { provider: provider?.label ?? "AI" })
                  : t("질문 저장 및 CLI 준비 중…")}
          </span>
          <span className="assistant-working__time" aria-label={t("경과 시간")}>{elapsedLabel(requestStarted, clockNow)}</span>
        </div> : null}
        {emptyActionBusy ? <div className="assistant-working">
          <span role="status"><LoaderCircle className="is-spinning" size={16} aria-hidden="true" />{t("앱에서 후보 확인 또는 휴지통 이동을 처리하고 있습니다.")}</span>
          <button type="button" className="text-button" onClick={() => void cancelScan().catch((reason) => setSessionError(normalizeAssistantError(reason, t)))}>{t("작업 중단")}</button>
        </div> : null}

        <form className="assistant-composer" onSubmit={(event) => void submitQuestion(event)}>
          <textarea
            value={draft}
            maxLength={2_000}
            rows={1}
            name="assistantQuestion"
            autoComplete="off"
            disabled={!summary || !provider?.available || !providerModelReady || sessionBusy}
            aria-label={activeScopeKind === "docker" ? t("Docker 용량에 관해 질문") : t("선택한 폴더에 관해 질문")}
            placeholder={composerPlaceholder(
              sessionBusy,
              Boolean(activeSession),
              activeScopeKind,
              provider,
              selectedModel,
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
            <button type="submit" aria-label={t("질문 보내기")} disabled={(!ready && !canAnswerTrash) || !draft.trim()}>
              <Send size={18} aria-hidden="true" />
            </button>
          )}
        </form>
        <AssistantModelPicker provider={provider} value={selectedModel} onChange={changeModel}
          reasoningEffort={selectedReasoningEffort}
          onReasoningEffortChange={value => modelPreference.setReasoningEffort(selectedProviderKind, selectedModel, value)}
          busy={checkingProviders || sending || sessionBusy || Boolean(provider?.busy)} />
        {modelPreference.storageError ? <p className="assistant-model-storage-error" role="alert">
          {t("모델 선택을 저장하지 못했습니다. 현재 실행 중에는 선택한 모델을 사용합니다.")}
        </p> : null}
        <p className="assistant-composer-note">{t(controlSettings.status.chatTrashWithoutConfirmation ? "앱이 조회하고 AI가 분석합니다. 명확한 삭제 요청은 설정한 권한으로 처리합니다." : "앱이 조회하고 AI가 분석합니다. 실행은 별도 확인합니다.")}
          <button type="button" onClick={() => setConnectionOpen(true)}>{t("전송 범위")}</button>
        </p>
        </div>
      </section>
      <dialog className="assistant-connection-dialog" ref={connectionDialog} aria-labelledby="assistant-connection-title"
        onCancel={() => setConnectionOpen(false)} onClose={() => {
          setConnectionOpen(false);
          // WKWebView can return focus to its container instead of the opener.
          connectionTrigger.current?.focus();
        }}>
        <header>
          <ShieldCheck size={17} aria-hidden="true" />
          <h2 id="assistant-connection-title">{t("연결과 권한")}</h2>
          <button type="button" aria-label={t("닫기")} onClick={() => setConnectionOpen(false)}><X size={20} aria-hidden="true" /></button>
        </header>
        {connectionOpen ? <div className="assistant-connection-dialog__body">
        {provider ? <section className={`assistant-cli-diagnostics ${provider.available ? "is-ready" : ""}`} aria-label={t("CLI 연결 상태")}>
          <strong>{providerOptionLabel(provider, t)}</strong><p>{provider.detail}</p>
          {provider.executablePath ? <details><summary>{t("실행 경로와 버전")}</summary><code>{provider.executablePath}</code>
            <span>{t("CLI 버전: {{version}}", { version: provider.version ?? t("확인 불가") })}</span></details> : null}
        </section> : null}
        <div className="assistant-access-details__copy">
          <p>
            {activeScopeKind === "docker"
              ? t("Docker 조회는 BroomSweepy가 수행합니다. 앱은 범주별 사용량과 정리 가능 참고 상한만 선택한 AI CLI의 질문 입력으로 보냅니다.")
              : t("조회·검색·측정은 BroomSweepy가 수행합니다. AI는 제한된 실제 목록을 받아 분석하며, 허용한 문서 검색에서만 일치 본문 일부가 전달됩니다.")}
          </p>
          <p>
            {providerPermissionDetail(provider, t)} {t("아래 설정은 별도 터미널 제어용입니다.")}
          </p>
          <p>{activeScopeKind === "docker"
            ? t("폴더나 파일 내용이 아니라, BroomSweepy가 Docker CLI로 읽은 범주별 용량 요약만 {{provider}}에 전달합니다.", { provider: providerConversationLabel(provider, selectedModel, t) })
            : t("파일 검사는 로컬에서 처리합니다. {{provider}}에는 제한된 이름·크기·후보 요약과 질문·대화 기록이 전달됩니다. 직접 입력한 경로나 내용도 포함될 수 있습니다.", { provider: providerConversationLabel(provider, selectedModel, t) })}</p>
          <p>{t("로컬 문서 검색은 외부로 보내지 않습니다. AI 문서 검색을 허용하면 문서 이름과 일치 본문 일부가 선택한 AI에 전달됩니다.")}</p>
        </div>
        <ControlStatusPanel {...controlSettings} onReviewPending={() => { setConnectionOpen(false); controlSettings.onReviewPending(); }} />
        </div> : null}
      </dialog>

      <DockerCleanupDialog
        preview={dockerPreview}
        onClose={() => setDockerPreview(null)}
        onCompleted={updateDockerContext}
      />
    </div>
  );
}

function AssistantEvidence({ title, status, warning, forceOpen, children }: {
  title: string; status: string; warning?: string; forceOpen: boolean; children: ReactNode;
}) {
  const [open, setOpen] = useState(forceOpen);
  useEffect(() => { if (forceOpen) setOpen(true); }, [forceOpen]);
  return <section className={`assistant-evidence${open ? " is-open" : ""}`}>
    <button type="button" className="assistant-evidence__summary" aria-expanded={open}
      onClick={() => setOpen((current) => !current)}>
      <ChevronDown size={16} aria-hidden="true" /><strong>{title}</strong><span>{status}</span>
      {warning ? <small>{warning}</small> : null}
    </button>
    {open ? <div className="assistant-evidence__body">{children}</div> : null}
  </section>;
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
  return t(assistantProviderStatusKey(provider), { provider: provider.label, count: provider.models.length });
}

function providerPermissionDetail(provider: AssistantProviderStatus | null, t: Translate): string {
  switch (provider?.provider) {
    case "codex":
      return t("Codex는 앱 전용 빈 폴더에서 읽기 전용 샌드박스로 실행합니다. Codex 자체 읽기 도구의 실제 범위는 Codex 샌드박스 정책을 따릅니다.");
    case "claudeCode":
      return t("Claude Code는 세션 저장·도구·MCP·사용자 설정·훅을 끄고 실행합니다. CLI의 조직 관리 정책은 적용될 수 있습니다.");
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

function chooseInitialOllamaModel(models: AssistantProviderStatus["models"]): string {
  const conversational = models.find((model) => !/embed|^bge-/i.test(model.id));
  return conversational?.id ?? models[0]?.id ?? "";
}

function providerConversationLabel(
  provider: AssistantProviderStatus | null,
  model: string,
  t: Translate,
): string {
  if (!provider) return t("선택한 AI CLI");
  return assistantModelSelection(provider) !== "unsupported" && model
    ? `${provider.label} · ${model}`
    : provider.label;
}

function providerUnavailableMessage(provider: AssistantProviderStatus, t: Translate): string {
  if (provider.state === "notInstalled") return t("{{provider}}를 먼저 설치해 주세요…", { provider: provider.label });
  if (provider.state === "noModels") return t("Ollama에 대화용 모델을 먼저 설치해 주세요…");
  if (provider.state === "loginRequired") return t("{{provider}}에서 먼저 로그인해 주세요…", { provider: provider.label });
  return providerOptionLabel(provider, t);
}

function normalizeAssistantError(reason: unknown, t: Translate): string {
  return assistantFailureMessage(reason) ?? t("AI CLI 응답을 받지 못했습니다");
}

function fileWorkspaceMessage(workspace: AssistantFileWorkspace, t: Translate): string {
  const result = workspace.plan
    ? t("일반 파일·폴더 {{count}}개의 최종 검토를 준비했습니다. 정확한 경로와 포함 항목을 확인한 뒤 아래 버튼으로 휴지통에 보낼 수 있습니다.", { count: formatCount(workspace.plan.entries.length) })
    : workspace.query
      ? t("앱에서 이름 검색을 완료했습니다. 결과 {{count}}개를 아래 카드에서 확인하세요. 삭제할 항목을 선택해 최종 검토할 수 있습니다.", { count: formatCount(workspace.totalEntries) })
      : t("앱에서 파일·폴더 검사를 완료했습니다. 현재 목록 {{count}}개에서 하위 항목 확인·열기·선택 정리를 할 수 있습니다.", { count: formatCount(workspace.totalEntries) });
  const ranked = !workspace.plan && workspace.sizeRanked
    ? t("현재 폴더의 직계 항목을 용량순으로 검사했습니다. 폴더 용량은 하위 항목의 합계입니다.") + "\n"
      + workspace.entries.slice(0, 5).map((entry) => `${entry.number}. ${entry.name.slice(0, 240)} — ${formatBytes(entry.logicalBytes ?? 0)}`).join("\n")
      + "\n" + t("용량만으로 삭제 안전성을 판단할 수 없습니다. 하위 항목·필요 여부·백업을 먼저 확인하세요.")
    : result;
  return ranked + " " + (workspace.truncated ? t("검사 상한에 도달한 부분 결과입니다.") + " " : "")
    + (workspace.unreadableEntries ? t("읽지 못한 항목이 있어 검사 결과가 완전하지 않을 수 있습니다.") + " " : "")
    + t("아직 휴지통으로 이동한 항목은 없습니다.");
}
