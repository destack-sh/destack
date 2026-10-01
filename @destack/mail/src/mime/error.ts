/** Failures composing a message. */
export type MimeErrorCode = "INVALID_ADDRESS" | "INVALID_HEADER" | "INVALID_KEY" | "INVALID_DATE";

/** A message composition failure with a stable code. */
export class MimeError extends Error {
    /** The failure code. */
    readonly code: MimeErrorCode;

    /** Create a message composition failure. */
    constructor(code: MimeErrorCode, message: string, options?: ErrorOptions) {
        super(message, options);
        this.name = "MimeError";
        this.code = code;
    }
}
