import { StackContextManager } from "@opentelemetry/sdk-trace-web";
import { Telemetry, type TelemetryOptions } from "../sdk/index.ts";

/** The window whose uncaught failures a page captures, and whose hiding exports them. */
export type CaptureWindow = Pick<Window, "addEventListener"> & {
    /** The page's document, read for its visibility. */
    readonly document: Pick<Document, "visibilityState">;
};

/** Start browser telemetry capturing a window's uncaught failures; pass context explicitly across asynchronous calls. */
export function startTelemetry(
    options: TelemetryOptions,
    target: CaptureWindow = window,
): Promise<Telemetry> {
    return Telemetry.start(options, new StackContextManager(), (telemetry) =>
        captureWindow(telemetry, target),
    );
}

/** Record a window's uncaught errors and unhandled rejections, exporting them at once and whenever the page hides. */
export function captureWindow(telemetry: Telemetry, target: CaptureWindow): () => void {
    // export what is buffered, the exporter reporting a failed delivery
    const flush = () => {
        void telemetry.flush().catch(() => {});
    };

    // record each failure and export it, leaving the browser's own reporting as it is
    const onError = (event: ErrorEvent) => {
        const error: unknown = event.error;
        telemetry.capture(error ?? event.message);
        flush();
    };
    const onRejection = (event: PromiseRejectionEvent) => {
        const reason: unknown = event.reason;
        telemetry.capture(reason);
        flush();
    };

    // export before the page hides, the last moment it reliably runs
    const onVisibility = () => {
        if (target.document.visibilityState === "hidden") {
            flush();
        }
    };

    // listen for failures and hiding until the telemetry shuts down
    const listening = new AbortController();
    const { signal } = listening;
    target.addEventListener("error", onError, { signal });
    target.addEventListener("unhandledrejection", onRejection, { signal });
    target.addEventListener("pagehide", flush, { signal });
    target.addEventListener("visibilitychange", onVisibility, { signal });

    return () => listening.abort();
}
