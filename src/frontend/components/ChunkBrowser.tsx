import type {
  DocumentSummary,
  SegmentSummary,
  TranslationChunkSegmentSummary,
  TranslationChunkSummary,
} from "../../shared/desktop";
import type { DesktopCommandError } from "../lib/desktop";

interface ChunkBrowserProps {
  activeDocument: DocumentSummary | null;
  chunkSegments: TranslationChunkSegmentSummary[];
  chunks: TranslationChunkSummary[];
  error: DesktopCommandError | null;
  isBuilding: boolean;
  isLoading: boolean;
  onBuildChunks: () => Promise<void>;
  onSelectChunk: (chunkId: string) => void;
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

export function ChunkBrowser({
  activeDocument,
  chunkSegments,
  chunks,
  error,
  isBuilding,
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

  return (
    <section className="workspace-panel">
      <div className="surface-card__heading">
        <div>
          <p className="surface-card__eyebrow">Translation chunks</p>
          <h3>
            {activeDocument
              ? `Chunked view for ${activeDocument.name}`
              : "Open a segmented document"}
          </h3>
        </div>

        <div className="chunk-browser__actions">
          <strong className="status-pill">
            {activeDocument
              ? `${chunks.length} persisted chunks`
              : "No document"}
          </strong>
          <button
            className="document-action-button"
            disabled={!activeDocument || isBuilding || isLoading}
            onClick={() => void onBuildChunks()}
            type="button"
          >
            {isBuilding
              ? "Building..."
              : chunks.length > 0
                ? "Rebuild chunks"
                : "Build chunks"}
          </button>
        </div>
      </div>

      {!activeDocument ? (
        <p className="surface-card__copy">
          Open a segmented document first. This view will then show persisted
          translation chunks, their core ranges, and the attached overlap
          segments.
        </p>
      ) : null}

      {activeDocument && isLoading ? (
        <p className="surface-card__copy">
          Loading persisted translation chunks for the selected document...
        </p>
      ) : null}

      {error ? (
        <p className="form-error" role="alert">
          {error.message}
        </p>
      ) : null}

      {activeDocument && !isLoading && !error && chunks.length === 0 ? (
        <p className="surface-card__copy">
          No persisted translation chunks exist for this document yet. Build
          them to inspect reproducible core ranges and adjacent context overlap.
        </p>
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
                      <span className="document-status-pill">
                        #{chunk.startSegmentSequence}-#
                        {chunk.endSegmentSequence}
                      </span>
                    </div>
                    <p>{truncateText(chunk.sourceText)}</p>
                    <span className="chunk-list__meta">
                      {chunk.segmentCount} core segments |{" "}
                      {chunk.sourceWordCount} words
                    </span>
                  </button>
                </li>
              ))}
            </ol>

            <p className="surface-card__copy">
              {chunkSegments.length} persisted chunk-to-segment links are loaded
              for this document.
            </p>
          </div>

          <div className="chunk-browser__detail">
            {selectedChunk ? (
              <>
                <div className="surface-card__heading">
                  <div>
                    <p className="surface-card__eyebrow">Selected chunk</p>
                    <h3>Chunk #{selectedChunk.sequence}</h3>
                  </div>

                  <span className="document-status-pill">
                    #{selectedChunk.startSegmentSequence}-#
                    {selectedChunk.endSegmentSequence}
                  </span>
                </div>

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
                            <span className="chunk-role-pill">
                              pos {chunkSegment.position}
                            </span>
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
              <p className="surface-card__copy">
                Select a chunk to inspect its core text, overlap context, and
                linked persisted segments.
              </p>
            )}
          </div>
        </div>
      ) : null}
    </section>
  );
}
