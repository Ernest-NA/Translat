import type {
  DocumentSummary,
  ProjectSummary,
  SegmentSummary,
} from "../../shared/desktop";
import type { DesktopCommandError } from "../lib/desktop";
import { DocumentImporter } from "./DocumentImporter";
import { DocumentList } from "./DocumentList";
import { SegmentBrowser } from "./SegmentBrowser";

interface ProjectWorkspaceProps {
  activeDocument: DocumentSummary | null;
  documents: DocumentSummary[];
  importError: DesktopCommandError | null;
  isImportingDocuments: boolean;
  isLoadingDocuments: boolean;
  isLoadingSegments: boolean;
  loadError: DesktopCommandError | null;
  onOpenDocument: (documentId: string) => Promise<void>;
  onImportDocuments: (files: FileList) => Promise<number>;
  onProcessDocument: (documentId: string) => Promise<void>;
  onSelectSegment: (segmentId: string) => void;
  processError: DesktopCommandError | null;
  processingDocumentId: string | null;
  project: ProjectSummary | null;
  segmentError: DesktopCommandError | null;
  segmentLoadingDocumentId: string | null;
  segments: SegmentSummary[];
  selectedSegment: SegmentSummary | null;
  selectedSegmentId: string | null;
}

function formatTimestamp(timestamp: number) {
  return new Date(timestamp * 1000).toLocaleString();
}

export function ProjectWorkspace({
  activeDocument,
  documents,
  importError,
  isImportingDocuments,
  isLoadingDocuments,
  isLoadingSegments,
  loadError,
  onOpenDocument,
  onImportDocuments,
  onProcessDocument,
  onSelectSegment,
  processError,
  processingDocumentId,
  project,
  segmentError,
  segmentLoadingDocumentId,
  segments,
  selectedSegment,
  selectedSegmentId,
}: ProjectWorkspaceProps) {
  if (!project) {
    return (
      <section className="surface-card surface-card--accent">
        <p className="surface-card__eyebrow">Workspace</p>
        <h2>No project open yet.</h2>
        <p className="surface-card__copy">
          Select a persisted project or create a new one. Document intake and
          segmentation only run after a workspace has been explicitly selected.
        </p>

        <ul className="readiness-list">
          <li>Projects are persisted in the encrypted SQLite database.</li>
          <li>The active project selection survives app restarts.</li>
          <li>C2 and C3 both work against the explicitly open project.</li>
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
          "This project has no description yet. It is ready to receive imported documents and segment them for C4."}
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
          activeDocumentId={activeDocument?.id ?? null}
          documents={documents}
          error={loadError}
          isLoading={isLoadingDocuments}
          onOpenDocument={onOpenDocument}
          onProcessDocument={onProcessDocument}
          processError={processError}
          processingDocumentId={processingDocumentId}
          segmentLoadingDocumentId={segmentLoadingDocumentId}
        />
      </div>

      <SegmentBrowser
        activeDocument={activeDocument}
        error={segmentError}
        isLoading={isLoadingSegments}
        onSelectSegment={onSelectSegment}
        project={project}
        segments={segments}
        selectedSegment={selectedSegment}
        selectedSegmentId={selectedSegmentId}
      />

      <section className="workspace-readiness">
        <p className="surface-card__eyebrow">Ready for C4</p>
        <h3>Segment navigation now runs on persisted C3 output</h3>
        <ul className="readiness-list">
          <li>Imported documents are linked explicitly to this project id.</li>
          <li>
            Segment processing persists ordered source segments per document.
          </li>
          <li>
            C4 opens segmented documents and lets the user inspect one segment
            at a time without editing it.
          </li>
        </ul>
      </section>
    </section>
  );
}
