import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type {
  CreateGlossaryEntryInput,
  GlossaryEntriesOverview,
  GlossaryEntrySummary,
  UpdateGlossaryEntryInput,
} from "../../shared/desktop";
import {
  createGlossaryEntry,
  DesktopCommandError,
  listGlossaryEntries,
  updateGlossaryEntry,
} from "../lib/desktop";

function sortEntries(entries: GlossaryEntrySummary[]) {
  return [...entries].sort((left, right) => {
    if (left.status !== right.status) {
      return left.status === "active" ? -1 : 1;
    }

    const sourceTermOrder = left.sourceTerm.localeCompare(
      right.sourceTerm,
      undefined,
      {
        sensitivity: "base",
      },
    );

    if (sourceTermOrder !== 0) {
      return sourceTermOrder;
    }

    const targetTermOrder = left.targetTerm.localeCompare(
      right.targetTerm,
      undefined,
      {
        sensitivity: "base",
      },
    );

    if (targetTermOrder !== 0) {
      return targetTermOrder;
    }

    return right.updatedAt - left.updatedAt;
  });
}

function normalizeOverview(
  overview: GlossaryEntriesOverview,
): GlossaryEntriesOverview {
  return {
    glossaryId: overview.glossaryId,
    entries: sortEntries(overview.entries),
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

function preferredSelectedEntryId(
  entries: GlossaryEntrySummary[],
  currentSelectedEntryId: string | null,
) {
  if (currentSelectedEntryId) {
    const matchingEntry = entries.find(
      (entry) => entry.id === currentSelectedEntryId,
    );

    if (matchingEntry) {
      return matchingEntry.id;
    }
  }

  return entries[0]?.id ?? null;
}

export function useGlossaryEntries(glossaryId: string | null) {
  const [overview, setOverview] = useState<GlossaryEntriesOverview | null>(
    null,
  );
  const [selectedEntryId, setSelectedEntryId] = useState<string | null>(null);
  const [error, setError] = useState<DesktopCommandError | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [isCreating, setIsCreating] = useState(false);
  const [isSaving, setIsSaving] = useState(false);
  const latestReloadRequestRef = useRef(0);
  const localStateVersionRef = useRef(0);

  const applyLocalOverview = useCallback(
    (
      updateOverview: (
        currentOverview: GlossaryEntriesOverview | null,
      ) => GlossaryEntriesOverview,
      nextSelectedEntryId?: string | null,
    ) => {
      localStateVersionRef.current += 1;

      setOverview((currentOverview) => {
        const normalizedOverview = normalizeOverview(
          updateOverview(currentOverview),
        );
        const resolvedSelectedEntryId =
          nextSelectedEntryId ??
          preferredSelectedEntryId(normalizedOverview.entries, selectedEntryId);

        setSelectedEntryId(resolvedSelectedEntryId);

        return normalizedOverview;
      });
    },
    [selectedEntryId],
  );

  const reload = useCallback(async () => {
    if (!glossaryId) {
      setOverview(null);
      setSelectedEntryId(null);
      setError(null);
      setIsLoading(false);
      return;
    }

    const reloadRequestId = latestReloadRequestRef.current + 1;
    const localStateVersionAtStart = localStateVersionRef.current;

    latestReloadRequestRef.current = reloadRequestId;
    setIsLoading(true);
    setError(null);

    try {
      const nextOverview = await listGlossaryEntries({ glossaryId });

      if (reloadRequestId !== latestReloadRequestRef.current) {
        return;
      }

      if (localStateVersionAtStart !== localStateVersionRef.current) {
        return;
      }

      const normalizedOverview = normalizeOverview(nextOverview);

      setOverview(normalizedOverview);
      setSelectedEntryId((currentSelectedEntryId) =>
        preferredSelectedEntryId(
          normalizedOverview.entries,
          currentSelectedEntryId,
        ),
      );
    } catch (caughtError) {
      if (reloadRequestId === latestReloadRequestRef.current) {
        setError(
          caughtError instanceof DesktopCommandError
            ? caughtError
            : buildUnexpectedError(
                "list_glossary_entries",
                "The desktop shell returned an unknown glossary entry error.",
              ),
        );
      }
    } finally {
      if (reloadRequestId === latestReloadRequestRef.current) {
        setIsLoading(false);
      }
    }
  }, [glossaryId]);

  useEffect(() => {
    void reload();
  }, [reload]);

  const submitEntry = useCallback(
    async (
      input: Omit<CreateGlossaryEntryInput, "glossaryId">,
    ): Promise<boolean> => {
      if (!glossaryId) {
        return false;
      }

      setIsCreating(true);
      setError(null);

      try {
        const createdEntry = await createGlossaryEntry({
          glossaryId,
          ...input,
        });

        applyLocalOverview(
          (currentOverview) => ({
            glossaryId,
            entries: [
              createdEntry,
              ...(currentOverview?.entries ?? []).filter(
                (entry) => entry.id !== createdEntry.id,
              ),
            ],
          }),
          createdEntry.id,
        );

        return true;
      } catch (caughtError) {
        setError(
          caughtError instanceof DesktopCommandError
            ? caughtError
            : buildUnexpectedError(
                "create_glossary_entry",
                "The desktop shell could not create the glossary entry.",
              ),
        );
        return false;
      } finally {
        setIsCreating(false);
      }
    },
    [applyLocalOverview, glossaryId],
  );

  const saveEntry = useCallback(
    async (input: UpdateGlossaryEntryInput): Promise<boolean> => {
      setIsSaving(true);
      setError(null);

      try {
        const updatedEntry = await updateGlossaryEntry(input);

        applyLocalOverview(
          (currentOverview) => ({
            glossaryId: currentOverview?.glossaryId ?? updatedEntry.glossaryId,
            entries: (currentOverview?.entries ?? []).map((entry) =>
              entry.id === updatedEntry.id ? updatedEntry : entry,
            ),
          }),
          updatedEntry.id,
        );

        return true;
      } catch (caughtError) {
        setError(
          caughtError instanceof DesktopCommandError
            ? caughtError
            : buildUnexpectedError(
                "update_glossary_entry",
                "The desktop shell could not save the glossary entry.",
              ),
        );
        return false;
      } finally {
        setIsSaving(false);
      }
    },
    [applyLocalOverview],
  );

  const activeEntry = useMemo(
    () =>
      overview?.entries.find((entry) => entry.id === selectedEntryId) ?? null,
    [overview, selectedEntryId],
  );

  const counts = useMemo(
    () => ({
      active:
        overview?.entries.filter((entry) => entry.status === "active").length ??
        0,
      archived:
        overview?.entries.filter((entry) => entry.status === "archived")
          .length ?? 0,
    }),
    [overview],
  );

  return {
    activeEntry,
    activeEntryCount: counts.active,
    archivedEntryCount: counts.archived,
    entries: overview?.entries ?? [],
    error,
    isCreating,
    isLoading,
    isSaving,
    reload,
    saveEntry,
    selectEntry: setSelectedEntryId,
    selectedEntryId,
    submitEntry,
    totalEntryCount: overview?.entries.length ?? 0,
  };
}
