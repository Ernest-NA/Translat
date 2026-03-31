import { DESKTOP_COMMANDS } from "../../shared/desktop";
import { HealthcheckPanel } from "../components/HealthcheckPanel";
import { useHealthcheck } from "../hooks/useHealthcheck";

function formatCheckedAt(value?: number) {
  if (!value) {
    return "Pending first response";
  }

  return new Date(value).toLocaleString();
}

export function AppShell() {
  const { error, healthcheck, isLoading, retry } = useHealthcheck();

  const runtimeLabel = healthcheck
    ? `${healthcheck.environment} | v${healthcheck.version}`
    : "desktop runtime";

  return (
    <main className="app-shell">
      <header className="app-shell__header">
        <div>
          <p className="app-shell__eyebrow">Translat</p>
          <h1>Desktop shell foundation</h1>
          <p className="app-shell__lead">
            A cleaner base application shell for the next modules, with a
            reusable desktop command pattern, explicit runtime feedback, and a
            layout that can grow without rewriting the entry point.
          </p>
        </div>

        <div className="app-shell__header-meta">
          <span>{runtimeLabel}</span>
          <span>Windows desktop</span>
          <span>Ready for B4</span>
        </div>
      </header>

      <section className="app-shell__grid">
        <div className="app-shell__primary">
          <HealthcheckPanel
            error={error}
            healthcheck={healthcheck}
            isLoading={isLoading}
            onRetry={retry}
          />

          <section className="surface-card surface-card--split">
            <div>
              <p className="surface-card__eyebrow">Current capabilities</p>
              <h2>
                Stable shell, explicit runtime handshake, no business logic.
              </h2>
            </div>

            <ul className="capability-list">
              <li>
                Tauri window bootstraps the React app inside the desktop shell.
              </li>
              <li>
                Frontend commands go through a shared, typed desktop wrapper.
              </li>
              <li>
                Command failures surface a normalized error with code and
                details.
              </li>
            </ul>
          </section>
        </div>

        <aside className="app-shell__sidebar">
          <section className="surface-card">
            <p className="surface-card__eyebrow">Command pattern</p>
            <h2>{DESKTOP_COMMANDS.healthcheck}</h2>

            <dl className="detail-list">
              <div>
                <dt>Wrapper</dt>
                <dd>`invokeDesktopCommand`</dd>
              </div>
              <div>
                <dt>Last check</dt>
                <dd>{formatCheckedAt(healthcheck?.checkedAt)}</dd>
              </div>
              <div>
                <dt>Error mode</dt>
                <dd>{error ? error.code : "No error"}</dd>
              </div>
            </dl>
          </section>

          <section className="surface-card">
            <p className="surface-card__eyebrow">Readiness notes</p>
            <ul className="readiness-list">
              <li>
                B2 keeps persistence, login, and business workflows out of
                scope.
              </li>
              <li>
                The shell now exposes clearer extension points for future
                commands.
              </li>
              <li>
                B4 can build on this foundation without revisiting shell wiring.
              </li>
            </ul>
          </section>
        </aside>
      </section>
    </main>
  );
}
