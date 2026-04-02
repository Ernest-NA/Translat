import type { DocumentSummary, ProjectSummary } from "../../shared/desktop";
import type { DesktopCommandError } from "../lib/desktop";
import { DocumentImporter } from "./DocumentImporter";
import { DocumentList } from "./DocumentList";

interface ProjectWorkspaceProps {
  documents: DocumentSummary[];
  importError: DesktopCommandError | null;
  isImportingDocuments: boolean;
  isLoadingDocuments: boolean;
  loadError: DesktopCommandError | null;
  onImportDocuments: (files: FileList) => Promise<number>;
  project: ProjectSummary | null;
}

function formatTimestamp(timestamp: number) {
  return new Date(timestamp * 1000).toLocaleString();
}

export function ProjectWorkspace({
  documents,
  importError,
  isImportingDocuments,
  isLoadingDocuments,
  loadError,
  onImportDocuments,
  project,
}: ProjectWorkspaceProps) {
  if (!project) {
    return (
      <section className="surface-card surface-card--accent">
        <p className="surface-card__eyebrow">Workspace</p>
        <h2>No project open yet.</h2>
        <p className="surface-card__copy">
          Select a persisted project or create a new one. C2 only imports
          documents after a workspace has been explicitly selected.
        </p>

        <ul className="readiness-list">
          <li>Projects are persisted in the encrypted SQLite database.</li>
          <li>The active project selection survives app restarts.</li>
          <li>C2 imports documents only after a project is explicitly open.</li>
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

      <div className="workspace-document-grid">
        <DocumentImporter
          error={importError}
          isImporting={isImportingDocuments}
          onImport={onImportDocuments}
          project={project}
        />

        <DocumentList
          documents={documents}
          error={loadError}
          isLoading={isLoadingDocuments}
        />
      </div>

      <section className="workspace-readiness">
        <p className="surface-card__eyebrow">Ready for C3</p>
        <h3>Documents are formally registered</h3>
        <ul className="readiness-list">
          <li>Imported documents are linked explicitly to this project id.</li>
          <li>
            Each file is copied into local Translat storage and persisted.
          </li>
          <li>Normalization and segmentation remain outside C2 by design.</li>
        </ul>
      </section>
    </section>
  );
}
