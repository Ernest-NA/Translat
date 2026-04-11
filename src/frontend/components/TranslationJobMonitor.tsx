import type {
  DocumentSummary,
  TaskRunSummary,
  TranslateDocumentJobStatus,
  TranslateDocumentStatus,
} from "../../shared/desktop";
import type { DesktopCommandError } from "../lib/desktop";
import { ActionButton } from "./ui/ActionButton";
import { PanelHeader } from "./ui/PanelHeader";
import { PanelMessage } from "./ui/PanelMessage";
import { StatusBadge } from "./ui/StatusBadge";

interface TranslationJobMonitorProps {
  activeDocument: DocumentSummary | null;
  error: DesktopCommandError | null;
  isCancelling: boolean;
  isRefreshing: boolean;
  isRestoringTrackedJob: boolean;
  isResuming: boolean;
  jobStatus: TranslateDocumentJobStatus | null;
  onCancelJob: () => Promise<TranslateDocumentJobStatus | null>;
  onClearTrackedJob: () => void;
  onRefreshStatus: () => Promise<TranslateDocumentJobStatus | null>;
  onResumeTranslation: () => Promise<string | null>;
  trackedJobId: string | null;
}

function formatTimestamp(timestamp?: number | null) {
  if (!timestamp) {
    return "No update yet";
  }

  return new Date(timestamp * 1000).toLocaleString();
}

function formatStatusLabel(status: TranslateDocumentStatus) {
  switch (status) {
    case "cancelled":
      return "Cancelled";
    case "completed":
      return "Completed";
    case "completed_with_errors":
      return "Completed with incidents";
    case "failed":
      return "Failed";
    case "pending":
      return "Pending";
    case "running":
      return "Running";
    default:
      return status;
  }
}

function getStatusTone(status?: string | null) {
  switch (status) {
    case "completed":
      return "success";
    case "completed_with_errors":
    case "cancelled":
      return "warning";
    case "failed":
      return "danger";
    case "pending":
    case "running":
      return "info";
    default:
      return "neutral";
  }
}

function formatTaskRunLabel(taskRun: TaskRunSummary) {
  if (taskRun.actionType === "translate_document") {
    return "Document orchestration";
  }

  if (taskRun.chunkId) {
    return `Chunk run ${taskRun.chunkId}`;
  }

  return taskRun.actionType;
}

export function TranslationJobMonitor({
  activeDocument,
  error,
  isCancelling,
  isRefreshing,
  isRestoringTrackedJob,
  isResuming,
  jobStatus,
  onCancelJob,
  onClearTrackedJob,
  onRefreshStatus,
  onResumeTranslation,
  trackedJobId,
}: TranslationJobMonitorProps) {
  const canCancel =
    jobStatus?.status === "pending" || jobStatus?.status === "running";
  const canClearTrackedJob =
    trackedJobId !== null && !canCancel && !isRestoringTrackedJob;
  const canResume =
    jobStatus?.status === "cancelled" ||
    jobStatus?.status === "completed_with_errors" ||
    jobStatus?.status === "failed";
  const recentTaskRuns = jobStatus
    ? [...jobStatus.taskRuns].slice(-8).reverse()
    : [];
  const unresolvedChunks =
    jobStatus?.chunkStatuses.filter(
      (chunkStatus) =>
        chunkStatus.status === "cancelled" || chunkStatus.status === "failed",
    ) ?? [];

  return (
    <aside className="workspace-panel translation-job-monitor">
      <PanelHeader
        eyebrow="Job monitor"
        meta={
          <StatusBadge
            emphasis="strong"
            size="md"
            tone={getStatusTone(jobStatus?.status)}
          >
            {jobStatus ? formatStatusLabel(jobStatus.status) : "No tracked job"}
          </StatusBadge>
        }
        title={
          activeDocument
            ? `translate_document for ${activeDocument.name}`
            : "Select a document"
        }
      />

      {!activeDocument ? (
        <PanelMessage>
          Open a segmented document and build chunks to make the document-level
          execution monitor meaningful.
        </PanelMessage>
      ) : null}

      {activeDocument && !trackedJobId ? (
        <PanelMessage>
          No translate_document job is tracked for this document yet. Launch one
          from the workspace header to seed progress, incidents, and chunk-level
          task runs.
        </PanelMessage>
      ) : null}

      {trackedJobId ? (
        <dl className="detail-list detail-list--single job-monitor__meta">
          <div>
            <dt>Tracked job id</dt>
            <dd>{trackedJobId}</dd>
          </div>
          <div>
            <dt>Last update</dt>
            <dd>{formatTimestamp(jobStatus?.lastUpdatedAt)}</dd>
          </div>
          <div>
            <dt>Current chunk</dt>
            <dd>
              {jobStatus?.currentChunkSequence
                ? `Chunk #${jobStatus.currentChunkSequence}`
                : "None"}
            </dd>
          </div>
        </dl>
      ) : null}

      <div className="translation-job-monitor__actions">
        <ActionButton
          disabled={!trackedJobId || isRefreshing}
          onClick={() => void onRefreshStatus()}
        >
          {isRefreshing ? "Refreshing..." : "Refresh status"}
        </ActionButton>
        <ActionButton
          disabled={!canCancel || isCancelling}
          onClick={() => void onCancelJob()}
          variant="danger"
        >
          {isCancelling ? "Cancelling..." : "Cancel job"}
        </ActionButton>
        <ActionButton
          disabled={!trackedJobId || !canResume || isResuming}
          onClick={() => void onResumeTranslation()}
          variant="ghost"
        >
          {isResuming ? "Resuming..." : "Resume job"}
        </ActionButton>
        <ActionButton
          disabled={!canClearTrackedJob}
          onClick={onClearTrackedJob}
          variant="ghost"
        >
          Clear tracked job
        </ActionButton>
      </div>

      {error ? (
        <PanelMessage role="alert" tone="danger">
          {error.message}
        </PanelMessage>
      ) : null}

      {jobStatus ? (
        <>
          <dl className="glossary-metrics glossary-metrics--job">
            <div>
              <dt>Total chunks</dt>
              <dd>{jobStatus.totalChunks}</dd>
            </div>
            <div>
              <dt>Completed</dt>
              <dd>{jobStatus.completedChunks}</dd>
            </div>
            <div>
              <dt>Pending</dt>
              <dd>{jobStatus.pendingChunks}</dd>
            </div>
            <div>
              <dt>Running</dt>
              <dd>{jobStatus.runningChunks}</dd>
            </div>
            <div>
              <dt>Failed</dt>
              <dd>{jobStatus.failedChunks}</dd>
            </div>
            <div>
              <dt>Cancelled</dt>
              <dd>{jobStatus.cancelledChunks}</dd>
            </div>
          </dl>

          {jobStatus.errorMessages.length > 0 ? (
            <section className="translation-job-monitor__section">
              <p className="surface-card__eyebrow">Incidents</p>
              <ul className="capability-list">
                {jobStatus.errorMessages.map((message) => (
                  <li key={message}>{message}</li>
                ))}
              </ul>
            </section>
          ) : null}

          {unresolvedChunks.length > 0 ? (
            <section className="translation-job-monitor__section">
              <p className="surface-card__eyebrow">Affected chunks</p>
              <ul className="job-incident-list">
                {unresolvedChunks.map((chunkStatus) => (
                  <li
                    className="job-incident-list__item"
                    key={chunkStatus.chunkId}
                  >
                    <div className="chunk-link-list__heading">
                      <strong>Chunk #{chunkStatus.chunkSequence}</strong>
                      <StatusBadge tone={getStatusTone(chunkStatus.status)}>
                        {chunkStatus.status}
                      </StatusBadge>
                    </div>
                    <p>
                      {chunkStatus.errorMessage ??
                        "This chunk needs manual inspection or a resume pass."}
                    </p>
                  </li>
                ))}
              </ul>
            </section>
          ) : null}

          <section className="translation-job-monitor__section">
            <p className="surface-card__eyebrow">Recent task runs</p>
            {recentTaskRuns.length > 0 ? (
              <ol className="job-run-list">
                {recentTaskRuns.map((taskRun) => (
                  <li className="job-run-list__item" key={taskRun.id}>
                    <div className="chunk-link-list__heading">
                      <strong>{formatTaskRunLabel(taskRun)}</strong>
                      <StatusBadge tone={getStatusTone(taskRun.status)}>
                        {taskRun.status}
                      </StatusBadge>
                    </div>
                    <p>
                      {taskRun.errorMessage ??
                        `Updated ${formatTimestamp(taskRun.updatedAt)}`}
                    </p>
                  </li>
                ))}
              </ol>
            ) : (
              <PanelMessage>
                No persisted task runs are loaded for this job yet.
              </PanelMessage>
            )}
          </section>
        </>
      ) : null}
    </aside>
  );
}
