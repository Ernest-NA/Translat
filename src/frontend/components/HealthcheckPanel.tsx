import type { HealthcheckResponse } from "../../shared/desktop";
import type { DesktopCommandError } from "../lib/desktop";

interface HealthcheckPanelProps {
  error: DesktopCommandError | null;
  healthcheck: HealthcheckResponse | null;
  isLoading: boolean;
  onRetry: () => void;
}

function getStatusLabel(
  isLoading: boolean,
  error: DesktopCommandError | null,
  healthcheck: HealthcheckResponse | null,
) {
  if (isLoading) {
    return "checking";
  }

  if (error) {
    return "error";
  }

  return healthcheck?.status ?? "idle";
}

export function HealthcheckPanel({
  error,
  healthcheck,
  isLoading,
  onRetry,
}: HealthcheckPanelProps) {
  return (
    <section
      className="surface-card surface-card--accent"
      data-state={error ? "error" : (healthcheck?.status ?? "idle")}
    >
      <div className="health-panel__header">
        <div>
          <p className="surface-card__eyebrow">Frontend to backend</p>
          <h2>Desktop handshake</h2>
        </div>

        <strong className="status-pill">
          {getStatusLabel(isLoading, error, healthcheck)}
        </strong>
      </div>

      <p className="health-panel__message">
        {error?.message ??
          healthcheck?.message ??
          "Waiting for the first backend response."}
      </p>

      <dl className="detail-list">
        <div>
          <dt>Application</dt>
          <dd>{healthcheck?.appName ?? "Translat"}</dd>
        </div>
        <div>
          <dt>Environment</dt>
          <dd>{healthcheck?.environment ?? "unknown"}</dd>
        </div>
        <div>
          <dt>Version</dt>
          <dd>{healthcheck?.version ?? "pending"}</dd>
        </div>
        <div>
          <dt>Database</dt>
          <dd>{healthcheck?.database.encryption ?? "pending"}</dd>
        </div>
        <div>
          <dt>Migrations</dt>
          <dd>
            {healthcheck?.database.appliedMigrations.join(", ") ?? "pending"}
          </dd>
        </div>
        <div>
          <dt>Schema ready</dt>
          <dd>{healthcheck?.database.schemaReady ? "yes" : "pending"}</dd>
        </div>
        <div>
          <dt>Error details</dt>
          <dd>{error?.details ?? "No desktop error reported."}</dd>
        </div>
      </dl>

      <button
        className="app-shell__button"
        disabled={isLoading}
        onClick={onRetry}
        type="button"
      >
        {isLoading ? "Checking backend..." : "Run healthcheck again"}
      </button>
    </section>
  );
}
