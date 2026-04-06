import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type {
  DocumentSummary,
  DocumentTranslationChunksOverview,
} from "../../shared/desktop";
import {
  buildDocumentTranslationChunks,
  DesktopCommandError,
  listDocumentTranslationChunks,
} from "../lib/desktop";

function emptyOverview(
  projectId: string,
  documentId: string,
): DocumentTranslationChunksOverview {
  return {
    chunkSegments: [],
    chunks: [],
    documentId,
    projectId,
  };
}

export function useDocumentChunks(
  activeProjectId: string | null,
  activeDocument: DocumentSummary | null,
) {
  const [overview, setOverview] =
    useState<DocumentTranslationChunksOverview | null>(null);
  const [selectedChunkId, setSelectedChunkId] = useState<string | null>(null);
  const [error, setError] = useState<DesktopCommandError | null>(null);
  const [isBuilding, setIsBuilding] = useState(false);
  const [isLoading, setIsLoading] = useState(false);
  const requestIdRef = useRef(0);

  const applyOverview = useCallback(
    (nextOverview: DocumentTranslationChunksOverview) => {
      setOverview(nextOverview);
      setSelectedChunkId((currentChunkId) =>
        nextOverview.chunks.some((chunk) => chunk.id === currentChunkId)
          ? currentChunkId
          : (nextOverview.chunks[0]?.id ?? null),
      );
    },
    [],
  );

  const loadChunks = useCallback(async () => {
    if (
      !activeProjectId ||
      !activeDocument ||
      activeDocument.status !== "segmented"
    ) {
      requestIdRef.current += 1;
      setOverview(null);
      setSelectedChunkId(null);
      setError(null);
      setIsBuilding(false);
      setIsLoading(false);
      return;
    }

    const requestId = requestIdRef.current + 1;
    requestIdRef.current = requestId;

    setIsBuilding(false);
    setOverview((currentOverview) =>
      currentOverview?.documentId === activeDocument.id
        ? currentOverview
        : emptyOverview(activeProjectId, activeDocument.id),
    );
    setError(null);
    setIsLoading(true);

    try {
      const nextOverview = await listDocumentTranslationChunks({
        documentId: activeDocument.id,
        projectId: activeProjectId,
      });

      if (requestIdRef.current !== requestId) {
        return;
      }

      applyOverview(nextOverview);
    } catch (caughtError) {
      if (requestIdRef.current !== requestId) {
        return;
      }

      setError(
        caughtError instanceof DesktopCommandError
          ? caughtError
          : new DesktopCommandError("list_document_translation_chunks", {
              code: "UNEXPECTED_DESKTOP_ERROR",
              message:
                "The desktop shell could not load translation chunks for the selected document.",
            }),
      );
    } finally {
      if (requestIdRef.current === requestId) {
        setIsLoading(false);
      }
    }
  }, [activeDocument, activeProjectId, applyOverview]);

  useEffect(() => {
    void loadChunks();
  }, [loadChunks]);

  const buildChunks = useCallback(async (): Promise<void> => {
    if (
      !activeProjectId ||
      !activeDocument ||
      activeDocument.status !== "segmented"
    ) {
      return;
    }

    const requestId = requestIdRef.current + 1;
    requestIdRef.current = requestId;

    setError(null);
    setIsBuilding(true);

    try {
      const nextOverview = await buildDocumentTranslationChunks({
        documentId: activeDocument.id,
        projectId: activeProjectId,
      });

      if (requestIdRef.current !== requestId) {
        return;
      }

      applyOverview(nextOverview);
    } catch (caughtError) {
      if (requestIdRef.current !== requestId) {
        return;
      }

      setError(
        caughtError instanceof DesktopCommandError
          ? caughtError
          : new DesktopCommandError("build_document_translation_chunks", {
              code: "UNEXPECTED_DESKTOP_ERROR",
              message:
                "The desktop shell could not build translation chunks for the selected document.",
            }),
      );
    } finally {
      if (requestIdRef.current === requestId) {
        setIsBuilding(false);
      }
    }
  }, [activeDocument, activeProjectId, applyOverview]);

  const chunks = overview?.chunks ?? [];
  const chunkSegments = overview?.chunkSegments ?? [];
  const selectedChunk = useMemo(
    () => chunks.find((chunk) => chunk.id === selectedChunkId) ?? null,
    [chunks, selectedChunkId],
  );
  const selectedChunkSegments = useMemo(
    () =>
      chunkSegments.filter(
        (chunkSegment) => chunkSegment.chunkId === selectedChunkId,
      ),
    [chunkSegments, selectedChunkId],
  );
  const selectChunk = useCallback((chunkId: string) => {
    setSelectedChunkId(chunkId);
  }, []);

  return {
    buildChunks,
    chunkSegments,
    chunks,
    error,
    isBuilding,
    isLoading,
    selectedChunk,
    selectedChunkId,
    selectedChunkSegments,
    selectChunk,
  };
}
