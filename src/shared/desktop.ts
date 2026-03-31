export const HEALTHCHECK_COMMAND = "healthcheck";

export interface HealthcheckResponse {
  app: string;
  message: string;
  status: "ok";
}
