import { useCallback, useEffect, useRef, useState } from "react";
import type {
  DocumentOperationalState,
  DocumentSummary,
  TaskRunSummary,
} from "../../shared/desktop";
import {
  type DesktopCommandError,
  inspectDocumentOperationalState,
} from "../lib/desktop";

interface OperationalDebugPanelProps {
  activeDocument: DocumentSummary | null;
  activeProjectId: string | null;
  trackedJobId: string | null;
}

function formatTimestamp(timestamp?: number | null) {
  if (!timestamp) {
    return "Not available";
  }

  return new Date(timestamp * 1000).toLocaleString();
}

function formatRunLabel(taskRun: TaskRunSummary) {
  if (taskRun.actionType === "translate_document") {
    return "Document run";
  }

  if (taskRun.actionType === "translate_chunk") {
    return taskRun.chunkId ? `Chunk run ${taskRun.chunkId}` : "Chunk run";
  }

  if (taskRun.actionType === "export_reconstructed_document") {
    return "Export snapshot";
  }

  return taskRun.actionType;
}

export function OperationalDebugPanel({
  activeDocument,
  activeProjectId,
  trackedJobId,
}: OperationalDebugPanelProps) {
  const [error, setError] = useState<DesktopCommandError | null>(null);
  const [inspection, setInspection] = useState<DocumentOperationalState | null>(
    null,
  );
  const [isLoading, setIsLoading] = useState(false);
  const requestTokenRef = useRef(0);

  const loadInspection = useCallback(async () => {
    const requestToken = requestTokenRef.current + 1;
    requestTokenRef.current = requestToken;

    if (!activeDocument || !activeProjectId) {
      setIsLoading(false);
      setInspection(null);
      setError(null);
      return;
    }

    setIsLoading(true);

    try {
      const nextInspection = await inspectDocumentOperationalState({
        projectId: activeProjectId,
        documentId: activeDocument.id,
        jobId: trackedJobId ?? undefined,
      });

      if (requestTokenRef.current !== requestToken) {
        return;
      }

      setInspection(nextInspection);
      setError(null);
    } catch (caughtError) {
      if (requestTokenRef.current !== requestToken) {
        return;
      }

      setInspection(null);
      setError(caughtError as DesktopCommandError);
    } finally {
      if (requestTokenRef.current === requestToken) {
        setIsLoading(false);
      }
    }
  }, [activeDocument, activeProjectId, trackedJobId]);

  useEffect(() => {
    void loadInspection();

    return () => {
      requestTokenRef.current += 1;
    };
  }, [loadInspection]);

  return (
    <aside className="workspace-panel operational-debug-panel">
      <div className="surface-card__heading">
        <div>
          <p className="surface-card__eyebrow">Operational trace</p>
          <h3>
            {activeDocument
              ? `Runs, QA, and export for ${activeDocument.name}`
              : "Select a document"}
          </h3>
        </div>

        <button
          className="document-action-button"
          disabled={!activeDocument || !activeProjectId || isLoading}
          onClick={() => void loadInspection()}
          type="button"
        >
          {isLoading ? "Refreshing..." : "Refresh trace"}
        </button>
      </div>

      {!activeDocument ? (
        <p className="surface-card__copy">
          Open a document to inspect its operational history without querying
          the database manually.
        </p>
      ) : null}

      {error ? (
        <p className="form-error" role="alert">
          {error.message}
        </p>
      ) : null}

      {inspection ? (
        <>
          <dl className="detail-list detail-list--single operational-debug-panel__meta">
            <div>
              <dt>Document state</dt>
              <dd>{inspection.documentStatus}</dd>
            </div>
            <div>
              <dt>Selected job</dt>
              <dd>{inspection.selectedJobId ?? "None"}</dd>
            </div>
            <div>
              <dt>Observed at</dt>
              <dd>{formatTimestamp(inspection.observedAt)}</dd>
            </div>
            <div>
              <dt>Reconstruction</dt>
              <dd>{inspection.reconstruction.status}</dd>
            </div>
            <div>
              <dt>Open findings</dt>
              <dd>{inspection.openFindingCount}</dd>
            </div>
            <div>
              <dt>Exports</dt>
              <dd>{inspection.exports.length}</dd>
            </div>
          </dl>

          {inspection.warnings.length > 0 ? (
            <section className="translation-job-monitor__section">
              <p className="surface-card__eyebrow">Warnings</p>
              <ul className="job-run-list">
                {inspection.warnings.map((warning) => (
                  <li
                    className="job-run-list__item"
                    data-severity={warning.severity}
                    key={warning.code}
                  >
                    <div className="chunk-link-list__heading">
                      <strong>{warning.code}</strong>
                      <span className="document-status-pill">
                        {warning.severity}
                      </span>
                    </div>
                    <p>{warning.message}</p>
                  </li>
                ))}
              </ul>
            </section>
          ) : null}

          {inspection.selectedJob ? (
            <section className="translation-job-monitor__section">
              <p className="surface-card__eyebrow">Selected job</p>
              <dl className="glossary-metrics glossary-metrics--job">
                <div>
                  <dt>Status</dt>
                  <dd>{inspection.selectedJob.overview.status}</dd>
                </div>
                <div>
                  <dt>Completed</dt>
                  <dd>{inspection.selectedJob.overview.completedChunks}</dd>
                </div>
                <div>
                  <dt>Failed</dt>
                  <dd>{inspection.selectedJob.overview.failedChunks}</dd>
                </div>
                <div>
                  <dt>Cancelled</dt>
                  <dd>{inspection.selectedJob.overview.cancelledChunks}</dd>
                </div>
                <div>
                  <dt>Findings</dt>
                  <dd>{inspection.selectedJob.overview.findingCount}</dd>
                </div>
                <div>
                  <dt>Updated</dt>
                  <dd>
                    {formatTimestamp(
                      inspection.selectedJob.overview.lastUpdatedAt,
                    )}
                  </dd>
                </div>
              </dl>
            </section>
          ) : null}

          {inspection.exports.length > 0 ? (
            <section className="translation-job-monitor__section">
              <p className="surface-card__eyebrow">Latest export</p>
              <ol className="job-run-list">
                {inspection.exports.slice(0, 3).map((exportTrace) => (
                  <li
                    className="job-run-list__item"
                    key={exportTrace.taskRun.id}
                  >
                    <div className="chunk-link-list__heading">
                      <strong>{exportTrace.fileName}</strong>
                      <span className="document-status-pill">
                        {exportTrace.reconstructedStatus}
                      </span>
                    </div>
                    <p>
                      Exported {formatTimestamp(exportTrace.exportedAt)} from{" "}
                      {exportTrace.sourceJobId ?? "no linked job"} with{" "}
                      {exportTrace.openFindingCount} open finding(s).
                    </p>
                  </li>
                ))}
              </ol>
            </section>
          ) : null}

          <section className="translation-job-monitor__section">
            <p className="surface-card__eyebrow">Recent runs</p>
            {inspection.recentRuns.length > 0 ? (
              <ol className="job-run-list">
                {inspection.recentRuns.slice(0, 8).map((taskRun) => (
                  <li className="job-run-list__item" key={taskRun.id}>
                    <div className="chunk-link-list__heading">
                      <strong>{formatRunLabel(taskRun)}</strong>
                      <span className="document-status-pill">
                        {taskRun.status}
                      </span>
                    </div>
                    <p>
                      {taskRun.errorMessage ??
                        `Updated ${formatTimestamp(taskRun.updatedAt)}`}
                    </p>
                  </li>
                ))}
              </ol>
            ) : (
              <p className="surface-card__copy">
                No task runs were found for this document yet.
              </p>
            )}
          </section>
        </>
      ) : null}
    </aside>
  );
}
