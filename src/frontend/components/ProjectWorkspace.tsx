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
import { SegmentBrowser } from "./SegmentBrowser";
import { TranslationJobMonitor } from "./TranslationJobMonitor";

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
  styleProfiles: StyleProfileSummary[];
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
  URL.revokeObjectURL(objectUrl);
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
  styleProfiles,
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
  const {
    error: contextPreviewError,
    isLoading: isLoadingContextPreview,
    preview: contextPreview,
  } = useTranslationContextPreview({
    activeDocument,
    activeProjectId: project?.id ?? null,
    editorialDefaultsFingerprint,
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

  if (!project) {
    return (
      <section className="surface-card surface-card--accent">
        <p className="surface-card__eyebrow">Workspace</p>
        <h2>No project open yet.</h2>
        <p className="surface-card__copy">
          Select a persisted project or create a new one. Document intake and
          editorial defaults only become active after a workspace has been
          explicitly selected.
        </p>

        <ul className="readiness-list">
          <li>Projects are persisted in the encrypted SQLite database.</li>
          <li>The active project selection survives app restarts.</li>
          <li>Each project can keep explicit default editorial artifacts.</li>
        </ul>
      </section>
    );
  }

  return (
    <section className="surface-card surface-card--accent">
      <p className="surface-card__eyebrow">Open workspace</p>
      <h2>{project.name}</h2>
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
        <p className="surface-card__eyebrow">Editorial defaults</p>
        <h3>Associate one glossary, style profile, and rule set by default</h3>
        <p className="surface-card__copy">
          D5 keeps the project baseline explicit and persisted. Each project can
          point to zero or one default glossary, style profile, and rule set
          without adding precedence logic or automatic AI usage.
        </p>

        {projectError ? (
          <p className="form-error">{projectError.message}</p>
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

            <button
              className="app-shell__button"
              disabled={isSavingEditorialDefaults || !isDirty}
              type="submit"
            >
              {isSavingEditorialDefaults
                ? "Saving editorial defaults..."
                : "Save editorial defaults"}
            </button>
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

      <section
        className="workspace-panel translation-workspace-header"
        data-state={workspaceState.state}
      >
        <div className="surface-card__heading">
          <div>
            <p className="surface-card__eyebrow">Translation workspace</p>
            <h3>
              {activeDocument
                ? activeDocument.name
                : "Select a document to start"}
            </h3>
          </div>

          <div className="translation-workspace-header__actions">
            <span className="document-status-pill">
              {formatWorkspaceStatusLabel(workspaceState.state)}
            </span>
            <button
              className="document-action-button"
              disabled={!canLaunchTranslation}
              onClick={() => void startTranslation()}
              type="button"
            >
              {isRestoringTrackedJob
                ? "Restoring job..."
                : isStarting
                  ? "Launching..."
                  : "Translate document"}
            </button>
            <button
              className="document-action-button"
              disabled={
                activeDocument?.status !== "segmented" ||
                disableChunkBuildActions ||
                isBuildingChunks
              }
              onClick={() => void handleBuildChunks()}
              type="button"
            >
              {isBuildingChunks ? "Building..." : "Build chunks"}
            </button>
            <button
              className="document-action-button"
              disabled={!canResumeTranslation}
              onClick={() => void resumeTranslation()}
              type="button"
            >
              {isResuming ? "Resuming..." : "Resume translation"}
            </button>
            <button
              className="document-action-button"
              disabled={!canExportDocument}
              onClick={() => void handleExportDocument()}
              type="button"
            >
              {isExportingDocument ? "Exporting..." : "Export markdown"}
            </button>
          </div>
        </div>

        <p className="surface-card__copy">{workspaceState.detail}</p>

        {exportError ? (
          <p className="form-error" role="alert">
            {exportError.message}
          </p>
        ) : null}

        {lastExport ? (
          <p className="surface-card__copy">
            Exported <strong>{lastExport.fileName}</strong> from the current{" "}
            {lastExport.status} reconstructed document snapshot.
          </p>
        ) : null}

        <div className="translation-workspace-header__badges">
          <span className="status-pill">
            {activeDocument ? activeDocument.status : "No active document"}
          </span>
          <span className="status-pill">
            {activeDocument
              ? `${chunks.length} chunks loaded`
              : "Chunk list idle"}
          </span>
          <span className="status-pill">
            {jobStatus
              ? `${jobStatus.completedChunks}/${jobStatus.totalChunks} completed`
              : "No job progress yet"}
          </span>
          <span className="status-pill">
            {jobStatus?.failedChunks
              ? `${jobStatus.failedChunks} failed chunks`
              : "No failed chunks"}
          </span>
          <span className="status-pill">
            {trackedJobId ? `Tracked job ${trackedJobId}` : "No tracked job"}
          </span>
          <span className="status-pill">
            {activeDocument
              ? `${findings.length} QA findings`
              : "QA findings idle"}
          </span>
          <span className="status-pill">
            {lastExport
              ? `Last export ${lastExport.fileName}`
              : "No export yet"}
          </span>
          <span className="status-pill">
            {isRestoringTrackedJob
              ? "Restoring tracked job"
              : "Job restore idle"}
          </span>
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

      <section className="workspace-readiness">
        <p className="surface-card__eyebrow">Workspace behavior</p>
        <h3>Document, job, and chunk stay aligned in one workspace</h3>
        <ul className="readiness-list">
          <li>Imported documents are linked explicitly to this project id.</li>
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
    </section>
  );
}
