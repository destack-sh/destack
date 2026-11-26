import type { Environment } from "@destack/environment";
import { BrowserContext } from "@destack/environment/browser";
import { Telemetry, type TelemetryOptions } from "../sdk/index.ts";

/** The global scope whose uncaught failures its telemetry captures: a window or a worker's. */
export interface CaptureScope {
    /** Listen to the scope's uncaught errors and unhandled rejections until a signal aborts. */
    readonly addEventListener: {
        (
            type: "error",
            listener: (event: ErrorEvent) => void,
            options: { readonly signal: AbortSignal },
        ): void;
        (
            type: "unhandledrejection",
            listener: (event: PromiseRejectionEvent) => void,
            options: { readonly signal: AbortSignal },
        ): void;
    };
}

/** The window whose uncaught failures a page captures, and whose hiding exports them. */
export type CaptureWindow = Pick<Window, "addEventListener"> & {
    /** The page's document, read for its visibility. */
    readonly document: Pick<Document, "visibilityState">;
};

/** Start browser telemetry that captures a window's uncaught failures. */
export function startTelemetry(
    options: TelemetryOptions,
    environment: Pick<Environment, "clock" | "random">,
    target: CaptureWindow,
): Promise<Telemetry> {
    return Telemetry.start(options, BrowserContext.install(), environment, (telemetry) =>
        captureWindow(telemetry, target),
    );
}

/** Start a worker's telemetry that captures its uncaught failures. */
export function startWorkerTelemetry(
    options: TelemetryOptions,
    environment: Pick<Environment, "clock" | "random">,
    target: CaptureScope,
): Promise<Telemetry> {
    return Telemetry.start(options, BrowserContext.install(), environment, (telemetry) =>
        captureFailures(telemetry, target),
    );
}

/** Record a window's session with its uncaught errors and unhandled rejections. */
export function captureWindow(telemetry: Telemetry, target: CaptureWindow): () => void {
    // run the page's session from now until it hides, capturing its failures
    telemetry.startSession();
    const stopFailures = captureFailures(telemetry, target);

    // end the session as the page goes away, exporting it
    const onHide = () => {
        telemetry.endSession();
        void telemetry.flush().catch(telemetry.report);
    };

    // export before the page hides, the last moment it reliably runs
    const onVisibility = () => {
        if (target.document.visibilityState === "hidden") {
            void telemetry.flush().catch(telemetry.report);
        }
    };

    // listen for hiding until the telemetry shuts down
    const listening = new AbortController();
    const { signal } = listening;
    target.addEventListener("pagehide", onHide, { signal });
    target.addEventListener("visibilitychange", onVisibility, { signal });

    return () => {
        listening.abort();
        stopFailures();
    };
}

/** Record a scope's uncaught errors and unhandled rejections, exporting each at once. */
export function captureFailures(telemetry: Telemetry, target: CaptureScope): () => void {
    // record each failure and export it, leaving the browser's own reporting as it is
    const onError = (event: ErrorEvent) => {
        const error: unknown = event.error;
        telemetry.capture(error ?? event.message);
        void telemetry.flush().catch(telemetry.report);
    };
    const onRejection = (event: PromiseRejectionEvent) => {
        const reason: unknown = event.reason;
        telemetry.capture(reason);
        void telemetry.flush().catch(telemetry.report);
    };

    // listen for failures until the telemetry shuts down
    const listening = new AbortController();
    const { signal } = listening;
    target.addEventListener("error", onError, { signal });
    target.addEventListener("unhandledrejection", onRejection, { signal });

    return () => listening.abort();
}
