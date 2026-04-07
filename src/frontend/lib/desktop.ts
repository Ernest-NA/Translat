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
  type DocumentSegmentsOverview,
  type DocumentSummary,
  type DocumentTranslationChunksOverview,
  type GlossariesOverview,
  type GlossaryEntriesOverview,
  type GlossaryEntrySummary,
  type GlossarySummary,
  type HealthcheckResponse,
  type ImportDocumentInput,
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
  type RuleSetRulesOverview,
  type RuleSetSummary,
  type RuleSetsOverview,
  type RuleSummary,
  type StyleProfileSummary,
  type StyleProfilesOverview,
  type TranslateChunkInput,
  type TranslateChunkResult,
  type TranslateDocumentInput,
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

function normalizeDesktopCommandError(
  command: DesktopCommandName,
  caughtError: unknown,
) {
  if (isDesktopCommandErrorPayload(caughtError)) {
    return new DesktopCommandError(command, caughtError);
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
