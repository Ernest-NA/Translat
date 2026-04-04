import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type {
  CreateStyleProfileInput,
  StyleProfileStatus,
  StyleProfileSummary,
  StyleProfilesOverview,
  UpdateStyleProfileInput,
} from "../../shared/desktop";
import {
  createStyleProfile,
  DesktopCommandError,
  listStyleProfiles,
  openStyleProfile,
  updateStyleProfile,
} from "../lib/desktop";

function sortStyleProfiles(styleProfiles: StyleProfileSummary[]) {
  return [...styleProfiles].sort((left, right) => {
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

function normalizeOverview(
  overview: StyleProfilesOverview,
): StyleProfilesOverview {
  return {
    activeStyleProfileId: overview.activeStyleProfileId,
    styleProfiles: sortStyleProfiles(overview.styleProfiles),
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

export function useStyleProfilesWorkspace() {
  const [overview, setOverview] = useState<StyleProfilesOverview>({
    activeStyleProfileId: null,
    styleProfiles: [],
  });
  const [error, setError] = useState<DesktopCommandError | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [isCreating, setIsCreating] = useState(false);
  const [isSaving, setIsSaving] = useState(false);
  const [openingStyleProfileId, setOpeningStyleProfileId] = useState<
    string | null
  >(null);
  const latestReloadRequestRef = useRef(0);
  const localStateVersionRef = useRef(0);

  const applyLocalOverview = useCallback(
    (
      updateOverview: (
        currentOverview: StyleProfilesOverview,
      ) => StyleProfilesOverview,
    ) => {
      localStateVersionRef.current += 1;
      setOverview((currentOverview) =>
        normalizeOverview(updateOverview(currentOverview)),
      );
    },
    [],
  );

  const reload = useCallback(async () => {
    const reloadRequestId = latestReloadRequestRef.current + 1;
    const localStateVersionAtStart = localStateVersionRef.current;

    latestReloadRequestRef.current = reloadRequestId;
    setIsLoading(true);
    setError(null);

    try {
      const nextOverview = await listStyleProfiles();

      if (reloadRequestId !== latestReloadRequestRef.current) {
        return;
      }

      if (localStateVersionAtStart !== localStateVersionRef.current) {
        return;
      }

      setOverview(normalizeOverview(nextOverview));
    } catch (caughtError) {
      if (reloadRequestId === latestReloadRequestRef.current) {
        setError(
          caughtError instanceof DesktopCommandError
            ? caughtError
            : buildUnexpectedError(
                "list_style_profiles",
                "The desktop shell returned an unknown style-profile error.",
              ),
        );
      }
    } finally {
      if (reloadRequestId === latestReloadRequestRef.current) {
        setIsLoading(false);
      }
    }
  }, []);

  useEffect(() => {
    void reload();
  }, [reload]);

  const submitStyleProfile = useCallback(
    async (input: CreateStyleProfileInput): Promise<boolean> => {
      setIsCreating(true);
      setError(null);

      try {
        const createdStyleProfile = await createStyleProfile(input);
        applyLocalOverview((currentOverview) => ({
          activeStyleProfileId: createdStyleProfile.id,
          styleProfiles: [
            createdStyleProfile,
            ...currentOverview.styleProfiles.filter(
              (styleProfile) => styleProfile.id !== createdStyleProfile.id,
            ),
          ],
        }));
        return true;
      } catch (caughtError) {
        setError(
          caughtError instanceof DesktopCommandError
            ? caughtError
            : buildUnexpectedError(
                "create_style_profile",
                "The desktop shell could not create the style profile.",
              ),
        );
        return false;
      } finally {
        setIsCreating(false);
      }
    },
    [applyLocalOverview],
  );

  const selectStyleProfile = useCallback(
    async (styleProfileId: string): Promise<boolean> => {
      setOpeningStyleProfileId(styleProfileId);
      setError(null);

      try {
        const openedStyleProfile = await openStyleProfile({ styleProfileId });
        applyLocalOverview((currentOverview) => ({
          activeStyleProfileId: openedStyleProfile.id,
          styleProfiles: [
            openedStyleProfile,
            ...currentOverview.styleProfiles.filter(
              (styleProfile) => styleProfile.id !== openedStyleProfile.id,
            ),
          ],
        }));
        return true;
      } catch (caughtError) {
        setError(
          caughtError instanceof DesktopCommandError
            ? caughtError
            : buildUnexpectedError(
                "open_style_profile",
                "The desktop shell could not open the style profile.",
              ),
        );
        return false;
      } finally {
        setOpeningStyleProfileId(null);
      }
    },
    [applyLocalOverview],
  );

  const saveStyleProfile = useCallback(
    async (input: UpdateStyleProfileInput): Promise<boolean> => {
      setIsSaving(true);
      setError(null);

      try {
        const updatedStyleProfile = await updateStyleProfile(input);
        applyLocalOverview((currentOverview) => ({
          activeStyleProfileId: updatedStyleProfile.id,
          styleProfiles: currentOverview.styleProfiles.map((styleProfile) =>
            styleProfile.id === updatedStyleProfile.id
              ? updatedStyleProfile
              : styleProfile,
          ),
        }));
        return true;
      } catch (caughtError) {
        setError(
          caughtError instanceof DesktopCommandError
            ? caughtError
            : buildUnexpectedError(
                "update_style_profile",
                "The desktop shell could not save the style profile.",
              ),
        );
        return false;
      } finally {
        setIsSaving(false);
      }
    },
    [applyLocalOverview],
  );

  const activeStyleProfile = useMemo(
    () =>
      overview.styleProfiles.find(
        (styleProfile) => styleProfile.id === overview.activeStyleProfileId,
      ) ?? null,
    [overview.activeStyleProfileId, overview.styleProfiles],
  );

  const counts = useMemo(
    () => ({
      active: overview.styleProfiles.filter(
        (styleProfile) => styleProfile.status === "active",
      ).length,
      archived: overview.styleProfiles.filter(
        (styleProfile) => styleProfile.status === "archived",
      ).length,
    }),
    [overview.styleProfiles],
  );

  const setStyleProfileStatus = useCallback(
    async (
      styleProfile: StyleProfileSummary,
      status: StyleProfileStatus,
    ): Promise<boolean> =>
      saveStyleProfile({
        styleProfileId: styleProfile.id,
        name: styleProfile.name,
        description: styleProfile.description ?? undefined,
        tone: styleProfile.tone,
        formality: styleProfile.formality,
        treatmentPreference: styleProfile.treatmentPreference,
        consistencyInstructions:
          styleProfile.consistencyInstructions ?? undefined,
        editorialNotes: styleProfile.editorialNotes ?? undefined,
        status,
      }),
    [saveStyleProfile],
  );

  return {
    activeStyleProfile,
    activeStyleProfileCount: counts.active,
    archivedStyleProfileCount: counts.archived,
    error,
    isCreating,
    isLoading,
    isSaving,
    openingStyleProfileId,
    reload,
    saveStyleProfile,
    selectStyleProfile,
    setStyleProfileStatus,
    styleProfiles: overview.styleProfiles,
    submitStyleProfile,
    totalStyleProfileCount: overview.styleProfiles.length,
  };
}
