import { useCallback } from "react";
import { DESKTOP_COMMANDS } from "../../shared/desktop";
import { HealthcheckPanel } from "../components/HealthcheckPanel";
import { ProjectComposer } from "../components/ProjectComposer";
import { ProjectList } from "../components/ProjectList";
import { ProjectWorkspace } from "../components/ProjectWorkspace";
import { useHealthcheck } from "../hooks/useHealthcheck";
import { useProjectDocuments } from "../hooks/useProjectDocuments";
import { useProjectsWorkspace } from "../hooks/useProjectsWorkspace";

function formatCheckedAt(value?: number) {
  if (!value) {
    return "Pending first response";
  }

  return new Date(value).toLocaleString();
}

export function AppShell() {
  const {
    activeProject,
    reload: reloadProjects,
    error: projectError,
    isCreating,
    isLoading: isLoadingProjects,
    openingProjectId,
    projects,
    selectProject,
    submitProject,
  } = useProjectsWorkspace();
  const {
    documents,
    importError,
    importDocuments,
    isImporting,
    isLoading: isLoadingDocuments,
    loadError,
    processDocument,
    processError,
    processingDocumentId,
  } = useProjectDocuments(activeProject?.id ?? null);
  const { error, healthcheck, isLoading, retry } = useHealthcheck();

  const handleImportDocuments = useCallback(
    async (files: FileList): Promise<number> => {
      const importedCount = await importDocuments(files);

      if (importedCount > 0) {
        try {
          await reloadProjects();
        } catch {
          // Keep the import result successful even if the sidebar refresh fails.
        }
      }

      return importedCount;
    },
    [importDocuments, reloadProjects],
  );

  const handleProcessDocument = useCallback(
    async (documentId: string): Promise<void> => {
      const processedDocument = await processDocument(documentId);

      if (processedDocument) {
        try {
          await reloadProjects();
        } catch {
          // Keep the segmentation result visible even if the sidebar refresh fails.
        }
      }
    },
    [processDocument, reloadProjects],
  );

  const runtimeLabel = healthcheck
    ? `${healthcheck.environment} | v${healthcheck.version}`
    : "desktop runtime";

  return (
    <main className="app-shell">
      <header className="app-shell__header">
        <div>
          <p className="app-shell__eyebrow">Translat</p>
          <h1>Project document processing</h1>
          <p className="app-shell__lead">
            C3 takes imported documents, normalizes their text, and persists
            ordered source segments so C4 can navigate a real document
            structure.
          </p>
        </div>

        <div className="app-shell__header-meta">
          <span>{runtimeLabel}</span>
          <span>{projects.length} persisted projects</span>
          <span>
            {activeProject
              ? `${documents.length} documents in workspace`
              : "No active project"}
          </span>
        </div>
      </header>

      <section className="app-shell__grid">
        <div className="app-shell__primary">
          <ProjectWorkspace
            documents={documents}
            importError={importError}
            isImportingDocuments={isImporting}
            isLoadingDocuments={isLoadingDocuments}
            loadError={loadError}
            onImportDocuments={handleImportDocuments}
            onProcessDocument={handleProcessDocument}
            processError={processError}
            processingDocumentId={processingDocumentId}
            project={activeProject}
          />

          <section className="surface-card surface-card--split">
            <div>
              <p className="surface-card__eyebrow">C3 scope</p>
              <h2>Normalize and persist source segments.</h2>
              <p className="surface-card__copy">
                This slice turns imported documents into ordered persisted
                segments and keeps segment navigation, translation, glossary
                work, and AI orchestration out of scope.
              </p>
            </div>

            <ul className="capability-list">
              <li>
                Imported UTF-8 documents can be processed inside the active
                project.
              </li>
              <li>
                Normalization is deterministic and intentionally minimal for the
                MVP.
              </li>
              <li>Segments are persisted with stable sequence per document.</li>
              <li>
                C4 can list and navigate the resulting segments without
                reprocessing.
              </li>
            </ul>
          </section>
        </div>

        <aside className="app-shell__sidebar">
          <ProjectComposer
            error={projectError}
            isCreating={isCreating}
            onSubmit={submitProject}
          />

          <ProjectList
            activeProjectId={activeProject?.id ?? null}
            isLoading={isLoadingProjects}
            onOpen={selectProject}
            openingProjectId={openingProjectId}
            projects={projects}
          />

          <section className="surface-card">
            <p className="surface-card__eyebrow">Command pattern</p>
            <h2>{DESKTOP_COMMANDS.processProjectDocument}</h2>

            <dl className="detail-list">
              <div>
                <dt>Wrapper</dt>
                <dd>`invokeDesktopCommand`</dd>
              </div>
              <div>
                <dt>Last check</dt>
                <dd>{formatCheckedAt(healthcheck?.checkedAt)}</dd>
              </div>
              <div>
                <dt>Open project</dt>
                <dd>{activeProject?.name ?? "None"}</dd>
              </div>
              <div>
                <dt>Imported docs</dt>
                <dd>{activeProject ? documents.length : 0}</dd>
              </div>
              <div>
                <dt>Segmented docs</dt>
                <dd>
                  {activeProject
                    ? documents.filter(
                        (document) => document.status === "segmented",
                      ).length
                    : 0}
                </dd>
              </div>
            </dl>
          </section>

          <HealthcheckPanel
            error={error}
            healthcheck={healthcheck}
            isLoading={isLoading}
            onRetry={retry}
          />
        </aside>
      </section>
    </main>
  );
}
