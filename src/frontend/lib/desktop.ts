import { invoke } from "@tauri-apps/api/core";
import {
  type BuildDocumentTranslationChunksInput,
  type BuildTranslationContextInput,
  type CreateGlossaryEntryInput,
  type CreateGlossaryInput,
  type CreateProjectInput,
  type CreateRuleInput,
  type CreateRuleSetInput,
  type CreateStyleProfileInput,
  DESKTOP_COMMANDS,
  type DesktopCommandErrorPayload,
  type DesktopCommandName,
  type DocumentConsistencyQaResult,
  type DocumentOperationalState,
  type DocumentQaFindingsOverview,
  type DocumentSegmentsOverview,
  type DocumentSummary,
  type DocumentTranslationChunksOverview,
  type ExportReconstructedDocumentInput,
  type ExportReconstructedDocumentResult,
  type GetReconstructedDocumentInput,
  type GlossariesOverview,
  type GlossaryEntriesOverview,
  type GlossaryEntrySummary,
  type GlossarySummary,
  type HealthcheckResponse,
  type ImportDocumentInput,
  type InspectDocumentOperationalStateInput,
  type InspectJobTraceInput,
  type InspectQaFindingInput,
  type JobTraceInspection,
  type ListDocumentQaFindingsInput,
  type ListDocumentSegmentsInput,
  type ListDocumentTranslationChunksInput,
  type ListGlossaryEntriesInput,
  type ListProjectDocumentsInput,
  type ListRuleSetRulesInput,
  type OpenGlossaryInput,
  type OpenProjectInput,
  type OpenRuleSetInput,
  type OpenStyleProfileInput,
  type ProcessDocumentInput,
  type ProjectDocumentsOverview,
  type ProjectSummary,
  type ProjectsOverview,
  type QaFindingRetranslationResult,
  type QaFindingReviewContext,
  type ReconstructedDocument,
  type RetranslateChunkFromQaFindingInput,
  type RuleSetRulesOverview,
  type RuleSetSummary,
  type RuleSetsOverview,
  type RuleSummary,
  type RunDocumentConsistencyQaInput,
  type StyleProfileSummary,
  type StyleProfilesOverview,
  type TranslateChunkInput,
  type TranslateChunkResult,
  type TranslateDocumentInput,
  type TranslateDocumentJobInput,
  type TranslateDocumentJobStatus,
  type TranslateDocumentResult,
  type TranslationContextPreview,
  type UpdateGlossaryEntryInput,
  type UpdateGlossaryInput,
  type UpdateProjectEditorialDefaultsInput,
  type UpdateRuleInput,
  type UpdateRuleSetInput,
  type UpdateStyleProfileInput,
} from "../../shared/desktop";

export class DesktopCommandError extends Error {
  code: string;
  command: DesktopCommandName;
  details?: string;

  constructor(
    command: DesktopCommandName,
    payload: DesktopCommandErrorPayload,
  ) {
    super(payload.message);
    this.name = "DesktopCommandError";
    this.code = payload.code;
    this.command = command;
    this.details = payload.details;
  }
}

export const DESKTOP_RUNTIME_UNAVAILABLE_CODE = "DESKTOP_RUNTIME_UNAVAILABLE";

function isDesktopCommandErrorPayload(
  value: unknown,
): value is DesktopCommandErrorPayload {
  if (!value || typeof value !== "object") {
    return false;
  }

  const candidate = value as Record<string, unknown>;

  return (
    typeof candidate.code === "string" && typeof candidate.message === "string"
  );
}

function isDesktopRuntimeUnavailableError(
  caughtError: unknown,
): caughtError is Error {
  if (!(caughtError instanceof Error)) {
    return false;
  }

  return (
    caughtError.message.includes("Cannot read properties of undefined") &&
    caughtError.message.includes("invoke")
  );
}

function normalizeDesktopCommandError(
  command: DesktopCommandName,
  caughtError: unknown,
) {
  if (isDesktopCommandErrorPayload(caughtError)) {
    return new DesktopCommandError(command, caughtError);
  }

  if (isDesktopRuntimeUnavailableError(caughtError)) {
    return new DesktopCommandError(command, {
      code: DESKTOP_RUNTIME_UNAVAILABLE_CODE,
      details: caughtError.stack,
      message:
        "Desktop runtime unavailable in this browser preview. Open the Tauri desktop app to use persisted project and document commands.",
    });
  }

  if (caughtError instanceof Error) {
    return new DesktopCommandError(command, {
      code: "UNEXPECTED_DESKTOP_ERROR",
      details: caughtError.stack,
      message: caughtError.message,
    });
  }

  return new DesktopCommandError(command, {
    code: "UNEXPECTED_DESKTOP_ERROR",
    message: "The desktop shell returned an unknown error.",
  });
}

export async function invokeDesktopCommand<TResponse>(
  command: DesktopCommandName,
  args?: Record<string, unknown>,
) {
  try {
    return await invoke<TResponse>(command, args);
  } catch (caughtError) {
    const normalizedError = normalizeDesktopCommandError(command, caughtError);

    if (import.meta.env.DEV) {
      console.error(`[desktop:${command}]`, normalizedError);
    }

    throw normalizedError;
  }
}

export function runHealthcheck() {
  return invokeDesktopCommand<HealthcheckResponse>(
    DESKTOP_COMMANDS.healthcheck,
  );
}

export function listProjects() {
  return invokeDesktopCommand<ProjectsOverview>(DESKTOP_COMMANDS.listProjects);
}

export function listGlossaries() {
  return invokeDesktopCommand<GlossariesOverview>(
    DESKTOP_COMMANDS.listGlossaries,
  );
}

export function listStyleProfiles() {
  return invokeDesktopCommand<StyleProfilesOverview>(
    DESKTOP_COMMANDS.listStyleProfiles,
  );
}

export function listRuleSets() {
  return invokeDesktopCommand<RuleSetsOverview>(DESKTOP_COMMANDS.listRuleSets);
}

export function listGlossaryEntries(input: ListGlossaryEntriesInput) {
  return invokeDesktopCommand<GlossaryEntriesOverview>(
    DESKTOP_COMMANDS.listGlossaryEntries,
    {
      input,
    },
  );
}

export function listRuleSetRules(input: ListRuleSetRulesInput) {
  return invokeDesktopCommand<RuleSetRulesOverview>(
    DESKTOP_COMMANDS.listRuleSetRules,
    {
      input,
    },
  );
}

export function createProject(input: CreateProjectInput) {
  return invokeDesktopCommand<ProjectSummary>(DESKTOP_COMMANDS.createProject, {
    input,
  });
}

export function createGlossary(input: CreateGlossaryInput) {
  return invokeDesktopCommand<GlossarySummary>(
    DESKTOP_COMMANDS.createGlossary,
    {
      input,
    },
  );
}

export function createStyleProfile(input: CreateStyleProfileInput) {
  return invokeDesktopCommand<StyleProfileSummary>(
    DESKTOP_COMMANDS.createStyleProfile,
    {
      input,
    },
  );
}

export function createRuleSet(input: CreateRuleSetInput) {
  return invokeDesktopCommand<RuleSetSummary>(DESKTOP_COMMANDS.createRuleSet, {
    input,
  });
}

export function createRule(input: CreateRuleInput) {
  return invokeDesktopCommand<RuleSummary>(DESKTOP_COMMANDS.createRule, {
    input,
  });
}

export function createGlossaryEntry(input: CreateGlossaryEntryInput) {
  return invokeDesktopCommand<GlossaryEntrySummary>(
    DESKTOP_COMMANDS.createGlossaryEntry,
    {
      input,
    },
  );
}

export function listProjectDocuments(input: ListProjectDocumentsInput) {
  return invokeDesktopCommand<ProjectDocumentsOverview>(
    DESKTOP_COMMANDS.listProjectDocuments,
    {
      input,
    },
  );
}

export function listDocumentSegments(input: ListDocumentSegmentsInput) {
  return invokeDesktopCommand<DocumentSegmentsOverview>(
    DESKTOP_COMMANDS.listDocumentSegments,
    {
      input,
    },
  );
}

export function buildDocumentTranslationChunks(
  input: BuildDocumentTranslationChunksInput,
) {
  return invokeDesktopCommand<DocumentTranslationChunksOverview>(
    DESKTOP_COMMANDS.buildDocumentTranslationChunks,
    {
      input,
    },
  );
}

export function buildTranslationContext(input: BuildTranslationContextInput) {
  return invokeDesktopCommand<TranslationContextPreview>(
    DESKTOP_COMMANDS.buildTranslationContext,
    {
      input,
    },
  );
}

export function listDocumentTranslationChunks(
  input: ListDocumentTranslationChunksInput,
) {
  return invokeDesktopCommand<DocumentTranslationChunksOverview>(
    DESKTOP_COMMANDS.listDocumentTranslationChunks,
    {
      input,
    },
  );
}

export function translateChunk(input: TranslateChunkInput) {
  return invokeDesktopCommand<TranslateChunkResult>(
    DESKTOP_COMMANDS.translateChunk,
    {
      input,
    },
  );
}

export function translateDocument(input: TranslateDocumentInput) {
  return invokeDesktopCommand<TranslateDocumentResult>(
    DESKTOP_COMMANDS.translateDocument,
    {
      input,
    },
  );
}

export function getReconstructedDocument(input: GetReconstructedDocumentInput) {
  return invokeDesktopCommand<ReconstructedDocument>(
    DESKTOP_COMMANDS.getReconstructedDocument,
    {
      input,
    },
  );
}

export function exportReconstructedDocument(
  input: ExportReconstructedDocumentInput,
) {
  return invokeDesktopCommand<ExportReconstructedDocumentResult>(
    DESKTOP_COMMANDS.exportReconstructedDocument,
    {
      input,
    },
  );
}

export function inspectDocumentOperationalState(
  input: InspectDocumentOperationalStateInput,
) {
  return invokeDesktopCommand<DocumentOperationalState>(
    DESKTOP_COMMANDS.inspectDocumentOperationalState,
    {
      input,
    },
  );
}

export function inspectJobTrace(input: InspectJobTraceInput) {
  return invokeDesktopCommand<JobTraceInspection>(
    DESKTOP_COMMANDS.inspectJobTrace,
    {
      input,
    },
  );
}

export function inspectQaFinding(input: InspectQaFindingInput) {
  return invokeDesktopCommand<QaFindingReviewContext>(
    DESKTOP_COMMANDS.inspectQaFinding,
    {
      input,
    },
  );
}

export function runDocumentConsistencyQa(input: RunDocumentConsistencyQaInput) {
  return invokeDesktopCommand<DocumentConsistencyQaResult>(
    DESKTOP_COMMANDS.runDocumentConsistencyQa,
    {
      input,
    },
  );
}

export function listDocumentQaFindings(input: ListDocumentQaFindingsInput) {
  return invokeDesktopCommand<DocumentQaFindingsOverview>(
    DESKTOP_COMMANDS.listDocumentQaFindings,
    {
      input,
    },
  );
}

export function retranslateChunkFromQaFinding(
  input: RetranslateChunkFromQaFindingInput,
) {
  return invokeDesktopCommand<QaFindingRetranslationResult>(
    DESKTOP_COMMANDS.retranslateChunkFromQaFinding,
    {
      input,
    },
  );
}

export function getTranslateDocumentJobStatus(
  input: TranslateDocumentJobInput,
) {
  return invokeDesktopCommand<TranslateDocumentJobStatus>(
    DESKTOP_COMMANDS.getTranslateDocumentJobStatus,
    {
      input,
    },
  );
}

export function cancelTranslateDocumentJob(input: TranslateDocumentJobInput) {
  return invokeDesktopCommand<TranslateDocumentJobStatus>(
    DESKTOP_COMMANDS.cancelTranslateDocumentJob,
    {
      input,
    },
  );
}

export function resumeTranslateDocumentJob(input: TranslateDocumentJobInput) {
  return invokeDesktopCommand<TranslateDocumentResult>(
    DESKTOP_COMMANDS.resumeTranslateDocumentJob,
    {
      input,
    },
  );
}

export function importProjectDocument(input: ImportDocumentInput) {
  return invokeDesktopCommand<DocumentSummary>(
    DESKTOP_COMMANDS.importProjectDocument,
    {
      input,
    },
  );
}

export function processProjectDocument(input: ProcessDocumentInput) {
  return invokeDesktopCommand<DocumentSummary>(
    DESKTOP_COMMANDS.processProjectDocument,
    {
      input,
    },
  );
}

export function openProject(input: OpenProjectInput) {
  return invokeDesktopCommand<ProjectSummary>(DESKTOP_COMMANDS.openProject, {
    input,
  });
}

export function updateProjectEditorialDefaults(
  input: UpdateProjectEditorialDefaultsInput,
) {
  return invokeDesktopCommand<ProjectSummary>(
    DESKTOP_COMMANDS.updateProjectEditorialDefaults,
    {
      input,
    },
  );
}

export function openGlossary(input: OpenGlossaryInput) {
  return invokeDesktopCommand<GlossarySummary>(DESKTOP_COMMANDS.openGlossary, {
    input,
  });
}

export function openStyleProfile(input: OpenStyleProfileInput) {
  return invokeDesktopCommand<StyleProfileSummary>(
    DESKTOP_COMMANDS.openStyleProfile,
    {
      input,
    },
  );
}

export function openRuleSet(input: OpenRuleSetInput) {
  return invokeDesktopCommand<RuleSetSummary>(DESKTOP_COMMANDS.openRuleSet, {
    input,
  });
}

export function updateGlossary(input: UpdateGlossaryInput) {
  return invokeDesktopCommand<GlossarySummary>(
    DESKTOP_COMMANDS.updateGlossary,
    {
      input,
    },
  );
}

export function updateStyleProfile(input: UpdateStyleProfileInput) {
  return invokeDesktopCommand<StyleProfileSummary>(
    DESKTOP_COMMANDS.updateStyleProfile,
    {
      input,
    },
  );
}

export function updateRuleSet(input: UpdateRuleSetInput) {
  return invokeDesktopCommand<RuleSetSummary>(DESKTOP_COMMANDS.updateRuleSet, {
    input,
  });
}

export function updateRule(input: UpdateRuleInput) {
  return invokeDesktopCommand<RuleSummary>(DESKTOP_COMMANDS.updateRule, {
    input,
  });
}

export function updateGlossaryEntry(input: UpdateGlossaryEntryInput) {
  return invokeDesktopCommand<GlossaryEntrySummary>(
    DESKTOP_COMMANDS.updateGlossaryEntry,
    {
      input,
    },
  );
}
