import { AsyncLocalStorageContextManager } from "@opentelemetry/context-async-hooks";
import { Telemetry, type TelemetryOptions } from "../sdk/index.ts";

/** How long a fatal failure waits for its export before the process ends. */
const FATAL_FLUSH_MILLISECONDS = 2000;

/** The process events reporting an uncaught failure. */
type FailureEvent = "uncaughtException" | "unhandledRejection";

/** Start host telemetry with asynchronous context propagation, capturing the process's uncaught failures. */
export function startTelemetry(options: TelemetryOptions): Promise<Telemetry> {
    return Telemetry.start(options, new AsyncLocalStorageContextManager(), captureProcess);
}

/** Record and export a process's uncaught exceptions and unhandled rejections, ending it as it would have ended. */
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

/** Settle once an export settles or its deadline passes, the exporter reporting a failed delivery. */
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
