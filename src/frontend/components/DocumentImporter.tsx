import type { ChangeEvent } from "react";
import { useRef } from "react";
import type { ProjectSummary } from "../../shared/desktop";
import type { DesktopCommandError } from "../lib/desktop";

interface DocumentImporterProps {
  error: DesktopCommandError | null;
  isImporting: boolean;
  onImport: (files: FileList) => Promise<number>;
  project: ProjectSummary;
}

export function DocumentImporter({
  error,
  isImporting,
  onImport,
  project,
}: DocumentImporterProps) {
  const inputRef = useRef<HTMLInputElement | null>(null);

  async function handleFileSelection(event: ChangeEvent<HTMLInputElement>) {
    const nextFiles = event.target.files;

    if (!nextFiles || nextFiles.length === 0) {
      return;
    }

    await onImport(nextFiles);
    event.target.value = "";
  }

  return (
    <section className="workspace-panel">
      <div className="surface-card__heading">
        <div>
          <p className="surface-card__eyebrow">Import documents</p>
          <h3>Register source files inside the project.</h3>
        </div>

        <strong className="status-pill">{project.name}</strong>
      </div>

      <p className="surface-card__copy">
        C2 copies selected files into Translat storage, records minimal
        metadata, and marks them as ready for C3 input. No normalization or
        segmentation happens here.
      </p>

      <div className="document-importer__actions">
        <button
          className="app-shell__button"
          disabled={isImporting}
          onClick={() => inputRef.current?.click()}
          type="button"
        >
          {isImporting ? "Importing document..." : "Select document file"}
        </button>

        <span className="project-form__hint">
          C2 imports one source document per action. The file is copied into
          local workspace storage before C3 processes it.
        </span>
      </div>

      <input hidden onChange={handleFileSelection} ref={inputRef} type="file" />

      {error ? (
        <p className="form-error" role="alert">
          {error.message}
        </p>
      ) : null}
    </section>
  );
}
