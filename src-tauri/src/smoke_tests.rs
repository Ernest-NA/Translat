#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::document_qa::DOCUMENT_QA_FINDING_TYPE_SOURCE_FALLBACK_SEGMENT;
    use crate::reconstructed_documents::{
        RECONSTRUCTED_DOCUMENT_STATUS_COMPLETE, RECONSTRUCTED_DOCUMENT_STATUS_PARTIAL,
    };
    use crate::test_support::{
        build_chunk_response, core_segment_ids_for_chunk, create_runtime_fixture,
        translate_chunk_failure, DocumentPipelineFixture, ScriptedChunkOutcome,
        ScriptedTranslateChunkExecutor, SAMPLE_CHAPTERED_DOCUMENT_TEXT,
    };
    use crate::translate_document::{
        TRANSLATE_DOCUMENT_STATUS_COMPLETED, TRANSLATE_DOCUMENT_STATUS_COMPLETED_WITH_ERRORS,
    };

    #[test]
    fn document_pipeline_smoke_runs_from_import_to_reviewed_export() {
        let fixture = create_runtime_fixture();
        let mut pipeline = DocumentPipelineFixture::import_document(
            &fixture.runtime,
            "TR-26 Smoke Project",
            "smoke-document.txt",
            SAMPLE_CHAPTERED_DOCUMENT_TEXT,
        )
        .expect("document fixture should import");

        let processed_document = pipeline
            .process_document(&fixture.runtime)
            .expect("document should process");
        assert_eq!(processed_document.status, "segmented");

        let segment_overview = pipeline
            .list_segments(&fixture.runtime)
            .expect("segments should list");
        let chunk_overview = pipeline
            .build_chunks(&fixture.runtime)
            .expect("chunks should build");

        assert_eq!(chunk_overview.chunks.len(), 2);

        let first_chunk_id = chunk_overview.chunks[0].id.clone();
        let second_chunk_id = chunk_overview.chunks[1].id.clone();
        let first_chunk_core_segment_ids =
            core_segment_ids_for_chunk(&chunk_overview, &first_chunk_id);
        let second_chunk_core_segment_ids =
            core_segment_ids_for_chunk(&chunk_overview, &second_chunk_id);
        let initial_job_id = "job_smoke_translate_001".to_owned();
        let initial_executor = ScriptedTranslateChunkExecutor::new(HashMap::from([
            (
                first_chunk_id.clone(),
                ScriptedChunkOutcome::Success(build_chunk_response(
                    &segment_overview.segments,
                    &first_chunk_core_segment_ids,
                    &HashMap::from([
                        ("Chapter I", "Capitulo I"),
                        ("The gate remained closed.", "La puerta permanecio cerrada."),
                        ("The lantern burned all night.", "La linterna ardio toda la noche."),
                    ]),
                    Some("initial smoke pass"),
                )),
            ),
            (
                second_chunk_id.clone(),
                ScriptedChunkOutcome::Failure(translate_chunk_failure(
                    "The smoke fixture simulates a downstream failure on chapter two.",
                )),
            ),
        ]));

        let initial_result = pipeline
            .translate_document(
                &fixture.runtime,
                Some(initial_job_id.clone()),
                &initial_executor,
            )
            .expect("initial translation job should complete with errors");
        assert_eq!(
            initial_executor.observed_chunk_ids(),
            vec![first_chunk_id.clone(), second_chunk_id.clone()]
        );
        assert_eq!(
            initial_result.status,
            TRANSLATE_DOCUMENT_STATUS_COMPLETED_WITH_ERRORS
        );

        let initial_job_status = pipeline
            .job_status(&fixture.runtime, &initial_job_id)
            .expect("initial job status should load");
        assert_eq!(
            initial_job_status.status,
            TRANSLATE_DOCUMENT_STATUS_COMPLETED_WITH_ERRORS
        );
        assert_eq!(initial_job_status.completed_chunks, 1);
        assert_eq!(initial_job_status.failed_chunks, 1);

        let partial_reconstruction = pipeline
            .reconstruct(&fixture.runtime)
            .expect("partial reconstruction should load");
        assert_eq!(
            partial_reconstruction.status,
            RECONSTRUCTED_DOCUMENT_STATUS_PARTIAL
        );
        assert!(partial_reconstruction
            .resolved_text
            .contains("Guard the archive."));

        let initial_qa = pipeline
            .run_qa(&fixture.runtime, Some(initial_job_id.clone()))
            .expect("QA should run on the partial translation");
        assert!(!initial_qa.generated_findings.is_empty());
        assert!(initial_qa.generated_findings.iter().any(|finding| {
            finding.finding_type == DOCUMENT_QA_FINDING_TYPE_SOURCE_FALLBACK_SEGMENT
        }));

        let review_context = initial_qa
            .generated_findings
            .iter()
            .find_map(|finding| {
                let context = pipeline
                    .inspect_finding(&fixture.runtime, &finding.id)
                    .expect("finding review context should load");

                context.anchor.can_retranslate.then_some(context)
            })
            .expect("at least one generated finding should resolve to a retranslation target");
        let review_job_id = "job_smoke_review_001".to_owned();
        let review_executor = ScriptedTranslateChunkExecutor::new(HashMap::from([(
            review_context
                .anchor
                .chunk_id
                .clone()
                .expect("review anchor should resolve a chunk id"),
            ScriptedChunkOutcome::Success(build_chunk_response(
                &segment_overview.segments,
                &second_chunk_core_segment_ids,
                &HashMap::from([
                    ("Chapter II", "Capitulo II"),
                    ("Guard the archive.", "Proteged el archivo."),
                    ("Keep the signal hidden.", "Mantened oculta la senal."),
                ]),
                Some("review correction"),
            )),
        )]));

        let review_result = pipeline
            .retranslate_finding(
                &fixture.runtime,
                &review_context.finding.id,
                Some(review_job_id.clone()),
                &review_executor,
            )
            .expect("finding-driven retranslation should succeed");
        assert_eq!(
            review_executor.observed_chunk_ids(),
            vec![second_chunk_id.clone()]
        );
        assert_eq!(review_result.correction_job_id, review_job_id);

        let corrected_reconstruction = pipeline
            .reconstruct(&fixture.runtime)
            .expect("corrected reconstruction should load");
        assert_eq!(
            corrected_reconstruction.status,
            RECONSTRUCTED_DOCUMENT_STATUS_COMPLETE
        );
        assert!(corrected_reconstruction
            .resolved_text
            .contains("Proteged el archivo."));
        assert!(corrected_reconstruction
            .resolved_text
            .contains("Mantened oculta la senal."));

        let post_review_qa = pipeline
            .run_qa(&fixture.runtime, Some(review_job_id.clone()))
            .expect("QA should run on the correction job");
        assert!(post_review_qa.generated_findings.is_empty());

        let export_result = pipeline
            .export_document(&fixture.runtime)
            .expect("complete reconstructed document should export");
        assert!(export_result.is_complete);
        assert!(export_result.content.contains("Capitulo II"));
        assert!(export_result.content.contains("Proteged el archivo."));

        let findings_overview = pipeline
            .list_qa_findings(&fixture.runtime, Some(initial_job_id.clone()))
            .expect("persisted QA findings should remain queryable");
        assert!(!findings_overview.findings.is_empty());

        let operational_state = pipeline
            .inspect_operational_state(&fixture.runtime, Some(review_job_id.clone()))
            .expect("operational inspection should load");
        assert_eq!(
            operational_state.selected_job_id.as_deref(),
            Some(review_job_id.as_str())
        );
        assert!(operational_state.jobs.iter().any(|job| {
            job.job_id == initial_job_id
                && job.status == TRANSLATE_DOCUMENT_STATUS_COMPLETED_WITH_ERRORS
        }));
        assert!(operational_state.jobs.iter().any(|job| {
            job.job_id == review_job_id && job.status == TRANSLATE_DOCUMENT_STATUS_COMPLETED
        }));
        assert_eq!(operational_state.exports.len(), 1);
        assert_eq!(
            operational_state.exports[0].source_job_id.as_deref(),
            Some(review_job_id.as_str())
        );
        assert_eq!(
            operational_state.exports[0].source_task_run_id.as_deref(),
            Some(review_result.translate_result.task_run.id.as_str())
        );
    }
}
