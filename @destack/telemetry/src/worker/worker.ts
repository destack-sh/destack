import type { Environment } from "@destack/environment";
import { Telemetry, type TelemetryOptions } from "../sdk/index.ts";
import { WorkerContextManager } from "./context.ts";

/** Start telemetry once per worker isolate. */
export function startTelemetry(
    options: TelemetryOptions,
    environment: Pick<Environment, "clock" | "random">,
): Promise<Telemetry> {
    return Telemetry.start(options, new WorkerContextManager(), environment);
}

/** Extend an invocation until its exports finish. */
export interface ExecutionContext {
    /** Hold the invocation open until an operation settles, and report its rejection. */
    readonly waitUntil: (operation: Promise<unknown>) => void;
}

/** Run an invocation with isolated providers and export before releasing them. */
export async function withTelemetry<Result>(
    options: TelemetryOptions,
    operation: (telemetry: Telemetry) => Result | Promise<Result>,
    context: ExecutionContext,
    environment: Pick<Environment, "clock" | "random">,
): Promise<Result> {
    const telemetry = new Telemetry(options, new WorkerContextManager().enable(), environment);

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
