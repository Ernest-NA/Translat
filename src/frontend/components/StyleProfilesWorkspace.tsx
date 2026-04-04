import { useEffect, useMemo, useRef, useState } from "react";
import type {
  CreateStyleProfileInput,
  StyleProfileFormality,
  StyleProfileStatus,
  StyleProfileSummary,
  StyleProfileTone,
  StyleProfileTreatmentPreference,
  UpdateStyleProfileInput,
} from "../../shared/desktop";
import type { DesktopCommandError } from "../lib/desktop";

interface StyleProfilesWorkspaceProps {
  activeStyleProfile: StyleProfileSummary | null;
  activeStyleProfileCount: number;
  archivedStyleProfileCount: number;
  error: DesktopCommandError | null;
  isCreating: boolean;
  isLoading: boolean;
  isSaving: boolean;
  onOpenStyleProfile: (styleProfileId: string) => Promise<boolean>;
  onSubmitStyleProfile: (input: CreateStyleProfileInput) => Promise<boolean>;
  onUpdateStyleProfile: (input: UpdateStyleProfileInput) => Promise<boolean>;
  openingStyleProfileId: string | null;
  styleProfiles: StyleProfileSummary[];
  totalStyleProfileCount: number;
}

const TONE_OPTIONS: Array<{ label: string; value: StyleProfileTone }> = [
  { label: "Neutral", value: "neutral" },
  { label: "Direct", value: "direct" },
  { label: "Warm", value: "warm" },
  { label: "Technical", value: "technical" },
];

const FORMALITY_OPTIONS: Array<{
  label: string;
  value: StyleProfileFormality;
}> = [
  { label: "Formal", value: "formal" },
  { label: "Neutral", value: "neutral" },
  { label: "Semi-formal", value: "semi_formal" },
  { label: "Informal", value: "informal" },
];

const TREATMENT_OPTIONS: Array<{
  label: string;
  value: StyleProfileTreatmentPreference;
}> = [
  { label: "Usted", value: "usted" },
  { label: "Tuteo", value: "tuteo" },
  { label: "Impersonal", value: "impersonal" },
  { label: "Mixed", value: "mixed" },
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

function profileSummary(styleProfile: StyleProfileSummary) {
  return [
    optionLabel(TONE_OPTIONS, styleProfile.tone),
    optionLabel(FORMALITY_OPTIONS, styleProfile.formality),
    optionLabel(TREATMENT_OPTIONS, styleProfile.treatmentPreference),
  ].join(" | ");
}

export function StyleProfilesWorkspace({
  activeStyleProfile,
  activeStyleProfileCount,
  archivedStyleProfileCount,
  error,
  isCreating,
  isLoading,
  isSaving,
  onOpenStyleProfile,
  onSubmitStyleProfile,
  onUpdateStyleProfile,
  openingStyleProfileId,
  styleProfiles,
  totalStyleProfileCount,
}: StyleProfilesWorkspaceProps) {
  const [createConsistencyInstructions, setCreateConsistencyInstructions] =
    useState("");
  const [createDescription, setCreateDescription] = useState("");
  const [createEditorialNotes, setCreateEditorialNotes] = useState("");
  const [createFormality, setCreateFormality] =
    useState<StyleProfileFormality>("neutral");
  const [createName, setCreateName] = useState("");
  const [createTone, setCreateTone] = useState<StyleProfileTone>("neutral");
  const [createTreatmentPreference, setCreateTreatmentPreference] =
    useState<StyleProfileTreatmentPreference>("usted");
  const [draftConsistencyInstructions, setDraftConsistencyInstructions] =
    useState("");
  const [draftDescription, setDraftDescription] = useState("");
  const [draftEditorialNotes, setDraftEditorialNotes] = useState("");
  const [draftFormality, setDraftFormality] =
    useState<StyleProfileFormality>("neutral");
  const [draftName, setDraftName] = useState("");
  const [draftStatus, setDraftStatus] = useState<StyleProfileStatus>("active");
  const [draftTone, setDraftTone] = useState<StyleProfileTone>("neutral");
  const [draftTreatmentPreference, setDraftTreatmentPreference] =
    useState<StyleProfileTreatmentPreference>("usted");
  const previousProfileRevisionRef = useRef<string | null>(null);

  useEffect(() => {
    const nextRevision = activeStyleProfile
      ? `${activeStyleProfile.id}:${activeStyleProfile.updatedAt}:${activeStyleProfile.lastOpenedAt}`
      : null;

    if (previousProfileRevisionRef.current === nextRevision) {
      return;
    }

    setDraftName(activeStyleProfile?.name ?? "");
    setDraftDescription(activeStyleProfile?.description ?? "");
    setDraftTone(activeStyleProfile?.tone ?? "neutral");
    setDraftFormality(activeStyleProfile?.formality ?? "neutral");
    setDraftTreatmentPreference(
      activeStyleProfile?.treatmentPreference ?? "usted",
    );
    setDraftConsistencyInstructions(
      activeStyleProfile?.consistencyInstructions ?? "",
    );
    setDraftEditorialNotes(activeStyleProfile?.editorialNotes ?? "");
    setDraftStatus(activeStyleProfile?.status ?? "active");

    previousProfileRevisionRef.current = nextRevision;
  }, [activeStyleProfile]);

  const isDirty = useMemo(() => {
    if (!activeStyleProfile) {
      return false;
    }

    return (
      draftName !== activeStyleProfile.name ||
      draftDescription !== (activeStyleProfile.description ?? "") ||
      draftTone !== activeStyleProfile.tone ||
      draftFormality !== activeStyleProfile.formality ||
      draftTreatmentPreference !== activeStyleProfile.treatmentPreference ||
      draftConsistencyInstructions !==
        (activeStyleProfile.consistencyInstructions ?? "") ||
      draftEditorialNotes !== (activeStyleProfile.editorialNotes ?? "") ||
      draftStatus !== activeStyleProfile.status
    );
  }, [
    activeStyleProfile,
    draftConsistencyInstructions,
    draftDescription,
    draftEditorialNotes,
    draftFormality,
    draftName,
    draftStatus,
    draftTone,
    draftTreatmentPreference,
  ]);

  async function handleCreateSubmit(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();

    const wasCreated = await onSubmitStyleProfile({
      name: createName,
      description: createDescription || undefined,
      tone: createTone,
      formality: createFormality,
      treatmentPreference: createTreatmentPreference,
      consistencyInstructions: createConsistencyInstructions || undefined,
      editorialNotes: createEditorialNotes || undefined,
    });

    if (wasCreated) {
      setCreateName("");
      setCreateDescription("");
      setCreateTone("neutral");
      setCreateFormality("neutral");
      setCreateTreatmentPreference("usted");
      setCreateConsistencyInstructions("");
      setCreateEditorialNotes("");
    }
  }

  async function handleUpdateSubmit(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();

    if (!activeStyleProfile) {
      return;
    }

    await onUpdateStyleProfile({
      styleProfileId: activeStyleProfile.id,
      name: draftName,
      description: draftDescription || undefined,
      tone: draftTone,
      formality: draftFormality,
      treatmentPreference: draftTreatmentPreference,
      consistencyInstructions: draftConsistencyInstructions || undefined,
      editorialNotes: draftEditorialNotes || undefined,
      status: draftStatus,
    });
  }

  async function handleOpenStyleProfile(styleProfileId: string) {
    if (styleProfileId === activeStyleProfile?.id) {
      return;
    }

    if (
      isDirty &&
      !window.confirm(
        "You have unsaved style-profile changes. Open another profile and discard them?",
      )
    ) {
      return;
    }

    await onOpenStyleProfile(styleProfileId);
  }

  async function handleToggleStatus() {
    if (!activeStyleProfile) {
      return;
    }

    await onUpdateStyleProfile({
      styleProfileId: activeStyleProfile.id,
      name: draftName,
      description: draftDescription || undefined,
      tone: draftTone,
      formality: draftFormality,
      treatmentPreference: draftTreatmentPreference,
      consistencyInstructions: draftConsistencyInstructions || undefined,
      editorialNotes: draftEditorialNotes || undefined,
      status: activeStyleProfile.status === "active" ? "archived" : "active",
    });
  }

  return (
    <section className="surface-card surface-card--accent">
      <div className="surface-card__heading">
        <div>
          <p className="surface-card__eyebrow">Style profiles</p>
          <h2>Reusable editorial criteria</h2>
          <p className="surface-card__copy">
            D3 adds persisted style profiles as standalone editorial artifacts.
            They stay reusable, editable, and explicitly separate from AI
            prompts or project defaults.
          </p>
        </div>

        <dl className="glossary-metrics">
          <div>
            <dt>Total</dt>
            <dd>{totalStyleProfileCount}</dd>
          </div>
          <div>
            <dt>Active</dt>
            <dd>{activeStyleProfileCount}</dd>
          </div>
          <div>
            <dt>Archived</dt>
            <dd>{archivedStyleProfileCount}</dd>
          </div>
        </dl>
      </div>

      <div className="glossary-grid">
        <section className="workspace-panel">
          <p className="surface-card__eyebrow">Create profile</p>
          <h3>Persist a reusable editorial baseline</h3>
          <p className="surface-card__copy">
            Keep the structure explicit but compact: tone, formality, treatment
            preference, consistency instructions, and editorial notes.
          </p>

          <form className="project-form" onSubmit={handleCreateSubmit}>
            <label className="field-group">
              <span>Profile name</span>
              <input
                autoComplete="off"
                className="field-control"
                disabled={isCreating}
                maxLength={120}
                onChange={(event) => setCreateName(event.target.value)}
                placeholder="Regulatory Spanish baseline"
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

            <div className="style-profile-form-grid">
              <label className="field-group">
                <span>Tone</span>
                <select
                  className="field-control"
                  disabled={isCreating}
                  onChange={(event) =>
                    setCreateTone(event.target.value as StyleProfileTone)
                  }
                  value={createTone}
                >
                  {TONE_OPTIONS.map((option) => (
                    <option key={option.value} value={option.value}>
                      {option.label}
                    </option>
                  ))}
                </select>
              </label>

              <label className="field-group">
                <span>Formality</span>
                <select
                  className="field-control"
                  disabled={isCreating}
                  onChange={(event) =>
                    setCreateFormality(
                      event.target.value as StyleProfileFormality,
                    )
                  }
                  value={createFormality}
                >
                  {FORMALITY_OPTIONS.map((option) => (
                    <option key={option.value} value={option.value}>
                      {option.label}
                    </option>
                  ))}
                </select>
              </label>

              <label className="field-group">
                <span>Treatment preference</span>
                <select
                  className="field-control"
                  disabled={isCreating}
                  onChange={(event) =>
                    setCreateTreatmentPreference(
                      event.target.value as StyleProfileTreatmentPreference,
                    )
                  }
                  value={createTreatmentPreference}
                >
                  {TREATMENT_OPTIONS.map((option) => (
                    <option key={option.value} value={option.value}>
                      {option.label}
                    </option>
                  ))}
                </select>
              </label>
            </div>

            <label className="field-group">
              <span>Consistency instructions</span>
              <textarea
                className="field-control field-control--textarea"
                disabled={isCreating}
                maxLength={2000}
                onChange={(event) =>
                  setCreateConsistencyInstructions(event.target.value)
                }
                rows={4}
                value={createConsistencyInstructions}
              />
            </label>

            <label className="field-group">
              <span>Editorial notes</span>
              <textarea
                className="field-control field-control--textarea"
                disabled={isCreating}
                maxLength={2000}
                onChange={(event) =>
                  setCreateEditorialNotes(event.target.value)
                }
                rows={4}
                value={createEditorialNotes}
              />
            </label>

            <div className="project-form__footer">
              <button
                className="app-shell__button"
                disabled={isCreating}
                type="submit"
              >
                {isCreating ? "Creating profile..." : "Create profile"}
              </button>

              <span className="project-form__hint">
                Profiles stay reusable and unattached to project defaults in D3.
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
              <p className="surface-card__eyebrow">Profile list</p>
              <h3>Open an existing editorial profile</h3>
            </div>

            <strong className="status-pill">
              {totalStyleProfileCount} total
            </strong>
          </div>

          {isLoading ? (
            <p className="surface-card__copy">
              Loading persisted style profiles...
            </p>
          ) : null}

          {!isLoading && styleProfiles.length === 0 ? (
            <p className="surface-card__copy">
              No style profile exists yet. Create the first reusable editorial
              baseline to leave D3 with persisted style guidance.
            </p>
          ) : null}

          {styleProfiles.length > 0 ? (
            <ul className="glossary-list">
              {styleProfiles.map((styleProfile) => {
                const isActive = styleProfile.id === activeStyleProfile?.id;
                const isOpening = styleProfile.id === openingStyleProfileId;

                return (
                  <li key={styleProfile.id}>
                    <button
                      className="project-list__item glossary-list__item"
                      data-active={isActive}
                      disabled={isOpening}
                      onClick={() =>
                        void handleOpenStyleProfile(styleProfile.id)
                      }
                      type="button"
                    >
                      <div className="project-list__item-heading">
                        <strong>{styleProfile.name}</strong>
                        <span>{isActive ? "Open" : "Manage profile"}</span>
                      </div>

                      <p>
                        {styleProfile.description ??
                          "No description yet. This profile is ready to persist editorial guidance."}
                      </p>

                      <p className="glossary-entry-list__meta">
                        {profileSummary(styleProfile)}
                      </p>

                      <dl className="project-list__meta">
                        <div>
                          <dt>Status</dt>
                          <dd>{styleProfile.status}</dd>
                        </div>
                        <div>
                          <dt>Opened</dt>
                          <dd>{formatTimestamp(styleProfile.lastOpenedAt)}</dd>
                        </div>
                        <div>
                          <dt>Updated</dt>
                          <dd>{formatTimestamp(styleProfile.updatedAt)}</dd>
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
            <p className="surface-card__eyebrow">Profile detail</p>
            <h3>
              {activeStyleProfile
                ? activeStyleProfile.name
                : "No style profile open yet"}
            </h3>
          </div>

          {activeStyleProfile ? (
            <button
              className="document-action-button"
              disabled={isSaving}
              onClick={() => void handleToggleStatus()}
              type="button"
            >
              {activeStyleProfile.status === "active"
                ? "Archive profile"
                : "Restore profile"}
            </button>
          ) : null}
        </div>

        {!activeStyleProfile ? (
          <div className="glossary-empty-state">
            <p className="surface-card__copy">
              Select a style profile from the list to edit its editorial
              parameters and keep it ready for future reuse.
            </p>

            <ul className="readiness-list">
              <li>Profiles remain independent from project defaults in D3.</li>
              <li>Editorial parameters stay explicit and structured.</li>
              <li>No AI prompts or complex validation rules are mixed in.</li>
            </ul>
          </div>
        ) : (
          <form className="project-form" onSubmit={handleUpdateSubmit}>
            <label className="field-group">
              <span>Profile name</span>
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
              <span>Description</span>
              <textarea
                className="field-control field-control--textarea"
                disabled={isSaving}
                maxLength={1000}
                onChange={(event) => setDraftDescription(event.target.value)}
                rows={4}
                value={draftDescription}
              />
            </label>

            <div className="style-profile-form-grid">
              <label className="field-group">
                <span>Tone</span>
                <select
                  className="field-control"
                  disabled={isSaving}
                  onChange={(event) =>
                    setDraftTone(event.target.value as StyleProfileTone)
                  }
                  value={draftTone}
                >
                  {TONE_OPTIONS.map((option) => (
                    <option key={option.value} value={option.value}>
                      {option.label}
                    </option>
                  ))}
                </select>
              </label>

              <label className="field-group">
                <span>Formality</span>
                <select
                  className="field-control"
                  disabled={isSaving}
                  onChange={(event) =>
                    setDraftFormality(
                      event.target.value as StyleProfileFormality,
                    )
                  }
                  value={draftFormality}
                >
                  {FORMALITY_OPTIONS.map((option) => (
                    <option key={option.value} value={option.value}>
                      {option.label}
                    </option>
                  ))}
                </select>
              </label>

              <label className="field-group">
                <span>Treatment preference</span>
                <select
                  className="field-control"
                  disabled={isSaving}
                  onChange={(event) =>
                    setDraftTreatmentPreference(
                      event.target.value as StyleProfileTreatmentPreference,
                    )
                  }
                  value={draftTreatmentPreference}
                >
                  {TREATMENT_OPTIONS.map((option) => (
                    <option key={option.value} value={option.value}>
                      {option.label}
                    </option>
                  ))}
                </select>
              </label>
            </div>

            <label className="field-group">
              <span>Status</span>
              <select
                className="field-control"
                disabled={isSaving}
                onChange={(event) =>
                  setDraftStatus(event.target.value as StyleProfileStatus)
                }
                value={draftStatus}
              >
                <option value="active">Active</option>
                <option value="archived">Archived</option>
              </select>
            </label>

            <label className="field-group">
              <span>Consistency instructions</span>
              <textarea
                className="field-control field-control--textarea"
                disabled={isSaving}
                maxLength={2000}
                onChange={(event) =>
                  setDraftConsistencyInstructions(event.target.value)
                }
                rows={4}
                value={draftConsistencyInstructions}
              />
            </label>

            <label className="field-group">
              <span>Editorial notes</span>
              <textarea
                className="field-control field-control--textarea"
                disabled={isSaving}
                maxLength={2000}
                onChange={(event) => setDraftEditorialNotes(event.target.value)}
                rows={4}
                value={draftEditorialNotes}
              />
            </label>

            <dl className="detail-list">
              <div>
                <dt>Profile ID</dt>
                <dd>{activeStyleProfile.id}</dd>
              </div>
              <div>
                <dt>Created</dt>
                <dd>{formatTimestamp(activeStyleProfile.createdAt)}</dd>
              </div>
              <div>
                <dt>Opened</dt>
                <dd>{formatTimestamp(activeStyleProfile.lastOpenedAt)}</dd>
              </div>
              <div>
                <dt>Updated</dt>
                <dd>{formatTimestamp(activeStyleProfile.updatedAt)}</dd>
              </div>
            </dl>

            <div className="project-form__footer">
              <button
                className="app-shell__button"
                disabled={isSaving}
                type="submit"
              >
                {isSaving ? "Saving profile..." : "Save profile"}
              </button>

              <span className="project-form__hint">
                {isDirty
                  ? "You have unsaved changes in this style profile."
                  : "This profile stays reusable for later project association."}
              </span>
            </div>

            {error ? (
              <p className="form-error" role="alert">
                {error.message}
              </p>
            ) : null}
          </form>
        )}
      </section>
    </section>
  );
}
