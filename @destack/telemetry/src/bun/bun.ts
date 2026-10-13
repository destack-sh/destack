import { AsyncLocalStorageContextManager } from "@opentelemetry/context-async-hooks";
import { ValueType } from "@opentelemetry/api";
import { PROCESS_CPU_TIME } from "../metric/index.ts";
import { Telemetry, type TelemetryOptions } from "../sdk/index.ts";

/** How long a fatal failure waits for its export before the process ends. */
const FATAL_FLUSH_MILLISECONDS = 2000;

/** The microseconds of a second, the unit process.cpuUsage reports in. */
const MICROSECONDS_PER_SECOND = 1_000_000;

/** The process events reporting an uncaught failure. */
type FailureEvent = "uncaughtException" | "unhandledRejection";

/** Start telemetry in a Bun process with asynchronous context propagation. */
export function startTelemetry(options: TelemetryOptions): Promise<Telemetry> {
    return Telemetry.start(options, new AsyncLocalStorageContextManager(), captureProcess);
}

/** Record and export a process's uncaught exceptions and unhandled rejections. */
export function captureProcess(telemetry: Telemetry): () => void {
    // record a failure and export it, ending the process when no other listener handles it
    const fail = (event: FailureEvent, error: unknown, raise: () => void) => {
        telemetry.capture(error);
        const isFatal = process.listenerCount(event) === 1;
        void within(telemetry.flush(), FATAL_FLUSH_MILLISECONDS).then(() => {
            if (isFatal) {
                remove();
                raise();
            }
        });
    };

    // raise each failure again once the capture is removed, as the process raises it by default
    const onException = (error: Error) =>
        fail("uncaughtException", error, () =>
            setTimeout(() => {
                throw error;
            }),
        );
    const onRejection = (reason: unknown) =>
        fail("unhandledRejection", reason, () => void Promise.reject(reason));
    const remove = () => {
        process.off("uncaughtException", onException);
        process.off("unhandledRejection", onRejection);
    };
    process.on("uncaughtException", onException);
    process.on("unhandledRejection", onRejection);

    return remove;
}

/** Observe a process's CPU time in seconds by CPU mode, as OpenTelemetry's process.cpu.time. */
export function observeProcess(
    telemetry: Telemetry,
    source: Pick<TelemetryOptions, "name" | "version">,
): void {
    // read the user and system CPU time at each collection
    const counter = telemetry.metrics
        .getMeter(source.name, source.version)
        .createObservableCounter(PROCESS_CPU_TIME, {
            unit: "s",
            description: "The CPU time the process used, by CPU mode.",
            valueType: ValueType.DOUBLE,
        });
    counter.addCallback((result) => {
        const { user, system } = process.cpuUsage();
        result.observe(user / MICROSECONDS_PER_SECOND, { "cpu.mode": "user" });
        result.observe(system / MICROSECONDS_PER_SECOND, { "cpu.mode": "system" });
    });
}

/** Settle once an export settles or its deadline passes. */
async function within(operation: Promise<void>, milliseconds: number): Promise<void> {
    let timer: ReturnType<typeof setTimeout> | undefined;
    const deadline = new Promise<void>((resolve) => {
        timer = setTimeout(resolve, milliseconds);
    });
    try {
        await Promise.race([operation.catch(() => {}), deadline]);
    } finally {
        clearTimeout(timer);
    }
}
