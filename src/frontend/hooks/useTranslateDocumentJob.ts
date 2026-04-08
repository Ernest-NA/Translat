import { useCallback, useEffect, useRef, useState } from "react";
import type {
  DocumentSummary,
  TranslateDocumentChunkResult,
  TranslateDocumentJobStatus,
  TranslateDocumentStatus,
  TranslationChunkSummary,
} from "../../shared/desktop";
import {
  cancelTranslateDocumentJob,
  DesktopCommandError,
  getTranslateDocumentJobStatus,
  resumeTranslateDocumentJob,
  translateDocument,
} from "../lib/desktop";

const JOB_STORAGE_KEY = "translat.translation-workspace-jobs.v1";
const JOB_STATUS_POLL_INTERVAL_MS = 4000;
const JOB_STATUS_MISSING_CLEAR_GRACE_MS = 15000;

interface UseTranslateDocumentJobOptions {
  activeDocument: DocumentSummary | null;
  activeProjectId: string | null;
  chunks: TranslationChunkSummary[];
  onDocumentStateSync?: (documentId: string) => Promise<void> | void;
}

function normalizeUnexpectedError(
  command:
    | "cancel_translate_document_job"
    | "get_translate_document_job_status"
    | "resume_translate_document_job"
    | "translate_document",
  message: string,
  caughtError: unknown,
) {
  if (caughtError instanceof DesktopCommandError) {
    return caughtError;
  }

  return new DesktopCommandError(command, {
    code: "UNEXPECTED_DESKTOP_ERROR",
    message,
  });
}

function isMissingTrackedJobError(error: DesktopCommandError) {
  return (
    error.code === "NOT_FOUND" ||
    error.code === "INVALID_INPUT" ||
    error.message.toLowerCase().includes("does not exist")
  );
}

function buildDocumentJobKey(projectId: string, documentId: string) {
  return `${projectId}:${documentId}`;
}

function readTrackedJobs(): Record<string, string> {
  if (typeof window === "undefined") {
    return {};
  }

  try {
    const storedValue = window.localStorage.getItem(JOB_STORAGE_KEY);

    if (!storedValue) {
      return {};
    }

    const parsedValue = JSON.parse(storedValue);

    if (!parsedValue || typeof parsedValue !== "object") {
      return {};
    }

    return Object.entries(parsedValue).reduce<Record<string, string>>(
      (jobs, [key, value]) => {
        if (typeof value === "string" && value.length > 0) {
          jobs[key] = value;
        }

        return jobs;
      },
      {},
    );
  } catch {
    return {};
  }
}

function writeTrackedJobs(trackedJobs: Record<string, string>) {
  if (typeof window === "undefined") {
    return false;
  }

  try {
    window.localStorage.setItem(JOB_STORAGE_KEY, JSON.stringify(trackedJobs));
    return true;
  } catch {
    return false;
  }
}

function persistTrackedJob(documentJobKey: string, jobId: string | null) {
  const trackedJobs = readTrackedJobs();

  if (jobId) {
    trackedJobs[documentJobKey] = jobId;
  } else {
    delete trackedJobs[documentJobKey];
  }

  return writeTrackedJobs(trackedJobs);
}

function clearTrackedJobState(
  documentJobKey: string,
  updateTrackedJobId: (jobId: string | null) => void,
  updateJobStatus: (status: TranslateDocumentJobStatus | null) => void,
) {
  persistTrackedJob(documentJobKey, null);
  updateTrackedJobId(null);
  updateJobStatus(null);
}

function generateJobId() {
  if (
    typeof globalThis.crypto !== "undefined" &&
    typeof globalThis.crypto.randomUUID === "function"
  ) {
    return globalThis.crypto.randomUUID();
  }

  return `translate-document-${Date.now().toString(36)}-${Math.random()
    .toString(36)
    .slice(2, 10)}`;
}

function isTerminalStatus(status: TranslateDocumentStatus) {
  return (
    status === "cancelled" ||
    status === "completed" ||
    status === "completed_with_errors" ||
    status === "failed"
  );
}

function buildOptimisticChunkStatuses(
  chunks: TranslationChunkSummary[],
): TranslateDocumentChunkResult[] {
  return chunks.map((chunk) => ({
    chunkId: chunk.id,
    chunkSequence: chunk.sequence,
    errorMessage: null,
    status: "pending",
    taskRun: null,
    translatedSegmentCount: 0,
  }));
}

function buildOptimisticJobStatus(
  projectId: string,
  documentId: string,
  jobId: string,
  chunks: TranslationChunkSummary[],
): TranslateDocumentJobStatus {
  return {
    cancelledChunks: 0,
    chunkStatuses: buildOptimisticChunkStatuses(chunks),
    completedChunks: 0,
    currentChunkId: null,
    currentChunkSequence: null,
    documentId,
    errorMessages: [],
    failedChunks: 0,
    jobId,
    lastCompletedChunkId: null,
    lastCompletedChunkSequence: null,
    lastUpdatedAt: null,
    latestDocumentTaskRun: null,
    pendingChunks: chunks.length,
    projectId,
    runningChunks: 0,
    status: "pending",
    taskRuns: [],
    totalChunks: chunks.length,
  };
}

function shouldSyncDocumentState(status: TranslateDocumentJobStatus) {
  return (
    isTerminalStatus(status.status) ||
    status.completedChunks > 0 ||
    status.failedChunks > 0 ||
    status.cancelledChunks > 0
  );
}

function buildDocumentSyncFingerprint(status: TranslateDocumentJobStatus) {
  return [
    status.jobId,
    status.status,
    status.completedChunks,
    status.failedChunks,
    status.cancelledChunks,
    status.lastCompletedChunkId ?? "",
    status.lastCompletedChunkSequence ?? "",
  ].join(":");
}

export function useTranslateDocumentJob({
  activeDocument,
  activeProjectId,
  chunks,
  onDocumentStateSync,
}: UseTranslateDocumentJobOptions) {
  const [error, setError] = useState<DesktopCommandError | null>(null);
  const [isCancelling, setIsCancelling] = useState(false);
  const [isRefreshing, setIsRefreshing] = useState(false);
  const [isRestoringTrackedJob, setIsRestoringTrackedJob] = useState(false);
  const [isResuming, setIsResuming] = useState(false);
  const [isStarting, setIsStarting] = useState(false);
  const [jobStatus, setJobStatus] = useState<TranslateDocumentJobStatus | null>(
    null,
  );
  const [trackedJobId, setTrackedJobId] = useState<string | null>(null);
  const latestDocumentKeyRef = useRef<string | null>(null);
  const latestSyncFingerprintRef = useRef<string | null>(null);
  const commandRequestIdRef = useRef(0);
  const cancelRequestIdRef = useRef(0);
  const commandInFlightRef = useRef(false);
  const cancelInFlightRef = useRef(false);
  const missingJobClearGraceUntilRef = useRef(0);
  const refreshRequestIdRef = useRef(0);

  const syncDocumentState = useCallback(
    async (status: TranslateDocumentJobStatus) => {
      if (!onDocumentStateSync || !shouldSyncDocumentState(status)) {
        return;
      }

      const nextFingerprint = buildDocumentSyncFingerprint(status);

      if (latestSyncFingerprintRef.current === nextFingerprint) {
        return;
      }

      latestSyncFingerprintRef.current = nextFingerprint;
      await onDocumentStateSync(status.documentId);
    },
    [onDocumentStateSync],
  );

  const refreshStatus = useCallback(
    async (
      nextJobId?: string | null,
      options?: {
        clearMissingJob?: boolean;
        silent?: boolean;
      },
    ) => {
      if (!activeProjectId || !activeDocument) {
        setJobStatus(null);
        return null;
      }

      const jobId = nextJobId ?? trackedJobId;

      if (!jobId) {
        setJobStatus(null);
        return null;
      }

      const documentJobKey = buildDocumentJobKey(
        activeProjectId,
        activeDocument.id,
      );
      const requestId = refreshRequestIdRef.current + 1;
      refreshRequestIdRef.current = requestId;

      if (!options?.silent) {
        setIsRefreshing(true);
      }

      try {
        const nextStatus = await getTranslateDocumentJobStatus({
          documentId: activeDocument.id,
          jobId,
          projectId: activeProjectId,
        });

        if (
          refreshRequestIdRef.current !== requestId ||
          latestDocumentKeyRef.current !== documentJobKey
        ) {
          return null;
        }

        setError(null);
        setJobStatus(nextStatus);
        setIsRestoringTrackedJob(false);
        await syncDocumentState(nextStatus);

        return nextStatus;
      } catch (caughtError) {
        if (
          refreshRequestIdRef.current !== requestId ||
          latestDocumentKeyRef.current !== documentJobKey
        ) {
          return null;
        }

        const normalizedError = normalizeUnexpectedError(
          "get_translate_document_job_status",
          "The desktop shell could not load translate_document job status for the active document.",
          caughtError,
        );
        const isTransientMissingJobError =
          isMissingTrackedJobError(normalizedError) &&
          !options?.clearMissingJob;

        if (
          options?.clearMissingJob &&
          isMissingTrackedJobError(normalizedError)
        ) {
          persistTrackedJob(documentJobKey, null);
          setTrackedJobId(null);
          setJobStatus(null);
        }

        setIsRestoringTrackedJob(false);

        if (!isTransientMissingJobError) {
          setError(normalizedError);
        }

        return null;
      } finally {
        if (refreshRequestIdRef.current === requestId) {
          setIsRefreshing(false);
        }
      }
    },
    [activeDocument, activeProjectId, syncDocumentState, trackedJobId],
  );

  const runDocumentCommand = useCallback(
    async (
      jobId: string,
      command: "resume_translate_document_job" | "translate_document",
    ) => {
      if (!activeProjectId || !activeDocument || commandInFlightRef.current) {
        return;
      }

      commandInFlightRef.current = true;
      missingJobClearGraceUntilRef.current =
        Date.now() + JOB_STATUS_MISSING_CLEAR_GRACE_MS;
      const commandRequestId = commandRequestIdRef.current + 1;
      commandRequestIdRef.current = commandRequestId;
      const documentJobKey = buildDocumentJobKey(
        activeProjectId,
        activeDocument.id,
      );

      persistTrackedJob(documentJobKey, jobId);
      setTrackedJobId(jobId);
      setError(null);

      if (command === "translate_document") {
        setIsStarting(true);
        setJobStatus(
          buildOptimisticJobStatus(
            activeProjectId,
            activeDocument.id,
            jobId,
            chunks,
          ),
        );
      } else {
        setIsResuming(true);
        setJobStatus((currentStatus) =>
          currentStatus
            ? {
                ...currentStatus,
                status: "running",
              }
            : buildOptimisticJobStatus(
                activeProjectId,
                activeDocument.id,
                jobId,
                chunks,
              ),
        );
      }

      try {
        if (command === "translate_document") {
          await translateDocument({
            documentId: activeDocument.id,
            jobId,
            projectId: activeProjectId,
          });
        } else {
          await resumeTranslateDocumentJob({
            documentId: activeDocument.id,
            jobId,
            projectId: activeProjectId,
          });
        }

        if (
          commandRequestIdRef.current !== commandRequestId ||
          latestDocumentKeyRef.current !== documentJobKey
        ) {
          return;
        }

        await refreshStatus(jobId, {
          clearMissingJob: false,
          silent: true,
        });
      } catch (caughtError) {
        if (
          commandRequestIdRef.current !== commandRequestId ||
          latestDocumentKeyRef.current !== documentJobKey
        ) {
          return;
        }

        const normalizedError = normalizeUnexpectedError(
          command,
          command === "translate_document"
            ? "The desktop shell could not launch translate_document for the active document."
            : "The desktop shell could not resume the tracked translate_document job.",
          caughtError,
        );
        setError(normalizedError);

        if (command === "translate_document") {
          missingJobClearGraceUntilRef.current = 0;
          const confirmedStatus = await refreshStatus(jobId, {
            clearMissingJob: true,
            silent: true,
          });

          if (
            confirmedStatus?.jobId !== jobId &&
            latestDocumentKeyRef.current === documentJobKey
          ) {
            clearTrackedJobState(documentJobKey, setTrackedJobId, setJobStatus);
          }
        } else {
          await refreshStatus(jobId, {
            clearMissingJob: true,
            silent: true,
          });
        }
      } finally {
        if (commandRequestIdRef.current === commandRequestId) {
          commandInFlightRef.current = false;
          setIsResuming(false);
          setIsStarting(false);
        }
      }
    },
    [activeDocument, activeProjectId, chunks, refreshStatus],
  );

  useEffect(() => {
    commandRequestIdRef.current += 1;
    cancelRequestIdRef.current += 1;
    refreshRequestIdRef.current += 1;
    commandInFlightRef.current = false;
    cancelInFlightRef.current = false;
    missingJobClearGraceUntilRef.current = 0;
    setIsCancelling(false);
    setIsRefreshing(false);
    setIsResuming(false);
    setIsStarting(false);

    if (!activeProjectId || !activeDocument) {
      latestDocumentKeyRef.current = null;
      latestSyncFingerprintRef.current = null;
      setError(null);
      setIsRestoringTrackedJob(false);
      setJobStatus(null);
      setTrackedJobId(null);
      return;
    }

    const documentJobKey = buildDocumentJobKey(
      activeProjectId,
      activeDocument.id,
    );
    latestDocumentKeyRef.current = documentJobKey;
    latestSyncFingerprintRef.current = null;
    setError(null);
    setJobStatus(null);
    const restoredTrackedJobId = readTrackedJobs()[documentJobKey] ?? null;

    setTrackedJobId(restoredTrackedJobId);
    setIsRestoringTrackedJob(restoredTrackedJobId !== null);
  }, [activeDocument, activeProjectId]);

  useEffect(() => {
    if (!trackedJobId) {
      setIsRestoringTrackedJob(false);
      setJobStatus(null);
      return;
    }

    void refreshStatus(trackedJobId, {
      clearMissingJob:
        Date.now() >= missingJobClearGraceUntilRef.current &&
        !commandInFlightRef.current,
      silent: true,
    });
  }, [refreshStatus, trackedJobId]);

  useEffect(() => {
    if (
      !(
        trackedJobId &&
        (jobStatus?.status === "pending" ||
          jobStatus?.status === "running" ||
          isStarting ||
          isResuming)
      )
    ) {
      return;
    }

    const intervalId = window.setInterval(() => {
      void refreshStatus(trackedJobId, {
        clearMissingJob:
          Date.now() >= missingJobClearGraceUntilRef.current &&
          !commandInFlightRef.current,
        silent: true,
      });
    }, JOB_STATUS_POLL_INTERVAL_MS);

    return () => {
      window.clearInterval(intervalId);
    };
  }, [isResuming, isStarting, jobStatus?.status, refreshStatus, trackedJobId]);

  const startTranslation = useCallback(async () => {
    if (
      !activeProjectId ||
      !activeDocument ||
      chunks.length === 0 ||
      commandInFlightRef.current ||
      cancelInFlightRef.current
    ) {
      return null;
    }

    const jobId = generateJobId();
    void runDocumentCommand(jobId, "translate_document");

    return jobId;
  }, [activeDocument, activeProjectId, chunks.length, runDocumentCommand]);

  const resumeTranslation = useCallback(async () => {
    if (
      !activeProjectId ||
      !activeDocument ||
      !trackedJobId ||
      commandInFlightRef.current ||
      cancelInFlightRef.current
    ) {
      return null;
    }

    void runDocumentCommand(trackedJobId, "resume_translate_document_job");

    return trackedJobId;
  }, [activeDocument, activeProjectId, runDocumentCommand, trackedJobId]);

  const cancelJob = useCallback(async () => {
    if (
      !activeProjectId ||
      !activeDocument ||
      !trackedJobId ||
      cancelInFlightRef.current ||
      commandInFlightRef.current
    ) {
      return null;
    }

    cancelInFlightRef.current = true;
    const cancelRequestId = cancelRequestIdRef.current + 1;
    cancelRequestIdRef.current = cancelRequestId;
    const documentJobKey = buildDocumentJobKey(
      activeProjectId,
      activeDocument.id,
    );
    setError(null);
    setIsCancelling(true);

    try {
      const nextStatus = await cancelTranslateDocumentJob({
        documentId: activeDocument.id,
        jobId: trackedJobId,
        projectId: activeProjectId,
      });

      if (
        cancelRequestIdRef.current !== cancelRequestId ||
        latestDocumentKeyRef.current !== documentJobKey
      ) {
        return null;
      }

      setJobStatus(nextStatus);
      await syncDocumentState(nextStatus);

      return nextStatus;
    } catch (caughtError) {
      if (
        cancelRequestIdRef.current !== cancelRequestId ||
        latestDocumentKeyRef.current !== documentJobKey
      ) {
        return null;
      }

      setError(
        normalizeUnexpectedError(
          "cancel_translate_document_job",
          "The desktop shell could not cancel the tracked translate_document job.",
          caughtError,
        ),
      );

      return null;
    } finally {
      if (cancelRequestIdRef.current === cancelRequestId) {
        cancelInFlightRef.current = false;
        setIsCancelling(false);
      }
    }
  }, [activeDocument, activeProjectId, syncDocumentState, trackedJobId]);

  const clearTrackedJob = useCallback(() => {
    if (!(activeProjectId && activeDocument)) {
      return;
    }

    const documentJobKey = buildDocumentJobKey(
      activeProjectId,
      activeDocument.id,
    );
    commandRequestIdRef.current += 1;
    cancelRequestIdRef.current += 1;
    refreshRequestIdRef.current += 1;
    commandInFlightRef.current = false;
    cancelInFlightRef.current = false;
    missingJobClearGraceUntilRef.current = 0;
    clearTrackedJobState(documentJobKey, setTrackedJobId, setJobStatus);
    setError(null);
    setIsCancelling(false);
    setIsRefreshing(false);
    setIsRestoringTrackedJob(false);
    setIsResuming(false);
    setIsStarting(false);
  }, [activeDocument, activeProjectId]);

  return {
    cancelJob,
    clearTrackedJob,
    error,
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
  };
}
