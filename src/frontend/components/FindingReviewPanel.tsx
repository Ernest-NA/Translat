import type {
  DocumentSummary,
  QaFindingRetranslationResult,
  QaFindingReviewContext,
  QaFindingSummary,
  ReconstructedSegment,
} from "../../shared/desktop";
import type { DesktopCommandError } from "../lib/desktop";
import { ActionButton } from "./ui/ActionButton";
import { PanelHeader } from "./ui/PanelHeader";
import { PanelMessage } from "./ui/PanelMessage";
import { StatusBadge } from "./ui/StatusBadge";

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
  refreshWarning: DesktopCommandError | null;
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

function getSeverityTone(severity: QaFindingSummary["severity"]) {
  switch (severity) {
    case "high":
      return "danger";
    case "low":
      return "info";
    default:
      return "warning";
  }
}

function getFindingStatusTone(status: QaFindingSummary["status"]) {
  switch (status) {
    case "resolved":
      return "success";
    case "dismissed":
      return "neutral";
    default:
      return "warning";
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
  refreshWarning,
  selectedFinding,
  selectedFindingId,
}: FindingReviewPanelProps) {
  const formattedDetails = prettyPrintDetails(
    inspection?.finding.details ?? selectedFinding?.details ?? null,
  );

  return (
    <section className="workspace-panel">
      <PanelHeader
        eyebrow="Finding review"
        meta={
          <StatusBadge size="md" tone="info">
            {activeDocument ? `${findings.length} findings` : "No document"}
          </StatusBadge>
        }
        title={
          activeDocument
            ? `QA findings for ${activeDocument.name}`
            : "Open a document to review QA findings"
        }
      />

      {!activeDocument ? (
        <PanelMessage>
          Findings review stays inside the Translation Workspace. Open a
          document first to inspect QA output and relaunch a focused chunk
          correction.
        </PanelMessage>
      ) : null}

      {activeDocument && isLoadingFindings ? (
        <PanelMessage tone="info">
          Loading persisted QA findings for the active document...
        </PanelMessage>
      ) : null}

      {loadError ? (
        <PanelMessage role="alert" tone="danger">
          {loadError.message}
        </PanelMessage>
      ) : null}

      {activeDocument &&
      !isLoadingFindings &&
      !loadError &&
      findings.length === 0 ? (
        <PanelMessage>
          No persisted QA findings exist for this document yet. Run QA first to
          surface chunk-level review anchors here.
        </PanelMessage>
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
                    <StatusBadge tone={getSeverityTone(finding.severity)}>
                      {formatSeverity(finding.severity)}
                    </StatusBadge>
                  </div>
                  <div className="chunk-list__badges">
                    <StatusBadge tone={getFindingStatusTone(finding.status)}>
                      {formatStatus(finding.status)}
                    </StatusBadge>
                    <StatusBadge tone="info">
                      {finding.chunkId ? "Chunk-linked" : "Document-linked"}
                    </StatusBadge>
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
                <PanelHeader
                  eyebrow="Selected finding"
                  meta={
                    <div className="chunk-detail__heading-badges">
                      <StatusBadge
                        tone={getSeverityTone(selectedFinding.severity)}
                      >
                        {formatSeverity(selectedFinding.severity)}
                      </StatusBadge>
                      <StatusBadge
                        tone={getFindingStatusTone(selectedFinding.status)}
                      >
                        {formatStatus(selectedFinding.status)}
                      </StatusBadge>
                    </div>
                  }
                  title={selectedFinding.findingType}
                />

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
                  <PanelMessage tone="info">
                    Resolving the current chunk anchor and reconstructed context
                    for this finding...
                  </PanelMessage>
                ) : null}

                {inspectionError ? (
                  <PanelMessage role="alert" tone="danger">
                    {inspectionError.message}
                  </PanelMessage>
                ) : null}

                {inspection ? (
                  <>
                    <PanelMessage
                      title="Anchor status"
                      tone={
                        inspection.anchor.canRetranslate ? "success" : "warning"
                      }
                    >
                      {inspection.anchor.resolutionMessage}
                    </PanelMessage>

                    {inspection.relatedBlock ? (
                      <PanelMessage title="Related block" tone="info">
                        {inspection.relatedBlock.title ??
                          inspection.relatedBlock.id}{" "}
                        | {inspection.relatedBlock.status} |{" "}
                        {inspection.relatedBlock.contentSource}
                      </PanelMessage>
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
                                <StatusBadge tone="info">
                                  {formatResolvedFrom(segment.resolvedFrom)}
                                </StatusBadge>
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
                  <ActionButton
                    disabled={
                      !inspection?.anchor.canRetranslate || isRetranslating
                    }
                    onClick={() => void onRetranslateSelectedFinding()}
                  >
                    {isRetranslating ? "Retranslating..." : "Retranslate chunk"}
                  </ActionButton>
                </div>

                {actionError ? (
                  <PanelMessage role="alert" tone="danger">
                    {actionError.message}
                  </PanelMessage>
                ) : null}

                {lastRetranslation ? (
                  <>
                    <PanelMessage title="Latest corrective run" tone="success">
                      Task run {lastRetranslation.translateResult.taskRun.id} |{" "}
                      job {lastRetranslation.correctionJobId}
                    </PanelMessage>

                    {lastRetranslation.reviewActionWarning ? (
                      <PanelMessage role="alert" tone="warning">
                        {lastRetranslation.reviewActionWarning}
                      </PanelMessage>
                    ) : null}

                    {refreshWarning ? (
                      <PanelMessage role="alert" tone="warning">
                        {refreshWarning.message}
                      </PanelMessage>
                    ) : null}
                  </>
                ) : null}
              </>
            ) : (
              <PanelMessage>
                Select a finding to resolve its chunk anchor, inspect affected
                segments, and launch a focused retranslation.
              </PanelMessage>
            )}
          </div>
        </div>
      ) : null}
    </section>
  );
}
