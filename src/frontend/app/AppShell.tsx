import { DESKTOP_COMMANDS } from "../../shared/desktop";
import { HealthcheckPanel } from "../components/HealthcheckPanel";
import { ProjectComposer } from "../components/ProjectComposer";
import { ProjectList } from "../components/ProjectList";
import { ProjectWorkspace } from "../components/ProjectWorkspace";
import { useHealthcheck } from "../hooks/useHealthcheck";
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
    error: projectError,
    isCreating,
    isLoading: isLoadingProjects,
    openingProjectId,
    projects,
    selectProject,
    submitProject,
  } = useProjectsWorkspace();
  const { error, healthcheck, isLoading, retry } = useHealthcheck();

  const runtimeLabel = healthcheck
    ? `${healthcheck.environment} | v${healthcheck.version}`
    : "desktop runtime";

  return (
    <main className="app-shell">
      <header className="app-shell__header">
        <div>
          <p className="app-shell__eyebrow">Translat</p>
          <h1>Project workspace foundation</h1>
          <p className="app-shell__lead">
            C1 turns the shell into a real project container: persisted
            workspaces, explicit selection, and a clean landing point for C2
            document import.
          </p>
        </div>

        <div className="app-shell__header-meta">
          <span>{runtimeLabel}</span>
          <span>{projects.length} persisted projects</span>
          <span>{activeProject ? "Workspace open" : "No active project"}</span>
        </div>
      </header>

      <section className="app-shell__grid">
        <div className="app-shell__primary">
          <ProjectWorkspace project={activeProject} />

          <section className="surface-card surface-card--split">
            <div>
              <p className="surface-card__eyebrow">C1 scope</p>
              <h2>Projects only, by design.</h2>
              <p className="surface-card__copy">
                This slice establishes the persisted project container and keeps
                documents, segmentation, glossary work, and AI orchestration out
                of scope.
              </p>
            </div>

            <ul className="capability-list">
              <li>
                Projects are stored in encrypted SQLite and survive restart.
              </li>
              <li>The currently open project is persisted explicitly.</li>
              <li>The frontend can create, list, and reopen projects.</li>
              <li>
                C2 can now attach imported documents to a real project id.
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
            <h2>{DESKTOP_COMMANDS.listProjects}</h2>

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
                <dt>DB path</dt>
                <dd>{healthcheck?.database.path ?? "Pending bootstrap"}</dd>
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
