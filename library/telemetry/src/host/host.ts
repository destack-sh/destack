import { AsyncLocalStorageContextManager } from "@opentelemetry/context-async-hooks";
import { Telemetry, type TelemetryOptions } from "../sdk/index.ts";

/** Start host telemetry with asynchronous context propagation. */
export function startTelemetry(options: TelemetryOptions): Promise<Telemetry> {
    return Telemetry.start(options, new AsyncLocalStorageContextManager());
}
