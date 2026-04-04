import { invoke } from "@tauri-apps/api/core";
import {
  type CreateGlossaryInput,
  type CreateProjectInput,
  DESKTOP_COMMANDS,
  type DesktopCommandErrorPayload,
  type DesktopCommandName,
  type DocumentSegmentsOverview,
  type DocumentSummary,
  type GlossariesOverview,
  type GlossarySummary,
  type HealthcheckResponse,
  type ImportDocumentInput,
  type ListDocumentSegmentsInput,
  type ListProjectDocumentsInput,
  type OpenGlossaryInput,
  type OpenProjectInput,
  type ProcessDocumentInput,
  type ProjectDocumentsOverview,
  type ProjectSummary,
  type ProjectsOverview,
  type UpdateGlossaryInput,
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

export function openGlossary(input: OpenGlossaryInput) {
  return invokeDesktopCommand<GlossarySummary>(DESKTOP_COMMANDS.openGlossary, {
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
