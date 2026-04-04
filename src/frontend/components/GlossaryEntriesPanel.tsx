import { useEffect, useMemo, useState } from "react";
import type {
  GlossaryEntryStatus,
  GlossarySummary,
} from "../../shared/desktop";
import { useGlossaryEntries } from "../hooks/useGlossaryEntries";

interface GlossaryEntriesPanelProps {
  glossary: GlossarySummary;
  onDirtyChange?: (isDirty: boolean) => void;
  onEntriesChanged?: () => Promise<void> | void;
}

function formatTimestamp(timestamp: number) {
  return new Date(timestamp * 1000).toLocaleString();
}

function joinTerms(terms: string[]) {
  return terms.join("\n");
}

function parseTerms(value: string) {
  return value
    .split(/[\n,;]+/u)
    .map((term) => term.trim())
    .filter((term) => term.length > 0);
}

function variantSummary(
  sourceVariants: string[],
  targetVariants: string[],
  forbiddenTerms: string[],
) {
  return [
    `${sourceVariants.length} source variants`,
    `${targetVariants.length} target variants`,
    `${forbiddenTerms.length} forbidden terms`,
  ].join(" | ");
}

export function GlossaryEntriesPanel({
  glossary,
  onDirtyChange,
  onEntriesChanged,
}: GlossaryEntriesPanelProps) {
  const {
    activeEntry,
    activeEntryCount,
    archivedEntryCount,
    entries,
    error,
    isCreating,
    isLoading,
    isSaving,
    saveEntry,
    selectEntry,
    selectedEntryId,
    submitEntry,
    totalEntryCount,
  } = useGlossaryEntries(glossary.id);
  const [createContextNote, setCreateContextNote] = useState("");
  const [createForbiddenTerms, setCreateForbiddenTerms] = useState("");
  const [createSourceTerm, setCreateSourceTerm] = useState("");
  const [createSourceVariants, setCreateSourceVariants] = useState("");
  const [createTargetTerm, setCreateTargetTerm] = useState("");
  const [createTargetVariants, setCreateTargetVariants] = useState("");
  const [draftContextNote, setDraftContextNote] = useState("");
  const [draftForbiddenTerms, setDraftForbiddenTerms] = useState("");
  const [draftSourceTerm, setDraftSourceTerm] = useState("");
  const [draftSourceVariants, setDraftSourceVariants] = useState("");
  const [draftStatus, setDraftStatus] = useState<GlossaryEntryStatus>("active");
  const [draftTargetTerm, setDraftTargetTerm] = useState("");
  const [draftTargetVariants, setDraftTargetVariants] = useState("");

  useEffect(() => {
    setDraftSourceTerm(activeEntry?.sourceTerm ?? "");
    setDraftTargetTerm(activeEntry?.targetTerm ?? "");
    setDraftContextNote(activeEntry?.contextNote ?? "");
    setDraftSourceVariants(joinTerms(activeEntry?.sourceVariants ?? []));
    setDraftTargetVariants(joinTerms(activeEntry?.targetVariants ?? []));
    setDraftForbiddenTerms(joinTerms(activeEntry?.forbiddenTerms ?? []));
    setDraftStatus(activeEntry?.status ?? "active");
  }, [activeEntry]);

  const hasUnsavedCreateDraft = useMemo(
    () =>
      createSourceTerm.trim().length > 0 ||
      createTargetTerm.trim().length > 0 ||
      createContextNote.trim().length > 0 ||
      parseTerms(createSourceVariants).length > 0 ||
      parseTerms(createTargetVariants).length > 0 ||
      parseTerms(createForbiddenTerms).length > 0,
    [
      createContextNote,
      createForbiddenTerms,
      createSourceTerm,
      createSourceVariants,
      createTargetTerm,
      createTargetVariants,
    ],
  );

  const hasUnsavedEditChanges = useMemo(() => {
    if (!activeEntry) {
      return false;
    }

    return (
      draftSourceTerm !== activeEntry.sourceTerm ||
      draftTargetTerm !== activeEntry.targetTerm ||
      draftContextNote !== (activeEntry.contextNote ?? "") ||
      draftSourceVariants !== joinTerms(activeEntry.sourceVariants) ||
      draftTargetVariants !== joinTerms(activeEntry.targetVariants) ||
      draftForbiddenTerms !== joinTerms(activeEntry.forbiddenTerms) ||
      draftStatus !== activeEntry.status
    );
  }, [
    activeEntry,
    draftContextNote,
    draftForbiddenTerms,
    draftSourceTerm,
    draftSourceVariants,
    draftStatus,
    draftTargetTerm,
    draftTargetVariants,
  ]);

  useEffect(() => {
    onDirtyChange?.(hasUnsavedCreateDraft || hasUnsavedEditChanges);
  }, [hasUnsavedCreateDraft, hasUnsavedEditChanges, onDirtyChange]);

  async function handleCreateSubmit(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();

    if (
      hasUnsavedEditChanges &&
      !window.confirm(
        "You have unsaved terminology changes in the selected entry. Create a new entry and discard them?",
      )
    ) {
      return;
    }

    const wasCreated = await submitEntry({
      sourceTerm: createSourceTerm,
      targetTerm: createTargetTerm,
      contextNote: createContextNote || undefined,
      sourceVariants: parseTerms(createSourceVariants),
      targetVariants: parseTerms(createTargetVariants),
      forbiddenTerms: parseTerms(createForbiddenTerms),
    });

    if (wasCreated) {
      await onEntriesChanged?.();
      setCreateSourceTerm("");
      setCreateTargetTerm("");
      setCreateContextNote("");
      setCreateSourceVariants("");
      setCreateTargetVariants("");
      setCreateForbiddenTerms("");
    }
  }

  async function handleUpdateSubmit(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();

    if (!activeEntry) {
      return;
    }

    const wasSaved = await saveEntry({
      glossaryEntryId: activeEntry.id,
      glossaryId: glossary.id,
      sourceTerm: draftSourceTerm,
      targetTerm: draftTargetTerm,
      contextNote: draftContextNote || undefined,
      sourceVariants: parseTerms(draftSourceVariants),
      targetVariants: parseTerms(draftTargetVariants),
      forbiddenTerms: parseTerms(draftForbiddenTerms),
      status: draftStatus,
    });

    if (wasSaved) {
      await onEntriesChanged?.();
    }
  }

  async function handleSelectEntry(entryId: string) {
    if (entryId === activeEntry?.id) {
      return;
    }

    if (
      hasUnsavedEditChanges &&
      !window.confirm(
        "You have unsaved terminology changes. Open another entry and discard them?",
      )
    ) {
      return;
    }

    selectEntry(entryId);
  }

  async function handleToggleStatus() {
    if (!activeEntry) {
      return;
    }

    const nextStatus: GlossaryEntryStatus =
      activeEntry.status === "active" ? "archived" : "active";

    const wasSaved = await saveEntry({
      glossaryEntryId: activeEntry.id,
      glossaryId: glossary.id,
      sourceTerm: draftSourceTerm,
      targetTerm: draftTargetTerm,
      contextNote: draftContextNote || undefined,
      sourceVariants: parseTerms(draftSourceVariants),
      targetVariants: parseTerms(draftTargetVariants),
      forbiddenTerms: parseTerms(draftForbiddenTerms),
      status: nextStatus,
    });

    if (wasSaved) {
      await onEntriesChanged?.();
    }
  }

  return (
    <section className="workspace-panel glossary-entry-workspace">
      <div className="surface-card__heading">
        <div>
          <p className="surface-card__eyebrow">Terminology entries</p>
          <h3>{glossary.name}</h3>
          <p className="surface-card__copy">
            D2 keeps terminology inside the selected glossary so source terms,
            target terms, variants, and forbidden terms remain explicitly
            attached to one persisted container.
          </p>
        </div>

        <dl className="glossary-metrics glossary-metrics--entries">
          <div>
            <dt>Total</dt>
            <dd>{totalEntryCount}</dd>
          </div>
          <div>
            <dt>Active</dt>
            <dd>{activeEntryCount}</dd>
          </div>
          <div>
            <dt>Archived</dt>
            <dd>{archivedEntryCount}</dd>
          </div>
        </dl>
      </div>

      <div className="glossary-entry-grid">
        <section className="workspace-panel">
          <p className="surface-card__eyebrow">Create entry</p>
          <h3>Add terminology to this glossary</h3>
          <p className="surface-card__copy">
            Keep D2 focused on practical terminology. Variants and forbidden
            terms stay basic, with one term per line or comma-separated.
          </p>

          <form className="project-form" onSubmit={handleCreateSubmit}>
            <label className="field-group">
              <span>Source term</span>
              <input
                className="field-control"
                disabled={isCreating}
                maxLength={240}
                onChange={(event) => setCreateSourceTerm(event.target.value)}
                placeholder="black box warning"
                required
                value={createSourceTerm}
              />
            </label>

            <label className="field-group">
              <span>Target term</span>
              <input
                className="field-control"
                disabled={isCreating}
                maxLength={240}
                onChange={(event) => setCreateTargetTerm(event.target.value)}
                placeholder="advertencia de recuadro negro"
                required
                value={createTargetTerm}
              />
            </label>

            <label className="field-group">
              <span>Context note</span>
              <textarea
                className="field-control field-control--textarea"
                disabled={isCreating}
                maxLength={2000}
                onChange={(event) => setCreateContextNote(event.target.value)}
                rows={4}
                value={createContextNote}
              />
            </label>

            <label className="field-group">
              <span>Source variants</span>
              <textarea
                className="field-control field-control--textarea field-control--compact"
                disabled={isCreating}
                onChange={(event) =>
                  setCreateSourceVariants(event.target.value)
                }
                placeholder="boxed warning"
                rows={3}
                value={createSourceVariants}
              />
            </label>

            <label className="field-group">
              <span>Target variants</span>
              <textarea
                className="field-control field-control--textarea field-control--compact"
                disabled={isCreating}
                onChange={(event) =>
                  setCreateTargetVariants(event.target.value)
                }
                placeholder="advertencia destacada"
                rows={3}
                value={createTargetVariants}
              />
            </label>

            <label className="field-group">
              <span>Forbidden terms</span>
              <textarea
                className="field-control field-control--textarea field-control--compact"
                disabled={isCreating}
                onChange={(event) =>
                  setCreateForbiddenTerms(event.target.value)
                }
                placeholder="caja negra"
                rows={3}
                value={createForbiddenTerms}
              />
            </label>

            <div className="project-form__footer">
              <button
                className="app-shell__button"
                disabled={isCreating}
                type="submit"
              >
                {isCreating ? "Creating entry..." : "Create entry"}
              </button>

              <span className="project-form__hint">
                Entries persist directly inside glossary{" "}
                <strong>{glossary.name}</strong>.
              </span>
            </div>
          </form>
        </section>

        <section className="workspace-panel">
          <div className="surface-card__heading">
            <div>
              <p className="surface-card__eyebrow">Entry list</p>
              <h3>Review persisted terms</h3>
            </div>

            <strong className="status-pill">{totalEntryCount} total</strong>
          </div>

          {isLoading ? (
            <p className="surface-card__copy">
              Loading persisted glossary entries...
            </p>
          ) : null}

          {!isLoading && entries.length === 0 ? (
            <p className="surface-card__copy">
              This glossary does not contain entries yet. Add the first source
              and target term to make D2 useful after restart.
            </p>
          ) : null}

          {entries.length > 0 ? (
            <ul className="glossary-list glossary-entry-list">
              {entries.map((entry) => (
                <li key={entry.id}>
                  <button
                    className="project-list__item glossary-list__item"
                    data-active={entry.id === selectedEntryId}
                    onClick={() => void handleSelectEntry(entry.id)}
                    type="button"
                  >
                    <div className="project-list__item-heading">
                      <strong>{entry.sourceTerm}</strong>
                      <span>{entry.status}</span>
                    </div>

                    <p className="glossary-entry-list__target">
                      {entry.targetTerm}
                    </p>
                    <p className="glossary-entry-list__meta">
                      {variantSummary(
                        entry.sourceVariants,
                        entry.targetVariants,
                        entry.forbiddenTerms,
                      )}
                    </p>

                    <dl className="project-list__meta">
                      <div>
                        <dt>Glossary</dt>
                        <dd>{glossary.name}</dd>
                      </div>
                      <div>
                        <dt>Updated</dt>
                        <dd>{formatTimestamp(entry.updatedAt)}</dd>
                      </div>
                    </dl>
                  </button>
                </li>
              ))}
            </ul>
          ) : null}
        </section>
      </div>

      <section className="workspace-panel glossary-entry-detail">
        <div className="surface-card__heading">
          <div>
            <p className="surface-card__eyebrow">Entry detail</p>
            <h3>
              {activeEntry
                ? `${activeEntry.sourceTerm} -> ${activeEntry.targetTerm}`
                : "No glossary entry selected"}
            </h3>
          </div>

          {activeEntry ? (
            <button
              className="document-action-button"
              disabled={isSaving}
              onClick={() => void handleToggleStatus()}
              type="button"
            >
              {activeEntry.status === "active"
                ? "Archive entry"
                : "Restore entry"}
            </button>
          ) : null}
        </div>

        {!activeEntry ? (
          <div className="glossary-empty-state">
            <p className="surface-card__copy">
              Select an entry from this glossary to edit its terms, context, and
              basic variants.
            </p>

            <ul className="readiness-list">
              <li>Each entry stays scoped to the selected glossary id.</li>
              <li>
                Source variants, target variants, and forbidden terms are
                persisted.
              </li>
              <li>No AI integration or automatic matching is mixed into D2.</li>
            </ul>
          </div>
        ) : (
          <form className="project-form" onSubmit={handleUpdateSubmit}>
            <label className="field-group">
              <span>Source term</span>
              <input
                className="field-control"
                disabled={isSaving}
                maxLength={240}
                onChange={(event) => setDraftSourceTerm(event.target.value)}
                required
                value={draftSourceTerm}
              />
            </label>

            <label className="field-group">
              <span>Target term</span>
              <input
                className="field-control"
                disabled={isSaving}
                maxLength={240}
                onChange={(event) => setDraftTargetTerm(event.target.value)}
                required
                value={draftTargetTerm}
              />
            </label>

            <label className="field-group">
              <span>Status</span>
              <select
                className="field-control"
                disabled={isSaving}
                onChange={(event) =>
                  setDraftStatus(event.target.value as GlossaryEntryStatus)
                }
                value={draftStatus}
              >
                <option value="active">Active</option>
                <option value="archived">Archived</option>
              </select>
            </label>

            <label className="field-group">
              <span>Context note</span>
              <textarea
                className="field-control field-control--textarea"
                disabled={isSaving}
                maxLength={2000}
                onChange={(event) => setDraftContextNote(event.target.value)}
                rows={4}
                value={draftContextNote}
              />
            </label>

            <div className="glossary-entry-form-grid">
              <label className="field-group">
                <span>Source variants</span>
                <textarea
                  className="field-control field-control--textarea field-control--compact"
                  disabled={isSaving}
                  onChange={(event) =>
                    setDraftSourceVariants(event.target.value)
                  }
                  rows={4}
                  value={draftSourceVariants}
                />
              </label>

              <label className="field-group">
                <span>Target variants</span>
                <textarea
                  className="field-control field-control--textarea field-control--compact"
                  disabled={isSaving}
                  onChange={(event) =>
                    setDraftTargetVariants(event.target.value)
                  }
                  rows={4}
                  value={draftTargetVariants}
                />
              </label>

              <label className="field-group">
                <span>Forbidden terms</span>
                <textarea
                  className="field-control field-control--textarea field-control--compact"
                  disabled={isSaving}
                  onChange={(event) =>
                    setDraftForbiddenTerms(event.target.value)
                  }
                  rows={4}
                  value={draftForbiddenTerms}
                />
              </label>
            </div>

            <dl className="detail-list">
              <div>
                <dt>Entry ID</dt>
                <dd>{activeEntry.id}</dd>
              </div>
              <div>
                <dt>Glossary ID</dt>
                <dd>{activeEntry.glossaryId}</dd>
              </div>
              <div>
                <dt>Created</dt>
                <dd>{formatTimestamp(activeEntry.createdAt)}</dd>
              </div>
              <div>
                <dt>Updated</dt>
                <dd>{formatTimestamp(activeEntry.updatedAt)}</dd>
              </div>
            </dl>

            <div className="project-form__footer">
              <button
                className="app-shell__button"
                disabled={isSaving}
                type="submit"
              >
                {isSaving ? "Saving entry..." : "Save entry"}
              </button>

              <span className="project-form__hint">
                {hasUnsavedEditChanges
                  ? "You have unsaved terminology changes."
                  : "Entry changes remain scoped to this glossary only."}
              </span>
            </div>
          </form>
        )}

        {error ? (
          <p className="form-error" role="alert">
            {error.message}
          </p>
        ) : null}
      </section>
    </section>
  );
}
