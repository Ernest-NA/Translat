import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type {
  CreateRuleSetInput,
  RuleSetStatus,
  RuleSetSummary,
  RuleSetsOverview,
  UpdateRuleSetInput,
} from "../../shared/desktop";
import {
  createRuleSet,
  DesktopCommandError,
  listRuleSets,
  openRuleSet,
  updateRuleSet,
} from "../lib/desktop";

function sortRuleSets(ruleSets: RuleSetSummary[]) {
  return [...ruleSets].sort((left, right) => {
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

function normalizeOverview(overview: RuleSetsOverview): RuleSetsOverview {
  return {
    activeRuleSetId: overview.activeRuleSetId,
    ruleSets: sortRuleSets(overview.ruleSets),
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

export function useRuleSetsWorkspace() {
  const [overview, setOverview] = useState<RuleSetsOverview>({
    activeRuleSetId: null,
    ruleSets: [],
  });
  const [error, setError] = useState<DesktopCommandError | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [isCreating, setIsCreating] = useState(false);
  const [isSaving, setIsSaving] = useState(false);
  const [openingRuleSetId, setOpeningRuleSetId] = useState<string | null>(null);
  const latestReloadRequestRef = useRef(0);
  const localStateVersionRef = useRef(0);

  const applyLocalOverview = useCallback(
    (
      updateOverview: (currentOverview: RuleSetsOverview) => RuleSetsOverview,
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
      const nextOverview = await listRuleSets();

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
                "list_rule_sets",
                "The desktop shell returned an unknown rule-set error.",
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

  const submitRuleSet = useCallback(
    async (input: CreateRuleSetInput): Promise<boolean> => {
      setIsCreating(true);
      setError(null);

      try {
        const createdRuleSet = await createRuleSet(input);
        applyLocalOverview((currentOverview) => ({
          activeRuleSetId: createdRuleSet.id,
          ruleSets: [
            createdRuleSet,
            ...currentOverview.ruleSets.filter(
              (ruleSet) => ruleSet.id !== createdRuleSet.id,
            ),
          ],
        }));
        return true;
      } catch (caughtError) {
        setError(
          caughtError instanceof DesktopCommandError
            ? caughtError
            : buildUnexpectedError(
                "create_rule_set",
                "The desktop shell could not create the rule set.",
              ),
        );
        return false;
      } finally {
        setIsCreating(false);
      }
    },
    [applyLocalOverview],
  );

  const selectRuleSet = useCallback(
    async (ruleSetId: string): Promise<boolean> => {
      setOpeningRuleSetId(ruleSetId);
      setError(null);

      try {
        const openedRuleSet = await openRuleSet({ ruleSetId });
        applyLocalOverview((currentOverview) => ({
          activeRuleSetId: openedRuleSet.id,
          ruleSets: [
            openedRuleSet,
            ...currentOverview.ruleSets.filter(
              (ruleSet) => ruleSet.id !== openedRuleSet.id,
            ),
          ],
        }));
        return true;
      } catch (caughtError) {
        setError(
          caughtError instanceof DesktopCommandError
            ? caughtError
            : buildUnexpectedError(
                "open_rule_set",
                "The desktop shell could not open the rule set.",
              ),
        );
        return false;
      } finally {
        setOpeningRuleSetId(null);
      }
    },
    [applyLocalOverview],
  );

  const saveRuleSet = useCallback(
    async (input: UpdateRuleSetInput): Promise<boolean> => {
      setIsSaving(true);
      setError(null);

      try {
        const updatedRuleSet = await updateRuleSet(input);
        applyLocalOverview((currentOverview) => ({
          activeRuleSetId: updatedRuleSet.id,
          ruleSets: currentOverview.ruleSets.map((ruleSet) =>
            ruleSet.id === updatedRuleSet.id ? updatedRuleSet : ruleSet,
          ),
        }));
        return true;
      } catch (caughtError) {
        setError(
          caughtError instanceof DesktopCommandError
            ? caughtError
            : buildUnexpectedError(
                "update_rule_set",
                "The desktop shell could not save the rule set.",
              ),
        );
        return false;
      } finally {
        setIsSaving(false);
      }
    },
    [applyLocalOverview],
  );

  const activeRuleSet = useMemo(
    () =>
      overview.ruleSets.find(
        (ruleSet) => ruleSet.id === overview.activeRuleSetId,
      ) ?? null,
    [overview.activeRuleSetId, overview.ruleSets],
  );

  const counts = useMemo(
    () => ({
      active: overview.ruleSets.filter((ruleSet) => ruleSet.status === "active")
        .length,
      archived: overview.ruleSets.filter(
        (ruleSet) => ruleSet.status === "archived",
      ).length,
    }),
    [overview.ruleSets],
  );

  const setRuleSetStatus = useCallback(
    async (ruleSet: RuleSetSummary, status: RuleSetStatus): Promise<boolean> =>
      saveRuleSet({
        ruleSetId: ruleSet.id,
        name: ruleSet.name,
        description: ruleSet.description ?? undefined,
        status,
      }),
    [saveRuleSet],
  );

  return {
    activeRuleSet,
    activeRuleSetCount: counts.active,
    archivedRuleSetCount: counts.archived,
    error,
    isCreating,
    isLoading,
    isSaving,
    openingRuleSetId,
    reload,
    ruleSets: overview.ruleSets,
    saveRuleSet,
    selectRuleSet,
    setRuleSetStatus,
    submitRuleSet,
    totalRuleSetCount: overview.ruleSets.length,
  };
}
