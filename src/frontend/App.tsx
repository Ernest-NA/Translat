import { useCallback, useEffect, useState } from "react";
import type { HealthcheckResponse } from "../shared/desktop";
import { runHealthcheck } from "./lib/desktop";

function App() {
  const [healthcheck, setHealthcheck] = useState<HealthcheckResponse | null>(
    null,
  );
  const [error, setError] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(true);

  const loadHealthcheck = useCallback(async () => {
    setIsLoading(true);
    setError(null);

    try {
      const response = await runHealthcheck();
      setHealthcheck(response);
    } catch (caughtError) {
      setHealthcheck(null);
      setError(
        caughtError instanceof Error
          ? caughtError.message
          : "Unknown desktop error.",
      );
    } finally {
      setIsLoading(false);
    }
  }, []);

  useEffect(() => {
    void loadHealthcheck();
  }, [loadHealthcheck]);

  return (
    <main className="shell">
      <section className="shell__panel">
        <p className="shell__eyebrow">Translat desktop shell</p>
        <h1>Foundation ready for the next layers.</h1>
        <p className="shell__lead">
          This placeholder keeps the Tauri container, the React UI, and the Rust
          backend connected without introducing persistence, business logic, or
          workflow-specific complexity yet.
        </p>

        <div
          className="healthcheck-card"
          data-state={error ? "error" : (healthcheck?.status ?? "idle")}
        >
          <div className="healthcheck-card__header">
            <span>Frontend to backend wiring</span>
            <strong>
              {isLoading ? "Checking" : (healthcheck?.status ?? "error")}
            </strong>
          </div>

          <p className="healthcheck-card__message">
            {error ??
              healthcheck?.message ??
              "Waiting for the first backend response."}
          </p>

          <dl className="healthcheck-card__meta">
            <div>
              <dt>Command</dt>
              <dd>`healthcheck`</dd>
            </div>
            <div>
              <dt>App</dt>
              <dd>{healthcheck?.app ?? "Translat"}</dd>
            </div>
          </dl>
        </div>

        <button
          className="shell__button"
          disabled={isLoading}
          onClick={() => void loadHealthcheck()}
          type="button"
        >
          {isLoading ? "Checking backend..." : "Run healthcheck again"}
        </button>
      </section>
    </main>
  );
}

export default App;
