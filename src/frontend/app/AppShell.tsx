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

  const runtimeLabel = healthcheck
    ? `${healthcheck.environment} | v${healthcheck.version}`
    : "desktop runtime";

  return (
    <main className="app-shell">
      <header className="app-shell__header">
        <div>
          <p className="app-shell__eyebrow">Translat</p>
          <h1>Project document intake</h1>
          <p className="app-shell__lead">
            C2 turns the open project into a real document workspace: imported
            files are copied into Translat storage, registered in persistence,
            and left in a clean imported state for C3.
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
            project={activeProject}
          />

          <section className="surface-card surface-card--split">
            <div>
              <p className="surface-card__eyebrow">C2 scope</p>
              <h2>Import and register documents only.</h2>
              <p className="surface-card__copy">
                This slice formalizes document intake inside a project and keeps
                normalization, segmentation, glossary work, and AI orchestration
                out of scope.
              </p>
            </div>

            <ul className="capability-list">
              <li>
                Imported documents are stored against the active project id.
              </li>
              <li>
                The backend keeps a metadata record plus internal file copy.
              </li>
              <li>The workspace reloads imported documents after restart.</li>
              <li>
                C3 can take this imported state as its real input boundary.
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
            <h2>{DESKTOP_COMMANDS.importProjectDocument}</h2>

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
