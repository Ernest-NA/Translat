import { useEffect, useRef, useState } from "react";
import type {
  DocumentSummary,
  TranslationChunkSummary,
  TranslationContextPreview,
} from "../../shared/desktop";
import { buildTranslationContext, DesktopCommandError } from "../lib/desktop";

interface UseTranslationContextPreviewOptions {
  activeDocument: DocumentSummary | null;
  activeProjectId: string | null;
  editorialDefaultsFingerprint?: string;
  enabled?: boolean;
  selectedChunk: TranslationChunkSummary | null;
}

export function useTranslationContextPreview({
  activeDocument,
  activeProjectId,
  editorialDefaultsFingerprint,
  enabled = true,
  selectedChunk,
}: UseTranslationContextPreviewOptions) {
  const [error, setError] = useState<DesktopCommandError | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [preview, setPreview] = useState<TranslationContextPreview | null>(
    null,
  );
  const requestIdRef = useRef(0);

  useEffect(() => {
    if (!(enabled && activeProjectId && activeDocument && selectedChunk)) {
      requestIdRef.current += 1;
      setError(null);
      setIsLoading(false);
      setPreview(null);
      return;
    }

    const contextInputFingerprint =
      editorialDefaultsFingerprint ??
      `${activeProjectId}:${activeDocument.id}:${selectedChunk.id}`;
    const requestId = requestIdRef.current + 1;
    requestIdRef.current = requestId;

    setError(null);
    setIsLoading(true);
    setPreview(null);

    void buildTranslationContext({
      actionScope: "translation",
      chunkId: selectedChunk.id,
      documentId: activeDocument.id,
      projectId: activeProjectId,
    })
      .then((nextPreview) => {
        if (
          requestIdRef.current !== requestId ||
          contextInputFingerprint.length === 0
        ) {
          return;
        }

        setPreview(nextPreview);
      })
      .catch((caughtError) => {
        if (
          requestIdRef.current !== requestId ||
          contextInputFingerprint.length === 0
        ) {
          return;
        }

        setPreview(null);
        setError(
          caughtError instanceof DesktopCommandError
            ? caughtError
            : new DesktopCommandError("build_translation_context", {
                code: "UNEXPECTED_DESKTOP_ERROR",
                message:
                  "The desktop shell could not load the translation context preview for the selected chunk.",
              }),
        );
      })
      .finally(() => {
        if (requestIdRef.current === requestId) {
          setIsLoading(false);
        }
      });
  }, [
    activeDocument,
    activeProjectId,
    editorialDefaultsFingerprint,
    enabled,
    selectedChunk,
  ]);

  return {
    error,
    isLoading,
    preview,
  };
}
