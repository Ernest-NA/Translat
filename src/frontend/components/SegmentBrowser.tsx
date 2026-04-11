import type {
  DocumentSectionSummary,
  DocumentSummary,
  ProjectSummary,
  SegmentSummary,
} from "../../shared/desktop";
import type { DesktopCommandError } from "../lib/desktop";
import { PanelHeader } from "./ui/PanelHeader";
import { PanelMessage } from "./ui/PanelMessage";
import { StatusBadge } from "./ui/StatusBadge";

interface SegmentBrowserProps {
  activeDocument: DocumentSummary | null;
  error: DesktopCommandError | null;
  isLoading: boolean;
  onSelectSection: (sectionId: string) => void;
  onSelectSegment: (segmentId: string) => void;
  project: ProjectSummary | null;
  sections: DocumentSectionSummary[];
  selectedSection: DocumentSectionSummary | null;
  segments: SegmentSummary[];
  selectedSegment: SegmentSummary | null;
  selectedSegmentId: string | null;
}

function truncateSegmentPreview(value: string) {
  return value.length > 92 ? `${value.slice(0, 89)}...` : value;
}

function getSegmentStatusTone(status: SegmentSummary["status"]) {
  return status === "translated" ? "success" : "warning";
}

export function SegmentBrowser({
  activeDocument,
  error,
  isLoading,
  onSelectSection,
  onSelectSegment,
  project,
  sections,
  selectedSection,
  segments,
  selectedSegment,
  selectedSegmentId,
}: SegmentBrowserProps) {
  return (
    <section className="workspace-panel">
      <PanelHeader
        eyebrow="Segment navigator"
        meta={
          <StatusBadge size="md" tone="info">
            {activeDocument
              ? `${sections.length} sections | ${segments.length} segments`
              : "No document"}
          </StatusBadge>
        }
        title={
          activeDocument ? activeDocument.name : "Open a segmented document"
        }
      />

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
        <PanelMessage tone="info">
          Loading persisted segments for the selected document...
        </PanelMessage>
      ) : null}

      {error ? (
        <PanelMessage role="alert" tone="danger">
          {error.message}
        </PanelMessage>
      ) : null}

      {activeDocument && !isLoading && !error ? (
        <div className="segment-browser">
          <div className="segment-browser__list">
            <div className="segment-outline">
              <p className="surface-card__eyebrow">Document structure</p>

              {sections.length > 0 ? (
                <ol className="segment-outline__list">
                  {sections.map((section) => (
                    <li key={section.id}>
                      <button
                        className="segment-outline__item"
                        data-active={section.id === selectedSection?.id}
                        onClick={() => onSelectSection(section.id)}
                        type="button"
                      >
                        <strong>{section.title}</strong>
                        <span>
                          {section.sectionType} | #
                          {section.startSegmentSequence}
                          -#{section.endSegmentSequence}
                        </span>
                      </button>
                    </li>
                  ))}
                </ol>
              ) : (
                <PanelMessage>
                  No persisted structure is available for this document yet.
                </PanelMessage>
              )}
            </div>

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
                        <StatusBadge
                          tone={getSegmentStatusTone(segment.status)}
                        >
                          {segment.status}
                        </StatusBadge>
                      </div>
                      <p>{truncateSegmentPreview(segment.sourceText)}</p>
                    </button>
                  </li>
                ))}
              </ol>
            ) : (
              <PanelMessage>
                This document does not have persisted segments yet.
              </PanelMessage>
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

                  <StatusBadge
                    emphasis="strong"
                    tone={getSegmentStatusTone(selectedSegment.status)}
                  >
                    {selectedSegment.status}
                  </StatusBadge>
                </div>

                <dl className="detail-list detail-list--single">
                  <div>
                    <dt>Segment ID</dt>
                    <dd>{selectedSegment.id}</dd>
                  </div>
                  <div>
                    <dt>Section</dt>
                    <dd>{selectedSection?.title ?? "Unassigned"}</dd>
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
              <PanelMessage>
                Select a segment from the list to inspect its sequence, state,
                source text, and current target text.
              </PanelMessage>
            )}
          </div>
        </div>
      ) : null}
    </section>
  );
}
