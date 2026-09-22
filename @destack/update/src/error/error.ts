/** A failure reported by the distribution updater. */
export class UpdateError extends Error {
    /** Stable failure category for native callers. */
    readonly code: UpdateErrorCode;

    /** Describe an update failure and retain its underlying cause. */
    constructor(code: UpdateErrorCode, message: string, options?: ErrorOptions) {
        super(message, options);
        this.name = "UpdateError";
        this.code = code;
    }
}

/** Failures detected by Destack's update and installation logic. */
export type UpdateErrorCode =
    | "BUSY"
    | "CLOSED"
    | "RELEASE"
    | "REPOSITORY"
    | "DOWNLOAD"
    | "ACTIVATION"
    | "INSTALL";
