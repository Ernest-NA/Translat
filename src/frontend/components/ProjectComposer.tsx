import { useState } from "react";
import type { DesktopCommandError } from "../lib/desktop";
import { ActionButton } from "./ui/ActionButton";
import { PanelHeader } from "./ui/PanelHeader";
import { PanelMessage } from "./ui/PanelMessage";

interface ProjectComposerProps {
  error: DesktopCommandError | null;
  isCreating: boolean;
  isRuntimeUnavailable?: boolean;
  onSubmit: (input: { description?: string; name: string }) => Promise<boolean>;
}

export function ProjectComposer({
  error,
  isCreating,
  isRuntimeUnavailable = false,
  onSubmit,
}: ProjectComposerProps) {
  const [description, setDescription] = useState("");
  const [name, setName] = useState("");
  const isSubmitDisabled = isCreating || isRuntimeUnavailable;
  const displayedError = isRuntimeUnavailable ? null : error;

  async function handleSubmit(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();

    const wasCreated = await onSubmit({
      description,
      name,
    });

    if (wasCreated) {
      setName("");
      setDescription("");
    }
  }

  return (
    <section className="surface-card surface-card--split">
      <PanelHeader
        description="Create the container that will hold documents and later workflow modules. C1 keeps the metadata deliberately small."
        eyebrow="Create project"
        title="Start a real translation workspace."
        titleLevel={2}
      />

      <form className="project-form" onSubmit={handleSubmit}>
        <label className="field-group">
          <span>Project name</span>
          <input
            autoComplete="off"
            className="field-control"
            disabled={isCreating}
            maxLength={120}
            name="name"
            onChange={(event) => setName(event.target.value)}
            placeholder="Clinical trials pilot"
            required
            value={name}
          />
        </label>

        <label className="field-group">
          <span>Short description</span>
          <textarea
            className="field-control field-control--textarea"
            disabled={isCreating}
            maxLength={1000}
            name="description"
            onChange={(event) => setDescription(event.target.value)}
            placeholder="Optional context for the project workspace."
            rows={4}
            value={description}
          />
        </label>

        <div className="project-form__footer">
          <ActionButton
            disabled={isSubmitDisabled}
            mobileFullWidth
            size="md"
            type="submit"
            variant="primary"
          >
            {isRuntimeUnavailable
              ? "Desktop app required"
              : isCreating
                ? "Creating project..."
                : "Create project"}
          </ActionButton>

          <span className="project-form__hint">
            {isRuntimeUnavailable
              ? "Persistence commands run in the desktop app."
              : "The new project is opened immediately after persistence."}
          </span>
        </div>

        {isRuntimeUnavailable ? (
          <PanelMessage tone="warning" title="Browser preview mode">
            This preview cannot create persisted projects because the Tauri
            desktop bridge is not available.
          </PanelMessage>
        ) : null}

        {displayedError ? (
          <PanelMessage role="alert" tone="danger">
            {displayedError.message}
          </PanelMessage>
        ) : null}
      </form>
    </section>
  );
}
