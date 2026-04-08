import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type {
  DocumentSummary,
  QaFindingRetranslationResult,
  QaFindingReviewContext,
  QaFindingSummary,
} from "../../shared/desktop";
import {
  DesktopCommandError,
  inspectQaFinding,
  listDocumentQaFindings,
  retranslateChunkFromQaFinding,
} from "../lib/desktop";

interface UseDocumentFindingReviewOptions {
  activeDocument: DocumentSummary | null;
  activeProjectId: string | null;
  onRefreshDocument?: (documentId: string) => Promise<void> | void;
  onSelectChunk?: (chunkId: string) => void;
}

function findingTargetKey(projectId: string, documentId: string) {
  return `${projectId}:${documentId}`;
}

function normalizeError(
  command:
    | "inspect_qa_finding"
    | "list_document_qa_findings"
    | "retranslate_chunk_from_qa_finding",
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

function findingPriority(finding: QaFindingSummary) {
  const statusWeight =
    finding.status === "open" ? 0 : finding.status === "resolved" ? 1 : 2;
  const severityWeight =
    finding.severity === "high" ? 0 : finding.severity === "medium" ? 1 : 2;

  return [
    statusWeight,
    severityWeight,
    -finding.updatedAt,
    finding.id,
  ] as const;
}

function compareFindingPriority(
  left: QaFindingSummary,
  right: QaFindingSummary,
) {
  const leftPriority = findingPriority(left);
  const rightPriority = findingPriority(right);

  for (let index = 0; index < leftPriority.length; index += 1) {
    if (leftPriority[index] < rightPriority[index]) {
      return -1;
    }

    if (leftPriority[index] > rightPriority[index]) {
      return 1;
    }
  }

  return 0;
}

export function useDocumentFindingReview({
  activeDocument,
  activeProjectId,
  onRefreshDocument,
  onSelectChunk,
}: UseDocumentFindingReviewOptions) {
  const currentTargetKey =
    activeProjectId && activeDocument
      ? findingTargetKey(activeProjectId, activeDocument.id)
      : null;
  const [findings, setFindings] = useState<QaFindingSummary[]>([]);
  const [selectedFindingId, setSelectedFindingId] = useState<string | null>(
    null,
  );
  const [inspection, setInspection] = useState<QaFindingReviewContext | null>(
    null,
  );
  const [lastRetranslation, setLastRetranslation] =
    useState<QaFindingRetranslationResult | null>(null);
  const [loadError, setLoadError] = useState<DesktopCommandError | null>(null);
  const [inspectionError, setInspectionError] =
    useState<DesktopCommandError | null>(null);
  const [actionError, setActionError] = useState<DesktopCommandError | null>(
    null,
  );
  const [refreshWarning, setRefreshWarning] =
    useState<DesktopCommandError | null>(null);
  const [isLoadingFindings, setIsLoadingFindings] = useState(false);
  const [isInspectingFinding, setIsInspectingFinding] = useState(false);
  const [isRetranslating, setIsRetranslating] = useState(false);
  const loadRequestIdRef = useRef(0);
  const inspectRequestIdRef = useRef(0);
  const actionRequestIdRef = useRef(0);
  const activeTargetKeyRef = useRef<string | null>(null);
  const lastSelectedChunkIdRef = useRef<string | null>(null);

  const sortedFindings = useMemo(
    () => [...findings].sort(compareFindingPriority),
    [findings],
  );
  const selectedFinding = useMemo(
    () =>
      sortedFindings.find((finding) => finding.id === selectedFindingId) ??
      null,
    [selectedFindingId, sortedFindings],
  );

  useEffect(() => {
    loadRequestIdRef.current += 1;
    inspectRequestIdRef.current += 1;
    actionRequestIdRef.current += 1;
    activeTargetKeyRef.current = currentTargetKey;
    lastSelectedChunkIdRef.current = null;
    setFindings([]);
    setSelectedFindingId(null);
    setInspection(null);
    setLastRetranslation(null);
    setLoadError(null);
    setInspectionError(null);
    setActionError(null);
    setRefreshWarning(null);
    setIsLoadingFindings(false);
    setIsInspectingFinding(false);
    setIsRetranslating(false);
  }, [currentTargetKey]);

  const loadFindings = useCallback(
    async (options?: { preserveSelection?: boolean }) => {
      if (!(activeProjectId && activeDocument)) {
        setIsLoadingFindings(false);
        return;
      }

      const targetKey = findingTargetKey(activeProjectId, activeDocument.id);
      activeTargetKeyRef.current = targetKey;
      const requestId = loadRequestIdRef.current + 1;
      loadRequestIdRef.current = requestId;
      setIsLoadingFindings(true);
      setLoadError(null);

      try {
        const overview = await listDocumentQaFindings({
          projectId: activeProjectId,
          documentId: activeDocument.id,
        });

        if (
          loadRequestIdRef.current !== requestId ||
          activeTargetKeyRef.current !== targetKey
        ) {
          return;
        }

        const sortedOverviewFindings = [...overview.findings].sort(
          compareFindingPriority,
        );
        setFindings(overview.findings);
        setSelectedFindingId((currentFindingId) => {
          if (
            options?.preserveSelection === true &&
            currentFindingId &&
            overview.findings.some((finding) => finding.id === currentFindingId)
          ) {
            return currentFindingId;
          }

          return sortedOverviewFindings[0]?.id ?? null;
        });
      } catch (caughtError) {
        if (
          loadRequestIdRef.current !== requestId ||
          activeTargetKeyRef.current !== targetKey
        ) {
          return;
        }

        setLoadError(
          normalizeError(
            "list_document_qa_findings",
            "The desktop shell could not load QA findings for the active document.",
            caughtError,
          ),
        );
      } finally {
        if (
          loadRequestIdRef.current === requestId &&
          activeTargetKeyRef.current === targetKey
        ) {
          setIsLoadingFindings(false);
        }
      }
    },
    [activeDocument, activeProjectId],
  );

  useEffect(() => {
    void loadFindings();
  }, [loadFindings]);

  useEffect(() => {
    inspectRequestIdRef.current += 1;
    setInspection(null);
    setInspectionError(null);
    setIsInspectingFinding(
      Boolean(activeProjectId && activeDocument && selectedFindingId),
    );
  }, [activeDocument, activeProjectId, selectedFindingId]);

  const loadInspection = useCallback(async () => {
    if (!(activeProjectId && activeDocument && selectedFindingId)) {
      inspectRequestIdRef.current += 1;
      setInspection(null);
      setInspectionError(null);
      setIsInspectingFinding(false);
      return;
    }

    const targetKey = findingTargetKey(activeProjectId, activeDocument.id);
    const requestId = inspectRequestIdRef.current + 1;
    inspectRequestIdRef.current = requestId;
    setIsInspectingFinding(true);
    setInspectionError(null);

    try {
      const nextInspection = await inspectQaFinding({
        projectId: activeProjectId,
        documentId: activeDocument.id,
        findingId: selectedFindingId,
      });

      if (
        inspectRequestIdRef.current !== requestId ||
        activeTargetKeyRef.current !== targetKey
      ) {
        return;
      }

      setInspection(nextInspection);
    } catch (caughtError) {
      if (
        inspectRequestIdRef.current !== requestId ||
        activeTargetKeyRef.current !== targetKey
      ) {
        return;
      }

      setInspection(null);
      setInspectionError(
        normalizeError(
          "inspect_qa_finding",
          "The desktop shell could not inspect the selected QA finding.",
          caughtError,
        ),
      );
    } finally {
      if (
        inspectRequestIdRef.current === requestId &&
        activeTargetKeyRef.current === targetKey
      ) {
        setIsInspectingFinding(false);
      }
    }
  }, [activeDocument, activeProjectId, selectedFindingId]);

  useEffect(() => {
    void loadInspection();
  }, [loadInspection]);

  useEffect(() => {
    const resolvedChunkId = inspection?.anchor.chunkId ?? null;

    if (!(resolvedChunkId && onSelectChunk)) {
      return;
    }

    if (lastSelectedChunkIdRef.current === resolvedChunkId) {
      return;
    }

    lastSelectedChunkIdRef.current = resolvedChunkId;
    onSelectChunk(resolvedChunkId);
  }, [inspection?.anchor.chunkId, onSelectChunk]);

  const selectFinding = useCallback(
    (findingId: string) => {
      inspectRequestIdRef.current += 1;
      setSelectedFindingId((currentFindingId) => {
        if (currentFindingId === findingId) {
          return currentFindingId;
        }

        return findingId;
      });
      setInspection(null);
      setInspectionError(null);
      setActionError(null);
      setRefreshWarning(null);
      setLastRetranslation(null);

      if (selectedFindingId === findingId) {
        setIsInspectingFinding(Boolean(activeProjectId && activeDocument));
        void loadInspection();
      }
    },
    [activeDocument, activeProjectId, loadInspection, selectedFindingId],
  );

  const retranslateSelectedFinding = useCallback(async () => {
    if (!(activeProjectId && activeDocument && selectedFindingId)) {
      return null;
    }

    const targetKey = findingTargetKey(activeProjectId, activeDocument.id);
    const requestId = actionRequestIdRef.current + 1;
    actionRequestIdRef.current = requestId;
    setIsRetranslating(true);
    setActionError(null);
    setRefreshWarning(null);

    try {
      const result = await retranslateChunkFromQaFinding({
        projectId: activeProjectId,
        documentId: activeDocument.id,
        findingId: selectedFindingId,
      });

      if (
        actionRequestIdRef.current !== requestId ||
        activeTargetKeyRef.current !== targetKey
      ) {
        return result;
      }

      setLastRetranslation(result);

      if (result.anchor.chunkId && onSelectChunk) {
        lastSelectedChunkIdRef.current = result.anchor.chunkId;
        onSelectChunk(result.anchor.chunkId);
      }

      let nextRefreshWarning: DesktopCommandError | null = null;

      if (onRefreshDocument) {
        try {
          await onRefreshDocument(activeDocument.id);
        } catch (caughtError) {
          nextRefreshWarning = normalizeError(
            "retranslate_chunk_from_qa_finding",
            "The chunk was retranslated, but the document view could not be refreshed afterwards.",
            caughtError,
          );
        }
      }

      await loadFindings({ preserveSelection: true });
      await loadInspection();

      if (
        nextRefreshWarning &&
        actionRequestIdRef.current === requestId &&
        activeTargetKeyRef.current === targetKey
      ) {
        setRefreshWarning(nextRefreshWarning);
      }

      return result;
    } catch (caughtError) {
      if (
        actionRequestIdRef.current !== requestId ||
        activeTargetKeyRef.current !== targetKey
      ) {
        return null;
      }

      const normalizedError = normalizeError(
        "retranslate_chunk_from_qa_finding",
        "The desktop shell could not launch a finding-driven chunk retranslation.",
        caughtError,
      );
      setActionError(normalizedError);
      return null;
    } finally {
      if (
        actionRequestIdRef.current === requestId &&
        activeTargetKeyRef.current === targetKey
      ) {
        setIsRetranslating(false);
      }
    }
  }, [
    activeDocument,
    activeProjectId,
    loadFindings,
    loadInspection,
    onRefreshDocument,
    onSelectChunk,
    selectedFindingId,
  ]);

  return {
    actionError,
    findings: sortedFindings,
    inspection,
    inspectionError,
    isInspectingFinding,
    isLoadingFindings,
    isRetranslating,
    lastRetranslation,
    loadError,
    refreshWarning,
    refreshFindings: loadFindings,
    retranslateSelectedFinding,
    selectedFinding,
    selectedFindingId,
    selectFinding,
  };
}
