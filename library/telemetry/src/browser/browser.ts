import { StackContextManager } from "@opentelemetry/sdk-trace-web";
import { Telemetry, type TelemetryOptions } from "../sdk/index.ts";

/** Start browser telemetry; pass context explicitly across asynchronous calls. */
export function startTelemetry(options: TelemetryOptions): Promise<Telemetry> {
    return Telemetry.start(options, new StackContextManager());
}
