import { trace } from "@opentelemetry/api";

function setupTelemetry() {
  // nocheckin
}

export function getTracer(name: string) {
  return trace.getTracer(name);
}
