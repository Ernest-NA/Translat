import { useCallback, useEffect, useRef, useState } from "react";
import type {
  DocumentSummary,
  ProjectDocumentsOverview,
} from "../../shared/desktop";
import {
  DesktopCommandError,
  importProjectDocument,
  listProjectDocuments,
} from "../lib/desktop";

const MAX_IMPORTABLE_DOCUMENT_BYTES = 20 * 1024 * 1024;

function sortDocuments(documents: DocumentSummary[]) {
  return [...documents].sort((left, right) => {
    if (left.createdAt !== right.createdAt) {
      return right.createdAt - left.createdAt;
    }

    return left.name.localeCompare(right.name, undefined, {
      sensitivity: "base",
    });
  });
}

function normalizeOverview(
  overview: ProjectDocumentsOverview,
): ProjectDocumentsOverview {
  return {
    documents: sortDocuments(overview.documents),
    projectId: overview.projectId,
  };
}

async function encodeFileAsBase64(file: File) {
  const bytes = new Uint8Array(await file.arrayBuffer());
  const chunkSize = 0x8000;
  let binary = "";

  for (let index = 0; index < bytes.length; index += chunkSize) {
    binary += String.fromCharCode(...bytes.subarray(index, index + chunkSize));
  }

  return btoa(binary);
}

export function useProjectDocuments(activeProjectId: string | null) {
  const [overview, setOverview] = useState<ProjectDocumentsOverview | null>(
    null,
  );
  const [importError, setImportError] = useState<DesktopCommandError | null>(
    null,
  );
  const [isImporting, setIsImporting] = useState(false);
  const [isLoading, setIsLoading] = useState(false);
  const [loadError, setLoadError] = useState<DesktopCommandError | null>(null);
  const importRequestIdRef = useRef(0);
  const loadRequestIdRef = useRef(0);
  const previousProjectIdRef = useRef<string | null>(null);

  useEffect(() => {
    if (previousProjectIdRef.current === activeProjectId) {
      return;
    }

    previousProjectIdRef.current = activeProjectId;
    importRequestIdRef.current += 1;
    setImportError(null);
    setIsImporting(false);
  }, [activeProjectId]);

  const reload = useCallback(async (): Promise<void> => {
    if (!activeProjectId) {
      loadRequestIdRef.current += 1;
      setOverview(null);
      setIsLoading(false);
      setLoadError(null);
      return;
    }

    const requestId = loadRequestIdRef.current + 1;
    loadRequestIdRef.current = requestId;

    setOverview({
      documents: [],
      projectId: activeProjectId,
    });
    setIsLoading(true);
    setLoadError(null);

    try {
      const nextOverview = await listProjectDocuments({
        projectId: activeProjectId,
      });

      if (loadRequestIdRef.current !== requestId) {
        return;
      }

      setOverview(normalizeOverview(nextOverview));
    } catch (caughtError) {
      if (loadRequestIdRef.current !== requestId) {
        return;
      }

      setOverview({
        documents: [],
        projectId: activeProjectId,
      });
      setLoadError(
        caughtError instanceof DesktopCommandError
          ? caughtError
          : new DesktopCommandError("list_project_documents", {
              code: "UNEXPECTED_DESKTOP_ERROR",
              message:
                "The desktop shell could not load the documents for the active project.",
            }),
      );
    } finally {
      if (loadRequestIdRef.current === requestId) {
        setIsLoading(false);
      }
    }
  }, [activeProjectId]);

  useEffect(() => {
    void reload();
  }, [reload]);

  const importDocuments = useCallback(
    async (files: FileList | File[]): Promise<number> => {
      if (!activeProjectId) {
        return 0;
      }

      const selectedFiles = Array.from(files);

      if (selectedFiles.length === 0) {
        return 0;
      }

      const oversizeFile = selectedFiles.find(
        (file) => file.size > MAX_IMPORTABLE_DOCUMENT_BYTES,
      );

      if (oversizeFile) {
        setImportError(
          new DesktopCommandError("import_project_document", {
            code: "INVALID_INPUT",
            message:
              "The selected document exceeds the current 20 MiB import limit for C2.",
          }),
        );

        return 0;
      }

      const requestId = importRequestIdRef.current + 1;
      importRequestIdRef.current = requestId;
      loadRequestIdRef.current += 1;

      setIsImporting(true);
      setImportError(null);
      setIsLoading(false);
      const importedDocuments: DocumentSummary[] = [];

      try {
        for (const file of selectedFiles) {
          if (importRequestIdRef.current !== requestId) {
            return importedDocuments.length;
          }

          const base64Content = await encodeFileAsBase64(file);

          if (importRequestIdRef.current !== requestId) {
            return importedDocuments.length;
          }

          const importedDocument = await importProjectDocument({
            base64Content,
            fileName: file.name,
            mimeType: file.type || undefined,
            projectId: activeProjectId,
          });

          importedDocuments.push(importedDocument);
        }

        if (importRequestIdRef.current !== requestId) {
          return importedDocuments.length;
        }

        setOverview((currentOverview) =>
          normalizeOverview({
            documents: [
              ...importedDocuments,
              ...(currentOverview?.documents ?? []).filter(
                (document) =>
                  !importedDocuments.some(
                    (importedDocument) => importedDocument.id === document.id,
                  ),
              ),
            ],
            projectId: activeProjectId,
          }),
        );

        return importedDocuments.length;
      } catch (caughtError) {
        if (importRequestIdRef.current !== requestId) {
          return importedDocuments.length;
        }

        if (importedDocuments.length > 0) {
          setOverview((currentOverview) =>
            normalizeOverview({
              documents: [
                ...importedDocuments,
                ...(currentOverview?.documents ?? []).filter(
                  (document) =>
                    !importedDocuments.some(
                      (importedDocument) => importedDocument.id === document.id,
                    ),
                ),
              ],
              projectId: activeProjectId,
            }),
          );
        }

        setImportError(
          caughtError instanceof DesktopCommandError
            ? caughtError
            : new DesktopCommandError("import_project_document", {
                code: "UNEXPECTED_DESKTOP_ERROR",
                message:
                  "The desktop shell could not import the selected document.",
              }),
        );

        return importedDocuments.length;
      } finally {
        if (importRequestIdRef.current === requestId) {
          setIsImporting(false);
        }
      }
    },
    [activeProjectId],
  );

  return {
    documents: overview?.documents ?? [],
    importError,
    importDocuments,
    isImporting,
    isLoading,
    loadError,
    reload,
  };
}
