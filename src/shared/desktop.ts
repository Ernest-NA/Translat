export const DESKTOP_COMMANDS = {
  createProject: "create_project",
  healthcheck: "healthcheck",
  listProjects: "list_projects",
  openProject: "open_project",
} as const;

export type DesktopCommandName =
  (typeof DESKTOP_COMMANDS)[keyof typeof DESKTOP_COMMANDS];

export interface DesktopCommandErrorPayload {
  code: string;
  details?: string;
  message: string;
}

export interface DatabaseStatus {
  appliedMigrations: string[];
  encryption: string;
  keyStorage: string;
  migrationCount: number;
  path: string;
  schemaReady: boolean;
}

export interface HealthcheckResponse {
  appName: string;
  checkedAt: number;
  database: DatabaseStatus;
  environment: "development" | "production";
  message: string;
  status: "ok";
  version: string;
}

export interface ProjectSummary {
  id: string;
  name: string;
  description: string | null;
  createdAt: number;
  updatedAt: number;
  lastOpenedAt: number;
}

export interface ProjectsOverview {
  activeProjectId: string | null;
  projects: ProjectSummary[];
}

export interface CreateProjectInput {
  name: string;
  description?: string;
}

export interface OpenProjectInput {
  projectId: string;
}
