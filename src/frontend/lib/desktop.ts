import { invoke } from "@tauri-apps/api/core";
import {
  HEALTHCHECK_COMMAND,
  type HealthcheckResponse,
} from "../../shared/desktop";

export function runHealthcheck() {
  return invoke<HealthcheckResponse>(HEALTHCHECK_COMMAND);
}
