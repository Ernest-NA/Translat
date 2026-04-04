import { useEffect, useMemo, useRef, useState } from "react";
import type {
  RuleSetStatus,
  RuleSetSummary,
  UpdateRuleSetInput,
} from "../../shared/desktop";
import type { DesktopCommandError } from "../lib/desktop";
import { RuleSetRulesPanel } from "./RuleSetRulesPanel";

interface RuleSetsWorkspaceProps {
  activeRuleSet: RuleSetSummary | null;
  activeRuleSetCount: number;
  archivedRuleSetCount: number;
  error: DesktopCommandError | null;
  isCreating: boolean;
  isLoading: boolean;
  isSaving: boolean;
  onOpenRuleSet: (ruleSetId: string) => Promise<boolean>;
  onReloadRuleSets: () => Promise<void>;
  onSubmitRuleSet: (input: {
    description?: string;
    name: string;
  }) => Promise<boolean>;
  onUpdateRuleSet: (input: UpdateRuleSetInput) => Promise<boolean>;
  openingRuleSetId: string | null;
  ruleSets: RuleSetSummary[];
  totalRuleSetCount: number;
}

function formatTimestamp(timestamp: number) {
  return new Date(timestamp * 1000).toLocaleString();
}

export function RuleSetsWorkspace({
  activeRuleSet,
  activeRuleSetCount,
  archivedRuleSetCount,
  error,
  isCreating,
  isLoading,
  isSaving,
  onOpenRuleSet,
  onReloadRuleSets,
  onSubmitRuleSet,
  onUpdateRuleSet,
  openingRuleSetId,
  ruleSets,
  totalRuleSetCount,
}: RuleSetsWorkspaceProps) {
  const [createDescription, setCreateDescription] = useState("");
  const [createName, setCreateName] = useState("");
  const [draftDescription, setDraftDescription] = useState("");
  const [draftName, setDraftName] = useState("");
  const [draftStatus, setDraftStatus] = useState<RuleSetStatus>("active");
  const [hasUnsavedRuleChanges, setHasUnsavedRuleChanges] = useState(false);
  const previousRuleSetIdRef = useRef<string | null>(null);
  const pendingRuleSetSyncRef = useRef<UpdateRuleSetInput | null>(null);

  useEffect(() => {
    const nextRuleSetId = activeRuleSet?.id ?? null;
    const hasRuleSetChanged = previousRuleSetIdRef.current !== nextRuleSetId;
    const pendingRuleSetSync = pendingRuleSetSyncRef.current;
    const matchesPendingRuleSetSync =
      !!activeRuleSet &&
      pendingRuleSetSync?.ruleSetId === activeRuleSet.id &&
      pendingRuleSetSync.name === activeRuleSet.name &&
      (pendingRuleSetSync.description ?? "") ===
        (activeRuleSet.description ?? "") &&
      pendingRuleSetSync.status === activeRuleSet.status;

    if (!hasRuleSetChanged && !matchesPendingRuleSetSync) {
      return;
    }

    setDraftName(activeRuleSet?.name ?? "");
    setDraftDescription(activeRuleSet?.description ?? "");
    setDraftStatus(activeRuleSet?.status ?? "active");

    if (hasRuleSetChanged) {
      setHasUnsavedRuleChanges(false);
    }

    if (matchesPendingRuleSetSync) {
      pendingRuleSetSyncRef.current = null;
    }

    previousRuleSetIdRef.current = nextRuleSetId;
  }, [activeRuleSet]);

  const isDirty = useMemo(() => {
    if (!activeRuleSet) {
      return false;
    }

    return (
      draftName !== activeRuleSet.name ||
      draftDescription !== (activeRuleSet.description ?? "") ||
      draftStatus !== activeRuleSet.status
    );
  }, [activeRuleSet, draftDescription, draftName, draftStatus]);

  async function handleCreateSubmit(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();

    if (
      (isDirty || hasUnsavedRuleChanges) &&
      !window.confirm(
        "You have unsaved rule-set or rule changes. Create a new rule set and discard them?",
      )
    ) {
      return;
    }

    const wasCreated = await onSubmitRuleSet({
      description: createDescription || undefined,
      name: createName,
    });

    if (wasCreated) {
      setCreateName("");
      setCreateDescription("");
    }
  }

  async function handleUpdateSubmit(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();

    if (!activeRuleSet) {
      return;
    }

    const ruleSetUpdate: UpdateRuleSetInput = {
      ruleSetId: activeRuleSet.id,
      name: draftName,
      description: draftDescription || undefined,
      status: draftStatus,
    };

    pendingRuleSetSyncRef.current = ruleSetUpdate;

    const wasUpdated = await onUpdateRuleSet(ruleSetUpdate);

    if (!wasUpdated) {
      pendingRuleSetSyncRef.current = null;
    }
  }

  async function handleOpenRuleSet(ruleSetId: string) {
    if (ruleSetId === activeRuleSet?.id) {
      return;
    }

    if (
      (isDirty || hasUnsavedRuleChanges) &&
      !window.confirm(
        "You have unsaved rule-set or rule changes. Open another rule set and discard them?",
      )
    ) {
      return;
    }

    await onOpenRuleSet(ruleSetId);
  }

  async function handleToggleStatus() {
    if (!activeRuleSet) {
      return;
    }

    const ruleSetUpdate: UpdateRuleSetInput = {
      ruleSetId: activeRuleSet.id,
      name: draftName,
      description: draftDescription || undefined,
      status: activeRuleSet.status === "active" ? "archived" : "active",
    };

    pendingRuleSetSyncRef.current = ruleSetUpdate;

    const wasUpdated = await onUpdateRuleSet(ruleSetUpdate);

    if (!wasUpdated) {
      pendingRuleSetSyncRef.current = null;
    }
  }

  return (
    <section className="surface-card surface-card--accent">
      <div className="surface-card__heading">
        <div>
          <p className="surface-card__eyebrow">Rule sets</p>
          <h2>Reusable constraints and editorial preferences</h2>
          <p className="surface-card__copy">
            D4 adds persisted rule sets plus individual rules with explicit
            severity, enablement, and guidance, while keeping automated
            execution and AI integration out of scope.
          </p>
        </div>

        <dl className="glossary-metrics">
          <div>
            <dt>Total</dt>
            <dd>{totalRuleSetCount}</dd>
          </div>
          <div>
            <dt>Active</dt>
            <dd>{activeRuleSetCount}</dd>
          </div>
          <div>
            <dt>Archived</dt>
            <dd>{archivedRuleSetCount}</dd>
          </div>
        </dl>
      </div>

      <div className="glossary-grid">
        <section className="workspace-panel">
          <p className="surface-card__eyebrow">Create rule set</p>
          <h3>Persist a reusable editorial container</h3>
          <p className="surface-card__copy">
            Keep the container small and explicit. The rule set owns its rules
            and remains independent of projects or AI usage in this phase.
          </p>

          <form className="project-form" onSubmit={handleCreateSubmit}>
            <label className="field-group">
              <span>Rule-set name</span>
              <input
                autoComplete="off"
                className="field-control"
                disabled={isCreating}
                maxLength={120}
                onChange={(event) => setCreateName(event.target.value)}
                placeholder="Medical warning guardrails"
                required
                value={createName}
              />
            </label>

            <label className="field-group">
              <span>Short description</span>
              <textarea
                className="field-control field-control--textarea"
                disabled={isCreating}
                maxLength={1000}
                onChange={(event) => setCreateDescription(event.target.value)}
                rows={4}
                value={createDescription}
              />
            </label>

            <div className="project-form__footer">
              <span className="project-form__hint">
                Archived rule sets stay reusable later, but do not disappear
                from the workspace.
              </span>

              <button
                className="app-shell__button"
                disabled={isCreating}
                type="submit"
              >
                {isCreating ? "Creating rule set..." : "Create rule set"}
              </button>
            </div>
          </form>
        </section>

        <section className="workspace-panel">
          <p className="surface-card__eyebrow">Rule-set list</p>
          <h3>Open a reusable set and manage its rules</h3>
          <p className="surface-card__copy">
            Rule sets are ordered by status and recent use. Opening one makes
            its rules available in the management panel below.
          </p>

          {error ? <p className="form-error">{error.message}</p> : null}

          {isLoading ? (
            <p className="project-form__hint">Loading rule sets...</p>
          ) : ruleSets.length === 0 ? (
            <p className="project-form__hint">
              No rule sets are persisted yet.
            </p>
          ) : (
            <ol className="glossary-list">
              {ruleSets.map((ruleSet) => (
                <li className="glossary-list__item" key={ruleSet.id}>
                  <button
                    className="project-list__item"
                    data-active={ruleSet.id === activeRuleSet?.id}
                    disabled={openingRuleSetId === ruleSet.id}
                    onClick={() => void handleOpenRuleSet(ruleSet.id)}
                    type="button"
                  >
                    <div className="project-list__item-heading">
                      <strong>{ruleSet.name}</strong>
                      <span>
                        {openingRuleSetId === ruleSet.id
                          ? "Opening..."
                          : ruleSet.status}
                      </span>
                    </div>

                    <p>{ruleSet.description ?? "No description provided."}</p>

                    <dl className="project-list__meta">
                      <div>
                        <dt>Updated</dt>
                        <dd>{formatTimestamp(ruleSet.updatedAt)}</dd>
                      </div>
                      <div>
                        <dt>Opened</dt>
                        <dd>{formatTimestamp(ruleSet.lastOpenedAt)}</dd>
                      </div>
                    </dl>
                  </button>
                </li>
              ))}
            </ol>
          )}

          {activeRuleSet ? (
            <section className="glossary-detail">
              <p className="surface-card__eyebrow">Selected rule set</p>
              <h3>Edit persisted metadata</h3>

              <form className="project-form" onSubmit={handleUpdateSubmit}>
                <label className="field-group">
                  <span>Name</span>
                  <input
                    autoComplete="off"
                    className="field-control"
                    disabled={isSaving}
                    maxLength={120}
                    onChange={(event) => setDraftName(event.target.value)}
                    required
                    value={draftName}
                  />
                </label>

                <label className="field-group">
                  <span>Description</span>
                  <textarea
                    className="field-control field-control--textarea"
                    disabled={isSaving}
                    maxLength={1000}
                    onChange={(event) =>
                      setDraftDescription(event.target.value)
                    }
                    rows={4}
                    value={draftDescription}
                  />
                </label>

                <label className="field-group">
                  <span>Status</span>
                  <select
                    className="field-control"
                    disabled={isSaving}
                    onChange={(event) =>
                      setDraftStatus(event.target.value as RuleSetStatus)
                    }
                    value={draftStatus}
                  >
                    <option value="active">Active</option>
                    <option value="archived">Archived</option>
                  </select>
                </label>

                <dl className="detail-list detail-list--single">
                  <div>
                    <dt>Created</dt>
                    <dd>{formatTimestamp(activeRuleSet.createdAt)}</dd>
                  </div>
                  <div>
                    <dt>Updated</dt>
                    <dd>{formatTimestamp(activeRuleSet.updatedAt)}</dd>
                  </div>
                  <div>
                    <dt>Opened</dt>
                    <dd>{formatTimestamp(activeRuleSet.lastOpenedAt)}</dd>
                  </div>
                </dl>

                <div className="project-form__footer">
                  <span className="project-form__hint">
                    {isDirty
                      ? "Unsaved rule-set changes detected."
                      : "Rule-set metadata is synchronized."}
                  </span>

                  <div className="document-list__actions">
                    <button
                      className="document-action-button"
                      disabled={isSaving}
                      onClick={() => void handleToggleStatus()}
                      type="button"
                    >
                      {activeRuleSet.status === "active"
                        ? "Archive rule set"
                        : "Restore rule set"}
                    </button>

                    <button
                      className="app-shell__button"
                      disabled={isSaving || !isDirty}
                      type="submit"
                    >
                      {isSaving ? "Saving rule set..." : "Save rule set"}
                    </button>
                  </div>
                </div>
              </form>
            </section>
          ) : null}
        </section>
      </div>

      <RuleSetRulesPanel
        key={activeRuleSet?.id ?? "empty-rule-set"}
        onDirtyChange={setHasUnsavedRuleChanges}
        onReloadRuleSets={onReloadRuleSets}
        ruleSetId={activeRuleSet?.id ?? null}
      />
    </section>
  );
}
