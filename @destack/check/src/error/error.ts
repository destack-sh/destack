/** A failed checker invocation or unsupported package. */
export class CheckError extends Error {
    /** Identify the failed operation. */
    constructor(
        readonly code: "tool" | "configuration" | "language",
        message: string,
        options?: ErrorOptions,
    ) {
        super(message, options);
        this.name = "CheckError";
    }
}
