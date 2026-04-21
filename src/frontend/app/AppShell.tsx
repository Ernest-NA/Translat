import { useCallback, useRef, useState } from "react";
import { DESKTOP_COMMANDS } from "../../shared/desktop";
import { GlossaryWorkspace } from "../components/GlossaryWorkspace";
import { HealthcheckPanel } from "../components/HealthcheckPanel";
import { ProjectComposer } from "../components/ProjectComposer";
import { ProjectList } from "../components/ProjectList";
import { ProjectWorkspace } from "../components/ProjectWorkspace";
import { RuleSetsWorkspace } from "../components/RuleSetsWorkspace";
import { StyleProfilesWorkspace } from "../components/StyleProfilesWorkspace";
import { PanelHeader } from "../components/ui/PanelHeader";
import { StatusBadge } from "../components/ui/StatusBadge";
import { useDocumentChunks } from "../hooks/useDocumentChunks";
import { useDocumentSegments } from "../hooks/useDocumentSegments";
import { useGlossariesWorkspace } from "../hooks/useGlossariesWorkspace";
import { useHealthcheck } from "../hooks/useHealthcheck";
import { useProjectDocuments } from "../hooks/useProjectDocuments";
import { useProjectsWorkspace } from "../hooks/useProjectsWorkspace";
import { useRuleSetsWorkspace } from "../hooks/useRuleSetsWorkspace";
import { useStyleProfilesWorkspace } from "../hooks/useStyleProfilesWorkspace";

type BadgeTone = "neutral" | "info" | "success" | "warning" | "danger";
type ShellView =
  | "projects"
  | "documents"
  | "translation"
  | "libraries"
  | "diagnostics";

interface ShellViewCopy {
  description: string;
  eyebrow: string;
  title: string;
}

interface NavigationItem {
  count: number;
  description: string;
  id: ShellView;
  label: string;
}

const SHELL_VIEW_COPY: Record<ShellView, ShellViewCopy> = {
  projects: {
    description: "Open, create, and orient the active translation workspace.",
    eyebrow: "Start",
    title: "Projects",
  },
  documents: {
    description:
      "Import source files and confirm document structure before chunk work.",
    eyebrow: "Document flow",
    title: "Documents",
  },
  translation: {
    description:
      "Translate chunk-based work, review QA findings, and monitor jobs.",
    eyebrow: "Operations",
    title: "Translation",
  },
  libraries: {
    description:
      "Manage rules, style profiles, glossaries, and project defaults.",
    eyebrow: "Editorial control",
    title: "Editorial libraries",
  },
  diagnostics: {
    description: "Check desktop connectivity and low-level command contracts.",
    eyebrow: "Runtime",
    title: "Diagnostics",
  },
};

function formatCheckedAt(value?: number) {
  if (!value) {
    return "Pending first response";
  }

  return new Date(value).toLocaleString();
}

function formatCount(value: number) {
  return new Intl.NumberFormat(undefined, { maximumFractionDigits: 0 }).format(
    value,
  );
}

export function AppShell() {
  const [activeView, setActiveView] = useState<ShellView>("projects");
  const {
    activeProject,
    reload: reloadProjects,
    error: projectError,
    isCreating,
    isLoading: isLoadingProjects,
    isSavingEditorialDefaults,
    openingProjectId,
    projects,
    saveProjectEditorialDefaults,
    selectProject,
    submitProject,
  } = useProjectsWorkspace();
  const {
    activeGlossary,
    activeGlossaryCount,
    archivedGlossaryCount,
    error: glossaryError,
    glossaries,
    isCreating: isCreatingGlossary,
    isLoading: isLoadingGlossaries,
    isSaving: isSavingGlossary,
    openingGlossaryId,
    reload: reloadGlossaries,
    saveGlossary,
    selectGlossary,
    submitGlossary,
    totalGlossaryCount,
  } = useGlossariesWorkspace();
  const {
    activeRuleSet,
    activeRuleSetCount,
    archivedRuleSetCount,
    error: ruleSetError,
    isCreating: isCreatingRuleSet,
    isLoading: isLoadingRuleSets,
    isSaving: isSavingRuleSet,
    openingRuleSetId,
    reload: reloadRuleSets,
    ruleSets,
    saveRuleSet,
    selectRuleSet,
    submitRuleSet,
    totalRuleSetCount,
  } = useRuleSetsWorkspace();
  const {
    activeStyleProfile,
    activeStyleProfileCount,
    archivedStyleProfileCount,
    error: styleProfileError,
    isCreating: isCreatingStyleProfile,
    isLoading: isLoadingStyleProfiles,
    isSaving: isSavingStyleProfile,
    openingStyleProfileId,
    saveStyleProfile,
    selectStyleProfile,
    styleProfiles,
    submitStyleProfile,
    totalStyleProfileCount,
  } = useStyleProfilesWorkspace();
  const {
    documents,
    importError,
    importDocuments,
    isImporting,
    isLoading: isLoadingDocuments,
    loadError,
    processDocument,
    processError,
    processingDocumentId,
  } = useProjectDocuments(activeProject?.id ?? null);
  const {
    activeDocument,
    error: segmentError,
    isLoading: isLoadingSegments,
    openDocument,
    refreshDocument,
    sections,
    segments,
    selectedSection,
    selectSection,
    selectedSegment,
    selectedSegmentId,
    selectSegment,
  } = useDocumentSegments(activeProject?.id ?? null, documents);
  const {
    buildChunks,
    chunkSegments,
    chunks,
    error: chunkError,
    isBuilding: isBuildingChunks,
    isLoading: isLoadingChunks,
    selectedChunk,
    selectedChunkId,
    selectedChunkSegments,
    selectChunk,
  } = useDocumentChunks(activeProject?.id ?? null, activeDocument);
  const {
    error: healthcheckError,
    healthcheck,
    isLoading: isLoadingHealthcheck,
    retry,
  } = useHealthcheck();
  const activeProjectIdRef = useRef<string | null>(activeProject?.id ?? null);
  const activeViewRef = useRef<ShellView>(activeView);
  const [hasUnsavedProjectDefaults, setHasUnsavedProjectDefaults] =
    useState(false);
  activeProjectIdRef.current = activeProject?.id ?? null;
  activeViewRef.current = activeView;

  const activeProjectDefaultGlossary =
    glossaries.find(
      (glossary) => glossary.id === activeProject?.defaultGlossaryId,
    ) ?? null;
  const activeProjectDefaultStyleProfile =
    styleProfiles.find(
      (styleProfile) =>
        styleProfile.id === activeProject?.defaultStyleProfileId,
    ) ?? null;
  const activeProjectDefaultRuleSet =
    ruleSets.find(
      (ruleSet) => ruleSet.id === activeProject?.defaultRuleSetId,
    ) ?? null;

  const confirmDiscardProjectDefaults = useCallback(
    (action: "create" | "switch") => {
      if (!hasUnsavedProjectDefaults) {
        return true;
      }

      if (action === "create") {
        return window.confirm(
          "You have unsaved project editorial defaults. Create a new project and discard them?",
        );
      }

      return window.confirm(
        "You have unsaved project editorial defaults. Open another project and discard them?",
      );
    },
    [hasUnsavedProjectDefaults],
  );

  const handleSubmitProject = useCallback(
    async (input: { description?: string; name: string }) => {
      if (!confirmDiscardProjectDefaults("create")) {
        return false;
      }

      const created = await submitProject(input);

      if (created) {
        setActiveView("documents");
      }

      return created;
    },
    [confirmDiscardProjectDefaults, submitProject],
  );

  const handleSelectProject = useCallback(
    async (projectId: string) => {
      if (projectId === activeProject?.id) {
        setActiveView("documents");
        return true;
      }

      if (!confirmDiscardProjectDefaults("switch")) {
        return false;
      }

      const selected = await selectProject(projectId);

      if (selected) {
        setActiveView("documents");
      }

      return selected;
    },
    [activeProject?.id, confirmDiscardProjectDefaults, selectProject],
  );

  const handleImportDocuments = useCallback(
    async (files: FileList): Promise<number> => {
      const initiatingView = activeViewRef.current;
      const importedCount = await importDocuments(files);

      if (importedCount > 0) {
        if (
          activeViewRef.current === initiatingView &&
          (initiatingView === "documents" || initiatingView === "translation")
        ) {
          setActiveView("documents");
        }

        try {
          await reloadProjects();
        } catch {
          // Keep the import result successful even if the sidebar refresh fails.
        }
      }

      return importedCount;
    },
    [importDocuments, reloadProjects],
  );

  const handleProcessDocument = useCallback(
    async (documentId: string): Promise<void> => {
      const initiatingView = activeViewRef.current;
      const processedDocument = await processDocument(documentId);

      if (processedDocument) {
        if (activeProjectIdRef.current !== processedDocument.projectId) {
          return;
        }

        await openDocument(processedDocument.id);

        if (activeProjectIdRef.current !== processedDocument.projectId) {
          return;
        }

        if (
          activeViewRef.current === initiatingView &&
          (initiatingView === "documents" || initiatingView === "translation")
        ) {
          setActiveView("translation");
        }

        try {
          await reloadProjects();
        } catch {
          // Keep the segmentation result visible even if the sidebar refresh fails.
        }
      }
    },
    [openDocument, processDocument, reloadProjects],
  );

  const viewCopy = SHELL_VIEW_COPY[activeView];
  const documentStateLabel = activeDocument
    ? chunks.length > 0
      ? "Chunk ready"
      : activeDocument.status === "segmented"
        ? "Segmented"
        : activeDocument.status
    : "No document";
  const documentStateTone: BadgeTone = activeDocument
    ? chunks.length > 0
      ? "success"
      : activeDocument.status === "segmented"
        ? "warning"
        : "info"
    : "neutral";
  const activeDefaultsCount = [
    activeProjectDefaultGlossary,
    activeProjectDefaultStyleProfile,
    activeProjectDefaultRuleSet,
  ].filter(Boolean).length;
  const navigationItems: NavigationItem[] = [
    {
      count: projects.length,
      description: "Create and select workspaces",
      id: "projects",
      label: "Projects",
    },
    {
      count: documents.length,
      description: "Import and segment sources",
      id: "documents",
      label: "Documents",
    },
    {
      count: chunks.length,
      description: "Chunks, jobs, and review",
      id: "translation",
      label: "Translation",
    },
    {
      count: totalGlossaryCount + totalRuleSetCount + totalStyleProfileCount,
      description: "Rules, style, glossary",
      id: "libraries",
      label: "Editorial",
    },
    {
      count: healthcheckError ? 1 : 0,
      description: "Runtime and command health",
      id: "diagnostics",
      label: "Diagnostics",
    },
  ];

  return (
    <main className="app-shell">
      <aside className="app-shell__nav" aria-label="Primary navigation">
        <div className="app-shell__brand">
          <span className="app-shell__brand-mark" aria-hidden="true">
            T
          </span>
          <div>
            <p>Translat</p>
            <strong>Translation workstation</strong>
          </div>
        </div>

        <nav className="app-shell__nav-list">
          {navigationItems.map((item) => (
            <button
              className="app-shell__nav-button"
              data-active={activeView === item.id}
              key={item.id}
              onClick={() => setActiveView(item.id)}
              type="button"
            >
              <span>
                <strong>{item.label}</strong>
                <small>{item.description}</small>
              </span>
              <span className="app-shell__nav-count">
                {formatCount(item.count)}
              </span>
            </button>
          ))}
        </nav>
      </aside>

      <section className="app-shell__main">
        <section className="app-shell__content">
          <div className="app-shell__view-header">
            <PanelHeader
              description={viewCopy.description}
              eyebrow={viewCopy.eyebrow}
              title={viewCopy.title}
              titleLevel={2}
            />
          </div>

          <div
            className="app-shell__view app-shell__view--projects"
            hidden={activeView !== "projects"}
          >
            <section className="app-shell__split">
              <ProjectComposer
                error={projectError}
                isCreating={isCreating}
                onSubmit={handleSubmitProject}
              />

              <ProjectList
                activeProjectId={activeProject?.id ?? null}
                isLoading={isLoadingProjects}
                onOpen={handleSelectProject}
                openingProjectId={openingProjectId}
                projects={projects}
              />
            </section>

            <section className="surface-card app-shell__overview-card">
              <PanelHeader
                description={
                  activeProject
                    ? (activeProject.description ??
                      "Ready for document import and editorial setup.")
                    : "Select or create a project to unlock document and translation work."
                }
                eyebrow="Active workspace"
                meta={
                  <>
                    <StatusBadge
                      tone={activeProject ? "success" : "neutral"}
                      size="sm"
                    >
                      {activeProject ? "Project open" : "Waiting"}
                    </StatusBadge>
                    <StatusBadge tone={documentStateTone} size="sm">
                      {documentStateLabel}
                    </StatusBadge>
                  </>
                }
                title={activeProject?.name ?? "No project selected"}
                titleLevel={2}
              />

              <div className="app-shell__metric-grid">
                <div>
                  <span>Documents</span>
                  <strong>{formatCount(documents.length)}</strong>
                </div>
                <div>
                  <span>Segments</span>
                  <strong>{formatCount(segments.length)}</strong>
                </div>
                <div>
                  <span>Chunks</span>
                  <strong>{formatCount(chunks.length)}</strong>
                </div>
                <div>
                  <span>Defaults</span>
                  <strong>{activeDefaultsCount}/3</strong>
                </div>
              </div>
            </section>
          </div>

          <div
            className="app-shell__view"
            hidden={
              activeView !== "documents" &&
              activeView !== "translation" &&
              activeView !== "diagnostics"
            }
          >
            <ProjectWorkspace
              activeDocument={activeDocument}
              chunkError={chunkError}
              chunkSegments={chunkSegments}
              chunks={chunks}
              documents={documents}
              importError={importError}
              isBuildingChunks={isBuildingChunks}
              isImportingDocuments={isImporting}
              isLoadingDocuments={isLoadingDocuments}
              isLoadingChunks={isLoadingChunks}
              isLoadingSegments={isLoadingSegments}
              isSavingEditorialDefaults={isSavingEditorialDefaults}
              loadError={loadError}
              glossaries={glossaries}
              onBuildChunks={buildChunks}
              onDirtyChange={setHasUnsavedProjectDefaults}
              onOpenDocument={openDocument}
              onSyncDocumentState={refreshDocument}
              onImportDocuments={handleImportDocuments}
              onProcessDocument={handleProcessDocument}
              onSelectChunk={selectChunk}
              onSaveEditorialDefaults={saveProjectEditorialDefaults}
              onSelectSection={selectSection}
              onSelectSegment={selectSegment}
              processError={processError}
              processingDocumentId={processingDocumentId}
              project={activeProject}
              projectError={projectError}
              ruleSets={ruleSets}
              segmentError={segmentError}
              segmentLoadingDocumentId={
                isLoadingSegments ? (activeDocument?.id ?? null) : null
              }
              sections={sections}
              selectedChunk={selectedChunk}
              selectedChunkId={selectedChunkId}
              selectedChunkSegments={selectedChunkSegments}
              selectedSection={selectedSection}
              segments={segments}
              selectedSegment={selectedSegment}
              selectedSegmentId={selectedSegmentId}
              showOperationalDebug={activeView === "diagnostics"}
              styleProfiles={styleProfiles}
              viewMode={
                activeView === "diagnostics"
                  ? "operational-debug"
                  : activeView === "translation"
                    ? "translation-workspace"
                    : "document-workspace"
              }
            />
          </div>

          <div
            className="app-shell__view app-shell__view--libraries"
            hidden={activeView !== "libraries"}
          >
            <RuleSetsWorkspace
              activeRuleSet={activeRuleSet}
              activeRuleSetCount={activeRuleSetCount}
              archivedRuleSetCount={archivedRuleSetCount}
              error={ruleSetError}
              isCreating={isCreatingRuleSet}
              isLoading={isLoadingRuleSets}
              isSaving={isSavingRuleSet}
              onOpenRuleSet={selectRuleSet}
              onReloadRuleSets={reloadRuleSets}
              onSubmitRuleSet={submitRuleSet}
              onUpdateRuleSet={saveRuleSet}
              openingRuleSetId={openingRuleSetId}
              ruleSets={ruleSets}
              totalRuleSetCount={totalRuleSetCount}
            />

            <StyleProfilesWorkspace
              activeStyleProfile={activeStyleProfile}
              activeStyleProfileCount={activeStyleProfileCount}
              archivedStyleProfileCount={archivedStyleProfileCount}
              error={styleProfileError}
              isCreating={isCreatingStyleProfile}
              isLoading={isLoadingStyleProfiles}
              isSaving={isSavingStyleProfile}
              onOpenStyleProfile={selectStyleProfile}
              onSubmitStyleProfile={submitStyleProfile}
              onUpdateStyleProfile={saveStyleProfile}
              openingStyleProfileId={openingStyleProfileId}
              styleProfiles={styleProfiles}
              totalStyleProfileCount={totalStyleProfileCount}
            />

            <GlossaryWorkspace
              activeGlossary={activeGlossary}
              activeGlossaryCount={activeGlossaryCount}
              archivedGlossaryCount={archivedGlossaryCount}
              error={glossaryError}
              glossaries={glossaries}
              isCreating={isCreatingGlossary}
              isLoading={isLoadingGlossaries}
              isSaving={isSavingGlossary}
              onOpenGlossary={selectGlossary}
              onSubmitGlossary={submitGlossary}
              onUpdateGlossary={saveGlossary}
              openingGlossaryId={openingGlossaryId}
              onReloadGlossaries={reloadGlossaries}
              projects={projects}
              totalGlossaryCount={totalGlossaryCount}
            />
          </div>

          {activeView === "diagnostics" ? (
            <div className="app-shell__view app-shell__view--diagnostics">
              <section className="surface-card">
                <PanelHeader
                  description="Desktop bridge command used for project-level editorial defaults."
                  eyebrow="Command contract"
                  meta={
                    <StatusBadge size="sm" tone="info">
                      invokeDesktopCommand
                    </StatusBadge>
                  }
                  title={DESKTOP_COMMANDS.updateProjectEditorialDefaults}
                  titleLevel={2}
                />

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
                    <dt>Open project</dt>
                    <dd>{activeProject?.name ?? "None"}</dd>
                  </div>
                  <div>
                    <dt>Open document</dt>
                    <dd>{activeDocument?.name ?? "None"}</dd>
                  </div>
                  <div>
                    <dt>Loaded sections</dt>
                    <dd>{activeDocument ? sections.length : 0}</dd>
                  </div>
                  <div>
                    <dt>Loaded segments</dt>
                    <dd>{activeDocument ? segments.length : 0}</dd>
                  </div>
                  <div>
                    <dt>Loaded chunks</dt>
                    <dd>{activeDocument ? chunks.length : 0}</dd>
                  </div>
                  <div>
                    <dt>Selected chunk</dt>
                    <dd>
                      {selectedChunk ? `#${selectedChunk.sequence}` : "None"}
                    </dd>
                  </div>
                </dl>
              </section>

              <HealthcheckPanel
                error={healthcheckError}
                healthcheck={healthcheck}
                isLoading={isLoadingHealthcheck}
                onRetry={retry}
              />
            </div>
          ) : null}
        </section>
      </section>
    </main>
  );
}
