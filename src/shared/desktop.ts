export const DESKTOP_COMMANDS = {
  createGlossary: "create_glossary",
  createGlossaryEntry: "create_glossary_entry",
  createProject: "create_project",
  createStyleProfile: "create_style_profile",
  healthcheck: "healthcheck",
  importProjectDocument: "import_project_document",
  listGlossaryEntries: "list_glossary_entries",
  listGlossaries: "list_glossaries",
  listProjects: "list_projects",
  listProjectDocuments: "list_project_documents",
  listStyleProfiles: "list_style_profiles",
  listDocumentSegments: "list_document_segments",
  openGlossary: "open_glossary",
  openProject: "open_project",
  openStyleProfile: "open_style_profile",
  processProjectDocument: "process_project_document",
  updateGlossaryEntry: "update_glossary_entry",
  updateGlossary: "update_glossary",
  updateStyleProfile: "update_style_profile",
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

export type GlossaryStatus = "active" | "archived";
export type GlossaryEntryStatus = "active" | "archived";
export type StyleProfileStatus = "active" | "archived";
export type StyleProfileTone = "neutral" | "direct" | "warm" | "technical";
export type StyleProfileFormality =
  | "formal"
  | "neutral"
  | "semi_formal"
  | "informal";
export type StyleProfileTreatmentPreference =
  | "usted"
  | "tuteo"
  | "impersonal"
  | "mixed";

export interface GlossarySummary {
  id: string;
  name: string;
  description: string | null;
  projectId: string | null;
  status: GlossaryStatus;
  createdAt: number;
  updatedAt: number;
  lastOpenedAt: number;
}

export interface GlossariesOverview {
  activeGlossaryId: string | null;
  glossaries: GlossarySummary[];
}

export interface GlossaryEntrySummary {
  id: string;
  glossaryId: string;
  sourceTerm: string;
  targetTerm: string;
  contextNote: string | null;
  status: GlossaryEntryStatus;
  createdAt: number;
  updatedAt: number;
  sourceVariants: string[];
  targetVariants: string[];
  forbiddenTerms: string[];
}

export interface GlossaryEntriesOverview {
  glossaryId: string;
  entries: GlossaryEntrySummary[];
}

export interface StyleProfileSummary {
  id: string;
  name: string;
  description: string | null;
  tone: StyleProfileTone;
  formality: StyleProfileFormality;
  treatmentPreference: StyleProfileTreatmentPreference;
  consistencyInstructions: string | null;
  editorialNotes: string | null;
  status: StyleProfileStatus;
  createdAt: number;
  updatedAt: number;
  lastOpenedAt: number;
}

export interface StyleProfilesOverview {
  activeStyleProfileId: string | null;
  styleProfiles: StyleProfileSummary[];
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

export interface CreateGlossaryInput {
  name: string;
  description?: string;
  projectId?: string;
}

export interface CreateGlossaryEntryInput {
  glossaryId: string;
  sourceTerm: string;
  targetTerm: string;
  contextNote?: string;
  sourceVariants?: string[];
  targetVariants?: string[];
  forbiddenTerms?: string[];
}

export interface CreateStyleProfileInput {
  name: string;
  description?: string;
  tone: StyleProfileTone;
  formality: StyleProfileFormality;
  treatmentPreference: StyleProfileTreatmentPreference;
  consistencyInstructions?: string;
  editorialNotes?: string;
}

export interface OpenProjectInput {
  projectId: string;
}

export interface OpenGlossaryInput {
  glossaryId: string;
}

export interface OpenStyleProfileInput {
  styleProfileId: string;
}

export interface ListGlossaryEntriesInput {
  glossaryId: string;
}

export interface UpdateGlossaryInput {
  glossaryId: string;
  name: string;
  description?: string;
  projectId?: string;
  status: GlossaryStatus;
}

export interface UpdateGlossaryEntryInput {
  glossaryEntryId: string;
  glossaryId: string;
  sourceTerm: string;
  targetTerm: string;
  contextNote?: string;
  sourceVariants?: string[];
  targetVariants?: string[];
  forbiddenTerms?: string[];
  status: GlossaryEntryStatus;
}

export interface UpdateStyleProfileInput {
  styleProfileId: string;
  name: string;
  description?: string;
  tone: StyleProfileTone;
  formality: StyleProfileFormality;
  treatmentPreference: StyleProfileTreatmentPreference;
  consistencyInstructions?: string;
  editorialNotes?: string;
  status: StyleProfileStatus;
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
