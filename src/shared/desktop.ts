export const DESKTOP_COMMANDS = {
  createGlossary: "create_glossary",
  createGlossaryEntry: "create_glossary_entry",
  createProject: "create_project",
  createRule: "create_rule",
  createRuleSet: "create_rule_set",
  createStyleProfile: "create_style_profile",
  buildTranslationContext: "build_translation_context",
  buildDocumentTranslationChunks: "build_document_translation_chunks",
  getReconstructedDocument: "get_reconstructed_document",
  inspectQaFinding: "inspect_qa_finding",
  listDocumentQaFindings: "list_document_qa_findings",
  healthcheck: "healthcheck",
  importProjectDocument: "import_project_document",
  retranslateChunkFromQaFinding: "retranslate_chunk_from_qa_finding",
  runDocumentConsistencyQa: "run_document_consistency_qa",
  listDocumentTranslationChunks: "list_document_translation_chunks",
  translateChunk: "translate_chunk",
  translateDocument: "translate_document",
  getTranslateDocumentJobStatus: "get_translate_document_job_status",
  cancelTranslateDocumentJob: "cancel_translate_document_job",
  resumeTranslateDocumentJob: "resume_translate_document_job",
  listGlossaryEntries: "list_glossary_entries",
  listGlossaries: "list_glossaries",
  listProjects: "list_projects",
  listProjectDocuments: "list_project_documents",
  listRuleSetRules: "list_rule_set_rules",
  listRuleSets: "list_rule_sets",
  listStyleProfiles: "list_style_profiles",
  listDocumentSegments: "list_document_segments",
  openGlossary: "open_glossary",
  openProject: "open_project",
  openRuleSet: "open_rule_set",
  openStyleProfile: "open_style_profile",
  processProjectDocument: "process_project_document",
  updateProjectEditorialDefaults: "update_project_editorial_defaults",
  updateRule: "update_rule",
  updateRuleSet: "update_rule_set",
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
  defaultGlossaryId: string | null;
  defaultStyleProfileId: string | null;
  defaultRuleSetId: string | null;
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
export type RuleSetStatus = "active" | "archived";
export type RuleType = "consistency" | "preference" | "restriction";
export type RuleActionScope =
  | "translation"
  | "retranslation"
  | "qa"
  | "export"
  | "consistency_review";
export type RuleSeverity = "low" | "medium" | "high";
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

export interface RuleSetSummary {
  id: string;
  name: string;
  description: string | null;
  status: RuleSetStatus;
  createdAt: number;
  updatedAt: number;
  lastOpenedAt: number;
}

export interface RuleSetsOverview {
  activeRuleSetId: string | null;
  ruleSets: RuleSetSummary[];
}

export interface RuleSummary {
  id: string;
  ruleSetId: string;
  actionScope: RuleActionScope;
  ruleType: RuleType;
  severity: RuleSeverity;
  name: string;
  description: string | null;
  guidance: string;
  isEnabled: boolean;
  createdAt: number;
  updatedAt: number;
}

export interface RuleSetRulesOverview {
  ruleSetId: string;
  rules: RuleSummary[];
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

export interface TranslationChunkSummary {
  id: string;
  documentId: string;
  sequence: number;
  builderVersion: string;
  strategy: string;
  sourceText: string;
  contextBeforeText: string | null;
  contextAfterText: string | null;
  startSegmentSequence: number;
  endSegmentSequence: number;
  segmentCount: number;
  sourceWordCount: number;
  sourceCharacterCount: number;
  createdAt: number;
  updatedAt: number;
}

export type TranslationChunkSegmentRole =
  | "core"
  | "context_before"
  | "context_after";

export interface TranslationChunkSegmentSummary {
  chunkId: string;
  segmentId: string;
  segmentSequence: number;
  position: number;
  role: TranslationChunkSegmentRole;
}

export interface DocumentTranslationChunksOverview {
  projectId: string;
  documentId: string;
  chunks: TranslationChunkSummary[];
  chunkSegments: TranslationChunkSegmentSummary[];
}

export type ReconstructedDocumentStatus =
  | "empty"
  | "untranslated"
  | "partial"
  | "complete";

export type ReconstructedContentSource =
  | "none"
  | "target"
  | "source_fallback"
  | "mixed";

export interface ReconstructedSegment {
  id: string;
  sequence: number;
  sourceText: string;
  finalText: string | null;
  resolvedText: string;
  resolvedFrom: "target" | "source_fallback";
  status: string;
  primaryChunkId: string | null;
  relatedChunkIds: string[];
}

export interface ReconstructedDocumentBlock {
  id: string;
  sectionId: string | null;
  title: string | null;
  sequence: number;
  kind: string;
  level: number | null;
  startSegmentSequence: number;
  endSegmentSequence: number;
  segmentCount: number;
  translatedSegmentCount: number;
  untranslatedSegmentCount: number;
  fallbackSegmentCount: number;
  status: ReconstructedDocumentStatus;
  contentSource: ReconstructedContentSource;
  finalText: string | null;
  resolvedText: string;
  segmentIds: string[];
  primaryChunkIds: string[];
  segments: ReconstructedSegment[];
}

export interface ReconstructedDocumentSection {
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
  status: ReconstructedDocumentStatus;
  contentSource: ReconstructedContentSource;
  translatedSegmentCount: number;
  untranslatedSegmentCount: number;
  fallbackSegmentCount: number;
  blockId: string;
}

export interface ReconstructedDocumentCompleteness {
  totalSegments: number;
  translatedSegments: number;
  untranslatedSegments: number;
  fallbackSegments: number;
  totalSections: number;
  totalBlocks: number;
  isComplete: boolean;
  hasTranslatedContent: boolean;
  hasReconstructibleContent: boolean;
}

export interface ReconstructedDocumentChunkTrace {
  chunkId: string;
  chunkSequence: number;
  startSegmentSequence: number;
  endSegmentSequence: number;
  coreSegmentIds: string[];
  contextBeforeSegmentIds: string[];
  contextAfterSegmentIds: string[];
  taskRunIds: string[];
  latestTaskRun: TaskRunSummary | null;
}

export interface ReconstructedDocumentTrace {
  chunkCount: number;
  taskRunCount: number;
  documentTaskRunIds: string[];
  latestDocumentTaskRun: TaskRunSummary | null;
  orphanedChunkTaskRuns: TaskRunSummary[];
  chunks: ReconstructedDocumentChunkTrace[];
}

export interface ReconstructedDocument {
  projectId: string;
  documentId: string;
  status: ReconstructedDocumentStatus;
  contentSource: ReconstructedContentSource;
  finalText: string | null;
  resolvedText: string;
  completeness: ReconstructedDocumentCompleteness;
  sections: ReconstructedDocumentSection[];
  blocks: ReconstructedDocumentBlock[];
  trace: ReconstructedDocumentTrace;
}

export type QaFindingSeverity = "low" | "medium" | "high";
export type QaFindingStatus = "open" | "resolved" | "dismissed";

export interface QaFindingSummary {
  id: string;
  documentId: string;
  chunkId: string | null;
  taskRunId: string | null;
  jobId: string | null;
  findingType: string;
  severity: QaFindingSeverity;
  status: QaFindingStatus;
  message: string;
  details: string | null;
  createdAt: number;
  updatedAt: number;
}

export interface TaskRunSummary {
  id: string;
  documentId: string;
  chunkId: string | null;
  jobId: string | null;
  actionType: string;
  status: string;
  inputPayload: string | null;
  outputPayload: string | null;
  errorMessage: string | null;
  startedAt: number;
  completedAt: number | null;
  createdAt: number;
  updatedAt: number;
}

export interface TranslatedChunkSegmentSummary {
  segmentId: string;
  sequence: number;
  targetText: string;
}

export interface TranslateChunkResult {
  projectId: string;
  documentId: string;
  chunkId: string;
  taskRun: TaskRunSummary;
  provider: string;
  model: string;
  actionVersion: string;
  promptVersion: string;
  translatedSegments: TranslatedChunkSegmentSummary[];
}

export type TranslateDocumentStatus =
  | "pending"
  | "running"
  | "completed"
  | "completed_with_errors"
  | "failed"
  | "cancelled";

export type TranslateDocumentChunkStatus =
  | "pending"
  | "running"
  | "completed"
  | "failed"
  | "cancelled";

export interface TranslateDocumentChunkResult {
  chunkId: string;
  chunkSequence: number;
  status: TranslateDocumentChunkStatus;
  taskRun: TaskRunSummary | null;
  translatedSegmentCount: number;
  errorMessage: string | null;
}

export interface TranslateDocumentJobInput {
  projectId: string;
  documentId: string;
  jobId: string;
}

export interface TranslateDocumentJobStatus {
  projectId: string;
  documentId: string;
  jobId: string;
  status: TranslateDocumentStatus;
  totalChunks: number;
  pendingChunks: number;
  runningChunks: number;
  completedChunks: number;
  failedChunks: number;
  cancelledChunks: number;
  currentChunkId: string | null;
  currentChunkSequence: number | null;
  lastCompletedChunkId: string | null;
  lastCompletedChunkSequence: number | null;
  lastUpdatedAt: number | null;
  latestDocumentTaskRun: TaskRunSummary | null;
  chunkStatuses: TranslateDocumentChunkResult[];
  taskRuns: TaskRunSummary[];
  errorMessages: string[];
}

export interface TranslateDocumentResult {
  projectId: string;
  documentId: string;
  jobId: string;
  status: TranslateDocumentStatus;
  actionVersion: string;
  taskRun: TaskRunSummary;
  totalChunks: number;
  completedChunks: number;
  failedChunks: number;
  chunkResults: TranslateDocumentChunkResult[];
  errorMessages: string[];
}

export interface ResolvedGlossaryLayer {
  glossary: GlossarySummary;
  layer: string;
  source: string;
  priority: number;
}

export interface ResolvedGlossaryEntry {
  entry: GlossaryEntrySummary;
  glossaryName: string;
  layer: string;
  source: string;
  priority: number;
}

export interface ResolvedStyleProfile {
  styleProfile: StyleProfileSummary;
  source: string;
  priority: number;
}

export interface ResolvedRuleSet {
  ruleSet: RuleSetSummary;
  source: string;
  priority: number;
}

export interface ResolvedRule {
  rule: RuleSummary;
  ruleSetName: string;
  source: string;
  priority: number;
}

export interface TranslationChunkContext {
  chunk: TranslationChunkSummary;
  section: DocumentSectionSummary | null;
  coreSegments: SegmentSummary[];
  contextBeforeSegments: SegmentSummary[];
  contextAfterSegments: SegmentSummary[];
}

export interface ResolvedChapterContext {
  chapterContext: {
    id: string;
    documentId: string;
    sectionId: string | null;
    taskRunId: string | null;
    scopeType: string;
    startSegmentSequence: number;
    endSegmentSequence: number;
    contextText: string;
    sourceSummary: string | null;
    contextWordCount: number;
    contextCharacterCount: number;
    createdAt: number;
    updatedAt: number;
  };
  matchReason: string;
  priority: number;
}

export interface TranslationContextResolution {
  glossaryIds: string[];
  styleProfileId: string | null;
  ruleSetId: string | null;
  sectionId: string | null;
  chapterContextIds: string[];
}

export interface TranslationContextPreview {
  projectId: string;
  documentId: string;
  chunkId: string;
  actionScope: RuleActionScope;
  glossaryLayers: ResolvedGlossaryLayer[];
  glossaryEntries: ResolvedGlossaryEntry[];
  styleProfile: ResolvedStyleProfile | null;
  ruleSet: ResolvedRuleSet | null;
  rules: ResolvedRule[];
  chunkContext: TranslationChunkContext;
  accumulatedContexts: ResolvedChapterContext[];
  resolution: TranslationContextResolution;
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

export interface CreateRuleSetInput {
  name: string;
  description?: string;
}

export interface CreateRuleInput {
  ruleSetId: string;
  actionScope: RuleActionScope;
  ruleType: RuleType;
  severity: RuleSeverity;
  name: string;
  description?: string;
  guidance: string;
  isEnabled: boolean;
}

export interface OpenProjectInput {
  projectId: string;
}

export interface UpdateProjectEditorialDefaultsInput {
  projectId: string;
  defaultGlossaryId?: string;
  defaultStyleProfileId?: string;
  defaultRuleSetId?: string;
}

export interface OpenGlossaryInput {
  glossaryId: string;
}

export interface OpenStyleProfileInput {
  styleProfileId: string;
}

export interface OpenRuleSetInput {
  ruleSetId: string;
}

export interface ListGlossaryEntriesInput {
  glossaryId: string;
}

export interface ListRuleSetRulesInput {
  ruleSetId: string;
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

export interface UpdateRuleSetInput {
  ruleSetId: string;
  name: string;
  description?: string;
  status: RuleSetStatus;
}

export interface UpdateRuleInput {
  ruleId: string;
  ruleSetId: string;
  actionScope: RuleActionScope;
  ruleType: RuleType;
  severity: RuleSeverity;
  name: string;
  description?: string;
  guidance: string;
  isEnabled: boolean;
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

export interface BuildDocumentTranslationChunksInput {
  projectId: string;
  documentId: string;
}

export interface BuildTranslationContextInput {
  projectId: string;
  documentId: string;
  chunkId: string;
  actionScope: RuleActionScope;
}

export interface TranslateChunkInput {
  projectId: string;
  documentId: string;
  chunkId: string;
  jobId?: string;
}

export interface TranslateDocumentInput {
  projectId: string;
  documentId: string;
  jobId?: string;
}

export interface GetReconstructedDocumentInput {
  projectId: string;
  documentId: string;
}

export interface RunDocumentConsistencyQaInput {
  projectId: string;
  documentId: string;
  jobId?: string | null;
}

export interface ListDocumentQaFindingsInput {
  projectId: string;
  documentId: string;
  jobId?: string | null;
}

export interface DocumentConsistencyQaResult {
  projectId: string;
  documentId: string;
  jobId: string | null;
  reconstructedStatus: ReconstructedDocumentStatus;
  reconstructedContentSource: ReconstructedContentSource;
  generatedFindings: QaFindingSummary[];
}

export interface DocumentQaFindingsOverview {
  projectId: string;
  documentId: string;
  jobId: string | null;
  findings: QaFindingSummary[];
}

export interface QaFindingChunkAnchor {
  findingId: string;
  chunkId: string | null;
  chunkSequence: number | null;
  resolutionKind: string;
  resolutionMessage: string;
  canRetranslate: boolean;
}

export interface QaFindingReviewContext {
  projectId: string;
  documentId: string;
  finding: QaFindingSummary;
  anchor: QaFindingChunkAnchor;
  chunk: TranslationChunkSummary | null;
  chunkSegments: TranslationChunkSegmentSummary[];
  latestChunkTaskRun: TaskRunSummary | null;
  latestDocumentTaskRun: TaskRunSummary | null;
  relatedBlock: ReconstructedDocumentBlock | null;
  relatedSegments: ReconstructedSegment[];
}

export interface QaFindingRetranslationResult {
  projectId: string;
  documentId: string;
  finding: QaFindingSummary;
  anchor: QaFindingChunkAnchor;
  correctionJobId: string;
  translateResult: TranslateChunkResult;
}

export interface ListDocumentSegmentsInput {
  projectId: string;
  documentId: string;
}

export interface InspectQaFindingInput {
  projectId: string;
  documentId: string;
  findingId: string;
}

export interface ListDocumentTranslationChunksInput {
  projectId: string;
  documentId: string;
}

export interface RetranslateChunkFromQaFindingInput {
  projectId: string;
  documentId: string;
  findingId: string;
  jobId?: string;
}
