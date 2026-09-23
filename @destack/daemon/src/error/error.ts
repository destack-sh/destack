/** Failures in local daemon discovery and lifecycle operations. */
export type DaemonErrorCode =
    | "NOT_RUNNING"
    | "ALREADY_RUNNING"
    | "INVALID_ENDPOINT"
    | "START_FAILED"
    | "STOP_FAILED";

/** A local daemon failure with a stable code and retained cause. */
export class DaemonError extends Error {
    /** Machine-readable failure code. */
    readonly code: DaemonErrorCode;

    constructor(code: DaemonErrorCode, message: string, options?: ErrorOptions) {
        super(message, options);
        this.name = "DaemonError";
        this.code = code;
    }
}
