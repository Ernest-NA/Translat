import { useCallback, useRef, useState } from "react";
import { DESKTOP_COMMANDS } from "../../shared/desktop";
import { GlossaryWorkspace } from "../components/GlossaryWorkspace";
import { HealthcheckPanel } from "../components/HealthcheckPanel";
import { ProjectComposer } from "../components/ProjectComposer";
import { ProjectList } from "../components/ProjectList";
import { ProjectWorkspace } from "../components/ProjectWorkspace";
import { RuleSetsWorkspace } from "../components/RuleSetsWorkspace";
import { StyleProfilesWorkspace } from "../components/StyleProfilesWorkspace";
import { useDocumentChunks } from "../hooks/useDocumentChunks";
import { useDocumentSegments } from "../hooks/useDocumentSegments";
import { useGlossariesWorkspace } from "../hooks/useGlossariesWorkspace";
import { useHealthcheck } from "../hooks/useHealthcheck";
import { useProjectDocuments } from "../hooks/useProjectDocuments";
import { useProjectsWorkspace } from "../hooks/useProjectsWorkspace";
import { useRuleSetsWorkspace } from "../hooks/useRuleSetsWorkspace";
import { useStyleProfilesWorkspace } from "../hooks/useStyleProfilesWorkspace";

function formatCheckedAt(value?: number) {
  if (!value) {
    return "Pending first response";
  }

  return new Date(value).toLocaleString();
}

export function AppShell() {
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
  const { error, healthcheck, isLoading, retry } = useHealthcheck();
  const activeProjectIdRef = useRef<string | null>(activeProject?.id ?? null);
  const [hasUnsavedProjectDefaults, setHasUnsavedProjectDefaults] =
    useState(false);
  activeProjectIdRef.current = activeProject?.id ?? null;

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

      return submitProject(input);
    },
    [confirmDiscardProjectDefaults, submitProject],
  );

  const handleSelectProject = useCallback(
    async (projectId: string) => {
      if (projectId === activeProject?.id) {
        return true;
      }

      if (!confirmDiscardProjectDefaults("switch")) {
        return false;
      }

      return selectProject(projectId);
    },
    [activeProject?.id, confirmDiscardProjectDefaults, selectProject],
  );

  const handleImportDocuments = useCallback(
    async (files: FileList): Promise<number> => {
      const importedCount = await importDocuments(files);

      if (importedCount > 0) {
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
      const processedDocument = await processDocument(documentId);

      if (processedDocument) {
        if (activeProjectIdRef.current !== processedDocument.projectId) {
          return;
        }

        await openDocument(processedDocument.id);

        if (activeProjectIdRef.current !== processedDocument.projectId) {
          return;
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

  const runtimeLabel = healthcheck
    ? `${healthcheck.environment} | v${healthcheck.version}`
    : "desktop runtime";

  return (
    <main className="app-shell">
      <header className="app-shell__header">
        <div>
          <p className="app-shell__eyebrow">Translat</p>
          <h1>Translation workspace</h1>
          <p className="app-shell__lead">
            The shell now keeps project defaults, document intake, chunk
            readiness, and document-level translation control in the same
            workspace. Translation stays document-first, with the job visible as
            an execution envelope and the chunk as the main inspection unit.
          </p>
        </div>

        <div className="app-shell__header-meta">
          <span>{runtimeLabel}</span>
          <span>{projects.length} persisted projects</span>
          <span>{totalRuleSetCount} persisted rule sets</span>
          <span>{totalStyleProfileCount} persisted style profiles</span>
          <span>{totalGlossaryCount} persisted glossaries</span>
          <span>
            {activeRuleSet
              ? `Open rule set: ${activeRuleSet.name}`
              : "No open rule set"}
          </span>
          <span>
            {activeStyleProfile
              ? `Open style profile: ${activeStyleProfile.name}`
              : "No open style profile"}
          </span>
          <span>
            {activeGlossary
              ? `Open glossary: ${activeGlossary.name}`
              : "No open glossary"}
          </span>
          <span>
            {activeProject
              ? `${documents.length} project documents`
              : "No active project"}
          </span>
          <span>
            {activeDocument
              ? `${chunks.length} loaded chunks for ${activeDocument.name}`
              : "No active document"}
          </span>
          <span>
            {activeProjectDefaultGlossary
              ? `Project default glossary: ${activeProjectDefaultGlossary.name}`
              : "Project default glossary: none"}
          </span>
          <span>
            {activeProjectDefaultStyleProfile
              ? `Project default style: ${activeProjectDefaultStyleProfile.name}`
              : "Project default style: none"}
          </span>
          <span>
            {activeProjectDefaultRuleSet
              ? `Project default rules: ${activeProjectDefaultRuleSet.name}`
              : "Project default rules: none"}
          </span>
        </div>
      </header>

      <section className="app-shell__grid">
        <div className="app-shell__primary">
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
            styleProfiles={styleProfiles}
          />
        </div>

        <aside className="app-shell__sidebar">
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

          <section className="surface-card">
            <p className="surface-card__eyebrow">Command pattern</p>
            <h2>{DESKTOP_COMMANDS.updateProjectEditorialDefaults}</h2>

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
                <dt>Open glossary</dt>
                <dd>{activeGlossary?.name ?? "None"}</dd>
              </div>
              <div>
                <dt>Open rule set</dt>
                <dd>{activeRuleSet?.name ?? "None"}</dd>
              </div>
              <div>
                <dt>Open style profile</dt>
                <dd>{activeStyleProfile?.name ?? "None"}</dd>
              </div>
              <div>
                <dt>Rule-set status</dt>
                <dd>{activeRuleSet?.status ?? "None"}</dd>
              </div>
              <div>
                <dt>Style profile status</dt>
                <dd>{activeStyleProfile?.status ?? "None"}</dd>
              </div>
              <div>
                <dt>Glossary status</dt>
                <dd>{activeGlossary?.status ?? "None"}</dd>
              </div>
              <div>
                <dt>Open project</dt>
                <dd>{activeProject?.name ?? "None"}</dd>
              </div>
              <div>
                <dt>Project default glossary</dt>
                <dd>{activeProjectDefaultGlossary?.name ?? "None"}</dd>
              </div>
              <div>
                <dt>Project default style</dt>
                <dd>{activeProjectDefaultStyleProfile?.name ?? "None"}</dd>
              </div>
              <div>
                <dt>Project default rules</dt>
                <dd>{activeProjectDefaultRuleSet?.name ?? "None"}</dd>
              </div>
              <div>
                <dt>Glossary totals</dt>
                <dd>
                  {activeGlossaryCount} active | {archivedGlossaryCount}{" "}
                  archived
                </dd>
              </div>
              <div>
                <dt>Rule-set totals</dt>
                <dd>
                  {activeRuleSetCount} active | {archivedRuleSetCount} archived
                </dd>
              </div>
              <div>
                <dt>Style profile totals</dt>
                <dd>
                  {activeStyleProfileCount} active | {archivedStyleProfileCount}{" "}
                  archived
                </dd>
              </div>
              <div>
                <dt>Open document</dt>
                <dd>{activeDocument?.name ?? "None"}</dd>
              </div>
              <div>
                <dt>Loaded segments</dt>
                <dd>{activeDocument ? segments.length : 0}</dd>
              </div>
              <div>
                <dt>Loaded sections</dt>
                <dd>{activeDocument ? sections.length : 0}</dd>
              </div>
              <div>
                <dt>Loaded chunks</dt>
                <dd>{activeDocument ? chunks.length : 0}</dd>
              </div>
              <div>
                <dt>Selected segment</dt>
                <dd>
                  {selectedSegment ? `#${selectedSegment.sequence}` : "None"}
                </dd>
              </div>
              <div>
                <dt>Selected chunk</dt>
                <dd>{selectedChunk ? `#${selectedChunk.sequence}` : "None"}</dd>
              </div>
              <div>
                <dt>Selected section</dt>
                <dd>{selectedSection?.title ?? "None"}</dd>
              </div>
            </dl>
          </section>

          <HealthcheckPanel
            error={error}
            healthcheck={healthcheck}
            isLoading={isLoading}
            onRetry={retry}
          />
        </aside>
      </section>
    </main>
  );
}
