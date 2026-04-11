import type { ProjectSummary } from "../../shared/desktop";
import { PanelHeader } from "./ui/PanelHeader";
import { PanelMessage } from "./ui/PanelMessage";
import { StatusBadge } from "./ui/StatusBadge";

interface ProjectListProps {
  activeProjectId: string | null;
  isLoading: boolean;
  onOpen: (projectId: string) => Promise<boolean>;
  openingProjectId: string | null;
  projects: ProjectSummary[];
}

function formatTimestamp(timestamp: number) {
  return new Date(timestamp * 1000).toLocaleString();
}

export function ProjectList({
  activeProjectId,
  isLoading,
  onOpen,
  openingProjectId,
  projects,
}: ProjectListProps) {
  return (
    <section className="surface-card">
      <PanelHeader
        eyebrow="Projects"
        meta={
          <StatusBadge size="md" tone="info">
            {projects.length} total
          </StatusBadge>
        }
        title="Persisted workspaces"
        titleLevel={2}
      />

      {isLoading ? (
        <PanelMessage tone="info">Loading persisted projects...</PanelMessage>
      ) : null}

      {!isLoading && projects.length === 0 ? (
        <PanelMessage>
          No project exists yet. Create the first one to unlock the workspace
          for C2 document import.
        </PanelMessage>
      ) : null}

      {projects.length > 0 ? (
        <ul className="project-list">
          {projects.map((project) => {
            const isActive = project.id === activeProjectId;
            const isOpening = project.id === openingProjectId;

            return (
              <li key={project.id}>
                <button
                  className="project-list__item"
                  data-active={isActive}
                  disabled={isOpening}
                  onClick={() => void onOpen(project.id)}
                  type="button"
                >
                  <div className="project-list__item-heading">
                    <strong>{project.name}</strong>
                    <span>{isActive ? "Open" : "Open project"}</span>
                  </div>

                  <p>
                    {project.description ??
                      "No description yet. This project is ready for its first document set."}
                  </p>

                  <dl className="project-list__meta">
                    <div>
                      <dt>Opened</dt>
                      <dd>{formatTimestamp(project.lastOpenedAt)}</dd>
                    </div>
                    <div>
                      <dt>Created</dt>
                      <dd>{formatTimestamp(project.createdAt)}</dd>
                    </div>
                  </dl>
                </button>
              </li>
            );
          })}
        </ul>
      ) : null}
    </section>
  );
}
