/** A process could not start or stop under the requested OS restrictions. */
export class SandboxError extends Error {
    /** The failed sandbox operation. */
    readonly code: "UNSUPPORTED" | "START_FAILED" | "STOP_FAILED";

    /** Retain the operation and its underlying failure. */
    constructor(code: SandboxError["code"], message: string, options?: ErrorOptions) {
        super(message, options);
        this.name = "SandboxError";
        this.code = code;
    }
}
