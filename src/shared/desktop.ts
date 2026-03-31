export const DESKTOP_COMMANDS = {
  healthcheck: "healthcheck",
} as const;

export type DesktopCommandName =
  (typeof DESKTOP_COMMANDS)[keyof typeof DESKTOP_COMMANDS];

export interface DesktopCommandErrorPayload {
  code: string;
  details?: string;
  message: string;
}

export interface HealthcheckResponse {
  appName: string;
  checkedAt: number;
  environment: "development" | "production";
  message: string;
  status: "ok";
  version: string;
}
