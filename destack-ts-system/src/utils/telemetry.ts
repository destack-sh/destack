import type { NodeSDK } from "@opentelemetry/sdk-node";
import { getFromEnvMaybe, setupTelemetry as setupTelemetryBase } from "destack";

let _sdk: NodeSDK | null = null;

const SERVICE_NAME = getFromEnvMaybe("SERVICE_NAME", "string") ?? "destack-ts-system";

export function setupTelemetry() {
  if (_sdk != null) {
    return;
  }
  setupTelemetryBase();
  // TODO: setup opentelemetry
}

setupTelemetry();
