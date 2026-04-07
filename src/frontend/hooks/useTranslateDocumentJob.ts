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

interface UseTranslateDocumentJobOptions {
  activeDocument: DocumentSummary | null;
  activeProjectId: string | null;
  chunks: TranslationChunkSummary[];
  onDocumentStateSync?: () => Promise<void> | void;
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
    return;
  }

  window.localStorage.setItem(JOB_STORAGE_KEY, JSON.stringify(trackedJobs));
}

function persistTrackedJob(documentJobKey: string, jobId: string | null) {
  const trackedJobs = readTrackedJobs();

  if (jobId) {
    trackedJobs[documentJobKey] = jobId;
  } else {
    delete trackedJobs[documentJobKey];
  }

  writeTrackedJobs(trackedJobs);
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
  return status.completedChunks > 0 || isTerminalStatus(status.status);
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
  const [isResuming, setIsResuming] = useState(false);
  const [isStarting, setIsStarting] = useState(false);
  const [jobStatus, setJobStatus] = useState<TranslateDocumentJobStatus | null>(
    null,
  );
  const [trackedJobId, setTrackedJobId] = useState<string | null>(null);
  const latestDocumentKeyRef = useRef<string | null>(null);
  const latestSyncFingerprintRef = useRef<string | null>(null);
  const refreshRequestIdRef = useRef(0);

  const syncDocumentState = useCallback(
    async (status: TranslateDocumentJobStatus) => {
      if (!onDocumentStateSync || !shouldSyncDocumentState(status)) {
        return;
      }

      const nextFingerprint = [
        status.jobId,
        status.status,
        status.completedChunks,
        status.failedChunks,
        status.cancelledChunks,
        status.lastUpdatedAt ?? 0,
      ].join(":");

      if (latestSyncFingerprintRef.current === nextFingerprint) {
        return;
      }

      latestSyncFingerprintRef.current = nextFingerprint;
      await onDocumentStateSync();
    },
    [onDocumentStateSync],
  );

  const refreshStatus = useCallback(
    async (nextJobId?: string | null, options?: { silent?: boolean }) => {
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
        await syncDocumentState(nextStatus);

        return nextStatus;
      } catch (caughtError) {
        if (
          refreshRequestIdRef.current !== requestId ||
          latestDocumentKeyRef.current !== documentJobKey
        ) {
          return null;
        }

        setError(
          normalizeUnexpectedError(
            "get_translate_document_job_status",
            "The desktop shell could not load translate_document job status for the active document.",
            caughtError,
          ),
        );

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
      if (!activeProjectId || !activeDocument) {
        return;
      }

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

        if (latestDocumentKeyRef.current !== documentJobKey) {
          return;
        }

        await refreshStatus(jobId, { silent: true });
      } catch (caughtError) {
        if (latestDocumentKeyRef.current !== documentJobKey) {
          return;
        }

        setError(
          normalizeUnexpectedError(
            command,
            command === "translate_document"
              ? "The desktop shell could not launch translate_document for the active document."
              : "The desktop shell could not resume the tracked translate_document job.",
            caughtError,
          ),
        );

        await refreshStatus(jobId, { silent: true });
      } finally {
        if (latestDocumentKeyRef.current === documentJobKey) {
          setIsResuming(false);
          setIsStarting(false);
        }
      }
    },
    [activeDocument, activeProjectId, chunks, refreshStatus],
  );

  useEffect(() => {
    if (!activeProjectId || !activeDocument) {
      latestDocumentKeyRef.current = null;
      latestSyncFingerprintRef.current = null;
      setError(null);
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
    setTrackedJobId(readTrackedJobs()[documentJobKey] ?? null);
  }, [activeDocument, activeProjectId]);

  useEffect(() => {
    if (!trackedJobId) {
      setJobStatus(null);
      return;
    }

    void refreshStatus(trackedJobId, { silent: true });
  }, [refreshStatus, trackedJobId]);

  useEffect(() => {
    if (
      !(
        trackedJobId &&
        (jobStatus?.status === "running" || isStarting || isResuming)
      )
    ) {
      return;
    }

    const intervalId = window.setInterval(() => {
      void refreshStatus(trackedJobId, { silent: true });
    }, JOB_STATUS_POLL_INTERVAL_MS);

    return () => {
      window.clearInterval(intervalId);
    };
  }, [isResuming, isStarting, jobStatus?.status, refreshStatus, trackedJobId]);

  const startTranslation = useCallback(async () => {
    if (!activeProjectId || !activeDocument || chunks.length === 0) {
      return null;
    }

    const jobId = generateJobId();
    void runDocumentCommand(jobId, "translate_document");

    return jobId;
  }, [activeDocument, activeProjectId, chunks.length, runDocumentCommand]);

  const resumeTranslation = useCallback(async () => {
    if (!(activeProjectId && activeDocument && trackedJobId)) {
      return null;
    }

    void runDocumentCommand(trackedJobId, "resume_translate_document_job");

    return trackedJobId;
  }, [activeDocument, activeProjectId, runDocumentCommand, trackedJobId]);

  const cancelJob = useCallback(async () => {
    if (!(activeProjectId && activeDocument && trackedJobId)) {
      return null;
    }

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

      if (latestDocumentKeyRef.current !== documentJobKey) {
        return null;
      }

      setJobStatus(nextStatus);
      await syncDocumentState(nextStatus);

      return nextStatus;
    } catch (caughtError) {
      if (latestDocumentKeyRef.current !== documentJobKey) {
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
      if (latestDocumentKeyRef.current === documentJobKey) {
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
    persistTrackedJob(documentJobKey, null);
    setTrackedJobId(null);
    setJobStatus(null);
    setError(null);
  }, [activeDocument, activeProjectId]);

  return {
    cancelJob,
    clearTrackedJob,
    error,
    isCancelling,
    isRefreshing,
    isResuming,
    isStarting,
    jobStatus,
    refreshStatus,
    resumeTranslation,
    startTranslation,
    trackedJobId,
  };
}
