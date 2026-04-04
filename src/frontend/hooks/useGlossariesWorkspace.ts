import { useCallback, useEffect, useMemo, useState } from "react";
import type {
  CreateGlossaryInput,
  GlossariesOverview,
  GlossaryStatus,
  GlossarySummary,
  UpdateGlossaryInput,
} from "../../shared/desktop";
import {
  createGlossary,
  DesktopCommandError,
  listGlossaries,
  openGlossary,
  updateGlossary,
} from "../lib/desktop";

function sortGlossaries(glossaries: GlossarySummary[]) {
  return [...glossaries].sort((left, right) => {
    if (left.status !== right.status) {
      return left.status === "active" ? -1 : 1;
    }

    if (left.lastOpenedAt !== right.lastOpenedAt) {
      return right.lastOpenedAt - left.lastOpenedAt;
    }

    if (left.createdAt !== right.createdAt) {
      return right.createdAt - left.createdAt;
    }

    return left.name.localeCompare(right.name, undefined, {
      sensitivity: "base",
    });
  });
}

function normalizeOverview(overview: GlossariesOverview): GlossariesOverview {
  return {
    activeGlossaryId: overview.activeGlossaryId,
    glossaries: sortGlossaries(overview.glossaries),
  };
}

function buildUnexpectedError(
  command: string,
  message: string,
): DesktopCommandError {
  return new DesktopCommandError(command as never, {
    code: "UNEXPECTED_DESKTOP_ERROR",
    message,
  });
}

export function useGlossariesWorkspace() {
  const [overview, setOverview] = useState<GlossariesOverview>({
    activeGlossaryId: null,
    glossaries: [],
  });
  const [error, setError] = useState<DesktopCommandError | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [isCreating, setIsCreating] = useState(false);
  const [isSaving, setIsSaving] = useState(false);
  const [openingGlossaryId, setOpeningGlossaryId] = useState<string | null>(
    null,
  );

  const reload = useCallback(async () => {
    setIsLoading(true);
    setError(null);

    try {
      const nextOverview = await listGlossaries();
      setOverview(normalizeOverview(nextOverview));
    } catch (caughtError) {
      setError(
        caughtError instanceof DesktopCommandError
          ? caughtError
          : buildUnexpectedError(
              "list_glossaries",
              "The desktop shell returned an unknown glossary error.",
            ),
      );
    } finally {
      setIsLoading(false);
    }
  }, []);

  useEffect(() => {
    void reload();
  }, [reload]);

  const submitGlossary = useCallback(
    async (input: CreateGlossaryInput): Promise<boolean> => {
      setIsCreating(true);
      setError(null);

      try {
        const createdGlossary = await createGlossary(input);
        setOverview((currentOverview) =>
          normalizeOverview({
            activeGlossaryId: createdGlossary.id,
            glossaries: [
              createdGlossary,
              ...currentOverview.glossaries.filter(
                (glossary) => glossary.id !== createdGlossary.id,
              ),
            ],
          }),
        );
        return true;
      } catch (caughtError) {
        setError(
          caughtError instanceof DesktopCommandError
            ? caughtError
            : buildUnexpectedError(
                "create_glossary",
                "The desktop shell could not create the glossary.",
              ),
        );
        return false;
      } finally {
        setIsCreating(false);
      }
    },
    [],
  );

  const selectGlossary = useCallback(
    async (glossaryId: string): Promise<boolean> => {
      setOpeningGlossaryId(glossaryId);
      setError(null);

      try {
        const openedGlossary = await openGlossary({ glossaryId });
        setOverview((currentOverview) =>
          normalizeOverview({
            activeGlossaryId: openedGlossary.id,
            glossaries: [
              openedGlossary,
              ...currentOverview.glossaries.filter(
                (glossary) => glossary.id !== openedGlossary.id,
              ),
            ],
          }),
        );
        return true;
      } catch (caughtError) {
        setError(
          caughtError instanceof DesktopCommandError
            ? caughtError
            : buildUnexpectedError(
                "open_glossary",
                "The desktop shell could not open the glossary.",
              ),
        );
        return false;
      } finally {
        setOpeningGlossaryId(null);
      }
    },
    [],
  );

  const saveGlossary = useCallback(
    async (input: UpdateGlossaryInput): Promise<boolean> => {
      setIsSaving(true);
      setError(null);

      try {
        const updatedGlossary = await updateGlossary(input);
        setOverview((currentOverview) =>
          normalizeOverview({
            activeGlossaryId: updatedGlossary.id,
            glossaries: currentOverview.glossaries.map((glossary) =>
              glossary.id === updatedGlossary.id ? updatedGlossary : glossary,
            ),
          }),
        );
        return true;
      } catch (caughtError) {
        setError(
          caughtError instanceof DesktopCommandError
            ? caughtError
            : buildUnexpectedError(
                "update_glossary",
                "The desktop shell could not save the glossary.",
              ),
        );
        return false;
      } finally {
        setIsSaving(false);
      }
    },
    [],
  );

  const activeGlossary = useMemo(
    () =>
      overview.glossaries.find(
        (glossary) => glossary.id === overview.activeGlossaryId,
      ) ?? null,
    [overview.activeGlossaryId, overview.glossaries],
  );

  const counts = useMemo(
    () => ({
      active: overview.glossaries.filter(
        (glossary) => glossary.status === "active",
      ).length,
      archived: overview.glossaries.filter(
        (glossary) => glossary.status === "archived",
      ).length,
    }),
    [overview.glossaries],
  );

  const setGlossaryStatus = useCallback(
    async (
      glossary: GlossarySummary,
      status: GlossaryStatus,
    ): Promise<boolean> =>
      saveGlossary({
        glossaryId: glossary.id,
        name: glossary.name,
        description: glossary.description ?? undefined,
        projectId: glossary.projectId ?? undefined,
        status,
      }),
    [saveGlossary],
  );

  return {
    activeGlossary,
    archivedGlossaryCount: counts.archived,
    error,
    glossaries: overview.glossaries,
    isCreating,
    isLoading,
    isSaving,
    openingGlossaryId,
    reload,
    saveGlossary,
    selectGlossary,
    setGlossaryStatus,
    submitGlossary,
    totalGlossaryCount: overview.glossaries.length,
    activeGlossaryCount: counts.active,
  };
}
