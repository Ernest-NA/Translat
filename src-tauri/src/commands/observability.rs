use std::collections::BTreeSet;

use serde::Deserialize;
use tauri::State;

use crate::commands::document_export::select_export_source_task_run;
use crate::commands::reconstructed_documents::get_reconstructed_document_with_runtime;
use crate::commands::translate_document_jobs::{
    build_job_status_if_exists, current_timestamp, validate_identifier,
};
use crate::document_export::EXPORT_RECONSTRUCTED_DOCUMENT_ACTION_TYPE;
use crate::error::DesktopCommandError;
use crate::observability::{
    DocumentJobOverview, DocumentOperationalState, InspectDocumentOperationalStateInput,
    InspectJobTraceInput, JobTraceInspection, OperationalExportTrace, OperationalInspectionWarning,
    ReconstructedOperationalSnapshot, OPERATIONAL_WARNING_SEVERITY_INFO,
    OPERATIONAL_WARNING_SEVERITY_WARNING,
};
use crate::persistence::bootstrap::DatabaseRuntime;
use crate::persistence::documents::DocumentRepository;
use crate::persistence::qa_findings::QaFindingRepository;
use crate::persistence::segments::SegmentRepository;
use crate::persistence::task_runs::TaskRunRepository;
use crate::qa_findings::{QaFindingSummary, QA_FINDING_STATUS_OPEN};
use crate::reconstructed_documents::GetReconstructedDocumentInput;
use crate::task_runs::{TaskRunSummary, TASK_RUN_STATUS_COMPLETED};
use crate::translate_chunk::TRANSLATE_CHUNK_ACTION_TYPE;
use crate::translate_document::TRANSLATE_DOCUMENT_ACTION_TYPE;

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct ExportTracePayload {
    file_name: Option<String>,
    format: Option<String>,
    exported_at: Option<i64>,
    reconstructed_status: Option<String>,
    content_source: Option<String>,
    is_complete: Option<bool>,
    total_segments: Option<i64>,
    translated_segments: Option<i64>,
    fallback_segments: Option<i64>,
    source_job_id: Option<String>,
    source_task_run_id: Option<String>,
    open_finding_count: Option<i64>,
}

#[tauri::command]
pub fn inspect_document_operational_state(
    input: InspectDocumentOperationalStateInput,
    database_runtime: State<'_, DatabaseRuntime>,
) -> Result<DocumentOperationalState, DesktopCommandError> {
    inspect_document_operational_state_with_runtime(input, database_runtime.inner())
}

#[tauri::command]
pub fn inspect_job_trace(
    input: InspectJobTraceInput,
    database_runtime: State<'_, DatabaseRuntime>,
) -> Result<JobTraceInspection, DesktopCommandError> {
    inspect_job_trace_with_runtime(input, database_runtime.inner())
}

pub(crate) fn inspect_document_operational_state_with_runtime(
    input: InspectDocumentOperationalStateInput,
    database_runtime: &DatabaseRuntime,
) -> Result<DocumentOperationalState, DesktopCommandError> {
    let project_id = validate_identifier(&input.project_id, "project id")?;
    let document_id = validate_identifier(&input.document_id, "document id")?;
    let selected_job_id = input
        .job_id
        .as_deref()
        .map(|job_id| validate_identifier(job_id, "job id"))
        .transpose()?;
    let observed_at = current_timestamp()?;
    let mut connection = database_runtime.open_connection().map_err(|error| {
        DesktopCommandError::internal(
            "The desktop shell could not open the encrypted database for operational inspection.",
            Some(error.to_string()),
        )
    })?;
    let document = DocumentRepository::new(&mut connection)
        .load_processing_record(&project_id, &document_id)
        .map_err(|error| {
            DesktopCommandError::internal(
                "The desktop shell could not load the selected document for operational inspection.",
                Some(error.to_string()),
            )
        })?
        .ok_or_else(|| {
            DesktopCommandError::validation(
                "The selected document does not exist in the active project.",
                None,
            )
        })?;
    let task_runs = TaskRunRepository::new(&mut connection)
        .list_by_document(&document_id)
        .map_err(|error| {
            DesktopCommandError::internal(
                "The desktop shell could not load task runs for the selected document.",
                Some(error.to_string()),
            )
        })?;
    let findings = sort_findings_desc(
        QaFindingRepository::new(&mut connection)
            .list_by_document(&document_id)
            .map_err(|error| {
                DesktopCommandError::internal(
                    "The desktop shell could not load QA findings for the selected document.",
                    Some(error.to_string()),
                )
            })?,
    );
    let segment_translation_traces = SegmentRepository::new(&mut connection)
        .list_translation_trace_by_document(&document_id)
        .map_err(|error| {
            DesktopCommandError::internal(
                "The desktop shell could not load segment translation provenance for operational inspection.",
                Some(error.to_string()),
            )
        })?;
    let reconstruction = get_reconstructed_document_with_runtime(
        GetReconstructedDocumentInput {
            project_id: project_id.clone(),
            document_id: document_id.clone(),
        },
        database_runtime,
    )?;
    let exports = collect_export_traces(&task_runs);
    let latest_reconstructed_source_task_run_id =
        select_export_source_task_run(&segment_translation_traces, &task_runs)
            .map(|task_run| task_run.id);
    let jobs = collect_job_overviews(
        &mut connection,
        database_runtime,
        &project_id,
        &document_id,
        &findings,
        &task_runs,
    )?;
    let requested_selected_job_id = selected_job_id.clone();
    let fallback_selected_job_id = jobs.first().map(|job| job.job_id.clone());
    let selected_job_id = match requested_selected_job_id.as_deref() {
        Some(job_id)
            if jobs
                .iter()
                .any(|job| job.job_id.as_str() == job_id) =>
        {
            Some(job_id.to_owned())
        }
        Some(_) => None,
        None => fallback_selected_job_id.clone(),
    };
    let selected_job = match selected_job_id.clone() {
        Some(job_id) => Some(inspect_job_trace_internal(
            &mut connection,
            database_runtime,
            &project_id,
            &document_id,
            &job_id,
            &findings,
            &exports,
        )?),
        None => None,
    };
    let warnings = derive_document_warnings(
        &task_runs,
        &findings,
        &reconstruction,
        &exports,
        latest_reconstructed_source_task_run_id.as_deref(),
        requested_selected_job_id.as_deref(),
        selected_job_id.as_deref(),
    );

    Ok(DocumentOperationalState {
        project_id,
        document_id,
        document_name: document.name,
        document_status: document.status,
        observed_at,
        selected_job_id,
        recent_runs: sort_task_runs_desc(task_runs).into_iter().take(12).collect(),
        jobs,
        selected_job,
        open_finding_count: open_finding_count(&findings),
        findings,
        reconstruction: ReconstructedOperationalSnapshot {
            status: reconstruction.status,
            content_source: reconstruction.content_source,
            total_segments: reconstruction.completeness.total_segments,
            translated_segments: reconstruction.completeness.translated_segments,
            fallback_segments: reconstruction.completeness.fallback_segments,
            is_complete: reconstruction.completeness.is_complete,
            latest_document_task_run_id: reconstruction
                .trace
                .latest_document_task_run
                .as_ref()
                .map(|task_run| task_run.id.clone()),
            orphaned_chunk_task_run_ids: reconstruction
                .trace
                .orphaned_chunk_task_runs
                .into_iter()
                .map(|task_run| task_run.id)
                .collect(),
        },
        exports,
        warnings,
    })
}

pub(crate) fn inspect_job_trace_with_runtime(
    input: InspectJobTraceInput,
    database_runtime: &DatabaseRuntime,
) -> Result<JobTraceInspection, DesktopCommandError> {
    let project_id = validate_identifier(&input.project_id, "project id")?;
    let document_id = validate_identifier(&input.document_id, "document id")?;
    let job_id = validate_identifier(&input.job_id, "job id")?;
    let mut connection = database_runtime.open_connection().map_err(|error| {
        DesktopCommandError::internal(
            "The desktop shell could not open the encrypted database for job-trace inspection.",
            Some(error.to_string()),
        )
    })?;
    let findings = sort_findings_desc(
        QaFindingRepository::new(&mut connection)
            .list_by_document(&document_id)
            .map_err(|error| {
                DesktopCommandError::internal(
                    "The desktop shell could not load QA findings for the selected document.",
                    Some(error.to_string()),
                )
            })?,
    );
    let task_runs = TaskRunRepository::new(&mut connection)
        .list_by_document(&document_id)
        .map_err(|error| {
            DesktopCommandError::internal(
                "The desktop shell could not load task runs for the selected document.",
                Some(error.to_string()),
            )
        })?;
    let exports = collect_export_traces(&task_runs);

    inspect_job_trace_internal(
        &mut connection,
        database_runtime,
        &project_id,
        &document_id,
        &job_id,
        &findings,
        &exports,
    )
}

fn inspect_job_trace_internal(
    connection: &mut rusqlite::Connection,
    database_runtime: &DatabaseRuntime,
    project_id: &str,
    document_id: &str,
    job_id: &str,
    document_findings: &[QaFindingSummary],
    exports: &[OperationalExportTrace],
) -> Result<JobTraceInspection, DesktopCommandError> {
    let job_status = build_job_status_if_exists(
        connection,
        database_runtime,
        project_id,
        document_id,
        job_id,
    )?
    .ok_or_else(|| {
        DesktopCommandError::validation(
            "The selected job does not exist for the active document.",
            None,
        )
    })?;
    let findings = sort_findings_desc(
        document_findings
            .iter()
            .filter(|finding| finding.job_id.as_deref() == Some(job_id))
            .cloned()
            .collect(),
    );
    let related_exports = exports
        .iter()
        .filter(|export_trace| export_trace.source_job_id.as_deref() == Some(job_id))
        .cloned()
        .collect::<Vec<_>>();

    let sorted_task_runs = sort_task_runs_desc(job_status.task_runs.clone());
    let warnings = derive_job_warnings(&job_status.task_runs, &related_exports, &job_status.status);

    Ok(JobTraceInspection {
        overview: build_job_overview(&job_status, &findings),
        task_runs: sorted_task_runs,
        findings,
        warnings,
        related_exports,
    })
}

fn collect_job_overviews(
    connection: &mut rusqlite::Connection,
    database_runtime: &DatabaseRuntime,
    project_id: &str,
    document_id: &str,
    findings: &[QaFindingSummary],
    task_runs: &[TaskRunSummary],
) -> Result<Vec<DocumentJobOverview>, DesktopCommandError> {
    let mut job_ids = BTreeSet::new();

    for task_run in task_runs.iter().filter(|task_run| {
        matches!(
            task_run.action_type.as_str(),
            TRANSLATE_DOCUMENT_ACTION_TYPE | TRANSLATE_CHUNK_ACTION_TYPE
        )
    }) {
        if let Some(job_id) = task_run.job_id.as_deref() {
            job_ids.insert(job_id.to_owned());
        }
    }

    let mut jobs = Vec::with_capacity(job_ids.len());

    for job_id in job_ids {
        if let Some(job_status) = build_job_status_if_exists(
            connection,
            database_runtime,
            project_id,
            document_id,
            &job_id,
        )? {
            let job_findings = findings
                .iter()
                .filter(|finding| finding.job_id.as_deref() == Some(job_id.as_str()))
                .cloned()
                .collect::<Vec<_>>();

            jobs.push(build_job_overview(&job_status, &job_findings));
        }
    }

    jobs.sort_by(|left, right| {
        right
            .last_updated_at
            .cmp(&left.last_updated_at)
            .then_with(|| right.job_id.cmp(&left.job_id))
    });

    Ok(jobs)
}

fn build_job_overview(
    job_status: &crate::translate_document::TranslateDocumentJobStatus,
    findings: &[QaFindingSummary],
) -> DocumentJobOverview {
    DocumentJobOverview {
        job_id: job_status.job_id.clone(),
        status: job_status.status.clone(),
        last_updated_at: job_status.last_updated_at,
        total_chunks: job_status.total_chunks,
        completed_chunks: job_status.completed_chunks,
        failed_chunks: job_status.failed_chunks,
        cancelled_chunks: job_status.cancelled_chunks,
        finding_count: i64::try_from(findings.len()).unwrap_or(i64::MAX),
        open_finding_count: open_finding_count(findings),
        latest_error_message: job_status.error_messages.last().cloned(),
    }
}

fn collect_export_traces(task_runs: &[TaskRunSummary]) -> Vec<OperationalExportTrace> {
    let mut traces = task_runs
        .iter()
        .filter(|task_run| {
            task_run.action_type == EXPORT_RECONSTRUCTED_DOCUMENT_ACTION_TYPE
                && task_run.status == TASK_RUN_STATUS_COMPLETED
        })
        .map(|task_run| {
            let payload = task_run
                .input_payload
                .as_deref()
                .and_then(|payload| serde_json::from_str::<ExportTracePayload>(payload).ok())
                .unwrap_or_default();

            OperationalExportTrace {
                task_run: task_run.clone(),
                file_name: payload.file_name.unwrap_or_else(|| "unknown.translated.md".to_owned()),
                format: payload.format.unwrap_or_else(|| "md".to_owned()),
                exported_at: payload.exported_at.unwrap_or(task_run.updated_at),
                reconstructed_status: payload
                    .reconstructed_status
                    .unwrap_or_else(|| "unknown".to_owned()),
                content_source: payload.content_source.unwrap_or_else(|| "unknown".to_owned()),
                is_complete: payload.is_complete.unwrap_or(false),
                total_segments: payload.total_segments.unwrap_or(0),
                translated_segments: payload.translated_segments.unwrap_or(0),
                fallback_segments: payload.fallback_segments.unwrap_or(0),
                source_job_id: payload.source_job_id,
                source_task_run_id: payload.source_task_run_id,
                open_finding_count: payload.open_finding_count.unwrap_or(0),
            }
        })
        .collect::<Vec<_>>();

    traces.sort_by(|left, right| {
        right
            .exported_at
            .cmp(&left.exported_at)
            .then_with(|| right.task_run.id.cmp(&left.task_run.id))
    });

    traces
}

fn derive_document_warnings(
    task_runs: &[TaskRunSummary],
    findings: &[QaFindingSummary],
    reconstruction: &crate::reconstructed_documents::ReconstructedDocument,
    exports: &[OperationalExportTrace],
    latest_reconstructed_source_task_run_id: Option<&str>,
    requested_selected_job_id: Option<&str>,
    resolved_selected_job_id: Option<&str>,
) -> Vec<OperationalInspectionWarning> {
    let mut warnings = Vec::new();
    let open_finding_count = open_finding_count(findings);

    if requested_selected_job_id.is_some() && resolved_selected_job_id.is_none() {
        warnings.push(OperationalInspectionWarning {
            code: "selected_job_id_not_found".to_owned(),
            severity: OPERATIONAL_WARNING_SEVERITY_INFO.to_owned(),
            message: "The requested job_id is no longer available for this document, so the document inspection was returned without a selected job trace.".to_owned(),
        });
    }

    if !reconstruction.trace.orphaned_chunk_task_runs.is_empty() {
        warnings.push(OperationalInspectionWarning {
            code: "orphaned_chunk_runs".to_owned(),
            severity: OPERATIONAL_WARNING_SEVERITY_WARNING.to_owned(),
            message: format!(
                "This document keeps {} orphaned chunk task run(s) after chunk history changes.",
                reconstruction.trace.orphaned_chunk_task_runs.len()
            ),
        });
    }

    if open_finding_count > 0 && !reconstruction.completeness.is_complete {
        warnings.push(OperationalInspectionWarning {
            code: "partial_document_with_open_findings".to_owned(),
            severity: OPERATIONAL_WARNING_SEVERITY_WARNING.to_owned(),
            message: format!(
                "The reconstructed document is still partial while {open_finding_count} QA finding(s) remain open."
            ),
        });
    }

    if task_runs.iter().any(|task_run| {
        matches!(
            task_run.action_type.as_str(),
            TRANSLATE_DOCUMENT_ACTION_TYPE | TRANSLATE_CHUNK_ACTION_TYPE
        ) && task_run.job_id.is_none()
    }) {
        warnings.push(OperationalInspectionWarning {
            code: "runs_without_job_id".to_owned(),
            severity: OPERATIONAL_WARNING_SEVERITY_INFO.to_owned(),
            message: "Some translation task runs do not carry a job_id, so they cannot be grouped into a single document job trace.".to_owned(),
        });
    }

    if let Some(latest_export) = exports.first() {
        if !latest_export.is_complete {
            warnings.push(OperationalInspectionWarning {
                code: "incomplete_export_snapshot".to_owned(),
                severity: OPERATIONAL_WARNING_SEVERITY_WARNING.to_owned(),
                message: format!(
                    "The latest export snapshot (`{}`) was produced from an incomplete reconstructed document.",
                    latest_export.file_name
                ),
            });
        }

        if latest_reconstructed_source_task_run_id.is_some()
            && latest_export.source_task_run_id.as_deref()
                != latest_reconstructed_source_task_run_id
        {
            warnings.push(OperationalInspectionWarning {
                code: "export_behind_latest_document_run".to_owned(),
                severity: OPERATIONAL_WARNING_SEVERITY_WARNING.to_owned(),
                message: "The latest export snapshot is not linked to the latest run that contributed to the current reconstructed state, so exported output may lag behind recent chunk-level corrections.".to_owned(),
            });
        }
    }

    if findings
        .iter()
        .any(|finding| finding.status == QA_FINDING_STATUS_OPEN && finding.task_run_id.is_none())
    {
        warnings.push(OperationalInspectionWarning {
            code: "findings_without_task_run".to_owned(),
            severity: OPERATIONAL_WARNING_SEVERITY_INFO.to_owned(),
            message: "Some open QA findings are not linked to a task run, which weakens chunk-level troubleshooting.".to_owned(),
        });
    }

    warnings
}

fn derive_job_warnings(
    task_runs: &[TaskRunSummary],
    related_exports: &[OperationalExportTrace],
    job_status: &str,
) -> Vec<OperationalInspectionWarning> {
    let mut warnings = Vec::new();
    let has_document_run = task_runs
        .iter()
        .any(|task_run| task_run.action_type == TRANSLATE_DOCUMENT_ACTION_TYPE);
    let has_chunk_run = task_runs
        .iter()
        .any(|task_run| task_run.action_type == TRANSLATE_CHUNK_ACTION_TYPE);

    if !has_document_run && has_chunk_run {
        warnings.push(OperationalInspectionWarning {
            code: "chunk_only_job_trace".to_owned(),
            severity: OPERATIONAL_WARNING_SEVERITY_INFO.to_owned(),
            message: "This job only keeps chunk-level task runs, which is valid for focused correction flows without a document-level orchestration run.".to_owned(),
        });
    }

    if task_runs.iter().any(|task_run| task_run.status == "failed") {
        warnings.push(OperationalInspectionWarning {
            code: "failed_task_runs".to_owned(),
            severity: OPERATIONAL_WARNING_SEVERITY_WARNING.to_owned(),
            message: "This job includes failed task runs. Inspect the latest failed chunk or document run before resuming.".to_owned(),
        });
    }

    if task_runs.iter().any(|task_run| task_run.status == "cancelled") {
        warnings.push(OperationalInspectionWarning {
            code: "cancelled_task_runs".to_owned(),
            severity: OPERATIONAL_WARNING_SEVERITY_INFO.to_owned(),
            message: "This job includes cancelled task runs and may require a resume pass to finish unresolved chunks.".to_owned(),
        });
    }

    if matches!(job_status, "completed" | "completed_with_errors") && related_exports.is_empty() {
        warnings.push(OperationalInspectionWarning {
            code: "job_without_export_snapshot".to_owned(),
            severity: OPERATIONAL_WARNING_SEVERITY_INFO.to_owned(),
            message: "This completed job does not have a recorded export snapshot yet.".to_owned(),
        });
    }

    warnings
}

fn open_finding_count(findings: &[QaFindingSummary]) -> i64 {
    i64::try_from(
        findings
            .iter()
            .filter(|finding| finding.status == QA_FINDING_STATUS_OPEN)
            .count(),
    )
    .unwrap_or(i64::MAX)
}

fn sort_task_runs_desc(mut task_runs: Vec<TaskRunSummary>) -> Vec<TaskRunSummary> {
    task_runs.sort_by(|left, right| {
        right
            .updated_at
            .cmp(&left.updated_at)
            .then_with(|| right.created_at.cmp(&left.created_at))
            .then_with(|| right.id.cmp(&left.id))
    });

    task_runs
}

fn sort_findings_desc(mut findings: Vec<QaFindingSummary>) -> Vec<QaFindingSummary> {
    findings.sort_by(|left, right| {
        right
            .updated_at
            .cmp(&left.updated_at)
            .then_with(|| right.created_at.cmp(&left.created_at))
            .then_with(|| right.id.cmp(&left.id))
    });

    findings
}

#[cfg(test)]
mod tests {
    use tempfile::{tempdir, TempDir};

    use super::{
        inspect_document_operational_state_with_runtime, inspect_job_trace_with_runtime,
    };
    use crate::commands::document_export::export_reconstructed_document_with_runtime;
    use crate::document_export::{
        ExportReconstructedDocumentInput, EXPORT_RECONSTRUCTED_DOCUMENT_ACTION_TYPE,
    };
    use crate::documents::{NewDocument, DOCUMENT_SOURCE_LOCAL_FILE, DOCUMENT_STATUS_SEGMENTED};
    use crate::observability::{InspectDocumentOperationalStateInput, InspectJobTraceInput};
    use crate::persistence::bootstrap::{bootstrap_database, DatabaseRuntime};
    use crate::persistence::documents::DocumentRepository;
    use crate::persistence::projects::ProjectRepository;
    use crate::persistence::qa_findings::QaFindingRepository;
    use crate::persistence::secret_store::load_or_create_encryption_key;
    use crate::persistence::sections::DocumentSectionRepository;
    use crate::persistence::segments::SegmentRepository;
    use crate::persistence::task_runs::TaskRunRepository;
    use crate::persistence::translation_chunks::TranslationChunkRepository;
    use crate::projects::NewProject;
    use crate::qa_findings::{
        NewQaFinding, QA_FINDING_SEVERITY_HIGH, QA_FINDING_STATUS_OPEN,
    };
    use crate::sections::{NewDocumentSection, DOCUMENT_SECTION_TYPE_CHAPTER};
    use crate::segments::{NewSegment, SegmentTranslationWrite, SEGMENT_STATUS_PENDING_TRANSLATION};
    use crate::task_runs::{
        NewTaskRun, TASK_RUN_STATUS_COMPLETED, TASK_RUN_STATUS_FAILED, TASK_RUN_STATUS_RUNNING,
    };
    use crate::translate_chunk::TRANSLATE_CHUNK_ACTION_TYPE;
    use crate::translate_document::TRANSLATE_DOCUMENT_ACTION_TYPE;
    use crate::translation_chunks::{
        NewTranslationChunk, NewTranslationChunkSegment,
        TRANSLATION_CHUNK_SEGMENT_ROLE_CONTEXT_AFTER, TRANSLATION_CHUNK_SEGMENT_ROLE_CORE,
    };

    const PROJECT_ID: &str = "prj_observability_001";
    const DOCUMENT_ID: &str = "doc_observability_001";
    const JOB_ID: &str = "job_observability_001";
    const NOW: i64 = 1_900_300_000;

    struct RuntimeFixture {
        _temporary_directory: TempDir,
        runtime: DatabaseRuntime,
    }

    #[test]
    fn inspect_document_operational_state_consolidates_runs_findings_and_exports() {
        let fixture = create_runtime_fixture();
        seed_observability_graph(&fixture.runtime);

        let export = export_reconstructed_document_with_runtime(
            ExportReconstructedDocumentInput {
                project_id: PROJECT_ID.to_owned(),
                document_id: DOCUMENT_ID.to_owned(),
            },
            &fixture.runtime,
        )
        .expect("export should succeed for observability inspection");

        let inspection = inspect_document_operational_state_with_runtime(
            InspectDocumentOperationalStateInput {
                project_id: PROJECT_ID.to_owned(),
                document_id: DOCUMENT_ID.to_owned(),
                job_id: Some(JOB_ID.to_owned()),
            },
            &fixture.runtime,
        )
        .expect("document operational state should load");

        assert_eq!(inspection.document_id, DOCUMENT_ID);
        assert_eq!(inspection.selected_job_id.as_deref(), Some(JOB_ID));
        assert_eq!(inspection.open_finding_count, 1);
        assert_eq!(inspection.jobs.len(), 1);
        assert_eq!(inspection.exports.len(), 1);
        assert_eq!(inspection.exports[0].file_name, export.file_name);
        assert_eq!(inspection.reconstruction.status, "partial");
        assert!(inspection
            .warnings
            .iter()
            .any(|warning| warning.code == "partial_document_with_open_findings"));
        assert!(inspection
            .warnings
            .iter()
            .any(|warning| warning.code == "incomplete_export_snapshot"));
    }

    #[test]
    fn inspect_document_operational_state_ignores_stale_selected_job_id() {
        let fixture = create_runtime_fixture();
        seed_observability_graph(&fixture.runtime);

        let inspection = inspect_document_operational_state_with_runtime(
            InspectDocumentOperationalStateInput {
                project_id: PROJECT_ID.to_owned(),
                document_id: DOCUMENT_ID.to_owned(),
                job_id: Some("job_missing_001".to_owned()),
            },
            &fixture.runtime,
        )
        .expect("document operational state should still load when the selected job is stale");

        assert_eq!(inspection.document_id, DOCUMENT_ID);
        assert_eq!(inspection.jobs.len(), 1);
        assert!(inspection.selected_job.is_none());
        assert!(inspection.selected_job_id.is_none());
        assert!(inspection
            .warnings
            .iter()
            .any(|warning| warning.code == "selected_job_id_not_found"));
    }

    #[test]
    fn inspect_document_operational_state_ignores_failed_export_attempts() {
        let fixture = create_runtime_fixture();
        seed_observability_graph(&fixture.runtime);
        let mut connection = fixture
            .runtime
            .open_connection()
            .expect("database connection should open");

        TaskRunRepository::new(&mut connection)
            .create(&NewTaskRun {
                id: "task_export_failed_0001".to_owned(),
                document_id: DOCUMENT_ID.to_owned(),
                chunk_id: None,
                job_id: None,
                action_type: EXPORT_RECONSTRUCTED_DOCUMENT_ACTION_TYPE.to_owned(),
                status: TASK_RUN_STATUS_FAILED.to_owned(),
                input_payload: Some(
                    "{\"fileName\":\"draft.translated.md\",\"sourceJobId\":\"job_observability_001\"}"
                        .to_owned(),
                ),
                output_payload: Some("{\"outcome\":\"validation_error\"}".to_owned()),
                error_message: Some("No reconstructible content".to_owned()),
                started_at: NOW + 70,
                completed_at: Some(NOW + 70),
                created_at: NOW + 70,
                updated_at: NOW + 70,
            })
            .expect("failed export attempt should persist");
        drop(connection);

        let inspection = inspect_document_operational_state_with_runtime(
            InspectDocumentOperationalStateInput {
                project_id: PROJECT_ID.to_owned(),
                document_id: DOCUMENT_ID.to_owned(),
                job_id: Some(JOB_ID.to_owned()),
            },
            &fixture.runtime,
        )
        .expect("document operational state should load without failed exports");

        assert!(inspection.exports.is_empty());
        assert!(inspection
            .warnings
            .iter()
            .all(|warning| warning.code != "incomplete_export_snapshot"));
        assert!(inspection
            .selected_job
            .as_ref()
            .expect("selected job should resolve")
            .warnings
            .iter()
            .any(|warning| warning.code == "job_without_export_snapshot"));
    }

    #[test]
    fn inspect_job_trace_links_findings_and_warns_when_export_is_missing() {
        let fixture = create_runtime_fixture();
        seed_observability_graph(&fixture.runtime);

        let inspection = inspect_job_trace_with_runtime(
            InspectJobTraceInput {
                project_id: PROJECT_ID.to_owned(),
                document_id: DOCUMENT_ID.to_owned(),
                job_id: JOB_ID.to_owned(),
            },
            &fixture.runtime,
        )
        .expect("job trace should load");

        assert_eq!(inspection.overview.job_id, JOB_ID);
        assert_eq!(inspection.findings.len(), 1);
        assert!(inspection.related_exports.is_empty());
        assert!(inspection
            .warnings
            .iter()
            .any(|warning| warning.code == "job_without_export_snapshot"));
    }

    #[test]
    fn inspect_job_trace_treats_chunk_only_correction_jobs_as_valid() {
        let fixture = create_runtime_fixture();
        seed_observability_graph(&fixture.runtime);
        let correction_job_id = "job_observability_review_001";
        let mut connection = fixture
            .runtime
            .open_connection()
            .expect("database connection should open");

        TaskRunRepository::new(&mut connection)
            .create(&NewTaskRun {
                id: "task_observability_chunk_review_0001".to_owned(),
                document_id: DOCUMENT_ID.to_owned(),
                chunk_id: Some("doc_observability_001_chunk_0002".to_owned()),
                job_id: Some(correction_job_id.to_owned()),
                action_type: TRANSLATE_CHUNK_ACTION_TYPE.to_owned(),
                status: TASK_RUN_STATUS_RUNNING.to_owned(),
                input_payload: Some("{\"chunk\":2,\"mode\":\"finding_review\"}".to_owned()),
                output_payload: None,
                error_message: None,
                started_at: NOW + 80,
                completed_at: None,
                created_at: NOW + 80,
                updated_at: NOW + 80,
            })
            .expect("chunk-only correction run should persist");
        TaskRunRepository::new(&mut connection)
            .mark_completed_with_translation_projection(
                PROJECT_ID,
                DOCUMENT_ID,
                "task_observability_chunk_review_0001",
                "{\"translations\":[3,4],\"review\":\"applied\"}",
                &[
                    SegmentTranslationWrite {
                        segment_id: "seg_observability_0003".to_owned(),
                        target_text: "Capítulo II".to_owned(),
                    },
                    SegmentTranslationWrite {
                        segment_id: "seg_observability_0004".to_owned(),
                        target_text: "La linterna siguió ardiendo toda la noche.".to_owned(),
                    },
                ],
                NOW + 90,
            )
            .expect("chunk-only correction projection should persist");
        drop(connection);

        let inspection = inspect_job_trace_with_runtime(
            InspectJobTraceInput {
                project_id: PROJECT_ID.to_owned(),
                document_id: DOCUMENT_ID.to_owned(),
                job_id: correction_job_id.to_owned(),
            },
            &fixture.runtime,
        )
        .expect("chunk-only correction job trace should load");

        assert_eq!(inspection.overview.job_id, correction_job_id);
        assert!(inspection
            .warnings
            .iter()
            .any(|warning| warning.code == "chunk_only_job_trace"));
        assert!(inspection
            .warnings
            .iter()
            .all(|warning| warning.code != "missing_document_orchestration_run"));
    }

    #[test]
    fn inspect_document_operational_state_does_not_flag_export_after_chunk_only_correction() {
        let fixture = create_runtime_fixture();
        seed_observability_graph(&fixture.runtime);
        let mut connection = fixture
            .runtime
            .open_connection()
            .expect("database connection should open");

        TaskRunRepository::new(&mut connection)
            .create(&NewTaskRun {
                id: "task_observability_chunk_review_0002".to_owned(),
                document_id: DOCUMENT_ID.to_owned(),
                chunk_id: Some("doc_observability_001_chunk_0002".to_owned()),
                job_id: Some("job_observability_review_002".to_owned()),
                action_type: TRANSLATE_CHUNK_ACTION_TYPE.to_owned(),
                status: TASK_RUN_STATUS_RUNNING.to_owned(),
                input_payload: Some("{\"chunk\":2,\"mode\":\"finding_review\"}".to_owned()),
                output_payload: None,
                error_message: None,
                started_at: NOW + 80,
                completed_at: None,
                created_at: NOW + 80,
                updated_at: NOW + 80,
            })
            .expect("chunk-only correction run should persist");
        TaskRunRepository::new(&mut connection)
            .mark_completed_with_translation_projection(
                PROJECT_ID,
                DOCUMENT_ID,
                "task_observability_chunk_review_0002",
                "{\"translations\":[3,4],\"review\":\"applied\"}",
                &[
                    SegmentTranslationWrite {
                        segment_id: "seg_observability_0003".to_owned(),
                        target_text: "Capítulo II".to_owned(),
                    },
                    SegmentTranslationWrite {
                        segment_id: "seg_observability_0004".to_owned(),
                        target_text: "La linterna siguió ardiendo toda la noche.".to_owned(),
                    },
                ],
                NOW + 90,
            )
            .expect("chunk-only correction projection should persist");
        drop(connection);

        let _ = export_reconstructed_document_with_runtime(
            ExportReconstructedDocumentInput {
                project_id: PROJECT_ID.to_owned(),
                document_id: DOCUMENT_ID.to_owned(),
            },
            &fixture.runtime,
        )
        .expect("export should succeed after chunk-only correction");

        let inspection = inspect_document_operational_state_with_runtime(
            InspectDocumentOperationalStateInput {
                project_id: PROJECT_ID.to_owned(),
                document_id: DOCUMENT_ID.to_owned(),
                job_id: Some("job_observability_review_002".to_owned()),
            },
            &fixture.runtime,
        )
        .expect("document operational state should load after chunk-only correction");

        assert!(inspection
            .warnings
            .iter()
            .all(|warning| warning.code != "export_behind_latest_document_run"));
        assert_eq!(
            inspection.exports.first().and_then(|export| export.source_task_run_id.as_deref()),
            Some("task_observability_chunk_review_0002")
        );
    }

    fn create_runtime_fixture() -> RuntimeFixture {
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

    fn seed_observability_graph(runtime: &DatabaseRuntime) {
        let mut connection = runtime
            .open_connection()
            .expect("database connection should open");

        ProjectRepository::new(&mut connection)
            .create(&NewProject {
                id: PROJECT_ID.to_owned(),
                name: "Observability project".to_owned(),
                description: None,
                created_at: NOW,
                updated_at: NOW,
                last_opened_at: NOW,
            })
            .expect("project should persist");
        ProjectRepository::new(&mut connection)
            .open_project(PROJECT_ID, NOW)
            .expect("project should become active");

        DocumentRepository::new(&mut connection)
            .create(&NewDocument {
                id: DOCUMENT_ID.to_owned(),
                project_id: PROJECT_ID.to_owned(),
                name: "chaptered draft.txt".to_owned(),
                source_kind: DOCUMENT_SOURCE_LOCAL_FILE.to_owned(),
                format: "txt".to_owned(),
                mime_type: Some("text/plain".to_owned()),
                stored_path: "ignored".to_owned(),
                file_size_bytes: 1_024,
                status: DOCUMENT_STATUS_SEGMENTED.to_owned(),
                created_at: NOW,
                updated_at: NOW,
            })
            .expect("document should persist");

        SegmentRepository::new(&mut connection)
            .replace_for_document(
                PROJECT_ID,
                DOCUMENT_ID,
                &[
                    NewSegment {
                        id: "seg_observability_0001".to_owned(),
                        document_id: DOCUMENT_ID.to_owned(),
                        sequence: 1,
                        source_text: "Chapter I".to_owned(),
                        source_word_count: 2,
                        source_character_count: 9,
                        status: SEGMENT_STATUS_PENDING_TRANSLATION.to_owned(),
                        created_at: NOW,
                        updated_at: NOW,
                    },
                    NewSegment {
                        id: "seg_observability_0002".to_owned(),
                        document_id: DOCUMENT_ID.to_owned(),
                        sequence: 2,
                        source_text: "The gate remained closed.".to_owned(),
                        source_word_count: 4,
                        source_character_count: 25,
                        status: SEGMENT_STATUS_PENDING_TRANSLATION.to_owned(),
                        created_at: NOW,
                        updated_at: NOW,
                    },
                    NewSegment {
                        id: "seg_observability_0003".to_owned(),
                        document_id: DOCUMENT_ID.to_owned(),
                        sequence: 3,
                        source_text: "Chapter II".to_owned(),
                        source_word_count: 2,
                        source_character_count: 10,
                        status: SEGMENT_STATUS_PENDING_TRANSLATION.to_owned(),
                        created_at: NOW,
                        updated_at: NOW,
                    },
                    NewSegment {
                        id: "seg_observability_0004".to_owned(),
                        document_id: DOCUMENT_ID.to_owned(),
                        sequence: 4,
                        source_text: "The lantern burned all night.".to_owned(),
                        source_word_count: 5,
                        source_character_count: 29,
                        status: SEGMENT_STATUS_PENDING_TRANSLATION.to_owned(),
                        created_at: NOW,
                        updated_at: NOW,
                    },
                ],
                NOW,
            )
            .expect("segments should persist");

        DocumentSectionRepository::new(&mut connection)
            .replace_for_document(
                DOCUMENT_ID,
                &[
                    NewDocumentSection {
                        id: "doc_observability_001_sec_0001".to_owned(),
                        document_id: DOCUMENT_ID.to_owned(),
                        sequence: 1,
                        title: "Chapter I".to_owned(),
                        section_type: DOCUMENT_SECTION_TYPE_CHAPTER.to_owned(),
                        level: 1,
                        start_segment_sequence: 1,
                        end_segment_sequence: 2,
                        segment_count: 2,
                        created_at: NOW,
                        updated_at: NOW,
                    },
                    NewDocumentSection {
                        id: "doc_observability_001_sec_0002".to_owned(),
                        document_id: DOCUMENT_ID.to_owned(),
                        sequence: 2,
                        title: "Chapter II".to_owned(),
                        section_type: DOCUMENT_SECTION_TYPE_CHAPTER.to_owned(),
                        level: 1,
                        start_segment_sequence: 3,
                        end_segment_sequence: 4,
                        segment_count: 2,
                        created_at: NOW,
                        updated_at: NOW,
                    },
                ],
            )
            .expect("sections should persist");

        TranslationChunkRepository::new(&mut connection)
            .replace_for_document(
                DOCUMENT_ID,
                &[
                    NewTranslationChunk {
                        id: "doc_observability_001_chunk_0001".to_owned(),
                        document_id: DOCUMENT_ID.to_owned(),
                        sequence: 1,
                        builder_version: "tr12-basic-v1".to_owned(),
                        strategy: "section-aware-fixed-word-target-v1".to_owned(),
                        source_text: "Chapter I\n\nThe gate remained closed.".to_owned(),
                        context_before_text: None,
                        context_after_text: Some("Chapter II".to_owned()),
                        start_segment_sequence: 1,
                        end_segment_sequence: 2,
                        segment_count: 2,
                        source_word_count: 6,
                        source_character_count: 34,
                        created_at: NOW,
                        updated_at: NOW,
                    },
                    NewTranslationChunk {
                        id: "doc_observability_001_chunk_0002".to_owned(),
                        document_id: DOCUMENT_ID.to_owned(),
                        sequence: 2,
                        builder_version: "tr12-basic-v1".to_owned(),
                        strategy: "section-aware-fixed-word-target-v1".to_owned(),
                        source_text: "Chapter II\n\nThe lantern burned all night.".to_owned(),
                        context_before_text: None,
                        context_after_text: None,
                        start_segment_sequence: 3,
                        end_segment_sequence: 4,
                        segment_count: 2,
                        source_word_count: 7,
                        source_character_count: 39,
                        created_at: NOW,
                        updated_at: NOW,
                    },
                ],
                &[
                    NewTranslationChunkSegment {
                        chunk_id: "doc_observability_001_chunk_0001".to_owned(),
                        segment_id: "seg_observability_0001".to_owned(),
                        segment_sequence: 1,
                        position: 1,
                        role: TRANSLATION_CHUNK_SEGMENT_ROLE_CORE.to_owned(),
                    },
                    NewTranslationChunkSegment {
                        chunk_id: "doc_observability_001_chunk_0001".to_owned(),
                        segment_id: "seg_observability_0002".to_owned(),
                        segment_sequence: 2,
                        position: 2,
                        role: TRANSLATION_CHUNK_SEGMENT_ROLE_CORE.to_owned(),
                    },
                    NewTranslationChunkSegment {
                        chunk_id: "doc_observability_001_chunk_0001".to_owned(),
                        segment_id: "seg_observability_0003".to_owned(),
                        segment_sequence: 3,
                        position: 1,
                        role: TRANSLATION_CHUNK_SEGMENT_ROLE_CONTEXT_AFTER.to_owned(),
                    },
                    NewTranslationChunkSegment {
                        chunk_id: "doc_observability_001_chunk_0002".to_owned(),
                        segment_id: "seg_observability_0003".to_owned(),
                        segment_sequence: 3,
                        position: 1,
                        role: TRANSLATION_CHUNK_SEGMENT_ROLE_CORE.to_owned(),
                    },
                    NewTranslationChunkSegment {
                        chunk_id: "doc_observability_001_chunk_0002".to_owned(),
                        segment_id: "seg_observability_0004".to_owned(),
                        segment_sequence: 4,
                        position: 2,
                        role: TRANSLATION_CHUNK_SEGMENT_ROLE_CORE.to_owned(),
                    },
                ],
            )
            .expect("chunks should persist");

        TaskRunRepository::new(&mut connection)
            .create(&NewTaskRun {
                id: "task_observability_doc_0001".to_owned(),
                document_id: DOCUMENT_ID.to_owned(),
                chunk_id: None,
                job_id: Some(JOB_ID.to_owned()),
                action_type: TRANSLATE_DOCUMENT_ACTION_TYPE.to_owned(),
                status: TASK_RUN_STATUS_COMPLETED.to_owned(),
                input_payload: Some("{\"job\":\"translate\"}".to_owned()),
                output_payload: Some("{\"status\":\"completed\"}".to_owned()),
                error_message: None,
                started_at: NOW,
                completed_at: Some(NOW + 60),
                created_at: NOW,
                updated_at: NOW + 60,
            })
            .expect("document run should persist");
        TaskRunRepository::new(&mut connection)
            .create(&NewTaskRun {
                id: "task_observability_chunk_0001".to_owned(),
                document_id: DOCUMENT_ID.to_owned(),
                chunk_id: Some("doc_observability_001_chunk_0001".to_owned()),
                job_id: Some(JOB_ID.to_owned()),
                action_type: TRANSLATE_CHUNK_ACTION_TYPE.to_owned(),
                status: TASK_RUN_STATUS_RUNNING.to_owned(),
                input_payload: Some("{\"chunk\":1}".to_owned()),
                output_payload: None,
                error_message: None,
                started_at: NOW + 1,
                completed_at: None,
                created_at: NOW + 1,
                updated_at: NOW + 1,
            })
            .expect("first chunk run should persist");
        TaskRunRepository::new(&mut connection)
            .mark_completed_with_translation_projection(
                PROJECT_ID,
                DOCUMENT_ID,
                "task_observability_chunk_0001",
                "{\"translations\":[1,2]}",
                &[
                    SegmentTranslationWrite {
                        segment_id: "seg_observability_0001".to_owned(),
                        target_text: "Capítulo I".to_owned(),
                    },
                    SegmentTranslationWrite {
                        segment_id: "seg_observability_0002".to_owned(),
                        target_text: "La puerta siguió cerrada.".to_owned(),
                    },
                ],
                NOW + 30,
            )
            .expect("first chunk translation should persist");

        QaFindingRepository::new(&mut connection)
            .upsert(&NewQaFinding {
                id: "qaf_observability_0001".to_owned(),
                document_id: DOCUMENT_ID.to_owned(),
                chunk_id: Some("doc_observability_001_chunk_0002".to_owned()),
                task_run_id: Some("task_observability_doc_0001".to_owned()),
                job_id: Some(JOB_ID.to_owned()),
                finding_type: "consistency".to_owned(),
                severity: QA_FINDING_SEVERITY_HIGH.to_owned(),
                status: QA_FINDING_STATUS_OPEN.to_owned(),
                message: "The second chapter still needs review.".to_owned(),
                details: Some("The reconstructed document still falls back to the source text.".to_owned()),
                created_at: NOW + 61,
                updated_at: NOW + 61,
            })
            .expect("qa finding should persist");
    }
}
