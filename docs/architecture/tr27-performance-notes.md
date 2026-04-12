# TR-27 Performance Notes

## Scope

TR-27 optimizes a small set of measured hotspots in the document pipeline without changing the product model, traceability, or job semantics.

Figma not required.
Reason: the only frontend adjustment is polling cadence and overlap control in an existing hook, not structural UI work.

## Measured hotspots

Measurements were captured with manual ignored Rust tests in `src-tauri/src/performance_tests.rs` over a synthetic multi-chapter document and repeated job history.

Benchmark labels and averages:

- `job_status`
  - legacy: `266.69 ms`
  - optimized: `167.36 ms`
  - improvement: `37.2%`
- `observability_job_overviews`
  - legacy: `304.97 ms`
  - optimized: `198.63 ms`
  - improvement: `34.9%`
- `reconstruction_trace_loading`
  - legacy: `270.94 ms`
  - optimized: `180.13 ms`
  - improvement: `33.5%`

## Changes applied

### Job and run aggregation

- Added a document-and-job scoped `task_runs` query to avoid loading every run for a `job_id` and filtering in Rust.
- Reworked translate-document status aggregation so current chunks are loaded from the persisted document record without rebuilding full segment overviews on every status refresh.
- Reused the same chunk snapshot across observability job overviews instead of rebuilding status independently per `job_id`.

### Reconstruction trace loading

- Added a trace-only document loader for `task_runs` so reconstruction does not pull full input/output payloads when it only needs provenance metadata.
- Removed a repeated linear dedupe in block primary chunk resolution.
- Avoided repeated block lookups when mapping reconstructed sections.

### Workspace refresh

- Reduced active job polling from `4s` fixed cadence to `6s` while visible and `30s` while hidden.
- Prevented overlapping job-status refreshes so polling cannot pile up concurrent requests for the same tracked job.
- Added a focused refresh when the tab becomes visible again during an active document job.

## Deliberately not optimized

- QA execution and export generation were inspected, but their current cost is dominated by reconstruction/state access that is already improved by the trace-loading change.
- No cross-request cache layer was added because the measured wins came from narrower queries and less repeated work.
- No persistence schema rewrite or architectural refactor was introduced.
