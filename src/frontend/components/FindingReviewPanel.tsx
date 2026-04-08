import type {
  DocumentSummary,
  QaFindingRetranslationResult,
  QaFindingReviewContext,
  QaFindingSummary,
  ReconstructedSegment,
} from "../../shared/desktop";
import type { DesktopCommandError } from "../lib/desktop";

interface FindingReviewPanelProps {
  activeDocument: DocumentSummary | null;
  actionError: DesktopCommandError | null;
  findings: QaFindingSummary[];
  inspection: QaFindingReviewContext | null;
  inspectionError: DesktopCommandError | null;
  isInspectingFinding: boolean;
  isLoadingFindings: boolean;
  isRetranslating: boolean;
  lastRetranslation: QaFindingRetranslationResult | null;
  loadError: DesktopCommandError | null;
  onRetranslateSelectedFinding: () => Promise<unknown>;
  onSelectFinding: (findingId: string) => void;
  selectedFinding: QaFindingSummary | null;
  selectedFindingId: string | null;
}

function formatSeverity(severity: QaFindingSummary["severity"]) {
  switch (severity) {
    case "high":
      return "High";
    case "low":
      return "Low";
    default:
      return "Medium";
  }
}

function formatStatus(status: QaFindingSummary["status"]) {
  switch (status) {
    case "dismissed":
      return "Dismissed";
    case "resolved":
      return "Resolved";
    default:
      return "Open";
  }
}

function truncateText(value: string) {
  return value.length > 140 ? `${value.slice(0, 137)}...` : value;
}

function formatResolvedFrom(segment: ReconstructedSegment["resolvedFrom"]) {
  return segment === "source_fallback" ? "Source fallback" : "Target";
}

function prettyPrintDetails(details: string | null) {
  if (!details) {
    return null;
  }

  try {
    return JSON.stringify(JSON.parse(details), null, 2);
  } catch {
    return details;
  }
}

export function FindingReviewPanel({
  activeDocument,
  actionError,
  findings,
  inspection,
  inspectionError,
  isInspectingFinding,
  isLoadingFindings,
  isRetranslating,
  lastRetranslation,
  loadError,
  onRetranslateSelectedFinding,
  onSelectFinding,
  selectedFinding,
  selectedFindingId,
}: FindingReviewPanelProps) {
  const formattedDetails = prettyPrintDetails(
    inspection?.finding.details ?? selectedFinding?.details ?? null,
  );

  return (
    <section className="workspace-panel">
      <div className="surface-card__heading">
        <div>
          <p className="surface-card__eyebrow">Finding review</p>
          <h3>
            {activeDocument
              ? `QA findings for ${activeDocument.name}`
              : "Open a document to review QA findings"}
          </h3>
        </div>

        <strong className="status-pill">
          {activeDocument ? `${findings.length} findings` : "No document"}
        </strong>
      </div>

      {!activeDocument ? (
        <p className="surface-card__copy">
          Findings review stays inside the Translation Workspace. Open a
          document first to inspect QA output and relaunch a focused chunk
          correction.
        </p>
      ) : null}

      {activeDocument && isLoadingFindings ? (
        <p className="surface-card__copy">
          Loading persisted QA findings for the active document...
        </p>
      ) : null}

      {loadError ? (
        <p className="form-error" role="alert">
          {loadError.message}
        </p>
      ) : null}

      {activeDocument &&
      !isLoadingFindings &&
      !loadError &&
      findings.length === 0 ? (
        <p className="surface-card__copy">
          No persisted QA findings exist for this document yet. Run QA first to
          surface chunk-level review anchors here.
        </p>
      ) : null}

      {activeDocument && findings.length > 0 ? (
        <div className="finding-review">
          <ol className="chunk-list">
            {findings.map((finding) => (
              <li key={finding.id}>
                <button
                  className="chunk-list__item"
                  data-active={finding.id === selectedFindingId}
                  onClick={() => onSelectFinding(finding.id)}
                  type="button"
                >
                  <div className="chunk-list__heading">
                    <strong>{finding.findingType}</strong>
                    <span className="document-status-pill">
                      {formatSeverity(finding.severity)}
                    </span>
                  </div>
                  <div className="chunk-list__badges">
                    <span className="document-status-pill">
                      {formatStatus(finding.status)}
                    </span>
                    <span className="chunk-role-pill">
                      {finding.chunkId ? "Chunk-linked" : "Document-linked"}
                    </span>
                  </div>
                  <p>{truncateText(finding.message)}</p>
                  <span className="chunk-list__meta">
                    {finding.jobId ? `Job ${finding.jobId}` : "Document scope"}
                  </span>
                </button>
              </li>
            ))}
          </ol>

          <div className="chunk-browser__detail">
            {selectedFinding ? (
              <>
                <div className="surface-card__heading">
                  <div>
                    <p className="surface-card__eyebrow">Selected finding</p>
                    <h3>{selectedFinding.findingType}</h3>
                  </div>

                  <div className="chunk-detail__heading-badges">
                    <span className="document-status-pill">
                      {formatSeverity(selectedFinding.severity)}
                    </span>
                    <span className="document-status-pill">
                      {formatStatus(selectedFinding.status)}
                    </span>
                  </div>
                </div>

                <p className="surface-card__copy">{selectedFinding.message}</p>

                <dl className="detail-list detail-list--single">
                  <div>
                    <dt>Finding ID</dt>
                    <dd>{selectedFinding.id}</dd>
                  </div>
                  <div>
                    <dt>Linked chunk</dt>
                    <dd>{inspection?.anchor.chunkId ?? "Not resolved"}</dd>
                  </div>
                  <div>
                    <dt>Chunk sequence</dt>
                    <dd>
                      {inspection?.anchor.chunkSequence
                        ? `#${inspection.anchor.chunkSequence}`
                        : "Not resolved"}
                    </dd>
                  </div>
                  <div>
                    <dt>Resolution</dt>
                    <dd>{inspection?.anchor.resolutionKind ?? "Pending"}</dd>
                  </div>
                  <div>
                    <dt>Latest chunk run</dt>
                    <dd>{inspection?.latestChunkTaskRun?.id ?? "None"}</dd>
                  </div>
                  <div>
                    <dt>Latest chunk job</dt>
                    <dd>{inspection?.latestChunkTaskRun?.jobId ?? "None"}</dd>
                  </div>
                </dl>

                {isInspectingFinding ? (
                  <p className="surface-card__copy">
                    Resolving the current chunk anchor and reconstructed context
                    for this finding...
                  </p>
                ) : null}

                {inspectionError ? (
                  <p className="form-error" role="alert">
                    {inspectionError.message}
                  </p>
                ) : null}

                {inspection ? (
                  <>
                    <div className="segment-detail__text segment-detail__text--muted">
                      <strong>Anchor status</strong>
                      <p>{inspection.anchor.resolutionMessage}</p>
                    </div>

                    {inspection.relatedBlock ? (
                      <div className="segment-detail__text segment-detail__text--muted">
                        <strong>Related block</strong>
                        <p>
                          {inspection.relatedBlock.title ??
                            inspection.relatedBlock.id}{" "}
                          | {inspection.relatedBlock.status} |{" "}
                          {inspection.relatedBlock.contentSource}
                        </p>
                      </div>
                    ) : null}

                    {inspection.relatedSegments.length > 0 ? (
                      <div className="chunk-link-workspace">
                        <p className="surface-card__eyebrow">
                          Related segments
                        </p>
                        <ol className="chunk-link-list">
                          {inspection.relatedSegments.map((segment) => (
                            <li
                              className="chunk-link-list__item"
                              key={segment.id}
                            >
                              <div className="chunk-link-list__heading">
                                <strong>Segment #{segment.sequence}</strong>
                                <span className="chunk-role-pill">
                                  {formatResolvedFrom(segment.resolvedFrom)}
                                </span>
                              </div>
                              <p>{segment.sourceText}</p>
                              <p>{segment.resolvedText}</p>
                            </li>
                          ))}
                        </ol>
                      </div>
                    ) : null}

                    {formattedDetails ? (
                      <div className="finding-review__details">
                        <p className="surface-card__eyebrow">Finding details</p>
                        <pre>{formattedDetails}</pre>
                      </div>
                    ) : null}
                  </>
                ) : null}

                <div className="translation-job-monitor__actions">
                  <button
                    className="document-action-button"
                    disabled={
                      !inspection?.anchor.canRetranslate || isRetranslating
                    }
                    onClick={() => void onRetranslateSelectedFinding()}
                    type="button"
                  >
                    {isRetranslating ? "Retranslating..." : "Retranslate chunk"}
                  </button>
                </div>

                {actionError ? (
                  <p className="form-error" role="alert">
                    {actionError.message}
                  </p>
                ) : null}

                {lastRetranslation ? (
                  <>
                    <div className="segment-detail__text segment-detail__text--muted">
                      <strong>Latest corrective run</strong>
                      <p>
                        Task run {lastRetranslation.translateResult.taskRun.id}{" "}
                        | job {lastRetranslation.correctionJobId}
                      </p>
                    </div>

                    {lastRetranslation.reviewActionWarning ? (
                      <p className="form-error" role="alert">
                        {lastRetranslation.reviewActionWarning}
                      </p>
                    ) : null}
                  </>
                ) : null}
              </>
            ) : (
              <p className="surface-card__copy">
                Select a finding to resolve its chunk anchor, inspect affected
                segments, and launch a focused retranslation.
              </p>
            )}
          </div>
        </div>
      ) : null}
    </section>
  );
}
