import { invoke } from "@tauri-apps/api/core";
import {
  DESKTOP_COMMANDS,
  type DesktopCommandErrorPayload,
  type DesktopCommandName,
  type HealthcheckResponse,
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
