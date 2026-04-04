import { useEffect, useMemo, useRef, useState } from "react";
import type {
  GlossaryStatus,
  GlossarySummary,
  ProjectSummary,
  UpdateGlossaryInput,
} from "../../shared/desktop";
import type { DesktopCommandError } from "../lib/desktop";
import { GlossaryEntriesPanel } from "./GlossaryEntriesPanel";

interface GlossaryWorkspaceProps {
  activeGlossary: GlossarySummary | null;
  activeGlossaryCount: number;
  archivedGlossaryCount: number;
  error: DesktopCommandError | null;
  glossaries: GlossarySummary[];
  isCreating: boolean;
  isLoading: boolean;
  isSaving: boolean;
  onOpenGlossary: (glossaryId: string) => Promise<boolean>;
  onSubmitGlossary: (input: {
    description?: string;
    name: string;
    projectId?: string;
  }) => Promise<boolean>;
  onUpdateGlossary: (input: UpdateGlossaryInput) => Promise<boolean>;
  openingGlossaryId: string | null;
  onReloadGlossaries: () => Promise<void>;
  projects: ProjectSummary[];
  totalGlossaryCount: number;
}

function formatTimestamp(timestamp: number) {
  return new Date(timestamp * 1000).toLocaleString();
}

function projectLabelById(projects: ProjectSummary[]) {
  return new Map(projects.map((project) => [project.id, project.name]));
}

export function GlossaryWorkspace({
  activeGlossary,
  activeGlossaryCount,
  archivedGlossaryCount,
  error,
  glossaries,
  isCreating,
  isLoading,
  isSaving,
  onOpenGlossary,
  onSubmitGlossary,
  onUpdateGlossary,
  openingGlossaryId,
  onReloadGlossaries,
  projects,
  totalGlossaryCount,
}: GlossaryWorkspaceProps) {
  const [createDescription, setCreateDescription] = useState("");
  const [createName, setCreateName] = useState("");
  const [createProjectId, setCreateProjectId] = useState("");
  const [draftDescription, setDraftDescription] = useState("");
  const [draftName, setDraftName] = useState("");
  const [draftProjectId, setDraftProjectId] = useState("");
  const [draftStatus, setDraftStatus] = useState<GlossaryStatus>("active");
  const [hasUnsavedEntryChanges, setHasUnsavedEntryChanges] = useState(false);
  const previousGlossaryIdRef = useRef<string | null>(null);

  useEffect(() => {
    const nextGlossaryId = activeGlossary?.id ?? null;
    const hasGlossaryChanged = previousGlossaryIdRef.current !== nextGlossaryId;

    setDraftName(activeGlossary?.name ?? "");
    setDraftDescription(activeGlossary?.description ?? "");
    setDraftProjectId(activeGlossary?.projectId ?? "");
    setDraftStatus(activeGlossary?.status ?? "active");

    if (hasGlossaryChanged) {
      setHasUnsavedEntryChanges(false);
    }

    previousGlossaryIdRef.current = nextGlossaryId;
  }, [activeGlossary]);

  const projectNames = useMemo(() => projectLabelById(projects), [projects]);
  const isDirty = useMemo(() => {
    if (!activeGlossary) {
      return false;
    }

    return (
      draftName !== activeGlossary.name ||
      draftDescription !== (activeGlossary.description ?? "") ||
      draftProjectId !== (activeGlossary.projectId ?? "") ||
      draftStatus !== activeGlossary.status
    );
  }, [
    activeGlossary,
    draftDescription,
    draftName,
    draftProjectId,
    draftStatus,
  ]);

  async function handleCreateSubmit(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();

    const wasCreated = await onSubmitGlossary({
      description: createDescription,
      name: createName,
      projectId: createProjectId || undefined,
    });

    if (wasCreated) {
      setCreateName("");
      setCreateDescription("");
      setCreateProjectId("");
    }
  }

  async function handleUpdateSubmit(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();

    if (!activeGlossary) {
      return;
    }

    await onUpdateGlossary({
      glossaryId: activeGlossary.id,
      name: draftName,
      description: draftDescription || undefined,
      projectId: draftProjectId || undefined,
      status: draftStatus,
    });
  }

  async function handleOpenGlossary(glossaryId: string) {
    if (glossaryId === activeGlossary?.id) {
      return;
    }

    if (
      (isDirty || hasUnsavedEntryChanges) &&
      !window.confirm(
        "You have unsaved glossary or terminology changes. Open another glossary and discard them?",
      )
    ) {
      return;
    }

    await onOpenGlossary(glossaryId);
  }

  async function handleToggleStatus() {
    if (!activeGlossary) {
      return;
    }

    await onUpdateGlossary({
      glossaryId: activeGlossary.id,
      name: draftName,
      description: draftDescription || undefined,
      projectId: draftProjectId || undefined,
      status: activeGlossary.status === "active" ? "archived" : "active",
    });
  }

  return (
    <section className="surface-card surface-card--accent">
      <div className="surface-card__heading">
        <div>
          <p className="surface-card__eyebrow">Glossaries</p>
          <h2>Persisted containers plus terminology entries</h2>
          <p className="surface-card__copy">
            D2 keeps the D1 glossary container intact and adds CRUD for source
            and target terminology, basic variants, and forbidden terms without
            mixing in AI behavior or project defaults.
          </p>
        </div>

        <dl className="glossary-metrics">
          <div>
            <dt>Total</dt>
            <dd>{totalGlossaryCount}</dd>
          </div>
          <div>
            <dt>Active</dt>
            <dd>{activeGlossaryCount}</dd>
          </div>
          <div>
            <dt>Archived</dt>
            <dd>{archivedGlossaryCount}</dd>
          </div>
        </dl>
      </div>

      <div className="glossary-grid">
        <section className="workspace-panel">
          <p className="surface-card__eyebrow">Create glossary</p>
          <h3>Persist a reusable editorial container</h3>
          <p className="surface-card__copy">
            Keep the metadata intentionally small. The glossary remains the
            explicit owner of all D2 terminology entries.
          </p>

          <form className="project-form" onSubmit={handleCreateSubmit}>
            <label className="field-group">
              <span>Glossary name</span>
              <input
                autoComplete="off"
                className="field-control"
                disabled={isCreating}
                maxLength={120}
                onChange={(event) => setCreateName(event.target.value)}
                placeholder="Cardiology core terminology"
                required
                value={createName}
              />
            </label>

            <label className="field-group">
              <span>Project scope</span>
              <select
                className="field-control"
                disabled={isCreating}
                onChange={(event) => setCreateProjectId(event.target.value)}
                value={createProjectId}
              >
                <option value="">Reusable across projects</option>
                {projects.map((project) => (
                  <option key={project.id} value={project.id}>
                    {project.name}
                  </option>
                ))}
              </select>
            </label>

            <label className="field-group">
              <span>Short description</span>
              <textarea
                className="field-control field-control--textarea"
                disabled={isCreating}
                maxLength={1000}
                onChange={(event) => setCreateDescription(event.target.value)}
                placeholder="Optional editorial framing for the termbase."
                rows={4}
                value={createDescription}
              />
            </label>

            <div className="project-form__footer">
              <button
                className="app-shell__button"
                disabled={isCreating}
                type="submit"
              >
                {isCreating ? "Creating glossary..." : "Create glossary"}
              </button>

              <span className="project-form__hint">
                New glossaries open immediately after persistence.
              </span>
            </div>

            {error ? (
              <p className="form-error" role="alert">
                {error.message}
              </p>
            ) : null}
          </form>
        </section>

        <section className="workspace-panel">
          <div className="surface-card__heading">
            <div>
              <p className="surface-card__eyebrow">Glossary list</p>
              <h3>Open an existing glossary</h3>
            </div>

            <strong className="status-pill">{totalGlossaryCount} total</strong>
          </div>

          {isLoading ? (
            <p className="surface-card__copy">
              Loading persisted glossaries...
            </p>
          ) : null}

          {!isLoading && glossaries.length === 0 ? (
            <p className="surface-card__copy">
              No glossary exists yet. Create the first reusable container to
              leave D2 with a persisted terminology home.
            </p>
          ) : null}

          {glossaries.length > 0 ? (
            <ul className="glossary-list">
              {glossaries.map((glossary) => {
                const linkedProject =
                  glossary.projectId && projectNames.get(glossary.projectId);
                const isActive = glossary.id === activeGlossary?.id;
                const isOpening = glossary.id === openingGlossaryId;

                return (
                  <li key={glossary.id}>
                    <button
                      className="project-list__item glossary-list__item"
                      data-active={isActive}
                      disabled={isOpening}
                      onClick={() => void handleOpenGlossary(glossary.id)}
                      type="button"
                    >
                      <div className="project-list__item-heading">
                        <strong>{glossary.name}</strong>
                        <span>{isActive ? "Open" : "Manage glossary"}</span>
                      </div>

                      <p>
                        {glossary.description ??
                          "No description yet. This glossary is ready for persisted terminology."}
                      </p>

                      <dl className="project-list__meta">
                        <div>
                          <dt>Status</dt>
                          <dd>{glossary.status}</dd>
                        </div>
                        <div>
                          <dt>Scope</dt>
                          <dd>{linkedProject ?? "Reusable"}</dd>
                        </div>
                        <div>
                          <dt>Opened</dt>
                          <dd>{formatTimestamp(glossary.lastOpenedAt)}</dd>
                        </div>
                        <div>
                          <dt>Updated</dt>
                          <dd>{formatTimestamp(glossary.updatedAt)}</dd>
                        </div>
                      </dl>
                    </button>
                  </li>
                );
              })}
            </ul>
          ) : null}
        </section>
      </div>

      <section className="workspace-panel glossary-detail">
        <div className="surface-card__heading">
          <div>
            <p className="surface-card__eyebrow">Glossary detail</p>
            <h3>
              {activeGlossary ? activeGlossary.name : "No glossary open yet"}
            </h3>
          </div>

          {activeGlossary ? (
            <button
              className="document-action-button"
              disabled={isSaving}
              onClick={() => void handleToggleStatus()}
              type="button"
            >
              {activeGlossary.status === "active"
                ? "Archive glossary"
                : "Restore glossary"}
            </button>
          ) : null}
        </div>

        {!activeGlossary ? (
          <div className="glossary-empty-state">
            <p className="surface-card__copy">
              Select a glossary from the list to edit its metadata and manage
              the terminology entries that belong to it.
            </p>

            <ul className="readiness-list">
              <li>The glossary remains independent from project defaults.</li>
              <li>
                Project linkage is optional and never assigned by default.
              </li>
              <li>
                Archiving continues to act as the glossary-level soft delete.
              </li>
            </ul>
          </div>
        ) : (
          <>
            <form className="project-form" onSubmit={handleUpdateSubmit}>
              <label className="field-group">
                <span>Glossary name</span>
                <input
                  className="field-control"
                  disabled={isSaving}
                  maxLength={120}
                  onChange={(event) => setDraftName(event.target.value)}
                  required
                  value={draftName}
                />
              </label>

              <label className="field-group">
                <span>Project scope</span>
                <select
                  className="field-control"
                  disabled={isSaving}
                  onChange={(event) => setDraftProjectId(event.target.value)}
                  value={draftProjectId}
                >
                  <option value="">Reusable across projects</option>
                  {projects.map((project) => (
                    <option key={project.id} value={project.id}>
                      {project.name}
                    </option>
                  ))}
                </select>
              </label>

              <label className="field-group">
                <span>Status</span>
                <select
                  className="field-control"
                  disabled={isSaving}
                  onChange={(event) =>
                    setDraftStatus(event.target.value as GlossaryStatus)
                  }
                  value={draftStatus}
                >
                  <option value="active">Active</option>
                  <option value="archived">Archived</option>
                </select>
              </label>

              <label className="field-group">
                <span>Description</span>
                <textarea
                  className="field-control field-control--textarea"
                  disabled={isSaving}
                  maxLength={1000}
                  onChange={(event) => setDraftDescription(event.target.value)}
                  rows={5}
                  value={draftDescription}
                />
              </label>

              <dl className="detail-list">
                <div>
                  <dt>Glossary ID</dt>
                  <dd>{activeGlossary.id}</dd>
                </div>
                <div>
                  <dt>Created</dt>
                  <dd>{formatTimestamp(activeGlossary.createdAt)}</dd>
                </div>
                <div>
                  <dt>Opened</dt>
                  <dd>{formatTimestamp(activeGlossary.lastOpenedAt)}</dd>
                </div>
                <div>
                  <dt>Updated</dt>
                  <dd>{formatTimestamp(activeGlossary.updatedAt)}</dd>
                </div>
              </dl>

              <div className="project-form__footer">
                <button
                  className="app-shell__button"
                  disabled={isSaving}
                  type="submit"
                >
                  {isSaving ? "Saving glossary..." : "Save glossary"}
                </button>

                <span className="project-form__hint">
                  {isDirty
                    ? "You have unsaved changes in this glossary."
                    : "Entries and variants remain explicitly attached to this glossary."}
                </span>
              </div>

              {error ? (
                <p className="form-error" role="alert">
                  {error.message}
                </p>
              ) : null}
            </form>

            <GlossaryEntriesPanel
              glossary={activeGlossary}
              onDirtyChange={setHasUnsavedEntryChanges}
              onEntriesChanged={onReloadGlossaries}
            />
          </>
        )}
      </section>
    </section>
  );
}
