/** A failed checker invocation or unsupported package. */
export class CheckError extends Error {
    /** Failure category. */
    readonly code: "tool" | "configuration" | "language";

    /** Identify the failed operation. */
    constructor(
        code: "tool" | "configuration" | "language",
        message: string,
        options?: ErrorOptions,
    ) {
        super(message, options);

        this.code = code;

        this.name = "CheckError";
    }
}
