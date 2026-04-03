import type {
  DocumentSummary,
  ProjectSummary,
  SegmentSummary,
} from "../../shared/desktop";
import type { DesktopCommandError } from "../lib/desktop";

interface SegmentBrowserProps {
  activeDocument: DocumentSummary | null;
  error: DesktopCommandError | null;
  isLoading: boolean;
  onSelectSegment: (segmentId: string) => void;
  project: ProjectSummary | null;
  segments: SegmentSummary[];
  selectedSegment: SegmentSummary | null;
  selectedSegmentId: string | null;
}

function truncateSegmentPreview(value: string) {
  return value.length > 92 ? `${value.slice(0, 89)}...` : value;
}

export function SegmentBrowser({
  activeDocument,
  error,
  isLoading,
  onSelectSegment,
  project,
  segments,
  selectedSegment,
  selectedSegmentId,
}: SegmentBrowserProps) {
  return (
    <section className="workspace-panel">
      <div className="surface-card__heading">
        <div>
          <p className="surface-card__eyebrow">Segment navigator</p>
          <h3>
            {activeDocument ? activeDocument.name : "Open a segmented document"}
          </h3>
        </div>

        <strong className="status-pill">
          {activeDocument ? `${segments.length} loaded` : "No document"}
        </strong>
      </div>

      {project && activeDocument ? (
        <p className="surface-card__copy">
          Project <strong>{project.name}</strong> | Document{" "}
          <strong>{activeDocument.name}</strong>
        </p>
      ) : (
        <p className="surface-card__copy">
          Select a segmented document from the workspace to inspect its
          persisted segments and navigate them one by one.
        </p>
      )}

      {isLoading ? (
        <p className="surface-card__copy">
          Loading persisted segments for the selected document...
        </p>
      ) : null}

      {error ? (
        <p className="form-error" role="alert">
          {error.message}
        </p>
      ) : null}

      {activeDocument && !isLoading && !error ? (
        <div className="segment-browser">
          <div className="segment-browser__list">
            {segments.length > 0 ? (
              <ol className="segment-list">
                {segments.map((segment) => (
                  <li key={segment.id}>
                    <button
                      className="segment-list__item"
                      data-active={segment.id === selectedSegmentId}
                      onClick={() => onSelectSegment(segment.id)}
                      type="button"
                    >
                      <div className="segment-list__heading">
                        <strong>#{segment.sequence}</strong>
                        <span className="document-status-pill">
                          {segment.status}
                        </span>
                      </div>
                      <p>{truncateSegmentPreview(segment.sourceText)}</p>
                    </button>
                  </li>
                ))}
              </ol>
            ) : (
              <p className="surface-card__copy">
                This document does not have persisted segments yet.
              </p>
            )}
          </div>

          <div className="segment-browser__detail">
            {selectedSegment ? (
              <>
                <div className="surface-card__heading">
                  <div>
                    <p className="surface-card__eyebrow">Selected segment</p>
                    <h3>Sequence #{selectedSegment.sequence}</h3>
                  </div>

                  <span className="document-status-pill">
                    {selectedSegment.status}
                  </span>
                </div>

                <dl className="detail-list detail-list--single">
                  <div>
                    <dt>Segment ID</dt>
                    <dd>{selectedSegment.id}</dd>
                  </div>
                  <div>
                    <dt>Words</dt>
                    <dd>{selectedSegment.sourceWordCount}</dd>
                  </div>
                  <div>
                    <dt>Characters</dt>
                    <dd>{selectedSegment.sourceCharacterCount}</dd>
                  </div>
                </dl>

                <div className="segment-detail__body">
                  <div>
                    <p className="surface-card__eyebrow">Source text</p>
                    <div className="segment-detail__text">
                      {selectedSegment.sourceText}
                    </div>
                  </div>

                  <div>
                    <p className="surface-card__eyebrow">Current target text</p>
                    <div className="segment-detail__text segment-detail__text--muted">
                      {selectedSegment.targetText ??
                        "No current target text is stored for this segment yet."}
                    </div>
                  </div>
                </div>
              </>
            ) : (
              <p className="surface-card__copy">
                Select a segment from the list to inspect its sequence, state,
                source text, and current target text.
              </p>
            )}
          </div>
        </div>
      ) : null}
    </section>
  );
}
