export const DESKTOP_COMMANDS = {
  createProject: "create_project",
  healthcheck: "healthcheck",
  importProjectDocument: "import_project_document",
  listProjects: "list_projects",
  listProjectDocuments: "list_project_documents",
  listDocumentSegments: "list_document_segments",
  openProject: "open_project",
  processProjectDocument: "process_project_document",
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

export interface DocumentSummary {
  id: string;
  projectId: string;
  name: string;
  sourceKind: string;
  format: string;
  mimeType: string | null;
  fileSizeBytes: number;
  status: string;
  segmentCount: number;
  createdAt: number;
  updatedAt: number;
}

export interface ProjectDocumentsOverview {
  projectId: string;
  documents: DocumentSummary[];
}

export interface SegmentSummary {
  id: string;
  documentId: string;
  sequence: number;
  sourceText: string;
  targetText: string | null;
  sourceWordCount: number;
  sourceCharacterCount: number;
  status: string;
  createdAt: number;
  updatedAt: number;
}

export interface DocumentSectionSummary {
  id: string;
  documentId: string;
  sequence: number;
  title: string;
  sectionType: string;
  level: number;
  startSegmentSequence: number;
  endSegmentSequence: number;
  segmentCount: number;
  createdAt: number;
  updatedAt: number;
}

export interface DocumentSegmentsOverview {
  projectId: string;
  documentId: string;
  sections: DocumentSectionSummary[];
  segments: SegmentSummary[];
}

export interface CreateProjectInput {
  name: string;
  description?: string;
}

export interface OpenProjectInput {
  projectId: string;
}

export interface ListProjectDocumentsInput {
  projectId: string;
}

export interface ImportDocumentInput {
  projectId: string;
  fileName: string;
  mimeType?: string;
  base64Content: string;
}

export interface ProcessDocumentInput {
  projectId: string;
  documentId: string;
}

export interface ListDocumentSegmentsInput {
  projectId: string;
  documentId: string;
}
