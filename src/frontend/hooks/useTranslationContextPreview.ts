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
  selectedChunk: TranslationChunkSummary | null;
}

export function useTranslationContextPreview({
  activeDocument,
  activeProjectId,
  selectedChunk,
}: UseTranslationContextPreviewOptions) {
  const [error, setError] = useState<DesktopCommandError | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [preview, setPreview] = useState<TranslationContextPreview | null>(
    null,
  );
  const requestIdRef = useRef(0);

  useEffect(() => {
    if (!(activeProjectId && activeDocument && selectedChunk)) {
      requestIdRef.current += 1;
      setError(null);
      setIsLoading(false);
      setPreview(null);
      return;
    }

    const requestId = requestIdRef.current + 1;
    requestIdRef.current = requestId;

    setError(null);
    setIsLoading(true);

    void buildTranslationContext({
      actionScope: "translation",
      chunkId: selectedChunk.id,
      documentId: activeDocument.id,
      projectId: activeProjectId,
    })
      .then((nextPreview) => {
        if (requestIdRef.current !== requestId) {
          return;
        }

        setPreview(nextPreview);
      })
      .catch((caughtError) => {
        if (requestIdRef.current !== requestId) {
          return;
        }

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
  }, [activeDocument, activeProjectId, selectedChunk]);

  return {
    error,
    isLoading,
    preview,
  };
}
