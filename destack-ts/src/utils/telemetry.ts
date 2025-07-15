import { trace } from "@opentelemetry/api";

function setupTelemetry() {
  // nocheckin: destack-ts telemetry
}

export function getTracer(name: string) {
  return trace.getTracer(name);
}
