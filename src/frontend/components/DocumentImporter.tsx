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

    try {
      await onImport(nextFiles);
    } finally {
      event.target.value = "";
    }
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
        C2 still handles intake here: import keeps the source file under local
        protected storage so C3 can normalize and segment it afterward.
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
          Import one source document per action. After it lands in the project,
          C3 can process it into persisted segments.
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
