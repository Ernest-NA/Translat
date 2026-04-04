import { useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import type { RuleSeverity, RuleSummary, RuleType } from "../../shared/desktop";
import { useRuleSetRules } from "../hooks/useRuleSetRules";
import type { DesktopCommandError } from "../lib/desktop";

interface RuleSetRulesPanelProps {
  onDirtyChange?: (isDirty: boolean) => void;
  onReloadRuleSets: () => Promise<void>;
  ruleSetId: string | null;
}

const RULE_TYPE_OPTIONS: Array<{ label: string; value: RuleType }> = [
  { label: "Consistency", value: "consistency" },
  { label: "Preference", value: "preference" },
  { label: "Restriction", value: "restriction" },
];

const RULE_SEVERITY_OPTIONS: Array<{ label: string; value: RuleSeverity }> = [
  { label: "High", value: "high" },
  { label: "Medium", value: "medium" },
  { label: "Low", value: "low" },
];

function formatTimestamp(timestamp: number) {
  return new Date(timestamp * 1000).toLocaleString();
}

function optionLabel<TValue extends string>(
  options: Array<{ label: string; value: TValue }>,
  value: TValue,
) {
  return options.find((option) => option.value === value)?.label ?? value;
}

function ruleSummary(rule: RuleSummary) {
  return [
    optionLabel(RULE_TYPE_OPTIONS, rule.ruleType),
    optionLabel(RULE_SEVERITY_OPTIONS, rule.severity),
    rule.isEnabled ? "Enabled" : "Disabled",
  ].join(" | ");
}

function errorMessage(error: DesktopCommandError | null) {
  return error ? error.message : null;
}

export function RuleSetRulesPanel({
  onDirtyChange,
  onReloadRuleSets,
  ruleSetId,
}: RuleSetRulesPanelProps) {
  const {
    activeRule,
    disabledRuleCount,
    enabledRuleCount,
    error,
    highSeverityRuleCount,
    isCreating,
    isLoading,
    isSaving,
    rules,
    saveRule,
    selectRule,
    selectedRuleId,
    submitRule,
    totalRuleCount,
  } = useRuleSetRules(ruleSetId);
  const [createDescription, setCreateDescription] = useState("");
  const [createGuidance, setCreateGuidance] = useState("");
  const [createIsEnabled, setCreateIsEnabled] = useState(true);
  const [createName, setCreateName] = useState("");
  const [createSeverity, setCreateSeverity] = useState<RuleSeverity>("medium");
  const [createType, setCreateType] = useState<RuleType>("consistency");
  const [draftDescription, setDraftDescription] = useState("");
  const [draftGuidance, setDraftGuidance] = useState("");
  const [draftIsEnabled, setDraftIsEnabled] = useState(true);
  const [draftName, setDraftName] = useState("");
  const [draftSeverity, setDraftSeverity] = useState<RuleSeverity>("medium");
  const [draftType, setDraftType] = useState<RuleType>("consistency");
  const previousRuleRevisionRef = useRef<string | null>(null);

  useEffect(() => {
    const nextRevision = activeRule
      ? JSON.stringify([
          activeRule.id,
          activeRule.updatedAt,
          activeRule.name,
          activeRule.ruleType,
          activeRule.severity,
          activeRule.description ?? "",
          activeRule.guidance,
          activeRule.isEnabled,
        ])
      : null;

    if (previousRuleRevisionRef.current === nextRevision) {
      return;
    }

    setDraftName(activeRule?.name ?? "");
    setDraftType(activeRule?.ruleType ?? "consistency");
    setDraftSeverity(activeRule?.severity ?? "medium");
    setDraftDescription(activeRule?.description ?? "");
    setDraftGuidance(activeRule?.guidance ?? "");
    setDraftIsEnabled(activeRule?.isEnabled ?? true);

    previousRuleRevisionRef.current = nextRevision;
  }, [activeRule]);

  const isCreateDirty = useMemo(
    () =>
      createName.trim().length > 0 ||
      createDescription.trim().length > 0 ||
      createGuidance.trim().length > 0 ||
      createType !== "consistency" ||
      createSeverity !== "medium" ||
      !createIsEnabled,
    [
      createDescription,
      createGuidance,
      createIsEnabled,
      createName,
      createSeverity,
      createType,
    ],
  );

  const isEditDirty = useMemo(() => {
    if (!activeRule) {
      return false;
    }

    return (
      draftName !== activeRule.name ||
      draftType !== activeRule.ruleType ||
      draftSeverity !== activeRule.severity ||
      draftDescription !== (activeRule.description ?? "") ||
      draftGuidance !== activeRule.guidance ||
      draftIsEnabled !== activeRule.isEnabled
    );
  }, [
    activeRule,
    draftDescription,
    draftGuidance,
    draftIsEnabled,
    draftName,
    draftSeverity,
    draftType,
  ]);

  const hasUnsavedChanges = isCreateDirty || isEditDirty;

  useLayoutEffect(() => {
    onDirtyChange?.(hasUnsavedChanges);
  }, [hasUnsavedChanges, onDirtyChange]);

  async function handleCreateSubmit(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();

    if (
      isEditDirty &&
      !window.confirm(
        "You have unsaved rule changes. Create a new rule and discard the current draft?",
      )
    ) {
      return;
    }

    const wasCreated = await submitRule({
      ruleType: createType,
      severity: createSeverity,
      name: createName,
      description: createDescription || undefined,
      guidance: createGuidance,
      isEnabled: createIsEnabled,
    });

    if (wasCreated) {
      setCreateName("");
      setCreateType("consistency");
      setCreateSeverity("medium");
      setCreateDescription("");
      setCreateGuidance("");
      setCreateIsEnabled(true);

      try {
        await onReloadRuleSets();
      } catch {
        // Keep the created rule visible even if the parent rule-set refresh fails.
      }
    }
  }

  async function handleUpdateSubmit(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();

    if (!activeRule) {
      return;
    }

    const wasUpdated = await saveRule({
      ruleId: activeRule.id,
      ruleSetId: activeRule.ruleSetId,
      ruleType: draftType,
      severity: draftSeverity,
      name: draftName,
      description: draftDescription || undefined,
      guidance: draftGuidance,
      isEnabled: draftIsEnabled,
    });

    if (wasUpdated) {
      try {
        await onReloadRuleSets();
      } catch {
        // Keep the updated rule visible even if the parent rule-set refresh fails.
      }
    }
  }

  async function handleOpenRule(ruleId: string) {
    if (ruleId === selectedRuleId) {
      return;
    }

    if (
      hasUnsavedChanges &&
      !window.confirm(
        "You have unsaved rule changes. Open another rule and discard them?",
      )
    ) {
      return;
    }

    selectRule(ruleId);
  }

  async function handleToggleEnabled() {
    if (!activeRule) {
      return;
    }

    const wasUpdated = await saveRule({
      ruleId: activeRule.id,
      ruleSetId: activeRule.ruleSetId,
      ruleType: draftType,
      severity: draftSeverity,
      name: draftName,
      description: draftDescription || undefined,
      guidance: draftGuidance,
      isEnabled: !draftIsEnabled,
    });

    if (wasUpdated) {
      try {
        await onReloadRuleSets();
      } catch {
        // Keep the updated rule visible even if the parent rule-set refresh fails.
      }
    }
  }

  if (!ruleSetId) {
    return (
      <section className="workspace-panel glossary-entry-workspace">
        <p className="surface-card__eyebrow">Rules</p>
        <h3>Open a rule set to manage its rules</h3>
        <p className="surface-card__copy">
          D4 keeps rule sets reusable and independent. Once you open one, this
          panel lets you create and edit individual rules with explicit
          severity, enablement, and editorial guidance.
        </p>
      </section>
    );
  }

  return (
    <section className="workspace-panel glossary-entry-workspace">
      <div className="surface-card__heading">
        <div>
          <p className="surface-card__eyebrow">Rules</p>
          <h3>Guidance inside the selected rule set</h3>
          <p className="surface-card__copy">
            Keep each rule explicit: what kind of constraint it is, how severe
            it should be treated, whether it is enabled, and the guidance the
            editor should follow.
          </p>
        </div>

        <dl className="glossary-metrics glossary-metrics--entries">
          <div>
            <dt>Total</dt>
            <dd>{totalRuleCount}</dd>
          </div>
          <div>
            <dt>Enabled</dt>
            <dd>{enabledRuleCount}</dd>
          </div>
          <div>
            <dt>High</dt>
            <dd>{highSeverityRuleCount}</dd>
          </div>
        </dl>
      </div>

      <div className="glossary-entry-grid">
        <section className="workspace-panel">
          <p className="surface-card__eyebrow">Create rule</p>
          <h3>Add a reusable editorial rule</h3>

          <form className="project-form" onSubmit={handleCreateSubmit}>
            <label className="field-group">
              <span>Rule name</span>
              <input
                autoComplete="off"
                className="field-control"
                disabled={isCreating}
                maxLength={160}
                onChange={(event) => setCreateName(event.target.value)}
                placeholder="Do not soften contraindications"
                required
                value={createName}
              />
            </label>

            <div className="rule-form-grid">
              <label className="field-group">
                <span>Rule type</span>
                <select
                  className="field-control"
                  disabled={isCreating}
                  onChange={(event) =>
                    setCreateType(event.target.value as RuleType)
                  }
                  value={createType}
                >
                  {RULE_TYPE_OPTIONS.map((option) => (
                    <option key={option.value} value={option.value}>
                      {option.label}
                    </option>
                  ))}
                </select>
              </label>

              <label className="field-group">
                <span>Severity</span>
                <select
                  className="field-control"
                  disabled={isCreating}
                  onChange={(event) =>
                    setCreateSeverity(event.target.value as RuleSeverity)
                  }
                  value={createSeverity}
                >
                  {RULE_SEVERITY_OPTIONS.map((option) => (
                    <option key={option.value} value={option.value}>
                      {option.label}
                    </option>
                  ))}
                </select>
              </label>

              <label className="field-group">
                <span>Enabled</span>
                <select
                  className="field-control"
                  disabled={isCreating}
                  onChange={(event) =>
                    setCreateIsEnabled(event.target.value === "enabled")
                  }
                  value={createIsEnabled ? "enabled" : "disabled"}
                >
                  <option value="enabled">Enabled</option>
                  <option value="disabled">Disabled</option>
                </select>
              </label>
            </div>

            <label className="field-group">
              <span>Short description</span>
              <textarea
                className="field-control field-control--textarea"
                disabled={isCreating}
                maxLength={1000}
                onChange={(event) => setCreateDescription(event.target.value)}
                rows={3}
                value={createDescription}
              />
            </label>

            <label className="field-group">
              <span>Editorial guidance</span>
              <textarea
                className="field-control field-control--textarea"
                disabled={isCreating}
                maxLength={2000}
                onChange={(event) => setCreateGuidance(event.target.value)}
                required
                rows={4}
                value={createGuidance}
              />
            </label>

            <div className="project-form__footer">
              <span className="project-form__hint">
                {disabledRuleCount} disabled rules currently stored in this rule
                set.
              </span>

              <button
                className="app-shell__button"
                disabled={isCreating}
                type="submit"
              >
                {isCreating ? "Creating rule..." : "Create rule"}
              </button>
            </div>
          </form>
        </section>

        <section className="workspace-panel">
          <p className="surface-card__eyebrow">Rule list</p>
          <h3>Enabled rules stay visually first</h3>
          <p className="surface-card__copy">
            Severity and activation are shown directly in the list so the rule
            set remains understandable without opening every detail.
          </p>

          {errorMessage(error) ? (
            <p className="form-error">{errorMessage(error)}</p>
          ) : null}

          {isLoading ? (
            <p className="project-form__hint">Loading rules...</p>
          ) : rules.length === 0 ? (
            <div className="glossary-empty-state">
              <p className="project-form__hint">
                This rule set does not have rules yet.
              </p>
            </div>
          ) : (
            <ol className="glossary-list">
              {rules.map((rule) => (
                <li className="glossary-list__item" key={rule.id}>
                  <button
                    className="project-list__item"
                    data-active={rule.id === selectedRuleId}
                    onClick={() => void handleOpenRule(rule.id)}
                    type="button"
                  >
                    <div className="project-list__item-heading">
                      <strong>{rule.name}</strong>
                      <span>{rule.isEnabled ? "Enabled" : "Disabled"}</span>
                    </div>

                    <p>{ruleSummary(rule)}</p>

                    {rule.description ? (
                      <p className="glossary-entry-list__meta">
                        {rule.description}
                      </p>
                    ) : null}
                  </button>
                </li>
              ))}
            </ol>
          )}

          {activeRule ? (
            <section className="glossary-entry-detail">
              <p className="surface-card__eyebrow">Selected rule</p>
              <h3>Edit persisted guidance</h3>

              <form className="project-form" onSubmit={handleUpdateSubmit}>
                <label className="field-group">
                  <span>Rule name</span>
                  <input
                    autoComplete="off"
                    className="field-control"
                    disabled={isSaving}
                    maxLength={160}
                    onChange={(event) => setDraftName(event.target.value)}
                    required
                    value={draftName}
                  />
                </label>

                <div className="rule-form-grid">
                  <label className="field-group">
                    <span>Rule type</span>
                    <select
                      className="field-control"
                      disabled={isSaving}
                      onChange={(event) =>
                        setDraftType(event.target.value as RuleType)
                      }
                      value={draftType}
                    >
                      {RULE_TYPE_OPTIONS.map((option) => (
                        <option key={option.value} value={option.value}>
                          {option.label}
                        </option>
                      ))}
                    </select>
                  </label>

                  <label className="field-group">
                    <span>Severity</span>
                    <select
                      className="field-control"
                      disabled={isSaving}
                      onChange={(event) =>
                        setDraftSeverity(event.target.value as RuleSeverity)
                      }
                      value={draftSeverity}
                    >
                      {RULE_SEVERITY_OPTIONS.map((option) => (
                        <option key={option.value} value={option.value}>
                          {option.label}
                        </option>
                      ))}
                    </select>
                  </label>

                  <label className="field-group">
                    <span>Enabled</span>
                    <select
                      className="field-control"
                      disabled={isSaving}
                      onChange={(event) =>
                        setDraftIsEnabled(event.target.value === "enabled")
                      }
                      value={draftIsEnabled ? "enabled" : "disabled"}
                    >
                      <option value="enabled">Enabled</option>
                      <option value="disabled">Disabled</option>
                    </select>
                  </label>
                </div>

                <label className="field-group">
                  <span>Short description</span>
                  <textarea
                    className="field-control field-control--textarea"
                    disabled={isSaving}
                    maxLength={1000}
                    onChange={(event) =>
                      setDraftDescription(event.target.value)
                    }
                    rows={3}
                    value={draftDescription}
                  />
                </label>

                <label className="field-group">
                  <span>Editorial guidance</span>
                  <textarea
                    className="field-control field-control--textarea"
                    disabled={isSaving}
                    maxLength={2000}
                    onChange={(event) => setDraftGuidance(event.target.value)}
                    required
                    rows={4}
                    value={draftGuidance}
                  />
                </label>

                <dl className="detail-list detail-list--single">
                  <div>
                    <dt>Created</dt>
                    <dd>{formatTimestamp(activeRule.createdAt)}</dd>
                  </div>
                  <div>
                    <dt>Updated</dt>
                    <dd>{formatTimestamp(activeRule.updatedAt)}</dd>
                  </div>
                  <div>
                    <dt>Status</dt>
                    <dd>{draftIsEnabled ? "Enabled" : "Disabled"}</dd>
                  </div>
                </dl>

                <div className="project-form__footer">
                  <span className="project-form__hint">
                    {isEditDirty
                      ? "Unsaved rule changes detected."
                      : "Rule detail is synchronized."}
                  </span>

                  <div className="document-list__actions">
                    <button
                      className="document-action-button"
                      disabled={isSaving}
                      onClick={() => void handleToggleEnabled()}
                      type="button"
                    >
                      {draftIsEnabled ? "Disable rule" : "Enable rule"}
                    </button>

                    <button
                      className="app-shell__button"
                      disabled={isSaving || !isEditDirty}
                      type="submit"
                    >
                      {isSaving ? "Saving rule..." : "Save rule"}
                    </button>
                  </div>
                </div>
              </form>
            </section>
          ) : rules.length > 0 ? (
            <div className="glossary-empty-state glossary-entry-detail">
              <p className="project-form__hint">
                Select a rule to inspect and edit its guidance.
              </p>
            </div>
          ) : null}
        </section>
      </div>
    </section>
  );
}
