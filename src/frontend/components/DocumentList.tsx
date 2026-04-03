import type { DocumentSummary } from "../../shared/desktop";
import type { DesktopCommandError } from "../lib/desktop";

interface DocumentListProps {
  activeDocumentId: string | null;
  documents: DocumentSummary[];
  error: DesktopCommandError | null;
  isLoading: boolean;
  onOpenDocument: (documentId: string) => Promise<void>;
  onProcessDocument: (documentId: string) => Promise<void>;
  processError: DesktopCommandError | null;
  processingDocumentId: string | null;
  segmentLoadingDocumentId: string | null;
}

function formatBytes(value: number) {
  if (value < 1024) {
    return `${value} B`;
  }

  if (value < 1024 * 1024) {
    return `${(value / 1024).toFixed(1)} KB`;
  }

  return `${(value / (1024 * 1024)).toFixed(1)} MB`;
}

function formatTimestamp(timestamp: number) {
  return new Date(timestamp * 1000).toLocaleString();
}

function formatSourceKind(value: string) {
  return value === "local_file" ? "Local file" : value;
}

export function DocumentList({
  activeDocumentId,
  documents,
  error,
  isLoading,
  onOpenDocument,
  onProcessDocument,
  processError,
  processingDocumentId,
  segmentLoadingDocumentId,
}: DocumentListProps) {
  return (
    <section className="workspace-panel">
      <div className="surface-card__heading">
        <div>
          <p className="surface-card__eyebrow">Project documents</p>
          <h3>Registered inputs</h3>
        </div>

        <strong className="status-pill">{documents.length} total</strong>
      </div>

      {isLoading ? (
        <p className="surface-card__copy">
          Loading persisted documents for this workspace...
        </p>
      ) : null}

      {!isLoading && !error && documents.length === 0 ? (
        <p className="surface-card__copy">
          No document is registered yet. Import the first file to leave a real
          input ready for segmentation and navigation.
        </p>
      ) : null}

      {documents.length > 0 ? (
        <ul className="document-list">
          {documents.map((document) => (
            <li className="document-list__item" key={document.id}>
              <div className="document-list__heading">
                <div>
                  <strong>{document.name}</strong>
                  <p>
                    {document.format.toUpperCase()} |{" "}
                    {formatSourceKind(document.sourceKind)}
                  </p>
                </div>

                <div className="document-list__actions">
                  <span className="document-status-pill">
                    {document.status}
                  </span>
                  <button
                    className="document-action-button"
                    disabled={
                      document.status !== "segmented" ||
                      (segmentLoadingDocumentId !== null &&
                        segmentLoadingDocumentId !== document.id)
                    }
                    onClick={() => void onOpenDocument(document.id)}
                    type="button"
                  >
                    {segmentLoadingDocumentId === document.id
                      ? "Opening..."
                      : activeDocumentId === document.id
                        ? "Viewing"
                        : "Open segments"}
                  </button>
                  <button
                    className="document-action-button"
                    disabled={processingDocumentId !== null}
                    onClick={() => void onProcessDocument(document.id)}
                    type="button"
                  >
                    {processingDocumentId === document.id
                      ? "Processing..."
                      : document.status === "segmented"
                        ? "Re-segment"
                        : "Segment"}
                  </button>
                </div>
              </div>

              <dl className="document-list__meta">
                <div>
                  <dt>Imported</dt>
                  <dd>{formatTimestamp(document.createdAt)}</dd>
                </div>
                <div>
                  <dt>Size</dt>
                  <dd>{formatBytes(document.fileSizeBytes)}</dd>
                </div>
                <div>
                  <dt>Document ID</dt>
                  <dd>{document.id}</dd>
                </div>
                <div>
                  <dt>Segments</dt>
                  <dd>{document.segmentCount}</dd>
                </div>
              </dl>
            </li>
          ))}
        </ul>
      ) : null}

      {processError ? (
        <p className="form-error" role="alert">
          {processError.message}
        </p>
      ) : null}

      {error ? (
        <p className="form-error" role="alert">
          {error.message}
        </p>
      ) : null}
    </section>
  );
}
