import type {
  DocumentSummary,
  SegmentSummary,
  TranslateDocumentChunkResult,
  TranslationChunkSegmentSummary,
  TranslationChunkSummary,
  TranslationContextPreview,
} from "../../shared/desktop";
import type { DesktopCommandError } from "../lib/desktop";
import { ActionButton } from "./ui/ActionButton";
import { PanelHeader } from "./ui/PanelHeader";
import { PanelMessage } from "./ui/PanelMessage";
import { StatusBadge } from "./ui/StatusBadge";

interface ChunkBrowserProps {
  activeDocument: DocumentSummary | null;
  chunkSegments: TranslationChunkSegmentSummary[];
  chunkStatuses?: TranslateDocumentChunkResult[];
  contextError?: DesktopCommandError | null;
  contextPreview?: TranslationContextPreview | null;
  disableBuild?: boolean;
  chunks: TranslationChunkSummary[];
  error: DesktopCommandError | null;
  isBuilding: boolean;
  isLoadingContext?: boolean;
  isLoading: boolean;
  onBuildChunks: () => Promise<void>;
  onSelectChunk: (chunkId: string | null) => void;
  segments: SegmentSummary[];
  selectedChunk: TranslationChunkSummary | null;
  selectedChunkId: string | null;
  selectedChunkSegments: TranslationChunkSegmentSummary[];
}

function formatChunkRole(role: TranslationChunkSegmentSummary["role"]) {
  if (role === "context_before") {
    return "Context before";
  }

  if (role === "context_after") {
    return "Context after";
  }

  return "Core";
}

function truncateText(value: string) {
  return value.length > 112 ? `${value.slice(0, 109)}...` : value;
}

function formatChunkExecutionStatus(
  status: TranslateDocumentChunkResult["status"],
) {
  switch (status) {
    case "cancelled":
      return "Cancelled";
    case "completed":
      return "Completed";
    case "failed":
      return "Error";
    case "running":
      return "In progress";
    default:
      return "Pending";
  }
}

function getChunkStatusTone(status: TranslateDocumentChunkResult["status"]) {
  switch (status) {
    case "completed":
      return "success";
    case "running":
      return "info";
    case "cancelled":
      return "warning";
    case "failed":
      return "danger";
    default:
      return "neutral";
  }
}

export function ChunkBrowser({
  activeDocument,
  chunkSegments,
  chunkStatuses = [],
  contextError = null,
  contextPreview = null,
  disableBuild = false,
  chunks,
  error,
  isBuilding,
  isLoadingContext = false,
  isLoading,
  onBuildChunks,
  onSelectChunk,
  segments,
  selectedChunk,
  selectedChunkId,
  selectedChunkSegments,
}: ChunkBrowserProps) {
  const segmentLookup = new Map(
    segments.map((segment) => [segment.id, segment]),
  );
  const chunkStatusLookup = new Map(
    chunkStatuses.map((chunkStatus) => [chunkStatus.chunkId, chunkStatus]),
  );
  const selectedCoreSegments = selectedChunkSegments
    .filter((chunkSegment) => chunkSegment.role === "core")
    .map((chunkSegment) => segmentLookup.get(chunkSegment.segmentId) ?? null)
    .filter((segment): segment is SegmentSummary => segment !== null);
  const selectedChunkStatus = selectedChunk
    ? (chunkStatusLookup.get(selectedChunk.id) ?? null)
    : null;

  return (
    <section className="workspace-panel">
      <PanelHeader
        actions={
          <div className="chunk-browser__actions">
            <StatusBadge size="md" tone="info">
              {activeDocument
                ? `${chunks.length} persisted chunks`
                : "No document"}
            </StatusBadge>
            <ActionButton
              disabled={
                !activeDocument || disableBuild || isBuilding || isLoading
              }
              onClick={() => void onBuildChunks()}
              variant={chunks.length > 0 ? "ghost" : "secondary"}
            >
              {isBuilding
                ? "Building..."
                : chunks.length > 0
                  ? "Rebuild chunks"
                  : "Build chunks"}
            </ActionButton>
          </div>
        }
        eyebrow="Translation chunks"
        title={
          activeDocument
            ? `Chunked view for ${activeDocument.name}`
            : "Open a segmented document"
        }
      />

      {!activeDocument ? (
        <PanelMessage>
          Open a segmented document first. This view will then show persisted
          translation chunks, their core ranges, and the attached overlap
          segments.
        </PanelMessage>
      ) : null}

      {activeDocument && isLoading ? (
        <PanelMessage tone="info">
          Loading persisted translation chunks for the selected document...
        </PanelMessage>
      ) : null}

      {error ? (
        <PanelMessage role="alert" tone="danger">
          {error.message}
        </PanelMessage>
      ) : null}

      {activeDocument && !isLoading && !error && chunks.length === 0 ? (
        <PanelMessage>
          No persisted translation chunks exist for this document yet. Build
          them to inspect reproducible core ranges and adjacent context overlap.
        </PanelMessage>
      ) : null}

      {activeDocument && !isLoading && !error && chunks.length > 0 ? (
        <div className="chunk-browser">
          <div className="chunk-browser__list">
            <ol className="chunk-list">
              {chunks.map((chunk) => (
                <li key={chunk.id}>
                  <button
                    className="chunk-list__item"
                    data-active={chunk.id === selectedChunkId}
                    onClick={() => onSelectChunk(chunk.id)}
                    type="button"
                  >
                    <div className="chunk-list__heading">
                      <strong>Chunk #{chunk.sequence}</strong>
                      <StatusBadge tone="info">
                        #{chunk.startSegmentSequence}-#
                        {chunk.endSegmentSequence}
                      </StatusBadge>
                    </div>
                    <div className="chunk-list__badges">
                      <StatusBadge
                        tone={getChunkStatusTone(
                          chunkStatusLookup.get(chunk.id)?.status ?? "pending",
                        )}
                      >
                        {formatChunkExecutionStatus(
                          chunkStatusLookup.get(chunk.id)?.status ?? "pending",
                        )}
                      </StatusBadge>
                      <StatusBadge tone="info">
                        {chunkStatusLookup.get(chunk.id)
                          ?.translatedSegmentCount ?? 0}
                        /{chunk.segmentCount} translated
                      </StatusBadge>
                    </div>
                    <p>{truncateText(chunk.sourceText)}</p>
                    <span className="chunk-list__meta">
                      {chunk.segmentCount} core segments |{" "}
                      {chunk.sourceWordCount} words
                    </span>
                    {chunkStatusLookup.get(chunk.id)?.errorMessage ? (
                      <span className="chunk-list__incident">
                        {chunkStatusLookup.get(chunk.id)?.errorMessage}
                      </span>
                    ) : null}
                  </button>
                </li>
              ))}
            </ol>

            <PanelMessage tone="info">
              {chunkSegments.length} persisted chunk-to-segment links are loaded
              for this document.
            </PanelMessage>
          </div>

          <div className="chunk-browser__detail">
            {selectedChunk ? (
              <>
                <PanelHeader
                  eyebrow="Selected chunk"
                  meta={
                    <div className="chunk-detail__heading-badges">
                      <StatusBadge
                        tone={getChunkStatusTone(
                          selectedChunkStatus?.status ?? "pending",
                        )}
                      >
                        {formatChunkExecutionStatus(
                          selectedChunkStatus?.status ?? "pending",
                        )}
                      </StatusBadge>
                      <StatusBadge tone="info">
                        #{selectedChunk.startSegmentSequence}-#
                        {selectedChunk.endSegmentSequence}
                      </StatusBadge>
                    </div>
                  }
                  title={`Chunk #${selectedChunk.sequence}`}
                />

                <dl className="detail-list detail-list--single">
                  <div>
                    <dt>Chunk ID</dt>
                    <dd>{selectedChunk.id}</dd>
                  </div>
                  <div>
                    <dt>Strategy</dt>
                    <dd>{selectedChunk.strategy}</dd>
                  </div>
                  <div>
                    <dt>Builder</dt>
                    <dd>{selectedChunk.builderVersion}</dd>
                  </div>
                  <div>
                    <dt>Core segments</dt>
                    <dd>{selectedChunk.segmentCount}</dd>
                  </div>
                  <div>
                    <dt>Words</dt>
                    <dd>{selectedChunk.sourceWordCount}</dd>
                  </div>
                  <div>
                    <dt>Characters</dt>
                    <dd>{selectedChunk.sourceCharacterCount}</dd>
                  </div>
                  <div>
                    <dt>Translated</dt>
                    <dd>
                      {selectedChunkStatus?.translatedSegmentCount ?? 0}/
                      {selectedChunk.segmentCount}
                    </dd>
                  </div>
                  <div>
                    <dt>Task status</dt>
                    <dd>
                      {formatChunkExecutionStatus(
                        selectedChunkStatus?.status ?? "pending",
                      )}
                    </dd>
                  </div>
                </dl>

                <div className="chunk-detail__body">
                  <div>
                    <p className="surface-card__eyebrow">Context before</p>
                    <div className="segment-detail__text segment-detail__text--muted">
                      {selectedChunk.contextBeforeText ??
                        "No prior overlap segment."}
                    </div>
                  </div>

                  <div>
                    <p className="surface-card__eyebrow">Core source text</p>
                    <div className="segment-detail__text">
                      {selectedChunk.sourceText}
                    </div>
                  </div>

                  <div>
                    <p className="surface-card__eyebrow">Context after</p>
                    <div className="segment-detail__text segment-detail__text--muted">
                      {selectedChunk.contextAfterText ??
                        "No trailing overlap segment."}
                    </div>
                  </div>
                </div>

                <div className="chunk-context-preview">
                  <p className="surface-card__eyebrow">
                    Context builder preview
                  </p>

                  {isLoadingContext ? (
                    <PanelMessage tone="info">
                      Loading the persisted translation context for this
                      chunk...
                    </PanelMessage>
                  ) : null}

                  {contextError ? (
                    <PanelMessage role="alert" tone="danger">
                      {contextError.message}
                    </PanelMessage>
                  ) : null}

                  {contextPreview ? (
                    <>
                      <dl className="detail-list detail-list--single">
                        <div>
                          <dt>Section</dt>
                          <dd>
                            {contextPreview.chunkContext.section?.title ??
                              "No matched section"}
                          </dd>
                        </div>
                        <div>
                          <dt>Glossary layers</dt>
                          <dd>{contextPreview.glossaryLayers.length}</dd>
                        </div>
                        <div>
                          <dt>Resolved rules</dt>
                          <dd>{contextPreview.rules.length}</dd>
                        </div>
                        <div>
                          <dt>Style profile</dt>
                          <dd>
                            {contextPreview.styleProfile?.styleProfile.name ??
                              "No resolved style profile"}
                          </dd>
                        </div>
                        <div>
                          <dt>Rule set</dt>
                          <dd>
                            {contextPreview.ruleSet?.ruleSet.name ??
                              "No resolved rule set"}
                          </dd>
                        </div>
                        <div>
                          <dt>Chapter contexts</dt>
                          <dd>{contextPreview.accumulatedContexts.length}</dd>
                        </div>
                      </dl>

                      <div className="chunk-context-preview__grid">
                        <div className="segment-detail__text segment-detail__text--muted">
                          <strong>Glossaries</strong>
                          <p>
                            {contextPreview.glossaryLayers.length > 0
                              ? contextPreview.glossaryLayers
                                  .map(
                                    (glossaryLayer) =>
                                      `${glossaryLayer.glossary.name} (${glossaryLayer.layer})`,
                                  )
                                  .join(", ")
                              : "No glossary layers are resolved for this chunk."}
                          </p>
                        </div>
                        <div className="segment-detail__text segment-detail__text--muted">
                          <strong>Accumulated context</strong>
                          <p>
                            {contextPreview.accumulatedContexts.length > 0
                              ? contextPreview.accumulatedContexts
                                  .map(
                                    (accumulatedContext) =>
                                      accumulatedContext.chapterContext
                                        .scopeType,
                                  )
                                  .join(", ")
                              : "No accumulated chapter context is attached to this chunk."}
                          </p>
                        </div>
                      </div>
                    </>
                  ) : null}
                </div>

                <div className="chunk-result-workspace">
                  <p className="surface-card__eyebrow">
                    Latest translation result
                  </p>

                  {selectedCoreSegments.length > 0 ? (
                    <ol className="chunk-result-list">
                      {selectedCoreSegments.map((segment) => (
                        <li
                          className="chunk-result-list__item"
                          key={segment.id}
                        >
                          <div className="chunk-link-list__heading">
                            <strong>Segment #{segment.sequence}</strong>
                            <StatusBadge
                              tone={
                                segment.status === "translated"
                                  ? "success"
                                  : "warning"
                              }
                            >
                              {segment.status}
                            </StatusBadge>
                          </div>
                          <div className="chunk-result-list__texts">
                            <div>
                              <p className="surface-card__eyebrow">Source</p>
                              <div className="segment-detail__text">
                                {segment.sourceText}
                              </div>
                            </div>
                            <div>
                              <p className="surface-card__eyebrow">Result</p>
                              <div className="segment-detail__text segment-detail__text--muted">
                                {segment.targetText ??
                                  "No translated target text is persisted for this segment yet."}
                              </div>
                            </div>
                          </div>
                        </li>
                      ))}
                    </ol>
                  ) : (
                    <PanelMessage>
                      This chunk does not expose persisted core segments yet.
                    </PanelMessage>
                  )}
                </div>

                {selectedChunkStatus?.errorMessage ? (
                  <div className="chunk-incident-panel">
                    <p className="surface-card__eyebrow">Incident</p>
                    <PanelMessage role="alert" tone="danger">
                      {selectedChunkStatus.errorMessage}
                    </PanelMessage>
                  </div>
                ) : null}

                <div className="chunk-link-workspace">
                  <p className="surface-card__eyebrow">Linked segments</p>
                  <ol className="chunk-link-list">
                    {selectedChunkSegments.map((chunkSegment) => {
                      const segment =
                        segmentLookup.get(chunkSegment.segmentId) ?? null;

                      return (
                        <li
                          className="chunk-link-list__item"
                          key={`${chunkSegment.chunkId}:${chunkSegment.segmentId}:${chunkSegment.role}`}
                        >
                          <div className="chunk-link-list__heading">
                            <strong>
                              #{chunkSegment.segmentSequence}{" "}
                              {formatChunkRole(chunkSegment.role)}
                            </strong>
                            <StatusBadge tone="info">
                              pos {chunkSegment.position}
                            </StatusBadge>
                          </div>
                          <p>
                            {segment
                              ? truncateText(segment.sourceText)
                              : chunkSegment.segmentId}
                          </p>
                        </li>
                      );
                    })}
                  </ol>
                </div>
              </>
            ) : (
              <PanelMessage>
                Select a chunk to inspect its core text, overlap context, and
                linked persisted segments.
              </PanelMessage>
            )}
          </div>
        </div>
      ) : null}
    </section>
  );
}
