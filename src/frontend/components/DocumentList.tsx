import type { DocumentSummary } from "../../shared/desktop";
import type { DesktopCommandError } from "../lib/desktop";
import { ActionButton } from "./ui/ActionButton";
import { PanelHeader } from "./ui/PanelHeader";
import { PanelMessage } from "./ui/PanelMessage";
import { StatusBadge } from "./ui/StatusBadge";

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

function getDocumentStatusTone(status: DocumentSummary["status"]) {
  return status === "segmented" ? "success" : "warning";
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
      <PanelHeader
        eyebrow="Project documents"
        meta={
          <StatusBadge size="md" tone="info">
            {documents.length} total
          </StatusBadge>
        }
        title="Registered inputs"
      />

      {isLoading ? (
        <PanelMessage tone="info">
          Loading persisted documents for this workspace...
        </PanelMessage>
      ) : null}

      {!isLoading && !error && documents.length === 0 ? (
        <PanelMessage>
          No document is registered yet. Import the first file to leave a real
          input ready for segmentation and navigation.
        </PanelMessage>
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
                  <StatusBadge
                    emphasis={
                      document.status === "segmented" ? "strong" : "soft"
                    }
                    tone={getDocumentStatusTone(document.status)}
                  >
                    {document.status}
                  </StatusBadge>
                  <ActionButton
                    disabled={
                      document.status !== "segmented" ||
                      (segmentLoadingDocumentId !== null &&
                        segmentLoadingDocumentId !== document.id)
                    }
                    onClick={() => void onOpenDocument(document.id)}
                  >
                    {segmentLoadingDocumentId === document.id
                      ? "Opening..."
                      : activeDocumentId === document.id
                        ? "Viewing"
                        : "Open segments"}
                  </ActionButton>
                  <ActionButton
                    disabled={processingDocumentId !== null}
                    onClick={() => void onProcessDocument(document.id)}
                    variant={
                      document.status === "segmented" ? "ghost" : "secondary"
                    }
                  >
                    {processingDocumentId === document.id
                      ? "Processing..."
                      : document.status === "segmented"
                        ? "Re-segment"
                        : "Segment"}
                  </ActionButton>
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
        <PanelMessage role="alert" tone="danger">
          {processError.message}
        </PanelMessage>
      ) : null}

      {error ? (
        <PanelMessage role="alert" tone="danger">
          {error.message}
        </PanelMessage>
      ) : null}
    </section>
  );
}
