import { useCallback, useEffect, useState } from "react";
import type { HealthcheckResponse } from "../../shared/desktop";
import { DesktopCommandError, runHealthcheck } from "../lib/desktop";

export function useHealthcheck() {
  const [healthcheck, setHealthcheck] = useState<HealthcheckResponse | null>(
    null,
  );
  const [error, setError] = useState<DesktopCommandError | null>(null);
  const [isLoading, setIsLoading] = useState(true);

  const retry = useCallback(async () => {
    setIsLoading(true);
    setError(null);

    try {
      const response = await runHealthcheck();
      setHealthcheck(response);
    } catch (caughtError) {
      setHealthcheck(null);
      setError(
        caughtError instanceof DesktopCommandError
          ? caughtError
          : new DesktopCommandError("healthcheck", {
              code: "UNEXPECTED_DESKTOP_ERROR",
              message: "The desktop shell returned an unknown error.",
            }),
      );
    } finally {
      setIsLoading(false);
    }
  }, []);

  useEffect(() => {
    void retry();
  }, [retry]);

  return { error, healthcheck, isLoading, retry };
}
