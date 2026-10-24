import { AsyncLocalStorageContextManager } from "@opentelemetry/context-async-hooks";
import { ValueType } from "@opentelemetry/api";
import type { Environment } from "@destack/environment";
import { PROCESS_CPU_TIME } from "../metric/index.ts";
import { Telemetry, type TelemetryOptions } from "../sdk/index.ts";

/** How long a fatal failure waits for its export before the process ends. */
const FATAL_FLUSH_MILLISECONDS = 2000;

/** Start telemetry in a Bun process with asynchronous context propagation. */
export function startTelemetry(
    options: TelemetryOptions,
    environment: Pick<Environment<"process">, "clock" | "random" | "process">,
): Promise<Telemetry> {
    return Telemetry.start(options, new AsyncLocalStorageContextManager(), environment, (started) =>
        captureProcess(started, environment),
    );
}

/** Record and export a process's failures nothing caught, before the process ends with them. */
export function captureProcess(
    telemetry: Telemetry,
    environment: Pick<Environment<"process">, "clock" | "process">,
): () => void {
    const subscription = environment.process.onFailure(async (error) => {
        telemetry.capture(error);
        await within(
            telemetry.flush().catch(telemetry.report),
            FATAL_FLUSH_MILLISECONDS,
            environment.clock,
        );
    });

    return () => subscription[Symbol.dispose]();
}

/** Observe a process's CPU time in seconds by CPU mode, as OpenTelemetry's process.cpu.time. */
export function observeProcess(
    telemetry: Telemetry,
    source: Pick<TelemetryOptions, "name" | "version">,
    environment: Pick<Environment<"process">, "process">,
): void {
    // read the user and system CPU time at each collection
    const counter = telemetry.meters
        .getMeter(source.name, source.version)
        .createObservableCounter(PROCESS_CPU_TIME, {
            unit: "s",
            description: "The CPU time the process used, by CPU mode.",
            valueType: ValueType.DOUBLE,
        });
    counter.addCallback((result) => {
        const { user, system } = environment.process.usage();
        result.observe(user, { "cpu.mode": "user" });
        result.observe(system, { "cpu.mode": "system" });
    });
}

/** Settle once an export settles or its deadline passes, the export reporting its own failure. */
async function within(
    operation: Promise<void>,
    milliseconds: number,
    clock: Environment["clock"],
): Promise<void> {
    // race the export against the deadline
    const deadline = new AbortController();
    const waited = clock.sleep(milliseconds, deadline.signal).catch((error: unknown) => {
        if (!deadline.signal.aborted) {
            throw error;
        }
    });
    try {
        await Promise.race([operation, waited]);
    } finally {
        deadline.abort();
    }
}
