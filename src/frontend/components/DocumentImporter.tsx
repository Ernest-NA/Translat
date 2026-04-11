import type { ChangeEvent } from "react";
import { useRef } from "react";
import type { ProjectSummary } from "../../shared/desktop";
import type { DesktopCommandError } from "../lib/desktop";
import { ActionButton } from "./ui/ActionButton";
import { PanelHeader } from "./ui/PanelHeader";
import { PanelMessage } from "./ui/PanelMessage";
import { StatusBadge } from "./ui/StatusBadge";

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
      <PanelHeader
        eyebrow="Import documents"
        meta={
          <StatusBadge size="md" tone="info">
            {project.name}
          </StatusBadge>
        }
        title="Register source files inside the project."
      />

      <p className="surface-card__copy">
        C2 still handles intake here: import keeps the source file under local
        protected storage so C3 can normalize and segment it afterward.
      </p>

      <div className="document-importer__actions">
        <ActionButton
          disabled={isImporting}
          onClick={() => inputRef.current?.click()}
          size="md"
          variant="primary"
        >
          {isImporting ? "Importing document..." : "Select document file"}
        </ActionButton>

        <span className="project-form__hint">
          Import one source document per action. After it lands in the project,
          C3 can process it into persisted segments.
        </span>
      </div>

      <input hidden onChange={handleFileSelection} ref={inputRef} type="file" />

      {error ? (
        <PanelMessage role="alert" tone="danger">
          {error.message}
        </PanelMessage>
      ) : null}
    </section>
  );
}
