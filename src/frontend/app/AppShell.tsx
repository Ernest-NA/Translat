import { useCallback, useRef } from "react";
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
    sections,
    segments,
    selectedSection,
    selectSection,
    selectedSegment,
    selectedSegmentId,
    selectSegment,
  } = useDocumentSegments(activeProject?.id ?? null, documents);
  const { error, healthcheck, isLoading, retry } = useHealthcheck();
  const activeProjectIdRef = useRef<string | null>(activeProject?.id ?? null);
  activeProjectIdRef.current = activeProject?.id ?? null;

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
        if (activeProjectIdRef.current !== processedDocument.projectId) {
          return;
        }

        await openDocument(processedDocument.id);

        if (activeProjectIdRef.current !== processedDocument.projectId) {
          return;
        }

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
          <h1>Document structure and segments</h1>
          <p className="app-shell__lead">
            C5 adds a minimal persisted section outline on top of segmented
            documents so the workspace can orient segment navigation with a
            stable document structure.
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
              ? `${sections.length} sections | ${segments.length} segments`
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
            onSelectSection={selectSection}
            onSelectSegment={selectSegment}
            processError={processError}
            processingDocumentId={processingDocumentId}
            project={activeProject}
            segmentError={segmentError}
            segmentLoadingDocumentId={
              isLoadingSegments ? (activeDocument?.id ?? null) : null
            }
            sections={sections}
            selectedSection={selectedSection}
            segments={segments}
            selectedSegment={selectedSegment}
            selectedSegmentId={selectedSegmentId}
          />

          <section className="surface-card surface-card--split">
            <div>
              <p className="surface-card__eyebrow">C5 scope</p>
              <h2>
                Orient segment navigation with a persisted section outline.
              </h2>
              <p className="surface-card__copy">
                This slice stays focused on adding a conservative document
                structure layer over persisted segments without introducing
                editing, translation, QA, or AI actions.
              </p>
            </div>

            <ul className="capability-list">
              <li>
                Segmented documents expose persisted sections alongside their
                ordered segments.
              </li>
              <li>
                The outline degrades gracefully to a single document-level
                section when no clearer structure is detected.
              </li>
              <li>
                The selected segment still shows sequence, state, source text,
                and current target text when present.
              </li>
              <li>Editing, translation, AI, and history remain outside C5.</li>
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
                <dt>Loaded sections</dt>
                <dd>{activeDocument ? sections.length : 0}</dd>
              </div>
              <div>
                <dt>Selected segment</dt>
                <dd>
                  {selectedSegment ? `#${selectedSegment.sequence}` : "None"}
                </dd>
              </div>
              <div>
                <dt>Selected section</dt>
                <dd>{selectedSection?.title ?? "None"}</dd>
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
