import { useCallback } from "react";
import { DESKTOP_COMMANDS } from "../../shared/desktop";
import { HealthcheckPanel } from "../components/HealthcheckPanel";
import { ProjectComposer } from "../components/ProjectComposer";
import { ProjectList } from "../components/ProjectList";
import { ProjectWorkspace } from "../components/ProjectWorkspace";
import { useDocumentSegments } from "../hooks/useDocumentSegments";
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
  const {
    activeDocument,
    error: segmentError,
    isLoading: isLoadingSegments,
    openDocument,
    segments,
    selectedSegment,
    selectedSegmentId,
    selectSegment,
  } = useDocumentSegments(activeProject?.id ?? null, documents);
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
        await openDocument(processedDocument.id);
        try {
          await reloadProjects();
        } catch {
          // Keep the segmentation result visible even if the sidebar refresh fails.
        }
      }
    },
    [openDocument, processDocument, reloadProjects],
  );

  const runtimeLabel = healthcheck
    ? `${healthcheck.environment} | v${healthcheck.version}`
    : "desktop runtime";

  return (
    <main className="app-shell">
      <header className="app-shell__header">
        <div>
          <p className="app-shell__eyebrow">Translat</p>
          <h1>Segment list and detail</h1>
          <p className="app-shell__lead">
            C4 opens a segmented document, lists its persisted segments in
            order, and shows a stable detail view for the selected segment.
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
          <span>
            {activeDocument
              ? `${segments.length} segments in open document`
              : "No open document"}
          </span>
        </div>
      </header>

      <section className="app-shell__grid">
        <div className="app-shell__primary">
          <ProjectWorkspace
            activeDocument={activeDocument}
            documents={documents}
            importError={importError}
            isImportingDocuments={isImporting}
            isLoadingDocuments={isLoadingDocuments}
            isLoadingSegments={isLoadingSegments}
            loadError={loadError}
            onOpenDocument={openDocument}
            onImportDocuments={handleImportDocuments}
            onProcessDocument={handleProcessDocument}
            onSelectSegment={selectSegment}
            processError={processError}
            processingDocumentId={processingDocumentId}
            project={activeProject}
            segmentError={segmentError}
            segmentLoadingDocumentId={
              isLoadingSegments ? (activeDocument?.id ?? null) : null
            }
            segments={segments}
            selectedSegment={selectedSegment}
            selectedSegmentId={selectedSegmentId}
          />

          <section className="surface-card surface-card--split">
            <div>
              <p className="surface-card__eyebrow">C4 scope</p>
              <h2>Open a segmented document and navigate its segments.</h2>
              <p className="surface-card__copy">
                This slice stays focused on querying persisted segments,
                selecting them, and showing their essential data without adding
                editing, translation, QA, or AI actions.
              </p>
            </div>

            <ul className="capability-list">
              <li>
                Segmented documents can be opened directly from the project
                workspace.
              </li>
              <li>
                Segments are listed in persisted sequence order from SQLite.
              </li>
              <li>
                The selected segment shows sequence, state, source text, and
                current target text when present.
              </li>
              <li>Editing, translation, AI, and history remain outside C4.</li>
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
            <h2>{DESKTOP_COMMANDS.listDocumentSegments}</h2>

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
                <dt>Open document</dt>
                <dd>{activeDocument?.name ?? "None"}</dd>
              </div>
              <div>
                <dt>Loaded segments</dt>
                <dd>{activeDocument ? segments.length : 0}</dd>
              </div>
              <div>
                <dt>Selected segment</dt>
                <dd>
                  {selectedSegment ? `#${selectedSegment.sequence}` : "None"}
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
