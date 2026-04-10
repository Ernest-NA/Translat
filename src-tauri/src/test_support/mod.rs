#![allow(dead_code)]

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use tempfile::{tempdir, TempDir};

use crate::commands::document_export::export_reconstructed_document_with_runtime;
use crate::commands::document_qa::{
    list_document_qa_findings_with_runtime, run_document_consistency_qa_with_runtime,
};
use crate::commands::documents::import_project_document_with_runtime;
use crate::commands::finding_review::{
    inspect_qa_finding_with_runtime, retranslate_chunk_from_qa_finding_with_runtime_and_executor,
};
use crate::commands::observability::inspect_document_operational_state_with_runtime;
use crate::commands::projects::{
    create_project_with_runtime, open_project_with_runtime, OpenProjectInput,
};
use crate::commands::reconstructed_documents::get_reconstructed_document_with_runtime;
use crate::commands::segments::{
    list_document_segments_with_runtime, process_project_document_with_runtime,
};
use crate::commands::translate_document::translate_document_with_runtime_and_executor;
use crate::commands::translate_document_jobs::get_translate_document_job_status_with_runtime;
use crate::commands::translation_chunks::build_document_translation_chunks_with_runtime;
use crate::document_export::{ExportReconstructedDocumentInput, ExportReconstructedDocumentResult};
use crate::document_qa::{
    DocumentConsistencyQaResult, DocumentQaFindingsOverview, ListDocumentQaFindingsInput,
    RunDocumentConsistencyQaInput,
};
use crate::documents::{DocumentSummary, ImportDocumentInput};
use crate::error::DesktopCommandError;
use crate::finding_review::{
    InspectQaFindingInput, QaFindingRetranslationResult, QaFindingReviewContext,
    RetranslateChunkFromQaFindingInput,
};
use crate::observability::{DocumentOperationalState, InspectDocumentOperationalStateInput};
use crate::persistence::bootstrap::{bootstrap_database, DatabaseRuntime};
use crate::persistence::secret_store::load_or_create_encryption_key;
use crate::projects::{CreateProjectInput, ProjectSummary};
use crate::reconstructed_documents::{GetReconstructedDocumentInput, ReconstructedDocument};
use crate::segments::{DocumentSegmentsOverview, SegmentSummary};
use crate::translate_chunk::{
    TranslateChunkActionRequest, TranslateChunkActionResponse, TranslateChunkExecutionFailure,
    TranslateChunkExecutor, TranslateChunkModelOutput, TranslateChunkTranslation,
};
use crate::translate_document::{
    TranslateDocumentInput, TranslateDocumentJobInput, TranslateDocumentJobStatus,
    TranslateDocumentResult,
};
use crate::translation_chunks::{
    BuildDocumentTranslationChunksInput, DocumentTranslationChunksOverview,
    TRANSLATION_CHUNK_SEGMENT_ROLE_CORE,
};

pub const SAMPLE_CHAPTERED_DOCUMENT_TEXT: &str = "Chapter I\n\nThe gate remained closed.\nThe lantern burned all night.\n\nChapter II\n\nGuard the archive.\nKeep the signal hidden.";

pub struct RuntimeFixture {
    _temporary_directory: TempDir,
    pub runtime: DatabaseRuntime,
}

pub fn create_runtime_fixture() -> RuntimeFixture {
    let temporary_directory = tempdir().expect("temp dir should be created");
    let database_path = temporary_directory.path().join("translat.sqlite3");
    let encryption_key_path = temporary_directory.path().join("translat.sqlite3.key");
    let runtime = DatabaseRuntime::new(database_path.clone(), encryption_key_path.clone());
    let encryption_key =
        load_or_create_encryption_key(&encryption_key_path).expect("key should persist");

    bootstrap_database(&database_path, &encryption_key)
        .expect("database bootstrap should succeed");

    RuntimeFixture {
        _temporary_directory: temporary_directory,
        runtime,
    }
}

#[derive(Debug, Clone)]
pub struct DocumentPipelineFixture {
    pub project: ProjectSummary,
    pub document: DocumentSummary,
    pub source_text: String,
}

impl DocumentPipelineFixture {
    pub fn import_document(
        database_runtime: &DatabaseRuntime,
        project_name: &str,
        file_name: &str,
        source_text: &str,
    ) -> Result<Self, DesktopCommandError> {
        let project = create_project_with_runtime(
            CreateProjectInput {
                name: project_name.to_owned(),
                description: Some("TR-26 reusable pipeline fixture".to_owned()),
            },
            database_runtime,
        )?;
        let opened_project = open_project_with_runtime(
            OpenProjectInput {
                project_id: project.id.clone(),
            },
            database_runtime,
        )?;
        let document = import_project_document_with_runtime(
            ImportDocumentInput {
                project_id: opened_project.id.clone(),
                file_name: file_name.to_owned(),
                mime_type: Some("text/plain".to_owned()),
                base64_content: STANDARD.encode(source_text.as_bytes()),
            },
            database_runtime,
        )?;

        Ok(Self {
            project: opened_project,
            document,
            source_text: source_text.to_owned(),
        })
    }

    pub fn process_document(
        &mut self,
        database_runtime: &DatabaseRuntime,
    ) -> Result<DocumentSummary, DesktopCommandError> {
        let document = process_project_document_with_runtime(
            crate::segments::ProcessDocumentInput {
                project_id: self.project.id.clone(),
                document_id: self.document.id.clone(),
            },
            database_runtime,
        )?;

        self.document = document.clone();

        Ok(document)
    }

    pub fn list_segments(
        &self,
        database_runtime: &DatabaseRuntime,
    ) -> Result<DocumentSegmentsOverview, DesktopCommandError> {
        list_document_segments_with_runtime(
            crate::segments::ListDocumentSegmentsInput {
                project_id: self.project.id.clone(),
                document_id: self.document.id.clone(),
            },
            database_runtime,
        )
    }

    pub fn build_chunks(
        &self,
        database_runtime: &DatabaseRuntime,
    ) -> Result<DocumentTranslationChunksOverview, DesktopCommandError> {
        build_document_translation_chunks_with_runtime(
            BuildDocumentTranslationChunksInput {
                project_id: self.project.id.clone(),
                document_id: self.document.id.clone(),
            },
            database_runtime,
        )
    }

    pub fn translate_document<E: TranslateChunkExecutor>(
        &self,
        database_runtime: &DatabaseRuntime,
        job_id: Option<String>,
        executor: &E,
    ) -> Result<TranslateDocumentResult, DesktopCommandError> {
        translate_document_with_runtime_and_executor(
            TranslateDocumentInput {
                project_id: self.project.id.clone(),
                document_id: self.document.id.clone(),
                job_id,
            },
            database_runtime,
            executor,
        )
    }

    pub fn job_status(
        &self,
        database_runtime: &DatabaseRuntime,
        job_id: &str,
    ) -> Result<TranslateDocumentJobStatus, DesktopCommandError> {
        get_translate_document_job_status_with_runtime(
            TranslateDocumentJobInput {
                project_id: self.project.id.clone(),
                document_id: self.document.id.clone(),
                job_id: job_id.to_owned(),
            },
            database_runtime,
        )
    }

    pub fn reconstruct(
        &self,
        database_runtime: &DatabaseRuntime,
    ) -> Result<ReconstructedDocument, DesktopCommandError> {
        get_reconstructed_document_with_runtime(
            GetReconstructedDocumentInput {
                project_id: self.project.id.clone(),
                document_id: self.document.id.clone(),
            },
            database_runtime,
        )
    }

    pub fn run_qa(
        &self,
        database_runtime: &DatabaseRuntime,
        job_id: Option<String>,
    ) -> Result<DocumentConsistencyQaResult, DesktopCommandError> {
        run_document_consistency_qa_with_runtime(
            RunDocumentConsistencyQaInput {
                project_id: self.project.id.clone(),
                document_id: self.document.id.clone(),
                job_id,
            },
            database_runtime,
        )
    }

    pub fn list_qa_findings(
        &self,
        database_runtime: &DatabaseRuntime,
        job_id: Option<String>,
    ) -> Result<DocumentQaFindingsOverview, DesktopCommandError> {
        list_document_qa_findings_with_runtime(
            ListDocumentQaFindingsInput {
                project_id: self.project.id.clone(),
                document_id: self.document.id.clone(),
                job_id,
            },
            database_runtime,
        )
    }

    pub fn inspect_finding(
        &self,
        database_runtime: &DatabaseRuntime,
        finding_id: &str,
    ) -> Result<QaFindingReviewContext, DesktopCommandError> {
        inspect_qa_finding_with_runtime(
            InspectQaFindingInput {
                project_id: self.project.id.clone(),
                document_id: self.document.id.clone(),
                finding_id: finding_id.to_owned(),
            },
            database_runtime,
        )
    }

    pub fn retranslate_finding<E: TranslateChunkExecutor>(
        &self,
        database_runtime: &DatabaseRuntime,
        finding_id: &str,
        job_id: Option<String>,
        executor: &E,
    ) -> Result<QaFindingRetranslationResult, DesktopCommandError> {
        retranslate_chunk_from_qa_finding_with_runtime_and_executor(
            RetranslateChunkFromQaFindingInput {
                project_id: self.project.id.clone(),
                document_id: self.document.id.clone(),
                finding_id: finding_id.to_owned(),
                job_id,
            },
            database_runtime,
            executor,
        )
    }

    pub fn export_document(
        &self,
        database_runtime: &DatabaseRuntime,
    ) -> Result<ExportReconstructedDocumentResult, DesktopCommandError> {
        export_reconstructed_document_with_runtime(
            ExportReconstructedDocumentInput {
                project_id: self.project.id.clone(),
                document_id: self.document.id.clone(),
            },
            database_runtime,
        )
    }

    pub fn inspect_operational_state(
        &self,
        database_runtime: &DatabaseRuntime,
        job_id: Option<String>,
    ) -> Result<DocumentOperationalState, DesktopCommandError> {
        inspect_document_operational_state_with_runtime(
            InspectDocumentOperationalStateInput {
                project_id: self.project.id.clone(),
                document_id: self.document.id.clone(),
                job_id,
            },
            database_runtime,
        )
    }
}

pub fn core_segment_ids_for_chunk(
    chunk_overview: &DocumentTranslationChunksOverview,
    chunk_id: &str,
) -> Vec<String> {
    let mut core_segments = chunk_overview
        .chunk_segments
        .iter()
        .filter(|segment| {
            segment.chunk_id == chunk_id && segment.role == TRANSLATION_CHUNK_SEGMENT_ROLE_CORE
        })
        .cloned()
        .collect::<Vec<_>>();

    core_segments.sort_by(|left, right| {
        left.position
            .cmp(&right.position)
            .then_with(|| left.segment_sequence.cmp(&right.segment_sequence))
    });

    core_segments
        .into_iter()
        .map(|segment| segment.segment_id)
        .collect()
}

pub fn build_chunk_response(
    segments: &[SegmentSummary],
    segment_ids: &[String],
    translation_by_source_text: &HashMap<&str, &str>,
    notes: Option<&str>,
) -> TranslateChunkActionResponse {
    let translations = segment_ids
        .iter()
        .map(|segment_id| {
            let segment = segments
                .iter()
                .find(|segment| segment.id == *segment_id)
                .expect("segment should exist in the segmented document overview");
            let target_text = translation_by_source_text
                .get(segment.source_text.as_str())
                .expect("translation should exist for the source text");

            TranslateChunkTranslation {
                segment_id: segment_id.clone(),
                target_text: (*target_text).to_owned(),
            }
        })
        .collect();

    TranslateChunkActionResponse {
        translations,
        notes: notes.map(str::to_owned),
    }
}

#[derive(Debug, Clone)]
pub enum ScriptedChunkOutcome {
    Success(TranslateChunkActionResponse),
    Failure(TranslateChunkExecutionFailure),
}

pub struct ScriptedTranslateChunkExecutor {
    responses: HashMap<String, ScriptedChunkOutcome>,
    observed_chunk_ids: Arc<Mutex<Vec<String>>>,
}

impl ScriptedTranslateChunkExecutor {
    pub fn new(responses: HashMap<String, ScriptedChunkOutcome>) -> Self {
        Self {
            responses,
            observed_chunk_ids: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn observed_chunk_ids(&self) -> Vec<String> {
        self.observed_chunk_ids
            .lock()
            .expect("observed chunk lock should open")
            .clone()
    }
}

impl TranslateChunkExecutor for ScriptedTranslateChunkExecutor {
    fn execute(
        &self,
        request: &TranslateChunkActionRequest,
    ) -> Result<TranslateChunkModelOutput, TranslateChunkExecutionFailure> {
        self.observed_chunk_ids
            .lock()
            .expect("observed chunk lock should open")
            .push(request.chunk_id.clone());

        match self.responses.get(&request.chunk_id) {
            Some(ScriptedChunkOutcome::Success(response)) => Ok(TranslateChunkModelOutput {
                provider: "fixture".to_owned(),
                model: "fixture-model".to_owned(),
                raw_output: serde_json::to_string(response)
                    .expect("fixture translate_chunk response should serialize"),
            }),
            Some(ScriptedChunkOutcome::Failure(error)) => Err(error.clone()),
            None => panic!("missing scripted response for chunk {}", request.chunk_id),
        }
    }
}

pub fn translate_chunk_failure(message: &str) -> TranslateChunkExecutionFailure {
    TranslateChunkExecutionFailure {
        message: message.to_owned(),
        details: Some("TR-26 scripted smoke failure".to_owned()),
        raw_output: None,
    }
}
