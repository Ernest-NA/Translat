import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { DocumentSummary, SegmentSummary } from "../../shared/desktop";
import { DesktopCommandError, listDocumentSegments } from "../lib/desktop";

export function useDocumentSegments(
  activeProjectId: string | null,
  documents: DocumentSummary[],
) {
  const [activeDocumentId, setActiveDocumentId] = useState<string | null>(null);
  const [selectedSegmentId, setSelectedSegmentId] = useState<string | null>(
    null,
  );
  const [segments, setSegments] = useState<SegmentSummary[]>([]);
  const [error, setError] = useState<DesktopCommandError | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const requestIdRef = useRef(0);

  const activeDocument =
    documents.find((document) => document.id === activeDocumentId) ?? null;

  const selectedSegment = useMemo(
    () => segments.find((segment) => segment.id === selectedSegmentId) ?? null,
    [segments, selectedSegmentId],
  );

  useEffect(() => {
    void activeProjectId;
    requestIdRef.current += 1;
    setActiveDocumentId(null);
    setSelectedSegmentId(null);
    setSegments([]);
    setError(null);
    setIsLoading(false);
  }, [activeProjectId]);

  useEffect(() => {
    if (!activeDocumentId) {
      return;
    }

    if (!documents.some((document) => document.id === activeDocumentId)) {
      setActiveDocumentId(null);
      setSelectedSegmentId(null);
      setSegments([]);
      setError(null);
    }
  }, [activeDocumentId, documents]);

  const openDocument = useCallback(
    async (documentId: string): Promise<void> => {
      if (!activeProjectId) {
        return;
      }

      const requestId = requestIdRef.current + 1;
      requestIdRef.current = requestId;

      setActiveDocumentId(documentId);
      setSelectedSegmentId(null);
      setSegments([]);
      setError(null);
      setIsLoading(true);

      try {
        const overview = await listDocumentSegments({
          documentId,
          projectId: activeProjectId,
        });

        if (requestIdRef.current !== requestId) {
          return;
        }

        setSegments(overview.segments);
        setSelectedSegmentId(overview.segments[0]?.id ?? null);
      } catch (caughtError) {
        if (requestIdRef.current !== requestId) {
          return;
        }

        setError(
          caughtError instanceof DesktopCommandError
            ? caughtError
            : new DesktopCommandError("list_document_segments", {
                code: "UNEXPECTED_DESKTOP_ERROR",
                message:
                  "The desktop shell could not open the persisted segments for the selected document.",
              }),
        );
      } finally {
        if (requestIdRef.current === requestId) {
          setIsLoading(false);
        }
      }
    },
    [activeProjectId],
  );

  const selectSegment = useCallback((segmentId: string) => {
    setSelectedSegmentId(segmentId);
  }, []);

  return {
    activeDocument,
    error,
    isLoading,
    openDocument,
    segments,
    selectedSegment,
    selectedSegmentId,
    selectSegment,
  };
}
