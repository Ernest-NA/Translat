import type { ProjectSummary } from "../../shared/desktop";

interface ProjectWorkspaceProps {
  project: ProjectSummary | null;
}

function formatTimestamp(timestamp: number) {
  return new Date(timestamp * 1000).toLocaleString();
}

export function ProjectWorkspace({ project }: ProjectWorkspaceProps) {
  if (!project) {
    return (
      <section className="surface-card surface-card--accent">
        <p className="surface-card__eyebrow">Workspace</p>
        <h2>No project open yet.</h2>
        <p className="surface-card__copy">
          Select a persisted project or create a new one. C1 keeps the workspace
          intentionally small so C2 can focus on document import.
        </p>

        <ul className="readiness-list">
          <li>Projects are persisted in the encrypted SQLite database.</li>
          <li>The active project selection survives app restarts.</li>
          <li>The next step is attaching documents to this workspace.</li>
        </ul>
      </section>
    );
  }

  return (
    <section className="surface-card surface-card--accent">
      <p className="surface-card__eyebrow">Open workspace</p>
      <h2>{project.name}</h2>
      <p className="surface-card__copy">
        {project.description ??
          "This project has no description yet. It is ready to receive imported documents in C2."}
      </p>

      <dl className="detail-list">
        <div>
          <dt>Project ID</dt>
          <dd>{project.id}</dd>
        </div>
        <div>
          <dt>Created</dt>
          <dd>{formatTimestamp(project.createdAt)}</dd>
        </div>
        <div>
          <dt>Last opened</dt>
          <dd>{formatTimestamp(project.lastOpenedAt)}</dd>
        </div>
        <div>
          <dt>Updated</dt>
          <dd>{formatTimestamp(project.updatedAt)}</dd>
        </div>
      </dl>

      <section className="workspace-readiness">
        <p className="surface-card__eyebrow">Ready for next slice</p>
        <h3>Project container established</h3>
        <ul className="readiness-list">
          <li>Documents can be attached to this project in C2.</li>
          <li>No segmentation or AI state is mixed into the project model.</li>
          <li>The backend selection state is explicit and persisted.</li>
        </ul>
      </section>
    </section>
  );
}
