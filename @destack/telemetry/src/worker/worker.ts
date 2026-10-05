import { Telemetry, type TelemetryOptions } from "../sdk/index.ts";
import { StorageContextManager } from "./context.ts";

/** Start telemetry once per worker isolate, keeping context in the AsyncLocalStorage workerd provides under nodejs_compat. */
export function startTelemetry(options: TelemetryOptions): Promise<Telemetry> {
    return Telemetry.start(options, new StorageContextManager());
}

/** Extend an invocation until its exports finish. */
export interface ExecutionContext {
    /** Keep the invocation alive and report a rejected background operation. */
    waitUntil(operation: Promise<unknown>): void;
}

/**
 * Run an invocation with isolated providers and export before releasing them.
 *
 * Record a failure the operation throws before throwing it on.
 * Use the supplied providers and pass trace context explicitly between operations.
 * Pass telemetry.propagator to extractContext and injectContext for HTTP propagation.
 * Resolve the operation after its instrumented work finishes.
 * Use Telemetry directly when a response stream or background task extends that work.
 */
export async function withTelemetry<Result>(
    options: TelemetryOptions,
    operation: (telemetry: Telemetry) => Result | Promise<Result>,
    context: ExecutionContext,
): Promise<Result> {
    const telemetry = new Telemetry(options);

    // keep exporters and credentials within their originating invocation
    try {
        return await operation(telemetry);
    } catch (error) {
        telemetry.capture(error);
        throw error;
    } finally {
        context.waitUntil(telemetry.shutdown());
    }
}
