import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type {
  CreateRuleInput,
  RuleSetRulesOverview,
  RuleSummary,
  UpdateRuleInput,
} from "../../shared/desktop";
import {
  createRule,
  DesktopCommandError,
  listRuleSetRules,
  updateRule,
} from "../lib/desktop";

function severityRank(severity: RuleSummary["severity"]) {
  switch (severity) {
    case "high":
      return 0;
    case "medium":
      return 1;
    default:
      return 2;
  }
}

function sortRules(rules: RuleSummary[]) {
  return [...rules].sort((left, right) => {
    if (left.isEnabled !== right.isEnabled) {
      return left.isEnabled ? -1 : 1;
    }

    const severityOrder =
      severityRank(left.severity) - severityRank(right.severity);

    if (severityOrder !== 0) {
      return severityOrder;
    }

    const nameOrder = left.name.localeCompare(right.name, undefined, {
      sensitivity: "base",
    });

    if (nameOrder !== 0) {
      return nameOrder;
    }

    return right.updatedAt - left.updatedAt;
  });
}

function normalizeOverview(
  overview: RuleSetRulesOverview,
): RuleSetRulesOverview {
  return {
    ruleSetId: overview.ruleSetId,
    rules: sortRules(overview.rules),
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

function preferredSelectedRuleId(
  rules: RuleSummary[],
  currentSelectedRuleId: string | null,
) {
  if (currentSelectedRuleId) {
    const matchingRule = rules.find(
      (rule) => rule.id === currentSelectedRuleId,
    );

    if (matchingRule) {
      return matchingRule.id;
    }
  }

  return rules[0]?.id ?? null;
}

export function useRuleSetRules(ruleSetId: string | null) {
  const [overview, setOverview] = useState<RuleSetRulesOverview | null>(null);
  const [selectedRuleId, setSelectedRuleId] = useState<string | null>(null);
  const [error, setError] = useState<DesktopCommandError | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [isCreating, setIsCreating] = useState(false);
  const [isSaving, setIsSaving] = useState(false);
  const latestReloadRequestRef = useRef(0);
  const localStateVersionRef = useRef(0);
  const activeRuleSetIdRef = useRef(ruleSetId);
  const previousRuleSetIdRef = useRef<string | null>(null);

  activeRuleSetIdRef.current = ruleSetId;

  useEffect(() => {
    if (previousRuleSetIdRef.current === ruleSetId) {
      return;
    }

    previousRuleSetIdRef.current = ruleSetId;
    latestReloadRequestRef.current += 1;
    localStateVersionRef.current += 1;
    setSelectedRuleId(null);
    setError(null);
    setIsCreating(false);
    setIsSaving(false);
    setOverview(
      ruleSetId
        ? {
            ruleSetId,
            rules: [],
          }
        : null,
    );
    setIsLoading(Boolean(ruleSetId));
  }, [ruleSetId]);

  const applyLocalOverview = useCallback(
    (
      expectedRuleSetId: string,
      updateOverview: (
        currentOverview: RuleSetRulesOverview | null,
      ) => RuleSetRulesOverview,
      nextSelectedRuleId?: string | null,
    ) => {
      setOverview((currentOverview) => {
        if (activeRuleSetIdRef.current !== expectedRuleSetId) {
          return currentOverview;
        }

        localStateVersionRef.current += 1;

        const normalizedOverview = normalizeOverview(
          updateOverview(currentOverview),
        );
        const resolvedSelectedRuleId =
          nextSelectedRuleId ??
          preferredSelectedRuleId(normalizedOverview.rules, selectedRuleId);

        setSelectedRuleId(resolvedSelectedRuleId);

        return normalizedOverview;
      });
    },
    [selectedRuleId],
  );

  const reportMutationError = useCallback(
    (expectedRuleSetId: string, nextError: DesktopCommandError) => {
      if (activeRuleSetIdRef.current === expectedRuleSetId) {
        setError(nextError);
      }
    },
    [],
  );

  const reload = useCallback(async () => {
    if (!ruleSetId) {
      latestReloadRequestRef.current += 1;
      return;
    }

    const reloadRequestId = latestReloadRequestRef.current + 1;
    const localStateVersionAtStart = localStateVersionRef.current;

    latestReloadRequestRef.current = reloadRequestId;
    setOverview((currentOverview) =>
      currentOverview?.ruleSetId === ruleSetId
        ? currentOverview
        : {
            ruleSetId,
            rules: [],
          },
    );
    setIsLoading(true);
    setError(null);

    try {
      const nextOverview = await listRuleSetRules({ ruleSetId });

      if (reloadRequestId !== latestReloadRequestRef.current) {
        return;
      }

      if (localStateVersionAtStart !== localStateVersionRef.current) {
        return;
      }

      const normalizedOverview = normalizeOverview(nextOverview);

      setOverview(normalizedOverview);
      setSelectedRuleId((currentSelectedRuleId) =>
        preferredSelectedRuleId(
          normalizedOverview.rules,
          currentSelectedRuleId,
        ),
      );
    } catch (caughtError) {
      if (reloadRequestId === latestReloadRequestRef.current) {
        setError(
          caughtError instanceof DesktopCommandError
            ? caughtError
            : buildUnexpectedError(
                "list_rule_set_rules",
                "The desktop shell returned an unknown rule error.",
              ),
        );
      }
    } finally {
      if (reloadRequestId === latestReloadRequestRef.current) {
        setIsLoading(false);
      }
    }
  }, [ruleSetId]);

  useEffect(() => {
    void reload();
  }, [reload]);

  const submitRule = useCallback(
    async (input: Omit<CreateRuleInput, "ruleSetId">): Promise<boolean> => {
      if (!ruleSetId) {
        return false;
      }

      const requestRuleSetId = ruleSetId;

      setIsCreating(true);
      setError(null);

      try {
        const createdRule = await createRule({
          ruleSetId: requestRuleSetId,
          ...input,
        });

        applyLocalOverview(
          requestRuleSetId,
          (currentOverview) => ({
            ruleSetId: requestRuleSetId,
            rules: [
              createdRule,
              ...(currentOverview?.rules ?? []).filter(
                (rule) => rule.id !== createdRule.id,
              ),
            ],
          }),
          createdRule.id,
        );

        return true;
      } catch (caughtError) {
        reportMutationError(
          requestRuleSetId,
          caughtError instanceof DesktopCommandError
            ? caughtError
            : buildUnexpectedError(
                "create_rule",
                "The desktop shell could not create the rule.",
              ),
        );
        return false;
      } finally {
        setIsCreating(false);
      }
    },
    [applyLocalOverview, reportMutationError, ruleSetId],
  );

  const saveRule = useCallback(
    async (input: UpdateRuleInput): Promise<boolean> => {
      const requestRuleSetId = input.ruleSetId;

      setIsSaving(true);
      setError(null);

      try {
        const updatedRule = await updateRule(input);

        applyLocalOverview(
          requestRuleSetId,
          (currentOverview) => ({
            ruleSetId: currentOverview?.ruleSetId ?? updatedRule.ruleSetId,
            rules: [
              updatedRule,
              ...(currentOverview?.rules ?? []).filter(
                (rule) => rule.id !== updatedRule.id,
              ),
            ],
          }),
          updatedRule.id,
        );

        return true;
      } catch (caughtError) {
        reportMutationError(
          requestRuleSetId,
          caughtError instanceof DesktopCommandError
            ? caughtError
            : buildUnexpectedError(
                "update_rule",
                "The desktop shell could not save the rule.",
              ),
        );
        return false;
      } finally {
        setIsSaving(false);
      }
    },
    [applyLocalOverview, reportMutationError],
  );

  const activeRule = useMemo(
    () => overview?.rules.find((rule) => rule.id === selectedRuleId) ?? null,
    [overview, selectedRuleId],
  );

  const counts = useMemo(
    () => ({
      enabled: overview?.rules.filter((rule) => rule.isEnabled).length ?? 0,
      disabled: overview?.rules.filter((rule) => !rule.isEnabled).length ?? 0,
      high:
        overview?.rules.filter((rule) => rule.severity === "high").length ?? 0,
    }),
    [overview],
  );

  return {
    activeRule,
    disabledRuleCount: counts.disabled,
    enabledRuleCount: counts.enabled,
    error,
    highSeverityRuleCount: counts.high,
    isCreating,
    isLoading,
    isSaving,
    reload,
    rules: overview?.rules ?? [],
    saveRule,
    selectRule: setSelectedRuleId,
    selectedRuleId,
    submitRule,
    totalRuleCount: overview?.rules.length ?? 0,
  };
}
