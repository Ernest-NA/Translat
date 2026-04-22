import {
  useCallback,
  useEffect,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import type {
  DocumentSectionSummary,
  DocumentSummary,
  ExportReconstructedDocumentResult,
  GlossarySummary,
  ProjectSummary,
  RuleSetSummary,
  SegmentSummary,
  StyleProfileSummary,
  TranslateDocumentChunkResult,
  TranslationChunkSegmentSummary,
  TranslationChunkSummary,
  UpdateProjectEditorialDefaultsInput,
} from "../../shared/desktop";
import { useDocumentFindingReview } from "../hooks/useDocumentFindingReview";
import { useTranslateDocumentJob } from "../hooks/useTranslateDocumentJob";
import { useTranslationContextPreview } from "../hooks/useTranslationContextPreview";
import {
  DesktopCommandError,
  exportReconstructedDocument,
} from "../lib/desktop";
import { ChunkBrowser } from "./ChunkBrowser";
import { DocumentImporter } from "./DocumentImporter";
import { DocumentList } from "./DocumentList";
import { FindingReviewPanel } from "./FindingReviewPanel";
import { OperationalDebugPanel } from "./OperationalDebugPanel";
import { SegmentBrowser } from "./SegmentBrowser";
import { TranslationJobMonitor } from "./TranslationJobMonitor";
import { ActionButton } from "./ui/ActionButton";
import { PanelHeader } from "./ui/PanelHeader";
import { PanelMessage } from "./ui/PanelMessage";
import { StatusBadge } from "./ui/StatusBadge";

interface ProjectWorkspaceProps {
  activeDocument: DocumentSummary | null;
  chunkError: DesktopCommandError | null;
  chunkSegments: TranslationChunkSegmentSummary[];
  chunks: TranslationChunkSummary[];
  documents: DocumentSummary[];
  glossaries: GlossarySummary[];
  importError: DesktopCommandError | null;
  isBuildingChunks: boolean;
  isImportingDocuments: boolean;
  isLoadingDocuments: boolean;
  isLoadingChunks: boolean;
  isLoadingSegments: boolean;
  isSavingEditorialDefaults: boolean;
  loadError: DesktopCommandError | null;
  onBuildChunks: () => Promise<void>;
  onDirtyChange: (isDirty: boolean) => void;
  onOpenDocument: (documentId: string) => Promise<void>;
  onSyncDocumentState: (documentId: string) => Promise<void>;
  onImportDocuments: (files: FileList) => Promise<number>;
  onProcessDocument: (documentId: string) => Promise<void>;
  onSelectChunk: (chunkId: string | null) => void;
  onSaveEditorialDefaults: (
    input: UpdateProjectEditorialDefaultsInput,
  ) => Promise<boolean>;
  onSelectSection: (sectionId: string) => void;
  onSelectSegment: (segmentId: string) => void;
  processError: DesktopCommandError | null;
  processingDocumentId: string | null;
  project: ProjectSummary | null;
  projectError: DesktopCommandError | null;
  ruleSets: RuleSetSummary[];
  segmentError: DesktopCommandError | null;
  segmentLoadingDocumentId: string | null;
  sections: DocumentSectionSummary[];
  selectedChunk: TranslationChunkSummary | null;
  selectedChunkId: string | null;
  selectedChunkSegments: TranslationChunkSegmentSummary[];
  selectedSection: DocumentSectionSummary | null;
  segments: SegmentSummary[];
  selectedSegment: SegmentSummary | null;
  selectedSegmentId: string | null;
  showOperationalDebug?: boolean;
  styleProfiles: StyleProfileSummary[];
  viewMode?:
    | "document-workspace"
    | "operational-debug"
    | "translation-workspace"
    | "workspace";
}

function formatTimestamp(timestamp: number) {
  return new Date(timestamp * 1000).toLocaleString();
}

function normalizeSelectionValue(value?: string | null) {
  return value ?? "";
}

function toOptionalSelection(value: string) {
  return value.length > 0 ? value : undefined;
}

function formatStatusSuffix(status: "active" | "archived") {
  return status === "archived" ? " (archived)" : "";
}

function downloadExportedDocument(result: ExportReconstructedDocumentResult) {
  const blob = new Blob([result.content], { type: result.mimeType });
  const objectUrl = URL.createObjectURL(blob);
  const anchor = document.createElement("a");

  anchor.href = objectUrl;
  anchor.download = result.fileName;
  anchor.rel = "noopener";
  anchor.style.display = "none";
  document.body.append(anchor);
  anchor.click();
  anchor.remove();
  window.setTimeout(() => {
    URL.revokeObjectURL(objectUrl);
  }, 1_000);
}

function formatWorkspaceStatusLabel(
  status:
    | "blocked"
    | "empty"
    | "incidents"
    | "ready"
    | "review"
    | "running"
    | "stale",
) {
  switch (status) {
    case "blocked":
      return "Blocked";
    case "empty":
      return "No document";
    case "incidents":
      return "With incidents";
    case "review":
      return "Pending review";
    case "running":
      return "In progress";
    case "stale":
      return "Needs refresh";
    default:
      return "Ready";
  }
}

function getWorkspaceStatusTone(
  status:
    | "blocked"
    | "empty"
    | "incidents"
    | "ready"
    | "review"
    | "running"
    | "stale",
) {
  switch (status) {
    case "ready":
      return "success";
    case "review":
      return "info";
    case "running":
      return "info";
    case "blocked":
    case "stale":
      return "warning";
    case "incidents":
      return "danger";
    default:
      return "neutral";
  }
}

function getOperationalCountTone(count: number) {
  return count > 0 ? "warning" : "neutral";
}

function formatBytes(value: number) {
  if (value < 1024) {
    return `${value} B`;
  }

  if (value < 1024 * 1024) {
    return `${(value / 1024).toFixed(1)} KB`;
  }

  return `${(value / (1024 * 1024)).toFixed(1)} MB`;
}

function formatSourceKind(value: string) {
  return value === "local_file" ? "Local file" : value;
}

function formatChunkRole(role: TranslationChunkSegmentSummary["role"]) {
  if (role === "context_before") {
    return "Context before";
  }

  if (role === "context_after") {
    return "Context after";
  }

  return "Core";
}

function truncateText(value: string, maxLength = 120) {
  return value.length > maxLength
    ? `${value.slice(0, Math.max(0, maxLength - 3))}...`
    : value;
}

function formatChunkExecutionStatus(
  status: TranslateDocumentChunkResult["status"],
) {
  switch (status) {
    case "cancelled":
      return "Cancelled";
    case "completed":
      return "Completed";
    case "failed":
      return "Incident";
    case "running":
      return "Running";
    default:
      return "Pending";
  }
}

function getChunkStatusTone(status: TranslateDocumentChunkResult["status"]) {
  switch (status) {
    case "completed":
      return "success";
    case "running":
      return "info";
    case "cancelled":
      return "warning";
    case "failed":
      return "danger";
    default:
      return "neutral";
  }
}

function getDocumentStatusTone(status: DocumentSummary["status"]) {
  return status === "segmented" ? "success" : "warning";
}

export function ProjectWorkspace({
  activeDocument,
  chunkError,
  chunkSegments,
  chunks,
  documents,
  glossaries,
  importError,
  isBuildingChunks,
  isImportingDocuments,
  isLoadingDocuments,
  isLoadingChunks,
  isLoadingSegments,
  isSavingEditorialDefaults,
  loadError,
  onBuildChunks,
  onDirtyChange,
  onOpenDocument,
  onSyncDocumentState,
  onImportDocuments,
  onProcessDocument,
  onSelectChunk,
  onSaveEditorialDefaults,
  onSelectSection,
  onSelectSegment,
  processError,
  processingDocumentId,
  project,
  projectError,
  ruleSets,
  segmentError,
  segmentLoadingDocumentId,
  sections,
  selectedChunk,
  selectedChunkId,
  selectedChunkSegments,
  selectedSection,
  segments,
  selectedSegment,
  selectedSegmentId,
  showOperationalDebug = true,
  styleProfiles,
  viewMode = "workspace",
}: ProjectWorkspaceProps) {
  const [draftDefaultGlossaryId, setDraftDefaultGlossaryId] = useState(() =>
    normalizeSelectionValue(project?.defaultGlossaryId),
  );
  const [draftDefaultStyleProfileId, setDraftDefaultStyleProfileId] = useState(
    () => normalizeSelectionValue(project?.defaultStyleProfileId),
  );
  const [draftDefaultRuleSetId, setDraftDefaultRuleSetId] = useState(() =>
    normalizeSelectionValue(project?.defaultRuleSetId),
  );
  const previousProjectIdRef = useRef<string | null>(null);
  const pendingDefaultsSyncRef =
    useRef<UpdateProjectEditorialDefaultsInput | null>(null);
  const currentActiveDocumentIdRef = useRef<string | null>(
    activeDocument?.id ?? null,
  );
  const [exportError, setExportError] = useState<DesktopCommandError | null>(
    null,
  );
  const [isExportingDocument, setIsExportingDocument] = useState(false);
  const [lastExport, setLastExport] =
    useState<ExportReconstructedDocumentResult | null>(null);

  useEffect(() => {
    currentActiveDocumentIdRef.current = activeDocument?.id ?? null;
  }, [activeDocument?.id]);

  useEffect(() => {
    const activeDocumentId = activeDocument?.id ?? null;
    const projectId = project?.id ?? null;

    void activeDocumentId;
    void projectId;
    setExportError(null);
    setIsExportingDocument(false);
    setLastExport(null);
  }, [activeDocument?.id, project?.id]);

  useEffect(() => {
    const nextProjectId = project?.id ?? null;
    const hasProjectChanged = previousProjectIdRef.current !== nextProjectId;
    const pendingDefaultsSync = pendingDefaultsSyncRef.current;
    const matchesPendingDefaultsSync =
      !!project &&
      pendingDefaultsSync?.projectId === project.id &&
      normalizeSelectionValue(pendingDefaultsSync.defaultGlossaryId) ===
        normalizeSelectionValue(project.defaultGlossaryId) &&
      normalizeSelectionValue(pendingDefaultsSync.defaultStyleProfileId) ===
        normalizeSelectionValue(project.defaultStyleProfileId) &&
      normalizeSelectionValue(pendingDefaultsSync.defaultRuleSetId) ===
        normalizeSelectionValue(project.defaultRuleSetId);

    if (!hasProjectChanged && !matchesPendingDefaultsSync) {
      return;
    }

    setDraftDefaultGlossaryId(
      normalizeSelectionValue(project?.defaultGlossaryId),
    );
    setDraftDefaultStyleProfileId(
      normalizeSelectionValue(project?.defaultStyleProfileId),
    );
    setDraftDefaultRuleSetId(
      normalizeSelectionValue(project?.defaultRuleSetId),
    );

    if (matchesPendingDefaultsSync) {
      pendingDefaultsSyncRef.current = null;
    }

    previousProjectIdRef.current = nextProjectId;
  }, [project]);

  const isDirty = useMemo(() => {
    if (!project) {
      return false;
    }

    return (
      draftDefaultGlossaryId !==
        normalizeSelectionValue(project.defaultGlossaryId) ||
      draftDefaultStyleProfileId !==
        normalizeSelectionValue(project.defaultStyleProfileId) ||
      draftDefaultRuleSetId !==
        normalizeSelectionValue(project.defaultRuleSetId)
    );
  }, [
    draftDefaultGlossaryId,
    draftDefaultRuleSetId,
    draftDefaultStyleProfileId,
    project,
  ]);

  useLayoutEffect(() => {
    onDirtyChange(isDirty);
  }, [isDirty, onDirtyChange]);

  const defaultGlossary = useMemo(
    () =>
      glossaries.find(
        (glossary) => glossary.id === project?.defaultGlossaryId,
      ) ?? null,
    [glossaries, project?.defaultGlossaryId],
  );
  const defaultStyleProfile = useMemo(
    () =>
      styleProfiles.find(
        (styleProfile) => styleProfile.id === project?.defaultStyleProfileId,
      ) ?? null,
    [project?.defaultStyleProfileId, styleProfiles],
  );
  const defaultRuleSet = useMemo(
    () =>
      ruleSets.find((ruleSet) => ruleSet.id === project?.defaultRuleSetId) ??
      null,
    [project?.defaultRuleSetId, ruleSets],
  );
  const editorialDefaultsFingerprint = useMemo(
    () =>
      [
        project?.defaultGlossaryId ?? "",
        project?.defaultStyleProfileId ?? "",
        project?.defaultRuleSetId ?? "",
        ...glossaries.map(
          (glossary) =>
            `${glossary.id}:${glossary.updatedAt}:${glossary.status}`,
        ),
        ...styleProfiles.map(
          (styleProfile) =>
            `${styleProfile.id}:${styleProfile.updatedAt}:${styleProfile.status}`,
        ),
        ...ruleSets.map(
          (ruleSet) => `${ruleSet.id}:${ruleSet.updatedAt}:${ruleSet.status}`,
        ),
      ].join(":"),
    [
      glossaries,
      project?.defaultGlossaryId,
      project?.defaultRuleSetId,
      project?.defaultStyleProfileId,
      ruleSets,
      styleProfiles,
    ],
  );
  const syncActiveDocumentState = useCallback(
    async (documentId: string) => {
      if (currentActiveDocumentIdRef.current !== documentId) {
        return;
      }

      await onSyncDocumentState(documentId);
    },
    [onSyncDocumentState],
  );
  const isTranslationWorkspace = viewMode === "translation-workspace";
  const isOperationalDebug = viewMode === "operational-debug";
  const {
    cancelJob,
    clearTrackedJob,
    error: translateJobError,
    isCancelling,
    isRefreshing,
    isRestoringTrackedJob,
    isResuming,
    isStarting,
    jobStatus,
    refreshStatus,
    resumeTranslation,
    startTranslation,
    trackedJobId,
  } = useTranslateDocumentJob({
    activeDocument,
    activeProjectId: project?.id ?? null,
    chunks,
    onDocumentStateSync: syncActiveDocumentState,
  });
  const activeDocumentTrackingKey =
    project && activeDocument ? `${project.id}:${activeDocument.id}` : null;
  const [documentFindingTrackingKey, setDocumentFindingTrackingKey] = useState<
    string | null
  >(null);
  const shouldKeepFindingReviewLive =
    Boolean(trackedJobId) || Boolean(jobStatus);

  useEffect(() => {
    if (!activeDocumentTrackingKey) {
      setDocumentFindingTrackingKey(null);
      return;
    }

    if (
      isTranslationWorkspace ||
      isOperationalDebug ||
      shouldKeepFindingReviewLive
    ) {
      setDocumentFindingTrackingKey(activeDocumentTrackingKey);
    }
  }, [
    activeDocumentTrackingKey,
    isOperationalDebug,
    isTranslationWorkspace,
    shouldKeepFindingReviewLive,
  ]);

  const shouldTrackDocumentFindings =
    activeDocumentTrackingKey !== null &&
    (documentFindingTrackingKey === activeDocumentTrackingKey ||
      isTranslationWorkspace ||
      isOperationalDebug ||
      shouldKeepFindingReviewLive);
  const shouldLoadSelectedChunkContext = isTranslationWorkspace;
  const {
    error: contextPreviewError,
    isLoading: isLoadingContextPreview,
    preview: contextPreview,
  } = useTranslationContextPreview({
    activeDocument,
    activeProjectId: project?.id ?? null,
    editorialDefaultsFingerprint,
    enabled: shouldLoadSelectedChunkContext,
    selectedChunk,
  });
  const {
    actionError: findingActionError,
    findings,
    inspection: findingInspection,
    inspectionError: findingInspectionError,
    isInspectingFinding,
    isLoadingFindings,
    isRetranslating,
    lastRetranslation,
    loadError: findingLoadError,
    refreshWarning,
    retranslateSelectedFinding,
    selectedFinding,
    selectedFindingId,
    selectFinding,
  } = useDocumentFindingReview({
    activeDocument,
    activeProjectId: project?.id ?? null,
    enabled: shouldTrackDocumentFindings,
    onRefreshDocument: syncActiveDocumentState,
    onSelectChunk,
  });
  const workspaceState = useMemo(() => {
    if (!activeDocument) {
      return {
        detail:
          "Select a document to evaluate readiness, launch document translation, and inspect chunk execution.",
        state: "empty" as const,
      };
    }

    if (activeDocument.status !== "segmented") {
      return {
        detail:
          "This document must be segmented before the translation workspace can load chunks or launch document translation.",
        state: "blocked" as const,
      };
    }

    if (chunks.length === 0) {
      return {
        detail:
          "The document is segmented but still needs persisted translation chunks. Build chunks to unlock translate_document.",
        state: "blocked" as const,
      };
    }

    if (trackedJobId && !jobStatus) {
      return {
        detail:
          "A tracked translate_document job exists for this document, but this session still needs a status refresh to recover its progress.",
        state: "stale" as const,
      };
    }

    switch (jobStatus?.status) {
      case "running":
      case "pending":
        return {
          detail:
            "Document translation is active. Keep the chunk detail open to inspect context, results, and incidents as the job advances.",
          state: "running" as const,
        };
      case "completed":
        return {
          detail:
            "The tracked document job completed successfully. The document is ready for result inspection and review handoff.",
          state: "review" as const,
        };
      case "completed_with_errors":
        return {
          detail:
            "The last document run completed with incidents. Failed or cancelled chunks remain visible and resumable from the job monitor.",
          state: "incidents" as const,
        };
      case "cancelled":
      case "failed":
        return {
          detail:
            "The tracked document job stopped before completion. Resume it to continue only the unresolved chunks without reopening the workflow.",
          state: "incidents" as const,
        };
      default:
        return {
          detail:
            "The document has persisted chunks and can launch translate_document directly from this workspace header.",
          state: "ready" as const,
        };
    }
  }, [activeDocument, chunks.length, jobStatus, trackedJobId]);
  const canLaunchTranslation =
    Boolean(activeDocument) &&
    activeDocument?.status === "segmented" &&
    chunks.length > 0 &&
    jobStatus?.status !== "pending" &&
    jobStatus?.status !== "running" &&
    (trackedJobId === null || jobStatus !== null) &&
    !isRestoringTrackedJob &&
    !isStarting &&
    !isResuming;
  const canResumeTranslation =
    trackedJobId !== null &&
    (jobStatus?.status === "cancelled" ||
      jobStatus?.status === "completed_with_errors" ||
      jobStatus?.status === "failed") &&
    !isStarting &&
    !isCancelling &&
    !isRestoringTrackedJob &&
    !isResuming;
  const canExportDocument =
    Boolean(project) &&
    Boolean(activeDocument) &&
    activeDocument?.status === "segmented" &&
    !isExportingDocument;
  const handleBuildChunks = useCallback(async () => {
    await onBuildChunks();
  }, [onBuildChunks]);
  const handleExportDocument = useCallback(async () => {
    if (!project || !activeDocument) {
      return;
    }

    const requestedDocumentId = activeDocument.id;

    setIsExportingDocument(true);
    setExportError(null);

    try {
      const exportedDocument = await exportReconstructedDocument({
        projectId: project.id,
        documentId: activeDocument.id,
      });

      downloadExportedDocument(exportedDocument);

      if (currentActiveDocumentIdRef.current === requestedDocumentId) {
        setLastExport(exportedDocument);
      }
    } catch (caughtError) {
      if (currentActiveDocumentIdRef.current === requestedDocumentId) {
        setExportError(
          caughtError instanceof DesktopCommandError
            ? caughtError
            : new DesktopCommandError(
                "export_reconstructed_document" as never,
                {
                  code: "UNEXPECTED_DESKTOP_ERROR",
                  message:
                    "The desktop shell could not export the reconstructed document.",
                },
              ),
        );
      }
    } finally {
      if (currentActiveDocumentIdRef.current === requestedDocumentId) {
        setIsExportingDocument(false);
      }
    }
  }, [activeDocument, project]);
  const disableChunkBuildActions =
    isLoadingChunks ||
    isRestoringTrackedJob ||
    isStarting ||
    isResuming ||
    isCancelling ||
    jobStatus?.status === "pending" ||
    jobStatus?.status === "running";
  const chunkStatusLookup = useMemo(
    () =>
      new Map(
        (jobStatus?.chunkStatuses ?? []).map((chunkStatus) => [
          chunkStatus.chunkId,
          chunkStatus,
        ]),
      ),
    [jobStatus?.chunkStatuses],
  );
  const segmentLookup = useMemo(
    () => new Map(segments.map((segment) => [segment.id, segment])),
    [segments],
  );
  const chunkCoreSegmentCounts = useMemo(() => {
    const counts = new Map<string, number>();

    for (const chunkSegment of chunkSegments) {
      if (chunkSegment.role !== "core") {
        continue;
      }

      counts.set(
        chunkSegment.chunkId,
        (counts.get(chunkSegment.chunkId) ?? 0) + 1,
      );
    }

    return counts;
  }, [chunkSegments]);
  const selectedChunkStatus = selectedChunk
    ? (chunkStatusLookup.get(selectedChunk.id) ?? null)
    : null;
  const selectedCoreSegments = selectedChunkSegments
    .filter((chunkSegment) => chunkSegment.role === "core")
    .map((chunkSegment) => segmentLookup.get(chunkSegment.segmentId) ?? null)
    .filter((segment): segment is SegmentSummary => segment !== null);
  const selectedCoreChunkSegmentCount = selectedChunk
    ? (chunkCoreSegmentCounts.get(selectedChunk.id) ??
      selectedCoreSegments.length)
    : 0;
  const orderedChunks = useMemo(() => {
    return [...chunks].sort((left, right) => {
      const leftStatus = chunkStatusLookup.get(left.id)?.status ?? "pending";
      const rightStatus = chunkStatusLookup.get(right.id)?.status ?? "pending";
      const leftHasIncident =
        leftStatus === "failed" || leftStatus === "cancelled";
      const rightHasIncident =
        rightStatus === "failed" || rightStatus === "cancelled";

      if (leftHasIncident !== rightHasIncident) {
        return leftHasIncident ? -1 : 1;
      }

      return left.sequence - right.sequence;
    });
  }, [chunkStatusLookup, chunks]);
  const runningJob =
    jobStatus?.status === "pending" || jobStatus?.status === "running";
  const translatedCoreCount = selectedCoreSegments.filter(
    (segment) => segment.status === "translated" || segment.targetText,
  ).length;
  const exportReady =
    activeDocument?.status === "segmented" &&
    chunks.length > 0 &&
    (jobStatus?.status === "completed" ||
      jobStatus?.status === "completed_with_errors" ||
      selectedCoreSegments.some((segment) => segment.targetText));

  async function handleSaveEditorialDefaults(
    event: React.FormEvent<HTMLFormElement>,
  ) {
    event.preventDefault();

    if (!project) {
      return;
    }

    const changes: UpdateProjectEditorialDefaultsInput = {
      projectId: project.id,
      defaultGlossaryId: toOptionalSelection(draftDefaultGlossaryId),
      defaultStyleProfileId: toOptionalSelection(draftDefaultStyleProfileId),
      defaultRuleSetId: toOptionalSelection(draftDefaultRuleSetId),
    };

    pendingDefaultsSyncRef.current = changes;

    const wasSaved = await onSaveEditorialDefaults(changes);

    if (!wasSaved) {
      pendingDefaultsSyncRef.current = null;
    }
  }

  if (viewMode === "operational-debug") {
    return (
      <OperationalDebugPanel
        activeDocument={activeDocument}
        activeProjectId={project?.id ?? null}
        trackedJobId={trackedJobId}
      />
    );
  }

  if (!project) {
    return (
      <section className="surface-card surface-card--accent">
        <PanelHeader
          description="Select a persisted project or create a new one. Document intake and editorial defaults only become active after a workspace has been explicitly selected."
          eyebrow="Workspace"
          title="No project open yet."
          titleLevel={2}
        />

        <ul className="readiness-list">
          <li>Projects are persisted in the encrypted SQLite database.</li>
          <li>The active project selection survives app restarts.</li>
          <li>Each project can keep explicit default editorial artifacts.</li>
        </ul>
      </section>
    );
  }

  if (viewMode === "translation-workspace") {
    return (
      <section className="translation-workspace">
        <section
          className="workspace-panel translation-workspace-hero"
          data-state={workspaceState.state}
        >
          <PanelHeader
            actions={
              <div className="translation-workspace-header__actions">
                {chunks.length === 0 ? (
                  <ActionButton
                    disabled={
                      activeDocument?.status !== "segmented" ||
                      disableChunkBuildActions ||
                      isBuildingChunks
                    }
                    onClick={() => void handleBuildChunks()}
                    size="md"
                    variant="primary"
                  >
                    {isBuildingChunks ? "Building chunks..." : "Build chunks"}
                  </ActionButton>
                ) : null}
                {chunks.length > 0 && !runningJob && !canResumeTranslation ? (
                  <ActionButton
                    disabled={!canLaunchTranslation}
                    onClick={() => void startTranslation()}
                    size="md"
                    variant="primary"
                  >
                    {isRestoringTrackedJob
                      ? "Restoring job..."
                      : isStarting
                        ? "Launching..."
                        : "Translate document"}
                  </ActionButton>
                ) : null}
                {chunks.length > 0 ? (
                  <ActionButton
                    disabled={
                      activeDocument?.status !== "segmented" ||
                      disableChunkBuildActions ||
                      isBuildingChunks
                    }
                    onClick={() => void handleBuildChunks()}
                    size="md"
                    variant="ghost"
                  >
                    {isBuildingChunks
                      ? "Rebuilding chunks..."
                      : "Rebuild chunks"}
                  </ActionButton>
                ) : null}
                {runningJob ? (
                  <ActionButton
                    disabled={isRefreshing}
                    onClick={() => void refreshStatus()}
                    size="md"
                    variant="secondary"
                  >
                    {isRefreshing ? "Refreshing..." : "Refresh progress"}
                  </ActionButton>
                ) : null}
                {canResumeTranslation ? (
                  <ActionButton
                    disabled={isResuming}
                    onClick={() => void resumeTranslation()}
                    size="md"
                    variant="primary"
                  >
                    {isResuming ? "Resuming..." : "Resume incidents"}
                  </ActionButton>
                ) : null}
                <ActionButton
                  disabled={!canExportDocument}
                  onClick={() => void handleExportDocument()}
                  size="md"
                  variant={exportReady ? "secondary" : "ghost"}
                >
                  {isExportingDocument ? "Exporting..." : "Export markdown"}
                </ActionButton>
              </div>
            }
            eyebrow="Translation workspace"
            meta={
              <StatusBadge
                emphasis="strong"
                size="md"
                tone={getWorkspaceStatusTone(workspaceState.state)}
              >
                {formatWorkspaceStatusLabel(workspaceState.state)}
              </StatusBadge>
            }
            title={
              activeDocument
                ? activeDocument.name
                : "Select a document to translate"
            }
            titleLevel={2}
          />

          <p className="surface-card__copy">{workspaceState.detail}</p>

          {exportError ? (
            <PanelMessage role="alert" tone="danger">
              {exportError.message}
            </PanelMessage>
          ) : null}

          {lastExport ? (
            <PanelMessage tone="success">
              Exported <strong>{lastExport.fileName}</strong> from the current{" "}
              {lastExport.status} reconstructed document snapshot.
            </PanelMessage>
          ) : null}

          <div className="translation-workspace__summary">
            <div>
              <span>Document</span>
              <strong>{activeDocument?.status ?? "No document"}</strong>
            </div>
            <div>
              <span>Chunks</span>
              <strong>{chunks.length}</strong>
            </div>
            <div>
              <span>Progress</span>
              <strong>
                {jobStatus
                  ? `${jobStatus.completedChunks}/${jobStatus.totalChunks}`
                  : "No job"}
              </strong>
            </div>
            <div>
              <span>Findings</span>
              <strong>{findings.length}</strong>
            </div>
            <div>
              <span>Export</span>
              <strong>{exportReady ? "Ready" : "Not ready"}</strong>
            </div>
          </div>
        </section>

        <div className="translation-workspace__grid">
          <aside className="translation-workspace__left-rail">
            <section className="workspace-panel translation-document-rail">
              <PanelHeader
                eyebrow="Documents"
                meta={
                  <StatusBadge size="sm" tone="info">
                    {documents.length} total
                  </StatusBadge>
                }
                title="Active input"
              />

              {activeDocument && isLoadingSegments ? (
                <PanelMessage tone="info">
                  Loading segment trace for {activeDocument.name}...
                </PanelMessage>
              ) : null}

              {segmentError ? (
                <PanelMessage role="alert" tone="danger">
                  {segmentError.message}
                </PanelMessage>
              ) : null}

              {activeDocument &&
              !isLoadingSegments &&
              !segmentError &&
              activeDocument.status === "segmented" &&
              segments.length === 0 ? (
                <PanelMessage tone="warning">
                  This document is segmented, but no segment trace is currently
                  loaded.
                </PanelMessage>
              ) : null}

              {documents.length > 0 ? (
                <ol className="translation-document-list">
                  {documents.map((document) => (
                    <li key={document.id}>
                      <button
                        className="translation-document-row"
                        data-active={document.id === activeDocument?.id}
                        data-state={document.status}
                        disabled={document.status !== "segmented"}
                        onClick={() => void onOpenDocument(document.id)}
                        title={
                          document.status === "segmented"
                            ? `Open ${document.name}`
                            : "Segment this document in Documents before translating it."
                        }
                        type="button"
                      >
                        <span>
                          <strong>{document.name}</strong>
                          <small>
                            {document.format.toUpperCase()} |{" "}
                            {formatSourceKind(document.sourceKind)} |{" "}
                            {formatBytes(document.fileSizeBytes)}
                          </small>
                        </span>
                        <StatusBadge
                          tone={getDocumentStatusTone(document.status)}
                        >
                          {document.status}
                        </StatusBadge>
                      </button>
                    </li>
                  ))}
                </ol>
              ) : (
                <PanelMessage>
                  Import a document in Documents before opening Translation.
                </PanelMessage>
              )}
            </section>

            <section className="workspace-panel translation-chunk-rail">
              <PanelHeader
                eyebrow="Chunks"
                meta={
                  <StatusBadge size="sm" tone="info">
                    {chunks.length} loaded
                  </StatusBadge>
                }
                title="Execution order"
              />

              {!activeDocument ? (
                <PanelMessage>Select a document to load chunks.</PanelMessage>
              ) : null}

              {activeDocument && isLoadingChunks ? (
                <PanelMessage tone="info">Loading chunks...</PanelMessage>
              ) : null}

              {chunkError ? (
                <PanelMessage role="alert" tone="danger">
                  {chunkError.message}
                </PanelMessage>
              ) : null}

              {activeDocument && !isLoadingChunks && chunks.length === 0 ? (
                <PanelMessage>
                  Build chunks before launching document translation.
                </PanelMessage>
              ) : null}

              {chunks.length > 0 ? (
                <ol className="translation-chunk-list">
                  {orderedChunks.map((chunk) => {
                    const status = chunkStatusLookup.get(chunk.id);
                    const statusName: TranslateDocumentChunkResult["status"] =
                      status?.status ?? "pending";
                    const coreSegmentCount =
                      chunkCoreSegmentCounts.get(chunk.id) ??
                      chunk.segmentCount;

                    return (
                      <li key={chunk.id}>
                        <button
                          className="translation-chunk-row"
                          data-active={chunk.id === selectedChunkId}
                          data-state={statusName}
                          onClick={() => onSelectChunk(chunk.id)}
                          type="button"
                        >
                          <span className="translation-chunk-row__index">
                            {chunk.sequence}
                          </span>
                          <span>
                            <strong>
                              #{chunk.startSegmentSequence}-#
                              {chunk.endSegmentSequence}
                            </strong>
                            <small>
                              {status?.translatedSegmentCount ?? 0}/
                              {coreSegmentCount} translated |{" "}
                              {chunk.sourceWordCount} words
                            </small>
                            {status?.errorMessage ? (
                              <em>{truncateText(status.errorMessage, 96)}</em>
                            ) : null}
                          </span>
                          <StatusBadge tone={getChunkStatusTone(statusName)}>
                            {formatChunkExecutionStatus(statusName)}
                          </StatusBadge>
                        </button>
                      </li>
                    );
                  })}
                </ol>
              ) : null}
            </section>
          </aside>

          <section className="workspace-panel translation-chunk-detail">
            {selectedChunk ? (
              <>
                <PanelHeader
                  eyebrow="Selected chunk"
                  meta={
                    <div className="chunk-detail__heading-badges">
                      <StatusBadge
                        tone={getChunkStatusTone(
                          selectedChunkStatus?.status ?? "pending",
                        )}
                      >
                        {formatChunkExecutionStatus(
                          selectedChunkStatus?.status ?? "pending",
                        )}
                      </StatusBadge>
                      <StatusBadge tone="info">
                        {translatedCoreCount}/{selectedCoreChunkSegmentCount}{" "}
                        result
                      </StatusBadge>
                    </div>
                  }
                  title={`Chunk #${selectedChunk.sequence}`}
                />

                <div className="translation-chunk-detail__meta">
                  <span>
                    Segments #{selectedChunk.startSegmentSequence}-#
                    {selectedChunk.endSegmentSequence}
                  </span>
                  <span>{selectedChunk.sourceWordCount} words</span>
                  <span>{selectedChunk.strategy}</span>
                </div>

                {selectedChunkStatus?.errorMessage ? (
                  <PanelMessage role="alert" tone="danger" title="Incident">
                    {selectedChunkStatus.errorMessage}
                  </PanelMessage>
                ) : null}

                <div className="translation-chunk-detail__sections">
                  <section>
                    <p className="surface-card__eyebrow">Source</p>
                    <div className="segment-detail__text">
                      {selectedChunk.sourceText}
                    </div>
                  </section>

                  <section>
                    <p className="surface-card__eyebrow">Context</p>
                    <div className="translation-context-stack">
                      <div className="segment-detail__text segment-detail__text--muted">
                        <strong>Before</strong>
                        <p>
                          {selectedChunk.contextBeforeText ??
                            "No prior overlap segment."}
                        </p>
                      </div>
                      <div className="segment-detail__text segment-detail__text--muted">
                        <strong>After</strong>
                        <p>
                          {selectedChunk.contextAfterText ??
                            "No trailing overlap segment."}
                        </p>
                      </div>
                    </div>

                    {isLoadingContextPreview ? (
                      <PanelMessage tone="info">
                        Loading context...
                      </PanelMessage>
                    ) : null}

                    {contextPreviewError ? (
                      <PanelMessage role="alert" tone="danger">
                        {contextPreviewError.message}
                      </PanelMessage>
                    ) : null}

                    {contextPreview ? (
                      <dl className="detail-list detail-list--single">
                        <div>
                          <dt>Section</dt>
                          <dd>
                            {contextPreview.chunkContext.section?.title ??
                              "No matched section"}
                          </dd>
                        </div>
                        <div>
                          <dt>Glossary layers</dt>
                          <dd>{contextPreview.glossaryLayers.length}</dd>
                        </div>
                        <div>
                          <dt>Rules</dt>
                          <dd>{contextPreview.rules.length}</dd>
                        </div>
                        <div>
                          <dt>Style</dt>
                          <dd>
                            {contextPreview.styleProfile?.styleProfile.name ??
                              "No style profile"}
                          </dd>
                        </div>
                      </dl>
                    ) : null}
                  </section>

                  <section>
                    <p className="surface-card__eyebrow">Result</p>
                    {selectedCoreSegments.length > 0 ? (
                      <ol className="chunk-result-list">
                        {selectedCoreSegments.map((segment) => (
                          <li
                            className="chunk-result-list__item"
                            key={segment.id}
                          >
                            <div className="chunk-link-list__heading">
                              <strong>Segment #{segment.sequence}</strong>
                              <StatusBadge
                                tone={
                                  segment.status === "translated"
                                    ? "success"
                                    : "warning"
                                }
                              >
                                {segment.status}
                              </StatusBadge>
                            </div>
                            <div className="chunk-result-list__texts">
                              <div>
                                <p className="surface-card__eyebrow">Source</p>
                                <div className="segment-detail__text">
                                  {segment.sourceText}
                                </div>
                              </div>
                              <div>
                                <p className="surface-card__eyebrow">Target</p>
                                <div className="segment-detail__text segment-detail__text--muted">
                                  {segment.targetText ??
                                    "No translated target text persisted yet."}
                                </div>
                              </div>
                            </div>
                          </li>
                        ))}
                      </ol>
                    ) : (
                      <PanelMessage>
                        No core segment trace is loaded for this chunk yet.
                      </PanelMessage>
                    )}
                  </section>

                  <section>
                    <p className="surface-card__eyebrow">Segment trace</p>
                    <ol className="chunk-link-list">
                      {selectedChunkSegments.map((chunkSegment) => {
                        const segment =
                          segmentLookup.get(chunkSegment.segmentId) ?? null;

                        return (
                          <li
                            className="chunk-link-list__item"
                            key={`${chunkSegment.chunkId}:${chunkSegment.segmentId}:${chunkSegment.role}`}
                          >
                            <div className="chunk-link-list__heading">
                              <strong>
                                #{chunkSegment.segmentSequence}{" "}
                                {formatChunkRole(chunkSegment.role)}
                              </strong>
                              <StatusBadge tone="info">
                                pos {chunkSegment.position}
                              </StatusBadge>
                            </div>
                            <p>
                              {segment
                                ? truncateText(segment.sourceText, 180)
                                : chunkSegment.segmentId}
                            </p>
                          </li>
                        );
                      })}
                    </ol>
                  </section>
                </div>
              </>
            ) : (
              <PanelMessage>
                Select a chunk to inspect source, context, result, and incident
                state.
              </PanelMessage>
            )}
          </section>

          <aside className="translation-workspace__right-rail">
            <TranslationJobMonitor
              activeDocument={activeDocument}
              error={translateJobError}
              isCancelling={isCancelling}
              isRefreshing={isRefreshing}
              isRestoringTrackedJob={isRestoringTrackedJob}
              isResuming={isResuming}
              jobStatus={jobStatus}
              onCancelJob={cancelJob}
              onClearTrackedJob={clearTrackedJob}
              onRefreshStatus={() => refreshStatus()}
              onResumeTranslation={resumeTranslation}
              trackedJobId={trackedJobId}
            />

            <FindingReviewPanel
              actionError={findingActionError}
              activeDocument={activeDocument}
              findings={findings}
              inspection={findingInspection}
              inspectionError={findingInspectionError}
              isInspectingFinding={isInspectingFinding}
              isLoadingFindings={isLoadingFindings}
              isRetranslating={isRetranslating}
              lastRetranslation={lastRetranslation}
              loadError={findingLoadError}
              refreshWarning={refreshWarning}
              onRetranslateSelectedFinding={retranslateSelectedFinding}
              onSelectFinding={selectFinding}
              selectedFinding={selectedFinding}
              selectedFindingId={selectedFindingId}
            />
          </aside>
        </div>
      </section>
    );
  }

  return (
    <section className="surface-card surface-card--accent">
      <PanelHeader
        eyebrow="Open workspace"
        title={project.name}
        titleLevel={2}
      />
      <p className="surface-card__copy">
        {project.description ??
          "This project has no description yet. It is ready to receive imported documents and keep a reusable editorial baseline visible from the same workspace."}
      </p>

      <dl className="detail-list">
        <div>
          <dt>Project ID</dt>
          <dd>{project.id}</dd>
        </div>
        <div>
          <dt>Created</dt>
          <dd>{formatTimestamp(project.createdAt)}</dd>
        </div>
        <div>
          <dt>Last opened</dt>
          <dd>{formatTimestamp(project.lastOpenedAt)}</dd>
        </div>
        <div>
          <dt>Updated</dt>
          <dd>{formatTimestamp(project.updatedAt)}</dd>
        </div>
      </dl>

      <section className="workspace-panel project-editorial-defaults">
        <PanelHeader
          description="D5 keeps the project baseline explicit and persisted. Each project can point to zero or one default glossary, style profile, and rule set without adding precedence logic or automatic AI usage."
          eyebrow="Editorial defaults"
          title="Associate one glossary, style profile, and rule set by default"
        />

        {projectError ? (
          <PanelMessage role="alert" tone="danger">
            {projectError.message}
          </PanelMessage>
        ) : null}

        <dl className="detail-list detail-list--single">
          <div>
            <dt>Default glossary</dt>
            <dd>
              {defaultGlossary
                ? `${defaultGlossary.name}${formatStatusSuffix(defaultGlossary.status)}`
                : "None"}
            </dd>
          </div>
          <div>
            <dt>Default style profile</dt>
            <dd>
              {defaultStyleProfile
                ? `${defaultStyleProfile.name}${formatStatusSuffix(defaultStyleProfile.status)}`
                : "None"}
            </dd>
          </div>
          <div>
            <dt>Default rule set</dt>
            <dd>
              {defaultRuleSet
                ? `${defaultRuleSet.name}${formatStatusSuffix(defaultRuleSet.status)}`
                : "None"}
            </dd>
          </div>
        </dl>

        <form className="project-form" onSubmit={handleSaveEditorialDefaults}>
          <div className="project-defaults-grid">
            <label className="field-group">
              <span>Default glossary</span>
              <select
                className="field-control"
                disabled={isSavingEditorialDefaults}
                onChange={(event) =>
                  setDraftDefaultGlossaryId(event.target.value)
                }
                value={draftDefaultGlossaryId}
              >
                <option value="">No default glossary</option>
                {glossaries.map((glossary) => (
                  <option key={glossary.id} value={glossary.id}>
                    {glossary.name}
                    {formatStatusSuffix(glossary.status)}
                  </option>
                ))}
              </select>
            </label>

            <label className="field-group">
              <span>Default style profile</span>
              <select
                className="field-control"
                disabled={isSavingEditorialDefaults}
                onChange={(event) =>
                  setDraftDefaultStyleProfileId(event.target.value)
                }
                value={draftDefaultStyleProfileId}
              >
                <option value="">No default style profile</option>
                {styleProfiles.map((styleProfile) => (
                  <option key={styleProfile.id} value={styleProfile.id}>
                    {styleProfile.name}
                    {formatStatusSuffix(styleProfile.status)}
                  </option>
                ))}
              </select>
            </label>

            <label className="field-group">
              <span>Default rule set</span>
              <select
                className="field-control"
                disabled={isSavingEditorialDefaults}
                onChange={(event) =>
                  setDraftDefaultRuleSetId(event.target.value)
                }
                value={draftDefaultRuleSetId}
              >
                <option value="">No default rule set</option>
                {ruleSets.map((ruleSet) => (
                  <option key={ruleSet.id} value={ruleSet.id}>
                    {ruleSet.name}
                    {formatStatusSuffix(ruleSet.status)}
                  </option>
                ))}
              </select>
            </label>
          </div>

          <div className="project-form__footer">
            <span className="project-form__hint">
              {isDirty
                ? "Unsaved project editorial defaults detected."
                : "Project editorial defaults are synchronized."}
            </span>

            <ActionButton
              disabled={isSavingEditorialDefaults || !isDirty}
              size="md"
              type="submit"
              variant="primary"
            >
              {isSavingEditorialDefaults
                ? "Saving editorial defaults..."
                : "Save editorial defaults"}
            </ActionButton>
          </div>
        </form>
      </section>

      <div className="workspace-document-grid">
        <DocumentImporter
          error={importError}
          isImporting={isImportingDocuments}
          onImport={onImportDocuments}
          project={project}
        />

        <DocumentList
          activeDocumentId={activeDocument?.id ?? null}
          documents={documents}
          error={loadError}
          isLoading={isLoadingDocuments}
          onOpenDocument={onOpenDocument}
          onProcessDocument={onProcessDocument}
          processError={processError}
          processingDocumentId={processingDocumentId}
          segmentLoadingDocumentId={segmentLoadingDocumentId}
        />
      </div>

      {viewMode === "workspace" ? (
        <>
          <section
            className="workspace-panel translation-workspace-header"
            data-state={workspaceState.state}
          >
            <PanelHeader
              actions={
                <div className="translation-workspace-header__actions">
                  <StatusBadge
                    emphasis="strong"
                    size="md"
                    tone={getWorkspaceStatusTone(workspaceState.state)}
                  >
                    {formatWorkspaceStatusLabel(workspaceState.state)}
                  </StatusBadge>
                  <ActionButton
                    disabled={!canLaunchTranslation}
                    onClick={() => void startTranslation()}
                    variant="primary"
                  >
                    {isRestoringTrackedJob
                      ? "Restoring job..."
                      : isStarting
                        ? "Launching..."
                        : "Translate document"}
                  </ActionButton>
                  <ActionButton
                    disabled={
                      activeDocument?.status !== "segmented" ||
                      disableChunkBuildActions ||
                      isBuildingChunks
                    }
                    onClick={() => void handleBuildChunks()}
                    variant="ghost"
                  >
                    {isBuildingChunks ? "Building..." : "Build chunks"}
                  </ActionButton>
                  <ActionButton
                    disabled={!canResumeTranslation}
                    onClick={() => void resumeTranslation()}
                    variant="secondary"
                  >
                    {isResuming ? "Resuming..." : "Resume translation"}
                  </ActionButton>
                  <ActionButton
                    disabled={!canExportDocument}
                    onClick={() => void handleExportDocument()}
                    variant="ghost"
                  >
                    {isExportingDocument ? "Exporting..." : "Export markdown"}
                  </ActionButton>
                </div>
              }
              eyebrow="Translation workspace"
              title={
                activeDocument
                  ? activeDocument.name
                  : "Select a document to start"
              }
            />

            <p className="surface-card__copy">{workspaceState.detail}</p>

            {exportError ? (
              <PanelMessage role="alert" tone="danger">
                {exportError.message}
              </PanelMessage>
            ) : null}

            {lastExport ? (
              <PanelMessage tone="success">
                Exported <strong>{lastExport.fileName}</strong> from the current{" "}
                {lastExport.status} reconstructed document snapshot.
              </PanelMessage>
            ) : null}

            <div className="translation-workspace-header__badges">
              <StatusBadge
                tone={
                  activeDocument?.status === "segmented" ? "success" : "warning"
                }
              >
                {activeDocument ? activeDocument.status : "No active document"}
              </StatusBadge>
              <StatusBadge tone="info">
                {activeDocument
                  ? `${chunks.length} chunks loaded`
                  : "Chunk list idle"}
              </StatusBadge>
              <StatusBadge tone="info">
                {jobStatus
                  ? `${jobStatus.completedChunks}/${jobStatus.totalChunks} completed`
                  : "No job progress yet"}
              </StatusBadge>
              <StatusBadge
                tone={getOperationalCountTone(jobStatus?.failedChunks ?? 0)}
              >
                {jobStatus?.failedChunks
                  ? `${jobStatus.failedChunks} failed chunks`
                  : "No failed chunks"}
              </StatusBadge>
              <StatusBadge tone={trackedJobId ? "info" : "neutral"}>
                {trackedJobId
                  ? `Tracked job ${trackedJobId}`
                  : "No tracked job"}
              </StatusBadge>
              <StatusBadge tone={getOperationalCountTone(findings.length)}>
                {activeDocument
                  ? `${findings.length} QA findings`
                  : "QA findings idle"}
              </StatusBadge>
              <StatusBadge tone={lastExport ? "success" : "neutral"}>
                {lastExport
                  ? `Last export ${lastExport.fileName}`
                  : "No export yet"}
              </StatusBadge>
              <StatusBadge tone={isRestoringTrackedJob ? "warning" : "neutral"}>
                {isRestoringTrackedJob
                  ? "Restoring tracked job"
                  : "Job restore idle"}
              </StatusBadge>
            </div>

            {activeDocument ? (
              <dl className="detail-list">
                <div>
                  <dt>Document id</dt>
                  <dd>{activeDocument.id}</dd>
                </div>
                <div>
                  <dt>Segments</dt>
                  <dd>{segments.length}</dd>
                </div>
                <div>
                  <dt>Chunks ready</dt>
                  <dd>{chunks.length}</dd>
                </div>
                <div>
                  <dt>Current chunk</dt>
                  <dd>
                    {jobStatus?.currentChunkSequence
                      ? `Chunk #${jobStatus.currentChunkSequence}`
                      : selectedChunk
                        ? `Chunk #${selectedChunk.sequence}`
                        : "None"}
                  </dd>
                </div>
                <div>
                  <dt>Last completed chunk</dt>
                  <dd>
                    {jobStatus?.lastCompletedChunkSequence
                      ? `Chunk #${jobStatus.lastCompletedChunkSequence}`
                      : "None"}
                  </dd>
                </div>
                <div>
                  <dt>Last job state</dt>
                  <dd>{jobStatus?.status ?? "No persisted job loaded"}</dd>
                </div>
              </dl>
            ) : null}
          </section>

          <div className="translation-workspace-layout">
            <ChunkBrowser
              activeDocument={activeDocument}
              chunkSegments={chunkSegments}
              chunkStatuses={jobStatus?.chunkStatuses}
              contextError={contextPreviewError}
              contextPreview={contextPreview}
              disableBuild={disableChunkBuildActions}
              chunks={chunks}
              error={chunkError}
              isBuilding={isBuildingChunks}
              isLoading={isLoadingChunks}
              isLoadingContext={isLoadingContextPreview}
              onBuildChunks={handleBuildChunks}
              onSelectChunk={onSelectChunk}
              segments={segments}
              selectedChunk={selectedChunk}
              selectedChunkId={selectedChunkId}
              selectedChunkSegments={selectedChunkSegments}
            />

            <div className="translation-workspace-sidebar">
              <FindingReviewPanel
                actionError={findingActionError}
                activeDocument={activeDocument}
                findings={findings}
                inspection={findingInspection}
                inspectionError={findingInspectionError}
                isInspectingFinding={isInspectingFinding}
                isLoadingFindings={isLoadingFindings}
                isRetranslating={isRetranslating}
                lastRetranslation={lastRetranslation}
                loadError={findingLoadError}
                refreshWarning={refreshWarning}
                onRetranslateSelectedFinding={retranslateSelectedFinding}
                onSelectFinding={selectFinding}
                selectedFinding={selectedFinding}
                selectedFindingId={selectedFindingId}
              />

              <TranslationJobMonitor
                activeDocument={activeDocument}
                error={translateJobError}
                isCancelling={isCancelling}
                isRefreshing={isRefreshing}
                isRestoringTrackedJob={isRestoringTrackedJob}
                isResuming={isResuming}
                jobStatus={jobStatus}
                onCancelJob={cancelJob}
                onClearTrackedJob={clearTrackedJob}
                onRefreshStatus={() => refreshStatus()}
                onResumeTranslation={resumeTranslation}
                trackedJobId={trackedJobId}
              />
            </div>
          </div>

          {showOperationalDebug ? (
            <OperationalDebugPanel
              activeDocument={activeDocument}
              activeProjectId={project?.id ?? null}
              trackedJobId={trackedJobId}
            />
          ) : null}
        </>
      ) : null}

      <SegmentBrowser
        activeDocument={activeDocument}
        error={segmentError}
        isLoading={isLoadingSegments}
        onSelectSection={onSelectSection}
        onSelectSegment={onSelectSegment}
        project={project}
        sections={sections}
        selectedSection={selectedSection}
        segments={segments}
        selectedSegment={selectedSegment}
        selectedSegmentId={selectedSegmentId}
      />

      {viewMode === "workspace" ? (
        <section className="workspace-readiness">
          <PanelHeader
            eyebrow="Workspace behavior"
            title="Document, job, and chunk stay aligned in one workspace"
          />
          <ul className="readiness-list">
            <li>
              Imported documents are linked explicitly to this project id.
            </li>
            <li>
              The active document remains the primary operating object in the
              workspace header.
            </li>
            <li>
              The tracked `job_id` acts as the visible execution envelope for
              translate_document.
            </li>
            <li>
              Chunk navigation stays persistent while the selected chunk becomes
              the main inspection surface.
            </li>
            <li>
              Segment-level target text remains available as the atomic review
              trace after chunk execution.
            </li>
          </ul>
        </section>
      ) : null}
    </section>
  );
}
