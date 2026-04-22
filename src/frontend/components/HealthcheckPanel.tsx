import type { HealthcheckResponse } from "../../shared/desktop";
import {
  DESKTOP_RUNTIME_UNAVAILABLE_CODE,
  type DesktopCommandError,
} from "../lib/desktop";
import { ActionButton } from "./ui/ActionButton";
import { PanelHeader } from "./ui/PanelHeader";
import { PanelMessage } from "./ui/PanelMessage";
import { StatusBadge } from "./ui/StatusBadge";

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

  if (error?.code === DESKTOP_RUNTIME_UNAVAILABLE_CODE) {
    return "web preview";
  }

  if (error) {
    return "error";
  }

  return healthcheck?.status ?? "idle";
}

function getStatusTone(
  isLoading: boolean,
  error: DesktopCommandError | null,
  healthcheck: HealthcheckResponse | null,
) {
  if (isLoading) {
    return "info";
  }

  if (error?.code === DESKTOP_RUNTIME_UNAVAILABLE_CODE) {
    return "warning";
  }

  if (error) {
    return "danger";
  }

  return healthcheck?.status === "ok" ? "success" : "neutral";
}

export function HealthcheckPanel({
  error,
  healthcheck,
  isLoading,
  onRetry,
}: HealthcheckPanelProps) {
  const isRuntimeUnavailable = error?.code === DESKTOP_RUNTIME_UNAVAILABLE_CODE;
  const errorDetails = isRuntimeUnavailable
    ? "Tauri desktop bridge unavailable in this browser preview."
    : (error?.details ?? "No desktop error reported.");

  return (
    <section
      className="surface-card surface-card--accent"
      data-state={
        isRuntimeUnavailable
          ? "preview"
          : error
            ? "error"
            : (healthcheck?.status ?? "idle")
      }
    >
      <PanelHeader
        eyebrow="Frontend to backend"
        meta={
          <StatusBadge
            size="md"
            tone={getStatusTone(isLoading, error, healthcheck)}
          >
            {getStatusLabel(isLoading, error, healthcheck)}
          </StatusBadge>
        }
        title="Desktop handshake"
        titleLevel={2}
      />

      <PanelMessage
        className="health-panel__message"
        tone={
          isRuntimeUnavailable
            ? "warning"
            : error
              ? "danger"
              : isLoading
                ? "info"
                : "neutral"
        }
      >
        {error?.message ??
          healthcheck?.message ??
          "Waiting for the first backend response."}
      </PanelMessage>

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
          <dd>{errorDetails}</dd>
        </div>
      </dl>

      <ActionButton
        disabled={isLoading}
        mobileFullWidth
        onClick={onRetry}
        size="md"
        variant="primary"
      >
        {isLoading ? "Checking backend..." : "Run healthcheck again"}
      </ActionButton>
    </section>
  );
}
