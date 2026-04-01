import { useState } from "react";
import type { DesktopCommandError } from "../lib/desktop";

interface ProjectComposerProps {
  error: DesktopCommandError | null;
  isCreating: boolean;
  onSubmit: (input: { description?: string; name: string }) => Promise<void>;
}

export function ProjectComposer({
  error,
  isCreating,
  onSubmit,
}: ProjectComposerProps) {
  const [description, setDescription] = useState("");
  const [name, setName] = useState("");

  async function handleSubmit(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();

    await onSubmit({
      description,
      name,
    });

    setName("");
    setDescription("");
  }

  return (
    <section className="surface-card surface-card--split">
      <div>
        <p className="surface-card__eyebrow">Create project</p>
        <h2>Start a real translation workspace.</h2>
        <p className="surface-card__copy">
          Create the container that will hold documents and later workflow
          modules. C1 keeps the metadata deliberately small.
        </p>
      </div>

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
          <button
            className="app-shell__button"
            disabled={isCreating}
            type="submit"
          >
            {isCreating ? "Creating project..." : "Create project"}
          </button>

          <span className="project-form__hint">
            The new project is opened immediately after persistence.
          </span>
        </div>

        {error ? (
          <p className="form-error" role="alert">
            {error.message}
          </p>
        ) : null}
      </form>
    </section>
  );
}
