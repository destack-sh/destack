import { context, propagation, trace } from "@destack/telemetry";
import { setGlobalOtelConfig } from "@orpc/shared";

/** Enable procedure and HTTP tracing through the application's telemetry providers. */
export function instrumentService(): void {
    setGlobalOtelConfig({
        tracer: trace.getTracer("@destack/service"),
        trace,
        context,
        propagation,
    });
}
