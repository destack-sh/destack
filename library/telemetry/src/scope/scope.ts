import { type Meter, metrics, trace, type Tracer } from "@opentelemetry/api";
import { type Logger, logs } from "@opentelemetry/api-logs";
import type { Package } from "@destack/package";

/** Trace, metric, and log instruments attributed to one package. */
export interface TelemetryScope {
    /** Create spans attributed to the package. */
    readonly tracer: Tracer;
    /** Create metric instruments attributed to the package. */
    readonly meter: Meter;
    /** Emit logs attributed to the package. */
    readonly logger: Logger;
}

/** Obtain package instruments from the providers registered by the host. */
export function scope(source: Package): TelemetryScope {
    return {
        tracer: trace.getTracer(source.name, source.version),
        meter: metrics.getMeter(source.name, source.version),
        logger: logs.getLogger(source.name, source.version),
    };
}
