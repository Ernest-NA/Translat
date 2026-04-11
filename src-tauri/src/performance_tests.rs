#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::time::Instant;

    use serde_json::json;

    use crate::commands::observability::collect_job_overviews;
    use crate::commands::reconstructed_documents::{
        build_reconstructed_document, load_reconstructed_document,
    };
    use crate::commands::segments::load_segmented_document_overview;
    use crate::commands::translate_document_jobs::{
        build_job_status_from_task_runs_and_chunks, build_job_status_if_exists, current_timestamp,
    };
    use crate::persistence::bootstrap::DatabaseRuntime;
    use crate::persistence::task_runs::TaskRunRepository;
    use crate::persistence::translation_chunks::TranslationChunkRepository;
    use crate::task_runs::{NewTaskRun, TASK_RUN_STATUS_COMPLETED};
    use crate::test_support::{
        core_segment_ids_for_chunk, create_runtime_fixture, generate_performance_document_text,
        DocumentPipelineFixture, RuntimeFixture,
    };
    use crate::translate_chunk::TRANSLATE_CHUNK_ACTION_TYPE;
    use crate::translate_document::TRANSLATE_DOCUMENT_ACTION_TYPE;

    const PERF_CHAPTER_COUNT: usize = 12;
    const PERF_PARAGRAPHS_PER_CHAPTER: usize = 24;
    const PERF_JOB_COUNT: usize = 12;
    const PERF_MEASUREMENT_ITERATIONS: usize = 10;

    struct PerformanceScenario {
        fixture: RuntimeFixture,
        document_id: String,
        job_id: String,
        project_id: String,
    }

    #[test]
    #[ignore = "manual local benchmark for TR-27"]
    fn benchmark_job_status_against_legacy_chunk_loading() {
        let scenario = seed_performance_scenario();

        let legacy_status = legacy_build_job_status(
            &scenario.fixture.runtime,
            &scenario.project_id,
            &scenario.document_id,
            &scenario.job_id,
        );
        let optimized_status = optimized_build_job_status(
            &scenario.fixture.runtime,
            &scenario.project_id,
            &scenario.document_id,
            &scenario.job_id,
        );

        assert_eq!(legacy_status.status, optimized_status.status);
        assert_eq!(legacy_status.total_chunks, optimized_status.total_chunks);
        assert_eq!(
            legacy_status.completed_chunks,
            optimized_status.completed_chunks
        );

        let legacy_elapsed = measure_case(PERF_MEASUREMENT_ITERATIONS, || {
            let _ = legacy_build_job_status(
                &scenario.fixture.runtime,
                &scenario.project_id,
                &scenario.document_id,
                &scenario.job_id,
            );
        });
        let optimized_elapsed = measure_case(PERF_MEASUREMENT_ITERATIONS, || {
            let _ = optimized_build_job_status(
                &scenario.fixture.runtime,
                &scenario.project_id,
                &scenario.document_id,
                &scenario.job_id,
            );
        });

        print_measurement(
            "job_status",
            PERF_MEASUREMENT_ITERATIONS,
            legacy_elapsed,
            optimized_elapsed,
        );
    }

    #[test]
    #[ignore = "manual local benchmark for TR-27"]
    fn benchmark_job_overview_collection_against_legacy_per_job_rebuilds() {
        let scenario = seed_performance_scenario();

        let legacy_job_count = legacy_collect_job_overviews(
            &scenario.fixture.runtime,
            &scenario.project_id,
            &scenario.document_id,
        );
        let optimized_job_count = optimized_collect_job_overviews(
            &scenario.fixture.runtime,
            &scenario.project_id,
            &scenario.document_id,
        );

        assert_eq!(legacy_job_count, optimized_job_count);

        let legacy_elapsed = measure_case(PERF_MEASUREMENT_ITERATIONS, || {
            let _ = legacy_collect_job_overviews(
                &scenario.fixture.runtime,
                &scenario.project_id,
                &scenario.document_id,
            );
        });
        let optimized_elapsed = measure_case(PERF_MEASUREMENT_ITERATIONS, || {
            let _ = optimized_collect_job_overviews(
                &scenario.fixture.runtime,
                &scenario.project_id,
                &scenario.document_id,
            );
        });

        print_measurement(
            "observability_job_overviews",
            PERF_MEASUREMENT_ITERATIONS,
            legacy_elapsed,
            optimized_elapsed,
        );
    }

    #[test]
    #[ignore = "manual local benchmark for TR-27"]
    fn benchmark_reconstruction_trace_loading_against_full_task_run_payloads() {
        let scenario = seed_performance_scenario();

        let reconstructed_legacy = legacy_reconstruct_document(
            &scenario.fixture.runtime,
            &scenario.project_id,
            &scenario.document_id,
        );
        let reconstructed_optimized = optimized_reconstruct_document(
            &scenario.fixture.runtime,
            &scenario.project_id,
            &scenario.document_id,
        );

        assert_eq!(reconstructed_legacy.status, reconstructed_optimized.status);
        assert_eq!(
            reconstructed_legacy.completeness.total_segments,
            reconstructed_optimized.completeness.total_segments
        );
        assert_eq!(
            reconstructed_legacy.trace.task_run_count,
            reconstructed_optimized.trace.task_run_count
        );

        let legacy_elapsed = measure_case(PERF_MEASUREMENT_ITERATIONS, || {
            let _ = legacy_reconstruct_document(
                &scenario.fixture.runtime,
                &scenario.project_id,
                &scenario.document_id,
            );
        });
        let optimized_elapsed = measure_case(PERF_MEASUREMENT_ITERATIONS, || {
            let _ = optimized_reconstruct_document(
                &scenario.fixture.runtime,
                &scenario.project_id,
                &scenario.document_id,
            );
        });

        print_measurement(
            "reconstruction_trace_loading",
            PERF_MEASUREMENT_ITERATIONS,
            legacy_elapsed,
            optimized_elapsed,
        );
    }

    fn seed_performance_scenario() -> PerformanceScenario {
        let fixture = create_runtime_fixture();
        let source_text =
            generate_performance_document_text(PERF_CHAPTER_COUNT, PERF_PARAGRAPHS_PER_CHAPTER);
        let mut pipeline = DocumentPipelineFixture::import_document(
            &fixture.runtime,
            "TR-27 Performance Project",
            "performance-document.txt",
            &source_text,
        )
        .expect("performance document should import");

        pipeline
            .process_document(&fixture.runtime)
            .expect("performance document should process");
        let chunk_overview = pipeline
            .build_chunks(&fixture.runtime)
            .expect("performance chunks should build");
        let chunk_ids = chunk_overview
            .chunks
            .iter()
            .map(|chunk| chunk.id.clone())
            .collect::<Vec<_>>();
        let mut connection = fixture
            .runtime
            .open_connection()
            .expect("database connection should open");
        let mut repository = TaskRunRepository::new(&mut connection);
        let mut timestamp = 1_744_000_000_i64;

        for job_index in 0..PERF_JOB_COUNT {
            let job_id = format!("job_perf_{job_index:03}");
            let document_task_run_id = format!("trun_doc_perf_{job_index:03}");
            let document_input_payload = serde_json::to_string(&json!({
                "projectId": pipeline.project.id,
                "documentId": pipeline.document.id,
                "jobId": job_id,
                "mode": "fresh",
                "chunkIds": chunk_ids,
                "notes": "performance benchmark document orchestration payload"
            }))
            .expect("document task-run payload should serialize");

            repository
                .create(&NewTaskRun {
                    id: document_task_run_id,
                    document_id: pipeline.document.id.clone(),
                    chunk_id: None,
                    job_id: Some(job_id.clone()),
                    action_type: TRANSLATE_DOCUMENT_ACTION_TYPE.to_owned(),
                    status: TASK_RUN_STATUS_COMPLETED.to_owned(),
                    input_payload: Some(document_input_payload),
                    output_payload: Some(
                        "{\"status\":\"completed\",\"note\":\"performance benchmark\"}".to_owned(),
                    ),
                    error_message: None,
                    started_at: timestamp,
                    completed_at: Some(timestamp + 1),
                    created_at: timestamp,
                    updated_at: timestamp + 1,
                })
                .expect("document task run should persist");
            timestamp += 2;

            for chunk in &chunk_overview.chunks {
                let chunk_task_run_id =
                    format!("trun_chunk_perf_{job_index:03}_{:04}", chunk.sequence);
                let chunk_input_payload = serde_json::to_string(&json!({
                    "projectId": pipeline.project.id,
                    "documentId": pipeline.document.id,
                    "jobId": job_id,
                    "chunkId": chunk.id,
                    "chunkSequence": chunk.sequence,
                    "actionVersion": "benchmark"
                }))
                .expect("chunk task-run payload should serialize");
                let core_segment_ids = core_segment_ids_for_chunk(&chunk_overview, &chunk.id);
                let output_payload = serde_json::to_string(&json!({
                    "translations": core_segment_ids.iter().map(|segment_id| {
                        json!({
                            "segmentId": segment_id,
                            "targetText": format!("translated {segment_id}")
                        })
                    }).collect::<Vec<_>>(),
                    "notes": "performance benchmark translated payload with repeated metadata for trace loading"
                }))
                .expect("chunk output payload should serialize");

                repository
                    .create(&NewTaskRun {
                        id: chunk_task_run_id,
                        document_id: pipeline.document.id.clone(),
                        chunk_id: Some(chunk.id.clone()),
                        job_id: Some(job_id.clone()),
                        action_type: TRANSLATE_CHUNK_ACTION_TYPE.to_owned(),
                        status: TASK_RUN_STATUS_COMPLETED.to_owned(),
                        input_payload: Some(chunk_input_payload),
                        output_payload: Some(output_payload),
                        error_message: None,
                        started_at: timestamp,
                        completed_at: Some(timestamp + 1),
                        created_at: timestamp,
                        updated_at: timestamp + 1,
                    })
                    .expect("chunk task run should persist");
                timestamp += 2;
            }
        }

        PerformanceScenario {
            fixture,
            project_id: pipeline.project.id,
            document_id: pipeline.document.id,
            job_id: format!("job_perf_{:03}", PERF_JOB_COUNT - 1),
        }
    }

    fn legacy_build_job_status(
        database_runtime: &DatabaseRuntime,
        project_id: &str,
        document_id: &str,
        job_id: &str,
    ) -> crate::translate_document::TranslateDocumentJobStatus {
        let observed_at = current_timestamp().expect("timestamp should load");
        let mut connection = database_runtime
            .open_connection()
            .expect("database connection should open");
        let task_runs = TaskRunRepository::new(&mut connection)
            .list_by_job_id(job_id)
            .expect("job task runs should load")
            .into_iter()
            .filter(|task_run| task_run.document_id == document_id)
            .collect::<Vec<_>>();
        let segment_overview = load_segmented_document_overview(
            &mut connection,
            database_runtime,
            project_id,
            document_id,
            false,
            observed_at,
        )
        .expect("legacy segment overview should load");
        let current_chunks = TranslationChunkRepository::new(&mut connection)
            .list_chunks_by_document(document_id)
            .expect("legacy chunks should load");
        let _ = segment_overview;

        build_job_status_from_task_runs_and_chunks(
            project_id,
            document_id,
            job_id,
            task_runs,
            &current_chunks,
            observed_at,
        )
        .expect("legacy job status should build")
    }

    fn optimized_build_job_status(
        database_runtime: &DatabaseRuntime,
        project_id: &str,
        document_id: &str,
        job_id: &str,
    ) -> crate::translate_document::TranslateDocumentJobStatus {
        let mut connection = database_runtime
            .open_connection()
            .expect("database connection should open");

        build_job_status_if_exists(&mut connection, project_id, document_id, job_id)
            .expect("optimized job status should load")
            .expect("job status should exist")
    }

    fn legacy_collect_job_overviews(
        database_runtime: &DatabaseRuntime,
        project_id: &str,
        document_id: &str,
    ) -> usize {
        let mut connection = database_runtime
            .open_connection()
            .expect("database connection should open");
        let task_runs = TaskRunRepository::new(&mut connection)
            .list_by_document(document_id)
            .expect("task runs should load");
        let job_ids = task_runs
            .iter()
            .filter_map(|task_run| task_run.job_id.clone())
            .collect::<BTreeSet<_>>();

        job_ids
            .into_iter()
            .filter(|job_id| {
                build_job_status_if_exists(&mut connection, project_id, document_id, job_id)
                    .expect("legacy job overview status should load")
                    .is_some()
            })
            .count()
    }

    fn optimized_collect_job_overviews(
        database_runtime: &DatabaseRuntime,
        project_id: &str,
        document_id: &str,
    ) -> usize {
        let mut connection = database_runtime
            .open_connection()
            .expect("database connection should open");
        let task_runs = TaskRunRepository::new(&mut connection)
            .list_by_document(document_id)
            .expect("task runs should load");

        collect_job_overviews(&mut connection, project_id, document_id, &[], &task_runs)
            .expect("optimized job overviews should load")
            .len()
    }

    fn legacy_reconstruct_document(
        database_runtime: &DatabaseRuntime,
        project_id: &str,
        document_id: &str,
    ) -> crate::reconstructed_documents::ReconstructedDocument {
        let reconstructed_at = current_timestamp().expect("timestamp should load");
        let mut connection = database_runtime
            .open_connection()
            .expect("database connection should open");
        let segment_overview = load_segmented_document_overview(
            &mut connection,
            database_runtime,
            project_id,
            document_id,
            false,
            reconstructed_at,
        )
        .expect("legacy segment overview should load");
        let mut chunk_repository = TranslationChunkRepository::new(&mut connection);
        let chunks = chunk_repository
            .list_chunks_by_document(document_id)
            .expect("legacy reconstruction chunks should load");
        let chunk_segments = chunk_repository
            .list_chunk_segments_by_document(document_id)
            .expect("legacy reconstruction chunk segments should load");
        let task_runs = TaskRunRepository::new(&mut connection)
            .list_by_document(document_id)
            .expect("legacy reconstruction task runs should load");

        build_reconstructed_document(
            project_id,
            document_id,
            &segment_overview.sections,
            &segment_overview.segments,
            &chunks,
            &chunk_segments,
            &task_runs,
        )
    }

    fn optimized_reconstruct_document(
        database_runtime: &DatabaseRuntime,
        project_id: &str,
        document_id: &str,
    ) -> crate::reconstructed_documents::ReconstructedDocument {
        let reconstructed_at = current_timestamp().expect("timestamp should load");
        let mut connection = database_runtime
            .open_connection()
            .expect("database connection should open");

        load_reconstructed_document(
            &mut connection,
            database_runtime,
            project_id,
            document_id,
            reconstructed_at,
        )
        .expect("optimized reconstruction should load")
    }

    fn measure_case(iterations: usize, mut run: impl FnMut()) -> std::time::Duration {
        let started_at = Instant::now();

        for _ in 0..iterations {
            run();
        }

        started_at.elapsed()
    }

    fn print_measurement(
        label: &str,
        iterations: usize,
        legacy_elapsed: std::time::Duration,
        optimized_elapsed: std::time::Duration,
    ) {
        let legacy_average_ms = legacy_elapsed.as_secs_f64() * 1_000.0 / iterations as f64;
        let optimized_average_ms = optimized_elapsed.as_secs_f64() * 1_000.0 / iterations as f64;
        let improvement = if legacy_elapsed.as_nanos() > 0 {
            100.0 * (legacy_elapsed.as_secs_f64() - optimized_elapsed.as_secs_f64())
                / legacy_elapsed.as_secs_f64()
        } else {
            0.0
        };

        println!(
            "[perf] {label}: legacy avg {:.2} ms, optimized avg {:.2} ms, delta {improvement:.1}%",
            legacy_average_ms, optimized_average_ms
        );
    }
}
