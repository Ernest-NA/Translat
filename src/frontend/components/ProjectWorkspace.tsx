import { useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import type {
  DocumentSectionSummary,
  DocumentSummary,
  GlossarySummary,
  ProjectSummary,
  RuleSetSummary,
  SegmentSummary,
  StyleProfileSummary,
  UpdateProjectEditorialDefaultsInput,
} from "../../shared/desktop";
import type { DesktopCommandError } from "../lib/desktop";
import { DocumentImporter } from "./DocumentImporter";
import { DocumentList } from "./DocumentList";
import { SegmentBrowser } from "./SegmentBrowser";

interface ProjectWorkspaceProps {
  activeDocument: DocumentSummary | null;
  documents: DocumentSummary[];
  glossaries: GlossarySummary[];
  importError: DesktopCommandError | null;
  isImportingDocuments: boolean;
  isLoadingDocuments: boolean;
  isLoadingSegments: boolean;
  isSavingEditorialDefaults: boolean;
  loadError: DesktopCommandError | null;
  onDirtyChange: (isDirty: boolean) => void;
  onOpenDocument: (documentId: string) => Promise<void>;
  onImportDocuments: (files: FileList) => Promise<number>;
  onProcessDocument: (documentId: string) => Promise<void>;
  onSaveEditorialDefaults: (
    input: UpdateProjectEditorialDefaultsInput,
  ) => Promise<boolean>;
  onSelectSection: (sectionId: string) => void;
  onSelectSegment: (segmentId: string) => void;
  processError: DesktopCommandError | null;
  processingDocumentId: string | null;
  project: ProjectSummary | null;
  projectError: DesktopCommandError | null;
  ruleSets: RuleSetSummary[];
  segmentError: DesktopCommandError | null;
  segmentLoadingDocumentId: string | null;
  sections: DocumentSectionSummary[];
  selectedSection: DocumentSectionSummary | null;
  segments: SegmentSummary[];
  selectedSegment: SegmentSummary | null;
  selectedSegmentId: string | null;
  styleProfiles: StyleProfileSummary[];
}

function formatTimestamp(timestamp: number) {
  return new Date(timestamp * 1000).toLocaleString();
}

function normalizeSelectionValue(value?: string | null) {
  return value ?? "";
}

function toOptionalSelection(value: string) {
  return value.length > 0 ? value : undefined;
}

function formatStatusSuffix(status: "active" | "archived") {
  return status === "archived" ? " (archived)" : "";
}

export function ProjectWorkspace({
  activeDocument,
  documents,
  glossaries,
  importError,
  isImportingDocuments,
  isLoadingDocuments,
  isLoadingSegments,
  isSavingEditorialDefaults,
  loadError,
  onDirtyChange,
  onOpenDocument,
  onImportDocuments,
  onProcessDocument,
  onSaveEditorialDefaults,
  onSelectSection,
  onSelectSegment,
  processError,
  processingDocumentId,
  project,
  projectError,
  ruleSets,
  segmentError,
  segmentLoadingDocumentId,
  sections,
  selectedSection,
  segments,
  selectedSegment,
  selectedSegmentId,
  styleProfiles,
}: ProjectWorkspaceProps) {
  const [draftDefaultGlossaryId, setDraftDefaultGlossaryId] = useState(() =>
    normalizeSelectionValue(project?.defaultGlossaryId),
  );
  const [draftDefaultStyleProfileId, setDraftDefaultStyleProfileId] = useState(
    () => normalizeSelectionValue(project?.defaultStyleProfileId),
  );
  const [draftDefaultRuleSetId, setDraftDefaultRuleSetId] = useState(() =>
    normalizeSelectionValue(project?.defaultRuleSetId),
  );
  const previousProjectIdRef = useRef<string | null>(null);
  const pendingDefaultsSyncRef =
    useRef<UpdateProjectEditorialDefaultsInput | null>(null);

  useEffect(() => {
    const nextProjectId = project?.id ?? null;
    const hasProjectChanged = previousProjectIdRef.current !== nextProjectId;
    const pendingDefaultsSync = pendingDefaultsSyncRef.current;
    const matchesPendingDefaultsSync =
      !!project &&
      pendingDefaultsSync?.projectId === project.id &&
      normalizeSelectionValue(pendingDefaultsSync.defaultGlossaryId) ===
        normalizeSelectionValue(project.defaultGlossaryId) &&
      normalizeSelectionValue(pendingDefaultsSync.defaultStyleProfileId) ===
        normalizeSelectionValue(project.defaultStyleProfileId) &&
      normalizeSelectionValue(pendingDefaultsSync.defaultRuleSetId) ===
        normalizeSelectionValue(project.defaultRuleSetId);

    if (!hasProjectChanged && !matchesPendingDefaultsSync) {
      return;
    }

    setDraftDefaultGlossaryId(
      normalizeSelectionValue(project?.defaultGlossaryId),
    );
    setDraftDefaultStyleProfileId(
      normalizeSelectionValue(project?.defaultStyleProfileId),
    );
    setDraftDefaultRuleSetId(
      normalizeSelectionValue(project?.defaultRuleSetId),
    );

    if (matchesPendingDefaultsSync) {
      pendingDefaultsSyncRef.current = null;
    }

    previousProjectIdRef.current = nextProjectId;
  }, [project]);

  const isDirty = useMemo(() => {
    if (!project) {
      return false;
    }

    return (
      draftDefaultGlossaryId !==
        normalizeSelectionValue(project.defaultGlossaryId) ||
      draftDefaultStyleProfileId !==
        normalizeSelectionValue(project.defaultStyleProfileId) ||
      draftDefaultRuleSetId !==
        normalizeSelectionValue(project.defaultRuleSetId)
    );
  }, [
    draftDefaultGlossaryId,
    draftDefaultRuleSetId,
    draftDefaultStyleProfileId,
    project,
  ]);

  useLayoutEffect(() => {
    onDirtyChange(isDirty);
  }, [isDirty, onDirtyChange]);

  const defaultGlossary = useMemo(
    () =>
      glossaries.find(
        (glossary) => glossary.id === project?.defaultGlossaryId,
      ) ?? null,
    [glossaries, project?.defaultGlossaryId],
  );
  const defaultStyleProfile = useMemo(
    () =>
      styleProfiles.find(
        (styleProfile) => styleProfile.id === project?.defaultStyleProfileId,
      ) ?? null,
    [project?.defaultStyleProfileId, styleProfiles],
  );
  const defaultRuleSet = useMemo(
    () =>
      ruleSets.find((ruleSet) => ruleSet.id === project?.defaultRuleSetId) ??
      null,
    [project?.defaultRuleSetId, ruleSets],
  );

  async function handleSaveEditorialDefaults(
    event: React.FormEvent<HTMLFormElement>,
  ) {
    event.preventDefault();

    if (!project) {
      return;
    }

    const changes: UpdateProjectEditorialDefaultsInput = {
      projectId: project.id,
      defaultGlossaryId: toOptionalSelection(draftDefaultGlossaryId),
      defaultStyleProfileId: toOptionalSelection(draftDefaultStyleProfileId),
      defaultRuleSetId: toOptionalSelection(draftDefaultRuleSetId),
    };

    pendingDefaultsSyncRef.current = changes;

    const wasSaved = await onSaveEditorialDefaults(changes);

    if (!wasSaved) {
      pendingDefaultsSyncRef.current = null;
    }
  }

  if (!project) {
    return (
      <section className="surface-card surface-card--accent">
        <p className="surface-card__eyebrow">Workspace</p>
        <h2>No project open yet.</h2>
        <p className="surface-card__copy">
          Select a persisted project or create a new one. Document intake and
          editorial defaults only become active after a workspace has been
          explicitly selected.
        </p>

        <ul className="readiness-list">
          <li>Projects are persisted in the encrypted SQLite database.</li>
          <li>The active project selection survives app restarts.</li>
          <li>Each project can keep explicit default editorial artifacts.</li>
        </ul>
      </section>
    );
  }

  return (
    <section className="surface-card surface-card--accent">
      <p className="surface-card__eyebrow">Open workspace</p>
      <h2>{project.name}</h2>
      <p className="surface-card__copy">
        {project.description ??
          "This project has no description yet. It is ready to receive imported documents and keep a reusable editorial baseline visible from the same workspace."}
      </p>

      <dl className="detail-list">
        <div>
          <dt>Project ID</dt>
          <dd>{project.id}</dd>
        </div>
        <div>
          <dt>Created</dt>
          <dd>{formatTimestamp(project.createdAt)}</dd>
        </div>
        <div>
          <dt>Last opened</dt>
          <dd>{formatTimestamp(project.lastOpenedAt)}</dd>
        </div>
        <div>
          <dt>Updated</dt>
          <dd>{formatTimestamp(project.updatedAt)}</dd>
        </div>
      </dl>

      <section className="workspace-panel project-editorial-defaults">
        <p className="surface-card__eyebrow">Editorial defaults</p>
        <h3>Associate one glossary, style profile, and rule set by default</h3>
        <p className="surface-card__copy">
          D5 keeps the project baseline explicit and persisted. Each project can
          point to zero or one default glossary, style profile, and rule set
          without adding precedence logic or automatic AI usage.
        </p>

        {projectError ? (
          <p className="form-error">{projectError.message}</p>
        ) : null}

        <dl className="detail-list detail-list--single">
          <div>
            <dt>Default glossary</dt>
            <dd>
              {defaultGlossary
                ? `${defaultGlossary.name}${formatStatusSuffix(defaultGlossary.status)}`
                : "None"}
            </dd>
          </div>
          <div>
            <dt>Default style profile</dt>
            <dd>
              {defaultStyleProfile
                ? `${defaultStyleProfile.name}${formatStatusSuffix(defaultStyleProfile.status)}`
                : "None"}
            </dd>
          </div>
          <div>
            <dt>Default rule set</dt>
            <dd>
              {defaultRuleSet
                ? `${defaultRuleSet.name}${formatStatusSuffix(defaultRuleSet.status)}`
                : "None"}
            </dd>
          </div>
        </dl>

        <form className="project-form" onSubmit={handleSaveEditorialDefaults}>
          <div className="project-defaults-grid">
            <label className="field-group">
              <span>Default glossary</span>
              <select
                className="field-control"
                disabled={isSavingEditorialDefaults}
                onChange={(event) =>
                  setDraftDefaultGlossaryId(event.target.value)
                }
                value={draftDefaultGlossaryId}
              >
                <option value="">No default glossary</option>
                {glossaries.map((glossary) => (
                  <option key={glossary.id} value={glossary.id}>
                    {glossary.name}
                    {formatStatusSuffix(glossary.status)}
                  </option>
                ))}
              </select>
            </label>

            <label className="field-group">
              <span>Default style profile</span>
              <select
                className="field-control"
                disabled={isSavingEditorialDefaults}
                onChange={(event) =>
                  setDraftDefaultStyleProfileId(event.target.value)
                }
                value={draftDefaultStyleProfileId}
              >
                <option value="">No default style profile</option>
                {styleProfiles.map((styleProfile) => (
                  <option key={styleProfile.id} value={styleProfile.id}>
                    {styleProfile.name}
                    {formatStatusSuffix(styleProfile.status)}
                  </option>
                ))}
              </select>
            </label>

            <label className="field-group">
              <span>Default rule set</span>
              <select
                className="field-control"
                disabled={isSavingEditorialDefaults}
                onChange={(event) =>
                  setDraftDefaultRuleSetId(event.target.value)
                }
                value={draftDefaultRuleSetId}
              >
                <option value="">No default rule set</option>
                {ruleSets.map((ruleSet) => (
                  <option key={ruleSet.id} value={ruleSet.id}>
                    {ruleSet.name}
                    {formatStatusSuffix(ruleSet.status)}
                  </option>
                ))}
              </select>
            </label>
          </div>

          <div className="project-form__footer">
            <span className="project-form__hint">
              {isDirty
                ? "Unsaved project editorial defaults detected."
                : "Project editorial defaults are synchronized."}
            </span>

            <button
              className="app-shell__button"
              disabled={isSavingEditorialDefaults || !isDirty}
              type="submit"
            >
              {isSavingEditorialDefaults
                ? "Saving editorial defaults..."
                : "Save editorial defaults"}
            </button>
          </div>
        </form>
      </section>

      <div className="workspace-document-grid">
        <DocumentImporter
          error={importError}
          isImporting={isImportingDocuments}
          onImport={onImportDocuments}
          project={project}
        />

        <DocumentList
          activeDocumentId={activeDocument?.id ?? null}
          documents={documents}
          error={loadError}
          isLoading={isLoadingDocuments}
          onOpenDocument={onOpenDocument}
          onProcessDocument={onProcessDocument}
          processError={processError}
          processingDocumentId={processingDocumentId}
          segmentLoadingDocumentId={segmentLoadingDocumentId}
        />
      </div>

      <SegmentBrowser
        activeDocument={activeDocument}
        error={segmentError}
        isLoading={isLoadingSegments}
        onSelectSection={onSelectSection}
        onSelectSegment={onSelectSegment}
        project={project}
        sections={sections}
        selectedSection={selectedSection}
        segments={segments}
        selectedSegment={selectedSegment}
        selectedSegmentId={selectedSegmentId}
      />

      <section className="workspace-readiness">
        <p className="surface-card__eyebrow">Project foundation</p>
        <h3>
          Document workflow stays available with explicit editorial defaults
        </h3>
        <ul className="readiness-list">
          <li>Imported documents are linked explicitly to this project id.</li>
          <li>
            The project can persist one default glossary, style profile, and
            rule set at the same time.
          </li>
          <li>
            Defaults remain editable and optional, with no automatic prompt or
            AI integration in this phase.
          </li>
          <li>
            Segment processing persists ordered source segments per document.
          </li>
        </ul>
      </section>
    </section>
  );
}
